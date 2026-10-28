//! The vector, quaternion and matrix helpers the scripts' values use, each step as FXS.dll's x87 code does it (sums in
//! the order it adds, rounded to floats where it stores them), as `AnarchyReimagined`'s `ao-fmath` `vecmath.rs` has
//! them; and the x87 steps they rest on (`x87.rs`: 53-bit precision, so single operations are `f64` arithmetic, `FISTP`
//! rounding to nearest even). Matrices are 16 floats, row by row, the translation in the last row. (The CRT's `_CIsin`
//! and `_CIcos` are the platform's here.)

/// A value stored to a float.
#[inline]
pub fn r32(x: f64) -> f32 {
    x as f32
}

/// `FISTP` to a 32-bit integer: round to nearest even; out of range or NaN is the integer indefinite, `0x80000000`.
pub fn fistp(x: f64) -> i32 {
    if x.is_nan() {
        return i32::MIN;
    }
    let r = x.round_ties_even();
    if !(-2147483648.0..2147483648.0).contains(&r) {
        return i32::MIN;
    }
    r as i32
}

/// The original's rounding of a float to an integer: `FISTP(x - 0.49999)`.
#[inline]
pub fn round(x: f32) -> i32 {
    fistp(x as f64 - 0.49999)
}

#[inline]
pub fn u2f(u: u32) -> f64 {
    u as f64
}

pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

fn sin(x: f64) -> f64 {
    x.sin()
}

fn cos(x: f64) -> f64 {
    x.cos()
}

/// `_CIfmod`.
pub fn fmod(x: f64, y: f64) -> f64 {
    x % y
}

pub type V3 = [f32; 3];
pub type Q4 = [f32; 4];
pub type M16 = [f32; 16];

pub const IDENTITY: M16 = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

fn d(x: f32) -> f64 {
    x as f64
}

/// FUN_100024d2: the squared length.
pub fn len2(v: &V3) -> f32 {
    r32((d(v[0]) * d(v[0]) + d(v[1]) * d(v[1])) + d(v[2]) * d(v[2]))
}

/// FUN_1000540c: one over the length.
pub fn inv_len(v: &V3) -> f32 {
    let l = r32(sqrt(d(len2(v))));
    r32(1.0 / d(l))
}

/// FUN_10008b54: scaled to length `s` (in place).
pub fn set_len(v: &mut V3, s: f32) {
    let k = r32(d(inv_len(v)) * d(s));
    for c in v.iter_mut() {
        *c = r32(d(*c) * d(k));
    }
}

/// FUN_10008b85: a copy scaled to length `s`.
pub fn with_len(v: &V3, s: f32) -> V3 {
    let mut t = *v;
    set_len(&mut t, s);
    t
}

/// FUN_10001ce8: `a + b`.
pub fn add(a: &V3, b: &V3) -> V3 {
    [r32(d(b[0]) + d(a[0])), r32(d(b[1]) + d(a[1])), r32(d(b[2]) + d(a[2]))]
}

/// FUN_10001d26: `a - b`.
pub fn sub(a: &V3, b: &V3) -> V3 {
    [r32(d(a[0]) - d(b[0])), r32(d(a[1]) - d(b[1])), r32(d(a[2]) - d(b[2]))]
}

/// FUN_10001d64: `a * s`.
pub fn scale(a: &V3, s: f32) -> V3 {
    [r32(d(a[0]) * d(s)), r32(d(a[1]) * d(s)), r32(d(a[2]) * d(s))]
}

/// FUN_10004e55: `a = a x b` (in place).
pub fn cross_in(a: &mut V3, b: &V3) {
    let x = r32(d(b[2]) * d(a[1]) - d(b[1]) * d(a[2]));
    let y = r32(d(a[2]) * d(b[0]) - d(a[0]) * d(b[2]));
    a[2] = r32(d(b[1]) * d(a[0]) - d(b[0]) * d(a[1]));
    a[0] = x;
    a[1] = y;
}

/// FUN_10004e9c: `a x b`.
pub fn cross(a: &V3, b: &V3) -> V3 {
    let mut t = *a;
    cross_in(&mut t, b);
    t
}

/// FUN_10004ec7: all three zero (ordered).
pub fn is_zero(v: &V3) -> bool {
    v[0] == 0.0 && v[1] == 0.0 && v[2] == 0.0
}

/// FUN_10004ddd.
pub fn v_eq(a: &V3, b: &V3) -> bool {
    a[0] == b[0] && a[1] == b[1] && a[2] == b[2]
}

/// FUN_10004efb (w compared first).
pub fn q_eq(a: &Q4, b: &Q4) -> bool {
    a[3] == b[3] && a[0] == b[0] && a[1] == b[1] && a[2] == b[2]
}

/// FUN_1000f35d.
pub fn m_eq(a: &M16, b: &M16) -> bool {
    a.iter().zip(b).all(|(x, y)| x == y)
}

/// FUN_1000502c: the rotation turning `a` toward `b`: their half-way
/// direction (or, when `a + b` is nearly zero, `(a.z, 0, -a.x)`)
/// normalized, then crossed and dotted with `a`.
pub fn rotation_between(a: &V3, b: &V3) -> Q4 {
    let mut h = add(a, b);
    let mut l2 = len2(&h);
    if 1e-12f32 > l2 {
        h = [a[2], 0.0, -a[0]];
        l2 = r32(d(h[2]) * d(h[2]) + d(h[0]) * d(h[0]));
    }
    let inv = r32(1.0 / d(r32(sqrt(d(l2)))));
    for c in h.iter_mut() {
        *c = r32(d(inv) * d(*c));
    }
    [
        r32(d(a[1]) * d(h[2]) - d(a[2]) * d(h[1])),
        r32(d(a[2]) * d(h[0]) - d(a[0]) * d(h[2])),
        r32(d(a[0]) * d(h[1]) - d(a[1]) * d(h[0])),
        r32((d(a[1]) * d(h[1]) + d(a[0]) * d(h[0])) + d(a[2]) * d(h[2])),
    ]
}

/// FUN_100054af: `v` rotated by the unit quaternion `q` (in place).
pub fn rotate_by(v: &mut V3, q: &Q4) {
    let u = [q[0], q[1], q[2]];
    let mut c1 = cross(&u, v);
    let mut c2 = cross(&u, &c1);
    let w2 = r32(d(q[3]) * 2.0);
    for c in c1.iter_mut() {
        *c = r32(d(*c) * d(w2));
    }
    for c in c2.iter_mut() {
        *c = r32(d(*c) * 2.0);
    }
    for k in 0..3 {
        let t = r32(d(v[k]) + d(c1[k]));
        v[k] = r32(d(t) + d(c2[k]));
    }
}

/// FUN_10005447: the point `v` transformed by `m` (in place).
pub fn transform(v: &mut V3, m: &M16) {
    let x = d(v[0]);
    let y = d(v[1]);
    let z = d(v[2]);
    let ny = r32(((d(m[1]) * x + d(m[5]) * y) + d(m[9]) * z) + d(m[13]));
    let nz = r32(((d(m[2]) * x + d(m[6]) * y) + d(m[10]) * z) + d(m[14]));
    v[0] = r32(((d(m[4]) * y + d(m[0]) * x) + d(m[8]) * z) + d(m[12]));
    v[1] = ny;
    v[2] = nz;
}

/// FUN_10004f91: `a = a * b` (in place).
pub fn q_mul_in(a: &mut Q4, b: &Q4) {
    let (ax, ay, az, aw) = (d(a[0]), d(a[1]), d(a[2]), d(a[3]));
    let (bx, by, bz, bw) = (d(b[0]), d(b[1]), d(b[2]), d(b[3]));
    let x = r32(((aw * bx + by * az) - bz * ay) + ax * bw);
    let y = r32(((aw * by - bx * az) + bz * ax) + ay * bw);
    let z = r32(((ay * bx - ax * by) + bz * aw) + bw * az);
    a[3] = r32(aw * bw - ((ay * by + bx * ax) + bz * az));
    a[0] = x;
    a[1] = y;
    a[2] = z;
}

/// FUN_100098e8: the rotation by `angle` (radians) about `axis`: half the
/// angle (wrapped into a turn when outside [0, 2pi)), its sine scaling the
/// axis, its cosine the w.
pub fn axis_angle(axis: &V3, angle: f32) -> Q4 {
    let a = d(angle);
    let half = if a < 0.0 || a >= std::f64::consts::TAU {
        let t = r32(a / std::f32::consts::TAU as f64);
        let fl = r32(d(t).floor());
        let frac = r32(d(t) - d(fl));
        r32(d(frac) * std::f32::consts::PI as f64)
    } else {
        r32(a * 0.5)
    };
    let s = r32(sin(d(half)));
    [r32(d(axis[0]) * d(s)), r32(d(axis[1]) * d(s)), r32(d(axis[2]) * d(s)), r32(cos(d(half)))]
}

/// FUN_1000514b: each row's columns scaled by `(sx, sy, sz)` (the
/// translation row too; the last column kept).
pub fn m_scale(m: &mut M16, s: &V3) {
    for row in 0..4 {
        for col in 0..3 {
            m[row * 4 + col] = r32(d(m[row * 4 + col]) * d(s[col]));
        }
    }
}

fn sincos(angle: f32) -> (f64, f64) {
    (d(r32(cos(d(angle)))), d(r32(sin(d(angle)))))
}

/// FUN_100051f0: the original's x rotation (which writes the second and
/// third rows from the first and second, as it does).
pub fn m_rotate_x(m: &mut M16, angle: f32) {
    let (c, s) = sincos(angle);
    let (t5, t6, t9) = (d(m[5]), d(m[6]), d(m[9]));
    m[4] = r32(c * d(m[1]) - s * d(m[2]));
    m[5] = r32(c * d(m[5]) - s * d(m[6]));
    m[6] = r32(c * d(m[9]) - s * d(m[10]));
    m[8] = r32(s * d(m[1]) + c * d(m[2]));
    m[9] = r32(s * t5 + c * t6);
    m[10] = r32(s * t9 + c * d(m[10]));
}

/// FUN_100052a2: the original's y rotation.
pub fn m_rotate_y(m: &mut M16, angle: f32) {
    let (c, s) = sincos(angle);
    let (t0, t2, t8) = (d(m[0]), d(m[2]), d(m[8]));
    m[0] = r32(c * d(m[0]) - s * d(m[2]));
    m[1] = r32(c * d(m[4]) - s * d(m[6]));
    m[2] = r32(c * d(m[8]) - s * d(m[10]));
    m[8] = r32(s * t0 + c * t2);
    m[9] = r32(c * d(m[6]) + s * d(m[4]));
    m[10] = r32(c * d(m[10]) + s * t8);
}

/// FUN_10005357: the original's z rotation.
pub fn m_rotate_z(m: &mut M16, angle: f32) {
    let (c, s) = sincos(angle);
    let (t0, t1, t4) = (d(m[0]), d(m[1]), d(m[4]));
    m[0] = r32(c * d(m[0]) - s * d(m[1]));
    m[1] = r32(c * d(m[4]) - s * d(m[5]));
    m[2] = r32(c * d(m[8]) - s * d(m[9]));
    m[4] = r32(s * t0 + c * t1);
    m[5] = r32(c * d(m[5]) + s * t4);
    m[6] = r32(c * d(m[9]) + s * d(m[8]));
}

/// FUN_10005118: the translation row moved by `t`.
pub fn m_translate(m: &mut M16, t: &V3) {
    m[12] = r32(d(m[12]) + d(t[0]));
    m[13] = r32(d(t[1]) + d(m[13]));
    m[14] = r32(d(m[14]) + d(t[2]));
}

/// FUN_1000f29d: `m = m * n`.
pub fn m_mul_in(m: &mut M16, n: &M16) {
    let mut r = IDENTITY;
    for i in 0..4 {
        for j in 0..4 {
            let a = &m[i * 4..i * 4 + 4];
            r[i * 4 + j] = r32(((d(a[0]) * d(n[j]) + d(n[4 + j]) * d(a[1])) + d(n[8 + j]) * d(a[2])) + d(n[12 + j]) * d(a[3]));
        }
    }
    *m = r;
}

/// FUN_1000fbf7 (with FUN_1000f39d): the rotation matrix of a quaternion.
pub fn q_matrix(q: &Q4) -> M16 {
    let (x, y, z, w) = (d(q[0]), d(q[1]), d(q[2]), d(q[3]));
    let (x2, y2, z2) = (x * 2.0, y * 2.0, z * 2.0);
    let xx2 = d(r32(x * x2));
    let yy2 = d(r32(y * y2));
    let zz2 = d(r32(z * z2));
    let xy2 = d(r32(y * x2));
    let xz2 = d(r32(z * x2));
    let xw2 = d(r32(w * x2));
    let yz2 = d(r32(z * y2));
    let yw2 = d(r32(w * y2));
    let zw2 = d(r32(z2 * w));
    let one_xx = 1.0 - xx2;
    let mut m = [0f32; 16];
    m[0] = r32((1.0 - yy2) - zz2);
    m[1] = r32(zw2 + xy2);
    m[2] = r32(xz2 - yw2);
    m[4] = r32(xy2 - zw2);
    m[5] = r32(one_xx - zz2);
    m[6] = r32(yz2 + xw2);
    m[8] = r32(yw2 + xz2);
    m[9] = r32(yz2 - xw2);
    m[10] = r32(one_xx - yy2);
    m[15] = 1.0;
    m
}
