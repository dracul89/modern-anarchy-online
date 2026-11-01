//! The performance overlay (`/perf`, or Options > Graphics > Show performance): the frame rate and frame times, the processor
//! and memory the client and the machine use, and the graphics card's share of the frame. The processor and memory are sampled
//! once a second (`sysinfo`); the card's time is the renderer's own timestamps (`Renderer::gpu_ms`).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::theme::Palette;

/// What the renderer tells the overlay each frame.
#[derive(Clone, Debug, Default)]
pub struct GpuInfo {
    /// The adapter's name and backend ("NVIDIA GeForce RTX 4080, Dx12").
    pub adapter: String,
    /// The scene's time on the card (milliseconds), where the card measures it.
    pub gpu_ms: Option<f32>,
    /// The graphics memory the renderer holds (bytes), where the backend reports it.
    pub gpu_memory: Option<u64>,
    /// Parts drawn, of all, and in the shadow pass.
    pub drawn: (usize, usize, usize),
    /// The size the scene is drawn at and the window's, and the anti-aliasing.
    pub render_size: (u32, u32),
    pub window_size: (u32, u32),
    pub antialiasing: &'static str,
}

/// A second's sample of the processor and memory.
#[derive(Clone, Copy, Default)]
struct Sample {
    /// The client's processor use in percent of one core, and the machine's in percent of all of them.
    process_cpu: f32,
    system_cpu: f32,
    cores: usize,
    /// The client's memory (resident) and the machine's used and total (bytes).
    process_memory: u64,
    used_memory: u64,
    total_memory: u64,
}

/// How far back the frame times go (seconds).
const WINDOW: f32 = 2.0;

pub struct Perf {
    /// The frame times of the last couple of seconds (milliseconds).
    frames: VecDeque<f32>,
    /// (none where the system can't be read)
    system: Option<sysinfo::System>,
    pid: Option<sysinfo::Pid>,
    sampled: Instant,
    sample: Sample,
    /// The card's frame times, smoothed.
    gpu_ms: Option<f32>,
}

impl Default for Perf {
    fn default() -> Perf {
        Perf { frames: VecDeque::new(), system: sysinfo::System::new().ok(), pid: sysinfo::get_current_pid().ok(), sampled: Instant::now() - Duration::from_secs(2), sample: Sample::default(), gpu_ms: None }
    }
}

/// Frames per second, the mean frame time and the slowest 1% of frames (milliseconds) of these frame times.
pub fn summary(frames: &[f32]) -> (f32, f32, f32) {
    if frames.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let total: f32 = frames.iter().sum();
    let mean = total / frames.len() as f32;
    let mut sorted = frames.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let worst = sorted[..(frames.len() / 100).max(1)].iter().sum::<f32>() / (frames.len() / 100).max(1) as f32;
    (1000.0 / mean.max(1e-3), mean, worst)
}

impl Perf {
    /// Notes a frame of `dt` seconds, and samples the processor and memory when a second has passed.
    pub fn frame(&mut self, dt: f32, gpu_ms: Option<f32>) {
        self.frames.push_back(dt * 1000.0);
        let mut total: f32 = self.frames.iter().sum();
        while total > WINDOW * 1000.0 && self.frames.len() > 1 {
            total -= self.frames.pop_front().unwrap_or(0.0);
        }
        if let Some(g) = gpu_ms {
            self.gpu_ms = Some(self.gpu_ms.map_or(g, |s| s + (g - s) * 0.1));
        }
        if let Some(system) = self.system.as_mut().filter(|_| self.sampled.elapsed() >= Duration::from_secs(1)) {
            self.sampled = Instant::now();
            system.refresh_cpu_usage();
            system.refresh_memory();
            if let Some(pid) = self.pid {
                system.refresh_processes_specifics(sysinfo::ProcessesToUpdate::Some(&[pid]), true, sysinfo::ProcessRefreshKind::nothing().with_cpu().with_memory());
            }
            let process = self.pid.and_then(|p| system.process(p));
            self.sample = Sample {
                process_cpu: process.map_or(0.0, |p| p.cpu_usage()),
                system_cpu: system.global_cpu_usage(),
                cores: system.cpus().len().max(1),
                process_memory: process.map_or(0, |p| p.memory()),
                used_memory: system.used_memory(),
                total_memory: system.total_memory(),
            };
        }
    }

    /// Draws the overlay at the top right.
    pub fn show(&self, ctx: &egui::Context, gpu: &GpuInfo, palette: &Palette) {
        let frames: Vec<f32> = self.frames.iter().copied().collect();
        let (fps, mean, worst) = summary(&frames);
        let s = self.sample;
        let gb = |b: u64| b as f64 / (1u64 << 30) as f64;
        let mb = |b: u64| b as f64 / (1u64 << 20) as f64;
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        egui::Area::new(egui::Id::new("performance")).anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 40.0)).interactable(false).show(ctx, |ui| {
            egui::Frame::new().fill(egui::Color32::from_black_alpha(170)).corner_radius(4.0).inner_margin(8.0).show(ui, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                let row = |ui: &mut egui::Ui, name: &str, value: String| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{name:<5}")).color(accent));
                        ui.label(egui::RichText::new(value).color(egui::Color32::WHITE));
                    });
                };
                row(ui, "FPS", format!("{fps:5.0}   {mean:5.2} ms  (1% low {worst:.1} ms)"));
                // (the frame-time graph: the last couple of seconds, 33 ms at the top)
                let (rect, _) = ui.allocate_exact_size(egui::vec2(220.0, 36.0), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(120));
                let n = frames.len().max(1);
                let line = |ms: f32| rect.bottom() - (ms / 33.3).min(1.0) * rect.height();
                painter.hline(rect.x_range(), line(16.7), egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40)));
                let points: Vec<egui::Pos2> = frames.iter().enumerate().map(|(i, &ms)| egui::pos2(rect.left() + rect.width() * i as f32 / n as f32, line(ms))).collect();
                painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, accent)));
                row(ui, "CPU", format!("{:5.1}% client ({:.1} of {} cores)   {:4.1}% system", s.process_cpu / s.cores as f32, s.process_cpu / 100.0, s.cores, s.system_cpu));
                row(ui, "RAM", format!("{:6.0} MB client   {:.1} / {:.1} GB system", mb(s.process_memory), gb(s.used_memory), gb(s.total_memory)));
                let gpu_time = match self.gpu_ms {
                    Some(g) => format!("{g:5.2} ms scene ({:.0}% of the frame)", (g / mean.max(1e-3) * 100.0).min(100.0)),
                    None => "the card's time can't be measured here".to_string(),
                };
                row(ui, "GPU", gpu_time);
                if let Some(m) = gpu.gpu_memory {
                    row(ui, "VRAM", format!("{:6.0} MB held by the renderer", mb(m)));
                }
                ui.label(egui::RichText::new(&gpu.adapter).small().color(egui::Color32::LIGHT_GRAY));
                ui.label(
                    egui::RichText::new(format!(
                        "{}x{} drawn at {}x{}, {}   {} of {} parts, {} in shadow",
                        gpu.window_size.0, gpu.window_size.1, gpu.render_size.0, gpu.render_size.1, gpu.antialiasing, gpu.drawn.0, gpu.drawn.1, gpu.drawn.2
                    ))
                    .small()
                    .color(egui::Color32::LIGHT_GRAY),
                );
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_reads_frame_times() {
        assert_eq!(summary(&[]), (0.0, 0.0, 0.0));
        // 99 frames of 10 ms and one of 50: 100 fps near enough, the slowest 1% the 50 ms one
        let mut frames = vec![10.0f32; 99];
        frames.push(50.0);
        let (fps, mean, worst) = summary(&frames);
        assert!((mean - 10.4).abs() < 1e-3 && (fps - 96.15).abs() < 0.1 && worst == 50.0, "{fps} {mean} {worst}");
    }

    #[test]
    fn the_frames_kept_cover_two_seconds() {
        let mut p = Perf::default();
        for _ in 0..500 {
            p.frame(0.010, None);
        }
        assert!(p.frames.len() <= 201 && p.frames.len() >= 199, "{}", p.frames.len());
        p.frame(0.010, Some(4.0));
        assert_eq!(p.gpu_ms, Some(4.0));
    }
}
