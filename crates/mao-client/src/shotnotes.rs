//! Notes beside a screenshot: what the server said of the looks of the characters around, so that a picture of something
//! drawn wrong comes with the data it was drawn from. The network side keeps each character's last full update here as a
//! line (its name, template, textures, clothes and worn meshes) with the update's bytes; a screenshot writes them to a text
//! file of its own name, the selected target first and with its bytes. The file stays in the screenshots' folder.

use std::collections::BTreeMap;
use std::sync::Mutex;

/// By identity: the name, the look's line, the update's bytes in hex.
static LOOKS: Mutex<BTreeMap<u32, (String, String, String)>> = Mutex::new(BTreeMap::new());

/// As many characters as are kept (a long session in a busy place: the oldest identities go).
const KEPT: usize = 1500;

/// A character's look from its full update.
pub fn set(id: u32, name: &str, line: String, bytes: &[u8]) {
    let mut looks = LOOKS.lock().unwrap_or_else(|e| e.into_inner());
    if looks.len() >= KEPT && !looks.contains_key(&id) {
        looks.pop_first();
    }
    looks.insert(id, (name.to_string(), line, bytes.iter().map(|b| format!("{b:02x}")).collect()));
}

/// The notes' text: the heading's lines, the target's look with its bytes (every character of that name), then the others'.
pub fn text(heading: &[String], target: Option<&str>) -> String {
    let looks = LOOKS.lock().unwrap_or_else(|e| e.into_inner());
    let mut out = heading.join("\n");
    out.push('\n');
    // (the target's line has more after the name: "Male Watcher  HP 7422/7422  (2 m)")
    let is_target = |name: &str| target.is_some_and(|t| !name.is_empty() && (t == name || t.starts_with(&format!("{name}  "))));
    for (id, (name, line, bytes)) in looks.iter().filter(|(_, l)| is_target(&l.0)) {
        out.push_str(&format!("\ntarget {id:#x} {name:?}\n  {line}\n  bytes {bytes}\n"));
    }
    out.push_str(&format!("\n{} characters known:\n", looks.len()));
    for (id, (name, line, _)) in looks.iter().filter(|(_, l)| !is_target(&l.0)) {
        out.push_str(&format!("{id:#x} {name:?}: {line}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_target_comes_first_with_its_bytes() {
        super::set(7, "Male Watcher", "template none".into(), &[0xab, 0x01]);
        super::set(9, "Somebody", "template 0x12".into(), &[0xcd]);
        let text = super::text(&["playfield Jobe".to_string()], Some("Male Watcher  HP 7422/7422  (2 m)"));
        let (target, others) = (text.find("target 0x7 \"Male Watcher\"").unwrap(), text.find("0x9 \"Somebody\": template 0x12").unwrap());
        assert!(text.starts_with("playfield Jobe\n") && target < others);
        assert!(text.contains("bytes ab01") && !text.contains("cd\n"));
    }
}
