//! The shop booth's window. What the captures show: using a booth (`GenericCmd` Use on `(0xc75b, serial)`)
//! makes the server send the stock (`ShopUpdate`: a list of `(low id, high id, quality)`) and open a trade
//! (`Trade`, action 0, naming the booth and a temporary bag `0xc767`); the client closes the window with a
//! `Trade` decline. **Buying** and **selling** are from the fifth capture: a `Trade` action 5 (add) naming the
//! stock item (booth, 0x6f, stock index) or your own item (yourself, page 0x68, slot), then action 1 (accept);
//! the server answers action 4 (complete) and the booth has to be used again for the next trade.

use std::sync::Arc;

use crate::gear::{GearItem, GearWindow, Icon};
use crate::theme::Palette;

/// One item for sale.
pub struct Slot {
    pub low: u32,
    pub high: u32,
    pub quality: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
}

/// What the window asks the network thread to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopCommand {
    /// Buy the stock item at this index.
    Buy(usize),
    /// Sell the inventory item in this slot.
    Sell(u32),
    /// Close the shop (a trade decline).
    Close,
}

/// A shop that was opened: the booth and its stock.
pub struct ShopState {
    pub shop: (u32, u32),
    pub stock: Vec<Slot>,
}

#[derive(Default)]
pub struct ShopWindow {
    pub open: bool,
    pub shop: (u32, u32),
    pub stock: Vec<Slot>,
    pub selected: Option<usize>,
    pub status: String,
}

impl ShopWindow {
    pub fn set(&mut self, state: ShopState) {
        self.shop = state.shop;
        if !self.open || self.status.is_empty() {
            self.status = format!("{} items for sale.", state.stock.len());
        }
        self.stock = state.stock;
        self.open = true;
        self.selected = None;
    }

    /// Draws the window; returns what to send, if anything.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, items: &[GearItem], palette: &Palette) -> Option<ShopCommand> {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut command = None;
        let mut is_open = self.open;
        let mut close = false;
        crate::chrome::window(ctx, palette, "Shop", "Shop", &mut is_open, |w| w.resizable(false).default_pos(crate::layout::pos("Shop", egui::pos2(300.0, 100.0))), |ui| {
            ui.set_width(330.0);
            ui.label(egui::RichText::new(format!("Booth {:x}", self.shop.1 & 0xffff)).small().weak());
            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(190.0), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                    for (i, slot) in self.stock.iter().enumerate() {
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::click());
                        let selected = self.selected == Some(i);
                        let painter = ui.painter();
                        painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(120));
                        painter.rect_stroke(rect, 3.0, egui::Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { accent } else { egui::Color32::from_white_alpha(50) }), egui::StrokeKind::Inside);
                        match &slot.icon {
                            Some(icon) => {
                                let id = gear.texture(ctx, icon);
                                ui.painter().image(id, rect.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                            None => {
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, slot.name.chars().next().unwrap_or('?'), egui::FontId::proportional(20.0), accent);
                            }
                        }
                        if response.clicked() {
                            self.selected = Some(i);
                        }
                        response.on_hover_text(&slot.name);
                    }
                });
            });
            if let Some(slot) = self.selected.and_then(|i| self.stock.get(i)) {
                ui.separator();
                ui.label(egui::RichText::new(&slot.name).strong().color(accent));
                if slot.quality > 1 {
                    ui.label(egui::RichText::new(format!("Quality {}", slot.quality)).small());
                }
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(110.0).id_salt("shop-description"), |ui| {
                    ui.label(slot.description.replace("\r\n", "\n"));
                });
            }
            if !self.status.is_empty() {
                ui.label(egui::RichText::new(&self.status).small());
            }
            egui::CollapsingHeader::new("Sell").show(ui, |ui| {
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(120.0).id_salt("shop-sell"), |ui| {
                    // The backpack (inventory slots from 64) holds what can be sold.
                    for item in items.iter().filter(|i| i.slot >= 64) {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} x{}", item.name, item.count));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("Sell").clicked() {
                                    command = Some(ShopCommand::Sell(item.slot));
                                    self.status = format!("Selling {} ...", item.name);
                                }
                            });
                        });
                    }
                });
            });
            ui.horizontal(|ui| {
                if crate::chrome::button(ui, palette, "Buy", self.selected.is_some()).clicked() {
                    if let Some(i) = self.selected {
                        command = Some(ShopCommand::Buy(i));
                        self.status = format!("Asking to buy {} ...", self.stock.get(i).map_or("", |s| s.name.as_str()));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::chrome::button(ui, palette, "Close", true).clicked() {
                        close = true;
                    }
                });
            });
        });
        if (!is_open || close) && self.open {
            self.open = false;
            command = Some(ShopCommand::Close);
        }
        command
    }
}
