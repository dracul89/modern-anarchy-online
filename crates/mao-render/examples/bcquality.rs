//! `bcquality <game> [count]`: how close the block encoders come on the game's own textures with alpha: the signal to
//! noise (dB, more is closer) of the colour and of the alpha after BC3 and after BC7 (the quickest setting), and the
//! time each took, over the first `count` (40) textures of at least 512 texels with a texel under full alpha.
use mao_formats::resource::ResourceDatabase;

fn psnr(a: &[u8], b: &[u8], channels: std::ops::Range<usize>) -> f64 {
    let (mut sum, mut n) = (0f64, 0f64);
    for (p, q) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        for c in channels.clone() {
            let d = p[c] as f64 - q[c] as f64;
            sum += d * d;
            n += 1.0;
        }
    }
    if sum == 0.0 { 99.0 } else { 10.0 * (255.0f64 * 255.0 / (sum / n)).log10() }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let count: usize = args.get(1).and_then(|c| c.parse().ok()).unwrap_or(40);
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let mut ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == 0xf6954).collect();
    ids.sort_by_key(|i| std::cmp::Reverse(i.instance));
    let (mut done, mut sums, mut times) = (0usize, [0f64; 4], [0f64; 2]);
    let mut worst = [99f64; 4];
    for id in ids {
        if done >= count {
            break;
        }
        let Some(object) = db.get(id).ok().flatten() else { continue };
        let Ok(t) = mao_formats::texture::decode(&object) else { continue };
        if t.width.max(t.height) < 512 || t.width % 4 != 0 || t.height % 4 != 0 || t.rgba.chunks_exact(4).all(|p| p[3] >= 250) {
            continue;
        }
        let surface = intel_tex_2::RgbaSurface { data: &t.rgba, width: t.width, height: t.height, stride: t.width * 4 };
        let (w, h) = (t.width as usize, t.height as usize);
        let unpack = |decoded: Vec<u32>| -> Vec<u8> { decoded.iter().flat_map(|p| { let [b, g, r, a] = p.to_le_bytes(); [r, g, b, a] }).collect() };
        let start = std::time::Instant::now();
        let bc3 = intel_tex_2::bc3::compress_blocks(&surface);
        times[0] += start.elapsed().as_secs_f64();
        let start = std::time::Instant::now();
        let bc7 = intel_tex_2::bc7::compress_blocks(&intel_tex_2::bc7::alpha_ultra_fast_settings(), &surface);
        times[1] += start.elapsed().as_secs_f64();
        let mut out = vec![0u32; w * h];
        texture2ddecoder::decode_bc3(&bc3, w, h, &mut out).unwrap();
        let from3 = unpack(out.clone());
        texture2ddecoder::decode_bc7(&bc7, w, h, &mut out).unwrap();
        let from7 = unpack(out);
        let row = [psnr(&t.rgba, &from3, 0..3), psnr(&t.rgba, &from3, 3..4), psnr(&t.rgba, &from7, 0..3), psnr(&t.rgba, &from7, 3..4)];
        println!("{:#x} {}x{}: bc3 colour {:.1} alpha {:.1} | bc7 colour {:.1} alpha {:.1}", id.instance, t.width, t.height, row[0], row[1], row[2], row[3]);
        for k in 0..4 {
            sums[k] += row[k];
            worst[k] = worst[k].min(row[k]);
        }
        done += 1;
    }
    let n = done.max(1) as f64;
    println!("{done} textures. mean: bc3 colour {:.1} dB alpha {:.1} dB in {:.2} s | bc7 colour {:.1} dB alpha {:.1} dB in {:.2} s", sums[0] / n, sums[1] / n, times[0], sums[2] / n, sums[3] / n, times[1]);
    println!("worst: bc3 colour {:.1} alpha {:.1} | bc7 colour {:.1} alpha {:.1}", worst[0], worst[1], worst[2], worst[3]);
}
