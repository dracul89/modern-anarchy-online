//! A window holding a view tree (GUI.dll's `Window` with a root view, as `InfoView_c` and the other XML windows are made):
//! the client's window frame (`chrome::window`) around the tree.

use super::view::{Id, Tree};

pub struct XmlWindow {
    /// The window's name (its place is remembered under it) and title.
    pub name: String,
    pub title: String,
    pub tree: Tree,
    pub root: Id,
    pub open: bool,
}

impl XmlWindow {
    /// A window of a skin views file (`Views/<file>`), through the skin's folders.
    pub fn load(ctx: &egui::Context, name: &str, title: &str, file: &str) -> Option<XmlWindow> {
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let (tree, root) = super::load_view(&dirs, file, "")?;
        Some(XmlWindow { name: name.to_string(), title: title.to_string(), tree, root, open: true })
    }

    /// Shows the window; the views' signals since the last call.
    pub fn show(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette) -> Vec<(Id, super::host::Signal)> {
        if !self.open {
            return Vec::new();
        }
        let skin = crate::skin::skin(ctx);
        let min = super::host::preferred_pixels(&self.tree, self.root, false);
        let (tree, root) = (&mut self.tree, self.root);
        crate::chrome::window(ctx, palette, &self.name, &self.title, &mut self.open, |w| w.default_size(min.max(egui::vec2(320.0, 240.0))).resizable(true), |ui| {
            super::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        std::mem::take(&mut self.tree.signals)
    }
}
