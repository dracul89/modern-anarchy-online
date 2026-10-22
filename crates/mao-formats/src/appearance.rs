//! Which meshes dress a character: the original's player-visual table
//! (`Gamecode.dll`, the function at RVA 0x57a2e), by the character's breed
//! and sex. The meshes are found by name in the [`crate::instances`] index.

/// Breed stat values.
pub const SOLITUS: u8 = 1;
pub const OPIFEX: u8 = 2;
pub const NANOMAGE: u8 = 3;
pub const ATROX: u8 = 4;

/// Sex stat values: 1 neuter, 2 male, 3 female.
pub const MALE: u8 = 2;
pub const FEMALE: u8 = 3;

/// The body (a CAT mesh, database type 0xf6952) and head (a static mesh,
/// 0xf6951) file names for a breed and sex, as the original picks them.
pub fn player_meshes(breed: u8, sex: u8) -> Option<(&'static str, &'static str)> {
    Some(match (breed, sex) {
        (SOLITUS, MALE) => ("solitus_male.cir", "head_solitusmale01.abiff"),
        (SOLITUS, FEMALE) => ("solitus_female.cir", "head_solitusfemale01.abiff"),
        (OPIFEX, MALE) => ("opifex_male.cir", "head_opifexmale01.abiff"),
        (OPIFEX, FEMALE) => ("opifex_female.cir", "head_opifexfemale01.abiff"),
        (NANOMAGE, MALE) => ("nanomage_male.cir", "head_nanomale01.abiff"),
        (NANOMAGE, FEMALE) => ("nanomage_female.cir", "head_nanofemale01.abiff"),
        (ATROX, 1) => ("athrox_male.cir", "head_athrox01.abiff"),
        _ => return None,
    })
}
