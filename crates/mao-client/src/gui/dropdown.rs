//! `DropdownMenu_c`: a button showing the chosen entry that opens the list of them.
//!
//! `Initialize` (0x1012c389) lays out, side by side (`HLayoutNode`): a three-piece border (`SetGfx(6, 7, 8)`:
//! `GFX_GUI_DROPDOWNMENU_LEFT`, `_MIDDLE`, `_RIGHT`, in colour code 0x1000000) holding the chosen entry's text (font 5) 6 in
//! from the sides and 3 from top and bottom, as wide as the widest entry (`RecalcMaxStringWidth` 0x1012be8b; 70 with
//! none, `_DAT_101c5d64`); then the arrow (`GFX_GUI_DROPDOWNMENU_CLOSED`, `_OPEN` while the list is open). A press opens
//! the list (`MouseDown` 0x1012c30d, `Open`): a [`PopupMenu`] of the entries (`SlotMenuOpening`) 5 right of the left
//! edge and a pixel under the bottom (`SlotMenuOpened`); choosing one selects it (`SlotMenuItemSelected`,
//! `SelectByIndex` 0x1012c153: the text set, `SelectionChanged`). The first entry added is selected (`InsertItem`).

use super::font::{FontId, Measure};
use super::geom::Point;
use super::host::{Env, Mouse, Paint, Signal};
use super::popup::{Entry, Outcome, PopupMenu};
use super::view::{Control, Id, Node, Tree};
use super::xml::Element;

pub const LEFT: &str = "GFX_GUI_DROPDOWNMENU_LEFT";
pub const MIDDLE: &str = "GFX_GUI_DROPDOWNMENU_MIDDLE";
pub const RIGHT: &str = "GFX_GUI_DROPDOWNMENU_RIGHT";
pub const CLOSED: &str = "GFX_GUI_DROPDOWNMENU_CLOSED";
pub const OPEN: &str = "GFX_GUI_DROPDOWNMENU_OPEN";

/// The text's room with no entries.
const EMPTY_WIDTH: f32 = 70.0;

#[derive(Default)]
pub struct DropdownMenu {
    pub items: Vec<String>,
    /// The chosen entry (`GetSelection`: -1 for none).
    pub selection: Option<usize>,
    /// `SetMaxStringWidth`: a fixed width for the text, else the widest entry's.
    pub max_width: Option<f32>,
    enabled: bool,
    /// The list while it is open, and a press asking for it.
    menu: Option<PopupMenu>,
    want_open: bool,
}

impl DropdownMenu {
    pub fn new() -> DropdownMenu {
        DropdownMenu { enabled: true, ..Default::default() }
    }

    /// `AppendItem`: its index; the first one added is selected.
    pub fn append(&mut self, label: &str) -> usize {
        self.items.push(label.to_string());
        if self.selection.is_none() {
            self.selection = Some(0);
        }
        self.items.len() - 1
    }

    /// `SelectByIndex` (no signal: the caller's own choice).
    pub fn select(&mut self, index: Option<usize>) {
        self.selection = index.filter(|&k| k < self.items.len());
    }

    /// `Clear`.
    pub fn clear(&mut self) {
        self.items.clear();
        self.selection = None;
    }

    /// `GetCurrentText`.
    pub fn current_text(&self) -> &str {
        self.selection.and_then(|k| self.items.get(k)).map_or("", String::as_str)
    }

    fn text_width(&self, m: &Measure) -> f32 {
        self.max_width.unwrap_or_else(|| if self.items.is_empty() { EMPTY_WIDTH } else { self.items.iter().map(|s| m.width(FontId::NORMAL, s)).fold(0.0, f32::max) })
    }

    fn sprite(m: &Measure, name: &str) -> egui::Vec2 {
        m.ctx.as_ref().and_then(crate::skin::skin).and_then(|s| s.get(name)).map_or(egui::Vec2::ZERO, |s| s.size)
    }
}

impl Control for DropdownMenu {
    fn class(&self) -> &'static str {
        "DropdownMenu"
    }
    fn preferred(&self, m: &Measure, _max: bool) -> Option<Point> {
        // (the border: the text 6 in from the sides and 3 from top and bottom; the arrow beside it, a pixel apart)
        let text = Point::new(self.text_width(m) - 1.0, m.height(FontId::NORMAL) - 1.0);
        let border = Point::new(text.x + 12.0, text.y + 6.0);
        let arrow = Self::sprite(m, CLOSED);
        Some(Point::new(border.x + arrow.x, border.y.max(arrow.y - 1.0)))
    }
    fn paint(&self, p: &Paint) {
        let m = p.env.measure;
        let arrow = Self::sprite(m, if self.menu.is_some() { OPEN } else { CLOSED });
        let (l, r) = (Self::sprite(m, LEFT).x, Self::sprite(m, RIGHT).x);
        let border = egui::Rect::from_min_max(p.rect.min, egui::pos2(p.rect.right() - arrow.x - 1.0, p.rect.bottom()));
        let tint = {
            let c = crate::chrome::code1(p.env.palette);
            let c = egui::Color32::from_rgb(c[0], c[1], c[2]);
            if p.enabled { c } else { egui::Color32::from_rgb((c.r() as u32 * 0x90 / 0xff) as u8, (c.g() as u32 * 0x90 / 0xff) as u8, (c.b() as u32 * 0x90 / 0xff) as u8) }
        };
        if let Some(skin) = p.env.skin {
            skin.draw_in(p.painter, LEFT, egui::Rect::from_min_size(border.min, egui::vec2(l, border.height())), tint);
            skin.draw_in(p.painter, MIDDLE, egui::Rect::from_min_max(egui::pos2(border.left() + l, border.top()), egui::pos2(border.right() - r, border.bottom())), tint);
            skin.draw_in(p.painter, RIGHT, egui::Rect::from_min_max(egui::pos2(border.right() - r, border.top()), border.max), tint);
            let y = p.rect.center().y - arrow.y * 0.5;
            skin.draw_in(p.painter, if self.menu.is_some() { OPEN } else { CLOSED }, egui::Rect::from_min_size(egui::pos2(border.right() + 1.0, y.floor()), arrow), tint);
        }
        let colour = if p.enabled { egui::Color32::WHITE } else { egui::Color32::from_gray(0xa0) };
        let clip = p.painter.with_clip_rect(border.shrink2(egui::vec2(6.0, 0.0)).intersect(p.painter.clip_rect()));
        let sub = Paint { painter: &clip, env: p.env, rect: p.rect, hovered: p.hovered, pressed: p.pressed, enabled: p.enabled };
        super::controls::text(&sub, FontId::NORMAL, egui::pos2(border.left() + 6.0, border.top() + 3.0), self.current_text(), colour);
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, _at: Point) -> Option<Signal> {
        if m == Mouse::Down && self.menu.is_none() {
            self.want_open = true;
        }
        None
    }
    fn overlay(&mut self, ctx: &egui::Context, env: &Env, rect: egui::Rect, id: egui::Id) -> Option<Signal> {
        if std::mem::take(&mut self.want_open) {
            let entries = self.items.iter().map(|s| Entry { label: s.clone(), enabled: true }).collect();
            self.menu = Some(PopupMenu::new(entries, egui::pos2(rect.left() + 5.0, rect.bottom() + 1.0), self.selection, ctx.cumulative_frame_nr()));
        }
        let menu = self.menu.as_mut()?;
        match menu.show(ctx, env, id.with("dropdown")) {
            Outcome::None => None,
            Outcome::Closed => {
                self.menu = None;
                None
            }
            Outcome::Chosen(k) => {
                self.menu = None;
                let changed = self.selection != Some(k);
                self.selection = Some(k);
                changed.then_some(Signal::Selected(k as i64))
            }
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A `DropdownMenu` element (its entries are added by the window's code).
pub fn create(tree: &mut Tree, e: &Element) -> Id {
    tree.add(Node::from_element(e, 4, Box::new(DropdownMenu::new())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_entry_is_chosen() {
        let mut d = DropdownMenu::new();
        assert_eq!(d.current_text(), "");
        d.append("Omni");
        d.append("Clan");
        assert_eq!((d.selection, d.current_text()), (Some(0), "Omni"));
        d.select(Some(1));
        assert_eq!(d.current_text(), "Clan");
        d.select(Some(5));
        assert_eq!(d.selection, None);
        // (the room: the widest entry, or 70 with none)
        let m = Measure::default();
        assert!(d.preferred(&m, false).unwrap().x > 12.0);
        assert_eq!(DropdownMenu::new().text_width(&m), 70.0);
    }
}
