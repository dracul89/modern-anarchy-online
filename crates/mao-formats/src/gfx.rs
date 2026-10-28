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

/// A kind-3034 template as `GfxControlShield2_t` reads it (FUN_10111a4d; FUN_1011148b the words, FUN_10111877 / FUN_10111698
/// the visual on the target's CAT mesh, FUN_10111110 the run) and `GfxVisualShield2` draws it (displaysystem `gfxshield2.rs`):
/// word 0 the flags (0x400 added, 0x800 a pulse, 0x1000 each pass lowered, 0x4000 / 0x8000 the depth modes, 0x10000 the mesh's
/// own material, 0x40000 nothing past 1000 vertices), word 8 the duration, words 9–11 the push (0 along the normals, 1
/// wobbled), the texture mapping and the alpha mode, word 12 the material, words 13–16 the texture's scales and speeds, word 17
/// unused by the draw, word 18 the pulse rate, word 19 the passes, word 20 the mesh (−1 the target's own, 0–9 the EP03 mechs'),
/// then a colour curve (a count, then time and ARGB pairs), a push curve (a count, then time and push pairs) and a last word.
/// The curves run over the fraction of the duration, as the visual's time does.
#[derive(Clone, Debug, PartialEq)]
pub struct Shield2 {
    pub flags: u32,
    pub duration: f32,
    pub wobble: i32,
    pub mapping: i32,
    pub alpha: i32,
    pub material: i32,
    pub scale: (f32, f32),
    pub speed: (f32, f32),
    pub rate: f32,
    pub passes: i32,
    pub mesh: i32,
    pub colours: Vec<(f32, u32)>,
    pub pushes: Vec<(f32, f32)>,
}

impl Shield2 {
    pub fn read(r: &Record) -> Option<Shield2> {
        if r.payload.len() < 22 {
            return None;
        }
        let mut at = 21;
        let mut pairs = |r: &Record| {
            let n = r.int(at).clamp(0, 64) as usize;
            at += 1;
            let keys: Vec<(f32, u32)> = (0..n).map(|k| (r.float(at + 2 * k), r.int(at + 2 * k + 1) as u32)).collect();
            at += 2 * n;
            keys
        };
        let colours = pairs(r);
        let pushes = pairs(r).into_iter().map(|(t, w)| (t, f32::from_bits(w))).collect();
        Some(Shield2 {
            flags: r.int(0) as u32,
            duration: r.float(8),
            wobble: r.int(9),
            mapping: r.int(10),
            alpha: r.int(11),
            material: r.int(12),
            scale: (r.float(13), r.float(14)),
            speed: (r.float(15), r.float(16)),
            rate: r.float(18),
            passes: r.int(19),
            mesh: r.int(20),
            colours,
            pushes,
        })
    }

    /// The key of `f` (FUN_1011678b / FUN_1011689d's search): the first whose time is at most `f` with the next's past it,
    /// else none (the last value holds).
    fn segment(times: impl Iterator<Item = f32> + Clone, f: f32) -> Option<usize> {
        let t: Vec<f32> = times.collect();
        (0..t.len().saturating_sub(1)).find(|&i| t[i] <= f && f < t[i + 1])
    }

    /// The colour (ARGB) at the fraction `f` (FUN_1011678b): the last key's (white with none), or the two keys about `f`
    /// mixed by `Color_t::Interpolate` (each channel `(a × (256 − w) + b × w) >> 8`, `w` the fraction by 256 truncated).
    pub fn colour_at(&self, f: f32) -> u32 {
        key_colour(&self.colours, f)
    }

    /// The push at the fraction `f` (FUN_1011689d, see [`key_curve`]).
    pub fn push_at(&self, f: f32) -> f32 {
        key_curve(&self.pushes, f)
    }
}

/// A curve of (time, ARGB) keys at `f`, as FUN_1011678b reads it: the last key's colour (white with none), or the two keys
/// about `f` mixed by `Color_t::Interpolate` (each channel `(a × (256 − w) + b × w) >> 8`, `w` the fraction by 256 truncated).
pub fn key_colour(keys: &[(f32, u32)], f: f32) -> u32 {
    let Some(&(_, last)) = keys.last() else { return u32::MAX };
    let Some(i) = (0..keys.len().saturating_sub(1)).find(|&i| keys[i].0 <= f && f < keys[i + 1].0) else { return last };
    let ((t0, a), (t1, b)) = (keys[i], keys[i + 1]);
    let span = if t1 - t0 == 0.0 { 1.0 } else { t1 - t0 };
    let w = ((f - t0) / span * 256.0) as i32;
    let mix = |shift: u32| ((((a >> shift) & 0xff) as i32 * (256 - w) + ((b >> shift) & 0xff) as i32 * w) >> 8) as u32 & 0xff;
    mix(24) << 24 | mix(16) << 16 | mix(8) << 8 | mix(0)
}

/// A kind-3039 template as `GfxControlTrail2_t` reads it (the dynel and point constructors FUN_1011548d / FUN_101153a5;
/// FUN_1011518a the words, FUN_10115260 the visual, FUN_10115605 the run, FUN_101150ff the sampling) and `GfxVisualTrail2`
/// draws it (displaysystem `gfxtrail.rs`): word 0 the flags (the locator's 1, 2 and 4; 0x400 the texture across the ribbons,
/// 0x800 coloured only while the head is not behind the oldest sample, 0x1000 the samples blown along the head's z), words 1–3
/// and 4–6 the locator's offset and turn, word 7 the attach point, word 8 the duration, word 10 the material, word 11 the
/// samples, word 12 the time between samples (0: every frame), word 13 the rate of the phase the curves run over, words 14 and
/// 15 the thrust and its random part (units a second along the head's z), then a colour curve and the curves of the frame's x
/// and y half-widths.
#[derive(Clone, Debug, PartialEq)]
pub struct Trail2 {
    pub flags: u32,
    pub place: [f32; 3],
    pub turn: [f32; 3],
    pub attach: i32,
    pub duration: f32,
    pub material: i32,
    pub samples: i32,
    pub period: f32,
    pub rate: f32,
    pub thrust: f32,
    pub fluctuate: f32,
    pub colours: Vec<(f32, u32)>,
    pub widths: (Vec<(f32, f32)>, Vec<(f32, f32)>),
}

impl Trail2 {
    pub fn read(r: &Record) -> Option<Trail2> {
        if r.payload.len() < 17 {
            return None;
        }
        let mut at = 16;
        let mut pairs = |r: &Record| {
            let n = r.int(at).clamp(0, 64) as usize;
            at += 1;
            let keys: Vec<(f32, i32)> = (0..n).map(|k| (r.float(at + 2 * k), r.int(at + 2 * k + 1))).collect();
            at += 2 * n;
            keys
        };
        let colours = pairs(r).into_iter().map(|(t, c)| (t, c as u32)).collect();
        let x = pairs(r).into_iter().map(|(t, w)| (t, f32::from_bits(w as u32))).collect();
        let y = pairs(r).into_iter().map(|(t, w)| (t, f32::from_bits(w as u32))).collect();
        let v = |k: usize| [r.float(k), r.float(k + 1), r.float(k + 2)];
        Some(Trail2 {
            flags: r.int(0) as u32,
            place: v(1),
            turn: v(4),
            attach: r.int(7),
            duration: r.float(8),
            material: r.int(10),
            samples: r.int(11),
            period: r.float(12),
            rate: r.float(13),
            thrust: r.float(14),
            fluctuate: r.float(15),
            colours,
            widths: (x, y),
        })
    }
}

/// A kind-1004 template as `_GfxControlFire_t` reads it (FUN_100dcb72 / FUN_100dc7ee; FUN_100dc433 the words, FUN_100dc59c the
/// `GfxVisualSprite2Type0`, FUN_100dbed5 the run): word 0 the flags (0x100 the flames rise; else they run back along the
/// locator's z), words 1–3 the locator's offset, word 7 the attach point, word 8 the duration (none made a life before it),
/// word 9 the material, word 10 the flames a second, words 11–12 the radius of the disc they start on, word 13 their speed
/// (times the height of their start on the sphere, over the life), words 14–17 the width and height at the start and the end,
/// words 18–21 and 22–25 the colours (alpha, red, green, blue) at the start and the end, word 26 the life, words 27–28 the push's
/// heights over the start, word 29 the push kind, word 30 its strength, word 31 a mask of `rand()`.
#[derive(Clone, Debug, PartialEq)]
pub struct Fire {
    pub flags: u32,
    pub place: [f32; 3],
    pub attach: i32,
    pub duration: f32,
    pub material: i32,
    pub rate: f32,
    pub radius: (f32, f32),
    pub speed: f32,
    pub width: (f32, f32),
    pub height: (f32, f32),
    pub start: [f32; 4],
    pub end: [f32; 4],
    pub life: f32,
    pub push_heights: (f32, f32),
    pub push: i32,
    pub push_strength: f32,
}

impl Fire {
    pub fn read(r: &Record) -> Option<Fire> {
        if r.payload.len() < 31 {
            return None;
        }
        let v4 = |k: usize| [r.float(k), r.float(k + 1), r.float(k + 2), r.float(k + 3)];
        Some(Fire {
            flags: r.int(0) as u32,
            place: [r.float(1), r.float(2), r.float(3)],
            attach: r.int(7),
            duration: r.float(8),
            material: r.int(9),
            rate: r.float(10),
            radius: (r.float(11), r.float(12)),
            speed: r.float(13),
            width: (r.float(14), r.float(15)),
            height: (r.float(16), r.float(17)),
            start: v4(18),
            end: v4(22),
            life: r.float(26),
            push_heights: (r.float(27), r.float(28)),
            push: r.int(29),
            push_strength: r.float(30),
        })
    }
}

/// A kind-3033 template as `GfxControlSpiral2_t` reads it (FUN_1011230d; FUN_10111e9d the words, FUN_10111fdb the
/// `GfxVisualSpiral2`s (displaysystem `gfxspiral2.rs`), FUN_10111c40 the run): word 0 the flags (0x200 added, 0x400 the alpha
/// pulsing), word 7 the attach point, word 8 the duration, word 9 the material, word 10 the ribbons, word 11 their segments,
/// word 12 the pulse's rate, word 13 the half height of the band, word 14 the rise along the ribbon, word 15 the first angle
/// (999 random), word 16 the turn along it, word 17 the texture's extent along it, words 18–19 the spin about the up axis and
/// its acceleration, words 20–21 the fractions faded out at the end and in at the start, then a colour curve and a radius
/// curve over the fraction of the duration (both read as words: the radius's keys are floats in them).
#[derive(Clone, Debug, PartialEq)]
pub struct Spiral2 {
    pub flags: u32,
    pub attach: i32,
    pub duration: f32,
    pub material: i32,
    pub ribbons: i32,
    pub segments: i32,
    pub pulse: f32,
    pub band: f32,
    pub rise: f32,
    pub angle: f32,
    pub turn: f32,
    pub texture: f32,
    pub spin: (f32, f32),
    pub fade_out: f32,
    pub fade_in: f32,
    pub colours: Vec<(f32, u32)>,
    pub radii: Vec<(f32, f32)>,
}

impl Spiral2 {
    pub fn read(r: &Record) -> Option<Spiral2> {
        if r.payload.len() < 23 {
            return None;
        }
        let mut at = 22;
        let mut pairs = |r: &Record| {
            let n = r.int(at).clamp(0, 64) as usize;
            at += 1;
            let keys: Vec<(f32, i32)> = (0..n).map(|k| (r.float(at + 2 * k), r.int(at + 2 * k + 1))).collect();
            at += 2 * n;
            keys
        };
        let colours = pairs(r).into_iter().map(|(t, c)| (t, c as u32)).collect();
        let radii = pairs(r).into_iter().map(|(t, w)| (t, f32::from_bits(w as u32))).collect();
        Some(Spiral2 {
            flags: r.int(0) as u32,
            attach: r.int(7),
            duration: r.float(8),
            material: r.int(9),
            ribbons: r.int(10),
            segments: r.int(11),
            pulse: r.float(12),
            band: r.float(13),
            rise: r.float(14),
            angle: r.float(15),
            turn: r.float(16),
            texture: r.float(17),
            spin: (r.float(18), r.float(19)),
            fade_out: r.float(20),
            fade_in: r.float(21),
            colours,
            radii,
        })
    }
}

/// A kind-3006 template as `_GfxControlSkyFlash_t` reads it (FUN_100ef80f / FUN_100ef6eb; FUN_100eefd2 the words, FUN_100ef58b
/// the `GfxVisualCone`s, FUN_100ef141 the run): word 0 the flags (0x200 added, 0x400 the cones widen from the top's radius
/// and lengthen as they rise, 0x800 / 0x1000 following the point while rising / falling, 0x2000 each cone turned a share of a
/// turn, 0x4000 on the ground, 0x8000 standing on it), word 7 the attach point, word 9 the material, word 10 the cycles, words
/// 11 and 12 a cycle's rise and fall, word 13 the cones, words 14–16 their segments and texture's extents, word 17 the height,
/// words 18–19 the radii below and above (below and negative: times the body's torso), words 20–21 the step of each a cone,
/// words 22–23 their growth over a cycle, words 24–29 the colours (ARGB) below and above at the start, the middle and the end.
#[derive(Clone, Debug, PartialEq)]
pub struct SkyFlash {
    pub flags: u32,
    pub attach: i32,
    pub material: i32,
    pub cycles: u32,
    pub rise: f32,
    pub fall: f32,
    pub cones: u32,
    pub segments: i32,
    pub texture: (f32, f32),
    pub height: f32,
    pub radii: (f32, f32),
    pub steps: (f32, f32),
    pub growth: (f32, f32),
    /// (below, above) at the start, the middle and the end.
    pub colours: [(u32, u32); 3],
}

impl SkyFlash {
    pub fn read(r: &Record) -> Option<SkyFlash> {
        if r.payload.len() < 30 {
            return None;
        }
        let c = |k: usize| r.int(k) as u32;
        Some(SkyFlash {
            flags: r.int(0) as u32,
            attach: r.int(7),
            material: r.int(9),
            cycles: r.int(10) as u32,
            rise: r.float(11),
            fall: r.float(12),
            cones: r.int(13) as u32,
            segments: r.int(14),
            texture: (r.float(15), r.float(16)),
            height: r.float(17),
            radii: (r.float(18), r.float(19)),
            steps: (r.float(20), r.float(21)),
            growth: (r.float(22), r.float(23)),
            colours: [(c(24), c(25)), (c(26), c(27)), (c(28), c(29))],
        })
    }

    /// How long it runs: its cycles of a rise and a fall.
    pub fn length(&self) -> f32 {
        self.cycles as f32 * (self.rise + self.fall).max(0.0)
    }
}

/// A kind-3020 template as `GfxControlTParticle_t` reads it (the dynel constructor FUN_101134ed; FUN_10112ccd the words,
/// FUN_10112b4e the `GfxVisualTParticle` (displaysystem `gfxbparticle.rs` `tp_new`, `tp_process`, `tp_draw`), FUN_10113101 the
/// run): word 0 the flags (0x100 flipped, 0x200 added, 0x400 the point on the ground), word 7 the attach point, word 8 the
/// duration, word 9 the material, word 10 the kind (0 scattered in a ball and flying, 1 tails scattered about the point, 2 in a
/// ring rising), word 11 the streaks, word 12 the spread, word 15 the drag (the tail is the head less the velocity times it),
/// word 16 the lift (along y), words 17–22 the velocity's least and most (x, x, y, y, z, z), words 23–28 the tail's and head's
/// colours (ARGB) at the start, the middle and the end, words 35–40 the tail's and head's half widths at the start, the middle
/// and the end (word 34 of 4: the fraction's eighth power).
#[derive(Clone, Debug, PartialEq)]
pub struct TParticle {
    pub flags: u32,
    pub attach: i32,
    pub duration: f32,
    pub material: i32,
    pub kind: i32,
    pub count: i32,
    pub spread: f32,
    pub drag: f32,
    pub lift: f32,
    pub low: [f32; 3],
    pub high: [f32; 3],
    /// (tail, head) at the start, the middle and the end.
    pub colours: [(u32, u32); 3],
    pub widths: [(f32, f32); 3],
    pub sharp: bool,
}

impl TParticle {
    pub fn read(r: &Record) -> Option<TParticle> {
        if r.payload.len() < 41 {
            return None;
        }
        let c = |k: usize| r.int(k) as u32;
        Some(TParticle {
            flags: r.int(0) as u32,
            attach: r.int(7),
            duration: r.float(8),
            material: r.int(9),
            kind: r.int(10),
            count: r.int(11),
            spread: r.float(12),
            drag: r.float(15),
            lift: r.float(16),
            low: [r.float(17), r.float(19), r.float(21)],
            high: [r.float(18), r.float(20), r.float(22)],
            colours: [(c(23), c(24)), (c(25), c(26)), (c(27), c(28))],
            widths: [(r.float(35), r.float(36)), (r.float(37), r.float(38)), (r.float(39), r.float(40))],
            sharp: r.int(34) == 4,
        })
    }

    /// The tail's and head's colours (ARGB) and half widths `t` seconds in (FUN_10113101), or none past the duration: over its
    /// first half from the start's to the middle's, over the second to the end's (`Color_t::Interpolate`; the widths by the
    /// fraction's eighth power with word 34 of 4); without a duration the start's.
    pub fn look(&self, t: f32) -> Option<((u32, u32), (f32, f32))> {
        let f = if self.duration >= 0.0 { t / self.duration } else { 0.0 };
        if f > 1.0 {
            return None;
        }
        let (a, b, u) = if f < 0.5 { (0, 1, 2.0 * f) } else { (1, 2, 2.0 * f - 1.0) };
        let mix = |x: u32, y: u32| key_colour(&[(0.0, x), (1.0, y)], u);
        let w = if self.sharp { u.powi(8) } else { u };
        let (wa, wb) = (self.widths[a], self.widths[b]);
        Some((
            (mix(self.colours[a].0, self.colours[b].0), mix(self.colours[a].1, self.colours[b].1)),
            (wa.0 + (wb.0 - wa.0) * w, wa.1 + (wb.1 - wa.1) * w),
        ))
    }
}

/// A kind-1007 template as `_GfxControlNano0_t` reads it (the point constructor FUN_100e73ea; FUN_100e67f8 the words, FUN_100e7146
/// the `GfxVisualSprite2Type0` and the first burst, FUN_100e6f76 the run, FUN_100e6ca4 / FUN_100e69ca a sprite): word 8 the
/// duration (it stops making sprites a life before it), word 9 the material, word 10 the sprites a second (`ftol(rate ×
/// elapsed)` made in all, laid along the way the point moved since the last frame), words 12–15 the sizes (width at the start
/// and end, height at the start and end), words 16–19 and 20–23 the colours at the start and the end (alpha, red, green, blue),
/// word 24 a mask of `rand()` (a frame makes none unless it clears it), words 25–30 the heading, the angle over the level and
/// the speed (each a range), word 31 the sprites made at once, word 32 the speed's scale, words 34–35 the life, words 35–36 the
/// push's heights over the sprite's birth, word 37 the push kind, word 38 its strength.
#[derive(Clone, Debug, PartialEq)]
pub struct Nano {
    pub flags: u32,
    pub attach: i32,
    pub duration: f32,
    pub material: i32,
    pub rate: f32,
    pub width: (f32, f32),
    pub height: (f32, f32),
    pub start: [f32; 4],
    pub end: [f32; 4],
    pub mask: u32,
    pub heading: (f32, f32),
    pub rise: (f32, f32),
    pub speed: (f32, f32),
    pub burst: i32,
    pub scale: f32,
    pub life: (f32, f32),
    pub push_heights: (f32, f32),
    pub push: i32,
    pub push_strength: f32,
}

impl Nano {
    pub fn read(r: &Record) -> Option<Nano> {
        if r.payload.len() < 38 {
            return None;
        }
        let v4 = |k: usize| [r.float(k), r.float(k + 1), r.float(k + 2), r.float(k + 3)];
        Some(Nano {
            flags: r.int(0) as u32,
            attach: r.int(7),
            duration: r.float(8),
            material: r.int(9),
            rate: r.float(10),
            width: (r.float(12), r.float(13)),
            height: (r.float(14), r.float(15)),
            start: v4(16),
            end: v4(20),
            mask: r.int(24) as u32,
            heading: (r.float(25), r.float(26)),
            rise: (r.float(27), r.float(28)),
            speed: (r.float(29), r.float(30)),
            burst: r.int(31),
            scale: r.float(32),
            life: (r.float(34), r.float(35)),
            push_heights: (r.float(35), r.float(36)),
            push: r.int(37),
            push_strength: r.float(38),
        })
    }
}

/// A kind-1022 template as `_GfxControlTracer3_t` reads it (made by `CreateGfxControlTracer` FUN_100d176c through FUN_100ff933;
/// FUN_100ff657 the words, FUN_100ff7f3 / FUN_100ff6e2 the start, FUN_100ff91b / FUN_100ff754 the run): word 10 the speed (at
/// most five lengths of the line a second), words 11–14 the colour its nano (word 15, kind 1007) starts with (alpha, red,
/// green, blue; it ends with that colour clear), dragged from the caster to the target and gone with the tracer when it
/// arrives.
#[derive(Clone, Debug, PartialEq)]
pub struct Tracer3 {
    pub speed: f32,
    pub colour: [f32; 4],
    pub child: Nano,
}

/// A kind-1019 template as `_GfxControlTracer1_t` reads it (made by `CreateGfxControlTracer` FUN_100d176c through FUN_100fec4e;
/// FUN_100fe3b0 the words, FUN_100fe9f9 the frame along the line, FUN_100fe845 the `GfxVisualFlareType0`, FUN_100fe545 its
/// streaks, FUN_100fe7e1 / FUN_100fe706 the run): words 1–3 and 4–6 the locator's offset and turn, word 9 the material, words
/// 10–13 the colour (packed as `gfxvec::pack`: alpha, red, green, blue), word 18 the streaks' size, word 19 a count of point
/// pairs from word 21 (each a streak's head and tail in the model), word 20 the copies of them turned about the model's y axis.
#[derive(Clone, Debug, PartialEq)]
pub struct Tracer1 {
    pub flags: u32,
    pub place: [f32; 3],
    pub turn: [f32; 3],
    pub material: i32,
    pub colour: u32,
    pub size: f32,
    pub copies: i32,
    pub streaks: Vec<([f32; 3], [f32; 3])>,
}

impl Tracer1 {
    pub fn read(r: &Record) -> Option<Tracer1> {
        if r.payload.len() < 21 {
            return None;
        }
        let byte = |k: usize| ((r.float(k) * 255.0 - 0.49999).round() as i32 as u32) & 0xff;
        let n = r.int(19).clamp(0, 64) as usize;
        let v = |k: usize| [r.float(k), r.float(k + 1), r.float(k + 2)];
        Some(Tracer1 {
            flags: r.int(0) as u32,
            place: v(1),
            turn: v(4),
            material: r.int(9),
            colour: byte(10) << 24 | byte(11) << 16 | byte(12) << 8 | byte(13),
            size: r.float(18),
            copies: r.int(20),
            streaks: (0..n).map(|k| (v(21 + 6 * k), v(24 + 6 * k))).collect(),
        })
    }
}

/// A kind-1025 template as `_GfxControlTracer5_t` reads it (made by `CreateGfxControlTracer` FUN_100d176c through FUN_10100b0d;
/// FUN_101003f9 the words, FUN_1010077d the frame along the line, FUN_101004b4 the `GfxVisualCord4` of three links,
/// FUN_10100aa3 / FUN_1010065a the run): word 9 the material, word 10 the speed of the head (at most five lengths of the line a
/// second), word 11 how far the lead link runs ahead of it, word 12 the links' half width, words 13–16 the colour (alpha, red,
/// green, blue, from 0 to 1).
#[derive(Clone, Debug, PartialEq)]
pub struct Tracer5 {
    pub flags: u32,
    pub material: i32,
    pub speed: f32,
    pub lead: f32,
    pub width: f32,
    /// ARGB, packed as FUN_1010077d packs words 13–16 (each times 255, rounded).
    pub colour: u32,
}

impl Tracer5 {
    pub fn read(r: &Record) -> Option<Tracer5> {
        if r.payload.len() < 17 {
            return None;
        }
        let byte = |k: usize| ((r.float(k) * 255.0 - 0.49999).round() as i32 as u32) & 0xff;
        Some(Tracer5 {
            flags: r.int(0) as u32,
            material: r.int(9),
            speed: r.float(10),
            lead: r.float(11),
            width: r.float(12),
            colour: byte(13) << 24 | byte(14) << 16 | byte(15) << 8 | byte(16),
        })
    }

    /// How long the head takes from the caster to the target `length` away (FUN_1010077d keeps the speed to five lengths a
    /// second; the run ends the control when the head is there).
    pub fn flight(&self, length: f32) -> f32 {
        let speed = self.speed.min(length * 5.0).max(1e-3);
        length / speed
    }
}

/// A kind-3024 template as `GfxControlBParticle_t` reads it (the dynel and point constructors FUN_1010ad28 / FUN_1010ac00;
/// FUN_1010a644 the words, FUN_1010a412 the `GfxVisualBParticle` (displaysystem `gfxbparticle.rs`), FUN_1010a909 the run):
/// word 0 the flags (0x100 flipped, 0x200 added, 0x400 the point on the ground, 0x1000 opaque), word 7 the attach point, word 8
/// the duration, word 9 the material, word 10 the visual's kind (how its particles are made and run), word 11 their count,
/// word 12 their spread, words 13–15 the motion, shape and fading, word 16 the spin rate (kind 8: how fast its wait runs
/// down), word 17 the phase rate, word 18 the lift, words 19–24 the velocity's least and most (x, then y, then z), words
/// 25–27 the colours (ARGB) at the start, the middle and the end, words 28–30 a fade with the distance, words 31–36 the half
/// sizes across and up at the start, the middle and the end, word 37 an interval, words 38 and 39 the frame rate and the
/// frames' bounds, words 40 and 41 the fractions of the duration the start and the end take, words 42 and 43 the lives.
#[derive(Clone, Debug, PartialEq)]
pub struct BParticle {
    pub flags: u32,
    pub attach: i32,
    pub duration: f32,
    pub material: i32,
    pub kind: i32,
    pub count: i32,
    pub spread: f32,
    pub motion: i32,
    pub shape: i32,
    pub fading: i32,
    pub spin: f32,
    pub phase_rate: f32,
    pub colours: [u32; 3],
    pub fade: [f32; 3],
    /// Across and up, at the start, the middle and the end.
    pub sizes: [(f32, f32); 3],
    pub frame_rate: f32,
    pub bounds: i32,
    pub ramp: (f32, f32),
}

impl BParticle {
    pub fn read(r: &Record) -> Option<BParticle> {
        if r.payload.len() < 42 {
            return None;
        }
        Some(BParticle {
            flags: r.int(0) as u32,
            attach: r.int(7),
            duration: r.float(8),
            material: r.int(9),
            kind: r.int(10),
            count: r.int(11),
            spread: r.float(12),
            motion: r.int(13),
            shape: r.int(14),
            fading: r.int(15),
            spin: r.float(16),
            phase_rate: r.float(17),
            colours: [r.int(25) as u32, r.int(26) as u32, r.int(27) as u32],
            fade: [r.float(28), r.float(29), r.float(30)],
            sizes: [(r.float(31), r.float(32)), (r.float(33), r.float(34)), (r.float(35), r.float(36))],
            frame_rate: r.float(38),
            bounds: r.int(39),
            ramp: (r.float(40), r.float(41)),
        })
    }

    /// The colour (ARGB) and half sizes the run gives the visual `t` seconds in (FUN_1010a909), or none once past the duration:
    /// without a duration the middle ones; else over the start (word 40 of the duration, 0.5 when negative) from the start's to
    /// the middle's, then the middle's, and past word 41 over `1 − word 41` (0.5 when that is negative) to the end's; the
    /// colours mixed by `Color_t::Interpolate`, the sizes by the fraction (its eighth power with motion 4).
    pub fn look(&self, t: f32) -> Option<(u32, (f32, f32))> {
        let mix = |a: (f32, f32), b: (f32, f32), u: f32| (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
        if self.duration < 0.0 {
            return Some((self.colours[1], self.sizes[1]));
        }
        let f = t / self.duration;
        if f > 1.0 {
            return None;
        }
        let start = if self.ramp.0 < 0.0 { 0.5 } else { self.ramp.0 };
        let end_span = if 1.0 - self.ramp.1 < 0.0 { 0.5 } else { 1.0 - self.ramp.1 };
        let power = |u: f32| if self.motion == 4 { u.powi(8) } else { u };
        if f < start {
            let u = f / if start == 0.0 { 1.0 } else { start };
            Some((key_colour(&[(0.0, self.colours[0]), (1.0, self.colours[1])], u), mix(self.sizes[0], self.sizes[1], power(u))))
        } else if f > self.ramp.1 {
            let u = (f - self.ramp.1) / if end_span == 0.0 { 1.0 } else { end_span };
            Some((key_colour(&[(0.0, self.colours[1]), (1.0, self.colours[2])], u), mix(self.sizes[1], self.sizes[2], power(u))))
        } else {
            Some((self.colours[1], self.sizes[1]))
        }
    }
}

/// A kind-1029 template as `_GfxControlVulcanRocks_t` reads it (the dynel and point constructors FUN_10103a18 / FUN_1010369d;
/// FUN_10103448 the words, FUN_10103595 the `GfxVisualRockList`, FUN_10103bc1 the run): word 0 the flags (the locator's 1, 2 and
/// 4; 0x400 kept without its locator), words 1–6 the locator's offset and turn, word 7 the attach point, word 8 the duration,
/// word 10 the speed a rock is thrown at, words 11 and 12 the least and most angle over the level, word 20 the most rocks
/// (and how many come to rest before the effect ends), word 21 the rocks tried a second, word 22 a count of the kinds that
/// follow (the visual environment's effect pools: `rock_mesh`), then a word that keeps resting rocks from counting.
#[derive(Clone, Debug, PartialEq)]
pub struct VulcanRocks {
    pub flags: u32,
    pub place: [f32; 3],
    pub attach: i32,
    pub duration: f32,
    pub speed: f32,
    pub angles: (f32, f32),
    pub most: f32,
    pub rate: f32,
    pub kinds: Vec<i32>,
    pub endless: bool,
}

impl VulcanRocks {
    pub fn read(r: &Record) -> Option<VulcanRocks> {
        if r.payload.len() < 23 {
            return None;
        }
        let n = r.int(22).clamp(0, 64) as usize;
        Some(VulcanRocks {
            flags: r.int(0) as u32,
            place: [r.float(1), r.float(2), r.float(3)],
            attach: r.int(7),
            duration: r.float(8),
            speed: r.float(10),
            angles: (r.float(11), r.float(12)),
            most: r.float(20),
            rate: r.float(21),
            kinds: (0..n).map(|k| r.int(23 + k)).collect(),
            endless: r.int(23 + n) > 0,
        })
    }
}

/// The mesh of an effect pool kind of the visual environment (displaysystem FUN_100616e4's switch, `envfx.rs` `source`), for
/// the kinds made from a mesh; the others are a material alone, and `GfxVisualRockList::GetNew` makes no rock of them
/// (FUN_1001c199 needs the pool's mesh data).
pub fn rock_mesh(kind: i32) -> Option<&'static str> {
    const GIBS: [&str; 24] = [
        "gib_skull.abiff", "gib01.abiff", "gib02.abiff", "gib03.abiff", "gib04.abiff", "gib05.abiff", "gib06.abiff", "gib07.abiff",
        "gib_skull_ice.abiff", "gib01_ice.abiff", "gib02_ice.abiff", "gib03_ice.abiff", "gib04_ice.abiff", "gib05_ice.abiff", "gib06_ice.abiff", "gib07_ice.abiff",
        "gib_skull_slime.abiff", "gib01_slime.abiff", "gib02_slime.abiff", "gib03_slime.abiff", "gib04_slime.abiff", "gib05_slime.abiff", "gib06_slime.abiff", "gib07_slime.abiff",
    ];
    const ROCKS: [&str; 7] = ["rock01.abiff", "rock02.abiff", "rock03.abiff", "rock04.abiff", "rock05.abiff", "rock06.abiff", "rock07.abiff"];
    const SHRAPNEL: [&str; 7] = ["tower_shrapnel1.abiff", "tower_shrapnel2.abiff", "tower_shrapnel3.abiff", "tower_shrapnel4.abiff", "tower_shrapnel5.abiff", "tower_shrapnel6.abiff", "tower_shrapnel7.abiff"];
    match kind {
        4 => Some("rock01.abiff"),
        0xb..=0x11 => Some(ROCKS[(kind - 0xb) as usize]),
        0x12..=0x29 => Some(GIBS[(kind - 0x12) as usize]),
        0x2e => Some("shell_casing.abiff"),
        0x2f..=0x35 => Some(SHRAPNEL[(kind - 0x2f) as usize]),
        _ => None,
    }
}

/// A curve of (time, value) keys at `f`, as FUN_1011689d reads it: the last key's value (1 with none), or the two keys about
/// `f` (the first whose time is at most `f` with the next's past it) mixed linearly.
pub fn key_curve(keys: &[(f32, f32)], f: f32) -> f32 {
    let Some(&(_, last)) = keys.last() else { return 1.0 };
    let Some(i) = (0..keys.len().saturating_sub(1)).find(|&i| keys[i].0 <= f && f < keys[i + 1].0) else { return last };
    let ((t0, a), (t1, b)) = (keys[i], keys[i + 1]);
    let span = if t1 - t0 == 0.0 { 1.0 } else { t1 - t0 };
    let u = (f - t0) / span;
    b * u + (1.0 - u) * a
}

/// A kind-3000 template as `_GfxControlShockWave_t` reads it (the dynel, point and matrix constructors FUN_100eeec0 / FUN_100eed45
/// / FUN_100eedfb; FUN_100ee3cc the words, FUN_100eeacb the visuals, FUN_100ee525 the run): word 0 the flags (1 following the
/// dynel, 0x800 the rings added, 0x1000 their texture round them, 0x2000 their radii kept from below zero, 0x4000 the cones'
/// texture up them), word 8 a ring's life, word 11 the rings (`GfxVisualGroundRing`s of word 10 segments and word 9's
/// material, its texture scaled by words 20 and 21), one born every word 23 seconds; over its life a ring's inner and outer
/// radii run from words 12 and 13 to 16 and 17 and its inner and outer colours (ARGB) from words 14 and 15 to 18 and 19, laid
/// word 22 over the ground. Word 24 the cones (`GfxVisualCone`s of word 26 segments, word 25's material, their texture's extent
/// words 27 and 28), cone k word 29 tall, `word 30 + k × word 32` wide below and `word 31 × word 33^k + k × word 32` above,
/// coloured words 34 below and 35 above with their alphas by the time into each ring's interval.
#[derive(Clone, Debug, PartialEq)]
pub struct ShockWave {
    pub flags: u32,
    pub life: f32,
    pub ring_material: i32,
    pub segments: i32,
    pub rings: i32,
    /// Inner and outer radii, born and at the end of its life.
    pub radii: [f32; 4],
    /// Inner and outer colours (ARGB), born and at the end.
    pub colours: [u32; 4],
    pub texture: (f32, f32),
    pub lift: f32,
    pub interval: f32,
    pub cones: i32,
    pub cone_material: i32,
    pub cone_segments: i32,
    pub cone_texture: (f32, f32),
    pub height: f32,
    /// Words 30–33: the width below, above, the step a cone and the factor a cone above.
    pub widths: [f32; 4],
    pub cone_colours: [u32; 2],
}

impl ShockWave {
    pub fn read(r: &Record) -> Option<ShockWave> {
        if r.payload.len() < 36 {
            return None;
        }
        Some(ShockWave {
            flags: r.int(0) as u32,
            life: r.float(8),
            ring_material: r.int(9),
            segments: r.int(10),
            rings: r.int(11),
            radii: [r.float(12), r.float(13), r.float(16), r.float(17)],
            colours: [r.int(14) as u32, r.int(15) as u32, r.int(18) as u32, r.int(19) as u32],
            texture: (r.float(20), r.float(21)),
            lift: r.float(22),
            interval: r.float(23),
            cones: r.int(24),
            cone_material: r.int(25),
            cone_segments: r.int(26),
            cone_texture: (r.float(27), r.float(28)),
            height: r.float(29),
            widths: [r.float(30), r.float(31), r.float(32), r.float(33)],
            cone_colours: [r.int(34) as u32, r.int(35) as u32],
        })
    }

    /// How long the rings run: the last born `(rings − 1) × interval` in, living its life.
    pub fn length(&self) -> f32 {
        (self.rings.max(1) - 1) as f32 * self.interval.max(0.0) + self.life.max(0.0)
    }
}

/// The meshes `GfxControlEffectMesh_t` shows by word 10 (FUN_1010d457), and the Atrox ones flag 0x10000 picks instead for an
/// Atrox (the body's stat 4 of 4) on the hoverbikes.
const EFFECT_MESHES: [&str; 28] = [
    "EP03_shoulder_rocket.abiff",
    "EP03_mech_heal_effect.abiff",
    "EP03_blast_wave_effect.abiff",
    "EP03_bullet_casing.abiff",
    "EP03_EMP_blast.abiff",
    "EP03_bomber_debris_01.abiff",
    "EP03_bomber_debris_02.abiff",
    "EP03_bomber_debris_03.abiff",
    "EP03_RUBIKA_asteroidbig.abiff",
    "EP03_RUBIKA_asteroidbig2.abiff",
    "EP03_preorder_mech_statel.abiff",
    "EP03_scout_mech_statel.abiff",
    "EP03_heavy_mech_statel.abiff",
    "EP03_anti_personnel_gun_statel.abiff",
    "EP03_anti_vehicle_gun_statel.abiff",
    "EP03_preorder_mech_upgraded_statel.abiff",
    "EP03_scout_mech_upgraded_statel.abiff",
    "EP03_heavy_mech_upgraded_statel.abiff",
    "EP03_anti_personnel_gun_upgraded_statel.abiff",
    "EP03_anti_vehicle_gun_upgraded_statel.abiff",
    "hoverbike_a_effect_circle.abiff",
    "hoverbike_b_effect_circle.abiff",
    "hoverboard_a_fan.abiff",
    "hoverboard_b_fan.abiff",
    "xan_grid_dependencybox.abiff",
    "penumbra_shoulderpads_atrox.abiff",
    "hoverboard_c_fx01.abiff",
    "hoverbike_b_effect_circle_red.abiff",
];

/// A kind-3025 template as `GfxControlEffectMesh_t` reads it (the dynel and point constructors FUN_1010da0d / FUN_1010d866;
/// FUN_1010caf8 the words, FUN_1010d457 the mesh, FUN_1010ce9c the run): word 0 the flags (the locator's 1, 2 and 4; 0x100 on
/// the ground; 0x200 by the body's scale; 0x400 / 0x800 facing and fading with the camera; 0x1000 the mesh's animation run;
/// 0x2000 / 0x4000 shown with a vehicle's speed; 0x8000 an Atrox's 1.45 times; 0x10000 the Atrox hoverbike circles), words
/// 1–3 and 4–6 the locator's offset and turn, word 7 the attach point, word 8 the duration, word 9 the motion (1 a bob
/// `sin(2π t / duration) × word 17` a second up, else words 16–18 a velocity and 19–21 its acceleration on the offset of words
/// 13–15), word 10 the mesh, words 22–24 the spin axis, 25–27 its angle, rate and acceleration, 28–30 the scale, its rate and
/// acceleration, word 31 the mesh's rendering effect (4–6 added), then an opacity curve over the fraction of the duration.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectMesh {
    pub flags: u32,
    pub place: [f32; 3],
    pub turn: [f32; 3],
    pub attach: i32,
    pub duration: f32,
    pub motion: i32,
    pub mesh: i32,
    pub offset: [f32; 3],
    pub velocity: [f32; 3],
    pub acceleration: [f32; 3],
    pub axis: [f32; 3],
    /// The angle, its rate and its acceleration.
    pub spin: [f32; 3],
    /// The scale, its rate and its acceleration.
    pub scale: [f32; 3],
    pub effect: i32,
    pub opacity: Vec<(f32, f32)>,
}

/// Where an effect mesh is `t` seconds in (see [`EffectMesh::pose`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshPose {
    pub offset: [f32; 3],
    pub angle: f32,
    pub scale: f32,
    pub opacity: f32,
}

impl EffectMesh {
    pub fn read(r: &Record) -> Option<EffectMesh> {
        if r.payload.len() < 33 {
            return None;
        }
        let v = |k: usize| [r.float(k), r.float(k + 1), r.float(k + 2)];
        let n = r.int(32).clamp(0, 64) as usize;
        let opacity = (0..n).map(|k| (r.float(33 + 2 * k), r.float(34 + 2 * k))).collect();
        Some(EffectMesh {
            flags: r.int(0) as u32,
            place: v(1),
            turn: v(4),
            attach: r.int(7),
            duration: r.float(8),
            motion: r.int(9),
            mesh: r.int(10),
            offset: v(13),
            velocity: v(16),
            acceleration: v(19),
            axis: v(22),
            spin: v(25),
            scale: v(28),
            effect: r.int(31),
            opacity,
        })
    }

    /// The mesh's file (FUN_1010d457): an Atrox's hoverbike circle with flag 0x10000, else word 10's.
    pub fn mesh_file(&self, atrox: bool) -> Option<&'static str> {
        if self.flags & 0x10000 != 0 && atrox {
            match self.mesh {
                0x14 => return Some("hoverbike_a_effect_circle_atrox.abiff"),
                0x15 => return Some("hoverbike_b_effect_circle_atrox.abiff"),
                0x1b => return Some("hoverbike_b_effect_circle_red_atrox.abiff"),
                _ => return None,
            }
        }
        EFFECT_MESHES.get(usize::try_from(self.mesh).ok()?).copied()
    }

    /// The run (FUN_1010ce9c) `t` seconds in, or none once it has ended: each frame the offset moves by the velocity and the
    /// velocity by the acceleration (motion 1: the offset's y by `sin(2π t / duration) × word 17`), the angle by its rate and the
    /// rate by its acceleration, the scale likewise (here the sums' closed forms); the opacity 1 (0 with the vehicle flags
    /// 0x6000, until a vehicle's speed shows it) times the curve at `t / duration` when that is not negative.
    pub fn pose(&self, t: f32) -> Option<MeshPose> {
        let d = self.duration;
        if t < 0.0 || (d > 0.0 && t >= d) {
            return None;
        }
        let along = |x: [f32; 3]| x[0] + x[1] * t + x[2] * t * t / 2.0;
        let mut offset = self.offset;
        if self.motion == 1 {
            if d != 0.0 {
                let tau = std::f32::consts::TAU;
                offset[1] += self.velocity[1] * d / tau * (1.0 - (tau * t / d).cos());
            }
        } else {
            for k in 0..3 {
                offset[k] += self.velocity[k] * t + self.acceleration[k] * t * t / 2.0;
            }
        }
        let fraction = t / d;
        let curve = if fraction >= 0.0 { key_curve(&self.opacity, fraction) } else { 1.0 };
        let base = if self.flags & 0x6000 != 0 { 0.0 } else { 1.0 };
        Some(MeshPose { offset, angle: along(self.spin), scale: along(self.scale), opacity: base * curve })
    }
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
    /// The character's own mesh drawn again as a shell by its colour and push curves (kind 3034, `GfxControlShield2_t`).
    Shield2(Shield2),
    /// A named mesh moved, spun and grown by its run (kind 3025, `GfxControlEffectMesh_t`).
    EffectMesh(EffectMesh),
    /// Four crossed ribbons along the attach point's recent frames (kind 3039, `GfxControlTrail2_t`).
    Trail2(Trail2),
    /// Rocks thrown up from the point, falling and bouncing on the ground (kind 1029, `_GfxControlVulcanRocks_t`).
    VulcanRocks(VulcanRocks),
    /// A pool of billboard particles in one of the visual's kinds (kind 3024, `GfxControlBParticle_t`).
    BParticle(BParticle),
    /// A thin streak running from the caster to the target (kind 1025, `_GfxControlTracer5_t`).
    Tracer5(Tracer5),
    /// A little model of streaks flying from the caster to the target (kind 1019, `_GfxControlTracer1_t`).
    Tracer1(Tracer1),
    /// A stream of sprites from its point (kind 1007, `_GfxControlNano0_t`).
    Nano(Nano),
    /// A pool of streaks, each two crossed strips from its head to its tail (kind 3020, `GfxControlTParticle_t`).
    TParticle(TParticle),
    /// Nested cones of light coming down from the sky (kind 3006, `_GfxControlSkyFlash_t`).
    SkyFlash(SkyFlash),
    /// Ribbons spiralling round the point, turning (kind 3033, `GfxControlSpiral2_t`).
    Spiral2(Spiral2),
    /// Flames off a disc at the point (kind 1004, `_GfxControlFire_t`).
    Fire(Fire),
    /// Kind 3032, `GfxControlGroundShake_t` (FUN_1010f333; FUN_1010eff5 the words, FUN_1010f12a the run): the camera's eye shaken
    /// by a random vector each frame (`n3Camera_t` +0x1d4 then `UpdateTargetEye`), as large as the curve (from word 13) at the
    /// fraction of the duration (word 8) times `(reach − d) / reach`, `d` the eye's distance from the point held to the reach
    /// (word 12).
    GroundShake { duration: f32, reach: f32, curve: Vec<(f32, f32)> },
    /// Kind 3031, `GfxControlTParticle2_t` (FUN_1011490c; FUN_1011373d the words, those of `GfxControlBParticle2_t` up to word 33,
    /// then its two colour curves from word 34; FUN_10113b55 the run): `BParticles`' particles, each drawn as a
    /// `GfxVisualTParticle2` streak from its place along its velocity times word 14 (its `length`; the run does not damp the
    /// velocity by it), its head its size wide and coloured by the second curve, its tail its size over word 33 wide and coloured
    /// by the first, both at the particle's age.
    TParticle2 { particles: BParticles, length: f32, head: Vec<(f32, u32)> },
    /// A nano emitter dragged from the caster to the target (kind 1022, `_GfxControlTracer3_t`).
    Tracer3(Tracer3),
    /// Kind 3036, `GfxControlToggle_t` (FUN_101128d1 / FUN_1011279a / FUN_101127fc; FUN_1011265a the words, FUN_10112952 the run,
    /// FUN_101125ed the playfield test, FUN_10112580 the vehicle's state): word 0 the flags, word 1 the duration, word 2 the child
    /// effect, word 3 a mask of the playfield's flags, word 4 a count of playfield ids that follow. The child is started when
    /// the playfield passes (its flags meet the mask, if any, and it is one of the ids, or with flag 0x800 none of them); with
    /// flag 0x2000 only as the body's vehicle starts to move forward; with 0x8000 it is stopped when the vehicle stops; with
    /// 0x400 the toggle waits for the next start once the child has ended.
    Toggle { child: Box<Visual>, flags: u32, mask: u32, places: Vec<i32> },
    /// A child started `start` seconds into its parent's run (a resolved `Toggle`: see the client's `resolve_toggles`).
    Delayed(Box<Visual>, f32),
    /// Rings spreading over the ground one after another and cones of light rising with them (kind 3000,
    /// `_GfxControlShockWave_t`).
    ShockWave(ShockWave),
    /// Kind 2005, `_GfxControlSuns_t` (FUN_100fd851; FUN_100fd14e the words, FUN_100fd274 the `GfxVisualSol`, FUN_100fc425 the
    /// run): 32 suns, each a camera-facing quad of the material (word 9) turned by its own angle, placed by the mode (word 10).
    /// Mode 4 (385 uses): up to three a frame born at the hit location's point and standing there, living word 30 ms, word 28
    /// wide, a random colour between words 18–21 and 22–25, their frame running from the sheet's 15th back as
    /// `15 − ⌊age / life × word 29⌋`, turned by `(k % 7) × 0.3`. Mode 0 (173 uses): all 32 two units above the point, sized
    /// `(word 28 + k × word 29) × (1 − (2 frac − 1)⁴)` over the duration (word 26), turned by `t × 0.5 × (k − 4)`, coloured from
    /// the table at 0x102c60a0 with their alpha held to 0xc0. Modes 1–3 (21 uses) are not played.
    Suns(Suns),
    /// Kind 3029, `GfxControlScatter_t` (FUN_10110fc8; FUN_10110d06 the words, FUN_10110db5 the birth times, FUN_101108a8 the
    /// run): word 6 copies of the effect of word 10, each born at a time (word 5: 0 random within word 7 seconds, 1 evenly over
    /// them) at a place (word 4: 0 a random point of a cube ±word 8, 1 a cube ±word 9 on a column climbing word 8 / count a
    /// child) about the target plus words 1–3 (flag 0x2000: on the target itself, the place unused); with flag 0x400 the round
    /// starts again once every child has ended.
    Scatter { child: Box<Visual>, children: Vec<(f32, [f32; 3])>, looping: bool },
    /// Kind 2007, `_GfxControlMeta_t` (FUN_100e5f7a and its kin): each of its first ten words names a child effect, and every one
    /// is started at once as an effect of its own (`_EffectHandler_t::CreateEffect2`). Given when two or more children have a look
    /// (one alone is given as itself).
    Meta(Vec<Visual>),
    /// Kind 2006, `_GfxControlElectra_t` (FUN_100d9aa9; FUN_100d97ba the words, FUN_100d98e0 the `GfxVisualElectra`
    /// (displaysystem `gfxsol.rs`), FUN_100d9de5 the run), mode 1 (word 10; every template a program names): 64 quads of the
    /// material (word 9), born at `28 / life` a second (at most two a frame) on a random point of a sphere stretched 1.5 up and
    /// scaled by word 29 about the attach point, each turned at random (its two half-axes word 28 long, flipped at random), living
    /// word 30 ms, its frame running over its life, its colour picked between words 18–21 and 22–25; the effect lasts word 26.
    Electra { material: i32, attach: i32, colour_a: [f32; 4], colour_b: [f32; 4], duration: f32, size: f32, reach: f32, life: f32 },
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
    /// The look of an effect id. A bundle (kind 2007: up to ten children in its first words) shows every child that has a look
    /// ([`Visual::Meta`]); a wrapper (kind 3004: its second word is a mode and its third the child) its first child that has one.
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
            2007 => {
                let mut looks: Vec<Visual> = (0..10.min(r.payload.len())).map(|k| r.int(k)).filter(|&c| c != 0 && c != id).filter_map(|c| self.visual(c)).collect();
                match looks.len() {
                    0 => None,
                    1 => looks.pop(),
                    _ => Some(Visual::Meta(looks)),
                }
            }
            3004 => self.references(id).into_iter().filter(|&c| c != id).find_map(|c| self.visual(c)),
            3029 if r.payload.len() >= 11 && r.int(10) != id => {
                let child = self.visual(r.int(10))?;
                let (flags, place, timing, count) = (r.int(0) as u32, r.int(4), r.int(5), r.int(6).clamp(0, 64));
                let (span, reach, spread) = (r.float(7), r.float(8), r.float(9));
                // (the original's rand() stood in for by a hash of the child's number)
                let unit = |k: i32, salt: u32| {
                    let mut x = (k as u32).wrapping_mul(0x9e37_79b1) ^ salt.wrapping_mul(0x85eb_ca6b) ^ (id as u32);
                    x ^= x >> 15;
                    x = x.wrapping_mul(0x2c1b_3c6d);
                    x ^= x >> 12;
                    (x & 0xffff) as f32 / 65535.0
                };
                let base = [r.float(1), r.float(3), r.float(2)];
                let children = (0..count)
                    .map(|k| {
                        let born = if timing == 1 { span * k as f32 / count.max(1) as f32 } else { unit(k, 1) * span };
                        let cube = |scale: f32| [0, 1, 2].map(|i| (2.0 * unit(k, 2 + i as u32) - 1.0) * scale);
                        let offset = if flags & 0x2000 != 0 {
                            [0.0; 3]
                        } else if place == 1 {
                            let c = cube(spread);
                            [c[0], c[1], c[2] + reach / count.max(1) as f32 * k as f32]
                        } else {
                            cube(reach)
                        };
                        (born, [0, 1, 2].map(|i| base[i] + offset[i]))
                    })
                    .collect();
                Some(Visual::Scatter { child: Box::new(child), children, looping: flags & 0x400 != 0 })
            }
            3003 => Shield::read(r).map(Visual::Shield),
            3034 => Shield2::read(r).map(Visual::Shield2),
            3025 => EffectMesh::read(r).map(Visual::EffectMesh),
            3000 => ShockWave::read(r).map(Visual::ShockWave),
            3039 => Trail2::read(r).map(Visual::Trail2),
            1029 => VulcanRocks::read(r).map(Visual::VulcanRocks),
            3024 => BParticle::read(r).map(Visual::BParticle),
            1025 => Tracer5::read(r).map(Visual::Tracer5),
            1019 => Tracer1::read(r).map(Visual::Tracer1),
            1007 => Nano::read(r).map(Visual::Nano),
            3020 => TParticle::read(r).map(Visual::TParticle),
            3006 => SkyFlash::read(r).map(Visual::SkyFlash),
            3033 => Spiral2::read(r).map(Visual::Spiral2),
            1004 => Fire::read(r).map(Visual::Fire),
            3032 if r.payload.len() >= 14 => {
                let n = r.int(13).clamp(0, 64) as usize;
                let curve = (0..n).map(|k| (r.float(14 + 2 * k), r.float(15 + 2 * k))).collect();
                Some(Visual::GroundShake { duration: r.float(8), reach: r.float(12), curve })
            }
            3031 => {
                // (its curves follow word 33, a word sooner than `BParticles`')
                let particles = BParticles::read(r)?;
                let curve = |at: usize| -> Option<(Vec<(f32, u32)>, usize)> {
                    let n = usize::try_from(r.int(at)).ok()?.min(64);
                    Some(((0..n).map(|k| (r.float(at + 1 + 2 * k), r.int(at + 2 + 2 * k) as u32)).collect(), at + 1 + 2 * n))
                };
                let (colours, next) = curve(34)?;
                let (head, _) = curve(next)?;
                Some(Visual::TParticle2 { length: r.float(14), particles: BParticles { damping: 1.0, colours, ..particles }, head })
            }
            1022 if r.payload.len() >= 16 => {
                let child = self.get(r.int(15)).filter(|c| c.kind == 1007).and_then(Nano::read)?;
                Some(Visual::Tracer3(Tracer3 { speed: r.float(10), colour: [r.float(11), r.float(12), r.float(13), r.float(14)], child }))
            }
            3036 if r.payload.len() >= 5 && r.int(2) != id => {
                let child = self.visual(r.int(2))?;
                let n = r.int(4).clamp(0, 64) as usize;
                Some(Visual::Toggle { child: Box::new(child), flags: r.int(0) as u32, mask: r.int(3) as u32, places: (0..n).map(|k| r.int(5 + k)).collect() })
            }
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
            2006 if r.payload.len() >= 32 && r.int(10) == 1 => Some(Visual::Electra {
                material: r.int(9),
                attach: r.int(7),
                colour_a: [r.float(18), r.float(19), r.float(20), r.float(21)],
                colour_b: [r.float(22), r.float(23), r.float(24), r.float(25)],
                duration: r.float(26),
                size: r.float(28),
                reach: r.float(29),
                life: r.int(30) as f32 / 1000.0,
            }),
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
