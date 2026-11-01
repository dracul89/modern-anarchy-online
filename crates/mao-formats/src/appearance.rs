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
    /// Moving backwards (the original's abstract animations 136 `walk-back` and 222 `run-back`).
    WalkBack,
    RunBack,
    /// A sidestep to the left or the right (134 `walk-left`, 135 `walk-right`; a creature's are its own, the wulf's and the cute
    /// creature's `sidestep-left` and `sidestep-right`, which roll the ball-shaped ones).
    StrafeLeft,
    StrafeRight,
    /// Swimming and treading water (the original's actions 0x85 `swim` and 0xc3 `idle-swim`).
    Swim,
    IdleSwim,
    /// Sneaking (`CharVehicle_t` mode 6): the character's sneak action (`SimpleChar_t`'s +0x1dc part +0x14, set to 0x66
    /// `sneakcool` by the constructor FUN_1005d0b1 through FUN_1003c66c), moving (played at the speed over 1.5) or standing
    /// (FUN_1006c370 picks it both ways: at rest the rate is left at 1, as FUN_1007009f sets none for a speed of 0).
    Sneak,
    SneakStand,
}

impl Action {
    /// The speed, in units a second, at which the original plays the action's animation at its calibrated rate (`CharVehicle_t`'s
    /// reference speed, +0x170: 5.0 running, 3.0 running backwards, 1.5 walking; FUN_1006f9eb). `None` for the ones not scaled.
    pub fn reference_speed(self) -> Option<f32> {
        match self {
            Action::Run => Some(5.0),
            Action::RunBack => Some(3.0),
            // (mode 4, swimming: `_DAT_1015e6dc`)
            Action::Swim => Some(3.0),
            // (mode 6, sneaking, falls to walking's speed in FUN_1006f9eb: `_DAT_1015e7ac`)
            Action::Walk | Action::WalkBack | Action::StrafeLeft | Action::StrafeRight | Action::Sneak => Some(1.5),
            _ => None,
        }
    }
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
        Action::WalkBack => "walk-back",
        Action::RunBack => "run-back",
        Action::StrafeLeft => "walk-left",
        Action::StrafeRight => "walk-right",
        Action::Swim => "swim",
        Action::IdleSwim => "idle-swim",
        Action::Sneak | Action::SneakStand => "sneakcool",
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

/// The original's animated actions (Gamecode FUN_100c0720 fills them into the map FUN_100c0693 reads; 197 of them): the action's
/// number, its animation's stem and its layer mask (FUN_10010cc2 hands back `~mask & 3` as the layer the animation claims).
/// The social actions below 0x48 are named by their own table (FUN_100c044d) instead.
pub const ACTION_NAMES: &[(u32, &str, u8)] = &[
    (0x64, "walk", 3), (0x65, "run", 3), (0x66, "sneakcool", 3), (0x67, "crawl", 3), (0x68, "crawl_start", 3),
    (0x69, "crawl_stop", 3), (0x6a, "climb", 3), (0x6b, "op1h-button", 2), (0x6c, "op2h-wheel", 2), (0x6d, "wield", 2),
    (0x6e, "wield", 2), (0x6f, "wield", 2), (0x70, "wield", 2), (0x71, "wield", 2), (0x72, "wield", 2), (0x73, "wield", 2),
    (0x74, "wield", 2), (0x75, "wield", 2), (0x76, "push", 3), (0x77, "pull", 3), (0x78, "idle-stand", 3),
    (0x7c, "fall-back", 3), (0x7d, "onback-stop", 3), (0x7e, "imp-back", 3), (0x7f, "imp-chest", 3), (0x80, "imp-head", 3),
    (0x81, "imp-larm", 3), (0x82, "imp-rarm", 3), (0x83, "imp-legs", 3), (0x84, "imp-stomach", 3), (0x85, "swim", 3),
    (0x86, "walk-left", 3), (0x87, "walk-right", 3), (0x88, "walk-back", 3), (0x89, "sneakcool", 3), (0x8a, "sneakgoof", 3),
    (0x8b, "hidecool", 3), (0x8c, "hidecool-start", 3), (0x8d, "hidecool-stop", 3), (0x8e, "hidegoof", 3),
    (0x8f, "hidegoof-start", 3), (0x90, "hidegoof-stop", 3), (0x91, "op1h-button", 3), (0x92, "op1h-keypad", 3),
    (0x93, "op1h-lever", 3), (0x94, "op2h-lever", 3), (0x95, "op2h-wheel", 3), (0x96, "op2h-firstaid", 3),
    (0x97, "opself-firstaid", 2), (0x98, "throw-gren-1h", 3), (0x99, "throw-gren", 3), (0x9a, "idle-crawl", 3),
    (0x9b, "wield-crawl", 3), (0x9c, "jump-stand", 3), (0x9d, "jump-forward", 3), (0x9e, "jump-forward", 3),
    (0x9f, "action-stand", 3), (0xa0, "unarmed-cranekick", 3), (0xa1, "unarmed-cranepunch", 3),
    (0xa2, "unarmed-frontkick-low", 3), (0xa3, "unarmed-palmpunch-double", 3), (0xa4, "unarmed-roundkick-double", 3),
    (0xa5, "unarmed-roundkick-jump", 3), (0xa6, "unarmed-saltokick", 3), (0xa7, "unarmed-sidepunch", 3),
    (0xa8, "unarmed-sidestrike", 3), (0xa9, "unarmed-spinkick", 3), (0xaa, "unarmed-spinkick-double", 3),
    (0xab, "unarmed-swipekick-double", 3), (0xac, "unarmed-capoeira", 3), (0xad, "unarmed-snakestrike", 3),
    (0xae, "unarmed-punch-oneinch", 3), (0xaf, "unarmed-flyingkick", 3), (0xb0, "unarmed-flyingkick-double", 3),
    (0xb1, "unarmed-powersidekick", 3), (0xb2, "idle-hover", 3), (0xb3, "hover-norm", 3), (0xb4, "hover-fast", 3),
    (0xb5, "bow-start", 3), (0xb6, "idle-bow", 3), (0xb7, "bow-shot", 3), (0xb8, "bow-stop", 3), (0xb9, "jump-land-walk", 3),
    (0xba, "jump-land-run", 3), (0xbb, "jump-land-idle", 3), (0xbc, "jump-land-walk-2h", 3), (0xbd, "jump-land-run-2h", 3),
    (0xbe, "jump-land-idle-2h", 3), (0xc3, "idle-swim", 3), (0xc4, "turn-left", 3), (0xc5, "turn-right", 3),
    (0xc8, "spell-gen", 3), (0xc9, "spell-dir", 3), (0xca, "spell-self", 3), (0xcb, "spell-sys", 3), (0xcc, "die-crawl", 3),
    (0xcd, "crawl_enter-rifle", 3), (0xce, "crawl_idle-rifle", 3), (0xcf, "crawl_exit-rifle", 3),
    (0xd0, "crawl_impact-rifle", 3), (0xd1, "crawl_shoot-rifle", 3), (0xd2, "fallback-start", 3), (0xd3, "fallback-stop", 3),
    (0xd4, "idle-fallback", 3), (0xd5, "ground-start", 3), (0xd6, "ground-stop", 3), (0xd7, "idle-ground", 3),
    (0xd8, "chair-start", 3), (0xd9, "chair-stop", 3), (0xda, "idle-chair", 3), (0xdc, "run-left", 3), (0xdd, "run-right", 3),
    (0xde, "run-back", 3), (0xdf, "spell-ant", 3), (0xe0, "unarmed-backstab", 3), (0xe1, "unarmed-charge", 3),
    (0xe2, "unarmed-groinkick", 3), (0xe3, "unarmed-headbutt", 3), (0xe4, "unarmed-spell1", 3), (0xe5, "unarmed-spell2", 3),
    (0xe6, "unarmed-spell3", 3), (0xe7, "crawl_enter-smallarms", 3), (0xe8, "crawl_exit-smallarms", 3),
    (0xe9, "crawl_idle-smallarms", 3), (0xea, "crawl_shoot-smallarmsLh", 3), (0xeb, "crawl_shoot-smallarmsRh", 3),
    (0xec, "crawl_impact-smallarms", 3), (0xed, "sleep-ground", 3), (0xee, "idle-sleep-ground", 3), (0xef, "lounging", 3),
    (0xf0, "idle-lounging", 3), (0xf1, "blade2h_EP3_area_attack", 3), (0xf2, "blade2h_EP3_1-2-chop", 3),
    (0xf3, "blade2h_EP3_dual_attack", 3), (0xf4, "blade2h_EP3_side_sweep", 3), (0xf5, "blade2h_EP3_front_attack", 3),
    (0xf6, "blade1h_EP3_heavystrike", 3), (0xf7, "blade1h_EP3_sidesweep", 3), (0xf8, "unarmed_EP3_faceslap", 3),
    (0xf9, "unarmed_EP3_leg_kick", 3), (0xfa, "unarmed_EP3_slam_heavy", 3), (0xfb, "unarmed-EP3_bodysmash", 3),
    (0xfc, "rifle-burst_EP3_front", 3), (0xfd, "rifle-burst_EP3_spread", 3), (0xfe, "smallarms-burst_EP3_2Xpistol_spread", 3),
    (0xff, "smallarms-burst_EP3_1Xpistol_front", 3), (0x100, "smallarms-burst_EP3_2Xpistol_front", 3), (0x1f4, "die-knees", 3),
    (0x1f5, "die-pain", 3), (0x1f6, "die-poison", 3), (0x1f7, "die-shot", 3), (0x1f8, "die-ground", 3), (0x1f9, "die-float", 3),
    (0x3e8, "blade-start", 3), (0x3e9, "idle-blade", 3), (0x3ea, "blade-stop", 3), (0x3eb, "blade1h-slash", 3),
    (0x3ec, "blade1h-stab", 3), (0x3ed, "blade1hlr-stab", 3), (0x3ee, "blade1hl-slash", 3), (0x3ef, "blade1hl-stab", 3),
    (0x3f2, "smallarms-start", 3), (0x3f3, "idle-smallarms", 3), (0x3f4, "smallarms-stop", 3), (0x3f5, "smallarms-shot", 3),
    (0x3f6, "smallarms-burst", 3), (0x3f7, "smallarms-auto", 3), (0x3f8, "smallarms-shotl", 3), (0x3f9, "smallarms-burstl", 3),
    (0x3fa, "smallarms-autol", 3), (0x3fc, "rifle-start", 3), (0x3fd, "idle-rifle", 3), (0x3fe, "rifle-stop", 3),
    (0x3ff, "rifle-shot", 3), (0x400, "rifle-burst", 3), (0x401, "rifle-auto", 3), (0x406, "unarmed-start", 3),
    (0x407, "idle-unarmed", 3), (0x408, "unarmed-stop", 3), (0x409, "unarmed-kick", 3), (0x40a, "unarmed-rswing", 3),
    (0x40b, "unarmed-uppercut", 3), (0x40c, "unarmed-twopunch", 3), (0x40d, "unarmed-lswing", 3), (0x41a, "blade2h-chop", 3),
    (0x41b, "blade2h-downcut", 3), (0x41c, "blade2h-slash", 3), (0x41d, "blade2h-stab", 3), (0x41e, "idle-2h", 3),
    (0x41f, "2h-start", 3), (0x420, "2h-stop", 3), (0x421, "walk-2h", 3), (0x422, "run-2h", 3), (0x424, "idle-bazooka", 3),
    (0x426, "bazooka-shot", 3), (0x1770, "die-pain", 3), (0x1a0a, "noanim", 3),
];

/// An action's animation stem and layer mask.
pub fn action_name(action: u32) -> Option<(&'static str, u8)> {
    ACTION_NAMES.binary_search_by_key(&action, |a| a.0).ok().map(|i| (ACTION_NAMES[i].1, ACTION_NAMES[i].2))
}

/// The action an action falls back to when the character has no animation for it (Gamecode FUN_10010b3c: a rifle's start,
/// idle, stop and shot fall to the small arms', the 2h walk and run to the plain ones, ...); 0 when there is none. (The turns
/// 0xc4/0xc5 fall to the side steps only for a client character with stat 662 set; taken as not set.)
pub fn action_fallback(action: u32) -> u32 {
    match action {
        0x3fc => 0x3f2,
        0xb4 => 0x65,
        0xcc | 500..=0x1f9 => 6000,
        0xb3 | 0x66 => 100,
        0x7e | 0x80..=0x84 => 0x7f,
        0xb2 | 0x41e => 0x78,
        0x422 => 0x65,
        0x424 => 0x3fd,
        0x426 => 0x3ff,
        0x421 => 100,
        0x3fd => 0x3f3,
        0x3fe => 0x3f4,
        0x3ff => 0x3f5,
        _ => 0,
    }
}

/// The animation files FUN_10010cc2 asks for an action at or above 0x48: `<prefix>_<stem>_01_01.ani`, then `<prefix>_<stem>.ani`
/// (the prefix the breed's and sex's, as [`animation_name`]'s), and the layer the animation claims.
pub fn action_animation_names(breed: u8, sex: u8, action: u32) -> Option<([String; 2], u32)> {
    let (stem, mask) = action_name(action)?;
    let prefix = if breed == ATROX { "athrox" } else if sex == FEMALE { "female" } else { "male" };
    Some(([format!("{prefix}_{stem}_01_01.ani"), format!("{prefix}_{stem}.ani")], (!mask & 3) as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_original_action_names() {
        assert_eq!(ACTION_NAMES.len(), 197);
        assert_eq!(action_name(0x3f5), Some(("smallarms-shot", 3)));
        assert_eq!(action_name(0x97), Some(("opself-firstaid", 2)));
        assert_eq!(action_animation_names(SOLITUS, MALE, 0x3eb).unwrap().0[0], "male_blade1h-slash_01_01.ani");
        assert_eq!(action_animation_names(ATROX, MALE, 0x40a).unwrap(), (["athrox_unarmed-rswing_01_01.ani".to_string(), "athrox_unarmed-rswing.ani".to_string()], 0));
        // (a rifle's shot falls to the small arms' when the body has none)
        assert_eq!(action_fallback(0x3ff), 0x3f5);
        assert_eq!(action_fallback(0x3f5), 0);
    }

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
