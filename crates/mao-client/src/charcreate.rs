//! Character creation's rules, from GUI.dll's `CharCreateModule_t` and its scenes: the screens' breed choice and profession
//! choice turned into the game's numbers (`SceneBase_t::ConvertCCBreedToGCBreedAndSex` 0x10122aa3,
//! `ConvertCCProfToGCProf` 0x10122a1a), the heads each breed and sex may have (`HeadMeshData_t`'s table, FUN_1011d692),
//! the height and build values the creation sends (`NameScene_t::SetState` 0x1011fc0f, case 0x1006), the name's rules
//! (FUN_10123cdc, from `StartServerCreation` 0x1012015d) and the login server's refusals (`SetState`'s codes, texts of
//! category 600). The message itself is `mao_net::create`.

/// The screen's seven breed choices (`CCSelectedBreed`, 1 to 7) as the game's breed and sex: Solitus female and male,
/// Opifex female and male, Nanomage female and male, Atrox.
pub fn breed_and_sex(choice: i32) -> Option<(i32, i32)> {
    Some(match choice {
        1 => (1, 3),
        2 => (1, 2),
        3 => (2, 3),
        4 => (2, 2),
        5 => (3, 3),
        6 => (3, 2),
        7 => (4, 1),
        _ => return None,
    })
}

/// The screen's fourteen profession choices (`CCSelectedProfession`, 1 to 14) as the game's professions (13 for any other).
pub fn profession(choice: i32) -> i32 {
    match choice {
        1 => 12,
        2 => 6,
        3 => 3,
        4 => 1,
        5 => 14,
        6 => 15,
        7 => 4,
        8 => 5,
        9 => 7,
        10 => 10,
        11 => 9,
        12 => 8,
        13 => 2,
        14 => 11,
        _ => 13,
    }
}

/// The height sent for the screen's choice (`CCSelectedHeight`: 0 short, 1 normal, 2 tall).
pub fn height(choice: i32) -> i32 {
    match choice {
        0 => 90,
        1 => 100,
        _ => 110,
    }
}

/// The head meshes' file names a breed and sex may wear, with each set's race (1, 2 African, 3 Asian) and how many of its
/// numbers are tried (the original's counts without the expansion flag `ExpansionFlags & 2`, which raises some of them);
/// the Opifex female set skips its number 30.
pub fn head_sets(breed: i32, sex: i32) -> &'static [(&'static str, u8, u32)] {
    match (breed, sex) {
        (2, 3) => &[("head_opifexfemale", 1, 32)],
        (2, _) => &[("head_opifexmale", 1, 30)],
        (1, 3) => &[("head_solitusfemale", 1, 50), ("head_solitusfemale_african", 2, 8), ("head_solitusfemale_asian", 3, 7)],
        (1, _) => &[("head_solitusmale", 1, 51), ("head_solitusmale_african", 2, 9), ("head_solitusmale_asian", 3, 6)],
        (3, 2) => &[("head_nanomale", 1, 30)],
        (3, _) => &[("head_nanofemale", 1, 30)],
        (4, _) => &[("head_athrox", 1, 30)],
        _ => &[],
    }
}

/// The heads offered, in order: each set's numbered meshes (`<set><nn>.abiff`, type 0xf6951) that the game has (`find`
/// gives a name's instance), with their race.
pub fn heads(breed: i32, sex: i32, find: &dyn Fn(&str) -> Option<u32>) -> Vec<(u32, u8, String)> {
    let mut out = Vec::new();
    for &(set, race, count) in head_sets(breed, sex) {
        for n in 0..count {
            if set == "head_opifexfemale" && n == 30 {
                continue;
            }
            let name = format!("{set}{n:02}.abiff");
            if let Some(mesh) = find(&name) {
                out.push((mesh, race, name));
            }
        }
    }
    out
}

/// FUN_10123cdc's verdict on a name, as the text `StartServerCreation` shows (category 600): 4 to 12 characters, letters,
/// digits only at the end and from the fifth character on, nothing else; the case is the client's to set (see [`proper`]).
pub fn name_problem(name: &str) -> Option<&'static str> {
    let chars: Vec<char> = name.chars().collect();
    if name.is_empty() {
        return Some("MustEnterName");
    }
    if !(4..=12).contains(&chars.len()) {
        return Some("NameMustBeBetween3And13Chars");
    }
    let mut digits = false;
    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_digit() {
            if i < 4 {
                return Some("DigitsAtEndAfter4");
            }
            digits = true;
        } else if !c.is_ascii_alphabetic() {
            return Some("NameMustBeAlphanum");
        } else if digits {
            return Some("DigitsAtEndAfter4");
        }
    }
    None
}

/// The name in the case the original wants (a capital, then small letters: FUN_10123cdc's code 3).
pub fn proper(name: &str) -> String {
    name.chars().enumerate().map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() }).collect()
}

/// The text (category 600) for a refusal code of the login server (`NameScene_t::SetState`).
pub fn refusal(code: i32) -> &'static str {
    match code {
        1 => "PlayerNotFound",
        2 => "IllegalPassword",
        3 => "ContactAdministration",
        4 => "CharacterNotFound",
        5 => "AccountHasIllegalPassword",
        6 => "CharacterAlreadyLoggedIn",
        7 => "CharacterProblem",
        8 => "CantCreateMoreChars",
        9 => "ServerProblem",
        10 => "ServerDown",
        19 => "AccountNotPaid",
        20 => "PlayerAlreadyLoggedIn",
        21 => "AccountLocked",
        28 => "ServerWrongRDB",
        30 => "NicknameTaken",
        31 => "NicknameInvalid",
        _ => "ServerProblem",
    }
}

/// A text of category 600 from text.mdb (its name when the file has none).
pub fn text(name: &str) -> String {
    crate::feedback::texts().and_then(|t| t.named(600, name)).map(str::to_string).unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules() {
        assert_eq!(breed_and_sex(1), Some((1, 3)));
        assert_eq!(breed_and_sex(7), Some((4, 1)));
        assert_eq!(profession(4), 1);
        assert_eq!(profession(13), 2);
        assert_eq!((height(0), height(1), height(2)), (90, 100, 110));
        assert_eq!(name_problem("Sad"), Some("NameMustBeBetween3And13Chars"));
        assert_eq!(name_problem("Sadlee"), None);
        assert_eq!(name_problem("Sadl2e"), Some("DigitsAtEndAfter4"));
        assert_eq!(name_problem("Sa2dle"), Some("DigitsAtEndAfter4"));
        assert_eq!(name_problem("Sadle22"), None);
        assert_eq!(name_problem("Sad-lee"), Some("NameMustBeAlphanum"));
        assert_eq!(proper("sADLEE"), "Sadlee");
        let found = |n: &str| (n == "head_opifexfemale00.abiff" || n == "head_opifexfemale30.abiff" || n == "head_opifexfemale31.abiff").then_some(n.len() as u32);
        let got: Vec<String> = heads(2, 3, &found).into_iter().map(|h| h.2).collect();
        assert_eq!(got, vec!["head_opifexfemale00.abiff", "head_opifexfemale31.abiff"]);
    }
}
