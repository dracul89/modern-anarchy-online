struct Globals {
    view_proj: mat4x4<f32>,
    // The sun's view and projection (the shadow map's).
    light_vp: mat4x4<f32>,
    light: vec4<f32>,
    // xyz: the player (world, Z up); w: seconds since the renderer started.
    player: vec4<f32>,
    // xyz: the eye; w: where the fog starts.
    eye: vec4<f32>,
    // rgb: the sky at the horizon, which is also the fog colour; w: where the fog ends.
    horizon: vec4<f32>,
    // rgb: the sky overhead.
    zenith: vec4<f32>,
    // x: 1 when shadows are drawn; y: a shadow texel in world units;
    // z: the water's level; w: 1 when there is water.
    params: vec4<f32>,
    // The water's box: min x, min y, max x, max y.
    rect: vec4<f32>,
    // The camera's right and up (world, Z up), for sprites that face it.
    right: vec4<f32>,
    up: vec4<f32>,
    // rgb: the area's ambient light (kept per channel at its maximum against the sky light); w: the day's intensity.
    ambient: vec4<f32>,
    // rgb: the fog's colour; w: 1 when it is an area's fog (not scaled by the day: it was scaled by the fog light already).
    fog: vec4<f32>,
}

// The day's light: the sky light, the sun and the sky keep a fifth at night so the world stays visible (the original's
// night comes from its own night sky and lights, which are not ported).
fn day() -> f32 {
    return mix(0.2, 1.0, globals.ambient.w);
}

// The fog's colour at the horizon: the sky's by day, an area's as it is.
fn fog_colour() -> vec3<f32> {
    return mix(globals.horizon.rgb * day(), globals.fog.rgb, globals.fog.w);
}

struct Part {
    // Direct3D's row-vector matrix, uploaded as it is: read as columns it
    // is the same transform for column vectors.
    model: mat4x4<f32>,
    colour: vec4<f32>,
    // x: 1 when the part has a texture (alpha-tested); y: 1 to ignore the
    // texture's alpha; z: 1 for foliage; w: the part's height (foliage).
    flags: vec4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> part: Part;
@group(1) @binding(1) var tex: texture_2d<f32>;
@group(1) @binding(2) var samp: sampler;
@group(2) @binding(0) var shadow_map: texture_depth_2d;
@group(2) @binding(1) var shadow_samp: sampler_comparison;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world: vec3<f32>,
}

// Plants move: a slow sway in the wind (more towards the top) and a bend
// away from the player when it walks through them. The base stays put.
fn bend(world: vec3<f32>) -> vec3<f32> {
    let height = max(part.flags.w, 0.3);
    let base = part.model[3].z;
    let up = clamp((world.z - base) / height, 0.0, 1.0);
    let weight = up * up;
    let t = globals.player.w;
    let phase = world.x * 0.31 + world.y * 0.23;
    let sway = vec2<f32>(sin(t * 1.6 + phase), cos(t * 1.25 + phase * 1.4)) * (0.03 * min(height, 8.0)) * weight;
    var offset = sway;
    let away = world.xy - globals.player.xy;
    let distance = length(away);
    let reach = 3.0;
    if distance < reach && abs(world.z - globals.player.z) < 4.0 {
        let push = 1.0 - distance / reach;
        // Tall trees hardly give; low plants lie over.
        let give = min(height * 0.9, 1.5) * min(1.0, 3.0 / height);
        offset = offset + normalize(away + vec2<f32>(0.0001, 0.0)) * (push * give * weight);
    }
    return vec3<f32>(world.xy + offset, world.z - 0.15 * length(offset) * weight);
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    var world = (part.model * vec4<f32>(v.position, 1.0)).xyz;
    if part.flags.z > 0.5 {
        world = bend(world);
    }
    out.clip = globals.view_proj * vec4<f32>(world, 1.0);
    out.uv = v.uv;
    out.normal = (part.model * vec4<f32>(v.normal, 0.0)).xyz;
    out.world = world;
    return out;
}

// How much of the sun reaches a point: 1 in the light, 0 in shadow (a soft
// 3x3 comparison, the point nudged along its normal against acne).
fn shadow_at(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if globals.params.x < 0.5 {
        return 1.0;
    }
    let sun = normalize(globals.light.xyz);
    let facing = select(-1.0, 1.0, dot(n, sun) >= 0.0);
    let p = world + n * (facing * globals.params.y * 2.0);
    let clip = globals.light_vp * vec4<f32>(p, 1.0);
    let uv = vec2<f32>(clip.x * 0.5 + 0.5, 0.5 - clip.y * 0.5);
    let step = 1.0 / 4096.0;
    var sum = 0.0;
    for (var i = -1; i <= 1; i = i + 1) {
        for (var j = -1; j <= 1; j = j + 1) {
            sum = sum + textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(f32(i), f32(j)) * step, clip.z - 0.0005);
        }
    }
    let lit = sum / 9.0;
    // Outside the map there are no shadows; they fade out towards its edge.
    let edge = max(abs(clip.x), abs(clip.y));
    let inside = 1.0 - smoothstep(0.85, 1.0, edge);
    return mix(1.0, lit, inside * select(0.0, 1.0, clip.z > 0.0 && clip.z < 1.0));
}

// The depth-only pass: plants cut out as they are in the main pass.
@fragment
fn fs_shadow(in: VertexOut) {
    if part.flags.x > 0.5 && part.flags.y < 0.5 {
        let t = textureSampleLevel(tex, samp, in.uv, 0.0);
        if t.a < 0.5 {
            discard;
        }
    }
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var base = part.colour;
    // Baked light in the texture's alpha (flags.y = 2): added to the lighting, as the original's texture stages add it to the
    // diffuse before the colour is multiplied in.
    var baked = 0.0;
    if part.flags.x > 0.5 {
        let t = textureSample(tex, samp, in.uv);
        if t.a < 0.5 && part.flags.y < 0.5 {
            discard;
        }
        if part.flags.y > 1.5 {
            baked = t.a;
        }
        base = vec4<f32>(base.rgb * t.rgb, 1.0);
    }
    let n = normalize(in.normal);
    // Lit from both sides: the winding of the original's meshes isn't
    // relied on. Sky light from above, a warmer bounce from below, and a sun.
    let sun = normalize(globals.light.xyz);
    let diffuse = abs(dot(n, sun)) * shadow_at(in.world, n);
    let d = day();
    // The area's ambient light is added as the original's `AddAmbientLight` adds it: each channel at its maximum.
    let ambient = max(mix(vec3<f32>(0.34, 0.32, 0.30), vec3<f32>(0.60, 0.66, 0.76), n.z * 0.5 + 0.5) * d, globals.ambient.rgb);
    var colour = base.rgb * min(ambient + vec3<f32>(0.60, 0.56, 0.48) * d * diffuse + vec3<f32>(baked), vec3<f32>(1.0));
    // Distance fades into the fog's colour (linear when the fog is an area's, as Direct3D's was).
    let far = globals.horizon.w;
    let near = globals.eye.w;
    let fog = clamp((distance(in.world, globals.eye.xyz) - near) / max(far - near, 1.0), 0.0, 1.0);
    let amount = mix(fog * fog * (3.0 - 2.0 * fog), fog, globals.fog.w);
    colour = mix(colour, fog_colour(), amount);
    return vec4<f32>(colour, 1.0);
}

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) height: f32,
}

// One triangle over the whole screen.
@vertex
fn vs_sky(@builtin(vertex_index) index: u32) -> SkyOut {
    var out: SkyOut;
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    out.clip = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 1.0, 1.0);
    out.height = y * 2.0 - 1.0;
    return out;
}

@fragment
fn fs_sky(in: SkyOut) -> @location(0) vec4<f32> {
    let t = clamp(in.height * 0.5 + 0.5, 0.0, 1.0);
    // An area's fog hides the horizon in its colour (the lower half of the screen is at or under it), thinning towards the
    // zenith; the sky above is the day's.
    let sky = mix(globals.horizon.rgb, globals.zenith.rgb, pow(t, 0.8)) * day();
    return vec4<f32>(mix(sky, fog_colour(), globals.fog.w * (1.0 - smoothstep(0.5, 1.0, t))), 1.0);
}

struct WaterOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    // The liquid type (the playfield's water polygons): 0 water, 2 toxic, 4 lava, 6 the fourth; odd ones fall.
    @location(1) kind: f32,
}

// The water's box as two triangles at its level.
@vertex
fn vs_water(@builtin(vertex_index) index: u32) -> WaterOut {
    var corners = array<vec2<f32>, 6>(vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0));
    let c = corners[index];
    let xy = mix(globals.rect.xy, globals.rect.zw, c);
    var out: WaterOut;
    out.world = vec3<f32>(xy, globals.params.z);
    out.clip = globals.view_proj * vec4<f32>(out.world, 1.0);
    out.kind = 0.0;
    return out;
}

// The playfield's own water triangles.
@vertex
fn vs_water_mesh(@location(0) position: vec3<f32>, @location(1) kind: f32) -> WaterOut {
    var out: WaterOut;
    out.world = position;
    out.clip = globals.view_proj * vec4<f32>(position, 1.0);
    out.kind = kind;
    return out;
}

// The waves at a point: two crossing swells and a ripple (the fine ones
// scaled by `detail`). x: the height; yz: its slope along x and y.
fn wave(p: vec2<f32>, t: f32, detail: f32) -> vec3<f32> {
    let a = 0.9 * p.x + 1.3 * t;
    let b = 1.1 * p.y - t + 0.3 * p.x;
    let c = 2.3 * (p.x + p.y) + 2.1 * t;
    let d = 3.7 * (p.x - p.y) - 2.7 * t;
    let h = 0.5 * sin(a) + 0.5 * sin(b) + detail * (0.25 * sin(c) + 0.15 * sin(d));
    let dx = 0.45 * cos(a) + 0.15 * cos(b) + detail * (0.575 * cos(c) + 0.555 * cos(d));
    let dy = 0.55 * cos(b) + detail * (0.575 * cos(c) - 0.555 * cos(d));
    return vec3<f32>(h, dx, dy);
}

@fragment
fn fs_water(in: WaterOut) -> @location(0) vec4<f32> {
    let t = globals.player.w;
    let dist = distance(in.world, globals.eye.xyz);
    // The fine ripples fade with distance (they would only shimmer).
    let detail = 1.0 - smoothstep(10.0, 45.0, dist);
    let w = wave(in.world.xy, t, detail);
    let h = w.x;
    // Past a while even the swells would alias: the surface flattens into a mirror.
    let calm = 1.0 - smoothstep(30.0, 220.0, dist);
    let n = normalize(vec3<f32>(-w.y, -w.z, 0.0) * (0.12 * calm) + vec3<f32>(0.0, 0.0, 1.0));
    let view = normalize(globals.eye.xyz - in.world);
    let fresnel = 0.04 + 0.96 * pow(1.0 - max(dot(n, view), 0.0), 5.0);
    let r = reflect(-view, n);
    let d = day();
    let sky = mix(globals.horizon.rgb, globals.zenith.rgb, pow(clamp(r.z, 0.0, 1.0), 0.6)) * d;
    // The liquid's colours by its layer (the type over two): water, toxic, lava, the fourth.
    let layer = u32(in.kind + 0.5) / 2u;
    var deep = vec3<f32>(0.03, 0.20, 0.27);
    var shallow = vec3<f32>(0.10, 0.42, 0.45);
    if layer == 1u {
        deep = vec3<f32>(0.05, 0.22, 0.06);
        shallow = vec3<f32>(0.16, 0.45, 0.10);
    } else if layer == 2u {
        deep = vec3<f32>(0.45, 0.08, 0.0);
        shallow = vec3<f32>(0.9, 0.35, 0.05);
    } else if layer == 3u {
        deep = vec3<f32>(0.18, 0.14, 0.08);
        shallow = vec3<f32>(0.32, 0.26, 0.16);
    }
    deep = deep * d;
    shallow = shallow * d;
    var colour = mix(mix(deep, shallow, 0.5 + 0.5 * h * 0.5), sky, fresnel);
    let sun = normalize(globals.light.xyz);
    colour = colour + vec3<f32>(1.0, 0.92, 0.75) * d * pow(max(dot(r, sun), 0.0), 140.0) * 1.5;
    let far = globals.horizon.w;
    let near = globals.eye.w;
    let fog = clamp((distance(in.world, globals.eye.xyz) - near) / max(far - near, 1.0), 0.0, 1.0);
    let amount = mix(fog * fog * (3.0 - 2.0 * fog), fog, globals.fog.w);
    colour = mix(colour, fog_colour(), amount);
    return vec4<f32>(colour, mix(0.62, 0.94, fresnel));
}
