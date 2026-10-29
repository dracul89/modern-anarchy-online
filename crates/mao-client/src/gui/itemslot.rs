//! `ItemSlotView_c`: a place for one item (trade, mail, the market deposit, the tradeskill kit).
//!
//! The XML constructor (FUN_10042d63) reads `iconview_flags` and `Initialize` (FUN_10042af9) draws
//! `GFX_GUI_MULTILISTVIEW_SLOT_48_CLOSED` (0x11d) over the view and centres in it an `ItemIconView_c` (FUN_1003f67b: 48 by
//! 48, `_DAT_101b0290` 47 in the views' units), the room left over split as its borders (the left and top halves rounded
//! down); the slot is as big as its picture. The item in it is the window's to set ([`ItemSlot::item`]); a click is the
//! slot's [`Signal::Clicked`].

use super::font::Measure;
use super::geom::Point;
use super::host::{Mouse, Paint, Signal};
use super::view::{Control, Id, Node, Tree};
use super::xml::Element;

pub const SLOT: &str = "GFX_GUI_MULTILISTVIEW_SLOT_48_CLOSED";
/// The icon's size.
pub const ICON: f32 = 48.0;

/// What a slot shows: the icon's texture and the item's name (its tip).
#[derive(Clone, Debug, PartialEq)]
pub struct SlotItem {
    pub texture: egui::TextureId,
    pub name: String,
}

#[derive(Default)]
pub struct ItemSlot {
    pub flags: i32,
    /// The slot's picture under the icon (an `ItemIconView` element alone has none).
    pub background: bool,
    pub item: Option<SlotItem>,
}

impl ItemSlot {
    fn slot_size(&self, m: &Measure) -> egui::Vec2 {
        if !self.background {
            return egui::Vec2::splat(ICON);
        }
        m.ctx.as_ref().and_then(crate::skin::skin).and_then(|s| s.get(SLOT)).map_or(egui::Vec2::splat(ICON), |s| s.size.max(egui::Vec2::splat(ICON)))
    }
}

impl Control for ItemSlot {
    fn class(&self) -> &'static str {
        "ItemSlotView"
    }
    fn preferred(&self, m: &Measure, _max: bool) -> Option<Point> {
        let s = self.slot_size(m);
        Some(Point::new(s.x - 1.0, s.y - 1.0))
    }
    fn paint(&self, p: &Paint) {
        if !self.background {
        } else if let Some(skin) = p.env.skin {
            skin.draw_in(p.painter, SLOT, p.rect, egui::Color32::WHITE);
        } else {
            p.painter.rect_filled(p.rect, 2.0, egui::Color32::from_black_alpha(110));
        }
        if let Some(item) = &self.item {
            // (the icon centred, the left and top halves of the room rounded down)
            let room = p.rect.size() - egui::Vec2::splat(ICON);
            let at = p.rect.min + egui::vec2((room.x * 0.5).floor(), (room.y * 0.5).floor());
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            p.painter.image(item.texture, egui::Rect::from_min_size(at, egui::Vec2::splat(ICON)), uv, egui::Color32::WHITE);
        }
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, _at: Point) -> Option<Signal> {
        (m == Mouse::Up { inside: true }).then_some(Signal::Clicked)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// An `ItemSlotView` element (`ItemIconView` for the icon alone).
pub fn create(tree: &mut Tree, e: &Element, background: bool) -> Id {
    // (View(element, 0, true): flags 0)
    tree.add(Node::from_element(e, 0, Box::new(ItemSlot { flags: e.int("iconview_flags", 0), background, item: None })))
}

/// Puts an item in a slot (none empties it); its name becomes the slot's tip.
pub fn set_item(tree: &mut Tree, id: Id, item: Option<SlotItem>) {
    let name = item.as_ref().map(|i| i.name.clone()).unwrap_or_default();
    if let Some(s) = tree.control_mut::<ItemSlot>(id) {
        s.item = item;
    }
    tree.get_mut(id).tooltip = name;
}
