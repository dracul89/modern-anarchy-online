// Camera-facing soft sprites, added to what is already drawn (nano effects: sparks, glows). Each instance is a point, a size
// (world units) and a colour; the corners come from the vertex index and the camera's right and up vectors.

struct Globals {
    view_proj: mat4x4<f32>,
    light_vp: mat4x4<f32>,
    light: vec4<f32>,
    player: vec4<f32>,
    eye: vec4<f32>,
    horizon: vec4<f32>,
    zenith: vec4<f32>,
    params: vec4<f32>,
    rect: vec4<f32>,
    // The camera's right and up (world, Z up).
    right: vec4<f32>,
    up: vec4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct In {
    @builtin(vertex_index) corner: u32,
    // xyz: the centre; w: the size (the sprite's width).
    @location(0) centre: vec4<f32>,
    @location(1) colour: vec4<f32>,
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec2<f32>,
    @location(1) colour: vec4<f32>,
}

@vertex
fn vs_particle(in: In) -> Out {
    // two triangles: (-1,-1) (1,-1) (-1,1) (-1,1) (1,-1) (1,1)
    var corners = array<vec2<f32>, 6>(vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0), vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0));
    let c = corners[in.corner];
    let world = in.centre.xyz + (globals.right.xyz * c.x + globals.up.xyz * c.y) * in.centre.w * 0.5;
    var out: Out;
    out.position = globals.view_proj * vec4(world, 1.0);
    out.offset = c;
    out.colour = in.colour;
    return out;
}

@fragment
fn fs_particle(in: Out) -> @location(0) vec4<f32> {
    let r2 = dot(in.offset, in.offset);
    if r2 >= 1.0 {
        discard;
    }
    // a bright core and a soft edge
    let falloff = (1.0 - r2) * (1.0 - r2) * (1.0 - r2);
    return vec4(in.colour.rgb, in.colour.a * falloff);
}
