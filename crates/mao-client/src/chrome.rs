//! Windows in the original's chrome. [`window`] is used in place of `egui::Window` by every window of the interface: when the
//! skin's pictures are there (`skin`), the window has no egui title bar and is drawn with the original's frame pieces
//! (`GFX_GUI_WINDOW3_BORDER_*`, greyscale masks tinted with the theme's border colour, as the original tints them with its
//! `GUIColors.xml`), the title inside the frame at the top left and the original's pin and close buttons at the top right;
//! dragging anywhere on the window moves it (egui's own fallback without a title bar). Without the pictures it is the plain
//! egui window it was.
//!
//! A pinned window (the pin button, as in the original) is left open by Escape; `pinned` tells `Ui::close_with_escape`. Each
//! window starts where the player left it (`layout.rs`), and closes the next time it is shown after Escape asked ([`close_soon`]).

use std::sync::Mutex;

use crate::theme::Palette;

/// The windows whose pin is down, by their name.
static PINNED: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn pinned(name: &str) -> bool {
    PINNED.lock().unwrap().iter().any(|n| n == name)
}

fn set_pinned(name: &str, on: bool) {
    let mut list = PINNED.lock().unwrap();
    list.retain(|n| n != name);
    if on {
        list.push(name.to_string());
    }
}

/// The windows Escape asked to close (`Ui::close_with_escape`): each closes the next time it is shown, as its close button would
/// close it (so a shop or an NPC chat tells the server); what is left at the end of the frame was not open and is dropped.
static CLOSING: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn close_soon(name: &str) {
    CLOSING.lock().unwrap().push(name.to_string());
}

/// The end of a frame: Escape's requests not taken (the windows not shown) are dropped.
pub fn end_frame() {
    CLOSING.lock().unwrap().clear();
}

pub(crate) fn take_closing(name: &str) -> bool {
    let mut list = CLOSING.lock().unwrap();
    let before = list.len();
    list.retain(|n| n != name);
    list.len() != before
}

/// The message boxes' window names ([`dialog`]): none keeps a place, and Escape closes each with `esc_dialogs`.
pub const DIALOGS: &[&str] = &["TradeDoubleConfirmDlg", "perk-dialog", "team-invite", "remove-friend", "ignore-friend", "delete-mail", "disable-tips", "delete-mission", "delete-item", "reset-skill", "rename-chat-window", "private-invite", "rename-backpack", "DuelChallenge"];

/// When each message box was last shown (the frame), to tell a new showing from the next frame of one.
static DIALOG_SEEN: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());

/// A box that asks for a text takes the keys when it comes up: GUI.dll's `TextInputDialogForm_c::AllAttached`
/// (0x101483d8) gives its text view the focus. Called every frame with the box's field, it asks for the focus on the
/// first frame of a showing; `name` is one of [`DIALOGS`].
pub fn focus_when_new(ctx: &egui::Context, name: &'static str, field: &egui::Response) {
    static SEEN: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());
    let frame = ctx.cumulative_frame_nr();
    let mut seen = SEEN.lock().unwrap();
    let was = seen.iter().find(|s| s.0 == name).map(|s| s.1);
    seen.retain(|s| s.0 != name);
    seen.push((name, frame));
    if was.is_none_or(|f| f + 1 < frame) {
        field.request_focus();
    }
}

/// The width a message box's text is wrapped to: `TextRenderer_c::SetAspectRatio(2.0)`, about twice as wide as tall.
pub fn dialog_width(height_at: impl Fn(f32) -> f32) -> f32 {
    let mut width = 120.0;
    while width < 600.0 && width < 2.0 * height_at(width) {
        width += 20.0;
    }
    width
}

/// A message box, GUI.dll's `DialogBox_c` (`_Init` 0x1012a7a8, `DialogBoxView_c` 0x1012aed3, `DialogButtons_c::_Initialize`
/// 0x1012b20f): a window that comes up at the pointer (`MoveToMouse`) holding its text (a `TextView_c` in font 5 with
/// markup, 15 from the sides, 5 from the top and 20 over the buttons, wrapped about twice as wide as tall) over its
/// buttons in a row 5 from the bottom, each 8 from the next: at the right, or in the middle when there is only one.
/// `Some(Some(k))` when button `k` was pressed, `Some(None)` when it was closed without one (its close button, or Escape
/// with `esc_dialogs`: `SlotSelected(-1)`), `None` while it waits. `name` is one of [`DIALOGS`].
pub fn dialog(ctx: &egui::Context, palette: &Palette, name: &'static str, title: &str, text: &str, buttons: &[&str]) -> Option<Option<usize>> {
    debug_assert!(DIALOGS.contains(&name), "{name} is not listed in chrome::DIALOGS");
    let frame = ctx.cumulative_frame_nr();
    let fresh = {
        let mut seen = DIALOG_SEEN.lock().unwrap();
        let was = seen.iter().find(|s| s.0 == name).map(|s| s.1);
        seen.retain(|s| s.0 != name);
        seen.push((name, frame));
        was.is_none_or(|f| f + 1 < frame)
    };
    let at = ctx.input(|i| i.pointer.latest_pos()).unwrap_or(ctx.content_rect().center());
    let font = crate::gui::font::FontId::NORMAL;
    let width = dialog_width(|w| crate::gui::controls::markup_height(ctx, text, font, w));
    let mut open = true;
    let mut answer = None;
    window(
        ctx,
        palette,
        name,
        title,
        &mut open,
        |w| {
            let w = w.resizable(false).pivot(egui::Align2::CENTER_CENTER);
            if fresh { w.current_pos(at) } else { w }
        },
        |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.allocate_ui_with_layout(egui::vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    crate::gui::controls::markup(ui, palette, text, font, rgb(palette.text));
                });
                ui.add_space(15.0);
            });
            ui.add_space(20.0);
            let row = |ui: &mut egui::Ui, answer: &mut Option<usize>| {
                for (k, label) in buttons.iter().enumerate().rev() {
                    ui.add_space(8.0);
                    if button(ui, palette, label, true).clicked() {
                        *answer = Some(k);
                    }
                    ui.add_space(8.0);
                }
            };
            if buttons.len() == 1 {
                ui.vertical_centered(|ui| {
                    if button(ui, palette, buttons[0], true).clicked() {
                        answer = Some(0);
                    }
                });
            } else {
                ui.allocate_ui_with_layout(egui::vec2(width + 30.0, 0.0), egui::Layout::right_to_left(egui::Align::Center), |ui| row(ui, &mut answer));
            }
            ui.add_space(5.0);
        },
    );
    match answer {
        Some(k) => Some(Some(k)),
        None if !open => Some(None),
        None => None,
    }
}

/// The texts of a yes/no message box's buttons (text.mdb 10000's `MsgBox_Yes` and `MsgBox_No`).
pub fn yes_no() -> [String; 2] {
    [crate::gui::xml::ldb_named(10000, "MsgBox_Yes", "Yes"), crate::gui::xml::ldb_named(10000, "MsgBox_No", "No")]
}

/// A yes/no message box ([`dialog`]): `Some(true)` for Yes, `Some(false)` for No or when it was closed.
pub fn question(ctx: &egui::Context, palette: &Palette, name: &'static str, title: &str, text: &str) -> Option<bool> {
    let [yes, no] = yes_no();
    dialog(ctx, palette, name, title, text, &[&yes, &no]).map(|a| a == Some(0))
}

/// A right-click menu as the original's `PopupMenu_c` (`gui::popup`), for the client's windows: `entries` are the labels
/// and whether each can be chosen.
pub fn menu_at(ctx: &egui::Context, at: egui::Pos2, entries: &[(String, bool)]) -> crate::gui::popup::PopupMenu {
    let entries = entries.iter().map(|(label, enabled)| crate::gui::popup::Entry { label: label.clone(), enabled: *enabled, mark: None }).collect();
    crate::gui::popup::PopupMenu::new(entries, at, None, ctx.cumulative_frame_nr())
}

/// A menu whose entries may have check marks ([`menu_at`]): label, whether it can be chosen, and the mark.
pub fn menu_marked(ctx: &egui::Context, at: egui::Pos2, entries: &[(String, bool, Option<bool>)]) -> crate::gui::popup::PopupMenu {
    let entries = entries.iter().map(|(label, enabled, mark)| crate::gui::popup::Entry { label: label.clone(), enabled: *enabled, mark: *mark }).collect();
    crate::gui::popup::PopupMenu::new(entries, at, None, ctx.cumulative_frame_nr())
}

/// A frame of an open menu ([`menu_at`]): the entry chosen, if one was; the menu is gone once it closes.
pub fn menu(ctx: &egui::Context, palette: &Palette, name: &str, menu: &mut Option<crate::gui::popup::PopupMenu>) -> Option<usize> {
    let open = menu.as_mut()?;
    let skin = crate::skin::skin(ctx);
    let measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
    let env = crate::gui::host::Env { skin: skin.as_deref(), palette, measure: &measure };
    match open.show(ctx, &env, egui::Id::unique(name)) {
        crate::gui::popup::Outcome::None => None,
        crate::gui::popup::Outcome::Chosen(k) => {
            *menu = None;
            Some(k)
        }
        crate::gui::popup::Outcome::Closed => {
            *menu = None;
            None
        }
    }
}

pub fn rgb(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

/// The frame's margins: the title row at the top, the border pieces around.
const LEFT: i8 = 7;
const RIGHT: i8 = 7;
const TOP: i8 = 24;
const BOTTOM: i8 = 10;

/// A window named `name` (its egui id and its `layout.ini` entry) titled `title`, shown while `*open`; `build` sets the egui window's
/// size and place (`|w| w.resizable(false).default_pos(..)`), `add` fills it. Returns what `egui::Window::show` returns.
pub fn window<R>(ctx: &egui::Context, palette: &Palette, name: &str, title: &str, open: &mut bool, build: impl FnOnce(egui::Window<'static>) -> egui::Window<'static>, add: impl FnOnce(&mut egui::Ui) -> R) -> Option<egui::InnerResponse<Option<R>>> {
    if *open && take_closing(name) {
        *open = false;
    }
    // (a window docked in the rollup area is drawn as a page of it: `rollup.rs`)
    if *open && crate::rollup::docked(name) {
        return crate::rollup::page(ctx, palette, name, title, open, add);
    }
    if !*open {
        return None;
    }
    // (a movable window waits for the screen's real size — egui's first frames report a placeholder — so its starting
    // place and size are not taken from a 10000-point screen)
    if crate::layout::kept(name) && crate::layout::screen().is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none() {
        return None;
    }
    // (the place the player left it last time, `layout.ini`: over the caller's starting place)
    crate::layout::track(name);
    let saved = crate::layout::kept(name).then(|| crate::layout::saved(name)).flatten();
    // (the original client's layout gives a size too, until the player has placed the window here)
    let sized = crate::layout::kept(name).then(|| crate::layout::original_size(name)).flatten();
    let relocate = crate::layout::relocate(name);
    let sizable = std::cell::Cell::new(false);
    let sizable = &sizable;
    let build = move |w: egui::Window<'static>| {
        let w = build(w);
        // (a window the player can size starts at the size it was left at, `sizes.ini`: over the original's frame and
        // the caller's own size)
        let w = w.resize(|r| {
            sizable.set(r.is_resizable().any());
            r
        });
        let sized = sizable.get().then(|| crate::layout::saved_size(name)).flatten().or(sized);
        let w = match sized {
            Some(size) => w.default_size(size),
            None => w,
        };
        match saved {
            Some(at) if relocate => w.current_pos(at),
            Some(at) => w.default_pos(at),
            None => w,
        }
    };
    let accent = rgb(palette.accent);
    let id = egui::Id::unique(name);
    let skin = crate::skin::skin(ctx).filter(|s| s.has("GFX_GUI_WINDOW3_BORDER_TL"));
    let Some(skin) = skin else {
        let mut is_open = *open;
        let shown = build(egui::Window::new(egui::RichText::new(title).color(accent)).collapsible(false)).id(id).open(&mut is_open).show(ctx, |ui| {
            let room = ui.max_rect().size();
            let result = add(ui);
            // (the size kept: the room and what egui's own frame and title add round the content)
            if sizable.get()
                && let Some(outer) = ctx.memory(|m| m.area_rect(id)).filter(|r| r.is_finite())
            {
                crate::layout::note_size(name, room + (outer.size() - ui.min_rect().size()).max(egui::Vec2::ZERO));
            }
            result
        });
        *open = is_open;
        return shown;
    };
    if !*open {
        return None;
    }
    let panel = egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], 225);
    let hover = rgb(palette.hover);
    let frame = egui::Frame::NONE.inner_margin(egui::Margin { left: LEFT, right: RIGHT, top: TOP, bottom: BOTTOM });
    let mut close = false;
    let title = title.to_string();
    let name = name.to_string();
    let shown = build(egui::Window::new(title.as_str()).title_bar(false).collapsible(false).frame(frame)).id(id).show(ctx, |ui| {
        let painter = ui.painter().clone();
        // the frame goes under the content: its place in the paint list is kept, it is drawn once the size is known
        let under = painter.add(egui::Shape::Noop);
        let room = ui.max_rect().size();
        let result = add(ui);
        // (the size kept: the room the window gives its content, and the frame)
        if sizable.get() {
            crate::layout::note_size(&name, room + egui::vec2((LEFT + RIGHT) as f32, (TOP + BOTTOM) as f32));
        }
        // The whole window as laid out: egui's record of the area (last frame's for a window that just appeared, hence the fallback
        // from the content).
        let content = ui.min_rect();
        let outer = ctx.memory(|m| m.area_rect(id)).filter(|r| r.is_finite() && r.width() > 20.0).unwrap_or_else(|| egui::Rect::from_min_max(content.min - egui::vec2(LEFT as f32, TOP as f32), content.max + egui::vec2(RIGHT as f32, BOTTOM as f32)));
        painter.set(under, frame_shape(&skin, outer, panel, accent));
        // the title row (the title cut where the buttons begin: a narrow window's long title does not run under them)
        let mut x = outer.right() - RIGHT as f32 - 15.0;
        // (a dockable window has a third button: into the rollup area down the right edge, `rollup.rs`)
        let dockable = crate::rollup::DOCKABLE.contains(&name.as_str());
        let buttons_left = x - 17.0 * if dockable { 2.0 } else { 1.0 } - 3.0;
        let title_room = egui::Rect::from_min_max(outer.min, egui::pos2(buttons_left.max(outer.left()), outer.bottom()));
        crate::font::text(ctx, &painter.with_clip_rect(title_room.intersect(painter.clip_rect())), egui::pos2(outer.left() + 9.0, outer.top() + 5.0), &title, 15.0, accent);
        for (button, what) in [("CLOSE_X", "close"), ("PIN", "pin"), ("ROLLUP_COLLAPSE", "dock")] {
            if what == "dock" && !dockable {
                break;
            }
            let rect = egui::Rect::from_min_size(egui::pos2(x, outer.top() + 5.0), egui::vec2(15.0, 15.0));
            let response = ui.interact(rect, id.with(what), egui::Sense::click());
            let down = what == "pin" && pinned(&name);
            let state = if response.is_pointer_button_down_on() || down { "_STATE3" } else if response.hovered() { "_STATE2" } else { "" };
            let tint = if response.hovered() { hover } else if down { rgb(palette.text_selected) } else { accent };
            let drawn = if what == "dock" {
                skin.draw_in(&painter, &format!("GFX_GUI_{button}{}", if state.is_empty() { "_STATE1" } else { state }), rect, tint)
            } else {
                skin.draw_in(&painter, &format!("GFX_GUI_WINDOW_{button}{state}"), rect, tint)
            };
            if !drawn {
                skin.draw_in(&painter, &format!("GFX_GUI_WINDOW_{button}"), rect, tint);
            }
            if response.clicked() {
                match what {
                    "pin" => set_pinned(&name, !down),
                    "dock" => crate::rollup::dock(&name),
                    _ => close = true,
                }
            }
            response.on_hover_text(match what {
                "pin" => "Pinned: stays open when Escape closes the windows",
                "dock" => "Dock in the column at the right edge",
                _ => "Close",
            });
            x -= 17.0;
        }
        result
    });
    if close {
        *open = false;
    }
    shown
}

/// A window without the frame and the title, only a background `solid` (0 to 1) of the panel's colour: a chat window
/// in its borderless mode. Its place and size are kept under `name` as a framed window's are, so one can be turned
/// into the other where it stands.
pub fn bare_window(ctx: &egui::Context, palette: &Palette, name: &str, solid: f32, build: impl FnOnce(egui::Window<'static>) -> egui::Window<'static>, add: impl FnOnce(&mut egui::Ui)) {
    if crate::layout::kept(name) && crate::layout::screen().is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none() {
        return;
    }
    crate::layout::track(name);
    let saved = crate::layout::kept(name).then(|| crate::layout::saved(name)).flatten();
    let fill = egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], (solid.clamp(0.0, 1.0) * 255.0) as u8);
    const MARGIN: i8 = 4;
    let mut w = build(egui::Window::new(name).title_bar(false).collapsible(false).frame(egui::Frame::NONE.fill(fill).inner_margin(MARGIN))).id(egui::Id::unique(name));
    if let Some(size) = crate::layout::saved_size(name) {
        w = w.default_size(size);
    }
    w = match saved {
        Some(at) if crate::layout::relocate(name) => w.current_pos(at),
        Some(at) => w.default_pos(at),
        None => w,
    };
    w.show(ctx, |ui| {
        let room = ui.max_rect().size();
        add(ui);
        crate::layout::note_size(name, room + egui::Vec2::splat(f32::from(MARGIN) * 2.0));
    });
}

/// The window's fill and the original's border pieces around `outer`: thin top and sides, rounded bottom corners.
pub(crate) fn frame_shape(skin: &crate::skin::Skin, outer: egui::Rect, panel: egui::Color32, tint: egui::Color32) -> egui::Shape {
    let mut shapes = vec![egui::Shape::rect_filled(outer.shrink(2.0), 2.0, panel)];
    let size = |name: &str| skin.get(&format!("GFX_GUI_WINDOW3_BORDER_{name}")).map(|s| s.size);
    let (Some(tl), Some(tr), Some(bl), Some(br), Some(top), Some(bottom), Some(left), Some(right)) = (size("TL"), size("TR"), size("BL"), size("BR"), size("TOP"), size("BOTTOM"), size("LEFT"), size("RIGHT")) else {
        shapes.push(egui::Shape::rect_stroke(outer, 2.0, egui::Stroke::new(1.0, tint), egui::StrokeKind::Inside));
        return egui::Shape::Vec(shapes);
    };
    let mut piece = |name: &str, rect: egui::Rect| {
        if let Some(sprite) = skin.get(&format!("GFX_GUI_WINDOW3_BORDER_{name}")) {
            shapes.push(egui::Shape::image(sprite.texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint));
        }
    };
    let (l, t, r, b) = (outer.left(), outer.top(), outer.right(), outer.bottom());
    piece("TL", egui::Rect::from_min_size(egui::pos2(l, t), tl));
    piece("TOP", egui::Rect::from_min_max(egui::pos2(l + tl.x, t), egui::pos2(r - tr.x, t + top.y)));
    piece("TR", egui::Rect::from_min_size(egui::pos2(r - tr.x, t), tr));
    piece("LEFT", egui::Rect::from_min_max(egui::pos2(l, t + tl.y), egui::pos2(l + left.x, b - bl.y)));
    piece("RIGHT", egui::Rect::from_min_max(egui::pos2(r - right.x, t + tr.y), egui::pos2(r, b - br.y)));
    piece("BL", egui::Rect::from_min_size(egui::pos2(l, b - bl.y), bl));
    piece("BOTTOM", egui::Rect::from_min_max(egui::pos2(l + bl.x, b - bottom.y), egui::pos2(r - br.x, b)));
    piece("BR", egui::Rect::from_min_size(egui::pos2(r - br.x, b - br.y), br));
    egui::Shape::Vec(shapes)
}

/// What `ScrollBar_c` (GUI.dll) is doing with the pointer: pressing an arrow (`HitItem_e` 1 the up, 2 the down; held, it repeats
/// after half a second every 0.08 s, `_SetButtonState` 0x10141175 / `SlotRepeatTimer` 0x10141d21) or dragging the knob (3, the
/// point grabbed below its top, `MouseDown` 0x101411fe).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ScrollPress {
    item: u8,
    grab: f32,
    repeat_at: f64,
}

/// The knob of a vertical `ScrollBar_c` in a bar `top`..`bottom` (screen points, the bottom exclusive) whose arrows are `up` and
/// `down` high, showing `proportion` of the content at `fraction` of the way down (`_CalcKnobSizeAndPos` 0x10141ab9): between
/// the arrows less a point each side, `floor(proportion × length)` long but at least 8 (less than the length when that is
/// short), placed `floor((length − knob) × fraction)` down; its top and its exclusive bottom (the original's frames are
/// inclusive, so its knob is a point longer than the number).
pub fn knob(top: f32, bottom: f32, up: f32, down: f32, proportion: f32, fraction: f32) -> (f32, f32) {
    let start = top + up;
    let length = (bottom - down - 1.0) - start;
    let mut size = (proportion * length).floor();
    if size < 8.0 {
        size = if length - 1.0 < 8.0 { length - 1.0 } else { 8.0 };
    }
    let at = start + ((length - size) * fraction.clamp(0.0, 1.0)).floor();
    (at, at + size + 1.0)
}

/// A vertical scroll area with the scroll bar of the original's scroll views (`ScrollView_c::_UpdateScrollBars` 0x10142b68 makes
/// a `ScrollBar_c` with `GFX_GUI_SIMPLE_SB_UP`, `_DOWN` and `_SLIDER_VERT` and no track, a point right of the content, as wide
/// as its arrows), tinted with the theme's border colour as the frame is: an arrow scrolls 5 points (`ScrollBar_c`'s small step,
/// 0.05 of the 0..100 its scroll view makes it with), a click above or below the knob 20 (its large step, 0.2 of it), the knob
/// drags. egui's own bar is hidden; without the skin's pictures it is egui's scroll area.
pub fn scroll<R>(ui: &mut egui::Ui, palette: &Palette, area: egui::ScrollArea, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_SIMPLE_SB_UP"));
    let Some(skin) = skin else { return area.show(ui, add).inner };
    let (Some(up), Some(down)) = (skin.get("GFX_GUI_SIMPLE_SB_UP"), skin.get("GFX_GUI_SIMPLE_SB_DOWN")) else { return area.show(ui, add).inner };
    let width = up.size.x;
    let key = ui.make_persistent_id(("mao-scroll", ui.next_auto_id()));
    let ctx = ui.ctx().clone();
    let pending = ctx.data_mut(|d| d.remove_temp::<f32>(key.with("to")));
    let mut area = area.scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).content_margin(egui::Margin { left: 0, right: (width + 1.0) as i8, top: 0, bottom: 0 });
    if let Some(to) = pending {
        area = area.vertical_scroll_offset(to);
    }
    let out = area.show(ui, add);
    let inner = out.inner_rect;
    let overflow = out.content_size.y - inner.height();
    if overflow <= 0.5 {
        return out.inner;
    }
    let bar = egui::Rect::from_min_max(egui::pos2(inner.right() - width, inner.top()), egui::pos2(inner.right(), inner.bottom()));
    let offset = out.state.clamped_offset().y.clamp(0.0, overflow);
    let (k0, k1) = knob(bar.top(), bar.bottom(), up.size.y, down.size.y, inner.height() / out.content_size.y, offset / overflow);
    let up_rect = egui::Rect::from_min_size(bar.min, up.size);
    let down_rect = egui::Rect::from_min_size(egui::pos2(bar.left(), bar.bottom() - down.size.y), down.size);
    let knob_rect = egui::Rect::from_min_max(egui::pos2(bar.left(), k0), egui::pos2(bar.right(), k1));
    let response = ui.interact(bar, key.with("bar"), egui::Sense::click_and_drag());
    let now = ctx.input(|i| i.time);
    let mut press: ScrollPress = ctx.data(|d| d.get_temp(key.with("press"))).unwrap_or_default();
    let mut to = None;
    if response.is_pointer_button_down_on() {
        let at = response.interact_pointer_pos().unwrap_or(bar.center());
        if response.drag_started() || (press.item == 0 && ctx.input(|i| i.pointer.primary_pressed())) {
            if knob_rect.contains(at) {
                press = ScrollPress { item: 3, grab: at.y - k0, repeat_at: 0.0 };
            } else if up_rect.contains(at) {
                press = ScrollPress { item: 1, grab: 0.0, repeat_at: now + 0.5 };
                to = Some(offset - 5.0);
            } else if down_rect.contains(at) {
                press = ScrollPress { item: 2, grab: 0.0, repeat_at: now + 0.5 };
                to = Some(offset + 5.0);
            } else {
                press = ScrollPress { item: 4, grab: 0.0, repeat_at: 0.0 };
                to = Some(if at.y < k0 { offset - 20.0 } else { offset + 20.0 });
            }
        } else if press.item == 3 {
            // (`MouseMove` 0x101416a0: the knob's top over the room it moves in)
            let (start, end) = (bar.top() + up.size.y, bar.bottom() - down.size.y - 1.0 - (k1 - k0 - 1.0));
            if end > start {
                to = Some(overflow * (at.y - press.grab - start) / (end - start));
            }
        } else if (press.item == 1 || press.item == 2) && now >= press.repeat_at {
            let over = if press.item == 1 { up_rect.contains(at) } else { down_rect.contains(at) };
            if over {
                to = Some(offset + if press.item == 1 { -5.0 } else { 5.0 });
            }
            press.repeat_at = now + 0.08;
        }
        ctx.request_repaint();
    } else {
        press = ScrollPress::default();
    }
    ctx.data_mut(|d| d.insert_temp(key.with("press"), press));
    if let Some(to) = to {
        ctx.data_mut(|d| d.insert_temp(key.with("to"), to.clamp(0.0, overflow).floor()));
        ctx.request_repaint();
    }
    let tint = rgb(palette.accent);
    let painter = ui.painter();
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_UP", up_rect, tint);
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_DOWN", down_rect, tint);
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_SLIDER_VERT", knob_rect, tint);
    out.inner
}

/// The colour of colour code 0x1000000 (`GUIColors.xml`'s `Default`) for a control's art and labels: `Default` itself, but on a
/// dark theme (the AOU Translucent's `Default` is near black, 0x051a2a) the palette's border colour, its `Selected`, as the
/// palette already draws the window frames (`Palette::from_gui_colors`), so the controls stay visible.
pub fn code1(palette: &Palette) -> crate::theme::Rgb {
    if dark(palette) { palette.accent } else { palette.default }
}

/// Whether the theme's `Default` is dark, the palette then bordering in `Selected` (`Palette::from_gui_colors`).
fn dark(palette: &Palette) -> bool {
    palette.accent != palette.default
}

/// The size `Button_c::CalculatePreferredSize` (GUI.dll 0x10128396) gives a button whose label is `label` big and whose border
/// pieces are `left`, `top`, `right`, `bottom` (`BorderView_c::GetBorderSizes`: the left and right pieces' widths, the top and
/// bottom's heights): the label plus the sides and 10, and the label plus the top and bottom and 4.
pub fn button_size(label: egui::Vec2, left: f32, top: f32, right: f32, bottom: f32) -> egui::Vec2 {
    egui::vec2(label.x + left + right + 10.0, label.y + top + bottom + 4.0)
}

/// A `Button_c` (GUI.dll 0x10129021) in the skin's art labelled `text`: `Initialize` (0x10128e46) makes it a `BorderView_c` of
/// `GFX_GUI_BORDER01_*` in the theme's `Default` colour, its label in font 5 (Verdana 13) in the same colour and centred;
/// `StateChanged` (0x101287ea) swaps in `GFX_GUI_BORDER04_*` in `Selected` (and the label in `Selected`) while it is pressed,
/// lays the `GFX_GUI_BORDER03_*` frame in `Hover` over it while the pointer is on it, and greys the whole by 0x909090 when it
/// is disabled. Without the pictures it is egui's button.
pub fn button(ui: &mut egui::Ui, palette: &Palette, text: &str, enabled: bool) -> egui::Response {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_BORDER01_TL") && s.has("GFX_GUI_BORDER04_TL"));
    let Some(skin) = skin else { return ui.add_enabled(enabled, egui::Button::new(text)) };
    let size = |name: &str| skin.get(name).map(|s| s.size).unwrap_or(egui::Vec2::ZERO);
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, egui::Color32::WHITE);
    let want = button_size(galley.size(), size("GFX_GUI_BORDER01_LEFT").x, size("GFX_GUI_BORDER01_TOP").y, size("GFX_GUI_BORDER01_RIGHT").x, size("GFX_GUI_BORDER01_BOTTOM").y);
    let (rect, response) = ui.allocate_exact_size(want, if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let pressed = enabled && response.is_pointer_button_down_on();
    let grey = |c: egui::Color32| if enabled { c } else { c * egui::Color32::from_rgb(0x90, 0x90, 0x90) };
    let colour = grey(rgb(if pressed { palette.selected } else { code1(palette) }));
    let painter = ui.painter();
    skin.nine_slice(painter, if pressed { "GFX_GUI_BORDER04" } else { "GFX_GUI_BORDER01" }, rect, colour, true);
    if enabled && response.hovered() {
        skin.nine_slice(painter, "GFX_GUI_BORDER03", rect, grey(rgb(palette.hover)), false);
    }
    painter.galley_with_override_text_color(rect.center() - galley.size() * 0.5, galley, colour);
    response
}

/// A `CheckBox_c` (GUI.dll, `Initialize` 0x10129eb6) in the skin's art: a toggle `Button_c` whose only picture is
/// `GFX_GUI_CHECKBOX_UNCHECKED` at rest (in the theme's `Default`) and `GFX_GUI_CHECKBOX_CHECKED` when on (the pressed state, in
/// `Selected`), nothing laid over it under the pointer, then its label 10 points to the right in Verdana 13 in `Default`. A
/// click flips `on`; the response is `changed()` then. Without the pictures it is egui's checkbox.
pub fn checkbox(ui: &mut egui::Ui, palette: &Palette, on: &mut bool, text: &str) -> egui::Response {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_CHECKBOX_CHECKED") && s.has("GFX_GUI_CHECKBOX_UNCHECKED"));
    let Some(skin) = skin else { return ui.checkbox(on, text) };
    let boxed = skin.get(if *on { "GFX_GUI_CHECKBOX_CHECKED" } else { "GFX_GUI_CHECKBOX_UNCHECKED" }).map(|s| s.size).unwrap_or(egui::vec2(11.0, 11.0));
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, egui::Color32::WHITE);
    let height = boxed.y.max(galley.size().y);
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(boxed.x + 10.0 + galley.size().x, height), egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let at = egui::Rect::from_min_size(egui::pos2(rect.left(), rect.center().y - boxed.y * 0.5), boxed);
    skin.draw_in(ui.painter(), if *on { "GFX_GUI_CHECKBOX_CHECKED" } else { "GFX_GUI_CHECKBOX_UNCHECKED" }, at, rgb(if *on { palette.selected } else { code1(palette) }));
    ui.painter().galley_with_override_text_color(egui::pos2(at.right() + 10.0, rect.center().y - galley.size().y * 0.5), galley, rgb(code1(palette)));
    response
}

/// A `TextInputView_c` (GUI.dll, `Initialize` 0x10148aff) in the skin's art: its text inside a `BorderView_c` with the class's
/// own pieces (`BorderView_c::BorderView_c` 0x10126da4: `GFX_GUI_INSET_TL`, `_TOP`, `_TR`, `_LEFT`, `_RIGHT`, `_BL`, `_BOTTOM`,
/// `_BR`, no background), the text 3 points in from the sides and 1 from the top and bottom (`BorderView_c::SetClient`); the
/// pieces in the theme's `Default` (the field sets no colour of its own and takes its window's). Without the pictures it is
/// egui's text field.
pub fn text_edit(ui: &mut egui::Ui, palette: &Palette, edit: egui::TextEdit<'_>) -> egui::Response {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_INSET_TL"));
    let Some(skin) = skin else { return ui.add(edit) };
    let size = |name: &str| skin.get(name).map(|s| s.size).unwrap_or(egui::Vec2::ZERO);
    let (left, top, right, bottom) = (size("GFX_GUI_INSET_LEFT").x, size("GFX_GUI_INSET_TOP").y, size("GFX_GUI_INSET_RIGHT").x, size("GFX_GUI_INSET_BOTTOM").y);
    let margin = egui::Margin { left: (left + 3.0) as i8, right: (right + 3.0) as i8, top: (top + 1.0) as i8, bottom: (bottom + 1.0) as i8 };
    let response = ui.add(edit.frame(egui::Frame::NONE).margin(margin));
    skin.nine_slice(ui.painter(), "GFX_GUI_INSET", response.rect, rgb(code1(palette)), false);
    response
}

/// The width a strip of these tabs takes when none is narrowed ([`tab_view`]'s own widths: each tab its text's width plus
/// a point and its 5 and 16 point margins, overlapping the one before by 6, 2 points in and 2 to spare), for a window
/// that sizes itself by its tabs.
pub fn tabs_width(ui: &egui::Ui, names: &[&str]) -> f32 {
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let widths: Vec<f32> = names.iter().map(|n| ui.painter().layout_no_wrap(n.to_string(), font.clone(), egui::Color32::WHITE).size().x.ceil() + 1.0 + 5.0 + 16.0).collect();
    tab_places(&widths, f32::MAX).last().map_or(40.0, |(x, w)| x + w + 2.0) + 2.0
}

/// Where `TopBorderView_c` (GUI.dll, laid out by FUN_10147438) puts tabs `widths` wide in a strip `room` wide: the first 2 points
/// in, each overlapping the one before by 6, every tab its own width (a `Tab`'s text width plus a point plus its 5 and 16 point
/// margins, FUN_101463a4) when they fit, else all narrowed in proportion (`View::SpaceOut`'s share; not followed exactly). The
/// tabs' left edges and widths.
pub fn tab_places(widths: &[f32], room: f32) -> Vec<(f32, f32)> {
    let total: f32 = widths.iter().sum::<f32>() - 6.0 * widths.len().saturating_sub(1) as f32;
    let space = room - 2.0;
    let scale = if total > space && total > 0.0 { (space / total).max(0.2) } else { 1.0 };
    let mut x = 2.0;
    widths
        .iter()
        .map(|w| {
            let w = (w * scale).floor();
            let at = (x, w);
            x += w - 6.0;
            at
        })
        .collect()
}

/// A `TabView` (GUI.dll 0x10146767) in the skin's art with `names` for tabs, `selected` the open one, `add` filling it: its strip
/// (`TopBorderView_c`, FUN_10147b0f / FUN_1014706e: `GFX_GUI_TAB_BORDER_TL`, `_TOP`, `_TR` along its foot) with the tabs (`Tab`,
/// FUN_1014694d: `BorderView_c`s of `GFX_GUI_TAB_INACTIVE_LEFT/MIDDLE/RIGHT` at 0.6 of the alpha with black text and the open
/// one `GFX_GUI_TAB_ACTIVE_*` in front with white text, FUN_101462d8; the text in font 5, Verdana 13, from 5 points in), then
/// the body (`TabView::SetRenderFlags` 0x101464c4 and `LayoutBorders` 0x10145dad: `GFX_GUI_TAB_BORDER_LEFT`, `_RIGHT`, `_BL`,
/// `_BOTTOM`, `_BR` round it and `GFX_GUI_TAB_BACKGROUND` inside, the content 2 points inside the borders), all tinted with the
/// theme's border colour. A tab opens when it is pressed; `add` is told which is open. Without the pictures it is a row of egui's
/// selectable labels.
pub fn tab_view<R>(ui: &mut egui::Ui, palette: &Palette, names: &[&str], selected: &mut usize, add: impl FnOnce(&mut egui::Ui, usize) -> R) -> R {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_TAB_ACTIVE_MIDDLE") && s.has("GFX_GUI_TAB_BORDER_LEFT"));
    let Some(skin) = skin else {
        ui.horizontal(|ui| {
            for (i, name) in names.iter().enumerate() {
                if ui.selectable_label(*selected == i, *name).clicked() {
                    *selected = i;
                }
            }
        });
        return add(ui, *selected);
    };
    let tint = rgb(palette.accent);
    let size = |name: &str| skin.get(name).map(|s| s.size).unwrap_or(egui::vec2(1.0, 1.0));
    let height = size("GFX_GUI_TAB_ACTIVE_MIDDLE").y;
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let widths: Vec<f32> = names.iter().map(|n| ui.painter().layout_no_wrap(n.to_string(), font.clone(), egui::Color32::WHITE).size().x.ceil() + 1.0 + 5.0 + 16.0).collect();
    let room = ui.available_width();
    let places = tab_places(&widths, room.max(40.0));
    let need = places.last().map(|(x, w)| x + w + 2.0).unwrap_or(40.0);
    let (strip, _) = ui.allocate_exact_size(egui::vec2(need.min(room), height + 1.0), egui::Sense::hover());
    let painter = ui.painter().clone();
    let under = painter.add(egui::Shape::Noop);
    let mut order: Vec<usize> = (0..names.len()).filter(|&i| i != *selected).collect();
    order.extend((*selected < names.len()).then_some(*selected));
    for i in order {
        let (x, w) = places[i];
        let rect = egui::Rect::from_min_size(egui::pos2(strip.left() + x, strip.top()), egui::vec2(w, height));
        let response = ui.interact(rect, ui.scope_id().with(("tab", i)), egui::Sense::click());
        // (opens when pressed, as the original's; egui may not count the tab hovered on the press's own frame, so a click
        // opens it too)
        if response.clicked() || response.is_pointer_button_down_on() || (response.hovered() && ui.input(|inp| inp.pointer.primary_pressed())) {
            *selected = i;
        }
        let active = i == *selected;
        skin.three_slice(&painter, if active { "GFX_GUI_TAB_ACTIVE" } else { "GFX_GUI_TAB_INACTIVE" }, rect, if active { tint } else { tint.gamma_multiply(0.6) });
        // (a dark theme's tabs are dark: their black text gives way to the theme's text colour there)
        let text = if active { egui::Color32::WHITE } else if dark(palette) { rgb(palette.text) } else { egui::Color32::BLACK };
        // (the text is the tab's child view: what does not fit inside its right margin is cut off)
        painter.with_clip_rect(rect.intersect(egui::Rect::from_min_max(rect.min, egui::pos2(rect.right() - 12.0, rect.bottom())))).text(egui::pos2(rect.left() + 5.0, rect.center().y), egui::Align2::LEFT_CENTER, names[i], font.clone(), text);
    }
    let (left, right, bottom) = (size("GFX_GUI_TAB_BORDER_LEFT").x, size("GFX_GUI_TAB_BORDER_RIGHT").x, size("GFX_GUI_TAB_BORDER_BOTTOM").y);
    let body = ui.painter().add(egui::Shape::Noop);
    let open = *selected;
    let inner = ui.scope(|ui| egui::Frame::NONE.inner_margin(egui::Margin { left: (left + 2.0) as i8, right: (right + 2.0) as i8, top: 2, bottom: (bottom + 2.0) as i8 }).show(ui, |ui| add(ui, open)).inner);
    let content = inner.response.rect;
    let (l, r) = (strip.left(), content.right().max(strip.right()));
    let piece = |shapes: &mut Vec<egui::Shape>, name: &str, rect: egui::Rect| {
        if let Some(s) = skin.get(name) {
            shapes.push(egui::Shape::image(s.texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint));
        }
    };
    // (the strip's foot: its corners at the bottom left and right, the top line between)
    let (tl, tr, top) = (size("GFX_GUI_TAB_BORDER_TL"), size("GFX_GUI_TAB_BORDER_TR"), size("GFX_GUI_TAB_BORDER_TOP").y);
    let foot = strip.bottom();
    let mut line = Vec::new();
    piece(&mut line, "GFX_GUI_TAB_BORDER_TL", egui::Rect::from_min_size(egui::pos2(l, foot - tl.y), tl));
    piece(&mut line, "GFX_GUI_TAB_BORDER_TOP", egui::Rect::from_min_max(egui::pos2(l + tl.x, foot - top), egui::pos2(r - tr.x, foot)));
    piece(&mut line, "GFX_GUI_TAB_BORDER_TR", egui::Rect::from_min_size(egui::pos2(r - tr.x, foot - tr.y), tr));
    painter.set(under, egui::Shape::Vec(line));
    // (the body round the content: the sides, the bottom corners and the bottom, the background inside)
    let (t, b) = (foot, content.bottom());
    let (bl, br) = (size("GFX_GUI_TAB_BORDER_BL"), size("GFX_GUI_TAB_BORDER_BR"));
    let mut frame = Vec::new();
    piece(&mut frame, "GFX_GUI_TAB_BACKGROUND", egui::Rect::from_min_max(egui::pos2(l + left + 1.0, t + 1.0), egui::pos2(r - right - 1.0, b - bottom - 1.0)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_LEFT", egui::Rect::from_min_max(egui::pos2(l, t), egui::pos2(l + left, b - bl.y)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_RIGHT", egui::Rect::from_min_max(egui::pos2(r - right, t), egui::pos2(r, b - br.y)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BL", egui::Rect::from_min_size(egui::pos2(l, b - bl.y), bl));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BOTTOM", egui::Rect::from_min_max(egui::pos2(l + bl.x, b - bottom), egui::pos2(r - br.x, b)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BR", egui::Rect::from_min_size(egui::pos2(r - br.x, b - br.y), br));
    ui.painter().set(body, egui::Shape::Vec(frame));
    inner.inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_are_kept_by_name() {
        assert!(!pinned("Test window"));
        set_pinned("Test window", true);
        assert!(pinned("Test window"));
        set_pinned("Test window", false);
        assert!(!pinned("Test window"));
    }

    #[test]
    fn a_button_is_its_label_and_borders() {
        // (the default skin's GFX_GUI_BORDER01 sides: 5 wide, 11 high; a Verdana 13 label about 16 high)
        assert_eq!(button_size(egui::vec2(40.0, 16.0), 5.0, 11.0, 5.0, 11.0), egui::vec2(60.0, 42.0));
    }

    #[test]
    fn tabs_overlap_by_six_from_two_in() {
        assert_eq!(tab_places(&[40.0, 30.0, 50.0], 500.0), vec![(2.0, 40.0), (36.0, 30.0), (60.0, 50.0)]);
        // (too wide: narrowed in proportion)
        let p = tab_places(&[100.0, 100.0], 98.0);
        assert!(p[1].0 + p[1].1 <= 98.0 + 1e-3);
    }

    #[test]
    fn the_knob_fills_its_share_between_the_arrows() {
        // (a 100-point bar with 9-point arrows: 81 points from a point below the one to a point above the other)
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.5, 0.0);
        assert_eq!((a, b), (9.0, 50.0));
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.5, 1.0);
        assert_eq!((a, b), (50.0, 91.0));
        // (never under 8)
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.01, 0.0);
        assert_eq!(b - a, 9.0);
    }
}
