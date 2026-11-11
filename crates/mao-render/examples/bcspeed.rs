//! `bcspeed`: how fast the block encoders are here (megapixels a second on one core) on a noisy 1024x1024 picture.
fn main() {
    let (w, h) = (1024u32, 1024u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let mut seed = 12345u32;
    for (i, p) in rgba.chunks_exact_mut(4).enumerate() {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let (x, y) = (i as u32 % w, i as u32 / w);
        p[0] = ((x / 4) as u8).wrapping_add((seed >> 28) as u8);
        p[1] = ((y / 4) as u8).wrapping_add((seed >> 24 & 15) as u8);
        p[2] = (((x + y) / 8) as u8).wrapping_add((seed >> 20 & 15) as u8);
        p[3] = if (x / 64 + y / 64) % 2 == 0 { 255 } else { (seed >> 16) as u8 };
    }
    let surface = intel_tex_2::RgbaSurface { data: &rgba, width: w, height: h, stride: w * 4 };
    let time = |name: &str, f: &dyn Fn() -> Vec<u8>| {
        let t = std::time::Instant::now();
        let out = f();
        let s = t.elapsed().as_secs_f64();
        println!("{name}: {:.1} ms, {:.0} megapixels a second, {} bytes", s * 1000.0, (w * h) as f64 / 1e6 / s, out.len());
    };
    time("bc1", &|| intel_tex_2::bc1::compress_blocks(&surface));
    time("bc3", &|| intel_tex_2::bc3::compress_blocks(&surface));
    time("bc7 alpha ultra fast", &|| intel_tex_2::bc7::compress_blocks(&intel_tex_2::bc7::alpha_ultra_fast_settings(), &surface));
    time("bc7 alpha very fast", &|| intel_tex_2::bc7::compress_blocks(&intel_tex_2::bc7::alpha_very_fast_settings(), &surface));
    time("bc7 alpha fast", &|| intel_tex_2::bc7::compress_blocks(&intel_tex_2::bc7::alpha_fast_settings(), &surface));
    time("bc7 opaque ultra fast", &|| intel_tex_2::bc7::compress_blocks(&intel_tex_2::bc7::opaque_ultra_fast_settings(), &surface));
    time("bc7 opaque very fast", &|| intel_tex_2::bc7::compress_blocks(&intel_tex_2::bc7::opaque_very_fast_settings(), &surface));
}
