//! What `Environment_t`'s per-frame work (FUN_100becbe) hands the renderer, from the areas' environments and their
//! weather at the character: the ambient light, one fog (gathered as `VisualFog_t` gathers its `AddFog` calls:
//! displaysystem `fog.rs` — the first or a stronger fog takes over, a weaker one is blended into the colour by the
//! weights), and how dark the weather makes the sky.
//!
//! The fogs, in the original's order:
//! - the weather fog (table [1]): grey `0.7 × [1]`, weight `[1]`;
//! - the mist ([0], FUN_100b9da8 over the object of FUN_100b9775): its colour is (0.4, 0.5, 0.7) when the darkness is 4
//!   or more, grey 0.8 at −0.5 or less, lerped between (FUN_100b98fe), scaled by the light × 1.4 kept within 0.5 and 1;
//!   fogged as `(r × 0.7, g × 0.8, b × 0.9)` with weight `[0] × 0.8`;
//! - the four precipitations ([3]–[6], FUN_100b9ba5 over the objects set up by FUN_100b9b10): colours (0.6, 0.35, 0),
//!   (0.18, 0.15, 0.15), (0.35, 0, 0) and (0, 0.4, 0), fogged at × 0.8 with weight `min(1, v) × 0.7`;
//! - the area's own fog ([0x12]–[0x15]), when its colour is not black, after `AddFogLightIntensity(min(day,
//!   1 − [8] × 0.8), at least 0.5)`.
//!
//! The darkness [8] also sets `VisualEnvFX_t`'s +0x1134 to `1 − [8] / 2.333`: the client takes that as a dimming of
//! the sky and its light.

use mao_formats::areas::{self, Area};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Environment {
    pub ambient: [f32; 3],
    /// The gathered fog's colour (lit by the fog light) and density, when any.
    pub fog: Option<([f32; 3], f32)>,
    /// What the weather leaves of the sky's light, 0 to 1.
    pub sky: f32,
    /// The weather values, for the debugging overlay.
    pub weather: [f32; 14],
}

/// `VisualFog_t`'s gather (`add_fog` / `process` without the renderer part).
#[derive(Default)]
struct Gather {
    colour: [f32; 3],
    density: f32,
    light: f32,
}

impl Gather {
    fn new() -> Gather {
        Gather { colour: [0.0; 3], density: 0.0, light: 1.0 }
    }

    fn add(&mut self, rgb: [f32; 3], weight: f32) {
        let w = weight.min(1.0);
        if w <= 0.0 {
            return;
        }
        let rgb = rgb.map(|c| c.clamp(0.0, 1.0));
        if self.density <= 0.0 {
            self.colour = rgb;
            self.density = w;
            return;
        }
        let t = self.density / (w + self.density);
        for k in 0..3 {
            self.colour[k] = self.colour[k] * t + (1.0 - t) * rgb[k];
        }
        if w > self.density {
            self.density = w;
        }
    }

    fn light(&mut self, i: f32) {
        if i < self.light {
            self.light = i;
        }
    }
}

/// The environment at `position` (server coordinates) on `day` at `seconds` into it, with the day's intensity.
pub fn compute(areas_list: &[Area], position: [f32; 3], day: i32, seconds: f32, daylight: f32) -> Environment {
    let (blend, weather) = areas::blend_with_weather(areas_list, position, Some((day, seconds)));
    let v = weather.v;
    let mut fog = Gather::new();
    // the weather fog
    fog.add([v[1] * 0.7; 3], v[1]);
    // the mist
    if v[0] > 0.0 {
        let day_colour = [0.4, 0.5, 0.7];
        let grey = [0.8, 0.8, 0.8];
        let f = if v[8] > 4.0 { 0.0 } else if v[8] <= -0.5 { 1.0 } else { 1.0 - (v[8] + 0.5) / 4.5 };
        let light = (daylight * 1.4).clamp(0.5, 1.0);
        let c = [0, 1, 2].map(|k| (day_colour[k] + f * (grey[k] - day_colour[k])) * light);
        fog.add([c[0] * 0.7, c[1] * 0.8, c[2] * 0.9], v[0] * 0.8);
    }
    // the precipitations
    const COLOURS: [[f32; 3]; 4] = [[0.6, 0.35, 0.0], [0.18, 0.15, 0.15], [0.35, 0.0, 0.0], [0.0, 0.4, 0.0]];
    for (k, colour) in COLOURS.iter().enumerate() {
        let x = v[3 + k];
        if x > 0.0 {
            fog.add(colour.map(|c| c * 0.8), x.min(1.0) * 0.7);
        }
    }
    // the area's fog
    if blend.fog.iter().any(|&c| c != 0.0) {
        fog.light((daylight.min(1.0 - v[8] * 0.8)).max(0.5));
        fog.add(blend.fog, blend.density);
    }
    let gathered = (fog.density > 0.0).then(|| (fog.colour.map(|c| c * fog.light), fog.density));
    Environment { ambient: blend.ambient, fog: gathered, sky: (1.0 - v[8] / 2.333).clamp(0.0, 1.0), weather: v }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gather_follows_the_originals_rules() {
        let mut g = Gather::new();
        g.add([1.0, 0.0, 0.0], 0.2);
        assert_eq!((g.colour, g.density), ([1.0, 0.0, 0.0], 0.2));
        // a stronger fog: the colour blends by the weights (0.2 against 0.6), the density rises
        g.add([0.0, 1.0, 0.0], 0.6);
        assert!((g.colour[0] - 0.25).abs() < 1e-6 && (g.colour[1] - 0.75).abs() < 1e-6);
        assert_eq!(g.density, 0.6);
        // a weaker one only tints
        g.add([0.0, 0.0, 1.0], 0.1);
        assert_eq!(g.density, 0.6);
        assert!(g.colour[2] > 0.1 && g.colour[2] < 0.2);
    }

    #[test]
    fn no_areas_means_no_fog() {
        let e = compute(&[], [0.0; 3], 1, 100.0, 1.0);
        assert_eq!(e.fog, None);
        assert_eq!(e.sky, 1.0);
    }
}
