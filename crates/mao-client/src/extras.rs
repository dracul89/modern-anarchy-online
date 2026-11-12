//! This client's own tool windows, which the original client does not have: kept together so that the interface has one
//! place to open, close and draw them by name (`Ui::set_window`, `/open <name>`, each tool's own chat command).
//!
//! | name | command | window |
//! |---|---|---|
//! | `damage` | `/damage`, `/dps` | the damage meter (`damage.rs`) |

use crate::theme::Palette;

#[derive(Default)]
pub struct Extras {
    pub damage: crate::damage::DamageWindow,
}

impl Extras {
    /// The open flag of the window a name means; none for a name that is no tool's.
    pub fn window(&mut self, name: &str) -> Option<&mut bool> {
        match name {
            "damage" | "dps" | "damagemeter" | "damage meter" => Some(&mut self.damage.open),
            _ => None,
        }
    }

    /// Whether a name's window is open.
    pub fn is_open(&self, name: &str) -> Option<bool> {
        match name {
            "damage" | "dps" | "damagemeter" | "damage meter" => Some(self.damage.open),
            _ => None,
        }
    }

    /// Closes them all (`/close` with no name).
    pub fn close_all(&mut self) {
        self.damage.open = false;
    }

    /// Draws the open ones.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        self.damage.show(ctx, palette);
    }
}
