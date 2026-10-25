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

/// What a character is doing, as far as the animation choice goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Idle,
    Walk,
    Run,
    /// A jump from standing and one while moving, and the landings (from idle, a walk and a run).
    JumpStand,
    JumpForward,
    LandIdle,
    LandWalk,
    LandRun,
    /// A creature's sidestep to the left or the right (its animations 134 and 135: the wulf's and the cute creature's `sidestep-left`
    /// and `sidestep-right`, which roll the ball-shaped ones).
    StrafeLeft,
    StrafeRight,
}

/// The file name of a human character's animation for an action, as the
/// database names them: `male_idle-stand_01_01.ani`, `female_walk_01_01.ani`,
/// `athrox_run_01_01.ani`. (Solitus, Opifex and Nanomage share the male and
/// female sets; the Atrox has its own.)
pub fn animation_name(breed: u8, sex: u8, action: Action) -> String {
    let prefix = if breed == ATROX { "athrox" } else if sex == FEMALE { "female" } else { "male" };
    let stem = match action {
        Action::Idle => "idle-stand",
        Action::Walk => "walk",
        Action::Run => "run",
        Action::JumpStand => "jump-stand",
        Action::JumpForward => "jump-forward",
        Action::LandIdle => "jump-land-idle",
        Action::LandWalk => "jump-land-walk",
        Action::LandRun => "jump-land-run",
        // (players walk sideways)
        Action::StrafeLeft | Action::StrafeRight => "walk",
    };
    format!("{prefix}_{stem}_01_01.ani")
}

/// The file names a social action's animation may have (`male_social-backflip.ani`, and a few with the `_01_01` suffix).
pub fn social_animation_names(breed: u8, sex: u8, name: &str) -> [String; 2] {
    let prefix = if breed == ATROX { "athrox" } else if sex == FEMALE { "female" } else { "male" };
    // (the casting animations are `male_spell-self_01_01.ani` and so on)
    if name.starts_with("spell-") {
        return [format!("{prefix}_{name}_01_01.ani"), format!("{prefix}_{name}_01_01.ani")];
    }
    [format!("{prefix}_social-{name}.ani"), format!("{prefix}_social-{name}_01_01.ani")]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn social_animation_names_follow_the_breed_and_sex() {
        assert_eq!(social_animation_names(SOLITUS, MALE, "backflip")[0], "male_social-backflip.ani");
        assert_eq!(social_animation_names(OPIFEX, FEMALE, "bow")[0], "female_social-bow.ani");
        assert_eq!(social_animation_names(ATROX, 1, "disco")[1], "athrox_social-disco_01_01.ani");
        assert_eq!(social_animation_names(SOLITUS, FEMALE, "spell-dir")[0], "female_spell-dir_01_01.ani");
    }

    #[test]
    fn animation_names() {
        assert_eq!(animation_name(SOLITUS, FEMALE, Action::Idle), "female_idle-stand_01_01.ani");
        assert_eq!(animation_name(OPIFEX, MALE, Action::Walk), "male_walk_01_01.ani");
        assert_eq!(animation_name(ATROX, 1, Action::Run), "athrox_run_01_01.ani");
        assert_eq!(animation_name(SOLITUS, MALE, Action::LandRun), "male_jump-land-run_01_01.ani");
    }
}
