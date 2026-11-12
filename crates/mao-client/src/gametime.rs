//! The game's clock, as `GameTime_t` (Gamecode, constructor 0x1000afdb) keeps it: a day of 27 hours of 60 minutes of
//! 60 seconds (+0x80, +0x84, +0x88; an hour is 3600 s, +0x8c), dawn from hour 7 for 2 hours and dusk from 21 for 2
//! (+0x98, +0x9c, +0x90, +0x94), the clock running 15 game seconds a real second (+0x5c, `RunFunction` 0x1000b27e adds
//! `15 × dt` scaled by the day and night multipliers, both 1). The server's `GameTime` message sets it (`Update`,
//! 0x1000b590: the seconds into the day); until one comes, the real clock stands in, so a login is not always at midnight
//! (the original starts its clock at 0 and waits for the server).
//!
//! `day_intensity` is `GameTime_t::GetDayIntensity` (0x1000af23): 0 at night, rising through dawn by the hour and
//! minute, 1 by day, falling through dusk.

use std::time::Instant;

/// Seconds in a day, hours in a day, the clock's pace.
pub const DAY: f32 = 27.0 * 3600.0;
pub const HOURS: i32 = 27;
pub const PACE: f32 = 15.0;

/// The game's hour and minute as the game connection last knew them (`Clock::publish`), for `/played`.
static NOW: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn now() -> (u32, u32) {
    let minutes = NOW.load(std::sync::atomic::Ordering::Relaxed);
    (minutes / 60, minutes % 60)
}
const DAWN_START: i32 = 7;
const DAWN_LENGTH: i32 = 2;
const DUSK_START: i32 = 21;
const DUSK_LENGTH: i32 = 2;

/// The server's time less the real clock's, as the live clock was last told: for the windows, which have no clock at
/// hand (the missions' time left).
static SERVER_OFFSET: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// The server's time now (`N3Msg_GetServerSynceSystemTime`: `GameTime_t` +0xb4), the real clock's until a server says.
pub fn server_now() -> u32 {
    unix_now().wrapping_add(SERVER_OFFSET.load(std::sync::atomic::Ordering::Relaxed)) as u32
}

#[derive(Clone, Copy, Debug)]
pub struct Clock {
    /// The seconds into the day when the clock was last set, and when.
    seconds: f32,
    set: Instant,
    /// The day count (`GameTime_t` +0x4c, the weather's seed): the server's, or the days of the real clock at the pace.
    day: i32,
    /// The server's time less the real clock's (`GameTime_t` +0xb8, `Update`'s last argument less `_time64`).
    server_offset: i32,
}

impl Default for Clock {
    fn default() -> Self {
        // (the real clock, at the game's pace, until the server says)
        let real = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
        let game = real * PACE as f64;
        Clock { seconds: (game % DAY as f64) as f32, set: Instant::now(), day: (game / DAY as f64) as i32, server_offset: 0 }
    }
}

impl Clock {
    /// The server's word: `seconds` into the day (and the day count, when it says).
    pub fn set(&mut self, seconds: f32) {
        let mut s = seconds;
        while s < 0.0 {
            s += DAY;
        }
        self.seconds = s % DAY;
        self.set = Instant::now();
    }

    pub fn set_day(&mut self, day: i32) {
        self.day = day;
    }

    /// The server's word of its time (`GameTime_t::Update`'s last argument, seconds).
    pub fn set_server_time(&mut self, time: i32) {
        self.server_offset = time.wrapping_sub(unix_now());
        SERVER_OFFSET.store(self.server_offset, std::sync::atomic::Ordering::Relaxed);
    }

    /// The server's time now (`GameTime_t` +0xb4: `_time64` plus +0xb8, `RunFunction` each frame).
    pub fn server_time(&self) -> i32 {
        unix_now().wrapping_add(self.server_offset)
    }

    /// The server's time in whole seconds and the second's fraction (`GameTime_t` +0xc0 and +0xc4, which `RunFunction`
    /// counts on by the frames' times from `Update`'s; the real clock's here).
    pub fn server_clock(&self) -> (u32, f32) {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
        let t = now + self.server_offset as f64;
        (t.floor() as i64 as u32, (t - t.floor()) as f32)
    }

    /// The day count now (the days passed since the clock was set count on).
    pub fn day(&self) -> i32 {
        self.day + ((self.seconds + self.set.elapsed().as_secs_f32() * PACE) / DAY) as i32
    }

    /// The seconds into the day now.
    pub fn seconds(&self) -> f32 {
        (self.seconds + self.set.elapsed().as_secs_f32() * PACE) % DAY
    }

    /// The hour and minute now.
    pub fn time(&self) -> (i32, i32) {
        let s = self.seconds() as i32;
        ((s / 3600) % HOURS, (s / 60) % 60)
    }

    /// Notes the hour and minute for [`now`].
    pub fn publish(&self) {
        let (hour, minute) = self.time();
        NOW.store((hour * 60 + minute) as u32, std::sync::atomic::Ordering::Relaxed);
    }

    /// How much of the day's light there is now, 0 to 1.
    pub fn day_intensity(&self) -> f32 {
        let (hour, minute) = self.time();
        day_intensity(hour, minute)
    }
}

/// `_time64`, in seconds (the original keeps its low word).
fn unix_now() -> i32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i32)
}

/// `GameTime_t::GetDayIntensity`: the hour's place in dawn or dusk, with the minute's share.
pub fn day_intensity(hour: i32, minute: i32) -> f32 {
    let hour = hour.rem_euclid(HOURS);
    let minute_share = (minute as f32 / 60.0) / DAWN_LENGTH as f32;
    if hour < DAWN_START {
        0.0
    } else if hour < DAWN_START + DAWN_LENGTH {
        (hour - DAWN_START) as f32 / DAWN_LENGTH as f32 + minute_share
    } else if hour < DUSK_START {
        1.0
    } else if hour < DUSK_START + DUSK_LENGTH {
        1.0 - ((hour - DUSK_START) as f32 / DUSK_LENGTH as f32 + (minute as f32 / 60.0) / DUSK_LENGTH as f32)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_day_rises_at_dawn_and_falls_at_dusk() {
        assert_eq!(day_intensity(3, 0), 0.0);
        assert_eq!(day_intensity(7, 0), 0.0);
        assert_eq!(day_intensity(8, 0), 0.5);
        assert!((day_intensity(8, 30) - 0.75).abs() < 1e-6);
        assert_eq!(day_intensity(9, 0), 1.0);
        assert_eq!(day_intensity(20, 59), 1.0);
        assert_eq!(day_intensity(21, 0), 1.0);
        assert_eq!(day_intensity(22, 0), 0.5);
        assert_eq!(day_intensity(23, 0), 0.0);
        assert_eq!(day_intensity(26, 0), 0.0);
    }

    #[test]
    fn the_clock_runs_at_the_games_pace() {
        let mut c = Clock::default();
        c.set(9.0 * 3600.0);
        assert_eq!(c.time(), (9, 0));
        c.set(-1.0);
        assert_eq!(c.time(), (26, 59));
    }
}
