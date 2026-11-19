//! What the mouse points at in the world and what its clicks do there: N3.dll's `n3Camera_t` and GUI.dll's
//! `ActionViewMouseHandler_c`.
//!
//! **The line.** `n3Camera_t::SetMousePos` (0x10020571) runs a line from the eye through the mouse, cut short where it
//! first meets the ground or a surface of the playfield (FUN_1001fe47: `viewer.rs`, with `Zones::line` and the terrain).
//! Its length is the view's; here [`LENGTH`], past anything [`GATHER`] lets in.
//!
//! **The list.** At most ten times a second ([`REBUILT`], `n3Playfield_t::RunCameraSelectionCollChecks` 0x1000d8d1) every
//! dynel within [`GATHER`] of the eye is tried against the line (FUN_10020a3c, `n3VisualDynel_t::FineCollisionCheck`
//! 0x10019452) and those it meets are kept, nearest first ([`on_line`]). **One's own character is one of them like any
//! other**, and is left out only in the first person.
//!
//! **The shapes** ([`Shape`]). A character is met by the box its posed vertices fill in the mesh's own axes (the option
//! `BoundingBoxTargeting`, on by default: randy31's `RCATMesh` line test, [`ray_box`]; the triangles of the option's
//! other setting are not ported). A built-in object is met by the triangles of its mesh that face the line
//! ([`PickMesh::hit`], `VisualMesh_t::IsLineIntersecting`), at rest: an open door's leaves are where they hang closed.
//! Anything whose mesh is not known here (an object the server announces that is not in the playfield's table) is
//! still a [`Shape::Post`], the upright cylinder everything used to be.
//!
//! **The clicks** ([`left_click`], [`right_click`], [`double_click`]; FUN_1002cfac on the button's release, FUN_1002ce31
//! on its press). All of them act on what is under the pointer, [`Reference`], which keeps the last thing pointed at for
//! half a second (`InputConfig_t::SetCurrentReference` 0x1001a191), and nothing happens over nothing.
//!
//! Not ported: a dynel's own visibility (`IsVisible`: a hidden character, a statel's owner and team rules), the
//! character's held weapons and such as meshes of their own, and the pointer's shape over what it points at.

use std::sync::Arc;
use std::time::{Duration, Instant};

use mao_formats::model::{Matrix, Model};

/// How far from the eye a dynel may stand to be tried (`n3Playfield_t::RunCameraSelectionCollChecks`' float 0x1003ce54).
pub const GATHER: f32 = 200.0;
/// The line's length before the ground cuts it.
pub const LENGTH: f32 = 250.0;
/// How often the list is made, in seconds (its double 0x1003c890).
pub const REBUILT: f32 = 0.1;
/// How long what was last pointed at stays under the pointer when the pointer has left it (`SetCurrentReference`).
pub const MEMORY: Duration = Duration::from_millis(500);
/// How far the mouse may go while a button is down for its release still to be a click (GUI.dll's float 0x101aeafc): the
/// mouse's counts over 1000 (`InputConfig_t::FrameProcess`, the double 0x101ade10), summed.
pub const CLICK_TRAVEL: f32 = 0.02;

type V = [f32; 3];

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V, b: V) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn length(a: V) -> f32 {
    dot(a, a).sqrt()
}

fn scaled(a: V, k: f32) -> V {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn transform(m: &Matrix, p: V) -> V {
    [p[0] * m[0] + p[1] * m[4] + p[2] * m[8] + m[12], p[0] * m[1] + p[1] * m[5] + p[2] * m[9] + m[13], p[0] * m[2] + p[1] * m[6] + p[2] * m[10] + m[14]]
}

/// The inverse of a placing matrix (rows: three axes and the place).
pub fn invert(m: &Matrix) -> Matrix {
    let (a, b, c) = ([m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]);
    let (bc, ca, ab) = (cross(b, c), cross(c, a), cross(a, b));
    let det = dot(a, bc);
    let k = if det != 0.0 { 1.0 / det } else { 0.0 };
    let mut out = [0.0; 16];
    for (i, row) in [[bc[0], ca[0], ab[0]], [bc[1], ca[1], ab[1]], [bc[2], ca[2], ab[2]]].into_iter().enumerate() {
        out[i * 4] = row[0] * k;
        out[i * 4 + 1] = row[1] * k;
        out[i * 4 + 2] = row[2] * k;
    }
    out[15] = 1.0;
    let t = transform(&out, [m[12], m[13], m[14]]);
    (out[12], out[13], out[14]) = (-t[0], -t[1], -t[2]);
    out
}

/// One of a mesh's parts: the sphere round it and its triangles, in the mesh's own space.
#[derive(Debug)]
pub struct MeshPart {
    centre: V,
    radius: f32,
    triangles: Vec<[V; 3]>,
}

/// A built-in object's mesh for the mouse.
#[derive(Debug, Default)]
pub struct PickMesh {
    parts: Vec<MeshPart>,
}

impl PickMesh {
    pub fn of(model: &Model) -> PickMesh {
        let parts = model
            .parts
            .iter()
            .filter(|p| !p.indices.is_empty())
            .map(|part| {
                let points: Vec<V> = part.vertices.iter().map(|v| transform(&part.world, v.position)).collect();
                let (centre, radius) = mao_formats::bounds::sphere(&points);
                let triangles = part.indices.chunks_exact(3).filter_map(|t| Some([*points.get(t[0] as usize)?, *points.get(t[1] as usize)?, *points.get(t[2] as usize)?])).collect();
                MeshPart { centre, radius, triangles }
            })
            .collect();
        PickMesh { parts }
    }

    /// `VisualMesh_t::IsLineIntersecting`: part after part, the line must pass through the part's sphere with the sphere
    /// ahead of it, and then the first triangle (in the mesh's order, not the nearest: randy31's `IsRayIntersecting` as
    /// the function asks it) that faces the line and that it crosses before its end is the hit. The distance along it.
    pub fn hit(&self, from: V, to: V) -> Option<f32> {
        let along = sub(to, from);
        let long2 = dot(along, along);
        if long2 == 0.0 {
            return None;
        }
        for part in &self.parts {
            let relative = sub(from, part.centre);
            let across = cross(relative, along);
            if dot(across, across) / long2 > part.radius * part.radius {
                continue;
            }
            let reach = dot(relative, along);
            let projection = scaled(along, reach / long2);
            if length(projection) >= long2.sqrt() + part.radius || dot(projection, along) >= 0.0 {
                continue;
            }
            for [a, b, c] in &part.triangles {
                let (a, b, c) = (*a, *b, *c);
                let normal = cross(sub(b, a), sub(c, b));
                let facing = dot(along, normal);
                if facing >= 0.0 {
                    continue;
                }
                let t = dot(sub(a, from), normal) / facing;
                let p = [from[0] + along[0] * t, from[1] + along[1] * t, from[2] + along[2] * t];
                let first = cross(sub(b, a), sub(p, a));
                if dot(first, cross(sub(c, b), sub(p, b))) < 0.0 || dot(cross(sub(a, c), sub(p, c)), first) < 0.0 || t.is_nan() || t > 1.0 {
                    continue;
                }
                return Some(length(scaled(along, t)));
            }
        }
        None
    }
}

/// randy31's FUN for `RCATMesh`'s bounds (`catmesh.rs` `ray_box`): where a line from `from` along the unit `along` enters
/// the box; `from` itself when it starts inside.
pub fn ray_box(from: V, along: V, lo: V, hi: V) -> Option<V> {
    let (mut side, mut plane, mut inside) = ([false; 3], [0.0; 3], true);
    for c in 0..3 {
        if lo[c] > from[c] {
            (plane[c], side[c], inside) = (lo[c], true, false);
        } else if hi[c] < from[c] {
            (plane[c], side[c], inside) = (hi[c], true, false);
        }
    }
    if inside {
        return Some(from);
    }
    let t: [f32; 3] = std::array::from_fn(|c| if side[c] && along[c] != 0.0 { (plane[c] - from[c]) / along[c] } else { -1.0 });
    let which = (1..3).fold(0, |w, c| if t[c] > t[w] { c } else { w });
    if t[which] < 0.0 {
        return None;
    }
    let mut hit = [0.0; 3];
    for c in 0..3 {
        hit[c] = if c == which { plane[c] } else { along[c] * t[which] + from[c] };
        if c != which && (lo[c] > hit[c] || hi[c] < hit[c]) {
            return None;
        }
    }
    Some(hit)
}

/// Where the line from `from` along the unit `along` first meets an upright cylinder.
fn post_hit(from: V, along: V, feet: V, r: f32, h: f32) -> Option<f32> {
    let (ox, oy) = (from[0] - feet[0], from[1] - feet[1]);
    let a = along[0] * along[0] + along[1] * along[1];
    let mut best: Option<f32> = None;
    let mut take = |t: f32| {
        if t > 0.0 && best.is_none_or(|b| t < b) {
            best = Some(t);
        }
    };
    // the side
    if a > 1e-9 {
        let b = 2.0 * (ox * along[0] + oy * along[1]);
        let c = ox * ox + oy * oy - r * r;
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            for t in [(-b - disc.sqrt()) / (2.0 * a), (-b + disc.sqrt()) / (2.0 * a)] {
                let z = from[2] + t * along[2];
                if z >= feet[2] && z <= feet[2] + h {
                    take(t);
                }
            }
        }
    }
    // the top and the bottom
    if along[2].abs() > 1e-9 {
        for cap in [feet[2], feet[2] + h] {
            let t = (cap - from[2]) / along[2];
            let (x, y) = (ox + t * along[0], oy + t * along[1]);
            if x * x + y * y <= r * r {
                take(t);
            }
        }
    }
    best
}

/// What the line is tried against (see the module's text). The matrices take the world (Z up) into the shape's own space.
#[derive(Clone, Debug)]
pub enum Shape {
    Body { to_mesh: Matrix, lo: V, hi: V },
    Mesh { mesh: Arc<PickMesh>, to_mesh: Matrix },
    Post { feet: V, radius: f32, height: f32 },
}

impl Shape {
    /// How far along the line from `from` to `to` (world, Z up) it is met; in the shape's own measure, as the original's
    /// numbers are.
    pub fn hit(&self, from: V, to: V) -> Option<f32> {
        match self {
            Shape::Body { to_mesh, lo, hi } => {
                let (a, b) = (transform(to_mesh, from), transform(to_mesh, to));
                let d = sub(b, a);
                let long = length(d);
                if long == 0.0 {
                    return None;
                }
                let entry = ray_box(a, scaled(d, 1.0 / long), *lo, *hi)?;
                let at = length(sub(a, entry));
                (long >= at).then_some(at)
            }
            Shape::Mesh { mesh, to_mesh } => mesh.hit(transform(to_mesh, from), transform(to_mesh, to)),
            Shape::Post { feet, radius, height } => {
                let d = sub(to, from);
                let long = length(d);
                if long == 0.0 {
                    return None;
                }
                post_hit(from, scaled(d, 1.0 / long), *feet, *radius, *height).filter(|t| *t <= long)
            }
        }
    }
}

/// Something the mouse can point at.
#[derive(Clone, Debug)]
pub struct Pickable {
    pub kind: u32,
    pub id: u32,
    /// Where it stands (world, Z up).
    pub at: V,
    pub shape: Shape,
    /// One's own character.
    pub own: bool,
}

/// The list: everything within [`GATHER`] of `from` that the line meets, nearest first; one's own character is left out
/// in the first person (FUN_10020a3c: the camera's mode 0).
pub fn on_line(pickables: &[Pickable], from: V, to: V, first_person: bool) -> Vec<(u32, u32)> {
    let mut met: Vec<(f32, (u32, u32))> = pickables
        .iter()
        .filter(|p| !(p.own && first_person) && length(sub(p.at, from)) < GATHER)
        .filter_map(|p| Some((p.shape.hit(from, to)?, (p.kind, p.id))))
        .collect();
    met.sort_by(|a, b| a.0.total_cmp(&b.0));
    met.into_iter().map(|m| m.1).collect()
}

/// `n3Camera_t::GetNextTarget` (0x10020723), asked with the selected target: the one after it on the line, round to the
/// first again, or the first when the selected one is not on it. So a second click in the same place takes what stands
/// behind the first.
pub fn next_on_line(line: &[(u32, u32)], selected: Option<u32>) -> Option<(u32, u32)> {
    match selected.and_then(|s| line.iter().position(|l| l.1 == s)) {
        Some(at) => line.get((at + 1) % line.len()).copied(),
        None => line.first().copied(),
    }
}

/// `n3Camera_t::GetObjectUnderColLine` (0x1002069c): the selected target when the line meets it, else the first on it.
pub fn under_line(line: &[(u32, u32)], selected: Option<u32>) -> Option<(u32, u32)> {
    selected.and_then(|s| line.iter().copied().find(|l| l.1 == s)).or_else(|| line.first().copied())
}

/// What is under the pointer (`InputConfig_t` +0xb0), kept as `SetCurrentReference` keeps it.
#[derive(Debug, Default)]
pub struct Reference {
    shown: Option<(u32, u32)>,
    kept: Option<(u32, u32)>,
    since: Option<Instant>,
}

impl Reference {
    pub fn set(&mut self, under: Option<(u32, u32)>, now: Instant) {
        match under {
            Some(_) => (self.shown, self.kept, self.since) = (under, under, Some(now)),
            None if self.since.is_none_or(|s| now.duration_since(s) > MEMORY) => (self.shown, self.kept) = (None, None),
            None => self.shown = self.kept,
        }
    }

    pub fn get(&self) -> Option<(u32, u32)> {
        self.shown
    }
}

/// The mouse in the world: the list, when it was last made, and what is under the pointer.
#[derive(Debug, Default)]
pub struct Pointer {
    pub line: Vec<(u32, u32)>,
    pub under: Reference,
    made: Option<Instant>,
    /// The left button's last press, for a double click.
    pressed: Option<(Instant, (f64, f64))>,
}

impl Pointer {
    /// Whether it is time to make the list again.
    pub fn due(&mut self, now: Instant) -> bool {
        let due = self.made.is_none_or(|made| now.duration_since(made).as_secs_f32() > REBUILT);
        if due {
            self.made = Some(now);
        }
        due
    }

    /// The left button went down at `at`: whether this is the second press of a double click. The original has the event
    /// from the window system (`NewGuiSpot_t::CheckHotSpot`'s event 3); here Windows' own defaults, half a second and a
    /// box four pixels across, and a third press starts afresh.
    pub fn second_press(&mut self, now: Instant, at: (f64, f64)) -> bool {
        let second = self.pressed.is_some_and(|(when, from)| now.duration_since(when) <= Duration::from_millis(500) && (at.0 - from.0).abs() <= 2.0 && (at.1 - from.1).abs() <= 2.0);
        self.pressed = (!second).then_some((now, at));
        second
    }
}

/// What a click in the world asks of the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Deed {
    /// A left click: the thing becomes the target.
    #[default]
    Select,
    /// A window's "use" of a thing: it becomes the target and is used.
    SelectAndUse,
    /// A right click: used, the target stays (`N3Msg_DefaultActionOnDynel` on a character, else `N3Msg_UseItem`).
    Use,
    /// A double click's `N3Msg_DefaultActionOnDynel`.
    Default,
    /// Ctrl + left click on a character: it becomes the target and the fight goes to it (`N3Msg_SwitchTarget`).
    Switch,
    /// Shift + left click: its info page.
    Info,
}

impl Deed {
    /// Whether it makes the thing the target.
    pub fn selects(self) -> bool {
        matches!(self, Deed::Select | Deed::SelectAndUse | Deed::Switch)
    }
}

/// Whether the deed uses the thing. `N3Msg_DefaultActionOnDynel` (Gamecode 0x1002939b) does nothing to one's own
/// character, and of anything but a character it takes only what its `Can` stat (30) says can be got (bit 0) or used
/// (bit 3); a right click on such a thing is a plain use (FUN_1002cfac). `can` is none where the thing's stats are not
/// known here.
pub fn uses(deed: Deed, character: bool, own: bool, can: Option<i64>) -> bool {
    match deed {
        Deed::SelectAndUse => true,
        Deed::Use => !(character && own),
        Deed::Default if character => !own,
        Deed::Default => can.is_none_or(|c| c & 9 != 0),
        _ => false,
    }
}

/// The left button's release (FUN_1002cfac): with Ctrl on a character the fight goes to it, with Shift the info page,
/// else the next on the line (what is under the pointer when the list has nothing). Alt is not looked at.
pub fn left_click(line: &[(u32, u32)], selected: Option<u32>, under: Option<(u32, u32)>, ctrl: bool, shift: bool) -> Option<((u32, u32), Deed)> {
    let under = under?;
    Some(if ctrl && under.0 == mao_net::n3::CHARACTER {
        (under, Deed::Switch)
    } else if shift {
        (under, Deed::Info)
    } else {
        (next_on_line(line, selected).unwrap_or(under), Deed::Select)
    })
}

/// The right button's release (FUN_1002cfac): what is under the pointer is used, whatever the keys.
pub fn right_click(under: Option<(u32, u32)>) -> Option<((u32, u32), Deed)> {
    under.map(|u| (u, Deed::Use))
}

/// A double click's second press (FUN_1002ce31): the default action on what is under the pointer, unless that is one's
/// own character or the option `DoubleclickAction` is off. (The release that follows is a click as any other.)
pub fn double_click(under: Option<(u32, u32)>, own: Option<u32>, option: bool) -> Option<((u32, u32), Deed)> {
    under.filter(|u| option && Some(u.1) != own).map(|u| (u, Deed::Default))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(id: (u32, u32), x: f32, y: f32, radius: f32, height: f32, own: bool) -> Pickable {
        Pickable { kind: id.0, id: id.1, at: [x, y, 0.0], shape: Shape::Post { feet: [x, y, 0.0], radius, height }, own }
    }

    #[test]
    fn a_line_meets_the_near_side_of_a_post() {
        let shape = Shape::Post { feet: [0.0; 3], radius: 0.5, height: 2.0 };
        // from 10 m west at chest height, looking east at a 0.5 m cylinder 2 m tall
        assert!((shape.hit([-10.0, 0.0, 1.0], [10.0, 0.0, 1.0]).unwrap() - 9.5).abs() < 1e-4);
        // over its top, behind the start, short of it: missed
        assert!(shape.hit([-10.0, 0.0, 3.0], [10.0, 0.0, 3.0]).is_none());
        assert!(shape.hit([10.0, 0.0, 1.0], [20.0, 0.0, 1.0]).is_none());
        assert!(shape.hit([-10.0, 0.0, 1.0], [-5.0, 0.0, 1.0]).is_none());
        // looking down on it: its top
        assert!((shape.hit([0.1, 0.0, 10.0], [0.1, 0.0, -10.0]).unwrap() - 8.0).abs() < 1e-4);
    }

    /// A character's box stands in its own axes: a body turned a quarter is met on its other side.
    #[test]
    fn a_body_is_met_by_its_box_as_it_is_turned() {
        // (a body 2 wide along its own x, 0.5 deep, 2 tall, standing at (10, 0) and turned a quarter: its x along the world's y)
        let place: Matrix = [0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 0.0, 0.0, 1.0];
        let shape = Shape::Body { to_mesh: invert(&place), lo: [-1.0, -0.25, 0.0], hi: [1.0, 0.25, 2.0] };
        // from the west it is 0.25 deep, so met at 9.75; and it is wide along y
        assert!((shape.hit([0.0, 0.8, 1.0], [20.0, 0.8, 1.0]).unwrap() - 9.75).abs() < 1e-3);
        assert!(shape.hit([0.0, 1.2, 1.0], [20.0, 1.2, 1.0]).is_none());
        // over its head; short of it
        assert!(shape.hit([0.0, 0.0, 2.5], [20.0, 0.0, 2.5]).is_none());
        assert!(shape.hit([0.0, 0.0, 1.0], [5.0, 0.0, 1.0]).is_none());
        // a line that starts inside it meets it at once
        assert_eq!(shape.hit([10.0, 0.0, 1.0], [20.0, 0.0, 1.0]), Some(0.0));
    }

    #[test]
    fn a_placing_matrix_is_undone() {
        let m: Matrix = [0.0, 2.0, 0.0, 0.0, -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 3.0, 4.0, 5.0, 1.0];
        let back = transform(&invert(&m), transform(&m, [1.0, 2.0, 3.0]));
        assert!(length(sub(back, [1.0, 2.0, 3.0])) < 1e-5, "{back:?}");
    }

    /// Only the triangle's front is met, and the first in the mesh's order.
    #[test]
    fn a_mesh_is_met_on_the_faces_towards_the_line() {
        // (two squares' halves across the x axis, at x 1 and x 3, their fronts to -x)
        let face = |x: f32| [[x, -1.0, -1.0], [x, 1.0, -1.0], [x, -1.0, 1.0]];
        let mesh = PickMesh { parts: vec![MeshPart { centre: [2.0, 0.0, 0.0], radius: 3.0, triangles: vec![face(3.0), face(1.0)] }] };
        let normal = cross(sub(face(1.0)[1], face(1.0)[0]), sub(face(1.0)[2], face(1.0)[1]));
        assert!(normal[0] > 0.0, "{normal:?}");
        // (the fronts here are towards +x: a line from there meets them, in the mesh's order, not the nearest)
        assert!((mesh.hit([10.0, -0.5, -0.5], [-10.0, -0.5, -0.5]).unwrap() - 7.0).abs() < 1e-4);
        // from behind: nothing
        assert!(mesh.hit([-10.0, -0.5, -0.5], [10.0, -0.5, -0.5]).is_none());
        // beside the triangle, short of it, or the sphere behind the line's start
        assert!(mesh.hit([10.0, 0.9, 0.9], [-10.0, 0.9, 0.9]).is_none());
        assert!(mesh.hit([10.0, -0.5, -0.5], [5.0, -0.5, -0.5]).is_none());
        assert!(mesh.hit([-10.0, -0.5, -0.5], [-20.0, -0.5, -0.5]).is_none());
    }

    /// A click steps along what the mouse's line meets, as the original's `GetNextTarget`: an NPC in front of a pad (the
    /// user's of 2026-11-15, a shuttle's ring behind an NPC in Tir County). One's own character takes its turn by its
    /// distance, and is not on the line in the first person.
    #[test]
    fn a_second_click_takes_what_stands_behind() {
        let (npc, pad, own) = ((0xc350, 0x7b23d5c0), (0xc73d, 0xc0070287), (0xc350, 7));
        let pickables = [post(pad, 20.0, 0.0, 2.0, 2.2, false), post(own, 2.0, 0.0, 0.5, 1.9, true), post(npc, 15.0, 0.0, 0.5, 1.9, false), post((0xc350, 9), 15.0, 9.0, 0.5, 1.9, false)];
        let (from, to) = ([0.0, 0.0, 1.0], [100.0, 0.0, 1.0]);
        let line = on_line(&pickables, from, to, false);
        assert_eq!(line, [own, npc, pad]);
        assert_eq!(on_line(&pickables, from, to, true), [npc, pad]);
        assert_eq!(next_on_line(&line, None), Some(own));
        assert_eq!(next_on_line(&line, Some(own.1)), Some(npc));
        assert_eq!(next_on_line(&line, Some(npc.1)), Some(pad));
        assert_eq!(next_on_line(&line, Some(pad.1)), Some(own));
        assert_eq!(next_on_line(&line, Some(0x1234)), Some(own));
        // (what is under the pointer: the selected one when the line meets it, else the first)
        assert_eq!(under_line(&line, Some(pad.1)), Some(pad));
        assert_eq!(under_line(&line, Some(0x1234)), Some(own));
        assert_eq!(under_line(&line, None), Some(own));
        // (alone on the line, a thing is taken again and again)
        assert_eq!(next_on_line(&[pad], Some(pad.1)), Some(pad));
        assert_eq!((next_on_line(&[], Some(7)), under_line(&[], None)), (None, None));
        // (past 200 m from the eye nothing is tried)
        assert_eq!(on_line(&[post(npc, 199.0, 0.0, 0.5, 1.9, false), post(pad, 201.0, 0.0, 2.0, 2.2, false)], from, [300.0, 0.0, 1.0], false), [npc]);
        // (the line cut short at the ground ends before what stands behind)
        assert_eq!(on_line(&pickables, from, [10.0, 0.0, 1.0], false), [own]);
    }

    /// FUN_1002cfac and FUN_1002ce31.
    #[test]
    fn what_the_clicks_ask_for() {
        let (npc, pad, own) = ((mao_net::n3::CHARACTER, 0x70), (0xc73d, 0x71), (mao_net::n3::CHARACTER, 7));
        let line = [npc, pad];
        // (a plain click steps on; Alt is no key to it)
        assert_eq!(left_click(&line, Some(npc.1), Some(npc), false, false), Some((pad, Deed::Select)));
        // (with nothing on the line any more, what the pointer last was on)
        assert_eq!(left_click(&[], None, Some(pad), false, false), Some((pad, Deed::Select)));
        // (over nothing, nothing, whatever the keys)
        assert_eq!(left_click(&line, None, None, true, true), None);
        assert_eq!(right_click(None), None);
        // (Ctrl on a character, also with Shift; on anything else Ctrl is no key)
        assert_eq!(left_click(&line, None, Some(npc), true, false), Some((npc, Deed::Switch)));
        assert_eq!(left_click(&line, None, Some(npc), true, true), Some((npc, Deed::Switch)));
        assert_eq!(left_click(&line, Some(npc.1), Some(pad), true, false), Some((pad, Deed::Select)));
        // (Shift: the page of what is under the pointer)
        assert_eq!(left_click(&line, None, Some(pad), false, true), Some((pad, Deed::Info)));
        assert_eq!(left_click(&line, None, Some(pad), true, true), Some((pad, Deed::Info)));
        assert_eq!(right_click(Some(pad)), Some((pad, Deed::Use)));
        // (a double click, not on oneself nor with the option off)
        assert_eq!(double_click(Some(pad), Some(own.1), true), Some((pad, Deed::Default)));
        assert_eq!((double_click(Some(own), Some(own.1), true), double_click(Some(pad), Some(own.1), false)), (None, None));
        // (who is made the target, and what is used)
        assert!(Deed::Select.selects() && Deed::Switch.selects() && !Deed::Use.selects() && !Deed::Default.selects() && !Deed::Info.selects());
        assert!(uses(Deed::Use, false, false, Some(0)) && uses(Deed::Use, true, false, None) && !uses(Deed::Use, true, true, None));
        assert!(uses(Deed::Default, false, false, Some(8)) && uses(Deed::Default, false, false, Some(1)) && uses(Deed::Default, false, false, None));
        assert!(!uses(Deed::Default, false, false, Some(0x200)) && !uses(Deed::Default, true, true, None) && !uses(Deed::Select, false, false, None));
    }

    #[test]
    fn what_was_pointed_at_stays_half_a_second() {
        let t = Instant::now();
        let mut r = Reference::default();
        r.set(None, t);
        assert_eq!(r.get(), None);
        r.set(Some((1, 2)), t);
        r.set(None, t + Duration::from_millis(400));
        assert_eq!(r.get(), Some((1, 2)));
        r.set(None, t + Duration::from_millis(501));
        assert_eq!(r.get(), None);
        // (and it does not come back)
        r.set(None, t + Duration::from_millis(502));
        assert_eq!(r.get(), None);
    }

    /// Two presses on one spot soon after each other; a third starts afresh; a press elsewhere or later is a first.
    #[test]
    fn a_double_click_is_two_presses_close_together() {
        let t = Instant::now();
        let at = |ms: u64| t + Duration::from_millis(ms);
        let mut p = Pointer::default();
        assert!(!p.second_press(at(0), (100.0, 100.0)));
        assert!(p.second_press(at(300), (101.0, 99.0)));
        assert!(!p.second_press(at(400), (101.0, 99.0)));
        assert!(!p.second_press(at(1000), (101.0, 99.0)));
        assert!(!p.second_press(at(1100), (110.0, 99.0)));
    }

    #[test]
    fn the_list_is_made_ten_times_a_second() {
        let t = Instant::now();
        let at = |ms: u64| t + Duration::from_millis(ms);
        let mut p = Pointer::default();
        assert!(p.due(at(0)) && !p.due(at(60)) && p.due(at(120)) && !p.due(at(180)) && p.due(at(240)));
    }
}
