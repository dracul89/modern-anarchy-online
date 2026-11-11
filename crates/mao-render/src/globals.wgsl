// The frame's globals, shared by every scene shader (`mesh.wgsl`, `liquid.wgsl`); the Rust side is `Globals` in lib.rs.
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
    // The effect scripts' lighting (the environment light's): rgb the ground light's colour, w 1 when it is given; and the
    // ambient light's colour.
    script_sun: vec4<f32>,
    script_ambient: vec4<f32>,
    // For the motion buffer (temporal anti-aliasing): this frame's view and projection without the jitter, the last frame's,
    // and the player and time the plants bent by then.
    current_vp: mat4x4<f32>,
    previous_vp: mat4x4<f32>,
    previous_player: vec4<f32>,
    // x: the textures' mip bias (the logarithm of the drawing scale: 0 at the window's size, -1 at a half).
    detail: vec4<f32>,
}

// Whether the effect scripts light the world (their colours already follow the day).
fn scripted() -> bool {
    return globals.script_sun.w > 0.5;
}

// The day's light: the sky light, the sun and the sky keep a fifth at night so the world stays visible (the original's
// night comes from its own night sky and lights, which are not ported).
fn day() -> f32 {
    return mix(0.2, 1.0, globals.ambient.w);
}

// The fog's colour at the horizon: the sky's by day, an area's as it is.
fn fog_colour() -> vec3<f32> {
    var haze = globals.horizon.rgb * day();
    if scripted() {
        haze = globals.horizon.rgb;
    }
    return mix(haze, globals.fog.rgb, globals.fog.w);
}

@group(0) @binding(0) var<uniform> globals: Globals;
