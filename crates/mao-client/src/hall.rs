//! The hall behind the login: the scene GUI.dll's `CharCreateModule_t` builds (`InitialiseMessage` 0x1011bdaa) and the
//! original shows from the moment the game's window opens.
//!
//! - Six meshes found by their file names (the table at 0x102724e4, loaded as `VisualMesh_t`s of type 0xf6951), in the
//!   meshes' own space: [`MESHES`].
//! - A camera of 60 degrees, near 0.5 and far 1000 (`VisualCamera_t(1.0472, width / height, 0.5, 1000)`), moved between
//!   the places of the game folder's `CharCreateCamera.dat` (`CameraCount`, then for each an `ID` of one to four
//!   numbers, `Pos`, `Rot` as a quaternion and `degFOV`); the first, `ID: 1`, looks down the hall.
//!
//! **The stretch.** The original's field of view is the angle across the window (`SetViewPlaneWindow(angle, aspect)`:
//! the window's half width is the angle's tangent, its height that over the aspect), so the wider the window, the less
//! is seen up and down: at 5120 by 1440 the hall's upper half is cut off and what is left looks stretched (the user's
//! screenshot of 2026-11-11). Here the angle is taken as the original has it for a 4:3 window, and a wider window sees
//! the same height and more to the sides ([`half_height`]). That is this client's own choice, asked for by the user.
//!
//! Not done: the seven breed figures the creation screens stand in the hall (`CCCharacter_t`), the camera's moves
//! between the screens, the hall's nano effect running.

use mao_formats::model::Model;
use mao_formats::resource::{Id, ResourceDatabase};

/// The hall's meshes, as `InitialiseMessage` loads them.
pub const MESHES: [&str; 6] = [
    "charactercreation_main.abiff",
    "charactercreation_professions.abiff",
    "charactercreation_seq01.abiff",
    "charactercreation_professions02.abiff",
    "charactercreation_nanoeffect.abiff",
    "charactercreation_adventurer.abiff",
];

/// The original camera's angle across a window, and the window shape it is kept for.
pub const NARROWEST: f32 = 4.0 / 3.0;

/// A camera place of `CharCreateCamera.dat`.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub id: Vec<u32>,
    pub position: [f32; 3],
    /// A quaternion (x, y, z, w) in the game's frame (Y up).
    pub rotation: [f32; 4],
    pub fov_degrees: f32,
}

/// The camera places of the file's text.
pub fn places(text: &str) -> Vec<Place> {
    let numbers = |rest: &str| -> Vec<f32> { rest.split_whitespace().filter_map(|t| t.parse().ok()).collect() };
    let mut out: Vec<Place> = Vec::new();
    for line in text.lines() {
        let Some((key, rest)) = line.split_once(':') else { continue };
        match key.trim() {
            "ID" => out.push(Place { id: rest.split_whitespace().filter_map(|t| t.parse().ok()).collect(), position: [0.0; 3], rotation: [0.0, 0.0, 0.0, 1.0], fov_degrees: 60.0 }),
            "Pos" => {
                if let (Some(p), [x, y, z]) = (out.last_mut(), &numbers(rest)[..]) {
                    p.position = [*x, *y, *z];
                }
            }
            "Rot" => {
                if let (Some(p), [x, y, z, w]) = (out.last_mut(), &numbers(rest)[..]) {
                    p.rotation = [*x, *y, *z, *w];
                }
            }
            "degFOV" => {
                if let (Some(p), [f]) = (out.last_mut(), &numbers(rest)[..]) {
                    p.fov_degrees = *f;
                }
            }
            _ => {}
        }
    }
    out
}

/// The place the login looks from: the file's `ID: 1`.
pub fn login_place(game: &std::path::Path) -> Option<Place> {
    let text = mao_formats::files::read_to_string(game.join("CharCreateCamera.dat")).ok()?;
    places(&text).into_iter().find(|p| p.id == [1])
}

/// A vector turned by a quaternion (x, y, z, w).
pub fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let (u, w) = ([q[0], q[1], q[2]], q[3]);
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let t = cross(u, v).map(|c| 2.0 * c);
    let c = cross(u, t);
    [v[0] + w * t[0] + c[0], v[1] + w * t[1] + c[1], v[2] + w * t[2] + c[2]]
}

/// Half the height of the view at unit distance for a camera angle (degrees across a 4:3 window, as the original has
/// it) and a window's shape: a window wider than 4:3 sees as much up and down as a 4:3 one, a narrower one keeps the
/// original's width.
pub fn half_height(fov_degrees: f32, aspect: f32) -> f32 {
    (fov_degrees.to_radians() * 0.5).tan() / aspect.clamp(0.1, NARROWEST)
}

/// The hall's meshes as one model (those the database has).
pub fn model(db: &ResourceDatabase) -> Option<Model> {
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).ok()??).ok()?;
    let mut hall = Model::default();
    for name in MESHES {
        let Some(instance) = index.find(0xf6951, name) else { continue };
        let Some(object) = db.get(Id { kind: 0xf6951, instance }).ok().flatten() else { continue };
        let Ok(mesh) = mao_formats::mesh::parse(&object) else { continue };
        if let Ok(built) = mao_formats::model::build(&mesh) {
            hall.parts.extend(built.parts);
        }
    }
    // (the meshes' frame has Y up, the renderer's Z: every part is drawn through the swap of the two)
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    hall.instances = (0..hall.parts.len()).map(|i| (i, swap)).collect();
    (!hall.parts.is_empty()).then_some(hall)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_file_gives_its_places() {
        let text = "CameraCount: 2\r\nID: 1 \r\nPos: 27.5 2 7.5\r\nRot: 0 1 0 0\r\ndegFOV: 60\r\nID: 1 1 2 \r\nPos: 50.25 109 40.5\r\nRot: 0 0 0 1\r\ndegFOV: 80\r\n";
        let list = places(text);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], Place { id: vec![1], position: [27.5, 2.0, 7.5], rotation: [0.0, 1.0, 0.0, 0.0], fov_degrees: 60.0 });
        assert_eq!((list[1].id.clone(), list[1].fov_degrees), (vec![1, 1, 2], 80.0));
    }

    #[test]
    fn a_half_turn_about_the_upright_looks_back() {
        let v = rotate([0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        assert!((v[0]).abs() < 1e-6 && (v[1]).abs() < 1e-6 && (v[2] + 1.0).abs() < 1e-6);
        assert_eq!(rotate([0.0, 0.0, 0.0, 1.0], [1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
    }

    /// A 4:3 window sees what the original's does; a wider one the same height (the original would see less: at 32:9 only
    /// three eighths of it); a narrower one the original's width.
    #[test]
    fn a_wide_window_sees_the_same_height() {
        let at_4_3 = half_height(60.0, 4.0 / 3.0);
        assert!((at_4_3 - (30f32.to_radians().tan() * 0.75)).abs() < 1e-6);
        assert_eq!(half_height(60.0, 32.0 / 9.0), at_4_3);
        assert_eq!(half_height(60.0, 16.0 / 9.0), at_4_3);
        assert!((half_height(60.0, 1.0) - 30f32.to_radians().tan()).abs() < 1e-6);
    }
}
