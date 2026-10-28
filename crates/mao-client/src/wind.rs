//! The wind, as the original's environment weather object makes it (Gamecode FUN_100bff68 and FUN_100bfc80, run each frame by
//! the environment's frame FUN_100becbe; `_EffectHandler_t::ComputeWind` FUN_100cdee7 reads it for the effects).
//!
//! - **The base wind** (FUN_100bff68): its strength is the weather table's wind (entry 7, `mao_formats::weather`), held to 0..33
//!   (0x10169558) and then no less than 0.001 (0x10161990); its direction stays the one it had (along +x the first time), its
//!   length the strength. The gust scale is `1 − (1 − strength / 33)³`.
//! - **The gusts** (FUN_100bfc80, given the environment's 0.9 and 0.5, +0x50 and +0x4c): each frame, with a chance of 0.9
//!   each, a level gust gets a kick of `((r % 100 − 50) / 450) / 0.016` across x and z, and a second gust a kick of that across
//!   x and z and `((r % 100 − 50) / 800) / 0.016` up, both times `(0.5 + 0.5 × scale) × 0.5`. The wind is the base plus the
//!   second gust; then both gusts are divided by 1.02.
//! - **The effects' wind** (`ComputeWind`): the handler copies the wind (+0x24, which Sparks are pushed by) and keeps a smoothed
//!   one, `0.04 × wind + 0.96 × itself` (+0x30, which Smoke is pushed by).
//!
//! The original steps once a frame; the client steps at 30 a second whatever its frame rate. The vectors are in the original's
//! Y-up frame; [`Wind::now`] and [`Wind::smooth`] give them in the client's Z-up one.

const STEP: f32 = 1.0 / 30.0;

#[derive(Clone, Debug)]
pub struct Wind {
    base: [f32; 3],
    gust: [f32; 3],
    level_gust: [f32; 3],
    wind: [f32; 3],
    smooth: [f32; 3],
    seed: u32,
    carry: f32,
}

impl Default for Wind {
    fn default() -> Wind {
        Wind { base: [0.0; 3], gust: [0.0; 3], level_gust: [0.0; 3], wind: [0.0; 3], smooth: [0.0; 3], seed: 0x2545_f491, carry: 0.0 }
    }
}

impl Wind {
    fn rand(&mut self) -> u32 {
        // (xorshift: the original's rand() is not reproduced)
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed
    }

    /// Runs the wind on by `dt` seconds under the weather's wind `strength` (entry 7 of the weather table).
    pub fn advance(&mut self, strength: f32, dt: f32) {
        self.carry += dt.clamp(0.0, 0.5);
        while self.carry >= STEP {
            self.carry -= STEP;
            self.step(strength);
        }
    }

    /// One frame of the original.
    pub fn step(&mut self, strength: f32) {
        // FUN_100bff68: the base wind's length and the gust scale
        let s = if strength.is_nan() { 0.0 } else { strength.clamp(0.0, 33.0) }.max(0.001);
        let len = (self.base[0] * self.base[0] + self.base[1] * self.base[1] + self.base[2] * self.base[2]).sqrt();
        self.base = if len == 0.0 { [s, 0.0, 0.0] } else { self.base.map(|c| c / len * s) };
        let f = 1.0 - s / 33.0;
        let scale = 1.0 - f * f * f;
        // FUN_100bfc80: the two gusts' kicks, the wind, the decay
        let kick = (0.5 + scale * 0.5) * 0.5;
        let across = |r: u32| (((r % 100) as f32 - 50.0) / 450.0) / 0.016;
        if self.rand() % 100 < 90 {
            let (x, z) = (across(self.rand()), across(self.rand()));
            self.level_gust[0] += x * kick;
            self.level_gust[2] += z * kick;
        }
        if self.rand() % 100 < 90 {
            let x = across(self.rand());
            let y = (((self.rand() % 100) as f32 - 50.0) / 800.0) / 0.016;
            let z = across(self.rand());
            self.gust[0] += x * kick;
            self.gust[1] += y * kick;
            self.gust[2] += z * kick;
        }
        self.wind = [0, 1, 2].map(|i| self.base[i] + self.gust[i]);
        self.gust = self.gust.map(|c| c / 1.02);
        self.level_gust = self.level_gust.map(|c| c / 1.02);
        // ComputeWind: the smoothed copy
        self.smooth = [0, 1, 2].map(|i| self.wind[i] * 0.04 + self.smooth[i] * 0.96);
    }

    /// The gust's length (the wind object's +0xc, which FUN_100bff68 scales the winds' sounds by).
    pub fn gust_length(&self) -> f32 {
        (self.gust[0] * self.gust[0] + self.gust[1] * self.gust[1] + self.gust[2] * self.gust[2]).sqrt()
    }

    /// The wind the Sparks are pushed by (the handler's +0x24), Z up.
    pub fn now(&self) -> [f32; 3] {
        [self.wind[0], self.wind[2], self.wind[1]]
    }

    /// The smoothed wind the Smoke is pushed by (the handler's +0x30), Z up.
    pub fn smooth(&self) -> [f32; 3] {
        [self.smooth[0], self.smooth[2], self.smooth[1]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wind_blows_at_the_weather_s_strength_with_gusts_round_it() {
        let mut w = Wind::default();
        // calm: the base is a thousandth along x, the gusts alone move it
        w.advance(0.0, 10.0);
        assert!(w.now().iter().all(|c| c.is_finite()));
        let calm = w.now();
        // a strong wind: the base along x at 20, the gusts round it larger (the scale nears 1)
        let mut strong = Wind::default();
        let mut sum = [0.0f32; 3];
        for _ in 0..30000 {
            strong.step(20.0);
            let n = strong.now();
            for i in 0..3 {
                sum[i] += n[i];
            }
        }
        let mean = sum.map(|c| c / 30000.0);
        assert!((mean[0] - 20.0).abs() < 3.0, "{mean:?}");
        assert!(mean[1].abs() < 3.0 && mean[2].abs() < 1.0);
        // the smoothed wind follows the wind
        assert!((strong.smooth()[0] - 20.0).abs() < 8.0);
        // the strength is held to 33
        let mut wild = Wind::default();
        wild.step(500.0);
        assert!((wild.base[0] - 33.0).abs() < 1e-4);
        let _ = calm;
    }
}
