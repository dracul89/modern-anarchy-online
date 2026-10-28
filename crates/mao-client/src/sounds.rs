//! Gamecode's sounds: what the characters' animation notes play through the sound module ([`crate::audio`]).
//!
//! A character's animations fire their notes as their time passes them (FUN_1003c248 over FUN_1003c0b7: see
//! `animstack::Entry::fired`), each note's event its name's number (FUN_10075c84: `catanim::Note::event`), and FUN_100452d3
//! plays the event's sound at the character: a step, a swim stroke or a landing on the material under it; a swish of a punch,
//! kick, whip or a huge blow; an NPC's idle and its attacks' starts. A character with an animation set (an NPC's `MonsterData`,
//! FUN_100524b2) plays its set's sound for the event (FUN_10045976 picking a variant); the others play the controller's own
//! sounds, looked up by name (FUN_1004496c).

use mao_formats::sound::sound_id;

/// The sound controller's own sounds (FUN_1004496c: `GetSoundID` of each name, in its +0 to +0x24 order).
pub struct Generic {
    pub walk: u32,
    pub run: u32,
    pub swim: u32,
    pub swim_fast: u32,
    pub punch: u32,
    pub kick: u32,
    pub tail: u32,
    pub huge: u32,
    pub land: u32,
    pub atrox_land: u32,
}

impl Default for Generic {
    fn default() -> Generic {
        Generic {
            walk: sound_id("SM_Sandy_Game_Feet_Man_Walk"),
            run: sound_id("SM_Sandy_Game_Feet_Man_Run"),
            swim: sound_id("SM_Sandy_Game_Feet_Man_Swim"),
            swim_fast: sound_id("SM_Sandy_Game_Feet_Man_SwimFast"),
            punch: sound_id("SM_Sandy_Swish_punch"),
            kick: sound_id("SM_Sandy_Swish_kick"),
            tail: sound_id("SM_Sandy_Swish_tail"),
            huge: sound_id("SM_Sandy_Swish_huge"),
            land: sound_id("SM_Sandy_Game_Feet_Man_Land"),
            atrox_land: sound_id("SM_Sandy_Game_Feet_Atrox_Land"),
        }
    }
}

/// A note's event at a character.
pub struct Note<'a> {
    pub event: i32,
    /// The character's global position.
    pub position: [f32; 3],
    /// Its animation set's sounds (`npc::Template::sounds`), if it has a set, and the seed picking among an event's variants.
    pub set: Option<(&'a mao_formats::npc::Template, u32)>,
    /// Its vehicle is falling (`Vehicle_t` +0x52): no steps.
    pub falling: bool,
    /// The material under it ([`ground_material`]).
    pub material: i32,
}

/// A play of FUN_100452d3: the sound, its volume and material (`PlayGameSound`'s arguments, the rest the defaults: no delay
/// or deferral, pitch 100, mode 1).
#[derive(Debug, PartialEq)]
pub struct Play {
    pub id: u32,
    pub volume: f32,
    pub material: i32,
}

/// FUN_100452d3: the sound a note's event plays, if any. The attack events (0xb and 0x8e to 0x91, FUN_10068e42's) are not
/// here: they belong to the attack the character is making.
pub fn note(generic: &Generic, n: &Note) -> Option<Play> {
    let event = n.event;
    if event > 0x91 {
        return None;
    }
    let set_sound = |key: i32| n.set.and_then(|(t, seed)| t.sound(key as u32, seed));
    let has_set = n.set.is_some();
    match event {
        // (a swish: the controller's own, else the set's; at material 0)
        0x73..=0x76 => {
            let id = if has_set {
                set_sound(event)?
            } else {
                match event {
                    0x73 => generic.punch,
                    0x74 => generic.kick,
                    0x75 => generic.tail,
                    _ => generic.huge,
                }
            };
            Some(Play { id, volume: 1.0, material: 0 })
        }
        // (an idle, an attack's start, entering, leaving and idling in a fight: the set's only)
        0xf | 0x77..=0x82 => has_set.then(|| set_sound(event)).flatten().map(|id| Play { id, volume: 1.0, material: 0 }),
        // (a step, a wing flap, a swim stroke, another movement, a landing: not while falling; on the material under it)
        0x25 | 0x26 | 0x36..=0x3b | 0x85 => {
            if n.falling {
                return None;
            }
            if has_set {
                return set_sound(event).map(|id| Play { id, volume: 1.0, material: n.material });
            }
            // (FUN_100452d3 plays a fly-mode, movement mode 6, step at 0.25: the client keeps no such mode)
            let volume = 1.0f32;
            let id = match event {
                0x26 => generic.walk,
                0x36 => generic.run,
                0x38 => generic.swim,
                0x39 => generic.swim_fast,
                // (a landing louder, twice the step's held to 1; none in a liquid, which the client does not track)
                0x85 => return Some(Play { id: generic.land, volume: (volume + volume).min(1.0), material: n.material }),
                _ => return None,
            };
            Some(Play { id, volume, material: n.material })
        }
        _ => None,
    }
}

/// The material FUN_100452d3 plays a step on: 5 (concrete) indoors and in the air (higher than 0.1 above the ground's height
/// at the tile's corner, FUN_10045a44: the height's high byte times the scale); on the ground the tile's material
/// (`n3Tilemap_t::GetTileTypeGround` at the position over the tile size, its main tile ID, `GetTileMaterialType`). In a liquid
/// 8 or 9 by depth (0.35), 10 and 11 for two liquid types: the client does not track the vehicle's liquid medium.
pub fn ground_material(terrain: Option<&mao_formats::terrain::Terrain>, materials: &std::collections::HashMap<u16, mao_formats::tilemap::TileMaterial>, position: [f32; 3]) -> i32 {
    let Some(t) = terrain else { return 5 };
    // (the x87 truncation of `_ftol`)
    let (tx, tz) = ((position[0] / t.tile_size) as i32, (position[2] / t.tile_size) as i32);
    let (cells_x, cells_z) = (t.width as i32 - 1, t.height as i32 - 1);
    let tile = if (0..cells_x).contains(&tx) && (0..cells_z).contains(&tz) { (t.tiles[(tz * cells_x + tx) as usize] & t.mask) as u8 } else { 0 };
    let height = |x: i32, z: i32| if (0..t.width as i32).contains(&x) && (0..t.height as i32).contains(&z) { (t.heights[z as usize * t.width + x as usize] >> 8) as f32 * t.height_scale } else { 0.0 };
    let ground = (height(tx, tz) as f64 + 0.1) as f32;
    if position[1] > ground {
        return 5;
    }
    let id = t.tile_types.get(tile as usize).copied().unwrap_or(0);
    materials.get(&id).map_or(0, |m| m.material as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_swishes_and_sets() {
        let g = Generic::default();
        let at = |event: i32| Note { event, position: [0.0; 3], set: None, falling: false, material: 3 };
        assert_eq!(note(&g, &at(0x26)), Some(Play { id: g.walk, volume: 1.0, material: 3 }));
        assert_eq!(note(&g, &at(0x85)), Some(Play { id: g.land, volume: 1.0, material: 3 }));
        assert_eq!(note(&g, &at(0x74)), Some(Play { id: g.kick, volume: 1.0, material: 0 }));
        // (no idle sound without a set; a falling step is silent; a hashed note is not an event here)
        assert_eq!(note(&g, &at(0xf)), None);
        assert_eq!(note(&g, &Note { falling: true, ..at(0x26) }), None);
        assert_eq!(note(&g, &at(sound_id("SM_Sandy_Swish_punch") as i32)), None);
        let mut set = mao_formats::npc::Template::default();
        set.sounds = vec![(0x26, 10), (0x26, 11), (0xf, 12)];
        let npc = |event: i32| Note { set: Some((&set, 1)), ..at(event) };
        assert_eq!(note(&g, &npc(0x26)), Some(Play { id: 11, volume: 1.0, material: 3 }));
        assert_eq!(note(&g, &npc(0xf)), Some(Play { id: 12, volume: 1.0, material: 0 }));
        assert_eq!(note(&g, &npc(0x73)), None);
    }

    #[test]
    fn the_material_under_a_character() {
        let t = mao_formats::terrain::Terrain { width: 3, height: 3, heights: vec![0x0100; 9], tiles: vec![1, 0, 0, 0], tile_types: vec![70, 71], height_scale: 0.5, mask: 0xff, tile_size: 4.0 };
        let mut m = std::collections::HashMap::new();
        m.insert(71, mao_formats::tilemap::TileMaterial { material: 0xf, ..Default::default() });
        // (on the ground of tile 1, ID 71: grass; the ground is 0.5 high, so 0.6 is still on it and 0.61 is in the air)
        assert_eq!(ground_material(Some(&t), &m, [1.0, 0.6, 1.0]), 0xf);
        assert_eq!(ground_material(Some(&t), &m, [1.0, 0.61, 1.0]), 5);
        assert_eq!(ground_material(Some(&t), &m, [5.0, 0.5, 1.0]), 0);
        assert_eq!(ground_material(None, &m, [1.0, 0.0, 1.0]), 5);
    }
}
