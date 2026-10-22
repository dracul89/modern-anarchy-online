struct Globals {
    view_proj: mat4x4<f32>,
    light: vec4<f32>,
    // xyz: the player (world, Z up); w: seconds since the renderer started.
    player: vec4<f32>,
    // xyz: the eye; w: where the fog starts.
    eye: vec4<f32>,
    // rgb: the sky at the horizon, which is also the fog colour; w: where the fog ends.
    horizon: vec4<f32>,
    // rgb: the sky overhead.
    zenith: vec4<f32>,
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

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var base = part.colour;
    if part.flags.x > 0.5 {
        let t = textureSample(tex, samp, in.uv);
        if t.a < 0.5 && part.flags.y < 0.5 {
            discard;
        }
        base = vec4<f32>(base.rgb * t.rgb, 1.0);
    }
    let n = normalize(in.normal);
    // Lit from both sides: the winding of the original's meshes isn't
    // relied on. Sky light from above, a warmer bounce from below, and a sun.
    let diffuse = abs(dot(n, normalize(globals.light.xyz)));
    let ambient = mix(vec3<f32>(0.34, 0.32, 0.30), vec3<f32>(0.60, 0.66, 0.76), n.z * 0.5 + 0.5);
    var colour = base.rgb * (ambient + vec3<f32>(0.60, 0.56, 0.48) * diffuse);
    // Distance fades into the horizon's colour.
    let far = globals.horizon.w;
    let near = globals.eye.w;
    let fog = clamp((distance(in.world, globals.eye.xyz) - near) / max(far - near, 1.0), 0.0, 1.0);
    colour = mix(colour, globals.horizon.rgb, fog * fog * (3.0 - 2.0 * fog));
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
    return vec4<f32>(mix(globals.horizon.rgb, globals.zenith.rgb, pow(t, 0.8)), 1.0);
}
