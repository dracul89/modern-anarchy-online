//! The original's mouse pointers (`GFX_GUI_POINTER_*` in the skin atlas, 32 x 32 with the green key) in place of the
//! system's: egui says which pointer it wants each frame (`CursorIcon`); the matching sprite is made into a winit custom
//! cursor once and set on the window whenever the wanted pointer changes. Without the skin nothing is done and
//! egui-winit's system pointers stay.

use std::cell::RefCell;
use std::collections::HashMap;

use winit::event_loop::ActiveEventLoop;
use winit::window::{CustomCursor, Window};

thread_local! {
    static CURSORS: RefCell<HashMap<&'static str, Option<CustomCursor>>> = RefCell::new(HashMap::new());
    static LAST: RefCell<Option<egui::CursorIcon>> = const { RefCell::new(None) };
}

/// The sprite for a pointer and its hot spot (the pixel that points).
fn sprite(icon: egui::CursorIcon) -> Option<(&'static str, u16, u16)> {
    use egui::CursorIcon::*;
    Some(match icon {
        Default | ContextMenu | Cell | Alias | Copy | Progress => ("GFX_GUI_POINTER_STANDARD", 0, 0),
        PointingHand | Grab | Grabbing => ("GFX_GUI_POINTER_HAND", 6, 10),
        Text | VerticalText => ("GFX_GUI_POINTER_IBEAM", 5, 8),
        Move | AllScroll => ("GFX_GUI_POINTER_4WAY_DRAG", 16, 23),
        ResizeHorizontal | ResizeEast | ResizeWest | ResizeColumn => ("GFX_GUI_POINTER_HOR_DRAG", 16, 7),
        ResizeVertical | ResizeNorth | ResizeSouth | ResizeRow => ("GFX_GUI_POINTER_VER_DRAG", 7, 16),
        ResizeNwSe | ResizeNorthWest | ResizeSouthEast => ("GFX_GUI_POINTER_NW_SE_DRAG", 12, 12),
        ResizeNeSw | ResizeNorthEast | ResizeSouthWest => ("GFX_GUI_POINTER_NE_SW_DRAG", 12, 12),
        Wait => ("GFX_GUI_POINTER_WAIT", 8, 12),
        Help => ("GFX_GUI_POINTER_QUESTIONMARK", 0, 0),
        NotAllowed | NoDrop => ("GFX_GUI_POINTER_CANT_DROP", 0, 0),
        _ => return Option::None,
    })
}

/// Drops the cursors made so far (the skin changed).
pub fn reset() {
    CURSORS.with(|c| c.borrow_mut().clear());
    LAST.with(|l| *l.borrow_mut() = None);
}

/// Sets the window's pointer to the skin's picture of `icon` when there is one; called after egui-winit set the system's.
pub fn apply(event_loop: &ActiveEventLoop, window: &Window, ctx: &egui::Context, icon: egui::CursorIcon) {
    if LAST.with(|l| *l.borrow() == Some(icon)) {
        return;
    }
    LAST.with(|l| *l.borrow_mut() = Some(icon));
    let Some(skin) = crate::skin::skin(ctx) else { return };
    let Some((name, hx, hy)) = sprite(icon) else { return };
    let name = if skin.has(name) { name } else { "GFX_GUI_POINTER_STANDARD" };
    let cursor = CURSORS.with(|cache| {
        cache
            .borrow_mut()
            .entry(name)
            .or_insert_with(|| {
                let (rgba, size) = crate::skin::decode(skin.bytes(name)?)?;
                let source = CustomCursor::from_rgba(rgba, size[0] as u16, size[1] as u16, hx.min(size[0] as u16 - 1), hy.min(size[1] as u16 - 1)).ok()?;
                Some(event_loop.create_custom_cursor(source))
            })
            .clone()
    });
    if let Some(cursor) = cursor {
        window.set_cursor(cursor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pointer_egui_asks_for_has_a_picture_or_none() {
        assert_eq!(sprite(egui::CursorIcon::Default).map(|s| s.0), Some("GFX_GUI_POINTER_STANDARD"));
        assert_eq!(sprite(egui::CursorIcon::Text).map(|s| s.0), Some("GFX_GUI_POINTER_IBEAM"));
        assert_eq!(sprite(egui::CursorIcon::ResizeNwSe).map(|s| (s.1, s.2)), Some((12, 12)));
        assert!(sprite(egui::CursorIcon::None).is_none());
    }
}
