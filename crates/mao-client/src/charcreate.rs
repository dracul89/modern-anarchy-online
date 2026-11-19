//! Character creation's rules, from GUI.dll's `CharCreateModule_t` and its scenes: the screens' breed choice and profession
//! choice turned into the game's numbers (`SceneBase_t::ConvertCCBreedToGCBreedAndSex` 0x10122aa3,
//! `ConvertCCProfToGCProf` 0x10122a1a), the heads each breed and sex may have (`HeadMeshData_t`'s table, FUN_1011d692),
//! the height and build values the creation sends (`NameScene_t::SetState` 0x1011fc0f, case 0x1006), the name's rules
//! (FUN_10123cdc, from `StartServerCreation` 0x1012015d) and the login server's refusals (`SetState`'s codes, texts of
//! category 600). The message itself is `mao_net::create`.
//!
//! And the four scenes' views (`BreedScene_t` 0x10113fab, `AppearanceScene_t` 0x10112c4d, `ProfessionScene_t`
//! 0x101216d1, `NameScene_t` 0x101203bb, on `SceneBase_t` 0x10123393): [`Creating::show`] draws the running scene's
//! buttons, labels and text area over the world `ccscene.rs` runs, at the places of [`place`], and turns the mouse
//! into choices and camera orders. `docs/CLIENT_UI.md`, "Character creation as the original's four scenes", says what
//! is the original's and what is this port's own.

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
/// numbers are tried (GUI.dll FUN_1011d692): `shadowlands` is the account's expansion flag `ExpansionFlags & 2`, which
/// raises eight of the counts.
pub fn head_sets(breed: i32, sex: i32, shadowlands: bool) -> Vec<(&'static str, u8, u32)> {
    let n = |plain: u32, more: u32| if shadowlands { more } else { plain };
    match (breed, sex) {
        (2, 3) => vec![("head_opifexfemale", 1, n(32, 43))],
        (2, _) => vec![("head_opifexmale", 1, n(30, 43))],
        (1, 3) => vec![("head_solitusfemale", 1, n(50, 75)), ("head_solitusfemale_african", 2, 8), ("head_solitusfemale_asian", 3, 7)],
        (1, _) => vec![("head_solitusmale", 1, n(51, 71)), ("head_solitusmale_african", 2, 9), ("head_solitusmale_asian", 3, n(6, 9))],
        (3, 2) => vec![("head_nanomale", 1, n(30, 41))],
        (3, _) => vec![("head_nanofemale", 1, n(30, 43))],
        (4, _) => vec![("head_athrox", 1, n(30, 41))],
        _ => Vec::new(),
    }
}

/// The numbers of a set that the same function never offers, whatever the flag: one of the Opifex women's, thirteen
/// of the Solitus women's first set and fourteen of the Solitus men's.
pub fn head_left_out(set: &str, n: u32) -> bool {
    match set {
        "head_opifexfemale" => n == 30,
        "head_solitusfemale" => [3, 4, 8, 9, 10, 13, 14, 22, 29, 30, 43, 46, 47].contains(&n),
        "head_solitusmale" => [3, 4, 10, 11, 12, 14, 28, 29, 31, 32, 34, 37, 41, 47].contains(&n),
        _ => false,
    }
}

/// The heads offered, in order: each set's numbered meshes (`<set><nn>.abiff`, type 0xf6951) that the game has (`find`
/// gives a name's instance), with their race.
pub fn heads(breed: i32, sex: i32, shadowlands: bool, find: &dyn Fn(&str) -> Option<u32>) -> Vec<(u32, u8, String)> {
    let mut out = Vec::new();
    for (set, race, count) in head_sets(breed, sex, shadowlands) {
        for n in 0..count {
            if head_left_out(set, n) {
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

/// The text (category 600) for a refusal code of the login server (`NameScene_t::SetState` 0x1011fc0f); none for a
/// code it has no case for (its default: no message, the buttons usable again).
pub fn refusal(code: i32) -> Option<&'static str> {
    Some(match code {
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
        _ => return None,
    })
}

/// A text of category 600 from text.mdb (its name when the file has none).
pub fn text(name: &str) -> String {
    crate::feedback::texts().and_then(|t| t.named(600, name)).map(str::to_string).unwrap_or_else(|| name.to_string())
}

// ---- The screens ----

/// The texts' name for a game breed (`<Breed>Description`).
fn breed_key(breed: i32) -> &'static str {
    match breed {
        1 => "Solitus",
        2 => "Opifex",
        3 => "Nanomage",
        _ => "Atrox",
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

/// The account's expansion flags as the login's character list gave them (-1: not known).
static EXPANSIONS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);

/// The login's expansion flags (GUI.dll's value `ExpansionFlags`, set by `CharSelectWindow_c::SlotCharListReceived`).
pub fn set_expansions(flags: Option<u32>) {
    EXPANSIONS.store(flags.map_or(-1, i64::from), std::sync::atomic::Ordering::Relaxed);
}

/// Whether the account has the Shadowlands: the bit 2 of its expansion flags (the head counts, FUN_1011d692, go by it
/// alone). An account whose flags are not known is let through: the server decides then.
pub fn shadowlands_professions(flags: i64) -> bool {
    flags < 0 || flags & 2 != 0
}

/// The list's `s_nSLProfsEnabled` (Interfaces.dll `Client_t::ProcessMessage` 0x10002a9e, message 0xe:
/// `mao_net::session::sl_professions_enabled`).
static SL_PROFESSIONS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The login's `s_nSLProfsEnabled`.
pub fn set_sl_professions(enabled: bool) {
    SL_PROFESSIONS.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the Keeper and the Shade may be chosen: `ProfessionScene_t` 0x101216d1 switches their buttons on when
/// `ExpansionFlags & 2` or `Client_t::s_nSLProfsEnabled`, and its `SlotNextPressed` 0x10121035 refuses them only when
/// neither is set.
pub fn keeper_and_shade(flags: i64, sl_professions_enabled: bool) -> bool {
    sl_professions_enabled || shadowlands_professions(flags)
}

/// What the launcher tells the views of the world it draws: the box each breed figure fills (the renderer's frame),
/// and where the chosen character's head is (the game's frame).
#[derive(Clone, Default)]
pub struct World {
    pub figures: [Option<([f32; 3], [f32; 3])>; 7],
    pub head: Option<[f32; 3]>,
}

/// The creation under way: the choices of the four scenes and the scenes' state.
pub struct Creating {
    /// The scene to start in (0 breed, 1 appearance, 2 profession, 3 name) when the intro is left out: the pictures
    /// the tests take. None: the creation as it runs, from the intro.
    pub start_scene: Option<usize>,
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
    /// The world's camera and states (`ccscene.rs`), made when the screens first show.
    pub director: Option<crate::ccscene::Director>,
    pub world: World,
    /// What the mouse is over (its sound plays when this changes) and the text that goes with it.
    over: String,
    /// The appearance scene's camera is at the head.
    at_head: bool,
    /// The exit question is up.
    asking: bool,
    /// The ambience was started.
    ambience: bool,
    last_frame: Option<std::time::Instant>,
    /// `MAO_CC_SCRIPT=<seconds>:<step>,...` (a test's hands): the steps still to come and the seconds gone. Steps:
    /// `esc`, `breed=<1-7>`, `profession=<1-14>`, `height=<0-2>`, `build=<0-2>`, `head=<n>`, `next`, `back`, `face`,
    /// `full`, and `made`, which walks to the shuttle as after a creation and then goes back to the list (nothing is
    /// sent to a server).
    script: Option<Vec<(f32, String)>>,
    script_clock: f32,
    rehearsal: bool,
}

impl Default for Creating {
    fn default() -> Self {
        Creating {
            start_scene: None,
            breed: 0,
            profession: 0,
            head: 0,
            height: 1,
            build: 1,
            name: String::new(),
            message: String::new(),
            pending: None,
            director: None,
            world: World::default(),
            over: String::new(),
            at_head: false,
            asking: false,
            ambience: false,
            last_frame: None,
            script: None,
            script_clock: 0.0,
            rehearsal: false,
        }
    }
}

/// The game's instance index (names to instances), read once.
fn heads_of(game: &str, breed: i32, sex: i32) -> Vec<(u32, u8, String)> {
    static INDEX: std::sync::OnceLock<Option<mao_formats::instances::Instances>> = std::sync::OnceLock::new();
    let index = INDEX.get_or_init(|| {
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(game)).ok()?;
        mao_formats::instances::Instances::parse(&db.get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 }).ok()??).ok()
    });
    // (an account whose flags are not known is offered the larger sets, as it is let to the Shadowlands' professions)
    let shadowlands = shadowlands_professions(EXPANSIONS.load(std::sync::atomic::Ordering::Relaxed));
    match index {
        Some(index) => heads(breed, sex, shadowlands, &|n| index.find(0xf6951, n)),
        None => Vec::new(),
    }
}

/// The scenes' places, as shares of the window (each scene's constructor: `MakeButton` and `MakeLabel` take the middle
/// of the thing, `SetTextAreaFrame` a box given by its size and its middle).
pub mod place {
    /// `SceneBase_t` (0x10123393): the banner, the exit, Next and Back.
    pub const BANNER: [f32; 2] = [0.5, 0.07];
    pub const EXIT: [f32; 2] = [0.975, 0.05];
    pub const NEXT: [f32; 2] = [0.925, 0.95];
    pub const BACK: [f32; 2] = [0.8, 0.95];
    /// The text areas: middle x, middle y, width, height. The profession scene's height is not a share in the
    /// original (156 units of its lowest resolution, scaled): 0.2 of the window here, by the original's pictures.
    pub const BREED_TEXT: [f32; 4] = [0.575, 0.85, 0.55, 0.25];
    pub const APPEARANCE_TEXT: [f32; 4] = [0.68, 0.77, 0.5, 0.2];
    pub const PROFESSION_TEXT: [f32; 4] = [0.325, 0.8125, 0.6, 0.2];
    pub const NAME_TEXT: [f32; 4] = [0.68, 0.77, 0.6, 0.25];
    /// The breed scene's label of the chosen breed.
    pub const BREED_LABEL: [f32; 2] = [0.17, 0.82];
    /// The appearance scene (0x10112c4d).
    pub const HEIGHT_LABEL: [f32; 2] = [0.79, 0.2];
    pub const BUILD_LABEL: [f32; 2] = [0.79, 0.5];
    pub const HEAD_LABEL: [f32; 2] = [0.25, 0.17];
    pub const HEIGHTS: [[f32; 2]; 3] = [[0.68, 0.28], [0.79, 0.28], [0.9, 0.28]];
    pub const BUILDS: [[f32; 2]; 3] = [[0.68, 0.58], [0.79, 0.58], [0.9, 0.58]];
    pub const NEXT_HEAD: [f32; 2] = [0.4, 0.17];
    pub const PREVIOUS_HEAD: [f32; 2] = [0.1, 0.17];
    /// The head's area, over which the camera goes to the face: middle and size.
    pub const HEAD_AREA: [f32; 4] = [0.23, 0.3, 0.48, 0.3];
    /// The profession scene (0x101216d1): the first button's middle; each next one is the button's height and 4 units
    /// (of 768 to the window's height) lower.
    pub const PROFESSIONS: [f32; 2] = [0.82, 0.2];
    /// The name scene (0x101203bb).
    pub const SUGGEST: [f32; 2] = [0.5, 0.23];
    pub const NICKNAME: [f32; 2] = [0.5, 0.17];
    pub const NAME_BACK: [f32; 2] = [0.774_609, 0.95];
    pub const FINISH: [f32; 2] = [0.912_305, 0.95];
    pub const SUMMARY: [f32; 2] = [0.24, 0.95];
}

/// A button's three pictures (`Button_c::SetGfx` 0, 1, 2: at rest, pressed, the mouse over it) by its sprites' stem.
fn pictures(stem: &str) -> [String; 3] {
    [format!("GFX_GUI_CC_{stem}_NORMAL"), format!("GFX_GUI_CC_{stem}_PRESSED"), format!("GFX_GUI_CC_{stem}_OVER")]
}

/// The appearance scene's height and build buttons: sprite stems and the texts' names of the mouse over each.
const HEIGHT_BUTTONS: [(&str, &str, &str); 3] = [("SHORT", "ButtonShort", "HeightToShort"), ("MEDIUM", "ButtonMedium", "HeightToMedium"), ("TALL", "ButtonTall", "HeightToTall")];
const BUILD_BUTTONS: [(&str, &str, &str); 3] = [("SMALL", "ButtonSlender", "BodytypeSlender"), ("MEDIUM", "ButtonBodyMedium", "BodytypeMedium"), ("LARGE", "ButtonHeavy", "BodytypeHeavy")];

/// The breed choices' text names (`Button<name>`, `<name>Selected`) and voices, in the choices' order.
const BREED_NAMES: [(&str, &str); 7] = [
    ("SolitusFemale", "SM_Sandy_CC_Solitus_Female"),
    ("SolitusMale", "SM_Sandy_CC_Solitus_Male"),
    ("OpifexFemale", "SM_Sandy_CC_Opifex_Female"),
    ("OpifexMale", "SM_Sandy_CC_Opifex_Male"),
    ("NanomageFemale", "SM_Sandy_CC_Nano_Female"),
    ("NanomageMale", "SM_Sandy_CC_Nano_Male"),
    ("Atrox", "SM_Sandy_CC_Atrox"),
];

/// The profession choices' text names (`Inspect<name>`, `<name>Selected`; the description is `Description<name>`,
/// the Meta-Physicist's `DescriptionMetaPhysicists`) and voices, in the choices' order.
pub const PROFESSION_NAMES: [(&str, &str); 14] = [
    ("Metaphysicist", "SM_Sandy_CC_Meta_Selected"),
    ("Adventurer", "SM_Sandy_CC_Adventurer"),
    ("Engineer", "SM_Sandy_CC_Engineer"),
    ("Soldier", "SM_Sandy_CC_Soldier"),
    ("Keeper", "SM_Sandy_CC_Keeper"),
    ("Shade", "SM_Sandy_CC_Shade"),
    ("Fixer", "SM_Sandy_CC_Fixer"),
    ("Agent", "SM_Sandy_CC_Agent"),
    ("Trader", "SM_Sandy_CC_Trader"),
    ("Doctor", "SM_Sandy_CC_Doctor"),
    ("Enforcer", "SM_Sandy_CC_Enforcer"),
    ("Bureaucrat", "SM_Sandy_CC_Bureaucrat"),
    ("MartialArtist", "SM_Sandy_CC_MA"),
    ("NanoTechnician", "SM_Sandy_CC_Nanotech_Selected"),
];

fn profession_description(name: &str) -> String {
    if name == "Metaphysicist" {
        "DescriptionMetaPhysicists".to_string()
    } else {
        format!("Description{name}")
    }
}

/// The views' purple (`ViewSurface_c::SetColor(0x6666aa)`), at the banner's and labels' strength (0.703) or the text
/// area's (0.7).
fn purple(alpha: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(0x66, 0x66, 0xaa, (alpha * 255.0) as u8)
}

/// One frame's drawing of a scene: the window's size, the scale of the original's 768-high layout, and whether the
/// scene takes the mouse.
struct Canvas<'a> {
    ui: &'a mut egui::Ui,
    size: egui::Vec2,
    scale: f32,
    live: bool,
    skin: Option<std::sync::Arc<crate::skin::Skin>>,
    /// What the mouse is over this frame, and the text that goes with it.
    over: Option<(String, String, String)>,
}

impl Canvas<'_> {
    fn at(&self, p: [f32; 2]) -> egui::Pos2 {
        egui::pos2(self.size.x * p[0], self.size.y * p[1])
    }

    fn font(&self, font: u8) -> egui::FontId {
        let measure = crate::gui::font::Measure { ctx: Some(self.ui.ctx().clone()) };
        let mut id = measure.egui_font(crate::gui::font::FontId(font));
        id.size *= self.scale.max(0.75);
        id
    }

    /// `SceneBase_t::MakeButton` (0x101226f3): the three pictures, the first one's size round the place, its width
    /// scaled by the window's width over 1024 and its height by the window's height over 768 (so a wide window pulls
    /// the buttons wide, as in the original: the user's choice of 2026-11-16). Gives whether it was clicked.
    fn button(&mut self, key: &str, stem: &[String; 3], at: [f32; 2], enabled: bool, info: (&str, &str)) -> bool {
        let Some(skin) = self.skin.clone() else { return false };
        let Some(normal) = skin.get(&stem[0]) else { return false };
        let size = egui::vec2(normal.size.x * self.size.x / 1024.0, normal.size.y * self.scale);
        let middle = self.at(at);
        let rect = egui::Rect::from_min_size(egui::pos2((middle.x - size.x * 0.5).floor(), (middle.y - size.y * 0.5).floor()), size);
        let sense = if self.live && enabled { egui::Sense::click() } else { egui::Sense::hover() };
        let response = self.ui.interact(rect, self.ui.id().with(key), sense);
        let picture = if !enabled || !self.live {
            &stem[0]
        } else if response.is_pointer_button_down_on() {
            &stem[1]
        } else if response.hovered() {
            &stem[2]
        } else {
            &stem[0]
        };
        let tint = if enabled { egui::Color32::WHITE } else { egui::Color32::from_gray(110) };
        skin.draw_in(self.ui.painter(), picture, rect, tint);
        if self.live && response.hovered() && !info.0.is_empty() {
            self.over = Some((key.to_string(), info.0.to_string(), info.1.to_string()));
        }
        self.live && enabled && response.clicked()
    }

    /// `SceneBase_t::MakeLabel` (0x10122bed): white text with a shadow on the purple, its middle at the place.
    fn label(&mut self, words: &str, at: [f32; 2], font: u8) -> egui::Rect {
        let font = self.font(font);
        let galley = self.ui.painter().layout_no_wrap(words.to_string(), font.clone(), egui::Color32::WHITE);
        let shadow = self.ui.painter().layout_no_wrap(words.to_string(), font, egui::Color32::BLACK);
        // (`SetBorders(5, 1, 5, 1)`)
        let size = galley.size() + egui::vec2(10.0, 2.0);
        let middle = self.at(at);
        let rect = egui::Rect::from_min_size(egui::pos2((middle.x - size.x * 0.5).floor(), (middle.y - size.y * 0.5).floor()), size);
        self.ui.painter().rect_filled(rect, 0.0, purple(0.703));
        self.ui.painter().galley(rect.min + egui::vec2(6.0, 2.0), shadow, egui::Color32::BLACK);
        self.ui.painter().galley(rect.min + egui::vec2(5.0, 1.0), galley, egui::Color32::WHITE);
        rect
    }

    /// The text area (`CreateTextArea` 0x10122f56, `SetNewInfoText` 0x1012367c): a head line in 0xeeeeee and the text
    /// in 0xcccccc on the purple.
    fn text_area(&mut self, frame: [f32; 4], head: &str, body: &str) {
        let size = egui::vec2((self.size.x * frame[2]).floor(), (self.size.y * frame[3]).floor());
        let middle = self.at([frame[0], frame[1]]);
        let rect = egui::Rect::from_min_size(egui::pos2((middle.x - size.x * 0.5).floor(), (middle.y - size.y * 0.5).floor()), size);
        let painter = self.ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, purple(0.7));
        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0x30, 0x30, 0x60, 180)), egui::StrokeKind::Inside);
        let font = self.font(10);
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = rect.width() - 10.0;
        job.append(&format!("{head}\n"), 0.0, egui::TextFormat { font_id: font.clone(), color: egui::Color32::from_gray(0xee), ..Default::default() });
        job.append(body, 0.0, egui::TextFormat { font_id: font, color: egui::Color32::from_gray(0xcc), ..Default::default() });
        let galley = painter.layout_job(job);
        // (the text view's borders: 5, 2, 5, 5)
        painter.galley(rect.min + egui::vec2(5.0, 2.0), galley, egui::Color32::WHITE);
    }
}

impl Creating {
    /// The game's breed and sex chosen (none before a choice).
    pub fn breed_and_sex(&self) -> Option<(i32, i32)> {
        breed_and_sex(self.breed)
    }

    /// The chosen head's mesh (the breed's `head`-th), for the figure.
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

    /// The sounds stop (the creation is left).
    fn hush(&mut self) {
        crate::audio::with(|m| m.stop_all_sound_effects());
        self.ambience = false;
    }

    /// The line from the eye through a point of the window (shares of it, from the top left): where it starts and its
    /// direction, in the renderer's frame, with the view's shape as the launcher draws it.
    fn line_through(&self, x: f32, y: f32, aspect: f32) -> Option<([f32; 3], [f32; 3])> {
        let mover = &self.director.as_ref()?.mover;
        let (eye, forward, up) = mover.view();
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let unit = |v: [f32; 3]| {
            let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
            [v[0] / n, v[1] / n, v[2] / n]
        };
        let right = unit(cross(forward, up));
        let up = cross(right, forward);
        let half = crate::hall::half_height(mover.fov, aspect);
        let (nx, ny) = ((x * 2.0 - 1.0) * half * aspect, (1.0 - y * 2.0) * half);
        Some((eye, unit([forward[0] + right[0] * nx + up[0] * ny, forward[1] + right[1] * nx + up[1] * ny, forward[2] + right[2] * nx + up[2] * ny])))
    }

    /// The scenes: the world's states run, the scene's views drawn over the world, the mouse and the keys taken.
    pub fn show(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette, account: &Account, game: &str) -> Outcome {
        use crate::ccscene::{Director, Order, Scene};
        let mut outcome = Outcome::Stay;
        // an answer from the login server
        if let Some((is_create, rx)) = &self.pending {
            let is_create = *is_create;
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(mao_net::create::Reply::Name(name)) => self.name = name,
                        // (made: the walk to the shuttle, then the game)
                        Ok(mao_net::create::Reply::Created(_)) if is_create => {
                            if let Some(d) = self.director.as_mut() {
                                d.go(Order::Made);
                            }
                        }
                        Ok(mao_net::create::Reply::Refused(code)) => {
                            if let Some(name) = refusal(code) {
                                self.message = plain_text(&text(name));
                                crate::audio::play_named(if matches!(code, 30 | 31) { "SM_Sandy_CC_Nick" } else { "SM_Sandy_CC_GUI_Error" });
                            }
                        }
                        Ok(_) => {}
                        Err(e) => self.message = e,
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(_) => self.pending = None,
            }
        }
        // the world: made the first time, then a frame of it
        if self.director.is_none() {
            crate::audio::start(std::path::Path::new(game));
            let file = crate::ccscene::CameraFile::parse(&mao_formats::files::read_to_string(std::path::Path::new(game).join("CharCreateCamera.dat")).unwrap_or_default());
            let mut director = match self.start_scene {
                Some(n) => Director::at_scene(file, [Scene::Breed, Scene::Appearance, Scene::Profession, Scene::Name][n.min(3)], self.profession),
                None => Director::new(file),
            };
            director.profession = self.profession;
            self.director = Some(director);
            self.over.clear();
        }
        let now = std::time::Instant::now();
        let dt = self.last_frame.replace(now).map_or(0.0, |t| now.duration_since(t).as_secs_f32());
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let Some(director) = self.director.as_mut() else { return outcome };
        if escape {
            director.skip();
        }
        if director.frame(dt) {
            self.hush();
            if std::mem::take(&mut self.rehearsal) {
                self.director = None;
                return Outcome::Back;
            }
            return Outcome::Created(self.name.clone());
        }
        // (the test's hands)
        let script = self.script.get_or_insert_with(|| {
            let text = std::env::var("MAO_CC_SCRIPT").unwrap_or_default();
            text.split(',').filter_map(|s| s.split_once(':')).filter_map(|(t, step)| Some((t.trim().parse().ok()?, step.trim().to_string()))).collect()
        });
        self.script_clock += dt;
        let mut scripted: Vec<Order> = Vec::new();
        while script.first().is_some_and(|s| s.0 <= self.script_clock) {
            let (_, step) = script.remove(0);
            let director = self.director.as_mut().unwrap();
            let number = |v: &str| v.parse::<i32>().unwrap_or(0);
            match (step.split_once('=').unwrap_or((&step, "")), director.scene()) {
                (("esc", _), _) => director.skip(),
                (("breed", v), _) => (self.breed, self.head) = (number(v), 0),
                (("height", v), _) => self.height = number(v),
                (("build", v), _) => self.build = number(v),
                (("head", v), _) => self.head = number(v) as usize,
                (("profession", v), _) => {
                    scripted.push(Order::Profession { from: self.profession });
                    self.profession = number(v);
                }
                (("face", _), _) => scripted.extend(self.world.head.map(|h| Order::Head([h[0] - 0.081, h[1] - 0.009, h[2] + 1.1112]))),
                (("full", _), _) => scripted.push(Order::FullView),
                (("next", _), Some(Scene::Breed)) => scripted.push(Order::BreedNext),
                (("next", _), Some(Scene::Appearance)) => scripted.push(Order::AppearanceNext),
                (("next", _), Some(Scene::Profession)) => scripted.push(Order::ProfessionNext),
                (("back", _), Some(Scene::Appearance)) => scripted.push(Order::AppearanceBack),
                (("back", _), Some(Scene::Profession)) => scripted.push(Order::ProfessionBack),
                (("back", _), Some(Scene::Name)) => scripted.push(Order::NameBack),
                (("made", _), _) => {
                    self.rehearsal = true;
                    scripted.push(Order::Made);
                }
                _ => eprintln!("creation script: {step:?} does nothing here"),
            }
        }
        let director = self.director.as_mut().unwrap();
        for sound in std::mem::take(&mut director.sounds) {
            // (`BreedScene_t::StartScene` stops what plays before its own line)
            if sound == "SM_Sandy_CC_Select_Breed" {
                crate::audio::with(|m| m.stop_all_sound_effects());
                self.ambience = false;
            }
            crate::audio::play_named(sound);
        }
        if director.ambience && !self.ambience {
            self.ambience = true;
            crate::audio::play_named("SM_Sandy_CC_Ambience");
        }
        crate::audio::with(|m| m.frame_process(dt));
        ctx.request_repaint();

        let (live_scene, black, captions) = (director.scene(), director.black, director.captions());
        let screen = ctx.content_rect();
        let aspect = screen.width() / screen.height().max(1.0);
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        // (the breed figure under the mouse: the line from the eye through it against each figure's box)
        let figure_under = match (live_scene, pointer) {
            (Some(Scene::Breed), Some(p)) if !self.asking => self.line_through(p.x / screen.width(), p.y / screen.height(), aspect).and_then(|(from, along)| {
                self.world.figures.iter().enumerate().filter_map(|(i, b)| b.and_then(|(lo, hi)| crate::hall::line_meets_box(from, along, lo, hi)).map(|t| (i, t))).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
            }),
            _ => None,
        };
        let mut orders: Vec<Order> = scripted;
        let mut leave = false;
        let mut sounds: Vec<&'static str> = Vec::new();
        let heads = self.breed_and_sex().map(|(breed, sex)| heads_of(game, breed, sex)).unwrap_or_default();
        let shadowlands = keeper_and_shade(EXPANSIONS.load(std::sync::atomic::Ordering::Relaxed), SL_PROFESSIONS.load(std::sync::atomic::Ordering::Relaxed));
        let busy = self.pending.is_some();
        egui::Area::new(egui::Id::new("character-creation")).fixed_pos(screen.min).order(egui::Order::Background).show(ctx, |ui| {
            ui.set_clip_rect(screen);
            ui.allocate_rect(screen, egui::Sense::hover());
            // the black over the picture, and the intro's lines
            if black > 0.0 {
                ui.painter().rect_filled(screen, 0.0, egui::Color32::from_black_alpha((black.clamp(0.0, 1.0) * 255.0) as u8));
            }
            let scale = screen.height() / 768.0;
            let mut canvas = Canvas { ui, size: screen.size(), scale, live: live_scene.is_some() && !self.asking, skin: crate::skin::skin(ctx), over: None };
            for caption in &captions {
                let font = canvas.font(4);
                let alpha = (caption.alpha * 255.0) as u8;
                let words = plain_text(&text(caption.text));
                let at = canvas.at(caption.at);
                canvas.ui.painter().text(at + egui::vec2(1.0, 1.0), egui::Align2::CENTER_CENTER, &words, font.clone(), egui::Color32::from_black_alpha(alpha));
                canvas.ui.painter().text(at, egui::Align2::CENTER_CENTER, &words, font, egui::Color32::from_white_alpha(alpha));
            }
            // (a scene's views show while it runs: `StopScene` hides them as the camera leaves, `StartScene` shows the
            // next scene's when it has come to rest)
            let Some(scene) = live_scene else { return };
            // (the mouse over the three buttons every scene has: `SceneBase_t::SlotMouseOverClose`, `SlotMouseOverNext`
            // 0x101237a2 and `SlotMouseOverPrev` 0x1012391f, which pick the text by the scene)
            let exit_info = ("ButtonExit", "SelectionsWillBeSaved");
            if canvas.button("exit", &pictures("EXIT"), place::EXIT, true, exit_info) {
                sounds.push("SM_Sandy_CC_GUI_Select");
                self.asking = true;
            }
            match scene {
                Scene::Breed => {
                    canvas.label(&plain_text(&text("BannerSelectBreed")), place::BANNER, 4);
                    if let Some((breed, sex)) = self.breed_and_sex() {
                        canvas.label(&format!("{} {}", crate::launcher::breed(breed), crate::launcher::sex(sex).to_lowercase()), place::BREED_LABEL, 4);
                    }
                    if canvas.button("next", &pictures("NEXT"), place::NEXT, self.breed > 0, ("ButtonNext", "BreedNext")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        orders.push(Order::BreedNext);
                    }
                    // (the figures: the mouse over one names it, a click chooses it)
                    if canvas.over.is_none() {
                        if let Some(i) = figure_under {
                            let (name, voice) = BREED_NAMES[i];
                            let key = breed_key(breed_and_sex(i as i32 + 1).map_or(1, |b| b.0));
                            canvas.over = Some((format!("figure{i}"), format!("Button{name}"), format!("{key}Description")));
                            if canvas.live && canvas.ui.input(|i| i.pointer.primary_clicked()) {
                                self.breed = i as i32 + 1;
                                self.head = 0;
                                sounds.push(voice);
                            }
                        }
                    }
                    let (head, body) = match (&canvas.over, self.breed) {
                        (Some((_, head, body)), _) => (head.clone(), body.clone()),
                        (None, 0) => ("ChooseBreed".to_string(), "BreedHelp".to_string()),
                        (None, choice) => (format!("{}Selected", BREED_NAMES[choice as usize - 1].0), format!("{}Description", breed_key(breed_and_sex(choice).map_or(1, |b| b.0)))),
                    };
                    canvas.text_area(place::BREED_TEXT, &plain_text(&text(&head)), &plain_text(&text(&body)));
                }
                Scene::Appearance => {
                    canvas.label(&plain_text(&text("BannerSelectAppearance")), place::BANNER, 4);
                    canvas.label(&plain_text(&text("SelectHeight")), place::HEIGHT_LABEL, 4);
                    canvas.label(&plain_text(&text("SelectBuild")), place::BUILD_LABEL, 4);
                    canvas.label(&plain_text(&text("SelectHead")), place::HEAD_LABEL, 4);
                    for (i, (stem, head, body)) in HEIGHT_BUTTONS.iter().enumerate() {
                        if canvas.button(&format!("height{i}"), &pictures(stem), place::HEIGHTS[i], true, (head, body)) {
                            sounds.push("SM_Sandy_CC_GUI_Select");
                            self.height = i as i32;
                        }
                    }
                    for (i, (stem, head, body)) in BUILD_BUTTONS.iter().enumerate() {
                        if canvas.button(&format!("build{i}"), &pictures(stem), place::BUILDS[i], true, (head, body)) {
                            sounds.push("SM_Sandy_CC_GUI_Select");
                            self.build = i as i32;
                        }
                    }
                    // (the head's area: the camera goes to the face while the mouse is in it)
                    let area = place::HEAD_AREA;
                    let middle = canvas.at([area[0], area[1]]);
                    let size = egui::vec2(canvas.size.x * area[2], canvas.size.y * area[3]);
                    let in_area = canvas.live && pointer.is_some_and(|p| egui::Rect::from_center_size(middle, size).contains(p));
                    if in_area {
                        canvas.over = Some(("head".to_string(), "ButtonSelectFace".to_string(), "Select Face".to_string()));
                    }
                    // (`SelectNextHead` / `SelectPrevHead`: round the breed's heads)
                    if canvas.button("nexthead", &pictures("NEXTHEAD"), place::NEXT_HEAD, heads.len() > 1, ("", "")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        self.head = (self.head + 1) % heads.len();
                    }
                    if canvas.button("prevhead", &pictures("PREVHEAD"), place::PREVIOUS_HEAD, heads.len() > 1, ("", "")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        self.head = (self.head + heads.len() - 1) % heads.len();
                    }
                    if canvas.live && in_area != self.at_head {
                        self.at_head = in_area;
                        match (in_area, self.world.head) {
                            // (`MoveCamereToHead`: before the head's attractor, by the function's own numbers)
                            (true, Some(head)) => orders.push(Order::Head([head[0] - 0.081, head[1] - 0.009, head[2] + 1.1112])),
                            (true, None) => {}
                            (false, _) => orders.push(Order::FullView),
                        }
                    }
                    if canvas.button("back", &pictures("BACK"), place::BACK, true, ("ButtonGoBack", "BackToBreed")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        orders.push(Order::AppearanceBack);
                        self.at_head = false;
                    }
                    if canvas.button("next", &pictures("NEXT"), place::NEXT, true, ("ButtonNext", "ProceedToProfession")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        orders.push(Order::AppearanceNext);
                        self.at_head = false;
                    }
                    let (head, body) = canvas.over.as_ref().map_or(("SelectAppearance".to_string(), "SelectionHelpText".to_string()), |o| (o.1.clone(), o.2.clone()));
                    canvas.text_area(place::APPEARANCE_TEXT, &plain_text(&text(&head)), &plain_text(&text(&body)));
                }
                Scene::Profession => {
                    canvas.label(&plain_text(&text("BannerSelectProfession")), place::BANNER, 4);
                    let mut y = place::PROFESSIONS[1];
                    for (i, (name, voice)) in PROFESSION_NAMES.iter().enumerate() {
                        let sprite = format!("GFX_GUI_CC_NEW_PROF{}", i + 1);
                        let stem = [sprite.clone(), sprite.clone(), sprite.clone()];
                        let height = canvas.skin.as_ref().and_then(|s| s.get(&sprite)).map_or(32.0, |s| s.size.y);
                        // (the Keeper and the Shade, the fifth and sixth, are switched off without the Shadowlands)
                        let allowed = !matches!(i, 4 | 5) || shadowlands;
                        if canvas.button(&format!("profession{i}"), &stem, [place::PROFESSIONS[0], y], allowed, (&format!("Inspect{name}"), &profession_description(name))) && self.profession != i as i32 + 1 {
                            orders.push(Order::Profession { from: self.profession });
                            self.profession = i as i32 + 1;
                            sounds.push(voice);
                        }
                        y += (height + 4.0) / 768.0;
                    }
                    if canvas.button("back", &pictures("BACK"), place::BACK, true, ("ButtonGoBack", "ProfBack")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        orders.push(Order::ProfessionBack);
                    }
                    if canvas.button("next", &pictures("NEXT"), place::NEXT, self.profession > 0, ("ButtonNext", "ProfNext")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        // (`SlotNextPressed`: a Keeper or a Shade chosen without the Shadowlands goes no further: the
                        // error's sound and the text `CharCreation_SLOnlyProfession`)
                        if matches!(self.profession, 5 | 6) && !shadowlands {
                            sounds.push("SM_Sandy_CC_GUI_Error");
                            self.message = crate::feedback::texts().and_then(|t| t.named(1003, "CharCreation_SLOnlyProfession")).map(plain_text).unwrap_or_else(|| "CharCreation_SLOnlyProfession".to_string());
                        } else {
                            self.message.clear();
                            orders.push(Order::ProfessionNext);
                        }
                    }
                    let (head, body) = match (&canvas.over, self.profession) {
                        (Some((_, head, body)), _) => (head.clone(), body.clone()),
                        (None, 0) => ("ProfButtonDefault".to_string(), "ProfessionDefault".to_string()),
                        (None, choice) => {
                            let name = PROFESSION_NAMES[choice as usize - 1].0;
                            (format!("{name}Selected"), profession_description(name))
                        }
                    };
                    if self.message.is_empty() || canvas.over.is_some() {
                        canvas.text_area(place::PROFESSION_TEXT, &plain_text(&text(&head)), &plain_text(&text(&body)));
                    } else {
                        canvas.text_area(place::PROFESSION_TEXT, &self.message, "");
                    }
                }
                Scene::Name => {
                    canvas.label(&plain_text(&text("BannerEnterName")), place::BANNER, 4);
                    // (the name's line: "Nickname:" and the input, 200 wide)
                    let font = canvas.font(4);
                    let middle = canvas.at(place::NICKNAME);
                    let label = plain_text(&text("Nickname:"));
                    let galley = canvas.ui.painter().layout_no_wrap(label.clone(), font.clone(), egui::Color32::WHITE);
                    let width = galley.size().x + 6.0 + 200.0 * scale.max(0.75);
                    let left = (middle.x - width * 0.5).floor();
                    let top = (middle.y - galley.size().y * 0.5).floor();
                    canvas.ui.painter().text(egui::pos2(left + 1.0, top + 1.0), egui::Align2::LEFT_TOP, &label, font.clone(), egui::Color32::BLACK);
                    canvas.ui.painter().text(egui::pos2(left, top), egui::Align2::LEFT_TOP, &label, font.clone(), egui::Color32::from_rgb(0x99, 0x99, 0xff));
                    let input = egui::Rect::from_min_size(egui::pos2(left + galley.size().x + 6.0, top - 2.0), egui::vec2(200.0 * scale.max(0.75), galley.size().y + 4.0));
                    canvas.ui.painter().rect_filled(input, 0.0, purple(0.5));
                    let edit = egui::TextEdit::singleline(&mut self.name).font(font).char_limit(12).frame(egui::Frame::NONE).text_color(egui::Color32::WHITE).interactive(canvas.live && !busy);
                    let edited = canvas.ui.put(input, edit);
                    if canvas.live && !edited.has_focus() && self.name.is_empty() {
                        edited.request_focus();
                    }
                    if canvas.button("suggest", &pictures("NAME"), place::SUGGEST, !busy, ("NameButtonSuggest", "NameSuggest")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        if let Some((breed, sex)) = self.breed_and_sex() {
                            self.ask(account, mao_net::create::suggest_name(breed, sex, profession(self.profession)), false);
                        }
                    }
                    // (`CreateCharInfo`: the text `BreedProfession` with the breed's and the profession's names)
                    if let Some((breed, _)) = self.breed_and_sex() {
                        let summary = mao_formats::ldb::format(&text("BreedProfession"), &[mao_formats::ldb::Arg::Text(crate::launcher::breed(breed)), mao_formats::ldb::Arg::Text(crate::launcher::profession(profession(self.profession)))]);
                        canvas.label(&plain_text(&summary), place::SUMMARY, 4);
                    }
                    if canvas.button("back", &pictures("BACK"), place::NAME_BACK, !busy, ("ButtonGoBack", "NameBack")) {
                        sounds.push("SM_Sandy_CC_GUI_Select");
                        orders.push(Order::NameBack);
                        self.message.clear();
                    }
                    let finish = canvas.button("finish", &pictures("RK"), place::FINISH, !busy, ("NameButtonFinish", "Finish_AreteLanding")) || (canvas.live && !busy && edited.lost_focus() && canvas.ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    if finish {
                        // (`StartServerCreation`: the name checked first, then sent)
                        self.name = proper(self.name.trim());
                        match name_problem(&self.name) {
                            Some(problem) => {
                                self.message = plain_text(&text(problem));
                                sounds.push(if problem == "MustEnterName" { "SM_Sandy_CC_Name" } else { "SM_Sandy_CC_Nick" });
                            }
                            None => {
                                let (breed, sex) = self.breed_and_sex().unwrap_or((1, 2));
                                match heads.get(self.head) {
                                    Some(&(mesh, _, _)) => {
                                        sounds.push("SM_Sandy_CC_GUI_Select");
                                        let c = mao_net::create::NewCharacter { name: self.name.clone(), breed, sex, profession: profession(self.profession), head: mesh as i32, height: height(self.height), width: self.build };
                                        self.ask(account, mao_net::create::create_character(&c), true);
                                    }
                                    None => self.message = "No heads were found for this breed in the game's files.".into(),
                                }
                            }
                        }
                    }
                    let (head, body) = canvas.over.as_ref().map_or(("ChooseNick".to_string(), "NickHelp".to_string()), |o| (o.1.clone(), o.2.clone()));
                    // (a refusal or a fault of the name shows where the help was)
                    if self.message.is_empty() {
                        canvas.text_area(place::NAME_TEXT, &plain_text(&text(&head)), &plain_text(&text(&body)));
                    } else {
                        canvas.text_area(place::NAME_TEXT, &self.message, "");
                    }
                }
            }
            // (the mouse comes over something: its sound)
            let over = canvas.over.as_ref().map(|o| o.0.clone()).unwrap_or_default();
            if over != self.over {
                if !over.is_empty() {
                    sounds.push("SM_Sandy_CC_GUI_Mouseover");
                }
                self.over = over;
            }
        });
        // the exit's question (`AskExitMessage`: the box "ExitCC", Yes and No)
        if self.asking {
            let words = |category: i32, name: &str| crate::feedback::texts().and_then(|t| t.named(category, name)).map(plain_text).unwrap_or_else(|| name.to_string());
            let mut open = true;
            crate::chrome::window(ctx, palette, "ExitCC", "", &mut open, |w| w.resizable(false).collapsible(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)), |ui| {
                ui.set_max_width(360.0);
                ui.label(words(1003, "CharCreation_ExitQuestion"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, &crate::chrome::yes_no()[0], true).clicked() {
                        leave = true;
                    }
                    if crate::chrome::button(ui, palette, &crate::chrome::yes_no()[1], true).clicked() {
                        self.asking = false;
                    }
                });
            });
            if !open {
                self.asking = false;
            }
        }
        for sound in sounds {
            crate::audio::play_named(sound);
        }
        if let Some(director) = self.director.as_mut() {
            director.profession = self.profession;
            for order in orders {
                director.go(order);
            }
        }
        if leave {
            // (`QuitCharacterCreation`: back to the character list; the choices stay for the next time)
            self.asking = false;
            self.director = None;
            self.last_frame = None;
            self.at_head = false;
            self.hush();
            outcome = Outcome::Back;
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Keeper and the Shade: the Shadowlands' flag or the list's `s_nSLProfsEnabled`.
    #[test]
    fn keeper_and_shade_by_the_flag_or_the_lists_switch() {
        assert!(!keeper_and_shade(0, false));
        assert!(keeper_and_shade(0, true));
        assert!(keeper_and_shade(2, false));
        assert!(keeper_and_shade(-1, false));
    }

    /// A refusal's text is `NameScene_t::SetState`'s for its code; a code it has no case for says nothing.
    #[test]
    fn refusals_have_the_originals_texts_or_none() {
        assert_eq!(refusal(8), Some("CantCreateMoreChars"));
        assert_eq!(refusal(28), Some("ServerWrongRDB"));
        assert_eq!(refusal(31), Some("NicknameInvalid"));
        assert_eq!((refusal(0), refusal(11), refusal(29), refusal(32)), (None, None, None, None));
    }

    /// The game's own files have every text and every picture the scenes ask for by name (`MAO_GAME`).
    #[test]
    fn the_scenes_texts_and_pictures_are_in_the_game() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let game = std::path::Path::new(&game);
        let texts = mao_formats::ldb::Texts::parse(&std::fs::read(game.join("cd_image/text/text.mdb")).unwrap()).unwrap();
        let mut names: Vec<String> = ["BannerSelectBreed", "BannerSelectAppearance", "BannerSelectProfession", "BannerEnterName", "ChooseBreed", "BreedHelp", "BreedNext"]
            .iter()
            .chain(&["SelectHeight", "SelectBuild", "SelectHead", "SelectAppearance", "SelectionHelpText", "ButtonSelectFace", "ButtonGoBack", "BackToBreed", "ButtonNext", "ProceedToProfession", "ButtonExit", "SelectionsWillBeSaved"])
            .chain(&["ProfBack", "ProfNext", "ProfButtonDefault", "ProfessionDefault"])
            .chain(&["Nickname:", "NameButtonSuggest", "NameSuggest", "NameButtonFinish", "Finish_AreteLanding", "NameBack", "ChooseNick", "NickHelp", "BreedProfession", "MustEnterName"])
            .chain(&["MorningStar", "GeosynchronousOrbit", "BoardingShuttle"])
            .map(|n| n.to_string())
            .collect();
        for (name, _) in BREED_NAMES {
            names.extend([format!("Button{name}"), format!("{name}Selected")]);
        }
        for breed in 1..=4 {
            names.push(format!("{}Description", breed_key(breed)));
        }
        for (_, head, body) in HEIGHT_BUTTONS.iter().chain(&BUILD_BUTTONS) {
            names.extend([head.to_string(), body.to_string()]);
        }
        for (name, _) in PROFESSION_NAMES {
            names.extend([format!("Inspect{name}"), format!("{name}Selected"), profession_description(name)]);
        }
        let missing: Vec<&String> = names.iter().filter(|n| texts.named(600, n).is_none()).collect();
        assert!(missing.is_empty(), "{missing:?}");
        assert!(texts.named(1003, "CharCreation_ExitQuestion").is_some());
        assert!(texts.named(1003, "CharCreation_SLOnlyProfession").is_some());
        let skin = crate::skin::Layer::open(&game.join("cd_image/gui/Default")).unwrap();
        let mut sprites: Vec<String> = ["EXIT", "NEXT", "BACK", "SHORT", "MEDIUM", "TALL", "SMALL", "LARGE", "NEXTHEAD", "PREVHEAD", "NAME", "RK"].iter().flat_map(|s| pictures(s)).collect();
        sprites.extend((1..=14).map(|n| format!("GFX_GUI_CC_NEW_PROF{n}")));
        let missing: Vec<&String> = sprites.iter().filter(|n| skin.bytes(n).is_none()).collect();
        assert!(missing.is_empty(), "{missing:?}");
    }

    /// The game's sound files have every sound the creation plays by name (`MAO_GAME`; no sound device is opened).
    #[test]
    fn the_creations_sounds_are_in_the_game() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let mut sandy = crate::audio::Sandy::new(&std::path::Path::new(&game).join("cd_image/sound"), crate::mixer::Engine::new(false));
        sandy.initialize();
        sandy.setup_gui_sounds();
        let mut names = vec!["SM_Sandy_CC_Opening_Music", "SM_Sandy_CC_Ambience", "SM_Sandy_CC_Select_Breed", "SM_Sandy_CC_Appearance", "SM_Sandy_CC_Select_Profession", "SM_Sandy_CC_Name_Character"];
        // (the name scene's `SM_Sandy_CC_Nick` and `SM_Sandy_CC_Name`, which GUI.dll asks for at a refused name, are in no
        // sound file: they are silent in the original too)
        names.extend(["SM_Sandy_CC_GUI_Select", "SM_Sandy_CC_GUI_Mouseover", "SM_Sandy_CC_GUI_Error"]);
        names.extend(BREED_NAMES.iter().map(|b| b.1));
        names.extend(PROFESSION_NAMES.iter().map(|p| p.1));
        let missing: Vec<&&str> = names.iter().filter(|n| sandy.source_of(mao_formats::sound::sound_id(n)).is_none()).collect();
        assert!(missing.is_empty(), "{missing:?}");
    }

    /// The Keeper and the Shade go by the Shadowlands' bit; an account whose flags are not known is let through.
    #[test]
    fn the_shadowlands_professions() {
        assert!(!shadowlands_professions(0) && !shadowlands_professions(1) && !shadowlands_professions(0x1d));
        assert!(shadowlands_professions(2) && shadowlands_professions(0x7f) && shadowlands_professions(-1));
    }

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
        let got: Vec<String> = heads(2, 3, false, &found).into_iter().map(|h| h.2).collect();
        assert_eq!(got, vec!["head_opifexfemale00.abiff", "head_opifexfemale31.abiff"]);
        // (FUN_1011d692's counts with and without the Shadowlands' flag, and the numbers it leaves out)
        let all = |n: &str| Some(n.len() as u32);
        let count = |breed, sex, shadowlands| heads(breed, sex, shadowlands, &all).len();
        assert_eq!((count(2, 3, false), count(2, 3, true), count(2, 2, false), count(2, 2, true)), (31, 42, 30, 43));
        assert_eq!((count(1, 3, false), count(1, 3, true)), (50 - 13 + 8 + 7, 75 - 13 + 8 + 7));
        assert_eq!((count(1, 2, false), count(1, 2, true)), (51 - 14 + 9 + 6, 71 - 14 + 9 + 9));
        assert_eq!((count(3, 2, false), count(3, 2, true), count(3, 3, false), count(3, 3, true)), (30, 41, 30, 43));
        assert_eq!((count(4, 1, false), count(4, 1, true)), (30, 41));
        assert!(head_left_out("head_solitusfemale", 22) && head_left_out("head_solitusmale", 37) && !head_left_out("head_solitusmale_asian", 3));
    }
}


