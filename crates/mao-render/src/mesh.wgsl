struct Globals {
    view_proj: mat4x4<f32>,
    light: vec4<f32>,
}

struct Part {
    // Direct3D's row-vector matrix, uploaded as it is: read as columns it
    // is the same transform for column vectors.
    model: mat4x4<f32>,
    colour: vec4<f32>,
    // x: 1 when the part has a texture (alpha-tested).
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
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world = part.model * vec4<f32>(v.position, 1.0);
    out.clip = globals.view_proj * world;
    out.uv = v.uv;
    out.normal = (part.model * vec4<f32>(v.normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var base = part.colour;
    if part.flags.x > 0.5 {
        let t = textureSample(tex, samp, in.uv);
        if t.a < 0.5 {
            discard;
        }
        base = vec4<f32>(base.rgb * t.rgb, 1.0);
    }
    let n = normalize(in.normal);
    // Lit from both sides: the winding of the original's meshes isn't
    // relied on.
    let diffuse = abs(dot(n, normalize(globals.light.xyz)));
    return vec4<f32>(base.rgb * (0.45 + 0.75 * diffuse), 1.0);
}
