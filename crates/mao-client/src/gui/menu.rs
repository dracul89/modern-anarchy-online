//! GUI.dll's scripted menus: `ControlMenu_c` (FUN_10071d7d, FUN_10071a6e) holding `CCMenuEntry_c`s (FUN_100669b6), read
//! from the skin's `ActionMenu/*.xml` scripts, with their actions (FUN_100661a4, the factory FUN_1006539e).
//!
//! A script's root `<Menu>` holds `<Menu>` and `<MenuEntry>` elements (an `<ItemTemplate>` styles the special actions'
//! menus; not kept). Each entry is a distributed value named by its `name` (an `inventory_window` entry is that
//! window's open flag), shown with its `label` (markup), `tooltip` and `tooltip_body`, only when its `criteria` holds.
//! Its `button_mode` is `none`, `toggle` (drawn pressed while its value is true) or `button`; without one an entry with
//! actions and no other `invoke` is a toggle. A `<Menu>` entry opens a menu of its own: the entries of its `sub_script`
//! (`ActionMenu/<file>`), else of its own children.
//!
//! An entry's `<Actions event="on_activate, on_invoke ..." criteria="...">` run on those events (`on_activate`,
//! `on_deactivate`, `on_invoke`, `on_begin_drag`), when their criteria hold (`(%s) && (%s)` with the entry's own):
//! `<SetValue name=... value=.../>` sets a distributed value to the expression's value (true without one,
//! FUN_100655bf), `<CloseMenu/>` closes the open menus, `<RunChatScript value="/..."/>` runs a chat command
//! (FUN_10065798's signal).

use super::xml::Element;

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    SetValue { name: String, value: String },
    CloseMenu,
    RunChatScript(String),
}

/// One `<Actions>` block.
#[derive(Clone, Debug)]
pub struct Actions {
    pub events: Vec<String>,
    pub criteria: String,
    pub actions: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    None,
    Toggle,
    Button,
}

#[derive(Clone, Debug)]
pub enum Sub {
    Script(String),
    Inline(Vec<Element>),
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub label: String,
    /// `bgicon`: the picture it is drawn as (an `id:<GFX name>` expression, whose DynamicID names the skin's picture).
    pub icon: String,
    pub tooltip: String,
    pub tooltip_body: String,
    pub criteria: String,
    pub invoke: String,
    pub mode: Mode,
    pub actions: Vec<Actions>,
    /// A `<Menu>` entry's own menu.
    pub sub: Option<Sub>,
}

impl Entry {
    /// FUN_100669b6.
    pub fn from_element(e: &Element) -> Entry {
        let text = |n: &str| e.attribute(n).unwrap_or("").to_string();
        let actions = actions(e);
        let invoke = e.attribute("invoke").filter(|v| !v.eq_ignore_ascii_case("toggle")).unwrap_or("").to_string();
        let mode = match e.attribute("button_mode") {
            Some(m) if m.eq_ignore_ascii_case("none") => Mode::None,
            Some(m) if m.eq_ignore_ascii_case("toggle") => Mode::Toggle,
            Some(m) if m.eq_ignore_ascii_case("button") => Mode::Button,
            Some(_) => Mode::None,
            // (FUN_1006519f: it has actions; then a toggle unless it invokes something else)
            None if !actions.is_empty() && invoke.is_empty() => Mode::Toggle,
            None => Mode::None,
        };
        let sub = e.name.eq_ignore_ascii_case("menu").then(|| match e.attribute("sub_script") {
            Some(s) => Sub::Script(s.to_string()),
            None => Sub::Inline(e.children.clone()),
        });
        Entry {
            name: text("name"),
            label: super::xml::ldb_text(&text("label")),
            icon: e.attribute("bgicon").map(|v| v.trim().strip_prefix("id:").unwrap_or(v.trim()).to_string()).unwrap_or_default(),
            tooltip: super::xml::ldb_text(&text("tooltip")),
            tooltip_body: super::xml::ldb_text(&text("tooltip_body")),
            criteria: text("criteria"),
            invoke,
            mode,
            actions,
            sub,
        }
    }

    /// The actions for an event (lowercase), with each block's criteria.
    pub fn actions_for<'a>(&'a self, event: &'a str) -> impl Iterator<Item = &'a Actions> + 'a {
        self.actions.iter().filter(move |a| a.events.iter().any(|e| e == event))
    }
}

/// FUN_100661a4: an element's `<Actions>` blocks.
fn actions(e: &Element) -> Vec<Actions> {
    let mut out = Vec::new();
    for a in e.children.iter().filter(|c| c.name.eq_ignore_ascii_case("actions")) {
        let events: Vec<String> = a.attribute("event").unwrap_or("").split(',').map(|s| s.trim().to_ascii_lowercase()).filter(|s| matches!(s.as_str(), "on_activate" | "on_deactivate" | "on_invoke" | "on_begin_drag")).collect();
        if events.is_empty() {
            continue;
        }
        // (the factory FUN_1006539e, by the element's name in any case)
        let list = a
            .children
            .iter()
            .filter_map(|c| match c.name.to_ascii_lowercase().as_str() {
                "setvalue" => c.attribute("name").map(|n| Action::SetValue { name: n.to_string(), value: c.attribute("value").unwrap_or("").to_string() }),
                "closemenu" => Some(Action::CloseMenu),
                "runchatscript" => c.attribute("value").map(|v| Action::RunChatScript(v.to_string())),
                _ => None,
            })
            .collect();
        out.push(Actions { events, criteria: a.attribute("criteria").unwrap_or("").to_string(), actions: list });
    }
    out
}

/// FUN_10071a6e: the entries of a `<Menu>` element's children.
pub fn entries(children: &[Element]) -> Vec<Entry> {
    children.iter().filter(|c| c.name.eq_ignore_ascii_case("menu") || c.name.eq_ignore_ascii_case("menuentry")).map(Entry::from_element).collect()
}

/// A script (`ActionMenu/<file>` of the first skin folder that has it): its root `<Menu>`'s entries.
pub fn load_script(dirs: &[std::path::PathBuf], file: &str) -> Vec<Entry> {
    let Some(text) = dirs.iter().find_map(|d| std::fs::read(d.join("ActionMenu").join(file)).ok()) else { return Vec::new() };
    let doc = super::xml::parse(&String::from_utf8_lossy(&text));
    doc.iter().find(|e| e.name.eq_ignore_ascii_case("menu")).map(|m| entries(&m.children)).unwrap_or_default()
}

/// A `<Menu>` entry's own entries.
pub fn sub_entries(dirs: &[std::path::PathBuf], sub: &Sub) -> Vec<Entry> {
    match sub {
        Sub::Script(file) => load_script(dirs, file),
        Sub::Inline(children) => entries(children),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_menu_script() {
        let doc = crate::gui::xml::parse(
            r#"<?xml version="1.0" ?><Menu>
	<Menu label="&lt;font color=#FFBE96&gt;View&lt;/font&gt;" name="char_menu" sub_script="Menu_Char.xml" tooltip="Char Menu">
		<Actions criteria="dvalue:AutoCloseWindows" event="on_activate">
			<SetValue name="aou_menu" value="false"/>
		</Actions>
	</Menu>
	<MenuEntry label="Inventory" name="inventory_window" criteria="stat:side!=3">
		<Actions event="on_invoke"><CloseMenu /><SetValue name="inventory_window" value="dvalue:inventory_window==false" /></Actions>
	</MenuEntry>
	<MenuEntry label="Daily Rewards" name="dailylogin_window"/>
	<MenuEntry label="Script"><Actions event="on_invoke"><RunChatScript value="/script aou_gui_slim"/></Actions></MenuEntry>
</Menu>"#,
        );
        let e = entries(&doc[0].children);
        assert_eq!(e.len(), 4);
        assert_eq!(e[0].label, "<font color=#FFBE96>View</font>");
        assert!(matches!(&e[0].sub, Some(Sub::Script(s)) if s == "Menu_Char.xml"));
        assert_eq!(e[0].actions_for("on_activate").count(), 1);
        assert_eq!(e[1].mode, Mode::Toggle);
        assert_eq!(e[1].actions_for("on_invoke").next().unwrap().actions[0], Action::CloseMenu);
        assert_eq!(e[2].mode, Mode::None);
        assert_eq!(e[3].actions[0].actions, vec![Action::RunChatScript("/script aou_gui_slim".into())]);
    }
}
