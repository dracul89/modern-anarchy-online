// The liquid surfaces as the original's `VisualLiquid_t` draws them (DisplaySystem FUN_1003f3db and its passes, ported in
// `AnarchyReimagined`'s `dropin/displaysystem/src/fxliquid.rs`), after `globals.wgsl`. Each surface's triangles carry
// their type's colour (`mao_formats::liquid::colour`); the passes texture them by their world place:
//
// - water (layer 0): `water5.png` over 40-unit tiles, drifting with the flow phase, lit by the water light (the vertex
//   colour as the diffuse, a black ambient, a white specular of 0.505 at power 10); then a second pass of the same
//   texture rippling down the slopes, blended `1 − α` over `α` (so a flat sheet shows a fifth of it);
// - toxic (layer 2): `toxicwater.png` over 12-unit tiles, unlit, in the vertex colour; a second pass stepping down the
//   slopes by their signs;
// - lava (layer 1): the water texture over 8-unit tiles in the vertex colour (red, opaque); then `env_lava2.png` twice,
//   over 75 and 250 units, drifting, its alpha the product of both (the original's camera-space projection comes to the
//   same world tiling);
// - the wide water (layer 4): the water texture over 160-unit tiles, lit with an ambient tint of (0.2, 0.3, 0.4) and no
//   specular.
//
// The frame block's wave offsets (`GameWaveCurve0..15`) are 0 in every script, so the waves they would add are left out.

struct Liquid {
    // rgb: the water light's diffuse colour (its brightest channel as grey, `set_light(brightest)`).
    light: vec4<f32>,
    // rgb: the light's specular colour (the water light times the scripts' `SpecularLightIntensity`).
    specular: vec4<f32>,
    // xyz: the way to the light (world, Z up).
    to_light: vec4<f32>,
    // rgb: the ambient light.
    ambient: vec4<f32>,
    // x: the water's flow phase; y, z, w: the water, toxic and lava ripple phases.
    phases: vec4<f32>,
    // x, y: the lava texture's two drifts.
    drift: vec4<f32>,
    // x: which pass (see `fs_liquid`).
    which: vec4<u32>,
    // x, y: the picture's size in pixels; z: the camera's tilt (its forward's up component): the refraction pass; w: the surfaces'
    // raise (0.2 with the camera under a liquid).
    screen: vec4<f32>,
}

@group(1) @binding(0) var<uniform> liquid: Liquid;
@group(1) @binding(1) var water_tex: texture_2d<f32>;
@group(1) @binding(2) var toxic_tex: texture_2d<f32>;
@group(1) @binding(3) var lava_tex: texture_2d<f32>;
@group(1) @binding(4) var liquid_samp: sampler;
// The picture so far (the renderer's screen copy), clamped and unfiltered.
@group(1) @binding(5) var screen_tex: texture_2d<f32>;
@group(1) @binding(6) var screen_samp: sampler;

struct LiquidOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec4<f32>,
}

@vertex
fn vs_liquid(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) colour: vec4<f32>) -> LiquidOut {
    var out: LiquidOut;
    // (the surfaces raised while the camera is under a liquid: `VisualLiquid_t`'s frame moved up 0.2)
    let raised = vec3<f32>(position.xy, position.z + liquid.screen.w);
    out.world = raised;
    out.clip = globals.view_proj * vec4<f32>(raised, 1.0);
    out.normal = normal;
    out.colour = colour;
    return out;
}

// A slope's ripple direction (FUN_10005229): the normal's horizontal part made a thousand times larger, the whole made unit.
fn slope(n: vec3<f32>) -> vec2<f32> {
    return normalize(vec3<f32>(n.x * 1000.0, n.y * 1000.0, n.z)).xy;
}

// The toxic ripple's step along an axis: the normal's component past 0.2 either way.
fn step_sign(c: f32) -> f32 {
    if c > 0.2 {
        return 1.0;
    }
    if c < -0.2 {
        return -1.0;
    }
    return 0.0;
}

// Direct3D's lit colour for a liquid vertex: the ambient light on the material's tint, the water light on the vertex
// colour by the slope it meets, clamped; the specular is added after the texture.
fn lit(normal: vec3<f32>, colour: vec3<f32>, tint: vec3<f32>) -> vec3<f32> {
    let n = normalize(normal);
    let l = normalize(liquid.to_light.xyz);
    let diffuse = liquid.light.rgb * colour * max(dot(n, l), 0.0);
    return clamp(liquid.ambient.rgb * tint + diffuse, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn glint(world: vec3<f32>, normal: vec3<f32>, strength: f32, shininess: f32) -> vec3<f32> {
    let n = normalize(normal);
    let l = normalize(liquid.to_light.xyz);
    let v = normalize(globals.eye.xyz - world);
    let h = normalize(l + v);
    return liquid.specular.rgb * strength * pow(max(dot(n, h), 0.0), shininess);
}

// The distance fog, as the meshes have it.
fn fogged(colour: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let far = globals.horizon.w;
    let near = globals.eye.w;
    let fog = clamp((distance(world, globals.eye.xyz) - near) / max(far - near, 1.0), 0.0, 1.0);
    let amount = mix(fog * fog * (3.0 - 2.0 * fog), fog, globals.fog.w);
    return mix(colour, fog_colour(), amount);
}

@fragment
fn fs_liquid(in: LiquidOut) -> @location(0) vec4<f32> {
    let p = liquid.which.x;
    let xy = in.world.xy;
    let n = normalize(in.normal);
    var rgb = vec3<f32>(0.0);
    var alpha = 0.0;
    if p == 0u || p == 1u || p == 4u {
        // the water's two passes (0: flowing, 1: rippling down the slopes) and the wide water (4)
        var uv = xy / 40.0;
        if p == 0u {
            uv = uv + vec2<f32>(liquid.phases.x, -liquid.phases.x);
        } else if p == 1u {
            if n.z < 0.9999 {
                uv = uv - liquid.phases.y * slope(n);
            }
        } else {
            uv = xy / 160.0;
        }
        let tex = textureSample(water_tex, liquid_samp, uv);
        var tint = vec3<f32>(0.0);
        var spec = vec3<f32>(0.0);
        if p == 4u {
            tint = vec3<f32>(0.2, 0.3, 0.4);
        } else {
            spec = glint(in.world, n, 0.505, 10.0);
        }
        rgb = tex.rgb * lit(n, in.colour.rgb, tint) + spec;
        alpha = tex.a * in.colour.a;
    } else if p == 2u || p == 3u {
        // the toxic liquid's two passes: still, then stepping down the slopes by their signs
        var uv = xy / 12.0;
        if p == 3u && n.z < 0.9 {
            uv = uv - liquid.phases.z * vec2<f32>(step_sign(n.x), step_sign(n.y));
        }
        let tex = textureSample(toxic_tex, liquid_samp, uv);
        rgb = tex.rgb * in.colour.rgb;
        alpha = tex.a * in.colour.a;
    } else if p == 7u {
        // The water's refraction (FUN_1003b9c9): the picture so far sampled at the fragment's own screen place mirrored top to
        // bottom and shifted by the camera's lean (1.414 times its tilt), so the water shows what stands above the horizon;
        // blended by a band about the mirror line, `1 - |5 (t - base)|` squared (nothing under 0.2) and times `(1 - |tilt|)²`,
        // so it is strongest with the camera level and gone looking straight down. (The frame block's wave offsets that
        // wobble the place are 0 in every script.)
        let px = in.clip.x / liquid.screen.x;
        let py = in.clip.y / liquid.screen.y;
        let tilt = liquid.screen.z;
        let c = 1.0 - abs(tilt);
        let lean = tilt * 1.414;
        let base = 0.5 - lean * 0.5;
        let t = 1.0 - py;
        let s = 1.0 - abs((t - base) * 5.0);
        var band = 0.0;
        if s > 0.2 {
            band = s * s * c * c;
        }
        rgb = textureSample(screen_tex, screen_samp, vec2<f32>(px, t + lean)).rgb;
        alpha = band;
    } else if p == 5u {
        // the lava's base: the water texture in the vertex colour, as opaque as the colour
        let tex = textureSample(water_tex, liquid_samp, xy / 8.0);
        rgb = tex.rgb * in.colour.rgb;
        alpha = in.colour.a;
    } else {
        // the lava's glow: its texture at two scales, the alpha of both
        let near = textureSample(lava_tex, liquid_samp, vec2<f32>(xy.x / 75.0 + liquid.drift.x, xy.y / 75.0));
        let far = textureSample(lava_tex, liquid_samp, vec2<f32>(xy.x / 250.0, xy.y / 250.0 + liquid.drift.y));
        rgb = near.rgb;
        alpha = near.a * far.a;
    }
    return vec4<f32>(fogged(rgb, in.world), alpha);
}
