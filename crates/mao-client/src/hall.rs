//! The hall behind the login and the character list: GUI.dll's `LoginWorld_c` (0x100160c5), the scene the original
//! shows from the moment the game's window opens until a character is played.
//!
//! - Five meshes found by their file names (`AddMesh` 0x10016000, `VisualMesh_t`s of type 0xf6951), in the meshes' own
//!   space: [`MESHES`]. They are the character creation's set (the hall with a profession's figure on each pedestal is
//!   part of it); the original loads them four times, a stage two units above the last, and shows one stage.
//! - One camera for every stage (`SetStage` 0x10015fa7): at [`LOGIN_CAMERA`], turned by [`LOGIN_ROTATION`] (down the
//!   hall, twelve degrees downwards), 60 degrees across, near 0.5 and far 1000
//!   (`VisualCamera_t(1.0472, width / height, 0.5, 1000)`).
//! - The character picked in the list stands at the camera's place plus [`VIEWER_OFFSET`] (`CharSelectWindow_c`
//!   0x1000efb1: `CharacterViewer_c::SetPos`): five units ahead of the camera and 1.9 below it, in its idle stance.
//!
//! Checked against the original's own pictures (the capture of 2026-10-30, taken every two seconds from its start):
//! drawn with the original's rule for the angle, the figures stand where they do there.
//!
//! **The stretch.** The original's field of view is the angle across the window (`SetViewPlaneWindow(angle, aspect)`:
//! the window's half width is the angle's tangent, its height that over the aspect), so the wider the window, the less
//! is seen up and down: at 5120 by 1440 the figures are cut off at the knees and the picked character at its head and
//! feet (the user's screenshot of 2026-11-11 and the capture). Here the angle is taken as the original has it for a 4:3
//! window, and a wider window sees the same height and more to the sides ([`half_height`]). That is this client's own
//! choice, asked for by the user.
//!
//! The character creation's own cameras are the game folder's `CharCreateCamera.dat` (`CameraCount`, then for each an
//! `ID` of one to four numbers, `Pos`, `Rot` as a quaternion and `degFOV`): [`places`] reads them; nothing uses them
//! yet (the creation screens still show the character alone).
//!
//! Not done: the picked character turned by the mouse, the creation's seven breed figures (`CCCharacter_t`) and its
//! camera moves, the hall's nano effect running, the figures' animations (each mesh loops in the original).

use mao_formats::model::Model;
use mao_formats::resource::{Id, ResourceDatabase};

/// The hall's meshes, as `LoginWorld_c` adds them to a stage.
pub const MESHES: [&str; 5] = [
    "charactercreation_main.abiff",
    "charactercreation_professions.abiff",
    "charactercreation_professions02.abiff",
    "charactercreation_nanoeffect.abiff",
    "charactercreation_adventurer.abiff",
];

/// The login world's camera: its place (GUI.dll's constants at 0x101a9aa4, 0x101a9aa0, 0x101a9a9c) and its rotation
/// (0x101aa06c to 0x101aa060), a quaternion (x, y, z, w) in the game's frame (Y up), and its angle in degrees.
pub const LOGIN_CAMERA: [f32; 3] = [0.657_693_86, 2.274_580_2, -10.548_863];
pub const LOGIN_ROTATION: [f32; 4] = [0.000_490_287_44, -0.994_476, 0.104_805_2, -0.005_759_124];
pub const LOGIN_FOV: f32 = 60.0;

/// Where the picked character stands, from the camera's place (0x1000efb1: 0, -1.9, -5).
pub const VIEWER_OFFSET: [f32; 3] = [0.0, -1.9, -5.0];

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

/// The place the login and the character list are seen from.
pub fn login_place() -> Place {
    // (`MAO_HALL_CAMERA=x,y,z,qx,qy,qz,qw,fov`: another place, to look round the hall)
    if let Some(v) = std::env::var("MAO_HALL_CAMERA").ok().map(|t| t.split(',').filter_map(|n| n.trim().parse::<f32>().ok()).collect::<Vec<f32>>()).filter(|v| v.len() == 8) {
        return Place { id: vec![0], position: [v[0], v[1], v[2]], rotation: [v[3], v[4], v[5], v[6]], fov_degrees: v[7] };
    }
    Place { id: Vec::new(), position: LOGIN_CAMERA, rotation: LOGIN_ROTATION, fov_degrees: LOGIN_FOV }
}

/// Where the picked character stands in the hall.
pub fn viewer_position() -> [f32; 3] {
    [LOGIN_CAMERA[0] + VIEWER_OFFSET[0], LOGIN_CAMERA[1] + VIEWER_OFFSET[1], LOGIN_CAMERA[2] + VIEWER_OFFSET[2]]
}

/// The swap of the game's upright (Y) and the renderer's (Z), which every part of the hall is drawn through.
const SWAP: mao_formats::model::Matrix = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];

/// A character's model (its parts in the game's frame, at the origin) stood where the picked character stands.
pub fn stand(mut character: Model) -> Model {
    let at = viewer_position();
    let place = mao_formats::model::trs_matrix(at, [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]);
    let matrix = mao_formats::model::multiply(&place, &SWAP);
    character.instances = (0..character.parts.len()).map(|i| (i, matrix)).collect();
    character
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
    hall.instances = (0..hall.parts.len()).map(|i| (i, SWAP)).collect();
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

    /// The login's camera looks down the hall (towards -z) and twelve degrees downwards; the picked character stands
    /// five units ahead of it, 1.9 below.
    #[test]
    fn the_login_camera_looks_down_the_hall() {
        let forward = rotate(LOGIN_ROTATION, [0.0, 0.0, 1.0]);
        assert!(forward[2] < -0.97 && forward[0].abs() < 0.02);
        assert!((forward[1].asin().to_degrees() + 12.03).abs() < 0.05);
        let at = viewer_position();
        assert!((at[0] - 0.6577).abs() < 1e-3 && (at[1] - 0.3746).abs() < 1e-3 && (at[2] + 15.5489).abs() < 1e-3);
        // (a part at the character's origin lands there, with the upright swapped for the renderer)
        let mut model = Model::default();
        model.parts.push(mao_formats::model::Part { name: "body".into(), world: mao_formats::model::IDENTITY, vertices: Vec::new(), indices: Vec::new(), material: Default::default() });
        let stood = stand(model);
        let m = stood.instances[0].1;
        assert!((m[12] - at[0]).abs() < 1e-5 && (m[13] - at[2]).abs() < 1e-5 && (m[14] - at[1]).abs() < 1e-5);
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
