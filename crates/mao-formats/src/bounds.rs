//! The bounding sphere of a set of points as the original computes it: randy31's `BVolume_t::Init` (FUN_10029b7e,
//! `AnarchyReimagined`'s `dropin/randy31/src/bvolume.rs`), Ritter's method. A mesh's "height" in the original is the
//! diameter of this sphere (`VisualMesh_t::GetMeshHeight`: twice the greatest radius among its triangle meshes), and
//! it is what the name tags and the target's indicator stand on.

type V = [f32; 3];

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn length2(a: V) -> f32 {
    a[0] * a[0] + a[1] * a[1] + a[2] * a[2]
}

/// The sphere round `points`: its centre and radius; a point at the origin of no size for none. The pair of points
/// furthest apart along an axis (x, then y where it is further, then z) gives the first sphere, which then grows to
/// take in each point outside it, half way.
pub fn sphere(points: &[V]) -> (V, f32) {
    let Some(&first) = points.first() else { return ([0.0; 3], 0.0) };
    // (the first of equals on each axis)
    let mut ends = [[0usize; 2]; 3];
    for axis in 0..3 {
        let (mut lo, mut hi) = (0, 0);
        for (k, p) in points.iter().enumerate() {
            if p[axis] < points[lo][axis] {
                lo = k;
            }
            if p[axis] > points[hi][axis] {
                hi = k;
            }
        }
        ends[axis] = [lo, hi];
    }
    let span = |axis: usize| length2(sub(points[ends[axis][1]], points[ends[axis][0]]));
    let (mut far, mut pair) = (span(0), ends[0]);
    if far < span(1) {
        (far, pair) = (span(1), ends[1]);
    }
    if far < span(2) {
        pair = ends[2];
    }
    let (lo, hi) = (points[pair[0]], points[pair[1]]);
    let mut centre = [(hi[0] + lo[0]) / 2.0, (hi[1] + lo[1]) / 2.0, (hi[2] + lo[2]) / 2.0];
    let mut r2 = length2(sub(hi, centre));
    let mut radius = r2.sqrt();
    let _ = first;
    for &p in points {
        let d2 = length2(sub(p, centre));
        if r2 < d2 {
            let d = d2.sqrt();
            let r = ((radius as f64 + d as f64) * 0.5) as f32;
            radius = r;
            r2 = r * r;
            for k in 0..3 {
                centre[k] = (p[k] * (d - r) + centre[k] * r) / d;
            }
        }
    }
    (centre, radius)
}

#[cfg(test)]
mod tests {
    use super::sphere;

    #[test]
    fn the_sphere_holds_every_point() {
        // (a body two units tall and half a unit across: the sphere is as wide as it is tall)
        let mut points = Vec::new();
        for k in 0..40 {
            let (a, y) = (k as f32 * 0.7, k as f32 * 0.05);
            points.push([0.25 * a.cos(), y, 0.25 * a.sin()]);
        }
        let (centre, radius) = sphere(&points);
        assert!(points.iter().all(|p| ((p[0] - centre[0]).powi(2) + (p[1] - centre[1]).powi(2) + (p[2] - centre[2]).powi(2)).sqrt() <= radius + 1e-4));
        assert!((2.0 * radius - 1.95).abs() < 0.15, "{radius}");
        assert!((centre[1] - 0.975).abs() < 0.1);
        // (two points: the sphere on them; none: nothing)
        assert_eq!(sphere(&[[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]]), ([1.0, 0.0, 0.0], 1.0));
        assert_eq!(sphere(&[]), ([0.0; 3], 0.0));
    }
}
