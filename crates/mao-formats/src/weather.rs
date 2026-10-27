//! The weather, as `Environment_t`'s weather object (Gamecode) rolls it: a deterministic schedule per area and day.
//!
//! - **The generator** (FUN_1013de47 / FUN_1013dec9 / FUN_1013de89) is an R250: a 250-word table copied from `.rdata`
//!   (0x101727f8), the seed added to the word at `seed & 0xff` (capped at 249) and that index kept as the current one;
//!   a step xors the word 147 back (or 103 ahead) into the current word, advances the index and gives the word, or the
//!   word over 2³² as a float.
//! - **The seeds** (FUN_100bce27): the sum of the environment bytes 19–25 and 8–18, the eight weather weights, the day
//!   count and −1, 0 or 1 (1 when that comes to nothing). The weights (FUN_100bd3b7) are the bytes 0–7, each doubled
//!   and raised by 100 when above 1.
//! - **The schedule** (FUN_100b1821): a period of 19440 s in three thirds of 6480; for each third, seeded with its own
//!   seed, states follow one another at random lengths of 120 to 1200 s until the third is full; each state then draws
//!   its own seed word, and from that 16 floats and 8 words of its own, read round in turn (FUN_100b138c / FUN_100b13b6,
//!   each starting at the second), and anew each time the state is entered (FUN_100b1277 resets the indices).
//! - **The roll** (FUN_100bcec6): a word modulo the weights' sum picks clear (weight 3) or fog (0, a random amount),
//!   mist (1, a random amount × 0.65), clouds (2, a random amount) or one of the four precipitations (4–7, full); the
//!   state fades in over `length × (0.25 + 0.75 × random)` (halved for a precipitation), the weather values scaled by
//!   the fade; then the wind (bytes 8–9, × 2.75, or the strongest precipitation × 33), the clouds raised to the other
//!   values, the darkness (bytes 10–11), the thunder and lightning chances (bytes 12–13, only under clouds past 0.3),
//!   the rest (bytes 14–18 over 3).
//!
//! `state` gives the values at a moment: the state being entered faded in over the one before it (the original keeps
//! a current and a next table and lerps them by the fade, FUN_100bd555 / FUN_100bf96e).

/// The weather values of a state, the first fourteen of `Environment_t`'s table.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Weather {
    /// 0 fog, 1 mist, 2 clouds, 3–6 the four precipitations, 7 wind, 8 darkness, 9 thunder, 10 lightning, 11–13 the rest.
    pub v: [f32; 14],
}

pub const PERIOD: f32 = 19440.0;
const THIRD: f32 = 6480.0;
const MIN_LENGTH: f32 = 120.0;
const MAX_LENGTH: f32 = 1200.0;

const TABLE: [u32; 250] = [
    0xf6230029, 0x26e16784, 0x20ae2cd6, 0xe7495f90, 0xa8bb5af1, 0xadb301eb, 0x923c12db, 0x963e390c,
    0x3d5e0124, 0xfa06491c, 0xe7de1547, 0xcf4d2d12, 0xedbb6443, 0x731f26a6, 0x7f7d7a5a, 0x24251238,
    0xd1d46e5d, 0x8a966bfc, 0x213b4e45, 0x6189260d, 0xa5db301c, 0x9b200732, 0x62ee2350, 0x96365878,
    0x28493e12, 0x479e3bf6, 0xa0dc5f49, 0xac14314f, 0x48404944, 0xf26b1cd0, 0x4cb74230, 0x37a12c3b,
    0x99223ef6, 0x99e1409d, 0xc3da121f, 0x349926ca, 0xca727bb9, 0xe92c7049, 0x8fc5187e, 0x93e93cd5,
    0xfaea5db2, 0xe85348cc, 0x83d65c67, 0x14d62f14, 0xdadc422d, 0x96830d66, 0x4d494657, 0xe3692fff,
    0xf3cd3a61, 0xc29d261e, 0x13721916, 0xb01d32e6, 0x354f0384, 0x93020677, 0x7c396be8, 0xa2cb1953,
    0x6c1e0e12, 0x289e7874, 0x89d511f4, 0xe8d45a9f, 0x277e2059, 0x453207cf, 0xc1cc1af4, 0x239001d3,
    0xd8d36048, 0xeee60975, 0xc02a591d, 0x62f71dc0, 0x93815078, 0x060e7b44, 0xff001850, 0x848d7f61,
    0x45050c7b, 0x443b3807, 0x3d1f7282, 0xee926270, 0x83545064, 0x72853bb1, 0x92156d69, 0x4c6a5c46,
    0x50731796, 0x301673d9, 0x8d684d67, 0x754a2cf7, 0x26575ed0, 0x70fa5876, 0xc21149bb, 0xd5244eae,
    0x2efe5579, 0x642548db, 0x9b3c0de5, 0xcbd35f45, 0x02ce0a28, 0x25c568f5, 0xb13d3459, 0x4a8a4027,
    0xf82d5e76, 0xa2c97ac2, 0x31d42668, 0x4179086a, 0x3a614e08, 0x68b17014, 0x51a50d6a, 0xf9c12528,
    0x05a954d6, 0xb3973087, 0x25f1412f, 0xd89a441d, 0xbb9b00c1, 0x877e4fc0, 0x989a5fa8, 0xf3c26486,
    0xcabf7a54, 0x33d92fe7, 0xde5579d1, 0xd6282a38, 0xcd6c10d9, 0x125e4c66, 0x8c3001e1, 0x6ad94efe,
    0xeae2159f, 0xb40c28e2, 0x024766b4, 0x202a4e38, 0xd2a91289, 0x677a2079, 0x55c20878, 0x992626b1,
    0x632927da, 0xc462113e, 0x51127296, 0x88394ebf, 0x30891d3f, 0x4b6d1ff1, 0x226c06e3, 0x2c1e36a1,
    0xdfcb721d, 0x363f1003, 0x08840607, 0x56057514, 0x0a28791b, 0xbfc56bc9, 0x468e212c, 0xef087a36,
    0xada84af3, 0xc5be78fe, 0x877100eb, 0x72d064a0, 0xbd061c75, 0x7287357e, 0xd84d6d73, 0xe68254be,
    0x11c243db, 0xd12b5841, 0x103003fa, 0xadf05a70, 0x35867954, 0x26aa1295, 0x7d132568, 0xeefc58e6,
    0x3fbe1eca, 0xf9890d9f, 0x991b0a41, 0x0b4f7cb8, 0x41721af6, 0xc3996014, 0x8f0d27d3, 0xac3a2044,
    0x1c6613a6, 0xd20b7833, 0x0a1052a1, 0x76ec3a4c, 0xb773134c, 0x49b43605, 0xe9aa4a0e, 0x9cd532cf,
    0xe62765ca, 0x5f9d31d8, 0x6af4194d, 0x0f593a27, 0x969c387c, 0xc7cd6af8, 0x53207987, 0xb1b87e64,
    0x74954987, 0x8b5a5ab0, 0xad2d214e, 0xec3d5ae7, 0x83de3260, 0x39ad2780, 0x3ae92d41, 0x8ba55e41,
    0x3ae4199f, 0x6d15749f, 0x8a260e00, 0x72d1424c, 0x24935804, 0x708f09b3, 0x1a402753, 0x76af328a,
    0xcc6f5cca, 0xf2d34ecf, 0x1fce0c95, 0x1c657cbe, 0x44f65d2a, 0x91a158ad, 0xd3a60665, 0x4ec007c9,
    0x20341b32, 0x394d012c, 0x37174d8f, 0xf6db2cc6, 0xa7a64fc8, 0xef88480b, 0x4f612738, 0x50124e68,
    0xea5403f9, 0x247177e7, 0x6a784e48, 0x51294a92, 0xa2296586, 0x3cdb4b72, 0x64fa561c, 0x9eb82237,
    0x789562b0, 0xdcdf0ef5, 0x9a6d726c, 0xe8d97374, 0x62651943, 0x9aa57a08, 0xe0bd5079, 0x981521eb,
    0x75e164e0, 0x366e3d8f, 0x54f46d7b, 0xb68a4962, 0x0127188f, 0xd5e501f7, 0x3b3b164a, 0x2b61569b,
    0xa0c62f84, 0x2da751b1, 0xca107c4a, 0x106f6275, 0x469e012f, 0x65102bef, 0xc4c80b9b, 0x7c292b43,
    0xa1f6448a, 0x80473b9e,
];

/// The R250 generator.
#[derive(Clone)]
pub struct R250 {
    table: [u32; 250],
    index: usize,
}

impl R250 {
    pub fn seeded(seed: u32) -> R250 {
        let mut table = TABLE;
        let i = ((seed & 0xff) as usize).min(0xf9);
        table[i] = table[i].wrapping_add(seed);
        R250 { table, index: i }
    }

    pub fn word(&mut self) -> u32 {
        let i = self.index;
        let j = if i < 0x93 { i + 0x67 } else { i - 0x93 };
        self.table[i] ^= self.table[j];
        let v = self.table[i];
        self.index = if i < 0xf9 { i + 1 } else { 0 };
        v
    }

    /// The word over 2³².
    pub fn float(&mut self) -> f32 {
        self.word() as f32 * 2.328_306_4e-10
    }
}

/// The weather weights of an environment: bytes 0–7, each doubled and raised by 100 when above 1.
pub fn weights(env: &[u8; 26]) -> [u32; 8] {
    let mut w = [0u32; 8];
    for k in 0..8 {
        let b = env[k] as u32;
        w[k] = if b > 1 { b * 2 + 100 } else { b };
    }
    w
}

/// FUN_100bce27: the seed for `which` (−1, 0 or 1).
pub fn seed(env: &[u8; 26], day: i32, which: i32) -> u32 {
    let w = weights(env);
    let mut s: i64 = w.iter().map(|&v| v as i64).sum();
    s += env[19..26].iter().map(|&b| b as i64).sum::<i64>();
    s += env[8..19].iter().map(|&b| b as i64).sum::<i64>();
    s += day as i64 + which as i64;
    if s == 0 {
        1
    } else {
        s as u32
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub start: f32,
    pub floats: [f32; 16],
    pub words: [u32; 8],
}

#[derive(Clone, Debug, Default)]
pub struct Schedule {
    pub entries: Vec<Entry>,
}

impl Schedule {
    /// FUN_100b1821 for an area's environment on a day.
    pub fn build(env: &[u8; 26], day: i32) -> Schedule {
        let seeds = [seed(env, day, -1), seed(env, day, 0), seed(env, day, 1)];
        let mut entries = Vec::new();
        for (third, &s) in seeds.iter().enumerate() {
            let mut rng = R250::seeded(s);
            let mut t = 0.0f32;
            let first = entries.len();
            while t < THIRD {
                t += rng.float() * (MAX_LENGTH - MIN_LENGTH) + MIN_LENGTH;
                if t < THIRD {
                    entries.push(Entry { start: third as f32 * THIRD + t, floats: [0.0; 16], words: [0; 8] });
                }
            }
            let seeds: Vec<u32> = (first..entries.len()).map(|_| rng.word()).collect();
            for (e, s) in entries[first..].iter_mut().zip(seeds) {
                let mut own = R250::seeded(s);
                for f in &mut e.floats {
                    *f = own.float();
                }
                for w in &mut e.words {
                    *w = own.word();
                }
            }
        }
        Schedule { entries }
    }

    /// The state in force at `t` seconds into the period and the one before it, with the time since the state began and
    /// its length: `(current, previous, elapsed, length)`.
    pub fn at(&self, t: f32) -> Option<(usize, usize, f32, f32)> {
        let n = self.entries.len();
        if n == 0 {
            return None;
        }
        let t = t.rem_euclid(PERIOD);
        let current = match self.entries.iter().rposition(|e| e.start <= t) {
            Some(i) => i,
            None => n - 1,
        };
        let previous = (current + n - 1) % n;
        let next = (current + 1) % n;
        let start = self.entries[current].start;
        let elapsed = (t - start).rem_euclid(PERIOD);
        let length = (self.entries[next].start - start).rem_euclid(PERIOD);
        Some((current, previous, elapsed, if length > 0.0 { length } else { PERIOD }))
    }
}

/// FUN_100bcec6: the weather of a state `elapsed` seconds in, and the fade weight it has reached.
pub fn roll(entry: &Entry, env: &[u8; 26], elapsed: f32, length: f32) -> (Weather, f32) {
    let w = weights(env);
    let sum: u32 = w.iter().sum::<u32>().max(1);
    let (mut fi, mut wi) = (0usize, 0usize);
    let mut next_f = || {
        fi = (fi + 1) & 15;
        entry.floats[fi]
    };
    let mut next_w = || {
        wi = (wi + 1) & 7;
        entry.words[wi]
    };
    let mut v = [0f32; 14];
    let mut factor = 1.0f32;
    let mut r = next_w() % sum;
    if r >= w[3] {
        r -= w[3];
        if r < w[0] {
            v[0] = next_f();
        } else {
            r -= w[0];
            if r < w[1] {
                v[1] = next_f() * 0.65;
            } else {
                r -= w[1];
                if r < w[2] {
                    v[2] = next_f();
                } else {
                    r -= w[2];
                    let mut hit = true;
                    if r < w[4] {
                        v[3] = 1.0;
                    } else {
                        r -= w[4];
                        if r < w[5] {
                            v[4] = 1.0;
                        } else {
                            r -= w[5];
                            if r < w[6] {
                                v[5] = 1.0;
                            } else if r - w[6] < w[7] {
                                v[6] = 1.0;
                            } else {
                                hit = false;
                            }
                        }
                    }
                    if hit {
                        factor = 0.5;
                    }
                }
            }
        }
    }
    let fade = (length * 0.25 + length * next_f() * 0.75) * factor;
    let weight = if elapsed <= fade {
        if elapsed <= 0.0 { 0.0 } else { elapsed / fade }
    } else {
        1.0
    };
    for x in &mut v[..7] {
        *x *= weight;
    }
    let b = |i: usize| env[i] as f32;
    v[7] = (next_f() * (b(9) - b(8)) * weight + b(8)) * 2.75;
    let strongest = if v[3] > 0.0 {
        v[3]
    } else if v[5] > 0.0 {
        v[5]
    } else if v[6] > 0.0 {
        v[6]
    } else {
        0.0
    };
    if strongest > 0.0 {
        v[7] = strongest * 33.0;
    }
    v[2] = v[2].max(strongest).max(v[4]).max(v[0]).max(v[1]);
    v[8] = next_f() * (b(10) - b(11)) * weight + b(11);
    v[9] = if v[2] > 0.3 && next_w() % 100 <= env[12] as u32 { next_f() * weight } else { 0.0 };
    v[10] = if v[2] > 0.3 && next_w() % 100 <= env[13] as u32 { next_f() * weight } else { 0.0 };
    v[11] = if next_w() % 100 <= env[14] as u32 { b(15) / 3.0 * next_f() * weight } else { 0.0 };
    v[12] = if next_w() % 100 <= env[16] as u32 { b(17) / 3.0 * next_f() * weight } else { 0.0 };
    v[13] = b(18) / 3.0 * next_f() * weight;
    (Weather { v }, weight)
}

/// The weather of an area at `seconds` into the day `day`: the state in force, faded in over the one before it. None
/// without weather weights.
pub fn state(env: &[u8; 26], day: i32, seconds: f32) -> Option<Weather> {
    if weights(env).iter().all(|&w| w == 0) {
        return None;
    }
    let schedule = Schedule::build(env, day);
    let (current, previous, elapsed, length) = schedule.at(seconds)?;
    let (now, weight) = roll(&schedule.entries[current], env, elapsed, length);
    if weight >= 1.0 || previous == current {
        return Some(now);
    }
    // the previous state at its full strength, fading out as this one fades in
    let prev_len = (schedule.entries[current].start - schedule.entries[previous].start).rem_euclid(PERIOD);
    let (before, _) = roll(&schedule.entries[previous], env, prev_len.max(1.0e9), prev_len);
    let mut v = now.v;
    for k in 0..14 {
        v[k] += before.v[k] * (1.0 - weight);
    }
    Some(Weather { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(weather: [u8; 8]) -> [u8; 26] {
        let mut e = [0u8; 26];
        e[..8].copy_from_slice(&weather);
        e[8] = 2;
        e[9] = 5;
        e[11] = 1;
        e[10] = 3;
        e
    }

    #[test]
    fn the_generator_is_deterministic_and_in_range() {
        let mut a = R250::seeded(12345);
        let mut b = R250::seeded(12345);
        let mut c = R250::seeded(12346);
        let mut same = 0;
        for _ in 0..1000 {
            let (x, y, z) = (a.word(), b.word(), c.word());
            assert_eq!(x, y);
            same += (x == z) as usize;
            let f = a.float();
            assert!((0.0..1.0).contains(&f));
            b.float();
        }
        assert!(same < 10, "{same}");
    }

    #[test]
    fn a_schedule_fills_the_period_with_states_of_the_right_length() {
        let s = Schedule::build(&env([1, 1, 30, 70, 10, 10, 5, 30]), 100);
        assert!(s.entries.len() > 20, "{}", s.entries.len());
        for w in s.entries.windows(2) {
            let d = w[1].start - w[0].start;
            assert!(d >= MIN_LENGTH - 1.0 && d <= MAX_LENGTH * 2.0, "{d}");
        }
        assert!(s.entries.last().unwrap().start < PERIOD);
        // the same day gives the same schedule; another day another
        let again = Schedule::build(&env([1, 1, 30, 70, 10, 10, 5, 30]), 100);
        assert_eq!(s.entries[3].words, again.entries[3].words);
        // (the seed only adds to one word of the generator's table, so a neighbouring day shifts little)
        let far = Schedule::build(&env([50, 1, 30, 70, 10, 10, 5, 30]), 100);
        assert_ne!(s.entries[3].words, far.entries[3].words);
    }

    #[test]
    fn the_state_fades_in_and_the_values_stay_sane() {
        let e = env([1, 1, 30, 70, 10, 10, 5, 30]);
        let mut kinds = [0usize; 7];
        for day in 0..5 {
            for step in 0..200 {
                let w = state(&e, day, step as f32 * 97.0).unwrap();
                for (k, x) in w.v[..7].iter().enumerate() {
                    assert!((0.0..=1.0001).contains(x), "{:?}", w.v);
                    if *x > 0.5 {
                        kinds[k] += 1;
                    }
                }
                assert!(w.v[2] >= w.v[0] && w.v[2] >= w.v[1], "{:?}", w.v);
            }
        }
        // clear is the likeliest but the others come round
        assert!(kinds.iter().skip(2).any(|&n| n > 0), "{kinds:?}");
        assert!(state(&[0u8; 26], 1, 10.0).is_none());
    }
}
