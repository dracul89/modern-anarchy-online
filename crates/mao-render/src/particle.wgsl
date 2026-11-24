// Camera-facing soft sprites, added to what is already drawn (nano effects: sparks, glows). Each instance is a point, a size
// (world units) and a colour; the corners come from the vertex index and the camera's right and up vectors. An instance with
// its own right and up (a quad laid in the world, the original's `GfxVisualVolGrid` slices) uses those as half extents instead,
// and its size is a flag: 1 fades it by how squarely it faces the camera (`GfxVisualVolGrid`'s flag 0x4000), 0 does not. An
// instance with an up but no right (a quad standing on its up, the original's `GfxVisualSprite3` turned to the camera) turns
// about that up to face the eye, its size the half width. A billboard turns in its own plane by its spin (right.w, radians).

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
    ambient: vec4<f32>,
    // rgb: the frame's fog colour, as shown; w: 1 when there is a fog (`globals.wgsl`); its start is `eye.w`, its end
    // `horizon.w`.
    fog: vec4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var sheet: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct In {
    @builtin(vertex_index) corner: u32,
    // xyz: the centre; w: the size (the sprite's width), the facing flag of a laid quad, or the half width of a standing one.
    @location(0) centre: vec4<f32>,
    @location(1) colour: vec4<f32>,
    // The part of the sprite sheet to show: left, top, width and height (a width of 0: no sheet, a soft dot).
    @location(2) rect: vec4<f32>,
    // The half extents of a laid quad (both zero: a billboard; right alone zero: a standing quad); w a billboard's spin.
    @location(3) right: vec4<f32>,
    // (w: a billboard's height when it is not its width; 0: as tall as wide)
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
    let has_right = any(in.right.xyz != vec3(0.0));
    let has_up = any(in.up.xyz != vec3(0.0));
    var world: vec3<f32>;
    var colour = in.colour;
    if has_right {
        world = in.centre.xyz + in.right.xyz * c.x + in.up.xyz * c.y;
        if in.centre.w > 0.5 {
            // (FUN_1002f69e: the alpha times the slice's unit normal dotted with the direction from the eye, made positive)
            let n = cross(in.right.xyz, in.up.xyz);
            let d = in.centre.xyz - globals.eye.xyz;
            if length(n) > 0.0 && length(d) > 0.0 {
                colour.a = colour.a * abs(dot(normalize(n), normalize(d)));
            }
        }
    } else if has_up {
        // (FUN_100f6d2b, flag 0x2000: the quad turned about its up so its face points at the eye)
        let to_eye = globals.eye.xyz - in.centre.xyz;
        var side = cross(in.up.xyz, to_eye);
        if length(side) == 0.0 {
            side = globals.right.xyz;
        }
        world = in.centre.xyz + normalize(side) * c.x * in.centre.w + in.up.xyz * c.y;
    } else {
        // (GfxVisualSol's draw, FUN_10020836: a growing angle turns the quad's up toward the view's right, clockwise)
        let cs = cos(in.right.w);
        let sn = sin(in.right.w);
        // (a width and a height of its own: the original's smoke sprites, FUN_10025391)
        let tall = select(in.centre.w, in.up.w, in.up.w > 0.0);
        let q = vec2(c.x * in.centre.w, c.y * tall);
        let turned = vec2(q.x * cs + q.y * sn, q.y * cs - q.x * sn);
        world = in.centre.xyz + (globals.right.xyz * turned.x + globals.up.xyz * turned.y) * 0.5;
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

// A sprite blended over the scene by its alpha (the original's sprites of a visual that is not additive: smoke): the
// texture's colour and alpha times the sprite's, as the effects' texture stage modulates both (`gfxvisual.rs
// texture_stages`). A soft dot fades to its edge by its alpha.
@fragment
fn fs_particle_blend(in: Out) -> @location(0) vec4<f32> {
    if in.flat_dot > 0.5 {
        let r2 = dot(in.offset, in.offset);
        if r2 >= 1.0 {
            discard;
        }
        let falloff = (1.0 - r2) * (1.0 - r2) * (1.0 - r2);
        return vec4(in.colour.rgb, clamp(in.colour.a * falloff, 0.0, 1.0));
    }
    let texel = textureSample(sheet, samp, in.uv);
    return vec4(clamp(texel.rgb * in.colour.rgb, vec3(0.0), vec3(1.0)), clamp(texel.a * in.colour.a, 0.0, 1.0));
}

// The shells (the original's `GfxVisualShield`): textured triangles, the texture times the vertex colour, added (its alpha
// scaling the colour) or blended by its alpha.
struct ShellIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
}

struct ShellOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    // A backdrop's depth along the view (the fog's measure); 0 for a shell.
    @location(2) depth: f32,
}

@vertex
fn vs_shell(in: ShellIn) -> ShellOut {
    var out: ShellOut;
    out.position = globals.view_proj * vec4(in.position, 1.0);
    out.uv = in.uv;
    out.colour = in.colour;
    out.depth = 0.0;
    return out;
}

// A backdrop's vertex (the effect scripts' sky): drawn behind everything, its depth held at the far plane so a dome bigger
// than the view is not cut away.
@vertex
fn vs_backdrop(in: ShellIn) -> ShellOut {
    var out: ShellOut;
    let p = globals.view_proj * vec4(in.position, 1.0);
    out.position = vec4(p.x, p.y, p.w * 0.999999, p.w);
    out.uv = in.uv;
    out.colour = in.colour;
    out.depth = p.w;
    return out;
}

// The backdrops are blended as Direct3D blended them: in the colours as shown, not as light. The sky is layers added
// over one another (the atmosphere, the vortex, its top, the suns' rays and flares), and two glows of a half added as
// shown colours are white, where added as light they stay a grey. The picture keeps light, so while the backdrops are
// drawn it holds the square roots of the light instead (close to the shown colours: the sRGB curve is near a power of
// 2.2), the blending adds and mixes those, and `fs_backdrop_resolve` squares the picture back before the world is
// drawn (`fs_sky` writes roots too when backdrops follow). A colour drawn alone comes out exactly as the script names
// it; the blends are those of a display of gamma 2 rather than sRGB's.
fn shown_of(light: vec3<f32>) -> vec3<f32> {
    let x = max(light, vec3<f32>(0.0));
    return select(1.055 * pow(x, vec3<f32>(1.0 / 2.4)) - 0.055, x * 12.92, x <= vec3<f32>(0.0031308));
}

fn kept(shown: vec3<f32>) -> vec3<f32> {
    let x = max(shown, vec3<f32>(0.0));
    return sqrt(select(pow((x + 0.055) / 1.055, vec3<f32>(2.4)), x / 12.92, x <= vec3<f32>(0.04045)));
}

// The texture times the diffuse colour, as shown colours, clamped as the fixed-function stages clamp.
fn backdrop_texel(in: ShellOut) -> vec4<f32> {
    let t = textureSample(sheet, samp, in.uv);
    return clamp(vec4(shown_of(t.rgb), t.a) * in.colour, vec4(0.0), vec4(1.0));
}

// A colour faded into the frame's fog by its depth, as Direct3D's vertex fog did it: linear from the fog's start to its
// end by the depth along the view, the fog's colour mixed in as a shown colour, before the blending.
fn backdrop_fogged(shown: vec3<f32>, depth: f32) -> vec3<f32> {
    let near = globals.eye.w;
    let far = globals.horizon.w;
    let amount = clamp((depth - near) / max(far - near, 1.0), 0.0, 1.0) * globals.fog.w;
    return mix(shown, globals.fog.rgb, amount);
}

@fragment
fn fs_backdrop_opaque(in: ShellOut) -> @location(0) vec4<f32> {
    return vec4(kept(backdrop_texel(in).rgb), 1.0);
}

@fragment
fn fs_backdrop_blend(in: ShellOut) -> @location(0) vec4<f32> {
    let t = backdrop_texel(in);
    return vec4(kept(t.rgb), t.a);
}

@fragment
fn fs_backdrop_add(in: ShellOut) -> @location(0) vec4<f32> {
    let t = backdrop_texel(in);
    return vec4(kept(t.rgb) * t.a, 0.0);
}

// A backdrop added whole (`SRCBLEND ONE`, `DESTBLEND ONE`): the texture times the diffuse colour, its alpha no weight.
@fragment
fn fs_backdrop_add_one(in: ShellOut) -> @location(0) vec4<f32> {
    return vec4(kept(backdrop_texel(in).rgb), 0.0);
}

@fragment
fn fs_backdrop_opaque_fogged(in: ShellOut) -> @location(0) vec4<f32> {
    return vec4(kept(backdrop_fogged(backdrop_texel(in).rgb, in.depth)), 1.0);
}

@fragment
fn fs_backdrop_blend_fogged(in: ShellOut) -> @location(0) vec4<f32> {
    let t = backdrop_texel(in);
    return vec4(kept(backdrop_fogged(t.rgb, in.depth)), t.a);
}

// The picture squared back to light after the backdrops: one triangle over the screen, the blending multiplying what
// is there by itself (the colour written counts for nothing).
@vertex
fn vs_backdrop_resolve(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 1.0, 1.0);
}

@fragment
fn fs_backdrop_resolve() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}

@fragment
fn fs_shell_add(in: ShellOut) -> @location(0) vec4<f32> {
    // (clamped as the fixed-function stages clamp: `GfxVisualShield2`'s colours are doubled for its `MODULATE2X`)
    let texel = clamp(textureSample(sheet, samp, in.uv) * in.colour, vec4(0.0), vec4(1.0));
    return vec4(texel.rgb * texel.a, 0.0);
}

@fragment
fn fs_shell_blend(in: ShellOut) -> @location(0) vec4<f32> {
    return clamp(textureSample(sheet, samp, in.uv) * in.colour, vec4(0.0), vec4(1.0));
}
