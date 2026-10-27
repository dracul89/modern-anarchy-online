// Camera-facing soft sprites, added to what is already drawn (nano effects: sparks, glows). Each instance is a point, a size
// (world units) and a colour; the corners come from the vertex index and the camera's right and up vectors. An instance with
// its own right and up (a quad laid in the world, the original's `GfxVisualVolGrid` slices) uses those as half extents instead,
// and its size is a flag: 1 fades it by how squarely it faces the camera (`GfxVisualVolGrid`'s flag 0x4000), 0 does not.

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
@group(1) @binding(0) var sheet: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct In {
    @builtin(vertex_index) corner: u32,
    // xyz: the centre; w: the size (the sprite's width), or the facing flag of a laid quad.
    @location(0) centre: vec4<f32>,
    @location(1) colour: vec4<f32>,
    // The part of the sprite sheet to show: left, top, width and height (a width of 0: no sheet, a soft dot).
    @location(2) rect: vec4<f32>,
    // The half extents of a laid quad (both zero: a billboard).
    @location(3) right: vec4<f32>,
    @location(4) up: vec4<f32>,
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) flat_dot: f32,
}

@vertex
fn vs_particle(in: In) -> Out {
    // two triangles: (-1,-1) (1,-1) (-1,1) (-1,1) (1,-1) (1,1)
    var corners = array<vec2<f32>, 6>(vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0), vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0));
    let c = corners[in.corner];
    let laid = any(in.right.xyz != vec3(0.0)) || any(in.up.xyz != vec3(0.0));
    var world: vec3<f32>;
    var colour = in.colour;
    if laid {
        world = in.centre.xyz + in.right.xyz * c.x + in.up.xyz * c.y;
        if in.centre.w > 0.5 {
            // (FUN_1002f69e: the alpha times the slice's unit normal dotted with the direction from the eye, made positive)
            let n = cross(in.right.xyz, in.up.xyz);
            let d = in.centre.xyz - globals.eye.xyz;
            if length(n) > 0.0 && length(d) > 0.0 {
                colour.a = colour.a * abs(dot(normalize(n), normalize(d)));
            }
        }
    } else {
        world = in.centre.xyz + (globals.right.xyz * c.x + globals.up.xyz * c.y) * in.centre.w * 0.5;
    }
    var out: Out;
    out.position = globals.view_proj * vec4(world, 1.0);
    out.offset = c;
    out.colour = colour;
    out.uv = in.rect.xy + vec2(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5) * in.rect.zw;
    out.flat_dot = select(0.0, 1.0, in.rect.z == 0.0);
    return out;
}

@fragment
fn fs_particle(in: Out) -> @location(0) vec4<f32> {
    // added to the scene: the colour is already multiplied by its strength
    if in.flat_dot > 0.5 {
        let r2 = dot(in.offset, in.offset);
        if r2 >= 1.0 {
            discard;
        }
        // a bright core and a soft edge
        let falloff = (1.0 - r2) * (1.0 - r2) * (1.0 - r2);
        return vec4(in.colour.rgb * in.colour.a * falloff, 0.0);
    }
    let texel = textureSample(sheet, samp, in.uv);
    return vec4(texel.rgb * texel.a * in.colour.rgb * in.colour.a, 0.0);
}
