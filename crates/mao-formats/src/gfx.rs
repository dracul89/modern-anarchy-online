//! The effect templates (`Setupf/gfxtweak.bin`): what the original draws when a nano program is cast, lands, runs on a character or
//! flies as a tracer. The ids a nano template names (`CastEffectType` 428, `HitEffectType` 361, `EffectType` 413, `TracerEffectType`
//! 419, `GatherEffectType` 366, `ImpactEffectType` 414) and the `GfxEffect` spell's argument are keys here.
//!
//! The file is little-endian 32-bit words: the number of records, then each record as `id, kind, n` and `n` payload words. Every record
//! of the real file is read this way (2687 of 2687, to the last word). The kind is the class of the template: `_EffectHandler_t::
//! CreateGfxControl` (Gamecode.dll) makes a control of a class for the kinds 1005, 1013, 1018, 1019, 1021, 1022, 1024-1027, 2002, 2004,
//! 2005, 2007, 2010, 2013, 3011, 3015, 3017, 3019, 3020, 3026, 3028 and 4000; the other kinds (1001-1004, 1006-1012...) are parts that
//! those refer to. A payload word is an integer, a float (as its bit pattern) or the id of another record: a template is a tree.
//!
//! What is known of the payloads (from the records themselves; the field meanings are partly guessed from their values):
//! * the first payload word is a flag set (5, 7, 513, 771, 1799...), words 1 to 6 are zero or an offset and a rotation (a position of
//!   `0, -0.25, 0` and an angle of 1.57 are seen), word 7 often a reference (a child template or a sprite);
//! * a particle emitter (kinds 1003, 1005-1008, 1010) carries a count, start and end sizes, a start and an end colour as alpha, red,
//!   green, blue (`1 0.5 0.4 1  0 0.5 0.4 1`: fading out), and angles `0 6.28 -1.57 1.57` (a full turn around, a hemisphere up);
//! * kind 1001, the persistent effect of a running buff (`EffectType` 413 of 486 nanos), is a colour (`0.2 0.2 0.99` blue,
//!   `0.99 0.2 0.2` red...), the seconds between its pulses as a float in word 20 (4 in 36 of them, 1 in 20, 2 in one) and two
//!   references to kind 1002 placeholders (`_GfxControlBuffPlaceHolder_t`): a point that rides a circle of radius word 21 (0.15 m)
//!   at word 3 metres from attach point word 7 on the character, from angle word 22 at word 23 radians per second (±12.6), for
//!   word 8 = 1.5 seconds, carrying the sparkle emitter word 10 (kind 1005) and trailing the emitter word 11 (kind 1003, the
//!   light_halo2 material at 60 a second): that trail is the ring one sees. Word 26 names a kind-0 table of (offset, radius) per
//!   breed, sex and fatness that replaces words 3 and 21 for the body. The 1001 itself is a state machine (`_GfxControlBPHFSM_t`,
//!   see `pulses` in the client) that plays the two placeholders every word-20 seconds, word-21 times (-1 forever), through a 5 s
//!   gate shared by all the buffs of one character;
//! * kind 1010 is the cast effect (`CastEffectType` of 1320 nanos): a few colours and the references of its six parts.

use std::collections::HashMap;

/// The sprite sheets the effects draw with, by the material number the templates name (word 9 of an emitter): the texture's file name
/// (a type-0xf6954 object), its columns and rows of frames, and the first and last frame played. Read from `_MaterialManager_t::
/// GetMaterial` (Gamecode.dll), whose switch has 102 cases.
pub const MATERIALS: [(&str, u8, u8, u8, u8); 102] = [
    ("advertisement_test.png", 2, 2, 0, 3),
    ("circle.png", 1, 1, 0, 0),
    ("Electricify.png", 4, 4, 0, 14),
    ("explo04_01_v01.png", 8, 8, 0, 30),
    ("explo05_01_v01.png", 4, 4, 0, 14),
    ("explo06_01_v01.png", 8, 8, 0, 45),
    ("hippoexplo01_01_v01.png", 4, 4, 0, 14),
    ("light_halo.png", 1, 1, 0, 0),
    ("light_halo2.png", 1, 1, 0, 0),
    ("mine_explosion02_01_v01.png", 8, 8, 0, 62),
    ("muzzleflash.png", 8, 8, 0, 52),
    ("new_explo01_v01.png", 8, 8, 0, 29),
    ("new_explo05_v01.png", 8, 8, 0, 30),
    ("noise.PNG", 1, 1, 0, 0),
    ("rocket_explosion01_v01.png", 8, 8, 0, 30),
    ("s_bullet.png", 1, 1, 0, 0),
    ("s_bulletfront.png", 1, 1, 0, 0),
    ("s_exp1noise.png", 8, 8, 0, 63),
    ("s_explosion1.png", 8, 8, 0, 63),
    ("s_explosion2.png", 8, 8, 0, 63),
    ("s_explosion3.png", 8, 8, 0, 63),
    ("s_flame.png", 8, 8, 0, 63),
    ("s_muzzleflash.png", 8, 8, 0, 52),
    ("s_smokebomb.png", 8, 8, 0, 63),
    ("s_smokebombblack.png", 8, 8, 0, 63),
    ("s_smokebombbrown.png", 8, 8, 0, 63),
    ("shock_stripes.png", 4, 2, 0, 7),
    ("shock_stripes2.png", 1, 1, 0, 0),
    ("smoke01_01_v02.png", 4, 4, 8, 15),
    ("snakeweapon_impact01_01_v01.png", 8, 8, 0, 30),
    ("swirl.png", 1, 1, 0, 0),
    ("x_smoke.png", 8, 8, 3, 63),
    ("thin_smoke.png", 1, 1, 0, 0),
    ("star_2_ball.png", 8, 2, 0, 15),
    ("wave_alpha.png", 1, 1, 0, 0),
    ("watersplash_64x64.png", 1, 1, 0, 0),
    ("ripple.png", 1, 1, 0, 0),
    ("ring_64x64.png", 1, 1, 0, 0),
    ("water_drop.png", 4, 2, 0, 0),
    ("whitesmoke.png", 8, 8, 3, 63),
    ("spritefont.png", 1, 1, 0, 0),
    ("tower_shockwave1.png", 1, 1, 0, 0),
    ("tower_lightningeffect.png", 4, 2, 0, 7),
    ("explosion_big.png", 4, 4, 0, 15),
    ("tower_sidebeam_clan.png", 1, 1, 0, 0),
    ("tower_sidebeam_omni.png", 1, 1, 0, 0),
    ("tower_sidebeam_neutral.png", 1, 1, 0, 0),
    ("vein.png", 1, 1, 0, 0),
    ("vein2.png", 1, 1, 0, 0),
    ("mine_explosion02_bw_01_v01.png", 8, 8, 0, 62),
    ("whitemap8x8.png", 8, 8, 0, 0),
    ("bubbles_4frames_animation.png", 2, 2, 0, 3),
    ("alien_beam.png", 1, 1, 0, 0),
    ("spaceship_engine_trail.png", 1, 1, 0, 0),
    ("spaceship_engine_trail_colour.png", 1, 1, 0, 0),
    ("spaceship_engine_trail2.png", 1, 1, 0, 0),
    ("fog_particle.png", 1, 1, 0, 0),
    ("Energy_ring.png", 1, 1, 0, 0),
    ("cloud.png", 1, 1, 0, 0),
    ("lightning.png", 1, 1, 0, 0),
    ("muzzle_flash_mg_side.png", 1, 1, 0, 0),
    ("muzzle_flash_mg_front.png", 1, 1, 0, 0),
    ("muzzle_flash_blast.png", 1, 1, 0, 0),
    ("rays.png", 1, 1, 0, 0),
    ("twirl.png", 1, 1, 0, 0),
    ("blue_flash_side.png", 1, 1, 0, 0),
    ("blue_flash_front.png", 1, 1, 0, 0),
    ("blue_flash_side_upgraded.png", 1, 1, 0, 0),
    ("blue_flash_front_upgraded.png", 1, 1, 0, 0),
    ("spark.png", 1, 1, 0, 0),
    ("mothership_particles.png", 2, 2, 0, 3),
    ("explosion2.png", 8, 4, 0, 31),
    ("alien_sprite_01.png", 1, 1, 0, 0),
    ("flare.png", 1, 1, 0, 0),
    ("beam.png", 1, 1, 0, 0),
    ("projectile_ring.png", 1, 1, 0, 0),
    ("alien_sprite_02.png", 1, 1, 0, 0),
    ("energy_grid.png", 1, 1, 0, 0),
    ("effect_lava.png", 1, 1, 0, 0),
    ("circular_energy.png", 1, 1, 0, 0),
    ("muzzleflash_1_front.png", 1, 1, 0, 0),
    ("muzzleflash_1_side.png", 1, 1, 0, 0),
    ("muzzleflash_2_front.png", 1, 1, 0, 0),
    ("muzzleflash_2_side.png", 1, 1, 0, 0),
    ("muzzleflash_3_front.png", 1, 1, 0, 0),
    ("muzzleflash_3_side.png", 1, 1, 0, 0),
    ("muzzleflash_4_front.png", 1, 1, 0, 0),
    ("muzzleflash_4_side.png", 1, 1, 0, 0),
    ("muzzleflash_5_front.png", 1, 1, 0, 0),
    ("muzzleflash_5_side.png", 1, 1, 0, 0),
    ("flame.png", 8, 2, 0, 13),
    ("dots.png", 1, 1, 0, 0),
    ("plume.png", 1, 1, 0, 0),
    ("whoopwhite.png", 1, 1, 0, 0),
    ("exp_particle.png", 1, 1, 0, 0),
    ("groundhit.png", 1, 1, 0, 0),
    ("cord1.png", 1, 1, 0, 0),
    ("cloud2.png", 1, 1, 0, 0),
    ("the_wave.png", 1, 1, 0, 0),
    ("the_wave_fractal.png", 1, 1, 0, 0),
    ("xan_grid.png", 1, 1, 0, 0),
    ("fear_overhead.png", 1, 1, 0, 0),
];

/// A material's sheet: (file name, columns, rows, first frame, last frame).
pub fn material(id: i32) -> Option<(&'static str, u8, u8, u8, u8)> {
    usize::try_from(id).ok().and_then(|i| MATERIALS.get(i)).copied()
}

/// One template: its id, its class and its payload.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub id: i32,
    pub kind: i32,
    pub payload: Vec<i32>,
}

impl Record {
    /// A payload word read as a float.
    pub fn float(&self, at: usize) -> f32 {
        self.payload.get(at).map_or(0.0, |&w| f32::from_bits(w as u32))
    }

    pub fn int(&self, at: usize) -> i32 {
        self.payload.get(at).copied().unwrap_or(0)
    }
}

/// All the templates, by id.
#[derive(Debug, Default)]
pub struct Gfx {
    pub records: Vec<Record>,
    by_id: HashMap<i32, usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Malformed {
    pub at_word: usize,
}

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the effect templates end badly at word {}", self.at_word)
    }
}

impl Gfx {
    pub fn parse(data: &[u8]) -> Result<Gfx, Malformed> {
        let words: Vec<i32> = data.chunks_exact(4).map(|c| i32::from_le_bytes(c.try_into().unwrap())).collect();
        let count = *words.first().ok_or(Malformed { at_word: 0 })?;
        let mut gfx = Gfx::default();
        let mut at = 1;
        for _ in 0..count.max(0) {
            if at + 3 > words.len() {
                return Err(Malformed { at_word: at });
            }
            let (id, kind, n) = (words[at], words[at + 1], words[at + 2]);
            if n < 0 || at + 3 + n as usize > words.len() {
                return Err(Malformed { at_word: at });
            }
            let payload = words[at + 3..at + 3 + n as usize].to_vec();
            // (the first record of an id wins: a few ids repeat)
            gfx.by_id.entry(id).or_insert(gfx.records.len());
            gfx.records.push(Record { id, kind, payload });
            at += 3 + n as usize;
        }
        Ok(gfx)
    }

    pub fn get(&self, id: i32) -> Option<&Record> {
        self.by_id.get(&id).map(|&i| &self.records[i])
    }

    /// The records a template refers to: the payload words that are ids of other templates (not the template itself). Small numbers
    /// that happen to be ids are skipped (a flag of 5 or a count of 8 is not a reference), so only ids of 100 and up count.
    pub fn references(&self, id: i32) -> Vec<i32> {
        let Some(r) = self.get(id) else { return Vec::new() };
        r.payload.iter().copied().filter(|&w| w >= 100 && w != id && self.by_id.contains_key(&w)).collect()
    }

    /// The template and every template below it, each once, in the order they are first reached.
    pub fn tree(&self, id: i32) -> Vec<&Record> {
        let mut seen = Vec::new();
        let mut stack = vec![id];
        while let Some(next) = stack.pop() {
            if seen.contains(&next) {
                continue;
            }
            seen.push(next);
            stack.extend(self.references(next).into_iter().rev());
        }
        seen.into_iter().filter_map(|i| self.get(i)).collect()
    }
}

/// One of the two spirals of a running buff's aura (a kind-1002 template).
#[derive(Clone, Debug, PartialEq)]
pub struct Helix {
    /// Radians per second (negative: the other way round).
    pub spin: f32,
    /// Where it starts (radians).
    pub phase: f32,
    /// How wide each spark is, in metres.
    pub size: f32,
    /// How many sparks run along it.
    pub count: u32,
    /// The colour of its sparks (red, green, blue) before the buff's own tint.
    pub colour: [f32; 3],
    /// The material (sprite sheet, see [`MATERIALS`]) of its sparks.
    pub material: i32,
    /// The attach point on the body (word 7): 1000-1018 the `Bip01 *_ac` bones (1000 pelvis, 1001-1004 spine to spine3, 1005 neck,
    /// 1006 head, 1007/1008 left/right upper arm, 1009/1010 forearms, 1011/1012 thighs, 1013/1014 calves, 1015/1016 feet, 1017/1018
    /// hands), 2000-2023 the `Attractor*` nodes (2000 right hand, 2001 left hand, 2002 head, 2003 back, 2004/2005 left/right shoulder,
    /// then the specials); the original's tables at Gamecode.dll 0x102c63a8 and 0x102c63f8.
    pub attach: i32,
    /// Where the ring sits: metres from the attach point along its bone (word 3; -0.25, -0.1...), and its radius in metres (word 21,
    /// 0.15). The original overrides both per body from `table`.
    pub offset: f32,
    pub radius: f32,
    /// The kind-0 table (word 26): a pair `(offset, radius)` for each breed (1-4), sex (male, female) and fatness (0-2), in that
    /// order of nesting; see [`Helix::fit`].
    pub table: Vec<(f32, f32)>,
    /// How long one pulse of the ring lasts, in seconds (1.5 in every template).
    pub life: f32,
}

impl Helix {
    /// The ring's offset and radius for a body: the table's entry for the breed (1 Solitus, 2 Opifex, 3 Nanomage, 4 Atrox), the sex
    /// (3 female) and the fatness stat (0-2), as `_GfxControlBuffPlaceHolder_t` indexes it (`breed * 12 - 12 + 6 * female + 2 *
    /// fatness` words); the template's own pair when the body is not known or outside the table.
    pub fn fit(&self, breed: i32, sex: i32, fatness: i32) -> (f32, f32) {
        if (1..=4).contains(&breed) && (0..=2).contains(&fatness) {
            let index = ((breed - 1) * 6 + if sex == 3 { 3 } else { 0 } + fatness) as usize;
            if let Some(&pair) = self.table.get(index) {
                return pair;
            }
        }
        (self.offset, self.radius)
    }
}

/// The persistent effect of a running buff (a kind-1001 template): its colour and two counter-rotating spirals of sparks.
#[derive(Clone, Debug, PartialEq)]
pub struct Aura {
    /// The buff's colour at the start and at the end of a spark's life (red, green, blue).
    pub colour: [f32; 3],
    pub end: [f32; 3],
    pub helices: Vec<Helix>,
    /// The seconds between two pulses of the effect (word 20), and how many pulses it plays (word 21: -1 forever).
    pub period: f32,
    pub plays: i32,
}

impl Gfx {
    /// The aura an effect id names: a kind-1001 template (the running effect of 486 programs) itself, or one of the classes that are
    /// built on one: the hit and tracer effects of kinds 2004, 2005, 2006 and 1018 each refer to a spiral and tint it with their own
    /// colour (words 19 to 21 of a 2004: red, green, blue; the tracer 17300 is the gold `0.961 0.753 0.227` one).
    pub fn aura(&self, id: i32) -> Option<Aura> {
        let top = self.get(id)?;
        let (root, tint) = match top.kind {
            1001 => (top, None),
            2004 | 2005 | 2006 | 1018 => {
                let spiral = self.tree(id).into_iter().find(|r| r.kind == 1001)?;
                (spiral, (top.kind == 2004).then(|| [top.float(19), top.float(20), top.float(21)]))
            }
            _ => return None,
        };
        let helices = self
            .references(root.id)
            .into_iter()
            .filter_map(|c| self.get(c).filter(|c| c.kind == 1002))
            .map(|c| Helix { spin: c.float(23), phase: c.float(22), size: c.float(21), count: c.int(20).max(1) as u32, colour: [c.float(13), c.float(14), c.float(15)], material: self.get(c.int(10)).map_or(15, |spark| spark.int(9)), attach: c.int(7), offset: c.float(3), radius: c.float(21).abs().max(0.02), table: self.get(c.int(26)).filter(|t| t.kind == 0).map(|t| t.payload.chunks_exact(2).map(|p| (f32::from_bits(p[0] as u32), f32::from_bits(p[1] as u32))).collect()).unwrap_or_default(), life: if (0.2..10.0).contains(&c.float(8)) { c.float(8) } else { 1.5 } })
            .collect();
        let rgb = |at: usize| tint.unwrap_or([root.float(at), root.float(at + 1), root.float(at + 2)]);
        let period = root.float(20);
        Some(Aura { colour: rgb(13), end: rgb(17), helices, period: if (0.2..120.0).contains(&period) { period } else { 4.0 }, plays: root.int(21) })
    }
}

/// What a nano effect looks like once its template is read: the few shapes the effect classes come to.
#[derive(Clone, Debug, PartialEq)]
pub enum Visual {
    /// Two spirals of sparks (the buff's aura, and the tracers and hits built on it).
    Spiral(Aura),
    /// A soft glow under the feet (kind 3003: the running effect of 114 programs, 52 of them the same cyan one).
    Glow { colour: [f32; 4], size: f32 },
    /// A short bright flash (kind 2001: the hit of 123 programs).
    Flash { colour: [f32; 3], size: f32 },
    /// A glowing ball (kind 2002: the tracer of 250 programs).
    Orb { colour: [f32; 3], size: f32 },
}

impl Gfx {
    /// The look of an effect id. A bundle (kind 2007: up to four children in its first words) or a wrapper (kind 3004: its second word
    /// is a mode and its third the child) shows its first child that has a look.
    pub fn visual(&self, id: i32) -> Option<Visual> {
        let r = self.get(id)?;
        match r.kind {
            1001 | 2004 | 2005 | 2006 | 1018 => self.aura(id).map(Visual::Spiral),
            2007 | 3004 => self.references(id).into_iter().filter(|&c| c != id).find_map(|c| self.visual(c)),
            3003 => {
                // word 10 is a packed colour (alpha, red, green, blue as one integer), word 11 the size of a sprite
                let argb = r.int(10) as u32;
                let channel = |shift: u32| ((argb >> shift) & 0xff) as f32 / 255.0;
                Some(Visual::Glow { colour: [channel(16), channel(8), channel(0), channel(24)], size: r.float(11) })
            }
            2001 => Some(Visual::Flash { colour: [1.0, 0.95, 0.8], size: 0.9 }),
            // (word 10 and the pairs after it are the range each colour channel is drawn from: 0.3 to 1)
            2002 => Some(Visual::Orb { colour: [0.65 + 0.35 * r.float(12), 0.65 + 0.35 * r.float(14), 0.65 + 0.35 * r.float(16)], size: 0.3 }),
            _ => None,
        }
    }
}

/// A particle emitter (kinds 1003, 1005, 1006, 1007 and 1008 share one field layout, which is how these are read: the colours, sizes
/// and angles sit at the same places; what is certain is the colour pair, the angles and the sprite counts, the rest is read by its
/// values, see the module's notes).
#[derive(Clone, Debug, PartialEq)]
pub struct Emitter {
    pub id: i32,
    pub kind: i32,
    /// A colour at the start and at the end of a particle's life: alpha, red, green, blue.
    pub start: [f32; 4],
    pub end: [f32; 4],
    /// A particle's size at the start and at the end of its life, in metres.
    pub size: (f32, f32),
    /// How long a particle lives, in seconds, and how many are made a second.
    pub life: f32,
    pub rate: f32,
    /// The directions particles leave in: the turn round (radians from 0 to this) and the elevation range.
    pub azimuth: f32,
    pub elevation: (f32, f32),
    /// How fast they leave (metres a second, from the first to the second) and the pull on them (up when positive).
    pub speed: (f32, f32),
    pub gravity: f32,
    /// The material (sprite sheet) of its particles.
    pub material: i32,
}

impl Gfx {
    /// The emitters of a template and everything under it (a cast effect has six).
    pub fn emitters(&self, id: i32) -> Vec<Emitter> {
        // a cast template (kind 1010) tints its parts: its colour sits in words 11 to 13 (red, green, blue)
        let tint = self.get(id).filter(|r| r.kind == 1010).map_or([1.0; 3], |r| [r.float(11), r.float(12), r.float(13)]);
        self.tree(id)
            .into_iter()
            .filter(|r| matches!(r.kind, 1003 | 1005 | 1006 | 1007 | 1008) && r.payload.len() >= 31)
            .map(|r| {
                let f = |i: usize| r.float(i);
                let life = if f(35) > 0.05 && f(35) < 20.0 { f(35) } else { 1.0 };
                let counted = r.int(10).max(0) as f32;
                let rate = if counted > 0.5 { counted } else { (r.int(31).max(1) as f32 / life).max(2.0) };
                Emitter {
                    id: r.id,
                    kind: r.kind,
                    start: [f(16), f(17) * tint[0], f(18) * tint[1], f(19) * tint[2]],
                    end: [f(20), f(21) * tint[0], f(22) * tint[1], f(23) * tint[2]],
                    size: (f(12).max(0.0), f(13).max(0.0)),
                    life,
                    rate: rate.min(150.0),
                    azimuth: f(26),
                    elevation: (f(27), f(28)),
                    speed: (f(29), f(30)),
                    gravity: if r.kind == 1005 { f(32) } else { 0.0 },
                    material: r.int(9),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(words: &[i32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn records_are_read_to_the_last_word_and_trees_follow_references() {
        // two records: a parent (kind 1001) naming its child (2004) and a float
        let data = bytes(&[2, 1000, 1001, 3, 5, 2004_0, 0x3f80_0000, 20040, 2004, 2, 5, -1]);
        let gfx = Gfx::parse(&data).unwrap();
        assert_eq!(gfx.records.len(), 2);
        assert_eq!(gfx.get(1000).unwrap().kind, 1001);
        assert_eq!(gfx.get(1000).unwrap().float(2), 1.0);
        assert_eq!(gfx.references(1000), vec![20040]);
        assert_eq!(gfx.tree(1000).iter().map(|r| r.id).collect::<Vec<_>>(), vec![1000, 20040]);
        // a count that outruns the file is an error, not a panic
        assert!(Gfx::parse(&bytes(&[5, 1, 2, 9, 0])).is_err());
    }

    #[test]
    fn a_kind_1001_template_is_an_aura_of_two_helices() {
        let f = |v: f32| v.to_bits() as i32;
        // the blue buff of the game's table, abridged: colour 0.2 0.2 0.99 and two kind-1002 children
        let parent = [5, 0, 0, 0, 0, 0, 0, 0, -1, -1, 20011, 20016, f(1.0), f(0.2), f(0.2), f(0.99), 0, f(0.2), f(0.2), f(0.99), f(4.0), -1];
        let helix = |spin: f32| {
            let mut p = vec![0i32; 27];
            p[13] = f(0.5);
            p[14] = f(0.4);
            p[15] = f(1.0);
            p[20] = 10;
            p[21] = f(0.15);
            p[22] = f(3.14);
            p[23] = f(spin);
            p
        };
        let mut words = vec![3, 1000, 1001, parent.len() as i32];
        words.extend_from_slice(&parent);
        for (id, spin) in [(20011, -12.566f32), (20016, 12.566)] {
            let h = helix(spin);
            words.extend_from_slice(&[id, 1002, h.len() as i32]);
            words.extend_from_slice(&h);
        }
        let gfx = Gfx::parse(&bytes(&words)).unwrap();
        let aura = gfx.aura(1000).unwrap();
        assert_eq!(aura.colour, [0.2, 0.2, 0.99]);
        assert_eq!(aura.helices.len(), 2);
        assert_eq!(aura.helices[0].spin, -12.566);
        assert_eq!(aura.helices[1].spin, 12.566);
        assert_eq!(aura.helices[0].count, 10);
        assert_eq!(aura.helices[0].colour, [0.5, 0.4, 1.0]);
        assert_eq!((aura.period, aura.plays, aura.helices[0].attach, aura.helices[0].offset, aura.helices[0].radius, aura.helices[0].life), (4.0, -1, 0, 0.0, 0.15, 1.5));
        assert_eq!(aura.helices[0].fit(1, 2, 1), (0.0, 0.15));
        assert!(gfx.aura(20011).is_none());
    }
}
