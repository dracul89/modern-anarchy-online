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
    /// that happen to be ids are skipped (a flag of 5 or a count of 8 is not a reference), so only ids of 100 and up count. So are
    /// the words that hold numbers which are not references but coincide with ids: word 7, the attach point of every control
    /// (`InitDynelTemplate` / FUN_10106be2 read it; 1000 the pelvis is also template 1000's id, 2000–2009 the right hand and the
    /// other attractors those of ten 1004–1006 parts), except in a bundle (2007), whose children are its first words; word 30 of
    /// a Stars template (its life in milliseconds: 1000, 2000, 3000); and the Deformer's bone list from word 14 (attach points).
    /// Read as references, these made every effect at the pelvis look as if it carried the buff spiral of template 1000.
    pub fn references(&self, id: i32) -> Vec<i32> {
        let Some(r) = self.get(id) else { return Vec::new() };
        let not_a_reference = |at: usize| (at == 7 && r.kind != 2007) || (at == 30 && r.kind == 2004) || (at >= 14 && r.kind == 3001);
        r.payload.iter().copied().enumerate().filter(|&(at, w)| !not_a_reference(at) && w >= 100 && w != id && self.by_id.contains_key(&w)).map(|(_, w)| w).collect()
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
    /// The cord the placeholder lays (word 11, kind 1003, `_GfxControlCord_t`: FUN_100d60d5 reads it): its links' width (word
    /// 12), life (word 35), the cord's duration (word 8) and material (word 9). Every placeholder's cord is the same: 0.05,
    /// 0.5 s, 1.5 s and light_halo2 (8).
    pub ribbon_width: f32,
    pub ribbon_life: f32,
    pub ribbon_duration: f32,
    pub ribbon_material: i32,
    /// The rate (word 10) of the flare the placeholder carries (word 10, kind 1005, `_GfxControlFlare_t`): sprites a second; 0 in
    /// every placeholder's flare, so the ring has no sparkles.
    pub sparkle_rate: f32,
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
            .map(|c| {
                let cord = self.get(c.int(11)).filter(|r| r.kind == 1003 && r.payload.len() >= 36);
                Helix {
                    spin: c.float(23),
                    phase: c.float(22),
                    size: c.float(21),
                    count: c.int(20).max(1) as u32,
                    colour: [c.float(13), c.float(14), c.float(15)],
                    material: self.get(c.int(10)).map_or(15, |spark| spark.int(9)),
                    attach: c.int(7),
                    offset: c.float(3),
                    radius: c.float(21).abs().max(0.02),
                    table: self.get(c.int(26)).filter(|t| t.kind == 0).map(|t| t.payload.chunks_exact(2).map(|p| (f32::from_bits(p[0] as u32), f32::from_bits(p[1] as u32))).collect()).unwrap_or_default(),
                    life: if (0.2..10.0).contains(&c.float(8)) { c.float(8) } else { 1.5 },
                    ribbon_width: cord.map_or(0.05, |r| r.float(12)),
                    ribbon_life: cord.map_or(0.5, |r| r.float(35)),
                    ribbon_duration: cord.map_or(1.5, |r| r.float(8)),
                    ribbon_material: cord.map_or(8, |r| r.int(9)),
                    sparkle_rate: self.get(c.int(10)).filter(|r| r.kind == 1005).map_or(0.0, |r| r.float(10)),
                }
            })
            .collect();
        let rgb = |at: usize| tint.unwrap_or([root.float(at), root.float(at + 1), root.float(at + 2)]);
        let period = root.float(20);
        Some(Aura { colour: rgb(13), end: rgb(17), helices, period: if (0.2..120.0).contains(&period) { period } else { 4.0 }, plays: root.int(21) })
    }
}

/// A kind-2004 template as `_GfxControlStars_t` reads it (Gamecode FUN_100f72a9, its run function FUN_100f8491 and start
/// FUN_100f7fb8): 128 billboard sprites (`GfxVisualDiaBill`) of the material word 9, each a random colour between the two
/// colours of words 18–21 and 22–25 (alpha, red, green, blue: FUN_10108663 picks each channel at random between them), a random
/// frame of the material's last sixteen, sized by word 28 along a parabola over its life (word 30, ms), born at the attach
/// point (word 7) within a radius (word 29). Word 10 is the mode, of which 29 exist; the two most used are here:
/// * 6 (204 templates): up to two stars a frame, born on a circle of the radius, rising at half a unit a second and swirling
///   (their velocity across the radius, scaled by word 31), their size `(t + 0.2) × (1 − t²) × word 28`;
/// * 19 (173 templates): up to fifteen stars a frame, born in a sphere of the radius at the hit location and standing there,
///   their size `(1 − (1 − 2t)²) × word 28`, each living a random share of the life.
/// Word 8 (−1: until the effect stops) is the duration; once stopped no more stars are born and the effect ends with the last.
#[derive(Clone, Debug, PartialEq)]
pub struct Stars {
    pub mode: i32,
    /// Alpha, red, green, blue.
    pub colour_a: [f32; 4],
    pub colour_b: [f32; 4],
    pub size: f32,
    pub radius: f32,
    /// Seconds.
    pub life: f32,
    /// Word 31 (the swirl of mode 6).
    pub swirl: f32,
    pub material: i32,
    pub attach: i32,
    /// Word 8, seconds (−1 until stopped).
    pub duration: f32,
    /// Word 11 (mode 10 adds it to the size) and word 30 as an integer (the ring modes' reach, mode 10's channel mask switch).
    pub word11: f32,
    pub word30: i32,
}

/// A kind-1018 template as `_GfxControlSparks_t` reads it (Gamecode FUN_100f1758; FUN_100f1bbe makes a spark, FUN_100f24c8 runs
/// them through `GfxVisualSprite2Type0`, `dropin/displaysystem/src/gfxsprite2.rs`): a burst of sprites of the material word
/// 9 thrown from the attach point (word 7). Each spark's direction is an azimuth between words 25 and 26 and an elevation
/// between 27 and 28, its speed between words 29 and 30 scaled by word 32; it lives between words 34 and 35 seconds, its width
/// runs from word 12 to 13 and its height from 14 to 15, its colour from words 16–19 to 20–23 (alpha, red, green, blue), its
/// frame across the material's sheet, and word 40 (−9.8) pulls it down. The control throws word 31 sparks at once and then
/// up to fifteen more over the frames the gate (word 24, a mask on `rand()`) lets through, so a hit is one small burst.
#[derive(Clone, Debug, PartialEq)]
pub struct Sparks {
    pub colour_a: [f32; 4],
    pub colour_b: [f32; 4],
    pub width: (f32, f32),
    pub height: (f32, f32),
    pub azimuth: (f32, f32),
    pub elevation: (f32, f32),
    pub speed: (f32, f32),
    pub scale: f32,
    pub life: (f32, f32),
    pub count: i32,
    pub gravity: f32,
    pub material: i32,
    pub attach: i32,
    /// Word 24: the burst's gate (0 lets every frame through, 15 one in sixteen).
    pub gate: i32,
    /// Word 38: the sparks' push kind (`GfxVisualSprite2Type0::ProcessSprites`: 0 none, 4 the wind times `(t² + t) / 2` of
    /// their height's fraction `t` between words 36 and 37 above their birth; 28 templates 0, 14 kind 4), and word 39 the
    /// strength (each spark `0.1 + 0.9 × random` of it).
    pub push: i32,
    pub push_heights: (f32, f32),
    pub push_strength: f32,
}

/// A kind-3001 template as `_GfxControlDeformer_t` reads it (Gamecode FUN_100d823f; FUN_100d7caa runs it, FUN_100d776f is the
/// vertex callback it registers on the target's `CATRender_t`): the target's mesh itself is bent. Word 8 is the duration, 10
/// the mode, 11 the strength, 12 the seconds the strength ramps in over and 13 the seconds it ramps out over once stopped
/// (`Stop`, slot 6, at `duration − word 13`). Mode 1 (the three hit templates: 1.1 or 3.1 s, strength 1, in 0.1 s, out 1–3
/// s) ripples every vertex by `amplitude × sin²(frequency × time + the vertex's phase) × strength`, words 15 and 14. Mode 0
/// pulses the vertices round named bones (a list of attach points, frequencies and radii) and mode 4 fades a second body
/// mesh over the character; neither is ported.
#[derive(Clone, Debug, PartialEq)]
pub struct Deform {
    pub duration: f32,
    pub strength: f32,
    pub ramp_in: f32,
    pub ramp_out: f32,
    pub frequency: f32,
    pub amplitude: f32,
}

impl Deform {
    /// The strength at `age` seconds: ramped in over `ramp_in`, out over `ramp_out` before the duration's end (FUN_100d7caa's
    /// tail: `strength × age / ramp_in` while in, then `(1 − (age − stop) / ramp_out) × strength`); 0 once over.
    pub fn strength_at(&self, age: f32) -> f32 {
        if age < 0.0 || (self.duration > 0.0 && age >= self.duration) {
            return 0.0;
        }
        let stop = if self.duration > 0.0 { self.duration - self.ramp_out } else { f32::MAX };
        if age >= stop {
            (1.0 - (age - stop) / self.ramp_out.max(0.001)).max(0.0) * self.strength
        } else if age < self.ramp_in {
            self.strength * age / self.ramp_in.max(0.001)
        } else {
            self.strength
        }
    }
}

/// A kind-1024 template as `_GfxControlTracer4_t` reads it (made by `_EffectHandler_t::CreateGfxControlTracer` FUN_100d176c
/// from the caster's and the target's points; FUN_100ffcac reads the words, FUN_100ffd67 makes three `GfxVisualCord4`s of
/// twenty links, FUN_100ffa7b winds them each frame): three cords spiralling round the straight line between the two points.
/// Word 8 is the duration (1 s), 9 the material (15, `s_bullet`), 10 the locator's speed (100), 11 the links' width, 12 the
/// winding's radius, 13–16 the colour (alpha, red, green, blue). Along the line a link `k` (0–19) sits at `k × 0.05 × 1.1` of
/// its length, out from it by `(sin(k × 0.05 × π × 20 / 19) + 0.2) × radius` at a phase that spins at 12.56 radians a second
/// and steps 0.314 a link (the step's sign flipped at random), each cord a third of a turn from the next; a quarter of the
/// links jitter by half the radius.
#[derive(Clone, Debug, PartialEq)]
pub struct Cords {
    pub width: f32,
    pub radius: f32,
    /// Alpha, red, green, blue.
    pub colour: [f32; 4],
    pub material: i32,
    pub duration: f32,
}

/// A kind-3038 template as `GfxControlVolGrid_t` reads it (FUN_10116562 the constructor, FUN_10115f8c the words, FUN_101166f2
/// each curve, FUN_10116292 the run, FUN_10115eb5 makes displaysystem's `GfxVisualVolGrid` at the locator's point): a volume
/// drawn as stacks of textured slices, `across` of them across x, `up` of them up and `deep` across z, inside a box whose
/// height, width and depth follow curves over the duration (word 8), coloured at the middle and the top by two colour curves.
/// Word 0 is the flags (0x800 follows the locator every frame, 0x1000 starts at a random turn, 0x2000 drops to the ground,
/// 0x400 turns the texture, 0x4000 fades each slice by how squarely it faces the camera), 9 the material, 13 the spin in radians
/// a second, 14–16 the slice counts; from word 17 the five curves, each a count then `(time, value)` pairs with the time as a
/// fraction of the duration — the height, the width, the depth, then the middle and the top colour (packed alpha, red, green,
/// blue). Words 10–12 are kept (4 and 1.0 in every template) but the run never reads them.
#[derive(Clone, Debug, PartialEq)]
pub struct VolGrid {
    pub flags: u32,
    pub duration: f32,
    pub material: i32,
    pub across: u32,
    pub up: u32,
    pub deep: u32,
    pub spin: f32,
    pub height: Vec<(f32, f32)>,
    pub width: Vec<(f32, f32)>,
    pub depth: Vec<(f32, f32)>,
    pub middle: Vec<(f32, u32)>,
    pub top: Vec<(f32, u32)>,
}

impl VolGrid {
    /// Reads the record's words (none when the curves run past the payload).
    pub fn read(r: &Record) -> Option<VolGrid> {
        let mut at = 17;
        let mut curve = |colour: bool| -> Option<Vec<(f32, u32)>> {
            let count = usize::try_from(*r.payload.get(at)?).ok()?;
            at += 1;
            let mut keys = Vec::with_capacity(count);
            for _ in 0..count {
                let (t, v) = (*r.payload.get(at)?, *r.payload.get(at + 1)?);
                at += 2;
                keys.push((f32::from_bits(t as u32), if colour { v as u32 } else { v as u32 }));
            }
            Some(keys)
        };
        let float = |keys: Vec<(f32, u32)>| keys.into_iter().map(|(t, v)| (t, f32::from_bits(v))).collect::<Vec<_>>();
        let (height, width, depth) = (float(curve(false)?), float(curve(false)?), float(curve(false)?));
        let (middle, top) = (curve(true)?, curve(true)?);
        Some(VolGrid { flags: r.int(0) as u32, duration: r.float(8), material: r.int(9), across: r.int(14).max(0) as u32, up: r.int(15).max(0) as u32, deep: r.int(16).max(0) as u32, spin: r.float(13), height, width, depth, middle, top })
    }
}

/// A curve of `(time, value)` keys sampled at `t` as FUN_1011689d does: linear between the key at or before `t` and the next,
/// the last key's value past the last key (and before the first), 1 with no keys.
pub fn curve(keys: &[(f32, f32)], t: f32) -> f32 {
    let Some(&(_, last)) = keys.last() else { return 1.0 };
    match keys.windows(2).find(|w| w[0].0 <= t && t < w[1].0) {
        Some(w) => {
            let span = if w[1].0 - w[0].0 == 0.0 { 1.0 } else { w[1].0 - w[0].0 };
            let u = (t - w[0].0) / span;
            w[1].1 * u + (1.0 - u) * w[0].1
        }
        None => last,
    }
}

/// A curve of `(time, packed colour)` keys sampled at `t` as FUN_1011678b does (the same search, the colours blended channel by
/// channel through `Color_t::Interpolate`), white with no keys.
pub fn colour_curve(keys: &[(f32, u32)], t: f32) -> u32 {
    let Some(&(_, last)) = keys.last() else { return u32::MAX };
    match keys.windows(2).find(|w| w[0].0 <= t && t < w[1].0) {
        Some(w) => {
            let span = if w[1].0 - w[0].0 == 0.0 { 1.0 } else { w[1].0 - w[0].0 };
            let u = (t - w[0].0) / span;
            let mut out = 0u32;
            for shift in [0, 8, 16, 24] {
                let (a, b) = (((w[0].1 >> shift) & 0xff) as f32, ((w[1].1 >> shift) & 0xff) as f32);
                out |= ((a + (b - a) * u).round().clamp(0.0, 255.0) as u32) << shift;
            }
            out
        }
        None => last,
    }
}

/// A packed colour (alpha, red, green, blue) as red, green, blue, alpha in 0..1.
pub fn unpack_argb(c: u32) -> [f32; 4] {
    let channel = |shift: u32| ((c >> shift) & 0xff) as f32 / 255.0;
    [channel(16), channel(8), channel(0), channel(24)]
}

/// A kind-1009 template as `_GfxControlSmoke_t` reads it (FUN_100f12bf the constructor, FUN_100f0b6b the words, FUN_100f0cd4
/// makes a `GfxVisualSprite2Type0` of the material, FUN_100f04bd runs it): a steady stream of sprites. Words 1–3 are the
/// emitter's offset from the attach point (word 7) and 4–6 its turn about x, y and z (FUN_10106be2), word 8 the duration (below
/// zero: for ever), 9 the material, 10 the rate (sprites a second: the run makes sprites until `rate × elapsed` of them exist,
/// while `elapsed < duration − life`). Each sprite is born in the emitter's frame at `(cos az × cos el × r, y, sin az × cos el × r)`
/// with `az` a random turn, `el` a random quarter turn, `r` between words 11 and 12 and `y` between words 31 and 32; its
/// velocity is `(0.3 × x, 0.3 × z, −1) × word 13 / life` (the original's own component order), so the stream drifts along the
/// emitter's −z, which the turn of word 4 (π/2 in most templates) points up. It lives word 26 seconds times `1 + 0.3 × random`,
/// its width runs from word 14 to 15 and its height from 16 to 17, both scaled by `1 + 0.3 × random`, its colour from words
/// 18–21 to 22–25 (alpha, red, green, blue), its frame across the material's sheet. Word 29 is the sprite's push kind
/// (`GfxVisualSprite2Type0::ProcessSprites`: 5 rises 1.5 a second, 7 flies at a random direction, the others ride the effect
/// handler's force), words 27, 28 and 30 that kind's heights and strength (the last times `0.1 + 0.9 × random`).
#[derive(Clone, Debug, PartialEq)]
pub struct Smoke {
    pub flags: u32,
    pub attach: i32,
    pub offset: [f32; 3],
    pub turn: [f32; 3],
    pub duration: f32,
    pub material: i32,
    pub rate: f32,
    pub radius: (f32, f32),
    pub drift: f32,
    pub width: (f32, f32),
    pub height: (f32, f32),
    pub colour_a: [f32; 4],
    pub colour_b: [f32; 4],
    pub life: f32,
    pub rise: (f32, f32),
    pub kind: i32,
    pub heights: (f32, f32),
    pub strength: f32,
}

impl Smoke {
    pub fn read(r: &Record) -> Option<Smoke> {
        if r.payload.len() < 33 {
            return None;
        }
        Some(Smoke {
            flags: r.int(0) as u32,
            attach: r.int(7),
            offset: [r.float(1), r.float(2), r.float(3)],
            turn: [r.float(4), r.float(5), r.float(6)],
            duration: r.float(8),
            material: r.int(9),
            rate: r.float(10),
            radius: (r.float(11), r.float(12)),
            drift: r.float(13),
            width: (r.float(14), r.float(15)),
            height: (r.float(16), r.float(17)),
            colour_a: [r.float(18), r.float(19), r.float(20), r.float(21)],
            colour_b: [r.float(22), r.float(23), r.float(24), r.float(25)],
            life: r.float(26),
            rise: (r.float(31), r.float(32)),
            kind: r.int(29),
            heights: (r.float(27), r.float(28)),
            strength: r.float(30),
        })
    }
}

/// A kind-1012 template as `_GfxControlSprite_t` reads it (FUN_100f6b86 the constructor, FUN_100f6369 the words, FUN_100f6483
/// picks the visual, FUN_100f64a7 makes it, FUN_100f6d2b runs it): one quad of the material (word 9) at the attach point (word 7)
/// plus the offset of words 1–3, cycling over the period of word 23 (0 counts as 1 s) for word 24 cycles (−1: for ever) when
/// flag 0x40 of word 10 is set, else until the effect ends (word 8). Word 10's low two bits pick the visual: 0 a
/// `GfxVisualSprite2` (a billboard), 3 a `GfxVisualSprite3` (a quad standing in the visual's frame, turned to face the camera
/// about the up axis with flag 0x2000, or set from the locator's turn with flags 0x8 and 0x10), anything else nothing. Flag 0x10
/// follows the locator every frame; 0x20 runs the sheet's frame from the first to the last over the cycle; 0x80 blends the colour
/// from words 15–18 to 19–22 (alpha, red, green, blue) over the cycle; 0x100 runs the width from word 11 to 12 and the height
/// from 13 to 14, and with 0x800 adds `word 25 × sin(2π × word 26 × elapsed)` to both; 0x4 clear samples a random spot of the
/// texture each frame instead of its frame; 0x1000 clear adds the quad to the scene, set blends it.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub attach: i32,
    pub offset: [f32; 3],
    pub duration: f32,
    pub material: i32,
    pub flags: u32,
    pub width: (f32, f32),
    pub height: (f32, f32),
    pub colour_a: [f32; 4],
    pub colour_b: [f32; 4],
    pub period: f32,
    pub cycles: i32,
    pub pulse: f32,
    pub pulse_rate: f32,
}

impl Sprite {
    pub fn read(r: &Record) -> Option<Sprite> {
        if r.payload.len() < 27 || r.int(10) & 3 != 0 && r.int(10) & 3 != 3 {
            return None;
        }
        Some(Sprite {
            attach: r.int(7),
            offset: [r.float(1), r.float(2), r.float(3)],
            duration: r.float(8),
            material: r.int(9),
            flags: r.int(10) as u32,
            width: (r.float(11), r.float(12)),
            height: (r.float(13), r.float(14)),
            colour_a: [r.float(15), r.float(16), r.float(17), r.float(18)],
            colour_b: [r.float(19), r.float(20), r.float(21), r.float(22)],
            period: if r.float(23) == 0.0 { 1.0 } else { r.float(23) },
            cycles: r.int(24),
            pulse: r.float(25),
            pulse_rate: r.float(26),
        })
    }

    /// Whether the quad stands in the world (`GfxVisualSprite3`) rather than facing the camera.
    pub fn standing(&self) -> bool {
        self.flags & 3 == 3
    }
}

/// A kind-3028 template as `GfxControlBParticle2_t` reads it (FUN_1010c5bd the constructor, FUN_1010b196 the words, FUN_1010bf63
/// makes the particles and a `GfxVisualBParticle2` each, FUN_1010b499 runs them): a pool of word 11 billboard particles of
/// the material (word 10) thrown from the locator's point. Each is born within ±word 13 of the point, with a velocity drawn
/// from words 17–18 (x), 19–20 (y) and 21–22 (z) in the locator's frame, a spin from words 23–24 (degrees a second), a life
/// from words 25–26, a size from 27–28; word 16 pulls it along y (gravity), word 14 damps its velocity (`v += v × (word 14 − 1)
/// × 100 × dt`), word 30 grows its size the same way, word 29 is the bounce off the ground under flag 0x8000, word 31 the
/// frame rate across the sheet under the frame mode of word 32 (1 wraps, 2 stops at the last, 3 bounces), word 33 the width
/// over the height. From word 35 a colour curve (count, then `(fraction of the life, colour)`) colours and fades it. Word 9 is
/// the mode: 0 one burst that ends when its particles are dead, 1 a stream that reborns dead ones every word 12 seconds, up
/// to word 15 a frame, while `elapsed < duration − longest life`. Flags (word 0): 0x100 flips the sprite, 0x200 adds it, 0x400
/// drops the point to the ground, 0x4000 no random start angle, 0x20000 a random start frame, 0x100000 keeps the particles at
/// the point, 0x1000000 scales by the body.
#[derive(Clone, Debug, PartialEq)]
pub struct BParticles {
    pub flags: u32,
    pub attach: i32,
    pub duration: f32,
    pub mode: i32,
    pub material: i32,
    pub count: i32,
    pub interval: f32,
    pub scatter: f32,
    pub damping: f32,
    pub per_frame: i32,
    pub gravity: f32,
    pub velocity: [(f32, f32); 3],
    /// Radians a second.
    pub spin: (f32, f32),
    pub life: (f32, f32),
    pub size: (f32, f32),
    pub bounce: f32,
    pub growth: f32,
    pub frame_rate: f32,
    pub frame_mode: i32,
    pub aspect: f32,
    pub colours: Vec<(f32, u32)>,
}

impl BParticles {
    pub fn read(r: &Record) -> Option<BParticles> {
        if r.payload.len() < 36 {
            return None;
        }
        let count = usize::try_from(r.int(35)).ok()?;
        let mut colours = Vec::with_capacity(count);
        for k in 0..count {
            colours.push((f32::from_bits(*r.payload.get(36 + 2 * k)? as u32), *r.payload.get(37 + 2 * k)? as u32));
        }
        let rad = std::f32::consts::PI / 180.0;
        Some(BParticles {
            flags: r.int(0) as u32,
            attach: r.int(7),
            duration: r.float(8),
            mode: r.int(9),
            material: r.int(10),
            count: r.int(11),
            interval: r.float(12),
            scatter: r.float(13),
            damping: r.float(14),
            per_frame: r.int(15),
            gravity: r.float(16),
            velocity: [(r.float(17), r.float(18)), (r.float(19), r.float(20)), (r.float(21), r.float(22))],
            spin: (r.float(23) * rad, r.float(24) * rad),
            life: (r.float(25), r.float(26)),
            size: (r.float(27), r.float(28)),
            bounce: r.float(29),
            growth: r.float(30),
            frame_rate: r.float(31),
            frame_mode: r.int(32),
            aspect: if r.float(33) == 0.0 { 1.0 } else { r.float(33) },
            colours,
        })
    }
}

/// A kind-1005 template as `_GfxControlFlare_t` reads it (FUN_100dcedb; FUN_100dd2eb makes a particle, FUN_100ddc6d runs them,
/// displaysystem's `GfxVisualFlareType0` (`gfxflare.rs`) draws each as a streak from its head to its tail): word 8 the duration
/// (below 0: for ever), 9 the material, 10 the rate (a run makes particles until `rate × elapsed` of them exist, while
/// `elapsed < duration − longest life`, through the gate of word 24, a mask on `rand()`), 12–13 the streak's width from birth to
/// death, 16–19 and 20–23 its colour (alpha, red, green, blue) from birth to death (a Spell1 sets both from its own), 25–26,
/// 27–28 and 29–30 the ranges of its direction's two angles and its speed, 32 and 33 how far the head and the tail travel along
/// that velocity over the life (the head −0.3, the tail 1: the streak grows backwards from the hand), 34–35 the life's range.
#[derive(Clone, Debug, PartialEq)]
pub struct Flare {
    pub duration: f32,
    pub material: i32,
    pub rate: f32,
    pub width: (f32, f32),
    pub colour_a: [f32; 4],
    pub colour_b: [f32; 4],
    pub gate: i32,
    pub azimuth: (f32, f32),
    pub elevation: (f32, f32),
    pub speed: (f32, f32),
    pub head: f32,
    pub tail: f32,
    pub life: (f32, f32),
    pub attach: i32,
}

impl Flare {
    pub fn read(r: &Record) -> Option<Flare> {
        if r.kind != 1005 || r.payload.len() < 36 {
            return None;
        }
        Some(Flare {
            duration: r.float(8),
            material: r.int(9),
            rate: r.float(10),
            width: (r.float(12), r.float(13)),
            colour_a: [r.float(16), r.float(17), r.float(18), r.float(19)],
            colour_b: [r.float(20), r.float(21), r.float(22), r.float(23)],
            gate: r.int(24),
            azimuth: (r.float(25), r.float(26)),
            elevation: (r.float(27), r.float(28)),
            speed: (r.float(29), r.float(30)),
            head: r.float(32),
            tail: r.float(33),
            life: (r.float(34), r.float(35)),
            attach: r.int(7),
        })
    }
}

/// A kind-1010 template, the cast effect, as `_GfxControlSpell1_t` reads it (made by the point-and-number `CreateGfxControl`,
/// FUN_100d15d0 / FUN_100f331d, from `CreateEffect2`; FUN_100f2789 the words, FUN_100f3f59 the run, FUN_100f3f30 its phases):
/// three locators, the left hand (attractor 2001), the right hand (2000) and the target's point, and its words: 9 the material,
/// 10–13 and 14–17 the two colours (alpha, red, green, blue) it gives its parts, 18–26 the phases' times, 27–28 the flares at
/// the left and right hand (kind 1005), 29–30 cords there (kind 1003: made at a point, their set-position moves their own
/// locator (FUN_100d69d8) instead of laying a link, so they draw nothing), 31 the projectile (kind 1007, `_GfxControlNano0_t`),
/// 32 the impact on the target (kind 1008 `_GfxControlNano1_t`, or a 1005), 33 whether phases 3 and 4 are off.
/// * Phase 1 (FUN_100f3022: words 18–20): the four hand parts made at word 18, moved to the hands and run every frame, stopped
///   at word 19 (no more particles), deleted after word 20;
/// * phase 2 (FUN_100f3ee7, FUN_100f3d22 / FUN_100f3205: words 21–22): a line of quads of the material between the hands in the
///   first colour, growing from both hands to the middle in its first half and shrinking onto the middle in its second;
/// * phases 3 and 4 (word 33 = 0 only): the projectile from the hands' midpoint to the target and the impact there; `NextState`
///   (the release of the cast, FUN_100f2617) retimes phases 1–3 to it.
/// 178 of the 180 templates have word 33 set, phase 1 at 0–6 s (deleted at 8) and phase 2 at 4–5 s; the two with the projectile
/// (9000, 9002) are named by no program.
#[derive(Clone, Debug, PartialEq)]
pub struct Cast {
    pub material: i32,
    pub colour_a: [f32; 4],
    pub colour_b: [f32; 4],
    /// Words 18 to 26.
    pub times: [f32; 9],
    /// The left hand's and the right hand's flares.
    pub flares: [Option<Flare>; 2],
    pub released: bool,
}

/// A kind-2005 template's words as `_GfxControlSuns_t` reads them (FUN_100fd14e); see [`Visual::Suns`].
#[derive(Clone, Debug, PartialEq)]
pub struct Suns {
    pub mode: i32,
    pub attach: i32,
    pub material: i32,
    /// Alpha, red, green, blue.
    pub colour_a: [f32; 4],
    pub colour_b: [f32; 4],
    pub duration: f32,
    /// Words 28 and 29: the size (mode 0 its first sun's) and mode 0's step of size a sun (mode 4's frame scale).
    pub size: f32,
    pub step: f32,
    /// Word 30 in seconds: mode 4's life.
    pub life: f32,
}

/// A kind-3003 template as `_GfxControlShield_t` reads it (FUN_100ee24e; FUN_100edeed the words, FUN_100ee1d2 / FUN_100edf78
/// the visual on the character's CAT mesh, FUN_100edcf4 the run) and `GfxVisualShield` draws it (displaysystem `gfxshield.rs`,
/// FUN_1001c94f): the character's skinned mesh again, every vertex pushed out along its normal by word 11, textured with the
/// material (word 9) mapped by word 18 (0: `(uv + speed × t) × scale`; 1: round the up axis; 2: `position × scale + speed × t`)
/// with the scales of words 19–20 and the speeds of words 24–25, coloured by word 10 (alpha, red, green, blue packed), its alpha
/// `sin²(clamp(word 21 × t, 0, word 23))` (with flag 0x800 a front from the point of words 12–14 at word 22 a unit of
/// distance, along the axis of words 15–17 with flag 0x1000), shimmering by `sin²(10(x + y + z) + 3t)` with flag 0x2000, added
/// with flag 0x400 or blended. The run: the time loops past word 27 (word 26: 1 wrapping to word 28, 2 back and forth); the
/// shield stops at its duration (word 8) less word 30 and ends word 30 after (word 29: 1 fading the alpha over it, 2 the time
/// running back, 3 the time counting down from word 31).
#[derive(Clone, Debug, PartialEq)]
pub struct Shield {
    pub flags: u32,
    pub duration: f32,
    pub material: i32,
    pub colour: u32,
    pub push: f32,
    pub point: [f32; 3],
    pub axis: [f32; 3],
    pub mapping: i32,
    pub scale: (f32, f32),
    pub fade: f32,
    pub distance: f32,
    pub most: f32,
    pub speed: (f32, f32),
    pub looping: i32,
    pub loop_range: (f32, f32),
    pub ending: i32,
    pub end_time: f32,
    pub end_from: f32,
}

impl Shield {
    pub fn read(r: &Record) -> Option<Shield> {
        if r.payload.len() < 32 {
            return None;
        }
        Some(Shield {
            flags: r.int(0) as u32,
            duration: r.float(8),
            material: r.int(9),
            colour: r.int(10) as u32,
            push: r.float(11),
            point: [r.float(12), r.float(13), r.float(14)],
            axis: [r.float(15), r.float(16), r.float(17)],
            mapping: r.int(18),
            scale: (r.float(19), r.float(20)),
            fade: r.float(21),
            distance: r.float(22),
            most: r.float(23),
            speed: (r.float(24), r.float(25)),
            looping: r.int(26),
            loop_range: (r.float(27), r.float(28)),
            ending: r.int(29),
            end_time: r.float(30),
            end_from: r.float(31),
        })
    }

    /// The visual's time and alpha factor `t` seconds in (FUN_100edcf4), or none once it has ended.
    pub fn time(&self, t: f32) -> Option<(f32, f32)> {
        let (mut time, mut factor) = (t, 1.0);
        if self.duration > 0.0 {
            let stop = self.duration - self.end_time;
            if t > stop {
                let since = t - stop;
                if since > self.end_time {
                    return None;
                }
                match self.ending {
                    1 => factor = 1.0 - since / self.end_time.max(1e-6),
                    2 => time = stop - since,
                    3 => time = self.end_from - since,
                    _ => {}
                }
            }
        }
        let (from, to) = self.loop_range;
        if time > from && to > from {
            let span = to - from;
            match self.looping {
                1 => time = from + (time - from) % span,
                2 => time = from + (span - (time - from) % (2.0 * span)).abs(),
                _ => {}
            }
        }
        Some((time, factor))
    }
}

/// The Stars modes the client plays as the original does; the others fall back to the spiral child:
/// * 3: born in a sphere of the radius, swirling across it (no rise), two a frame, the parabolic size of mode 6;
/// * 4: born at the point, thrown in a random direction at the radius a second, slowed by 5% a frame and pulled down by 0.1,
///   the size shrinking from twice word 28, frames from the whole sheet;
/// * 7 and 8: all 128 stars on a ring that swells `word 31` times over the duration (its radius `√frac × 2 × word 30`, their
///   size `word 28 × √frac + word 29`);
/// * 10: five a frame on a sphere of the radius, standing, living word 28 seconds, sized `(1 − (2t − 1)²) × 2 × word 31 +
///   word 11`, each a random alpha and (word 30 set) a channel of the colour;
/// * 11: as 6;
/// * 5: as 6 but born 1.2 below the point, flying outward at the radius a second and pulled back toward the axis by word 31
///   hundredths of their place a frame (no rise);
/// * 20: fifteen a frame born in a sphere of the radius about the hit location's point (`_GfxHitLocation_t`'s two points,
///   which are the same point), standing, at the constant size word 28, living word 30 ms;
/// * 2: all 128 at once on random points of the unit sphere for 1.5 s (the start function's duration), the sphere grown to
///   `√frac × 0.7 + 0.6`, the stars sized `(1 − frac) × word 28 × 20 / table[frame]` (the table at 0x102c5fa8) with the frame
///   `(1 − frac) × 15` (odd stars one behind), coloured between the second colour and a palette of eight (0x102c5ee8);
/// * 13: a ray of all 128 curling out from the point: a direction turned word 29 radians a step about a drifting axis, seven
///   steps a frame into a ring of 128, each star on its slot's direction at `word 28 × age / 128`, shown while `age / 128 <
///   1 − (2 frac − 1)²`, at the size word 11;
/// * 15: up to ten a frame thrown from 30 of the target's skinned vertices along their normals at word 29, slowed to 0.96
///   a frame and moving a tenth of their velocity, living word 30 ms, sized `(1.1 − age / life) × word 28`;
/// * 1: two a frame at rest on the unit sphere, pulled onto the point by a damped spring (`a = −p − 2v`, a tenth a frame)
///   over 1.2 s, sized from the frame table at 0x102c5f68 (the size word is 0);
/// * 9: 64 of them, two a frame on a level disc of the radius, falling from 2 m at word 28 a second for word 30 ms, sized
///   word 11 plus `h × word 31 / 100` while the height term `h = 0.5 − age × word 28` is positive;
/// * 17 (the three templates named by 4008 programs): fifteen a frame on a spoke half a metre long across x, at the angle
///   `word 29 − 4 × birth time`, standing, sized `word 28 × (1 − (1 − 2a)²)`: the three phases make a turning pinwheel;
/// * 16 and 18: fifteen and ten a frame standing in a sphere of word 29, sized as 17;
/// * 22: word 31 a frame born on the body's six edges (between the attach points of the table at 0x102c5fe8: calves to
///   thighs, pelvis to head, forearms to upper arms, the upper arms to each other), word 28 out (the field the other modes
///   size by), springing back through the edge and dying as they cross it or after word 30 ms, sized `(age / life × 0.7 +
///   0.3) × word 29` (the other modes' radius).
pub const STARS_MODES: [i32; 19] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 16, 17, 18, 19, 20, 22];

/// What a nano effect looks like once its template is read: the few shapes the effect classes come to.
#[derive(Clone, Debug, PartialEq)]
pub enum Visual {
    /// The star burst of a hit or a tracer (kind 2004 in a mode the client plays).
    Stars(Stars),
    /// The sparks thrown by a hit (kind 1018).
    Sparks(Sparks),
    /// A hit that ripples the target's mesh (kind 3001, mode 1): no sprites of its own.
    Deform(Deform),
    /// A tracer of three cords wound round the line of flight (kind 1024).
    Cords(Cords),
    /// A volume of textured slices whose box and colours follow curves (kind 3038).
    VolGrid(VolGrid),
    /// A steady stream of sprites drifting from a point (kind 1009).
    Smoke(Smoke),
    /// One quad at a point, cycling its size, colour and frame (kind 1012).
    Sprite(Sprite),
    /// A pool of thrown billboard particles (kind 3028).
    BParticles(BParticles),
    /// Two spirals of sparks (the buff's aura, and the tracers and hits built on it).
    Spiral(Aura),
    /// The character's own mesh drawn again as a shell (kind 3003, `_GfxControlShield_t`).
    Shield(Shield),
    /// Kind 2005, `_GfxControlSuns_t` (FUN_100fd851; FUN_100fd14e the words, FUN_100fd274 the `GfxVisualSol`, FUN_100fc425 the
    /// run): 32 suns, each a camera-facing quad of the material (word 9) turned by its own angle, placed by the mode (word 10).
    /// Mode 4 (385 uses): up to three a frame born at the hit location's point and standing there, living word 30 ms, word 28
    /// wide, a random colour between words 18–21 and 22–25, their frame running from the sheet's 15th back as
    /// `15 − ⌊age / life × word 29⌋`, turned by `(k % 7) × 0.3`. Mode 0 (173 uses): all 32 two units above the point, sized
    /// `(word 28 + k × word 29) × (1 − (2 frac − 1)⁴)` over the duration (word 26), turned by `t × 0.5 × (k − 4)`, coloured from
    /// the table at 0x102c60a0 with their alpha held to 0xc0. Modes 1–3 (21 uses) are not played.
    Suns(Suns),
    /// Kind 2001, `_GfxControlSpiral_t` (the point `CreateGfxControl` FUN_100cea4b / FUN_100f500b; FUN_100f4ef1 the words,
    /// FUN_100f4f45 the visuals, FUN_100f4cb5 the run; displaysystem `gfxbeams.rs`'s `GfxVisualSpiral`): two strips of the
    /// material (word 9) half a turn apart, each one turn of 12 segments 0.6 out, rising 0.3 a radian (1.88 over the turn) and
    /// 0.3 tall, turned about the up axis at 3.7 rad/s; over the first half of the duration (word 8) each grows from its start to
    /// its end, over the second it is eaten from its start, its ends black; colours words 10–13 and 14–17 (the hit of 286
    /// programs).
    Coil { colour: [f32; 4], material: i32, duration: f32 },
    /// A kind-2002 template, `_GfxControlPlasma_t` (made by the number-taking `CreateGfxControl` FUN_100d19b0 / FUN_100ec5ae;
    /// FUN_100ec4c8 the words, FUN_100ec522 the visual, FUN_100ec2e6 the run): a `GfxVisualPlasma` of the material (word 9) at the
    /// hit location's point, its colour picked each frame between words 10–13 and 14–17 (alpha, red, green, blue; FUN_10108663),
    /// running word 18 seconds (the tracer of 611 programs). The visual (displaysystem `gfxplasma.rs`) is a band of 75 segments
    /// along the step between the location's two points, which are the same point, so along 0.01 of x each: 0.75 m along the
    /// visual's x, 0.2 across, displaced across by a tenth of the sum of four cubed sine waves of the time.
    Plasma { colour_a: [f32; 4], colour_b: [f32; 4], material: i32, duration: f32 },
}

impl Gfx {
    /// The look of an effect id. A bundle (kind 2007: up to four children in its first words) or a wrapper (kind 3004: its second word
    /// is a mode and its third the child) shows its first child that has a look.
    pub fn visual(&self, id: i32) -> Option<Visual> {
        let r = self.get(id)?;
        match r.kind {
            2004 if r.payload.len() >= 32 && STARS_MODES.contains(&r.int(10)) => Some(Visual::Stars(Stars {
                mode: r.int(10),
                colour_a: [r.float(18), r.float(19), r.float(20), r.float(21)],
                colour_b: [r.float(22), r.float(23), r.float(24), r.float(25)],
                size: r.float(28),
                radius: r.float(29),
                life: (r.int(30) as f32 / 1000.0).max(0.1),
                swirl: r.int(31) as f32,
                material: r.int(9),
                attach: r.int(7),
                duration: r.float(8),
                word11: r.float(11),
                word30: r.int(30),
            })),
            1018 if r.payload.len() >= 41 => Some(Visual::Sparks(Sparks {
                colour_a: [r.float(16), r.float(17), r.float(18), r.float(19)],
                colour_b: [r.float(20), r.float(21), r.float(22), r.float(23)],
                width: (r.float(12), r.float(13)),
                height: (r.float(14), r.float(15)),
                azimuth: (r.float(25), r.float(26)),
                elevation: (r.float(27), r.float(28)),
                speed: (r.float(29), r.float(30)),
                scale: r.float(32),
                life: (r.float(34).max(0.05), r.float(35).max(0.05)),
                count: r.int(31),
                gravity: r.float(40),
                material: r.int(9),
                attach: r.int(7),
                gate: r.int(24),
                push: r.int(38),
                push_heights: (r.float(36), r.float(37)),
                push_strength: r.float(39),
            })),
            1024 if r.payload.len() >= 17 => Some(Visual::Cords(Cords { width: r.float(11), radius: r.float(12), colour: [r.float(13), r.float(14), r.float(15), r.float(16)], material: r.int(9), duration: r.float(8) })),
            3038 => VolGrid::read(r).map(Visual::VolGrid),
            1009 => Smoke::read(r).map(Visual::Smoke),
            1012 => Sprite::read(r).map(Visual::Sprite),
            3028 => BParticles::read(r).map(Visual::BParticles),
            3001 if r.payload.len() >= 16 && r.int(10) == 1 => Some(Visual::Deform(Deform { duration: r.float(8), strength: r.float(11), ramp_in: r.float(12), ramp_out: r.float(13), frequency: r.float(14), amplitude: r.float(15) })),
            1001 | 2004 | 2005 | 2006 | 1018 => self.aura(id).map(Visual::Spiral),
            2007 | 3004 => self.references(id).into_iter().filter(|&c| c != id).find_map(|c| self.visual(c)),
            3003 => Shield::read(r).map(Visual::Shield),
            2005 if r.payload.len() >= 32 && matches!(r.int(10), 0 | 4) => Some(Visual::Suns(Suns {
                mode: r.int(10),
                attach: r.int(7),
                material: r.int(9),
                colour_a: [r.float(18), r.float(19), r.float(20), r.float(21)],
                colour_b: [r.float(22), r.float(23), r.float(24), r.float(25)],
                duration: r.float(26),
                size: r.float(28),
                step: r.float(29),
                life: r.int(30) as f32 / 1000.0,
            })),
            2001 if r.payload.len() >= 18 => Some(Visual::Coil { colour: [r.float(10), r.float(11), r.float(12), r.float(13)], material: r.int(9), duration: r.float(8) }),
            // (word 10 and the pairs after it are the range each colour channel is drawn from: 0.3 to 1)
            2002 if r.payload.len() >= 19 => Some(Visual::Plasma {
                colour_a: [r.float(10), r.float(11), r.float(12), r.float(13)],
                colour_b: [r.float(14), r.float(15), r.float(16), r.float(17)],
                material: r.int(9),
                duration: r.float(18),
            }),
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
    /// The cast effect an id names (a kind-1010 template), as `_GfxControlSpell1_t` plays it.
    pub fn cast(&self, id: i32) -> Option<Cast> {
        let r = self.get(id).filter(|r| r.kind == 1010 && r.payload.len() >= 33)?;
        let flare = |at: usize| self.get(r.int(at)).and_then(Flare::read);
        Some(Cast {
            material: r.int(9),
            colour_a: [r.float(10), r.float(11), r.float(12), r.float(13)],
            colour_b: [r.float(14), r.float(15), r.float(16), r.float(17)],
            times: std::array::from_fn(|i| r.float(18 + i)),
            flares: [flare(27), flare(28)],
            released: r.int(33) == 0,
        })
    }

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

    #[test]
    fn a_volgrid_curve_is_linear_between_its_keys_and_holds_its_last_value() {
        let keys = [(0.0, 0.0), (0.1, 80.0), (1.0, 20.0)];
        assert_eq!(curve(&keys, 0.0), 0.0);
        assert!((curve(&keys, 0.05) - 40.0).abs() < 1e-4);
        assert!((curve(&keys, 0.55) - 50.0).abs() < 1e-4);
        // at and past the last key, and before the first: the last value; no keys: 1
        assert_eq!(curve(&keys, 1.0), 20.0);
        assert_eq!(curve(&keys, -0.5), 20.0);
        assert_eq!(curve(&[], 0.5), 1.0);
        // colours blend channel by channel
        let colours = [(0.0, 0x0000_0000u32), (0.5, 0xff80_80ff)];
        assert_eq!(colour_curve(&colours, 0.25), 0x8040_4080);
        assert_eq!(colour_curve(&colours, 0.75), 0xff80_80ff);
        assert_eq!(unpack_argb(0xff80_00ff), [128.0 / 255.0, 0.0, 1.0, 1.0]);
        // a record: flags, 7 words, the duration, the material, 10–13, the counts, then five curves
        let mut payload = vec![0x4003, 0, 0, 0, 0, 0, 0, 0, 3.0f32.to_bits() as i32, 100, 0, 4, 1.0f32.to_bits() as i32, 0, 20, 1, 20];
        for _ in 0..3 {
            payload.extend([2, 0, 2.5f32.to_bits() as i32, 1.0f32.to_bits() as i32, 5.0f32.to_bits() as i32]);
        }
        for _ in 0..2 {
            payload.extend([1, 0, 0x00ff_ffff]);
        }
        let g = VolGrid::read(&Record { id: 1, kind: 3038, payload: payload.clone() }).unwrap();
        assert_eq!((g.across, g.up, g.deep, g.duration, g.material), (20, 1, 20, 3.0, 100));
        assert_eq!(g.depth, vec![(0.0, 2.5), (1.0, 5.0)]);
        assert_eq!(g.top, vec![(0.0, 0x00ff_ffff)]);
        payload.pop();
        assert!(VolGrid::read(&Record { id: 1, kind: 3038, payload }).is_none());
    }

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
        // an attach point (word 7) that equals a template's id is not a reference to it; nor is a Stars template's life (word 30)
        let mut stars = vec![3, 1000, 1001, 1, 5, 7, 2004, 31];
        stars.extend([0; 7]);
        stars.push(1000);
        stars.extend([0; 22]);
        stars.push(1000);
        stars.extend([5, 1, 2, 1, 7]);
        let gfx = Gfx::parse(&bytes(&stars)).unwrap();
        assert!(gfx.references(7).is_empty());
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
