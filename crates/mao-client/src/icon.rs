//! The window's icon: the original GUI's steel emblem (`GFX_GUI_AO_LOGO`) on a slate square, `assets/icon/mao_256.png`,
//! compiled into the executable. The executable's own icon on Windows is the matching `mao.ico` (`build.rs`).

use std::sync::OnceLock;

use winit::window::Icon;

const PNG: &[u8] = include_bytes!("../../../assets/icon/mao_256.png");

/// The icon for a window of the client's, decoded once (none if the picture does not decode, which a test checks).
pub fn window_icon() -> Option<Icon> {
    static ICON: OnceLock<Option<Icon>> = OnceLock::new();
    ICON.get_or_init(|| {
        let picture = image::load_from_memory(PNG).ok()?.to_rgba8();
        let (width, height) = picture.dimensions();
        Icon::from_rgba(picture.into_raw(), width, height).ok()
    })
    .clone()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_icon_decodes() {
        let picture = image::load_from_memory(super::PNG).expect("a PNG").to_rgba8();
        assert_eq!(picture.dimensions(), (256, 256));
        // (the slate square's corner is transparent, its middle is not)
        assert_eq!(picture.get_pixel(0, 0)[3], 0);
        assert_eq!(picture.get_pixel(128, 128)[3], 255);
    }
}
