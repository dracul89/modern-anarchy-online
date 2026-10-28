//! Gamecode's suns (the environment's +0xc4 object): FUN_100b8c22 makes it (FUN_100b93fe with 176, 45, 90, the day's
//! turn 0.0037037 degrees a second, 70, 0, 145, 0, six times the turn, 5 and 15), FUN_100b959c moves it to the clock's
//! seconds into the day (`GameTime_t::GetCurrentRealTime`), and FUN_100b8cf3 gives the environment the two suns'
//! rotations (`GameSun1Rotation`, `GameSun2Rotation`: FUN_1013c747's look rotation toward each sun, up (0, 1, 0), made a
//! quaternion by FUN_1013c83b), which the scripts turn the sun rays, the moons and the lights by.
//!
//! The model: a point 70 out along z turned by the day's angle about y (FUN_100b9299's yaw) after a tilt of 45° then 90°
//! about x; each sun 5 (the first) or −15 (the second) out along z from it, turned by six times the day's angle after a
//! tilt of 145° about x. Matrices are 3 by 3, row by row, applied to row vectors (FUN_100b939f), multiplied left to right
//! (FUN_1013ccea, FUN_1013cbdb).

use super::math::{r32, Q4, V3};

type M9 = [f32; 9];

/// The degrees made radians as the constructor does (π of the double 3.1415927410125732, over 180).
fn rad(deg: f32) -> f32 {
    r32(deg as f64 * 3.1415927410125732 / 180.0)
}

/// FUN_100b9299: the rotation of angles `x` (pitch), `y` (yaw) and `z` (roll).
fn euler(x: f32, y: f32, z: f32) -> M9 {
    let (cx, cy, cz) = (r32((x as f64).cos()), r32((y as f64).cos()), r32((z as f64).cos()));
    let (sx, sy, sz) = (r32((x as f64).sin()), r32((y as f64).sin()), r32((z as f64).sin()));
    let d = |v: f32| v as f64;
    let czcy = r32(d(cz) * d(cy));
    let szcy = r32(d(sz) * d(cy));
    let sycz = r32(d(sy) * d(cz));
    let szsy = r32(d(sz) * d(sy));
    [
        r32(d(szsy) * d(sx) + d(czcy)),
        r32(d(sycz) * d(sx) - d(szcy)),
        r32(d(sy) * d(cx)),
        r32(d(sz) * d(cx)),
        r32(d(cz) * d(cx)),
        -sx,
        r32(d(szcy) * d(sx) - d(sycz)),
        r32(d(czcy) * d(sx) + d(szsy)),
        r32(d(cx) * d(cy)),
    ]
}

/// FUN_1013ccea / FUN_1013cbdb: `a × b`.
fn mul(a: &M9, b: &M9) -> M9 {
    let d = |v: f32| v as f64;
    let mut m = [0f32; 9];
    for r in 0..3 {
        for c in 0..3 {
            m[r * 3 + c] = r32(d(b[6 + c]) * d(a[r * 3 + 2]) + d(b[c]) * d(a[r * 3]) + d(a[r * 3 + 1]) * d(b[3 + c]));
        }
    }
    m
}

/// FUN_100b939f: the row vector `v` by `m`.
fn apply(v: V3, m: &M9) -> V3 {
    let d = |v: f32| v as f64;
    [
        r32(d(m[6]) * d(v[2]) + d(m[0]) * d(v[0]) + d(m[3]) * d(v[1])),
        r32(d(m[7]) * d(v[2]) + d(m[4]) * d(v[1]) + d(m[1]) * d(v[0])),
        r32(d(m[8]) * d(v[2]) + d(m[5]) * d(v[1]) + d(m[2]) * d(v[0])),
    ]
}

/// The angle wrapped below 2π (the double 6.2831854820251465) as FUN_100b959c's loop does.
fn wrap(mut a: f32) -> f32 {
    let turn = 6.2831854820251465f64;
    while a as f64 >= turn {
        a = r32(a as f64 - turn);
    }
    a
}

/// The suns' places (the environment's +0x2c and +0x38) at `t` seconds into the day.
pub fn places(t: f32) -> (V3, V3) {
    let turn = 0.003703703638166189f32;
    let (p0, p3, p8) = (rad(176.0), rad(turn), rad(turn * 6.0));
    let tilt = mul(&euler(rad(45.0), 0.0, 0.0), &euler(rad(90.0), 0.0, 0.0));
    let suns = mul(&euler(rad(145.0), 0.0, 0.0), &euler(rad(0.0), 0.0, 0.0));
    let a = wrap(r32(p3 as f64 * t as f64 + p0 as f64));
    let planet = apply([0.0, 0.0, 70.0], &mul(&euler(0.0, a, 0.0), &tilt));
    let b = wrap(r32(p8 as f64 * t as f64));
    let u = mul(&euler(0.0, b, 0.0), &suns);
    let add = |w: V3| [0, 1, 2].map(|k| r32(w[k] as f64 + planet[k] as f64));
    (add(apply([0.0, 0.0, 5.0], &u)), add(apply([0.0, 0.0, -15.0], &u)))
}

/// FUN_1013c747 then FUN_1013c83b: the rotation turning +z to `forward` (normalised) with `up` keeping its side: the
/// rows `forward × up`, `forward × that`, `forward`, made a quaternion (x, y, z, w) of the matrix's transpose.
pub fn look(forward: V3, up: V3) -> Q4 {
    let cross = |a: V3, b: V3| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let unit = |v: V3| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if l > 0.0 { v.map(|c| c / l) } else { v }
    };
    let r = cross(forward, up);
    if r == [0.0; 3] && forward == [0.0; 3] {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let r = unit(r);
    let f = unit(forward);
    let u = cross(f, r);
    // (the rotation's columns are the rows: x → r, y → u, z → f)
    let m = [[r[0], u[0], f[0]], [r[1], u[1], f[1]], [r[2], u[2], f[2]]];
    let trace = m[0][0] + m[1][1] + m[2][2];
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt();
        let w = s * 0.5;
        let k = 0.5 / s;
        [(m[2][1] - m[1][2]) * k, (m[0][2] - m[2][0]) * k, (m[1][0] - m[0][1]) * k, w]
    } else {
        let i = if m[1][1] > m[0][0] { if m[2][2] > m[1][1] { 2 } else { 1 } } else if m[2][2] > m[0][0] { 2 } else { 0 };
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        let s = (m[i][i] - m[j][j] - m[k][k] + 1.0).sqrt();
        let mut q = [0f32; 4];
        q[i] = s * 0.5;
        let h = 0.5 / s;
        q[3] = (m[k][j] - m[j][k]) * h;
        q[j] = (m[j][i] + m[i][j]) * h;
        q[k] = (m[k][i] + m[i][k]) * h;
        q
    }
}

/// FUN_100b8cf3: the two suns' rotations at `t` seconds into the day: each the look rotation toward the sun's place
/// negated (its light's way), up (0, 1, 0).
pub fn rotations(t: f32) -> (Q4, Q4) {
    let (s1, s2) = places(t);
    let neg = |v: V3| v.map(|c| -c);
    (look(neg(s1), [0.0, 1.0, 0.0]), look(neg(s2), [0.0, 1.0, 0.0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_suns_rise_and_set() {
        // (the first sun's place over the day: up (y) by day, down at night; its rotation turns +z to its light's way)
        let height = |hour: f32| {
            let (s, _) = places(hour * 3600.0);
            s[1] / (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
        };
        let heights: Vec<f32> = (0..27).map(|h| height(h as f32)).collect();
        eprintln!("sun heights by the hour: {heights:?}");
        let (q, _) = rotations(12.0 * 3600.0);
        let mut v = [0.0, 0.0, 1.0];
        super::super::math::rotate_by(&mut v, &q);
        let (s, _) = places(12.0 * 3600.0);
        let l = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
        for k in 0..3 {
            assert!((v[k] + s[k] / l).abs() < 1e-4, "{v:?} {s:?}");
        }
    }
}
