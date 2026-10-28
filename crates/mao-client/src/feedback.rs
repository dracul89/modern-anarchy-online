//! The fight lines of the Combat tab in the original's own words: Gamecode FUN_10012c3e formats each feedback event (FUN_10012a87
//! queues it: type, victim, amount, source, text, damage type, hit type, nano) with `LDBface::GetText(110, ElfHash(name))` from
//! `cd_image/text/text.mdb` and `LDBformat::Feed`. The texts are read from the game's file; the English ones are kept here too
//! (copied from that file) for when it can't be read.

use mao_formats::ldb::{self, Arg};

/// The game's text database (`cd_image/text/text.mdb`), read once it can be (the game's folder may be set later).
pub fn texts() -> Option<&'static ldb::Texts> {
    static TEXTS: std::sync::OnceLock<ldb::Texts> = std::sync::OnceLock::new();
    if let Some(t) = TEXTS.get() {
        return Some(t);
    }
    let parsed = ldb::Texts::parse(&std::fs::read(crate::theme::game_dir()?.join("cd_image/text/text.mdb")).ok()?)?;
    Some(TEXTS.get_or_init(|| parsed))
}

/// The feedback text of a name, filled in order.
fn line(name: &str, args: &[Arg]) -> String {
    let text = texts().and_then(|t| t.named(ldb::FEEDBACK, name)).unwrap_or_else(|| english(name));
    ldb::format(text, args)
}

/// The English texts of the names used here (text.mdb, category 110).
fn english(name: &str) -> &'static str {
    match name {
        "Feedback_FallDamage" => "You took %u points of fall damage.",
        "Feedback_AttackedByForPointsOfDamage" => "You were attacked with %s for %u points of %s damage.",
        "Feedback_YouWereAttackedByNanobotsFrom" => "You were attacked with nanobots from %s for %u points of %s damage.",
        "Feedback_AttackedByNanobotsForPointsOfDamage" => "You were attacked with nanobots for %u points of %s damage.",
        "Feedback_TookPointsOfFallDamage" => "%s took %u points of fall damage.",
        "Feedback_WasAttackedByForPointsOfDamageFrom" => "%s was attacked with %s from %s for %u points of %s damage.",
        "Feedback_WasAttackedByForPointsOfDamage" => "%s was attacked with %s for %u points of %s damage.",
        "Feedback_WasAttackedByNanobotsFrom" => "%s was attacked with nanobots from %s for %u points of %s damage.",
        "Feedback_WasAttackedByNanobots" => "%s was attacked with nanobots for %u points of %s damage.",
        "Feedback_YouHitOtherWith" => "You hit %s with %s for %u points of %s damage.",
        "Feedback_YouHitOtherByNanobots" => "You hit %s with nanobots for %u points of %s damage.",
        "Feedback_HitYouForPointsOfDamage" => "%s hit you for %u points of %s damage.",
        "Feedback_WereHitForPointsOfDamage" | "Feedback_YouWereHitForPointsOfDamage" => "You were hit for %u points of damage.",
        "Feedback_PlayerHitYouForPointsOfDamage" => "Player %s hit you for %u points of %s damage.",
        "Feedback_PlayerHitYou" => "A player hit you for %u points of damage.",
        "Feedback_HitWithSpecial" => "You hit %s for %u points of %s damage.",
        "Feedback_OtherHitOther" => "%s hit %s for %u points of %s damage.",
        "Feedback_SomethingHitOther" => "Something hit %s for %u points of %s damage.",
        "Feedback_HealedForPoints" => "You were healed for %u points.",
        "TryToAttackWithSpecialButMiss" => "You try to attack %s with %s, but you miss!",
        "Feedback_YouTriedToHitMissed" => "You tried to hit %s, but missed!",
        "OtherTriesToAttackWithSpecialButMisses" => "%s tries to attack you with %s, but misses!",
        "Feedback_OtherTriedToHitMissed" => "%s tried to hit you, but missed!",
        "Feedback_YouAbsorbedDamage" => "You absorbed %u points of %s damage.",
        "Feedback_SomeoneAbsorbedDamage" => "Someone absorbed %u points of %s damage.",
        "Feedback_SomeonesDamageShieldHitYou" => "Someone's damage shield hit you for %u points of damage.",
        "Feedback_YourDamageShieldHitOther" => "Your damage shield hit %s for %u points of damage.",
        "Feedback_OtherDamageShieldHitOther" => "%s's damage shield hit %s for %u points of damage.",
        "Feedback_SomethingHitOtherWithDamageShield" => "Something hit %s for %u points of damage by damage shield.",
        "Feedback_SomeonesReflectShieldHitYou" => "Someone's reflect shield hit you for %u points of damage.",
        "Feedback_YourReflectShieldHitOther" => "Your reflect shield hit %s for %u points of damage.",
        "Feedback_OtherReflectShieldHitOther" => "%s's reflect shield hit %s for %u points of damage.",
        "Feedback_SomethingHitOtherWithReflectShield" => "Something hit %s for %u points of damage by reflect shield.",
        "Feedback_LostXP" => "You lost %u xp.",
        "Feedback_ReceivedXP" => "You received %u xp.",
        "Feedback_GainedAlienXP" => "You gained %d new Alien Experience Points.",
        "Feedback_LostSK" => "You lost %d points of Shadowknowledge.",
        "Feedback_GainedSK" => "You gained %d points of Shadowknowledge.",
        "Feedback_ExecutingNanoProgram" => "Executing Nano Program: %s.",
        "Feedback_Reloading" => "Reloading...",
        "Feedback_ReloadingCompleted" => "Reloading completed",
        "Feedback_CriticalHit" => "Critical hit!",
        "Feedback_GlancingHit" => "Glancing hit.",
        "Feedback_MustSitToLoungeOrSleep" => "You must sit in order to use /lounge or /sleep.",
        "Feedback_MineWasSprung" => "The mine was sprung! You failed disarming the mine!",
        "Feedback_ATrapWasSprung" => "A trap was sprung! You failed disarming the trap!.",
        "Feedback_YouSuccededDisarmingMine" => "You succeeded disarming the mine!",
        "Feedback_YouSuccededDisarmingTrap" => "You succeded disarming the trap!.",
        _ => "",
    }
}

/// The name of a damage type (FUN_10036cf5, from the names FUN_1003268e registers by the armour-class stat they are met by);
/// no type (0) is 0x1b, "unknown" (FUN_10012c3e's default).
pub fn damage_type(stat: u32) -> &'static str {
    match stat {
        90 => "projectile",
        91 => "melee",
        92 => "energy",
        93 => "chemical",
        94 => "radiation",
        95 => "cold",
        96 => "poison",
        97 => "fire",
        0xa8 => "nano",
        0x1da => "fall",
        _ => "unknown",
    }
}

/// The damage type of a weapon's attack (FUN_1009b525: the weapon's stat 436 when it is one of the types, 90..=97 or 0xa8,
/// else projectile).
pub fn weapon_damage_type(stat_436: Option<i64>) -> u32 {
    match stat_436 {
        Some(t @ (90..=97 | 0xa8)) => t as u32,
        _ => 90,
    }
}

/// Who is in a line: the client's own character, another one by name (and whether a player), or one the client doesn't know.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Who<'a> {
    You,
    Named(&'a str, bool),
    Unknown,
}

impl<'a> Who<'a> {
    fn name(self) -> Option<&'a str> {
        match self {
            Who::Named(n, _) => Some(n),
            _ => None,
        }
    }
}

/// A weapon hit (`AttackInfo`, FUN_1009f257 → FUN_1006ae3c → FUN_1009b6b7): the victim, the attacker, the amount, the damage type
/// and the hit type (4 critical, 2 glancing: FUN_10012c3e adds those lines after the hit's).
pub fn attack(victim: Who, attacker: Who, amount: i64, kind: u32, hit: i64) -> Option<String> {
    if amount == 0 {
        return None;
    }
    let (n, kind) = (Arg::Int(amount.abs()), Arg::Text(damage_type(kind)));
    let text = match (victim, attacker) {
        // (an attacker the client doesn't know: FUN_1009f257 says only that the client was hit, FUN_100698ec)
        (Who::You, Who::Unknown) => line("Feedback_YouWereHitForPointsOfDamage", &[n]),
        (_, Who::Unknown) | (Who::Unknown, _) => return None,
        // (0x1e: a player hit the client; 0x1d: anything else did, "You were hit" when it hit itself)
        (Who::You, Who::Named(name, true)) => line("Feedback_PlayerHitYouForPointsOfDamage", &[Arg::Text(name), n, kind]),
        (Who::You, Who::Named(name, false)) => line("Feedback_HitYouForPointsOfDamage", &[Arg::Text(name), n, kind]),
        (Who::You, Who::You) => line("Feedback_WereHitForPointsOfDamage", &[n]),
        // (0x1f: the client hit something)
        (Who::Named(name, _), Who::You) => line("Feedback_HitWithSpecial", &[Arg::Text(name), n, kind]),
        // (0x20/0x21: others)
        (Who::Named(victim, _), Who::Named(attacker, _)) => line("Feedback_OtherHitOther", &[Arg::Text(attacker), Arg::Text(victim), n, kind]),
    };
    // (the critical or glancing line is joined on after a space)
    Some(match hit {
        4 => format!("{text} {}", line("Feedback_CriticalHit", &[])),
        2 => format!("{text} {}", line("Feedback_GlancingHit", &[])),
        _ => text,
    })
}

/// A miss (`MissedAttackInfo`, FUN_100a1077 → FUN_1006b399, feedback 0x3b): only the client's own misses and the misses at it
/// have a line; a skill named (the message's stat) says what the attack was made with.
pub fn miss(defender: Who, attacker: Who, skill: Option<&str>) -> Option<String> {
    match (attacker, defender) {
        (Who::You, defender) => {
            let name = defender.name()?;
            Some(match skill {
                Some(s) => line("TryToAttackWithSpecialButMiss", &[Arg::Text(name), Arg::Text(s)]),
                None => line("Feedback_YouTriedToHitMissed", &[Arg::Text(name)]),
            })
        }
        (attacker, Who::You) => {
            let name = attacker.name()?;
            Some(match skill {
                Some(s) => line("OtherTriesToAttackWithSpecialButMisses", &[Arg::Text(name), Arg::Text(s)]),
                None => line("Feedback_OtherTriedToHitMissed", &[Arg::Text(name)]),
            })
        }
        _ => None,
    }
}

/// A health change (`HealthDamage`, FUN_100a0611: the one hurt is the message's own identity, the source its `Target`): damage
/// to the client (0x1a), by the client (0x40) or between others (0x1b); healing of the client only (0x22). `nano` is the name
/// of the item the message names (its last field, an item template), if any.
pub fn health(victim: Who, source: Who, amount: i64, stat: u32, nano: Option<&str>) -> Option<String> {
    if amount == 0 {
        return None;
    }
    let n = Arg::Int(amount.abs());
    let kind = Arg::Text(damage_type(if stat == 0 { 0x1b } else { stat }));
    // (without a nano, a source that is the one hurt counts as none)
    let from = if source == victim && nano.is_none() { None } else { source.name() };
    if amount > 0 {
        return (victim == Who::You).then(|| line("Feedback_HealedForPoints", &[n]));
    }
    Some(match victim {
        Who::You => match (stat, nano, from) {
            (0x1da, _, _) => line("Feedback_FallDamage", &[n]),
            (_, Some(nano), _) => line("Feedback_AttackedByForPointsOfDamage", &[Arg::Text(nano), n, kind]),
            (_, None, Some(from)) => line("Feedback_YouWereAttackedByNanobotsFrom", &[Arg::Text(from), n, kind]),
            (_, None, None) => line("Feedback_AttackedByNanobotsForPointsOfDamage", &[n, kind]),
        },
        Who::Named(victim, _) if source == Who::You => match nano {
            Some(nano) => line("Feedback_YouHitOtherWith", &[Arg::Text(victim), Arg::Text(nano), n, kind]),
            None => line("Feedback_YouHitOtherByNanobots", &[Arg::Text(victim), n, kind]),
        },
        Who::Named(victim, _) => match (stat, nano, from) {
            (0x1da, _, _) => line("Feedback_TookPointsOfFallDamage", &[Arg::Text(victim), n]),
            (_, Some(nano), Some(from)) => line("Feedback_WasAttackedByForPointsOfDamageFrom", &[Arg::Text(victim), Arg::Text(nano), Arg::Text(from), n, kind]),
            (_, Some(nano), None) => line("Feedback_WasAttackedByForPointsOfDamage", &[Arg::Text(victim), Arg::Text(nano), n, kind]),
            (_, None, Some(from)) => line("Feedback_WasAttackedByNanobotsFrom", &[Arg::Text(victim), Arg::Text(from), n, kind]),
            (_, None, None) => line("Feedback_WasAttackedByNanobots", &[Arg::Text(victim), n, kind]),
        },
        Who::Unknown => return None,
    })
}

/// Damage soaked up (`Absorb`, FUN_1009ef78: the one absorbing is the message's identity): 0x34 for the client, 0x35 for the
/// others, who are never named (the handler passes no source, so FUN_10012c3e takes its "Someone" branch).
pub fn absorb(who: Who, amount: i64, kind: u32) -> Option<String> {
    if amount == 0 {
        return None;
    }
    let (n, kind) = (Arg::Int(amount), Arg::Text(damage_type(if kind == 0 { 0x1b } else { kind })));
    match who {
        Who::You => Some(line("Feedback_YouAbsorbedDamage", &[n, kind])),
        Who::Named(..) => Some(line("Feedback_SomeoneAbsorbedDamage", &[n, kind])),
        Who::Unknown => None,
    }
}

/// A damage shield (`ShieldAttack`, FUN_100a134d: 0x25/0x26) or reflect shield (`ReflectAttack`, FUN_100a11c2: 0x27/0x28)
/// striking: the one hurt is the message's identity, the shield's owner its `Target`. When the client is the one hurt the
/// handler drops the owner, so the line is always "Someone's".
pub fn shield(victim: Who, owner: Who, amount: i64, reflect: bool) -> Option<String> {
    if amount == 0 {
        return None;
    }
    let n = Arg::Int(amount);
    let pick = |damage: &'static str, reflected: &'static str| if reflect { reflected } else { damage };
    Some(match (victim, owner) {
        (Who::You, _) => line(pick("Feedback_SomeonesDamageShieldHitYou", "Feedback_SomeonesReflectShieldHitYou"), &[n]),
        (Who::Named(victim, _), Who::You) => line(pick("Feedback_YourDamageShieldHitOther", "Feedback_YourReflectShieldHitOther"), &[Arg::Text(victim), n]),
        (Who::Named(victim, _), Who::Named(owner, _)) => line(pick("Feedback_OtherDamageShieldHitOther", "Feedback_OtherReflectShieldHitOther"), &[Arg::Text(owner), Arg::Text(victim), n]),
        (Who::Named(victim, _), Who::Unknown) => line(pick("Feedback_SomethingHitOtherWithDamageShield", "Feedback_SomethingHitOtherWithReflectShield"), &[Arg::Text(victim), n]),
        (Who::Unknown, _) => return None,
    })
}

/// A change of one of the client's own stats (the `Stat` message, FUN_100a2006, by the change from the value held): its health
/// (27: 0x1a / 0x22, no source and no type), experience (52: 0x24), alien experience (40, gains only: 0x45) and
/// shadowknowledge (573: 0x3e).
pub fn own_stat(stat: u32, change: i64) -> Option<String> {
    if change == 0 {
        return None;
    }
    let n = Arg::Int(change.abs());
    Some(match stat {
        27 if change < 0 => line("Feedback_AttackedByNanobotsForPointsOfDamage", &[n, Arg::Text(damage_type(0x1b))]),
        27 => line("Feedback_HealedForPoints", &[n]),
        52 if change < 0 => line("Feedback_LostXP", &[n]),
        52 => line("Feedback_ReceivedXP", &[n]),
        40 if change > 0 => line("Feedback_GainedAlienXP", &[n]),
        573 if change < 0 => line("Feedback_LostSK", &[n]),
        573 => line("Feedback_GainedSK", &[n]),
        _ => return None,
    })
}

/// A feedback text with nothing to fill (FUN_10059047: a name hashed and told as it is).
pub fn plain(name: &str) -> String {
    line(name, &[])
}

/// The client starting a nano program (N3Msg_CastNanoSpell, 0x1001b5d4: feedback 0x47 with the nano's name).
pub fn executing(nano: &str) -> String {
    line("Feedback_ExecutingNanoProgram", &[Arg::Text(nano)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_lines() {
        let rat = Who::Named("Rollerrat", false);
        assert_eq!(attack(rat, Who::You, -49, 92, 3).unwrap(), "You hit Rollerrat for 49 points of energy damage.");
        assert_eq!(attack(Who::You, rat, 8, 91, 4).unwrap(), "Rollerrat hit you for 8 points of melee damage. Critical hit!");
        assert_eq!(attack(Who::You, Who::Named("Bob", true), 8, 90, 2).unwrap(), "Player Bob hit you for 8 points of projectile damage. Glancing hit.");
        assert_eq!(attack(rat, Who::Named("Bob", true), 8, 90, 3).unwrap(), "Bob hit Rollerrat for 8 points of projectile damage.");
        assert_eq!(attack(Who::You, Who::Unknown, 8, 90, 3).unwrap(), "You were hit for 8 points of damage.");
        assert_eq!(attack(rat, Who::Unknown, 8, 90, 3), None);
        assert_eq!(attack(rat, Who::You, 0, 90, 3), None);
        assert_eq!(weapon_damage_type(Some(95)), 95);
        assert_eq!(weapon_damage_type(Some(0)), 90);
    }

    #[test]
    fn miss_lines() {
        let rat = Who::Named("Rollerrat", false);
        assert_eq!(miss(rat, Who::You, None).unwrap(), "You tried to hit Rollerrat, but missed!");
        assert_eq!(miss(Who::You, rat, None).unwrap(), "Rollerrat tried to hit you, but missed!");
        assert_eq!(miss(Who::You, rat, Some("Aimed Shot")).unwrap(), "Rollerrat tries to attack you with Aimed Shot, but misses!");
        assert_eq!(miss(rat, Who::Named("Bob", true), None), None);
    }

    #[test]
    fn shield_absorb_and_stat_lines() {
        let rat = Who::Named("Rollerrat", false);
        // (captured: the client's damage shield struck 0x7b017c24 for 173; the client absorbed 1581 cold)
        assert_eq!(shield(rat, Who::You, 173, false).unwrap(), "Your damage shield hit Rollerrat for 173 points of damage.");
        assert_eq!(shield(Who::You, rat, 9, true).unwrap(), "Someone's reflect shield hit you for 9 points of damage.");
        assert_eq!(shield(rat, Who::Named("Bob", true), 9, false).unwrap(), "Bob's damage shield hit Rollerrat for 9 points of damage.");
        assert_eq!(shield(Who::You, rat, 0, true), None);
        assert_eq!(absorb(Who::You, 1581, 95).unwrap(), "You absorbed 1581 points of cold damage.");
        assert_eq!(absorb(rat, 10, 91).unwrap(), "Someone absorbed 10 points of melee damage.");
        assert_eq!(own_stat(52, 1200).unwrap(), "You received 1200 xp.");
        assert_eq!(own_stat(52, -5).unwrap(), "You lost 5 xp.");
        assert_eq!(own_stat(40, -5), None);
        assert_eq!(own_stat(573, 3).unwrap(), "You gained 3 points of Shadowknowledge.");
        assert_eq!(own_stat(27, 40).unwrap(), "You were healed for 40 points.");
        assert_eq!(own_stat(1, 40), None);
        assert_eq!(executing("Heal Me"), "Executing Nano Program: Heal Me.");
    }

    #[test]
    fn health_lines() {
        let rat = Who::Named("Rollerrat", false);
        // (captured: the client's nuke on a monster, cold against the client, its own heal)
        assert_eq!(health(rat, Who::You, -2833, 91, None).unwrap(), "You hit Rollerrat with nanobots for 2833 points of melee damage.");
        assert_eq!(health(Who::You, rat, -1581, 95, None).unwrap(), "You were attacked with nanobots from Rollerrat for 1581 points of cold damage.");
        assert_eq!(health(Who::You, Who::You, 116, 0, None).unwrap(), "You were healed for 116 points.");
        assert_eq!(health(rat, rat, 49, 0, None), None);
        assert_eq!(health(rat, rat, -49, 92, None).unwrap(), "Rollerrat was attacked with nanobots for 49 points of energy damage.");
        assert_eq!(health(Who::You, Who::Unknown, -7, 0, None).unwrap(), "You were attacked with nanobots for 7 points of unknown damage.");
        assert_eq!(health(rat, Who::Named("Bob", true), -7, 97, Some("Fireball")).unwrap(), "Rollerrat was attacked with Fireball from Bob for 7 points of fire damage.");
        assert_eq!(health(Who::You, rat, -7, 0x1da, None).unwrap(), "You took 7 points of fall damage.");
    }
}
