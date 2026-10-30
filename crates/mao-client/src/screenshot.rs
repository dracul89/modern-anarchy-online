//! The screenshot key's picture: on Windows the game window's client area as it shows on the screen (the monitor's
//! picture cut to the window: the scene, every open window and the web page laid over the game, which is a window of its
//! own), and the local time for the file's name. Elsewhere the client renders the scene and the HUD offscreen instead.

/// The local date and time as `YYYY-MM-DD-HHMMSS` (none when the local offset is unknown).
pub fn local_stamp() -> Option<String> {
    let t = time::OffsetDateTime::now_local().ok()?;
    Some(format!("{:04}-{:02}-{:02}-{:02}{:02}{:02}", t.year(), t.month() as u8, t.day(), t.hour(), t.minute(), t.second()))
}

/// The window's client area as it is on the screen: its width, height and RGBA pixels, top row first.
#[cfg(windows)]
pub fn capture(window: &winit::window::Window) -> Option<(u32, u32, Vec<u8>)> {
    let at = window.inner_position().ok()?;
    let size = window.inner_size();
    let monitor = xcap::Monitor::from_point(at.x, at.y).ok()?;
    let (mx, my) = (monitor.x().ok()?, monitor.y().ok()?);
    let (mw, mh) = (monitor.width().ok()?, monitor.height().ok()?);
    // (the part of the window on its monitor)
    let (x, y) = ((at.x - mx).max(0) as u32, (at.y - my).max(0) as u32);
    let (w, h) = (size.width.min(mw.saturating_sub(x)), size.height.min(mh.saturating_sub(y)));
    if w == 0 || h == 0 {
        return None;
    }
    let image = monitor.capture_region(x, y, w, h).ok()?;
    Some((image.width(), image.height(), image.into_raw()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_name_is_the_local_time() {
        let name = super::local_stamp().expect("a local time");
        assert_eq!(name.len(), "2026-10-29-201240".len(), "{name}");
        eprintln!("{name}");
    }
}
