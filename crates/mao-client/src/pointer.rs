//! The original's mouse pointers (`GFX_GUI_POINTER_*` in the skin atlas, 32 x 32 with the green key) in place of the
//! system's: egui says which pointer it wants each frame (`CursorIcon`); the matching sprite is made into a winit custom
//! cursor once and set on the window whenever the wanted pointer changes. Without the skin nothing is done and
//! egui-winit's system pointers stay.
//!
//! Over the world the pointer tells what is under it, as GUI.dll's `MousePointerModule_t` draws it: a coloured arrow
//! (`Pointer_e`) and up to two small pictures beside it (`Detail_e`, what the left and the right button would do), each
//! with a mouse under it showing its button, chosen by `InputConfig_t::CheckObjectUnderMouse` (0x1001a33f, [`world`]).
//! The original draws these as sprites of its own; here they are put together into one picture for the window's cursor.

use std::cell::RefCell;
use std::collections::HashMap;

use winit::event_loop::ActiveEventLoop;
use winit::window::{CustomCursor, Window};

use crate::picking::Class;

/// `MousePointerData_s::Pointer_e`, the arrow's colour: GUI.dll's table at 0x10263170 gives each its sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Colour {
    Standard,
    Blue,
    Green,
    Yellow,
    Purple,
    Red,
    Gray,
}

impl Colour {
    fn sprite(self) -> &'static str {
        match self {
            Colour::Standard => "GFX_GUI_POINTER_STANDARD",
            Colour::Blue => "GFX_GUI_POINTER_STANDARD_BLUE",
            Colour::Green => "GFX_GUI_POINTER_STANDARD_GREEN",
            Colour::Yellow => "GFX_GUI_POINTER_STANDARD_YELLOW",
            Colour::Purple => "GFX_GUI_POINTER_STANDARD_PURPLE",
            Colour::Red => "GFX_GUI_POINTER_STANDARD_RED",
            Colour::Gray => "GFX_GUI_POINTER_STANDARD_GRAY",
        }
    }
}

/// `MousePointerData_s::Detail_e`, a picture beside the arrow: GUI.dll's table at 0x102632a0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Detail {
    Attack,
    PickUp,
    Operate,
    LookAt,
    Question,
    Denied,
    Talk,
    Trade,
}

impl Detail {
    fn sprite(self) -> &'static str {
        match self {
            Detail::Attack => "GFX_GUI_POINTERDETAIL_ATTACK",
            Detail::PickUp => "GFX_GUI_POINTERDETAIL_PICKUP",
            Detail::Operate => "GFX_GUI_POINTERDETAIL_OPERATE",
            Detail::LookAt => "GFX_GUI_POINTERDETAIL_LOOKAT",
            Detail::Question => "GFX_GUI_POINTERDETAIL_QUESTION",
            Detail::Denied => "GFX_GUI_POINTERDETAIL_DENIED",
            Detail::Talk => "GFX_GUI_POINTERDETAIL_TALK",
            Detail::Trade => "GFX_GUI_POINTERDETAIL_TRADE",
        }
    }
}

/// The pointer over the world: its arrow, what the left button and the right would do, and whether the mice under the
/// pictures show (`MouseCursorMode` 1 has none).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Look {
    pub colour: Colour,
    pub left: Option<Detail>,
    pub right: Option<Detail>,
    pub buttons: bool,
}

impl Look {
    pub const STANDARD: Look = Look { colour: Colour::Standard, left: None, right: None, buttons: true };
}

/// `InputConfig_t::CheckObjectUnderMouse` (GUI.dll 0x1001a33f): the pointer over a thing in the world. One's own character:
/// green. Another player: a trade on the right button (and on the left, a double click's, while the `DoubleclickAction`
/// option is on), green in one's team, else yellow, purple on one's side and red on another when it can be fought where
/// one stands. An NPC: blue when it cannot be attacked (state flag 0x20000000) or talks, with a talk on the right button
/// when it talks, else red. Ctrl puts an attack on a character's left button. Anything else: blue, with a pick-up (`Can`
/// bit 0) or a use (bit 3) on the right button and on the left with the option. Shift puts a look at on the left button
/// of anything. (The check that the camera is not in mode 0x39, which leaves the pointer alone, is not made: no such
/// mode here.)
pub fn world(class: Class, ctrl: bool, shift: bool, double_click: bool) -> Look {
    let mut look = Look::STANDARD;
    match class {
        Class::Nothing => return look,
        Class::Own => look.colour = Colour::Green,
        Class::Npc { friendly, talks } => {
            look.colour = if friendly || talks { Colour::Blue } else { Colour::Red };
            look.right = talks.then_some(Detail::Talk);
        }
        Class::Player { mate, same_side, vulnerable } => {
            look.right = Some(Detail::Trade);
            look.left = double_click.then_some(Detail::Trade);
            look.colour = match (mate, same_side, vulnerable) {
                (true, ..) => Colour::Green,
                (false, true, true) => Colour::Purple,
                (false, false, true) => Colour::Red,
                (false, _, false) => Colour::Yellow,
            };
        }
        Class::Thing { can } => {
            let can = can.unwrap_or(0);
            look.colour = Colour::Blue;
            let detail = if can & 1 != 0 {
                Some(Detail::PickUp)
            } else if can & 8 != 0 {
                Some(Detail::Operate)
            } else {
                None
            };
            look.right = detail;
            look.left = detail.filter(|_| double_click);
        }
    }
    if ctrl && matches!(class, Class::Npc { .. } | Class::Player { .. }) {
        look.left = Some(Detail::Attack);
    }
    if shift {
        look.left = Some(Detail::LookAt);
    }
    look
}

/// The `MouseCursorMode` option (`FrameProcess` FUN_10020629): 0 shows no left picture, 1 only one picture (the left when
/// there are both) and no mice under them, 2 (the game's default) both.
pub fn with_mode(look: Look, mode: i64) -> Look {
    match mode {
        0 => Look { left: None, ..look },
        1 => Look { right: if look.left.is_some() { None } else { look.right }, buttons: false, ..look },
        _ => look,
    }
}

/// Where FUN_100202e6 puts each part of a look, from the arrow's top left (its point; the coloured arrows' hot spot is
/// their top left): the left picture 18 to the right, the right picture there too, or 27 further when both show; the
/// mouse under a picture 10 right and 20 down from it (`POINTEROVERLAY_LEFTCLICK` under the left, `_RIGHTCLICK` under
/// the right; one picture for both buttons gets both mice, the left one 9 further left), none under a denial.
fn parts(look: Look) -> Vec<(&'static str, i32, i32)> {
    let mut out = vec![(look.colour.sprite(), 0, 0)];
    let same = look.left.is_some() && look.left == look.right;
    if let Some(left) = look.left {
        out.push((left.sprite(), 18, 0));
        if look.buttons && left != Detail::Denied {
            out.push(("GFX_GUI_POINTEROVERLAY_LEFTCLICK", if same { 18 + 10 - 9 } else { 18 + 10 }, 20));
        }
    }
    if let Some(right) = look.right {
        let x = if same { 18 } else if look.left.is_some() { 18 + 27 } else { 18 };
        if !same {
            out.push((right.sprite(), x, 0));
        }
        if look.buttons && right != Detail::Denied {
            out.push(("GFX_GUI_POINTEROVERLAY_RIGHTCLICK", x + 10, 20));
        }
    }
    out
}

/// The look's parts drawn into one picture (each sprite's key already clear): RGBA and its size.
fn compose(skin: &crate::skin::Skin, look: Look) -> Option<(Vec<u8>, [usize; 2])> {
    let mut pieces = Vec::new();
    for (name, x, y) in parts(look) {
        let Some((rgba, size)) = skin.bytes(name).and_then(crate::skin::decode) else { continue };
        pieces.push((rgba, size, x as usize, y as usize));
    }
    let width = pieces.iter().map(|p| p.2 + p.1[0]).max()?;
    let height = pieces.iter().map(|p| p.3 + p.1[1]).max()?;
    let mut out = vec![0u8; width * height * 4];
    for (rgba, [w, h], x0, y0) in pieces {
        for y in 0..h {
            for x in 0..w {
                let from = (y * w + x) * 4;
                if rgba[from + 3] == 0 {
                    continue;
                }
                let to = ((y0 + y) * width + x0 + x) * 4;
                out[to..to + 4].copy_from_slice(&rgba[from..from + 4]);
            }
        }
    }
    Some((out, [width, height]))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Icon(&'static str),
    World(Look),
}

thread_local! {
    static CURSORS: RefCell<HashMap<Key, Option<CustomCursor>>> = RefCell::new(HashMap::new());
    static LAST: RefCell<Option<(egui::CursorIcon, Option<Look>)>> = const { RefCell::new(None) };
}

/// The sprite for a pointer and its hot spot (the pixel that points): GUI.dll's table at 0x10263168 (each row's first two
/// words, read 2026-11-21; the hand's, the drags' and the wait's were guessed before).
fn sprite(icon: egui::CursorIcon) -> Option<(&'static str, u16, u16)> {
    use egui::CursorIcon::*;
    Some(match icon {
        Default | ContextMenu | Cell | Alias | Copy | Progress => ("GFX_GUI_POINTER_STANDARD", 0, 0),
        PointingHand | Grab | Grabbing => ("GFX_GUI_POINTER_HAND", 5, 1),
        Text | VerticalText => ("GFX_GUI_POINTER_IBEAM", 5, 8),
        Move | AllScroll => ("GFX_GUI_POINTER_4WAY_DRAG", 16, 16),
        ResizeHorizontal | ResizeEast | ResizeWest | ResizeColumn => ("GFX_GUI_POINTER_HOR_DRAG", 16, 6),
        ResizeVertical | ResizeNorth | ResizeSouth | ResizeRow => ("GFX_GUI_POINTER_VER_DRAG", 6, 16),
        ResizeNwSe | ResizeNorthWest | ResizeSouthEast => ("GFX_GUI_POINTER_NW_SE_DRAG", 12, 12),
        ResizeNeSw | ResizeNorthEast | ResizeSouthWest => ("GFX_GUI_POINTER_NE_SW_DRAG", 12, 12),
        Wait => ("GFX_GUI_POINTER_WAIT", 16, 16),
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
/// `world` is the look over the world, used while egui wants its plain pointer (the pointer is over no window).
pub fn apply(event_loop: &ActiveEventLoop, window: &Window, ctx: &egui::Context, icon: egui::CursorIcon, world: Option<Look>) {
    let world = world.filter(|_| icon == egui::CursorIcon::Default);
    if LAST.with(|l| *l.borrow() == Some((icon, world))) {
        return;
    }
    LAST.with(|l| *l.borrow_mut() = Some((icon, world)));
    let Some(skin) = crate::skin::skin(ctx) else { return };
    let cursor = match world.filter(|l| *l != Look::STANDARD) {
        Some(look) => CURSORS.with(|cache| {
            cache
                .borrow_mut()
                .entry(Key::World(look))
                .or_insert_with(|| {
                    let (rgba, size) = compose(&skin, look)?;
                    let source = CustomCursor::from_rgba(rgba, size[0] as u16, size[1] as u16, 0, 0).ok()?;
                    Some(event_loop.create_custom_cursor(source))
                })
                .clone()
        }),
        None => {
            let Some((name, hx, hy)) = sprite(icon) else { return };
            let name = if skin.has(name) { name } else { "GFX_GUI_POINTER_STANDARD" };
            CURSORS.with(|cache| {
                cache
                    .borrow_mut()
                    .entry(Key::Icon(name))
                    .or_insert_with(|| {
                        let (rgba, size) = crate::skin::decode(skin.bytes(name)?)?;
                        let source = CustomCursor::from_rgba(rgba, size[0] as u16, size[1] as u16, hx.min(size[0] as u16 - 1), hy.min(size[1] as u16 - 1)).ok()?;
                        Some(event_loop.create_custom_cursor(source))
                    })
                    .clone()
            })
        }
    };
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

    /// CheckObjectUnderMouse's choices: a terminal (Can bit 3) is a blue arrow with a use on the right button (and on the
    /// left with `DoubleclickAction`), something to pick up the pick-up, any other thing a plain blue arrow.
    #[test]
    fn usable_things_get_the_blue_arrow_and_their_use() {
        let terminal = Class::Thing { can: Some(8 | 0x4000) };
        assert_eq!(world(terminal, false, false, true), Look { colour: Colour::Blue, left: Some(Detail::Operate), right: Some(Detail::Operate), buttons: true });
        assert_eq!(world(terminal, false, false, false), Look { colour: Colour::Blue, left: None, right: Some(Detail::Operate), buttons: true });
        assert_eq!(world(Class::Thing { can: Some(9) }, false, false, false).right, Some(Detail::PickUp));
        assert_eq!(world(Class::Thing { can: None }, false, false, true), Look { colour: Colour::Blue, ..Look::STANDARD });
        assert_eq!(world(Class::Nothing, true, true, true), Look::STANDARD);
        // (Shift: a look at on the left; Ctrl is for characters only)
        assert_eq!(world(terminal, true, true, true).left, Some(Detail::LookAt));
        assert_eq!(world(terminal, true, false, false).left, None);
    }

    #[test]
    fn characters_are_coloured_by_what_they_are_to_oneself() {
        assert_eq!(world(Class::Own, true, false, true), Look { colour: Colour::Green, ..Look::STANDARD });
        let npc = |friendly, talks| world(Class::Npc { friendly, talks }, false, false, true);
        assert_eq!((npc(false, false).colour, npc(false, false).right), (Colour::Red, None));
        assert_eq!((npc(true, false).colour, npc(true, false).right), (Colour::Blue, None));
        assert_eq!((npc(false, true).colour, npc(false, true).right), (Colour::Blue, Some(Detail::Talk)));
        let player = |mate, same_side, vulnerable| world(Class::Player { mate, same_side, vulnerable }, false, false, true);
        assert_eq!(player(false, true, false), Look { colour: Colour::Yellow, left: Some(Detail::Trade), right: Some(Detail::Trade), buttons: true });
        assert_eq!(player(false, true, true).colour, Colour::Purple);
        assert_eq!(player(false, false, true).colour, Colour::Red);
        assert_eq!(player(false, false, false).colour, Colour::Yellow);
        assert_eq!(player(true, false, true).colour, Colour::Green);
        // (Ctrl: an attack on the left button)
        assert_eq!(world(Class::Npc { friendly: false, talks: false }, true, false, false).left, Some(Detail::Attack));
    }

    /// The game's own skin put together: a terminal's look is the blue arrow with the use's cog beside it and both mice
    /// under it (`MAO_POINTER_PNG=<file>` writes the picture to look at).
    #[test]
    fn the_parts_put_together_from_the_games_skin() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let skin = crate::skin::Skin::open(&egui::Context::default(), &[std::path::Path::new(&game).join("cd_image/gui/Default")]).unwrap();
        let look = world(Class::Thing { can: Some(8) }, false, false, true);
        let (rgba, [w, h]) = compose(&skin, look).unwrap();
        assert_eq!((w, h), (50, 36));
        if let Some(out) = std::env::var_os("MAO_POINTER_PNG") {
            image::save_buffer(out, &rgba, w as u32, h as u32, image::ColorType::Rgba8).unwrap();
        }
        let any = |xs: std::ops::Range<usize>, ys: std::ops::Range<usize>| ys.clone().any(|y| xs.clone().any(|x| rgba[(y * w + x) * 4 + 3] > 0));
        // (the arrow at the point, the cog beside it, the mice under the cog)
        assert!(any(0..4, 0..4), "the arrow's point");
        assert!(any(36..50, 8..20), "the cog");
        assert!(any(19..44, 30..36), "the mice");
    }

    /// The option and where the parts go: one picture for both buttons gets both mice, two pictures sit side by side.
    #[test]
    fn the_mode_and_the_parts() {
        let both = Look { colour: Colour::Blue, left: Some(Detail::Operate), right: Some(Detail::Operate), buttons: true };
        assert_eq!(parts(both), vec![("GFX_GUI_POINTER_STANDARD_BLUE", 0, 0), ("GFX_GUI_POINTERDETAIL_OPERATE", 18, 0), ("GFX_GUI_POINTEROVERLAY_LEFTCLICK", 19, 20), ("GFX_GUI_POINTEROVERLAY_RIGHTCLICK", 28, 20)]);
        let two = Look { left: Some(Detail::Attack), right: Some(Detail::Talk), ..both };
        assert_eq!(parts(two)[3..], [("GFX_GUI_POINTERDETAIL_TALK", 45, 0), ("GFX_GUI_POINTEROVERLAY_RIGHTCLICK", 55, 20)]);
        assert_eq!(with_mode(two, 0), Look { left: None, ..two });
        assert_eq!(with_mode(two, 1), Look { right: None, buttons: false, ..two });
        assert_eq!(with_mode(Look { left: None, ..two }, 1).right, Some(Detail::Talk));
        assert_eq!(parts(with_mode(both, 1)).len(), 2);
    }
}
