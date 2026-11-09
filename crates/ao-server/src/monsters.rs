//! The monsters the server places: their place, their health and what a kill gives. The names, places and numbers are the
//! server's own; the appearance is a captured monster's, kept in the template (`data/templates.txt`, kind `271b3a6b`).

use mao_net::n3::{set_monster, Message, CHARACTER};

use crate::templates;

/// The playfield Old Athens is (its number, which the captures' playfield messages carry).
pub const OLD_ATHENS: u32 = 540;
/// Where a new character starts in Old Athens (the captured arrival).
pub const SPAWN: [f32; 3] = [490.45, 8.01, 447.38];
/// The item a monster gives when it dies: a stock item of the captured vending machine.
pub const LOOT: u32 = 156020;

/// One monster in the world.
#[derive(Clone, Debug, PartialEq)]
pub struct Monster {
    pub id: u32,
    pub name: String,
    pub position: [f32; 3],
    pub playfield: u32,
    pub health: u32,
    pub max_health: u32,
    pub xp: u32,
    pub loot: u32,
}

/// The monsters a playfield has when a character enters it (only Old Athens has any: the server's own table).
pub fn spawns(playfield: u32) -> Vec<Monster> {
    if playfield != OLD_ATHENS {
        return Vec::new();
    }
    (0..4u32)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::FRAC_PI_2;
            Monster {
                id: 0x7f00_0001 + i,
                name: format!("Training Dummy {}", i + 1),
                position: [SPAWN[0] + 10.0 * angle.cos() + 8.0, SPAWN[1], SPAWN[2] + 10.0 * angle.sin()],
                playfield,
                health: 120,
                max_health: 120,
                xp: 400,
                loot: LOOT,
            }
        })
        .collect()
}

/// The monster's entity message (its place, name and health), made from the captured monster.
pub fn entity(m: &Monster) -> Option<Message> {
    let (_, data) = templates::raw(0x271b3a6b)?;
    let data = set_monster(&data, &m.name, m.playfield, m.position, m.health, m.max_health)?;
    Some(Message { kind: 0x271b3a6b, identity: (CHARACTER, m.id), data })
}

/// An NPC's entity message (`knubot`'s): the captured monster's, with the NPC's name and place and a full health bar.
pub fn npc_entity(n: &crate::knubot::Npc) -> Option<Message> {
    let (_, data) = templates::raw(0x271b3a6b)?;
    let data = set_monster(&data, n.name, n.playfield, n.position, 1000, 1000)?;
    Some(Message { kind: 0x271b3a6b, identity: (CHARACTER, n.id), data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_monsters_are_in_old_athens_only() {
        assert_eq!(spawns(OLD_ATHENS).len(), 4);
        assert!(spawns(1234).is_empty());
    }

    #[test]
    fn a_monster_is_an_entity_message_the_decoder_reads() {
        let m = &spawns(OLD_ATHENS)[1];
        let msg = entity(m).expect("the template");
        let info = msg.entity_info().expect("decodes");
        assert_eq!(info.name, m.name);
        assert_eq!(info.playfield, Some(OLD_ATHENS));
        assert_eq!(info.health, Some((m.health, m.max_health)));
        assert_eq!(msg.identity, (CHARACTER, m.id));
    }
}
