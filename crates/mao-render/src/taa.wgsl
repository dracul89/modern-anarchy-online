// Temporal anti-aliasing: each frame is drawn with the projection nudged by a sub-pixel jitter (a Halton (2, 3) sequence), and
// blended into the frames before it, followed to where they were by the motion buffer. The history is clipped to the colours
// around the pixel this frame (in YCoCg, about their mean and spread), so what moved or appeared doesn't leave a trail.
//
// The motion buffer (the scene pass's second target): rg the move since the last frame in texture coordinates (now − before),
// b 1 for a part that animates in ways the matrices don't show (the characters: less history is kept), a 1 where a mesh was
// drawn. Where nothing was (the sky, a backdrop), the move is the camera's alone, found from the depth.

struct Params {
    // This frame's view and projection undone (unjittered), and the last frame's.
    inverse_vp: mat4x4<f32>,
    previous_vp: mat4x4<f32>,
    // xy: one texel; z: 1 when there is a history to blend with; w: the share of this frame in the blend.
    texel: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var current: texture_2d<f32>;
@group(0) @binding(2) var motion: texture_2d<f32>;
@group(0) @binding(3) var depth: texture_depth_2d;
@group(0) @binding(4) var history: texture_2d<f32>;
@group(0) @binding(5) var bilinear: sampler;

@vertex
fn vs_taa(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

fn to_ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.25 * c.r + 0.5 * c.g + 0.25 * c.b, 0.5 * c.r - 0.5 * c.b, -0.25 * c.r + 0.5 * c.g - 0.25 * c.b);
}

fn from_ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(c.x + c.y - c.z, c.x + c.z, c.x - c.y - c.z);
}

// The history at `uv`, sharp: a Catmull-Rom filter in five bilinear taps (its corners left out).
fn history_at(uv: vec2<f32>) -> vec3<f32> {
    let size = 1.0 / params.texel.xy;
    let position = uv * size;
    let centre = floor(position - 0.5) + 0.5;
    let f = position - centre;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let offset12 = w2 / w12;
    let t0 = (centre - 1.0) * params.texel.xy;
    let t3 = (centre + 2.0) * params.texel.xy;
    let t12 = (centre + offset12) * params.texel.xy;
    var sum = vec3<f32>(0.0);
    var weight = 0.0;
    let taps = array<vec3<f32>, 5>(
        vec3<f32>(t12.x, t0.y, w12.x * w0.y),
        vec3<f32>(t0.x, t12.y, w0.x * w12.y),
        vec3<f32>(t12.x, t12.y, w12.x * w12.y),
        vec3<f32>(t3.x, t12.y, w3.x * w12.y),
        vec3<f32>(t12.x, t3.y, w12.x * w3.y),
    );
    for (var i = 0; i < 5; i = i + 1) {
        let t = taps[i];
        sum = sum + textureSampleLevel(history, bilinear, t.xy, 0.0).rgb * t.z;
        weight = weight + t.z;
    }
    return max(sum / max(weight, 1.0e-4), vec3<f32>(0.0));
}

// The history pulled towards the box's centre until it is inside it (clipping, not clamping: the colour keeps its hue).
fn clip_to_box(c: vec3<f32>, lo: vec3<f32>, hi: vec3<f32>) -> vec3<f32> {
    let centre = 0.5 * (hi + lo);
    let extent = 0.5 * (hi - lo) + vec3<f32>(1.0e-4);
    let v = c - centre;
    let units = abs(v / extent);
    let most = max(units.x, max(units.y, units.z));
    if most > 1.0 {
        return centre + v / most;
    }
    return c;
}

@fragment
fn fs_taa(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(current));
    let p = vec2<i32>(floor(frag.xy));
    let uv = (vec2<f32>(p) + 0.5) * params.texel.xy;
    let now = textureLoad(current, p, 0).rgb;
    // The neighbourhood: its mean and spread (in YCoCg) and the nearest depth, whose motion is followed (so an edge in front
    // brings its own move with it).
    var m1 = vec3<f32>(0.0);
    var m2 = vec3<f32>(0.0);
    var nearest = 1.0;
    var nearest_at = p;
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            let q = clamp(p + vec2<i32>(i, j), vec2<i32>(0), size - 1);
            let c = to_ycocg(textureLoad(current, q, 0).rgb);
            m1 = m1 + c;
            m2 = m2 + c * c;
            let d = textureLoad(depth, q, 0);
            if d < nearest {
                nearest = d;
                nearest_at = q;
            }
        }
    }
    let mean = m1 / 9.0;
    let spread = sqrt(max(m2 / 9.0 - mean * mean, vec3<f32>(0.0)));
    let moved = textureLoad(motion, nearest_at, 0);
    var before_uv: vec2<f32>;
    if moved.a > 0.5 {
        before_uv = uv - moved.xy;
    } else {
        // (nothing drawn there that moves of itself: where this point of the view was a frame ago)
        let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, nearest, 1.0);
        let world = params.inverse_vp * ndc;
        let before = params.previous_vp * vec4<f32>(world.xyz / world.w, 1.0);
        before_uv = vec2<f32>(before.x / before.w * 0.5 + 0.5, 0.5 - before.y / before.w * 0.5);
    }
    let outside = any(before_uv < vec2<f32>(0.0)) || any(before_uv > vec2<f32>(1.0));
    if params.texel.z < 0.5 || outside {
        return vec4<f32>(now, 1.0);
    }
    // The faster the move, the more the history is resampled and the softer it gets: less of it is kept.
    let speed = length((uv - before_uv) / params.texel.xy);
    let reactive = textureLoad(motion, p, 0).b;
    let share = clamp(max(params.texel.w + speed * 0.02, reactive * 0.4), params.texel.w, 0.6);
    let gamma = mix(1.25, 0.75, clamp(speed / 4.0, 0.0, 1.0));
    let lo = mean - spread * gamma;
    let hi = mean + spread * gamma;
    let past = from_ycocg(clip_to_box(to_ycocg(history_at(before_uv)), lo, hi));
    // (blended by weights that undo each colour's brightness, so a bright speck doesn't streak)
    let wn = share / (1.0 + dot(now, vec3<f32>(0.299, 0.587, 0.114)));
    let wp = (1.0 - share) / (1.0 + dot(past, vec3<f32>(0.299, 0.587, 0.114)));
    return vec4<f32>((now * wn + past * wp) / (wn + wp), 1.0);
}
