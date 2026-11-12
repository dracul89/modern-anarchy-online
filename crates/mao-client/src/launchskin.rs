//! The launcher's own backgrounds: this client's, not the original's.
//!
//! The original's launcher has ten pictures and shows one. The launcher's `background` (in `launcher.ini`, set on the
//! settings' GUI page) is `game` for those, `stars` for a night sky drawn here with the game's steel emblem
//! (`GFX_GUI_AO_LOGO` of the game's own skin, read from the game folder when the launcher opens) on it, or a picture
//! file of the user's own. With the news page off (`page = off`) the emblem stands large in the page's place; with it
//! on, small at the left of the strip under the page.
//!
//! The sky is made from a seed, the same each time: a dark gradient, a band of cloud across it (value noise), some
//! fifteen hundred stars, most of them faint and more of them along the band, and the brightest few with short spikes. A
//! few dozen more are drawn each frame over it and change their brightness slowly.

use std::path::PathBuf;

pub const GAME: &str = "game";
pub const STARS: &str = "stars";

/// The frame all places here are measured in: the launcher window's inside.
pub const FRAME: [f32; 2] = [794.0, 575.0];
/// The emblem's own size.
pub const EMBLEM: [f32; 2] = [161.0, 285.0];
const SEED: u64 = 0x5eed_0f_a_517;

#[derive(Debug, Clone, PartialEq)]
pub enum Background {
    Game,
    Stars,
    Picture(PathBuf),
}

/// What a `background` setting names (nothing, or `game`: the game's pictures).
pub fn background(setting: &str) -> Background {
    let setting = setting.trim();
    if setting.is_empty() || setting.eq_ignore_ascii_case(GAME) {
        Background::Game
    } else if setting.eq_ignore_ascii_case(STARS) {
        Background::Stars
    } else {
        Background::Picture(PathBuf::from(setting))
    }
}

/// The emblem's place in the frame (left, top, width, height): large in the page's place without the page, small in the
/// strip under it with it.
pub fn emblem_place(page: bool) -> [f32; 4] {
    let (height, centre) = if page { (112.0, [78.0, 502.0]) } else { (300.0, [FRAME[0] / 2.0, 222.0]) };
    let width = height * EMBLEM[0] / EMBLEM[1];
    [centre[0] - width / 2.0, centre[1] - height / 2.0, width, height]
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number from 0 up to, not with, 1.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn lattice(x: i32, y: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x85eb_ca6b) ^ (y as u32).wrapping_mul(0xc2b2_ae35) ^ salt.wrapping_mul(0x27d4_eb2f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn step(low: f32, high: f32, v: f32) -> f32 {
    smooth(((v - low) / (high - low)).clamp(0.0, 1.0))
}

/// Value noise, 0 to 1.
fn noise(x: f32, y: f32, salt: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - x0), smooth(y - y0));
    let (ix, iy) = (x0 as i32, y0 as i32);
    let top = lattice(ix, iy, salt) + (lattice(ix + 1, iy, salt) - lattice(ix, iy, salt)) * fx;
    let bottom = lattice(ix, iy + 1, salt) + (lattice(ix + 1, iy + 1, salt) - lattice(ix, iy + 1, salt)) * fx;
    top + (bottom - top) * fy
}

/// Four octaves of it, 0 to 1.
fn clouds(x: f32, y: f32, salt: u32) -> f32 {
    let (mut sum, mut weight, mut scale) = (0.0, 0.5, 1.0);
    for octave in 0..4 {
        sum += noise(x * scale, y * scale, salt + octave) * weight;
        weight *= 0.5;
        scale *= 2.0;
    }
    sum / 0.9375
}

/// How much of the band of cloud a place of the frame is in: a line through the frame, rising to the right.
fn band(u: f32, v: f32) -> f32 {
    let (dx, dy) = (u - FRAME[0] * 0.5, v - FRAME[1] * 0.42);
    let across = dx * 0.34 + dy * 0.94;
    (-(across / 150.0) * (across / 150.0)).exp()
}

/// The sky's picture, `width` by `height` pixels of red, green, blue and an alpha of 255. The same sky at every size:
/// places and sizes are the frame's, scaled.
pub fn sky(width: usize, height: usize) -> Vec<u8> {
    let (width, height) = (width.max(1), height.max(1));
    let k = width as f32 / FRAME[0];
    let mut light = vec![[0.0f32; 3]; width * height];
    for y in 0..height {
        for x in 0..width {
            let (u, v) = ((x as f32 + 0.5) / width as f32 * FRAME[0], (y as f32 + 0.5) / height as f32 * FRAME[1]);
            let down = v / FRAME[1];
            let mut c = [7.0 - 5.0 * down, 9.0 - 6.0 * down, 22.0 - 13.0 * down];
            // (the band: cloud where the noise is high, with darker lanes through it; blue turning violet)
            let cloud = band(u, v) * step(0.38, 0.85, clouds(u / 170.0, v / 170.0, 11)) * (1.0 - 0.7 * step(0.5, 0.72, clouds(u / 80.0 + 9.0, v / 80.0 + 4.0, 23)));
            let violet = clouds(u / 240.0 + 3.0, v / 240.0 + 8.0, 37);
            let tint = [34.0 + 62.0 * violet, 58.0 - 14.0 * violet, 128.0 - 12.0 * violet];
            // (and a little of it everywhere, so the dark is not flat)
            let haze = 0.05 * clouds(u / 300.0 + 20.0, v / 300.0 + 20.0, 51);
            for i in 0..3 {
                c[i] += tint[i] * (cloud * 0.62 + haze);
            }
            light[y * width + x] = c;
        }
    }
    let mut rng = Rng(SEED);
    let mut made = 0;
    while made < 1500 {
        let (u, v) = (rng.unit() * FRAME[0], rng.unit() * FRAME[1]);
        let (keep, bright, hue) = (rng.unit(), rng.unit().powf(4.0), rng.unit());
        if keep > 0.4 + 0.6 * band(u, v) {
            continue;
        }
        made += 1;
        let colour = if hue < 0.6 { [255.0, 255.0, 255.0] } else if hue < 0.8 { [185.0, 208.0, 255.0] } else if hue < 0.92 { [255.0, 238.0, 200.0] } else { [255.0, 200.0, 160.0] };
        let strength = 0.16 + 0.9 * bright;
        let sigma = (0.42 + 0.75 * bright) * k;
        let spike = if bright > 0.93 { (4.0 + 9.0 * (bright - 0.93) / 0.07) * k } else { 0.0 };
        let (cx, cy) = (u * k, v * k);
        let reach = (sigma * 3.5).max(spike).ceil() as i32 + 1;
        for py in (cy as i32 - reach).max(0)..=(cy as i32 + reach).min(height as i32 - 1) {
            for px in (cx as i32 - reach).max(0)..=(cx as i32 + reach).min(width as i32 - 1) {
                let (dx, dy) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
                let mut add = strength * (-(dx * dx + dy * dy) / (2.0 * sigma * sigma)).exp();
                if spike > 0.0 {
                    // (the spikes: thin along each axis, fading towards their ends)
                    let thin = 0.45 * k;
                    let along = |a: f32, b: f32| (1.0 - a.abs() / spike).max(0.0).powi(2) * (-(b * b) / (2.0 * thin * thin)).exp();
                    add += strength * 0.4 * (along(dx, dy) + along(dy, dx));
                }
                if add > 0.002 {
                    let cell = &mut light[py as usize * width + px as usize];
                    for i in 0..3 {
                        cell[i] += colour[i] * add;
                    }
                }
            }
        }
    }
    let mut out = Vec::with_capacity(width * height * 4);
    for (i, c) in light.iter().enumerate() {
        // (half a step of noise, so the dark gradient does not show bands)
        let dither = lattice((i % width) as i32, (i / width) as i32, 77) - 0.5;
        out.extend([(c[0] + dither).clamp(0.0, 255.0) as u8, (c[1] + dither).clamp(0.0, 255.0) as u8, (c[2] + dither).clamp(0.0, 255.0) as u8, 255]);
    }
    out
}

/// A star drawn each frame over the sky: its place as parts of the frame, its radius in the frame's units, its colour
/// and how its brightness moves.
#[derive(Debug, Clone, PartialEq)]
pub struct Twinkler {
    pub at: [f32; 2],
    pub radius: f32,
    pub colour: [u8; 3],
    phase: f32,
    rate: f32,
}

impl Twinkler {
    /// Its brightness, 0.25 to 1, after `seconds`.
    pub fn brightness(&self, seconds: f32) -> f32 {
        0.625 + 0.375 * (seconds * self.rate + self.phase).sin()
    }
}

pub fn twinklers() -> Vec<Twinkler> {
    let mut rng = Rng(SEED ^ 0x7717);
    (0..48)
        .map(|_| {
            let hue = rng.unit();
            Twinkler {
                at: [rng.unit(), rng.unit()],
                radius: 0.6 + 0.7 * rng.unit(),
                colour: if hue < 0.6 { [255, 255, 255] } else if hue < 0.85 { [190, 212, 255] } else { [255, 232, 196] },
                phase: rng.unit() * std::f32::consts::TAU,
                rate: 0.5 + 1.6 * rng.unit(),
            }
        })
        .collect()
}

/// The emblem from the skin's picture of it, `height` pixels high: the green of its key (and what of it runs into the
/// edge) taken out, its edge softened, alpha premultiplied. With the pixels, the picture's size.
pub fn emblem(png: &[u8], height: u32) -> Option<(Vec<u8>, [usize; 2])> {
    let mut picture = image::load_from_memory(png).ok()?.to_rgba8();
    for pixel in picture.pixels_mut() {
        let [r, g, b, _] = pixel.0;
        let key = g > 150 && g as i32 > r as i32 + 60 && g as i32 > b as i32 + 60;
        pixel.0 = if key { [0, 0, 0, 0] } else { [r, g, b, 255] };
    }
    let height = height.clamp(8, 4096);
    let width = ((height as f32 * picture.width() as f32 / picture.height() as f32).round() as u32).max(1);
    // (the key's pixels are black and clear and the others whole, so the picture is premultiplied as it stands and stays
    // so through the resize, but for what the filter overshoots)
    let mut scaled = image::imageops::resize(&picture, width, height, image::imageops::FilterType::CatmullRom);
    for pixel in scaled.pixels_mut() {
        let a = pixel.0[3];
        for i in 0..3 {
            pixel.0[i] = pixel.0[i].min(a);
        }
    }
    Some((scaled.into_raw(), [width as usize, height as usize]))
}

/// A round glow to lay under the emblem: white, fading from its middle to nothing at its edge (premultiplied).
pub fn glow(size: usize) -> Vec<u8> {
    let size = size.max(2);
    let mut out = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = ((x as f32 + 0.5) / size as f32 * 2.0 - 1.0, (y as f32 + 0.5) / size as f32 * 2.0 - 1.0);
            let far = (dx * dx + dy * dy).sqrt().min(1.0);
            let a = ((1.0 - far).powf(1.8) * 255.0) as u8;
            out.extend([a, a, a, a]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_names_its_background() {
        assert_eq!(background(""), Background::Game);
        assert_eq!(background(" Game "), Background::Game);
        assert_eq!(background("stars"), Background::Stars);
        assert_eq!(background("D:/pictures/mine.png"), Background::Picture(PathBuf::from("D:/pictures/mine.png")));
    }

    #[test]
    fn the_sky_is_dark_with_stars_and_the_same_each_time() {
        let sky = super::sky(397, 288);
        assert_eq!(sky.len(), 397 * 288 * 4);
        assert_eq!(sky, super::sky(397, 288));
        let brightest: Vec<u8> = sky.chunks(4).map(|p| p[0].max(p[1]).max(p[2])).collect();
        // (mostly night: nine pixels in ten are dark; and there are stars: some hundreds are bright)
        let dark = brightest.iter().filter(|&&v| v < 70).count();
        let lit = brightest.iter().filter(|&&v| v > 150).count();
        assert!(dark * 10 > brightest.len() * 9, "{dark} of {}", brightest.len());
        assert!(lit > 100 && lit < 4000, "{lit}");
        assert!(sky.chunks(4).all(|p| p[3] == 255));
    }

    #[test]
    fn the_emblem_has_its_place_with_and_without_the_page() {
        let large = emblem_place(false);
        assert!(large[1] > 0.0 && large[1] + large[3] < 428.0);
        assert!((large[0] + large[2] / 2.0 - 397.0).abs() < 0.01);
        let small = emblem_place(true);
        // (under the page, and left of the dimension list)
        assert!(small[1] > 428.0 && small[1] + small[3] < 575.0 && small[0] + small[2] < 403.0);
        for place in [large, small] {
            assert!((place[2] / place[3] - 161.0 / 285.0).abs() < 0.001);
        }
    }

    #[test]
    fn the_emblem_loses_its_key() {
        // (a picture of the key's green with a grey square in it and a pixel of the two mixed)
        let mut picture = image::RgbaImage::from_pixel(16, 28, image::Rgba([0, 255, 0, 255]));
        for y in 8..20 {
            for x in 4..12 {
                picture.put_pixel(x, y, image::Rgba([90, 95, 110, 255]));
            }
        }
        picture.put_pixel(3, 10, image::Rgba([40, 190, 50, 255]));
        let mut png = Vec::new();
        picture.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let (pixels, size) = emblem(&png, 56).unwrap();
        assert_eq!(size, [32, 56]);
        let at = |x: usize, y: usize| &pixels[(y * size[0] + x) * 4..][..4];
        assert_eq!(at(0, 0), [0, 0, 0, 0]);
        assert_eq!(at(16, 28)[3], 255);
        assert!(at(16, 28)[2] > at(16, 28)[0]);
        // (no green left anywhere: every pixel's green is near its red and blue)
        assert!(pixels.chunks(4).all(|p| (p[1] as i32) < p[0].max(p[2]) as i32 + 12));
        assert!(emblem(b"not a picture", 56).is_none());
    }

    #[test]
    fn the_twinklers_stay_in_the_frame_and_in_their_range() {
        let stars = twinklers();
        assert_eq!(stars.len(), 48);
        assert_eq!(stars, twinklers());
        for star in &stars {
            assert!((0.0..1.0).contains(&star.at[0]) && (0.0..1.0).contains(&star.at[1]));
            for seconds in [0.0, 1.3, 77.7] {
                assert!((0.25..=1.0).contains(&star.brightness(seconds)));
            }
        }
        let glow = glow(32);
        assert_eq!(glow.len(), 32 * 32 * 4);
        assert_eq!(glow[..4], [0, 0, 0, 0]);
        assert!(glow[(16 * 32 + 16) * 4] > 200);
    }
}
