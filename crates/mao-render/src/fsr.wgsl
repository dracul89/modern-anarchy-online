// AMD FidelityFX Super Resolution 1.0 (spatial upscaling): EASU, an
// edge-adaptive upscale, then RCAS, a robust contrast-adaptive sharpen.
// Ported to WGSL from AMD's ffx_fsr1.h (v1.20210629), which is
// Copyright (c) 2021 Advanced Micro Devices, Inc., MIT licence:
//   Permission is hereby granted, free of charge, to any person obtaining a
//   copy of this software and associated documentation files (the "Software"),
//   to deal in the Software without restriction, including without limitation
//   the rights to use, copy, modify, merge, publish, distribute, sublicense,
//   and / or sell copies of the Software, and to permit persons to whom the
//   Software is furnished to do so, subject to the following conditions: the
//   above copyright notice and this permission notice shall be included in all
//   copies or substantial portions of the Software. THE SOFTWARE IS PROVIDED
//   "AS IS", WITHOUT WARRANTY OF ANY KIND.
//
// The frames are sRGB targets (reads give linear values, writes encode): the
// filters work on a gamma-2 version of them, as FSR expects.

struct Params {
    con0: vec4<f32>,
    con1: vec4<f32>,
    con2: vec4<f32>,
    con3: vec4<f32>,
    // x: the sharpening amount (1 = maximum, halved per stop).
    rcas: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@vertex
fn vs_fsr(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

fn rcp_lo(a: f32) -> f32 {
    return bitcast<f32>(0x7ef07ebbu - bitcast<u32>(a));
}

fn rsq_lo(a: f32) -> f32 {
    return bitcast<f32>(0x5f347d74u - (bitcast<u32>(a) >> 1u));
}

fn rcp_med(a: f32) -> f32 {
    let b = bitcast<f32>(0x7ef19fffu - bitcast<u32>(a));
    return b * (-b * a + 2.0);
}

fn gamma(c: vec4<f32>) -> vec4<f32> {
    return sqrt(max(c, vec4<f32>(0.0)));
}

fn easu_tap(acc: ptr<function, vec3<f32>>, weight: ptr<function, f32>, off: vec2<f32>, dir: vec2<f32>, len: vec2<f32>, lob: f32, clp: f32, c: vec3<f32>) {
    // Rotate the offset by the direction, then stretch it.
    var v = vec2<f32>(off.x * dir.x + off.y * dir.y, off.x * (-dir.y) + off.y * dir.x);
    v = v * len;
    var d2 = v.x * v.x + v.y * v.y;
    d2 = min(d2, clp);
    // An approximation of lanczos2 without sin(), rcp() or sqrt().
    var wb = 0.4 * d2 - 1.0;
    var wa = lob * d2 - 1.0;
    wb = wb * wb;
    wa = wa * wa;
    wb = 25.0 / 16.0 * wb - (25.0 / 16.0 - 1.0);
    let w = wb * wa;
    *acc = *acc + c * w;
    *weight = *weight + w;
}

fn easu_set(dir: ptr<function, vec2<f32>>, len: ptr<function, f32>, pp: vec2<f32>, bi_s: bool, bi_t: bool, bi_u: bool, bi_v: bool, la: f32, lb: f32, lc: f32, ld: f32, le: f32) {
    var w = 0.0;
    if bi_s { w = (1.0 - pp.x) * (1.0 - pp.y); }
    if bi_t { w = pp.x * (1.0 - pp.y); }
    if bi_u { w = (1.0 - pp.x) * pp.y; }
    if bi_v { w = pp.x * pp.y; }
    let dc = ld - lc;
    let cb = lc - lb;
    var len_x = max(abs(dc), abs(cb));
    len_x = rcp_lo(len_x);
    let dir_x = ld - lb;
    (*dir).x = (*dir).x + dir_x * w;
    len_x = clamp(abs(dir_x) * len_x, 0.0, 1.0);
    len_x = len_x * len_x;
    *len = *len + len_x * w;
    let ec = le - lc;
    let ca = lc - la;
    var len_y = max(abs(ec), abs(ca));
    len_y = rcp_lo(len_y);
    let dir_y = le - la;
    (*dir).y = (*dir).y + dir_y * w;
    len_y = clamp(abs(dir_y) * len_y, 0.0, 1.0);
    len_y = len_y * len_y;
    *len = *len + len_y * w;
}

@fragment
fn fs_easu(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let ip = floor(frag.xy);
    // The position of 'f'.
    var pp = ip * params.con0.xy + params.con0.zw;
    let fp = floor(pp);
    pp = pp - fp;
    // The 12-tap kernel:   b c / e f g h / i j k l / n o
    let p0 = fp * params.con1.xy + params.con1.zw;
    let p1 = p0 + params.con2.xy;
    let p2 = p0 + params.con2.zw;
    let p3 = p0 + params.con3.xy;
    let bczz_r = gamma(textureGather(0, source, samp, p0));
    let bczz_g = gamma(textureGather(1, source, samp, p0));
    let bczz_b = gamma(textureGather(2, source, samp, p0));
    let ijfe_r = gamma(textureGather(0, source, samp, p1));
    let ijfe_g = gamma(textureGather(1, source, samp, p1));
    let ijfe_b = gamma(textureGather(2, source, samp, p1));
    let klhg_r = gamma(textureGather(0, source, samp, p2));
    let klhg_g = gamma(textureGather(1, source, samp, p2));
    let klhg_b = gamma(textureGather(2, source, samp, p2));
    let zzon_r = gamma(textureGather(0, source, samp, p3));
    let zzon_g = gamma(textureGather(1, source, samp, p3));
    let zzon_b = gamma(textureGather(2, source, samp, p3));
    // The simplest luma there is (times 2).
    let bczz_l = bczz_b * 0.5 + (bczz_r * 0.5 + bczz_g);
    let ijfe_l = ijfe_b * 0.5 + (ijfe_r * 0.5 + ijfe_g);
    let klhg_l = klhg_b * 0.5 + (klhg_r * 0.5 + klhg_g);
    let zzon_l = zzon_b * 0.5 + (zzon_r * 0.5 + zzon_g);
    let bl = bczz_l.x;
    let cl = bczz_l.y;
    let il = ijfe_l.x;
    let jl = ijfe_l.y;
    let fl = ijfe_l.z;
    let el = ijfe_l.w;
    let kl = klhg_l.x;
    let ll = klhg_l.y;
    let hl = klhg_l.z;
    let gl = klhg_l.w;
    let ol = zzon_l.z;
    let nl = zzon_l.w;
    // Accumulate the gradient direction and length over the bilinear weights.
    var dir = vec2<f32>(0.0);
    var len = 0.0;
    easu_set(&dir, &len, pp, true, false, false, false, bl, el, fl, gl, jl);
    easu_set(&dir, &len, pp, false, true, false, false, cl, fl, gl, hl, kl);
    easu_set(&dir, &len, pp, false, false, true, false, fl, il, jl, kl, nl);
    easu_set(&dir, &len, pp, false, false, false, true, gl, jl, kl, ll, ol);
    // Normalise, and clean up near zero.
    let dir2 = dir * dir;
    var dir_r = dir2.x + dir2.y;
    let zro = dir_r < 1.0 / 32768.0;
    dir_r = rsq_lo(dir_r);
    dir_r = select(dir_r, 1.0, zro);
    dir.x = select(dir.x, 1.0, zro);
    dir = dir * dir_r;
    len = len * 0.5;
    len = len * len;
    // Stretch the kernel: 1 vertical or horizontal, up to sqrt(2) on a diagonal.
    let stretch = (dir.x * dir.x + dir.y * dir.y) * rcp_lo(max(abs(dir.x), abs(dir.y)));
    let len2 = vec2<f32>(1.0 + (stretch - 1.0) * len, 1.0 - 0.5 * len);
    // The window shifts with the amount of edge.
    let lob = 0.5 + ((1.0 / 4.0 - 0.04) - 0.5) * len;
    let clp = rcp_lo(lob);
    // The accumulation is limited by the min and max of the 4 nearest.
    let f_c = vec3<f32>(ijfe_r.z, ijfe_g.z, ijfe_b.z);
    let g_c = vec3<f32>(klhg_r.w, klhg_g.w, klhg_b.w);
    let j_c = vec3<f32>(ijfe_r.y, ijfe_g.y, ijfe_b.y);
    let k_c = vec3<f32>(klhg_r.x, klhg_g.x, klhg_b.x);
    let min4 = min(min(min(f_c, g_c), j_c), k_c);
    let max4 = max(max(max(f_c, g_c), j_c), k_c);
    var acc = vec3<f32>(0.0);
    var weight = 0.0;
    easu_tap(&acc, &weight, vec2<f32>(0.0, -1.0) - pp, dir, len2, lob, clp, vec3<f32>(bczz_r.x, bczz_g.x, bczz_b.x));
    easu_tap(&acc, &weight, vec2<f32>(1.0, -1.0) - pp, dir, len2, lob, clp, vec3<f32>(bczz_r.y, bczz_g.y, bczz_b.y));
    easu_tap(&acc, &weight, vec2<f32>(-1.0, 1.0) - pp, dir, len2, lob, clp, vec3<f32>(ijfe_r.x, ijfe_g.x, ijfe_b.x));
    easu_tap(&acc, &weight, vec2<f32>(0.0, 1.0) - pp, dir, len2, lob, clp, vec3<f32>(ijfe_r.y, ijfe_g.y, ijfe_b.y));
    easu_tap(&acc, &weight, vec2<f32>(0.0, 0.0) - pp, dir, len2, lob, clp, vec3<f32>(ijfe_r.z, ijfe_g.z, ijfe_b.z));
    easu_tap(&acc, &weight, vec2<f32>(-1.0, 0.0) - pp, dir, len2, lob, clp, vec3<f32>(ijfe_r.w, ijfe_g.w, ijfe_b.w));
    easu_tap(&acc, &weight, vec2<f32>(1.0, 1.0) - pp, dir, len2, lob, clp, vec3<f32>(klhg_r.x, klhg_g.x, klhg_b.x));
    easu_tap(&acc, &weight, vec2<f32>(2.0, 1.0) - pp, dir, len2, lob, clp, vec3<f32>(klhg_r.y, klhg_g.y, klhg_b.y));
    easu_tap(&acc, &weight, vec2<f32>(2.0, 0.0) - pp, dir, len2, lob, clp, vec3<f32>(klhg_r.z, klhg_g.z, klhg_b.z));
    easu_tap(&acc, &weight, vec2<f32>(1.0, 0.0) - pp, dir, len2, lob, clp, vec3<f32>(klhg_r.w, klhg_g.w, klhg_b.w));
    easu_tap(&acc, &weight, vec2<f32>(1.0, 2.0) - pp, dir, len2, lob, clp, vec3<f32>(zzon_r.z, zzon_g.z, zzon_b.z));
    easu_tap(&acc, &weight, vec2<f32>(0.0, 2.0) - pp, dir, len2, lob, clp, vec3<f32>(zzon_r.w, zzon_g.w, zzon_b.w));
    // Normalise and de-ring; back to linear for the sRGB target to encode.
    let pix = min(max4, max(min4, acc * (1.0 / weight)));
    return vec4<f32>(pix * pix, 1.0);
}

fn load_gamma(p: vec2<i32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(source));
    return sqrt(max(textureLoad(source, clamp(p, vec2<i32>(0), size - vec2<i32>(1)), 0).rgb, vec3<f32>(0.0)));
}

@fragment
fn fs_rcas(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let sp = vec2<i32>(floor(frag.xy));
    let b = load_gamma(sp + vec2<i32>(0, -1));
    let d = load_gamma(sp + vec2<i32>(-1, 0));
    let e = load_gamma(sp);
    let f = load_gamma(sp + vec2<i32>(1, 0));
    let h = load_gamma(sp + vec2<i32>(0, 1));
    let mn4 = min(min(min(b, d), f), h);
    let mx4 = max(max(max(b, d), f), h);
    let peak = vec2<f32>(1.0, -4.0);
    // (The denominators are kept off zero: 0 * infinity is not a number.)
    let hit_min = min(mn4, e) * (1.0 / (4.0 * max(mx4, vec3<f32>(1.0e-5))));
    let hit_max = (peak.x - max(mx4, e)) * (1.0 / min(4.0 * mn4 + peak.y, vec3<f32>(-1.0e-5)));
    let lobe_c = max(-hit_min, hit_max);
    let limit = 0.25 - 1.0 / 16.0;
    let lobe = max(-limit, min(max(max(lobe_c.x, lobe_c.y), lobe_c.z), 0.0)) * params.rcas.x;
    let rcp_l = rcp_med(4.0 * lobe + 1.0);
    let pix = (lobe * b + lobe * d + lobe * h + lobe * f + e) * rcp_l;
    return vec4<f32>(pix * pix, 1.0);
}
