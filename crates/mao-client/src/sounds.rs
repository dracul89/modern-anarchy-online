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
    /// The material under it ([`ground_material`], then [`liquid_material`]).
    pub material: i32,
    /// How deep under a liquid's surface it is (`Vehicle_t` +0x10c, [`liquid_depth`]): 0 or less out of one.
    pub liquid: f32,
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
                // (a landing louder, twice the step's held to 1; none in a liquid)
                0x85 if 0.0 < n.liquid => return None,
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
/// [`liquid_material`] has it.
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

/// How deep a character is under the playfield's liquid, as its vehicle keeps it (`Vehicle_t` +0x10c, the vehicle drop-in's
/// `ensure_surface_alignment`): the surface's height (`LiquidMediumData_t::m_vLiquidHeight`, from the ground probe's
/// `n3Playfield_t::GetLiquidSurfaceHeight`) less its own when under it, with the liquid's type (the probe's +0x110); none
/// out of one. (The original's probe tests at the ground point under the character with the character's height for the
/// depth; here the test is at the character, and the 0.1 above the surface where the vehicle keeps its last depth is out.)
pub fn liquid_depth(liquid: Option<&crate::liquid::Liquid>, position: [f32; 3]) -> (f32, u32) {
    match liquid.and_then(|l| l.at(position)) {
        Some(s) if position[1] < s.height => (s.height - position[1], s.kind),
        _ => (-9999.0, 0),
    }
}

/// FUN_100452d3's material in a liquid (over [`ground_material`]'s): 10 for the liquid type 2, 11 for type 6, else 8 when
/// deeper than 0.35 (`_DAT_1015ef34`) and 9 when not.
pub fn liquid_material(ground: i32, depth: f32, kind: u32) -> i32 {
    if !(0.0 < depth) {
        ground
    } else if kind == 2 {
        10
    } else if kind == 6 {
        11
    } else if depth <= 0.35 {
        9
    } else {
        8
    }
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
    /// Outdoors, a zone's side in world units (the tile size times the zone size in tiles).
    pub fn zone_side(&self) -> Option<f32> {
        self.grid.map(|(_, _, tile, zone)| tile * zone as f32)
    }

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

    /// The district of the zone a position is in (outdoors the zone grid's; indoors zone 0: the client keeps no rooms'
    /// zones), as `PlayfieldDistrictInfo_t::GetDistrictData(GetZoneInstanceID())` finds it for the character.
    pub fn district_at(&self, position: [f32; 3]) -> Option<&mao_formats::districts::District> {
        let zone = match self.grid {
            Some((w, h, tile, size)) => mao_formats::districts::zone_at(position, tile, w, h, size),
            None => 0,
        };
        self.districts.of_zone(zone)
    }

    /// Gamecode's `N3Msg_GetLCDistrictInfo` (0x1001cfe9) for the character at a position: its district's name and land
    /// control level range (`DistrictData_t` +8, +0x44, +0x48); none outside a district.
    pub fn land_control_info(&self, position: [f32; 3]) -> Option<(String, i32, i32)> {
        self.district_at(position).map(|d| (d.name.clone(), d.control.0 as i32, d.control.1 as i32))
    }

    /// FUN_100b72bf's district sound and music layer at a position and hour under the weather: the district of the
    /// character's zone (indoors zone 0: the client keeps no rooms' zones), its music of the slot [`music_slot`] picks, or
    /// the layer `over` finds for the district's name (the playfield's override, `GetDistrictMusicOverride` then
    /// `FindLayerID`), 0xffff when out of 0..0xffff; with Notum Wars (`expansion`: the sound module's +0xbc, FUN_100b1f3c
    /// setting it for good once the character's stat 389 has bit 0) in a district of land control (its range's low end,
    /// `DistrictData_t` +0x44, not 0) the land control layer -911 (`SetupENVSounds`' `Landcontrol_neutral`).
    pub fn music(&self, position: [f32; 3], hour: i32, weather: &[f32; 14], expansion: bool, over: impl FnOnce(&str) -> Option<i32>) -> Option<(u32, i32)> {
        let d = self.district_at(position)?;
        let layer = over(&d.name).unwrap_or(d.music[music_slot(hour, weather)] as i32);
        let layer = if (0..=0xffff).contains(&layer) { layer } else { 0xffff };
        Some((d.music[0] as u32, if expansion && d.control.0 != 0 { -0x38f } else { layer }))
    }
}

/// A `ScriptIIR_t` (0x204f4871): what FUN_101308ee reads after the header byte, big-endian: three words (+0x18, +0x1c, the
/// type +0x20), a text (+0x24: a 16-bit length and the bytes, FUN_10038af8; none past 0x7fff), the start, the length and
/// the server's time it was sent (+0x40, +0x44, +0x48, seconds), a text (+0x4c) and a place (+0x68, +0x6c).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptIir {
    pub words: [i32; 2],
    pub kind: i32,
    pub script: String,
    pub start: i32,
    pub length: i32,
    pub sent: i32,
    pub name: String,
    pub place: [i32; 2],
}

/// The message kind of a `ScriptIIR_t`.
pub const SCRIPT_IIR: u32 = 0x204f4871;

impl ScriptIir {
    pub fn parse(data: &[u8]) -> Option<ScriptIir> {
        let mut at = 1;
        let mut word = || {
            let w = i32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?);
            at += 4;
            Some(w)
        };
        let (a, b, kind) = (word()?, word()?, word()?);
        let text = |at: &mut usize| {
            let n = u16::from_be_bytes(data.get(*at..*at + 2)?.try_into().ok()?) as usize;
            *at += 2;
            if n >= 0x8000 {
                return None;
            }
            let s = String::from_utf8_lossy(data.get(*at..*at + n)?).into_owned();
            *at += n;
            Some(s)
        };
        let script = text(&mut at)?;
        let w = |at: &mut usize| {
            let v = i32::from_be_bytes(data.get(*at..*at + 4)?.try_into().ok()?);
            *at += 4;
            Some(v)
        };
        let (start, length, sent) = (w(&mut at)?, w(&mut at)?, w(&mut at)?);
        let name = text(&mut at)?;
        let place = [w(&mut at)?, w(&mut at)?];
        Some(ScriptIir { words: [a, b], kind, script, start, length, sent, name, place })
    }
}

/// The playfield's district music overrides (`PlayfieldAnarchy_t` +0xe0: by the `ElfHash` of a district's name, a stack of
/// (music name, when it ends; 0 or less never)), set by a `ScriptIIR_t` of type 3 and popped by one of type 1 (FUN_10130a1b),
/// with the clock's lag behind the server's it keeps (DAT_102eaca4: the first IIR's, then any smaller).
#[derive(Clone, Debug, Default)]
pub struct MusicOverrides {
    lag: Option<i32>,
    map: std::collections::HashMap<u32, Vec<(String, i32)>>,
}

impl MusicOverrides {
    /// FUN_10130a1b at `now` (`GameTime_t` +0xb4, the server's seconds: `Clock::server_time`): the start moved by the lag
    /// (returned, for the script types' text: `fxs::script_event`); type 3 pushes the IIR's script text as its name's
    /// override for the start plus the length less now, `SetDistrictMusicOverride` making it an end (when over 0); type 1
    /// pops its name's last (`PopDistrictMusicOverride`) when no script has made an object (`scripted`, FUN_1002a15e's).
    pub fn script(&mut self, iir: &ScriptIir, now: i32, scripted: bool) -> i32 {
        let behind = now.wrapping_sub(iir.sent);
        let lag = match self.lag {
            Some(l) if l <= behind => l,
            _ => behind,
        };
        self.lag = Some(lag);
        let start = iir.start.wrapping_add(lag);
        let key = mao_formats::ldb::elf_hash(&iir.name);
        match iir.kind {
            3 => {
                let left = iir.length.wrapping_sub(now).wrapping_add(start);
                let end = if 0 < left { left.wrapping_add(now) } else { left };
                self.map.entry(key).or_default().push((iir.script.clone(), end));
            }
            1 if !scripted => {
                if let Some(s) = self.map.get_mut(&key) {
                    s.pop();
                }
            }
            _ => {}
        }
        start
    }

    /// `GetDistrictMusicOverride(name)` at `now`: the ended overrides dropped from the top (FUN_10122d49: an end other than
    /// 0 at or before now, as unsigned), then the top's music name, none for an empty stack.
    pub fn get(&mut self, name: &str, now: i32) -> Option<String> {
        let s = self.map.get_mut(&mao_formats::ldb::elf_hash(name))?;
        while let Some(&(_, end)) = s.last() {
            if end == 0 || (now as u32) <= (end as u32) {
                break;
            }
            s.pop();
        }
        s.last().map(|o| o.0.clone())
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

/// FUN_100b72bf's music slot: past 0.4 of the strongest precipitation (the weather's entries 3 to 6, FUN_100be088 over
/// FUN_100bf4f9) the seventh, else past 0.4 of entry 0 (FUN_100bdb85) the sixth, else past 0.4 of entry 1 (FUN_100bdb9e) the
/// fifth, else the day's period's (+1: 1 dawn to 4 night).
pub fn music_slot(hour: i32, weather: &[f32; 14]) -> usize {
    // (FUN_1003c45f keeps the first unless the second is greater: max, with a NaN losing to the other)
    let most = |a: f32, b: f32| if b <= a { a } else { b };
    let precipitation = most(most(weather[6], weather[5]), most(weather[3], weather[4]));
    if precipitation > 0.4 {
        7
    } else if weather[0] > 0.4 {
        6
    } else if weather[1] > 0.4 {
        5
    } else {
        (day_period(hour) + 1) as usize
    }
}

/// A nano cast's sounds, as `CharCastNano_t` plays the nano's own (its template's stats 269 to 272): made (FUN_1007b5cd), the
/// gather's (269) at the caster, at 0.6 and held 0.2 s; each frame while it gathers (FUN_1007b1e3's phase 0, until the server
/// starts the cast) the gather's again at full volume; when the cast starts (FUN_1004f434) the cast's (270) and the travel's
/// (271, held 0.2 s) at the caster; while the cast animation plays (phase 1) the travel's each frame; when it ends the hit's
/// (272) at the target. The cast animation's end is taken as the nano's cast time (its stat 294, in hundredths of a second):
/// the client does not ask the animation.
#[derive(Clone, Debug)]
pub struct NanoCast {
    pub nano: u32,
    pub target: u32,
    /// None while gathering; the cast's end once started.
    pub until: Option<std::time::Instant>,
    /// When the gather began (a gather the server never answers ends after 5 s).
    pub began: std::time::Instant,
}

/// The casts under way, by caster.
#[derive(Default)]
pub struct NanoCasts {
    pub casts: std::collections::HashMap<u32, NanoCast>,
}

/// `GetStat(stat)` of a nano's template, a sound to play (0 for none).
pub type NanoStat<'a> = &'a dyn Fn(u32, u32) -> Option<i64>;

impl NanoCasts {
    /// FUN_1007b5cd: a cast made (the client's own request, or the server's cast of someone): the gather's sound at 0.6, held.
    pub fn gather(&mut self, caster: u32, nano: u32, stat: NanoStat) -> Vec<(u32, f32, f32)> {
        let now = std::time::Instant::now();
        self.casts.insert(caster, NanoCast { nano, target: 0, until: None, began: now });
        sound(stat(nano, 0x10d)).map(|id| vec![(id, 0.2, f32::from_bits(0x3f19999a))]).unwrap_or_default()
    }

    /// The server's `CastNanoSpell`: the cast made if it was not gathering, then started (FUN_1004f434): the cast's and the
    /// travel's sounds.
    pub fn cast(&mut self, caster: u32, target: u32, nano: u32, stat: NanoStat) -> Vec<(u32, f32, f32)> {
        let mut plays = Vec::new();
        if self.casts.get(&caster).is_none_or(|c| c.nano != nano || c.until.is_some()) {
            plays.extend(self.gather(caster, nano, stat));
        }
        let time = stat(nano, 294).unwrap_or(0).max(0) as u64 * 10;
        if let Some(c) = self.casts.get_mut(&caster) {
            c.target = target;
            c.until = Some(std::time::Instant::now() + std::time::Duration::from_millis(time));
        }
        plays.extend(sound(stat(nano, 0x10e)).map(|id| (id, 0.0, 1.0)));
        plays.extend(sound(stat(nano, 0x10f)).map(|id| (id, 0.2, 1.0)));
        plays
    }

    /// FUN_1007b1e3's frame: (caster or target, sound, delay, volume) to play now; the ended casts dropped.
    pub fn frame(&mut self, stat: NanoStat) -> Vec<(u32, u32, f32, f32)> {
        let now = std::time::Instant::now();
        let mut plays = Vec::new();
        self.casts.retain(|&caster, c| match c.until {
            None => {
                if now.duration_since(c.began) > std::time::Duration::from_secs(5) {
                    return false;
                }
                plays.extend(sound(stat(c.nano, 0x10d)).map(|id| (caster, id, 0.2, 1.0)));
                true
            }
            Some(end) if now < end => {
                plays.extend(sound(stat(c.nano, 0x10f)).map(|id| (caster, id, 0.2, 1.0)));
                true
            }
            Some(_) => {
                if c.target != 0 {
                    plays.extend(sound(stat(c.nano, 0x110)).map(|id| (c.target, id, 0.0, 1.0)));
                }
                false
            }
        });
        plays
    }
}

fn sound(v: Option<i64>) -> Option<u32> {
    v.filter(|&v| v != 0).map(|v| v as u32)
}

/// The sounds `_GfxControlAudio_t` (an effect template of kind 4000, Gamecode FUN_100d3616) names, by its word 0 (the table at
/// 0x102c4f28, 64-byte names).
const EFFECT_SOUNDS: [&str; 14] = [
    "SM_Sandy_Buff_Ping",
    "SM_Sandy_Debuff_Ping",
    "SM_Sandy_Tower_Beam",
    "SM_Sandy_Tower_Hit",
    "SM_Sandy_Tower_Fire_Start",
    "SM_Sandy_Tower_Fire",
    "SM_Sandy_Tower_Creation_Basic",
    "SM_Sandy_Tower_Creation_Superior",
    "SM_Sandy_Tower_Destruction_Basic",
    "SM_Sandy_Tower_Destruction_Superior",
    "SM_Sandy_Tower_Powerdown",
    "SM_Sandy_Game_Explo_small",
    "SM_Sandy_Game_Explo_Med",
    "SM_Sandy_Game_Explo_Big",
];

/// A sound control of an effect (`_GfxControlAudio_t`, FUN_100d3616): its sound, velocity (words 1–3), volume (4, held to
/// 0..1), range (5), delay (6), deferral (7) and pitch (8). When it is first run (FUN_100d3567) it sends SandyInterface's
/// `PlaySoundCommand` (AFCM 0x1a, 0x103) at the effect's place: once.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectSound {
    pub id: u32,
    pub velocity: [f32; 3],
    pub volume: f32,
    pub range: f32,
    pub delay: f32,
    pub defer: f32,
    pub pitch: i32,
}

/// The sound controls of an effect template's tree (each made with the effect, `CreateGfxControl`).
pub fn effect_sounds(gfx: &mao_formats::gfx::Gfx, effect: i32) -> Vec<EffectSound> {
    gfx.tree(effect)
        .into_iter()
        .filter(|r| r.kind == 4000)
        .filter_map(|r| {
            let id = sound_id(EFFECT_SOUNDS.get(usize::try_from(r.int(0)).ok()?)?);
            let volume = r.float(4);
            Some(EffectSound {
                id,
                velocity: [r.float(1), r.float(2), r.float(3)],
                volume: if volume < 0.0 { 0.0 } else if 1.0 < volume { 1.0 } else { volume },
                range: r.float(5),
                delay: r.float(6),
                defer: r.float(7),
                pitch: r.int(8),
            })
        })
        .collect()
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
    let weapon_sound = |key: i32, pick: &mut dyn FnMut() -> u32| {
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
        let at = |event: i32| Note { event, position: [0.0; 3], set: None, falling: false, material: 3, liquid: -9999.0 };
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
    fn a_nano_cast_s_sounds() {
        // (a nano of gather 1, cast 2, travel 3, hit 4 and no cast time)
        let stat = |_: u32, s: u32| Some(match s {
            0x10d => 1,
            0x10e => 2,
            0x10f => 3,
            0x110 => 4,
            _ => 0,
        });
        let mut c = NanoCasts::default();
        assert_eq!(c.gather(7, 99, &stat), vec![(1, 0.2, f32::from_bits(0x3f19999a))]);
        assert_eq!(c.frame(&stat), vec![(7, 1, 0.2, 1.0)]);
        assert_eq!(c.cast(7, 8, 99, &stat), vec![(2, 0.0, 1.0), (3, 0.2, 1.0)]);
        // (the cast time is over: the hit at the target, the cast gone)
        assert_eq!(c.frame(&stat), vec![(8, 4, 0.0, 1.0)]);
        assert!(c.casts.is_empty());
        // (someone else's cast, not gathered: made then started)
        assert_eq!(c.cast(5, 5, 99, &stat).len(), 3);
    }

    #[test]
    fn an_effect_s_sound_controls() {
        // (a bundle, 500, of a sound control, 600: the medium explosion at full volume, range 120, pitch 100)
        let mut words: Vec<u32> = vec![2, 500, 2007, 1, 600, 600, 4000, 9, 12];
        words.extend([0.0f32, 0.0, 0.0, 1.5, 120.0, 0.0, 0.25].map(f32::to_bits));
        words.push(100);
        let data: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let gfx = mao_formats::gfx::Gfx::parse(&data).unwrap();
        assert_eq!(effect_sounds(&gfx, 500), vec![EffectSound { id: sound_id("SM_Sandy_Game_Explo_Med"), velocity: [0.0; 3], volume: 1.0, range: 120.0, delay: 0.0, defer: 0.25, pitch: 100 }]);
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

    #[test]
    fn steps_in_a_liquid() {
        assert_eq!(liquid_material(0xf, -9999.0, 0), 0xf);
        assert_eq!(liquid_material(0xf, 0.2, 0), 9);
        assert_eq!(liquid_material(0xf, 0.35, 4), 9);
        assert_eq!(liquid_material(0xf, 0.5, 0), 8);
        assert_eq!(liquid_material(0xf, 0.01, 2), 10);
        assert_eq!(liquid_material(0xf, 3.0, 6), 11);
        let tri = |points| mao_formats::playfield::WaterTriangle { kind: 4, value: 100000.0, points };
        let l = crate::liquid::Liquid::new(&[tri([[100.0, 20.0, 100.0], [300.0, 20.0, 100.0], [300.0, 20.0, 300.0]]), tri([[100.0, 20.0, 100.0], [300.0, 20.0, 300.0], [100.0, 20.0, 300.0]])]);
        assert_eq!(liquid_depth(Some(&l), [210.0, 19.5, 160.0]), (0.5, 4));
        assert_eq!(liquid_depth(Some(&l), [210.0, 20.5, 160.0]).0, -9999.0);
        assert_eq!(liquid_depth(None, [210.0, 19.5, 160.0]).0, -9999.0);
        // (a landing in it is silent; a step in it sounds)
        let g = Generic::default();
        let n = |event: i32| Note { event, position: [0.0; 3], set: None, falling: false, material: 9, liquid: 0.5 };
        assert_eq!(note(&g, &n(0x85)), None);
        assert_eq!(note(&g, &n(0x26)), Some(Play { id: g.walk, volume: 1.0, material: 9 }));
    }

    #[test]
    fn a_district_s_music_override() {
        let mut data = vec![0u8];
        let text = |d: &mut Vec<u8>, s: &str| {
            d.extend((s.len() as u16).to_be_bytes());
            d.extend(s.as_bytes());
        };
        for w in [1, 0, 3] {
            data.extend(i32::to_be_bytes(w));
        }
        text(&mut data, "EP_01\\Battle_new");
        for w in [1000, 60, 990] {
            data.extend(i32::to_be_bytes(w));
        }
        text(&mut data, "Newland City");
        data.extend([0u8; 8]);
        let iir = ScriptIir::parse(&data).unwrap();
        assert_eq!((iir.kind, iir.script.as_str(), iir.name.as_str(), iir.start, iir.length), (3, "EP_01\\Battle_new", "Newland City", 1000, 60));
        // (the client 10 s behind: it starts at 1010 and ends at 1070)
        let mut o = MusicOverrides::default();
        o.script(&iir, 1000, false);
        assert_eq!(o.get("Newland City", 1069).as_deref(), Some("EP_01\\Battle_new"));
        assert_eq!(o.get("Other", 1069), None);
        assert_eq!(o.get("Newland City", 1071), None);
        // (one already over never ends; a type 1 pops it)
        o.script(&ScriptIir { length: 0, sent: 0, start: -100, ..iir.clone() }, 1000, false);
        assert!(o.get("Newland City", 5000).is_some());
        o.script(&ScriptIir { kind: 1, ..iir.clone() }, 1000, true);
        assert!(o.get("Newland City", 5000).is_some());
        o.script(&ScriptIir { kind: 1, ..iir.clone() }, 1000, false);
        assert_eq!(o.get("Newland City", 5000), None);
    }

    #[test]
    fn the_weather_s_music() {
        let mut w = [0.0f32; 14];
        assert_eq!(music_slot(12, &w), 2);
        assert_eq!(music_slot(3, &w), 4);
        w[1] = 0.5;
        assert_eq!(music_slot(12, &w), 5);
        w[0] = 0.41;
        assert_eq!(music_slot(12, &w), 6);
        w[5] = 0.6;
        assert_eq!(music_slot(12, &w), 7);
        // (0.4 itself is not past it)
        assert_eq!(music_slot(8, &[0.4; 14]), 1);
    }
}
