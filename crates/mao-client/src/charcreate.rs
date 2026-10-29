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

// ---- The screens ----

/// The breed choices' names, in the original's order (`CCSelectedBreed` 1 to 7).
pub const BREED_CHOICES: [&str; 7] = ["Solitus Female", "Solitus Male", "Opifex Female", "Opifex Male", "Nanomage Female", "Nanomage Male", "Atrox"];

/// The texts' names for a game breed (`<Breed>Description`) and a profession (`Description<Profession>`).
fn breed_key(breed: i32) -> &'static str {
    match breed {
        1 => "Solitus",
        2 => "Opifex",
        3 => "Nanomage",
        _ => "Atrox",
    }
}

fn profession_key(profession: i32) -> &'static str {
    match profession {
        1 => "Soldier",
        2 => "MartialArtist",
        3 => "Engineer",
        4 => "Fixer",
        5 => "Agent",
        6 => "Adventurer",
        7 => "Trader",
        8 => "Bureaucrat",
        9 => "Enforcer",
        10 => "Doctor",
        11 => "NanoTechnician",
        12 => "Metaphysicist",
        14 => "Keeper",
        _ => "Shade",
    }
}

/// A description's text without its `<font>` and other tags (`<br>` a new line).
pub fn plain_text(markup: &str) -> String {
    let mut out = String::new();
    let mut rest = markup;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else { break };
        let tag = rest[start + 1..start + end].trim().to_ascii_lowercase();
        if tag == "br" || tag == "br/" || tag == "p" {
            out.push('\n');
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    // (and the entities the texts use)
    out.replace("\r\n", "\n").replace("&#8217;", "\u{2019}").replace("&#8216;", "\u{2018}").replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The account a creation logs in with (each request logs in on a connection of its own, as the list does).
#[derive(Clone)]
pub struct Account {
    pub user: String,
    pub password: String,
    pub server: String,
    pub version: String,
}

/// What the screens want of the launcher.
pub enum Outcome {
    Stay,
    Back,
    /// Made: the new character's name.
    Created(String),
}

/// The creation under way: the choices of the four steps (breed, profession, appearance, name).
pub struct Creating {
    pub step: usize,
    /// The breed choice (1 to 7, 0 none yet) and the profession choice (1 to 14, 0 none yet).
    pub breed: i32,
    pub profession: i32,
    pub head: usize,
    /// 0 short, 1 normal, 2 tall; 0 small, 1 medium, 2 large.
    pub height: i32,
    pub build: i32,
    pub name: String,
    message: String,
    pending: Option<(bool, std::sync::mpsc::Receiver<Result<mao_net::create::Reply, String>>)>,
}

impl Default for Creating {
    fn default() -> Self {
        Creating { step: 0, breed: 0, profession: 0, head: 0, height: 1, build: 1, name: String::new(), message: String::new(), pending: None }
    }
}

/// The game's instance index (names to instances), read once.
fn heads_of(game: &str, breed: i32, sex: i32) -> Vec<(u32, u8, String)> {
    static INDEX: std::sync::OnceLock<Option<mao_formats::instances::Instances>> = std::sync::OnceLock::new();
    let index = INDEX.get_or_init(|| {
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(game)).ok()?;
        mao_formats::instances::Instances::parse(&db.get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 }).ok()??).ok()
    });
    match index {
        Some(index) => heads(breed, sex, &|n| index.find(0xf6951, n)),
        None => Vec::new(),
    }
}

impl Creating {
    /// The game's breed and sex chosen (none before a choice).
    pub fn breed_and_sex(&self) -> Option<(i32, i32)> {
        breed_and_sex(self.breed)
    }

    /// The chosen head's mesh (the breed's `head`-th), for the preview.
    pub fn head_mesh(&self, game: &str) -> Option<u32> {
        let (breed, sex) = self.breed_and_sex()?;
        heads_of(game, breed, sex).get(self.head).map(|h| h.0)
    }

    fn ask(&mut self, account: &Account, frame: mao_net::frame::Frame, is_create: bool) {
        let (tx, rx) = std::sync::mpsc::channel();
        let account = account.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<mao_net::create::Reply, String> {
                let mut connection = mao_net::session::Connection::connect(account.server.as_str()).map_err(|e| e.to_string())?;
                mao_net::session::login(&mut connection, &account.user, &account.password, &account.version).map_err(|e| e.to_string())?;
                let reply = mao_net::session::ask_creation(&mut connection, &frame).map_err(|e| e.to_string());
                connection.close();
                reply
            })();
            let _ = tx.send(result);
        });
        self.pending = Some((is_create, rx));
        self.message.clear();
    }

    /// The window of the step: the choices, the step's texts from text.mdb, Back and Next (and Suggest Name and Create on the
    /// last step).
    pub fn show(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette, account: &Account, game: &str) -> Outcome {
        let mut outcome = Outcome::Stay;
        // an answer from the login server
        if let Some((is_create, rx)) = &self.pending {
            let is_create = *is_create;
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(mao_net::create::Reply::Name(name)) => self.name = name,
                        Ok(mao_net::create::Reply::Created(_)) if is_create => outcome = Outcome::Created(self.name.clone()),
                        Ok(mao_net::create::Reply::Refused(code)) => self.message = text(refusal(code)),
                        Ok(_) => {}
                        Err(e) => self.message = e,
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
                Err(_) => self.pending = None,
            }
        }
        let busy = self.pending.is_some();
        let titles = ["Breed", "Profession", "Appearance", "Name"];
        let mut open = true;
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let title = format!("New Character: {}", titles[self.step]);
        crate::chrome::window(ctx, palette, "NewCharacter", &title, &mut open, |w| w.resizable(false).anchor(egui::Align2::LEFT_CENTER, egui::vec2(24.0, 0.0)).default_width(380.0), |ui| {
            ui.set_width(380.0);
            match self.step {
                0 => {
                    ui.label(plain_text(&text("BreedHelp")));
                    ui.add_space(6.0);
                    egui::Grid::new("cc-breeds").num_columns(2).spacing(egui::vec2(6.0, 4.0)).show(ui, |ui| {
                        for (i, name) in BREED_CHOICES.iter().enumerate() {
                            let choice = i as i32 + 1;
                            if ui.selectable_label(self.breed == choice, *name).clicked() && self.breed != choice {
                                self.breed = choice;
                                self.head = 0;
                            }
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                    ui.add_space(6.0);
                    match self.breed_and_sex() {
                        Some((breed, _)) => {
                            let picked = BREED_CHOICES[self.breed as usize - 1].replace(' ', "");
                            ui.colored_label(accent, text(&format!("{picked}Selected")));
                            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                                ui.label(plain_text(&text(&format!("{}Description", breed_key(breed)))));
                            });
                        }
                        None => {
                            ui.colored_label(accent, text("ChooseBreed"));
                        }
                    }
                }
                1 => {
                    egui::Grid::new("cc-professions").num_columns(2).spacing(egui::vec2(6.0, 4.0)).show(ui, |ui| {
                        for choice in 1..=14 {
                            let name = crate::launcher::profession(profession(choice));
                            if ui.selectable_label(self.profession == choice, name).clicked() {
                                self.profession = choice;
                            }
                            if choice % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                    ui.add_space(6.0);
                    if self.profession > 0 {
                        let key = profession_key(profession(self.profession));
                        ui.colored_label(accent, text(&format!("{key}Selected")));
                        // (the Meta-Physicist's description is GUI.dll's `DescriptionMetaPhysicists`)
                        let description = if key == "Metaphysicist" { "DescriptionMetaPhysicists".to_string() } else { format!("Description{key}") };
                        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                            ui.label(plain_text(&text(&description)));
                        });
                    }
                }
                2 => {
                    ui.label(text("SelectAppearance"));
                    ui.add_space(6.0);
                    let (breed, sex) = self.breed_and_sex().unwrap_or((1, 2));
                    let heads = heads_of(game, breed, sex);
                    ui.horizontal(|ui| {
                        ui.label("Head:");
                        if ui.add_enabled(self.head > 0, egui::Button::new("<")).clicked() {
                            self.head -= 1;
                        }
                        ui.label(format!("{} of {}", self.head + 1, heads.len().max(1)));
                        if ui.add_enabled(self.head + 1 < heads.len(), egui::Button::new(">")).clicked() {
                            self.head += 1;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        for (i, name) in ["Short", "Normal", "Tall"].iter().enumerate() {
                            ui.radio_value(&mut self.height, i as i32, *name);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Build:");
                        for (i, name) in ["Small", "Medium", "Large"].iter().enumerate() {
                            ui.radio_value(&mut self.build, i as i32, *name);
                        }
                    });
                }
                _ => {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(170.0).char_limit(12));
                        if ui.add_enabled(!busy, egui::Button::new("Suggest Name")).clicked() {
                            if let Some((breed, sex)) = self.breed_and_sex() {
                                self.ask(account, mao_net::create::suggest_name(breed, sex, profession(self.profession)), false);
                            }
                        }
                    });
                    let (breed, sex) = self.breed_and_sex().unwrap_or((1, 2));
                    ui.label(format!(
                        "{} {}, {}, {} height, {} build",
                        crate::launcher::breed(breed),
                        crate::launcher::sex(sex),
                        crate::launcher::profession(profession(self.profession)),
                        ["short", "normal", "tall"][self.height as usize],
                        ["small", "medium", "large"][self.build as usize]
                    ));
                    if busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Asking the server...");
                        });
                    }
                }
            }
            if !self.message.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(240, 120, 110), &self.message);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if crate::chrome::button(ui, palette, "Back", !busy).clicked() {
                    if self.step == 0 {
                        outcome = Outcome::Back;
                    } else {
                        self.step -= 1;
                        self.message.clear();
                    }
                }
                let ready = match self.step {
                    0 => self.breed > 0,
                    1 => self.profession > 0,
                    _ => true,
                };
                if self.step < 3 {
                    if crate::chrome::button(ui, palette, "Next", ready).clicked() {
                        self.step += 1;
                    }
                } else if crate::chrome::button(ui, palette, "Create", !busy).clicked() {
                    // (`StartServerCreation`: the name checked first, then sent)
                    self.name = proper(self.name.trim());
                    match name_problem(&self.name) {
                        Some(problem) => self.message = text(problem),
                        None => {
                            let (breed, sex) = self.breed_and_sex().unwrap_or((1, 2));
                            let heads = heads_of(game, breed, sex);
                            match heads.get(self.head) {
                                Some(&(mesh, _, _)) => {
                                    let c = mao_net::create::NewCharacter { name: self.name.clone(), breed, sex, profession: profession(self.profession), head: mesh as i32, height: height(self.height), width: self.build };
                                    self.ask(account, mao_net::create::create_character(&c), true);
                                }
                                None => self.message = "No heads were found for this breed in the game's files.".into(),
                            }
                        }
                    }
                }
            });
        });
        if !open {
            outcome = Outcome::Back;
        }
        outcome
    }
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
        assert_eq!(plain_text("An Agent&#8217;s life &amp; more"), "An Agent\u{2019}s life & more");
        assert_eq!(plain_text("Difficulty: <FONT color=\"#ffff00\"> High </FONT><br>The Bureaucrat"), "Difficulty:  High 
The Bureaucrat");
        let found = |n: &str| (n == "head_opifexfemale00.abiff" || n == "head_opifexfemale30.abiff" || n == "head_opifexfemale31.abiff").then_some(n.len() as u32);
        let got: Vec<String> = heads(2, 3, &found).into_iter().map(|h| h.2).collect();
        assert_eq!(got, vec!["head_opifexfemale00.abiff", "head_opifexfemale31.abiff"]);
    }
}


