//! This client's own tool windows, which the original client does not have: kept together so that the interface has one
//! place to open, close and draw them by name (`Ui::set_window`, `/open <name>`, each tool's own chat command).
//!
//! | name | command | window |
//! |---|---|---|
//! | `damage` | `/damage`, `/dps` | the damage meter (`damage.rs`) |
//! | `find` | `/find <part of a name>` | where a thing is, on any of the player's characters (`itemindex.rs`) |
//! | `crafting` | `/crafting [text]`, `/craft` | the tradeskill guide read from the game's items (`craftguide.rs`) |

use crate::theme::Palette;

#[derive(Default)]
pub struct Extras {
    pub damage: crate::damage::DamageWindow,
    pub finder: crate::itemindex::FindWindow,
    pub crafting: crate::craftguide::CraftWindow,
}

impl Extras {
    /// The open flag of the window a name means; none for a name that is no tool's.
    pub fn window(&mut self, name: &str) -> Option<&mut bool> {
        // (`/find <text>`: the window with the text to look for)
        if let Some(query) = name.strip_prefix("find ") {
            self.finder.query = query.trim().to_string();
            return Some(&mut self.finder.open);
        }
        // (`/crafting <text>`: the guide with the text to look for)
        if let Some(query) = name.strip_prefix("crafting ") {
            self.crafting.look_for(query);
            return Some(&mut self.crafting.open);
        }
        match name {
            "crafting" | "craft" | "craftguide" | "tradeskill guide" | "tradeskills" | "recipes" => Some(&mut self.crafting.open),
            "find" | "finder" | "find items" | "finditems" => Some(&mut self.finder.open),
            "damage" | "dps" | "damagemeter" | "damage meter" => Some(&mut self.damage.open),
            _ => None,
        }
    }

    /// Whether a name's window is open.
    pub fn is_open(&self, name: &str) -> Option<bool> {
        match name {
            "crafting" | "craft" | "craftguide" | "tradeskill guide" | "tradeskills" | "recipes" => Some(self.crafting.open),
            "find" | "finder" | "find items" | "finditems" => Some(self.finder.open),
            "damage" | "dps" | "damagemeter" | "damage meter" => Some(self.damage.open),
            _ => None,
        }
    }

    /// Closes them all (`/close` with no name).
    pub fn close_all(&mut self) {
        self.damage.open = false;
        self.finder.open = false;
        self.crafting.open = false;
    }

    /// Draws the open ones; gives back the info pages they ask for (`itemref://` addresses, for the info view).
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Vec<String> {
        self.damage.show(ctx, palette);
        let mut infos = self.finder.show(ctx, palette);
        infos.extend(self.crafting.show(ctx, palette));
        infos
    }
}
