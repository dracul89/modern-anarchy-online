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

/// `GameTime_t::UpdateDayPeriod` (0x1000b22c): 0 dawn (from hour 7 for 2), 1 day, 2 dusk (from 21 for 2), 3 night.
pub fn day_period(hour: i32) -> i32 {
    if hour < 7 {
        3
    } else if hour < 9 {
        0
    } else if hour < 21 {
        1
    } else {
        (23 <= hour) as i32 + 2
    }
}

/// A playfield's districts and how its positions map to their zones.
pub struct DistrictMap {
    pub districts: mao_formats::districts::Districts,
    /// Outdoors: the tile map's size in tiles, its tile size and the zone size in tiles (N3's zone grid); none indoors.
    grid: Option<(u32, u32, f32, u32)>,
}

impl DistrictMap {
    /// The playfield's districts (0xf424e) and, outdoors, its zone grid (the tile map 0xf4249 and the zone size of 0xf4241).
    pub fn load(db: &mao_formats::resource::ResourceDatabase, playfield: u32) -> Option<DistrictMap> {
        use mao_formats::resource::Id;
        let districts = mao_formats::districts::parse(&db.get(Id { kind: mao_formats::districts::KIND, instance: playfield }).ok()??)?;
        let grid = (|| {
            let definition = db.get(Id { kind: mao_formats::playfield::KIND, instance: playfield }).ok()??;
            let map = mao_formats::tilemap::parse_old(&db.get(Id { kind: 0xf4249, instance: playfield }).ok()??).ok()?;
            Some((map.width as u32, map.height as u32, map.tile_size, mao_formats::districts::zone_size(&definition)?))
        })();
        Some(DistrictMap { districts, grid })
    }

    /// FUN_100b72bf's district sound and music layer at a position and hour: the district of the character's zone (indoors
    /// zone 0: the client keeps no rooms' zones), its music for the day's period (+1: 1 dawn to 4 night; the weather's 5 to
    /// 7, past 0.4 of a precipitation, are not followed), held to 0..0xffff. (The playfield's music override and the land
    /// control layer, -911, are not followed.)
    pub fn music(&self, position: [f32; 3], hour: i32) -> Option<(u32, i32)> {
        let zone = match self.grid {
            Some((w, h, tile, size)) => mao_formats::districts::zone_at(position, tile, w, h, size),
            None => 0,
        };
        let d = self.districts.of_zone(zone)?;
        let layer = d.music[(day_period(hour) + 1) as usize] as i32;
        Some((d.music[0] as u32, if (0..=0xffff).contains(&layer) { layer } else { 0xffff }))
    }
}

/// A death's sound (`CharDie_t`, FUN_1007b803): an animation set's for the death (0x1e), else by the sex stat (59) a woman's
/// (3) or a man's.
pub fn death(set: Option<(&mao_formats::npc::Template, u32)>, sex: u8) -> Option<u32> {
    match set {
        Some((set, seed)) => set.sound(0x1e, seed),
        None => Some(sound_id(if sex == 3 { "SM_Sandy_Game_FemaleDies" } else { "SM_Sandy_Game_MaleDies" })),
    }
}

/// A play of `PlayGameSound` with its place, deferral and mode (the variant offset by a hit's size).
#[derive(Debug, PartialEq)]
pub struct GamePlay {
    pub id: u32,
    pub position: [f32; 3],
    pub volume: f32,
    pub material: i32,
    pub defer: f32,
    pub mode: i32,
}

/// The one hit by an attack (FUN_1009b9f3's `SimpleChar_t`).
pub struct Target<'a> {
    pub position: [f32; 3],
    /// Its maximum health (+0x218).
    pub max_health: i64,
    /// Its animation set (FUN_100524b2) and the seed picking a variant.
    pub set: Option<(&'a mao_formats::npc::Template, u32)>,
    /// Its breed (stat 4) and sex (stat 59).
    pub breed: u8,
    pub sex: u8,
}

/// An attack note's attack: the attacker, the weapon of the slot the swing used (its template's sound set, chunk 0x14 into the
/// item's +0x88, and its Energy, stat 26) and what FUN_1006ae3c put on the slot's weapon from the server's `AttackInfo`: the
/// hit type (+0x2c: 2 glancing, 3 normal, 4 critical; a miss less) and the damage (+0x30).
pub struct Attack<'a> {
    pub event: i32,
    pub position: [f32; 3],
    /// The attacker is an NPC (+0x21c).
    pub npc: bool,
    pub weapon: &'a [(u32, Vec<u32>)],
    pub energy: Option<i64>,
    pub hit: i32,
    pub amount: i64,
    pub target: Option<Target<'a>>,
}

/// The sounds of an attack note (FUN_10068e42 → FUN_1009bd92): the weapon's (FUN_1009d2b2: its sound for the event, or with no
/// Energy left in a player's hands its 0x2c, at the attacker) and the hit's (FUN_1009b9f3, for a hit: the weapon's sound for
/// the event or else its attack's, 0xb, at the attacker; with damage, at the one hit and 0.4 s later, the impacts of an animation
/// set (the weapon's 0x1f and the set's 0x1f on the set's material, stat 0x29) or a player's being hit (by sex, on material 7),
/// the variant moved by the damage against the maximum health: mode 2 from a tenth, 1 from a twentieth, else 0). `pick`
/// stands for FUN_10045976's `rand()`.
pub fn attack(a: &Attack, mut pick: impl FnMut() -> u32) -> Vec<GamePlay> {
    let mut plays = Vec::new();
    let mut weapon_sound = |key: i32, pick: &mut dyn FnMut() -> u32| {
        let all: Vec<u32> = a.weapon.iter().filter(|s| s.0 as i32 == key).flat_map(|s| s.1.iter().copied()).collect();
        (!all.is_empty()).then(|| all[pick() as usize % all.len()])
    };
    let at = |id: u32, position: [f32; 3]| GamePlay { id, position, volume: 1.0, material: 0, defer: 0.0, mode: 1 };
    if !a.weapon.is_empty() {
        let key = if a.energy == Some(0) && !a.npc { 0x2c } else { a.event };
        if let Some(id) = weapon_sound(key, &mut pick) {
            plays.push(at(id, a.position));
        }
    }
    let Some(t) = &a.target else { return plays };
    if a.hit < 2 {
        return plays;
    }
    let fired = weapon_sound(a.event, &mut pick).or_else(|| if a.event != 0xb { weapon_sound(0xb, &mut pick) } else { None });
    if let Some(id) = fired {
        plays.push(at(id, a.position));
    }
    if a.amount <= 0 {
        return plays;
    }
    let mode = if a.amount >= t.max_health / 10 {
        2
    } else if a.amount >= t.max_health / 20 {
        1
    } else {
        0
    };
    let impact = |id: u32, material: i32| GamePlay { id, position: t.position, volume: 1.0, material, defer: f32::from_bits(0x3ecccccd), mode };
    let (mut material, mut sound) = (0i32, None);
    if let Some((set, seed)) = t.set {
        material = set.stat(0x29) as i32;
        sound = set.sound(0x1f, seed);
        if (1..=0x11).contains(&material) {
            if let Some(id) = weapon_sound(0x1f, &mut pick) {
                plays.push(impact(id, material));
            }
        }
    }
    if material == 0 && matches!(t.breed, 1 | 7 | 3 | 2 | 4) {
        material = 7;
        match t.sex {
            2 | 1 => sound = Some(sound_id("SM_Sandy_Game_MaleGetsHit")),
            3 => sound = Some(sound_id("SM_Sandy_Game_FemaleGetsHit")),
            _ => {}
        }
    }
    if let Some(id) = sound.filter(|_| (1..=0x11).contains(&material)) {
        plays.push(impact(id, material));
    }
    plays
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
    fn a_shot_and_its_hit() {
        let weapon = vec![(0xb, vec![100, 101]), (0x1f, vec![200]), (0x2c, vec![300])];
        let mut set = mao_formats::npc::Template::default();
        set.stats = vec![(0x29, 3)];
        set.sounds = vec![(0x1f, 400)];
        let target = |set| Some(Target { position: [5.0; 3], max_health: 1000, set, breed: 1, sex: 3 });
        let shot = |hit, amount, target| Attack { event: 0xb, position: [1.0; 3], npc: false, weapon: &weapon, energy: Some(5), hit, amount, target };
        // (a normal hit for 60 of 1000 on a creature: the shot twice (one play in the frame), its impact and the creature's on dirt)
        let p = attack(&shot(3, 60, target(Some((&set, 0)))), || 1);
        assert_eq!(p.iter().map(|p| (p.id, p.position[0], p.material, p.mode)).collect::<Vec<_>>(), vec![(101, 1.0, 0, 1), (101, 1.0, 0, 1), (200, 5.0, 3, 1), (400, 5.0, 3, 1)]);
        assert_eq!(p[3].defer, 0.4);
        // (a woman hit hard; a miss only fires; a player's empty gun clicks)
        let p = attack(&shot(4, 100, target(None)), || 0);
        assert_eq!(p.last().map(|p| (p.id, p.material, p.mode)), Some((sound_id("SM_Sandy_Game_FemaleGetsHit"), 7, 2)));
        assert_eq!(attack(&shot(1, 0, target(None)), || 0).len(), 1);
        assert_eq!(attack(&Attack { energy: Some(0), ..shot(1, 0, None) }, || 0)[0].id, 300);
    }

    #[test]
    fn deaths() {
        let mut set = mao_formats::npc::Template::default();
        set.sounds = vec![(0x1e, 9)];
        assert_eq!(death(Some((&set, 4)), 2), Some(9));
        assert_eq!(death(None, 3), Some(sound_id("SM_Sandy_Game_FemaleDies")));
        assert_eq!(death(None, 1), Some(sound_id("SM_Sandy_Game_MaleDies")));
        assert_eq!(death(Some((&mao_formats::npc::Template::default(), 0)), 3), None);
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
