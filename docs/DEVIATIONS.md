# Where the client departs from the original

A catalog of every place found where this client does not do what the original Anarchy Online client does, so that
each can be decided: **port** the original's behaviour, or **leave** the client's. Made 2026-11-18.

## How to use it

Every entry has a number (`MOV-004`). Answer with numbers: "port MOV-004, CMB-011; leave the rest of Combat", or mark
the boxes in this file. An entry says what the original does (as the code's comments or the notes state it), what the
client does today, what a player notices, and where it is in the code.

- **Kind** says what sort of departure it is:
  - `OWN-RULE`: the client knowingly does otherwise.
  - `SIMPLIFIED`: an approximation or a stand-in.
  - `GUESS`: the original was not read: inferred or measured.
  - `SAFETY`: a guard or fallback the original lacks.
  - `MISSING`: the original does it, the client does not.
  - `ADDITION`: the client has it, the original does not.
- **Weight** is how much a player notices it: `high` (often, or it changes what one can do), `medium` (now and
  then, or plain but cosmetic), `low` (rare, subtle or internal). It is a judgement, made to sort the list.
- **(notes only)** marks an entry that rests on the project's notes and could not be found in the code either way.

## How it was made, and what it is not

- Seventeen readers went through the whole client (120,000 lines of code in `crates/`) and the project's notes in
  `docs/`, and wrote down every stated departure: 1767 raw entries. Every one quoted its source line, and each quote
  was checked by program against the file.
- A second pass, by area, folded the repeats, checked every notes-only entry against today's code and dropped the
  ones the code had since put right, and dropped what was only "not tried live". 936 entries were left.
- It lists what the code and the notes **say**. A place where the client departs from the original without anybody
  having written so (a rule fitted long ago and never marked, an original behaviour nobody has read) is not in it.
  What the original does is given as the comments give it; it was not read again from the decompiled code for this
  catalog.
- "The original" under ADDITION entries means the original has no such thing; whether a thing is wanted is yours
  to say.


<!-- status: made by status.py; run it again after marking entries -->
## Where it stands

Counted from the entries' own marks on 2026-11-19. "Open" is not looked at yet; "looked at, left" was read and not ported for a reason its entry gives; "left by decision" is your call to keep the client's way.

| | all | done | part done | looked at, left | left by decision | your call | open |
|---|---:|---:|---:|---:|---:|---:|---:|
| **high** | 54 | 46 | 7 | 0 | 1 | 0 | **0** |
| **medium** | 158 | 15 | 8 | 22 | 0 | 16 | **97** |
| **low** | 725 | 41 | 13 | 216 | 0 | 26 | **429** |
| **all** | 937 | 102 | 28 | 238 | 1 | 42 | **526** |

### By area

| Area | all | done | part done | looked at, left | left by decision | your call | open |
|---|---:|---:|---:|---:|---:|---:|---:|
| Movement (`MOV`) | 33 | 4 | 1 | 0 | 0 | 0 | **28** |
| Combat (`CMB`) | 38 | 1 | 2 | 1 | 0 | 0 | **34** |
| Targeting and mouse (`TGT`) | 17 | 2 | 0 | 0 | 1 | 0 | **14** |
| Death and revival (`DTH`) | 14 | 2 | 0 | 0 | 0 | 0 | **12** |
| NPCs and creatures (`NPC`) | 40 | 5 | 4 | 1 | 0 | 0 | **30** |
| Inventory and items (`INV`) | 84 | 12 | 3 | 4 | 0 | 0 | **65** |
| Nano programs (`NAN`) | 60 | 2 | 2 | 0 | 0 | 0 | **56** |
| Zoning and playfields (`ZON`) | 28 | 2 | 0 | 1 | 0 | 0 | **25** |
| Chat and commands (`CHT`) | 80 | 4 | 0 | 0 | 0 | 0 | **76** |
| Windows and interface (`WIN`) | 180 | 16 | 5 | 1 | 0 | 0 | **158** |
| Camera (`CAM`) | 13 | 1 | 0 | 0 | 0 | 0 | **12** |
| Rendering and world (`RND`) | 194 | 20 | 4 | 158 | 0 | 12 | **0** |
| Sound and music (`SND`) | 17 | 1 | 0 | 0 | 0 | 0 | **16** |
| Launcher, login, character creation (`LCH`) | 51 | 17 | 4 | 23 | 0 | 7 | **0** |
| Settings and files (`SET`) | 54 | 8 | 2 | 27 | 0 | 17 | **0** |
| Network and protocol (`NET`) | 13 | 2 | 0 | 9 | 0 | 2 | **0** |
| Other (`OTH`) | 21 | 3 | 1 | 13 | 0 | 4 | **0** |

### High weight not finished

- `CMB-022` Special attacks are not checked before sending (weapon bit, range, line of sight, queue) (MISSING) - part done
- `NPC-006` Vehicle riders and other characters use plain/medium bodies only (SIMPLIFIED) - part done
- `WIN-112` Several character action tiles do nothing (MISSING) - part done
- `WIN-114` Window chrome: no "?" help button, docking, auto-fade, original titles or art (MISSING) - part done
- `RND-137` Statels of generated buildings and rendering effects get no effects (MISSING) - part done
- `LCH-035` No account activation window and no limbo window (MISSING) - part done
- `LCH-036` Patching and licence dialog not done (MISSING) - part done

### Medium weight not finished, by area

**Movement** (7)

- `MOV-001` Own flight also read from worn items and running programs (OWN-RULE)
- `MOV-004` Own walking and collision where a playfield has no surfaces (and with noclip) (SIMPLIFIED)
- `MOV-005` Walk or run of other characters told by speed (SIMPLIFIED)
- `MOV-017` Client goes back to a place the server reports (SAFETY)
- `MOV-021` Flight: entering effect and the ways flight ends missing (MISSING) - part done
- `MOV-027` /follow is the client's own walk-after-target (ADDITION)
- `MOV-028` Walk-to-place with its own obstacle detours (ADDITION)

**Combat** (7)

- `CMB-003` Attack key checks only the first refusals (SIMPLIFIED)
- `CMB-004` Health drops in Stat messages become "unknown damage" lines (SIMPLIFIED)
- `CMB-005` Start Combat action attacks the target directly (SIMPLIFIED)
- `CMB-016` Creature with weapon points but no weapon swings the unarmed swing (GUESS)
- `CMB-023` Action tiles do not follow state; some tiles, CharacterAction 0xb2/0x14/0xfc, icon time text, special at attackable item undone (MISSING)
- `CMB-037` Damage meter window (client's own) (ADDITION)
- `CMB-038` Floating combat numbers (ADDITION)

**Targeting and mouse** (2)

- `TGT-004` Attack key: can-attack check is partial (SIMPLIFIED)
- `TGT-005` Friendly means the same side only for targeting keys (SIMPLIFIED)

**Death and revival** (1)

- `DTH-007` White fade at own death: that effect 3000 is the white is a guess (GUESS)

**NPCs and creatures** (10)

- `NPC-001` Silent NPC is told its chat is closed and asked again after two seconds (OWN-RULE)
- `NPC-007` A creature with no standing animation holds its walk's first frame (SIMPLIFIED)
- `NPC-008` Hoverboards/hoverbikes drawn from a template worked out from the program (SIMPLIFIED)
- `NPC-015` Built-in machines, doors and mail terminals found by inferred rules (GUESS)
- `NPC-016` Creature movement mode (ground/stand) and run speed read from measured bytes (GUESS)
- `NPC-017` Creature run speed is the characters' rule, learnt or fallback, not read (GUESS)
- `NPC-018` Creature template found by scanning, not by the original's field order (GUESS)
- `NPC-019` Creatures walk at a flat 1.5 m/s (GUESS)
- `NPC-020` Mission request scope for an unknown terminal kind is a fixed 5 (GUESS)
- `NPC-039` Mission helper: "Looking for" lists and "Keep asking until one fits" (ADDITION)

**Inventory and items** (22)

- `INV-005` Item tooltip is the client's own layout, with requirements and "When worn" list (OWN-RULE)
- `INV-006` Moves by use go one at a time with a wait (OWN-RULE)
- `INV-007` Right click takes an item out of a corpse (OWN-RULE)
- `INV-008` Splitting a stack asks by a number box (OWN-RULE)
- `INV-019` Item cells counted along rows and kept in one places.ini for all characters (SIMPLIFIED)
- `INV-020` Item dropped on a wear cell is sent without the original's drop handling (SIMPLIFIED)
- `INV-021` Loot windows keep the server's place order instead of sorting by name (SIMPLIFIED)
- `INV-022` Right click sends Use; wear/take-off rule not in the window (SIMPLIFIED)
- `INV-023` Spell texts: only listed spell types written, some English words fixed (SIMPLIFIED)
- `INV-038` Using a bag again closes/shows its window locally, sending nothing (GUESS)
- `INV-051` Client does not send bank use (0x6a), corpse slot action (0x8e), pet command (0x80) (MISSING)
- `INV-052` Inventory behaviours left from the original reading (ground drop, use on a character, and others) (MISSING)
- `INV-053` Item effects with requirements other than level are left out (MISSING)
- `INV-054` Item pages: shop price, nano radius, armour/implant/nano sections, fixtures, towers, buildings missing (MISSING)
- `INV-055` Player trade: partner's items not named; no out-of-range check (MISSING)
- `INV-056` Requirement checks: only the client's own character, no apply-on target/user/secondary (MISSING)
- `INV-057` Requirement operators needing world state give "unknown" (MISSING)
- `INV-058` Requirement texts: special criteria, breed/gender names, apply-on reordering not ported (MISSING)
- `INV-079` Auto loot of fresh corpses (ADDITION)
- `INV-080` Container and overflow windows have a "Take all" button (ADDITION)
- `INV-081` Tradeskill guide made from item descriptions (ADDITION)
- `INV-084` A swap by right click is judged without the item that comes off (ADDITION)

**Nano programs** (9)

- `NAN-001` Target NCU windows are turned on with /setoption (OWN-RULE)
- `NAN-004` NCU memory in use is summed by the client (SIMPLIFIED)
- `NAN-006` Spell criteria the client cannot decide count as failing (SIMPLIFIED)
- `NAN-028` Cast, tracer, hit and spiral effects: partial decode with chosen sizes and timings (GUESS)
- `NAN-038` Cast effect: hand cords draw nothing; phases 3-4 (projectile, impact) not played (MISSING)
- `NAN-039` Nano programs window: programs cannot be placed by hand; radius value, widths and sort not kept (MISSING) - part done
- `NAN-040` NCU list: no Auto Arrange submenu, radius value or kept widths and sort (MISSING) - part done
- `NAN-056` Nano landing glow: three widening rings, colour from stats or school (ADDITION)
- `NAN-057` Programs fade while a cast or recharge runs; locks from the client's own timing (ADDITION)

**Zoning and playfields** (6)

- `ZON-003` Door zoning by distance to the table door's place (SIMPLIFIED)
- `ZON-004` Own door use opens the door for a fixed 5 seconds (SIMPLIFIED)
- `ZON-006` Instanced playfield object ids read from live bytes (GUESS)
- `ZON-024` Client says the Shadowlands expansion is missing at the Jobe booth (ADDITION)
- `ZON-025` Whom-Pah destinations learned from play and remembered (ADDITION)
- `ZON-026` Whom-Pah window with a Go button that walks to the booth (ADDITION)

**Chat and commands** (6)

- `CHT-002` /help and /version show the client's own text (OWN-RULE)
- `CHT-016` Fight lines cannot tell a pet hit or a player attacker (SIMPLIFIED)
- `CHT-050` Friends list: no flashing row for unread tells, no red folders, tell windows not under Chat Windows, no tell-window menu (MISSING)
- `CHT-051` Private group invitation box has no chat-window list (MISSING)
- `CHT-069` Chat commands of the client's own (ADDITION)
- `CHT-070` Use key adds a "Using X" line and a "Nothing targeted to use" notice (ADDITION)

**Windows and interface** (29)

- `WIN-002` Dark themes draw controls in the border colour instead of the skin's Default (OWN-RULE)
- `WIN-003` Grid columns centred in the width (OWN-RULE)
- `WIN-004` In-game web pages are a WebView2 overlay behind a local proxy (OWN-RULE)
- `WIN-005` Map: interiors and instanced playfields get the client's own explored-grid map (OWN-RULE)
- `WIN-006` Wear window grid scaled to fill the window (OWN-RULE)
- `WIN-025` Chat windows of a frame are tabs of one window, others get windows of their own (SIMPLIFIED)
- `WIN-026` Control center menus are the client's own hand-written lists (SIMPLIFIED)
- `WIN-027` Fonts: Verdana drawn regular whatever the weight, sizes by egui points (SIMPLIFIED)
- `WIN-028` Inventory and wear windows share one key and one window (SIMPLIFIED)
- `WIN-029` Mission selection window drawn by hand with its own words (SIMPLIFIED)
- `WIN-030` NPC give-trade bar drawn by hand with its own texts (SIMPLIFIED)
- `WIN-031` Only some menu entries open windows (SIMPLIFIED)
- `WIN-032` Option panel pages come from an AOU skin's Root.xml, not the original's built-in pages (SIMPLIFIED)
- `WIN-033` Option window "Key bindings" tab is not the original's editor (SIMPLIFIED)
- `WIN-034` Rollup area: no drag to reorder, client's own widths, heights and docking controls (SIMPLIFIED)
- `WIN-035` Shop lists are hand-drawn rows with fixed sizes, not item-container lists (SIMPLIFIED)
- `WIN-036` Stats not in the server's list shown as a dash (SIMPLIFIED)
- `WIN-037` Tool tip frame drawn as lines, not the original's surfaces (SIMPLIFIED)
- `WIN-083` Skill values for other characters are computed and unchecked (GUESS)
- `WIN-084` Window frame and panel translucency, title size are the client's own (GUESS)
- `WIN-116` Character info page leaves out several of the original's parts (MISSING)
- `WIN-118` Land control Build and War buttons and placement circle not ported (MISSING)
- `WIN-119` Mission journal does not keep columns, sort or active mission, and has no Update Team Members (MISSING) - part done
- `WIN-120` Playfield map: shadow textures absent, no second patch texture, no rooms' pictures (MISSING)
- `WIN-121` Text views that are not editable cannot be selected by mouse (MISSING)
- `WIN-153` Client settings window beside the original's option panel (ADDITION)
- `WIN-154` Client's own message box for 14 seconds (ADDITION)
- `WIN-155` Hotbar "is not ready: queued" message and own cooldowns (ADDITION)
- `WIN-156` Map: dots for nearby characters and the character arrow (ADDITION)

**Camera** (1)

- `CAM-006` Mouse turn, zoom speed and wheel step are fixed numbers (GUESS)

**Rendering and world** (15)

- `RND-001` Client's own sky and light when the scripts give none (OWN-RULE) - looked at, left
- `RND-002` Plants kept cut out at half alpha instead of blended (OWN-RULE) - looked at, left
- `RND-004` Spell flight 0.45 s and landing 1.4 s are the client's own timings (OWN-RULE) - looked at, left
- `RND-006` Terrain keeps slope lighting and shadow map instead of the original's flat-lit ground (OWN-RULE) - looked at, left
- `RND-022` Attach points use fixed figures for a standing body, not the posed skeleton (SIMPLIFIED) - looked at, left
- `RND-023` Buff ring ribbon drawn as sprites, clocks tied to 30 frames per second (SIMPLIFIED) - looked at, left
- `RND-024` Flat grey floor under interiors whose ground is not read (SIMPLIFIED) - looked at, left
- `RND-026` Map: ground picture shrunk and averaged (SIMPLIFIED) - looked at, left
- `RND-027` Name tags hidden by a line-of-sight test through the surfaces, a fourth of them per frame (SIMPLIFIED) - looked at, left
- `RND-098` Atlas of Rubi-Ka map coordinates measured (GUESS) - looked at, left
- `RND-099` Shadowlands sky still wrong, cause not found (GUESS) - looked at, left
- `RND-139` Effect locators: bone, weapon and mesh attractors; effect mesh details not drawn (MISSING) - part done
- `RND-140` Statel zones of the second layout are not read (MISSING) - looked at, left
- `RND-141` Statels' own matrix (second flag layout) not applied (MISSING) - part done
- `RND-142` Worn meshes: the original's two hiding modes not ported (MISSING) - looked at, left

**Sound and music** (1)

- `SND-002` Indoors the zone is always 0 (music, district info) (SIMPLIFIED)

**Launcher, login, character creation** (4)

- `LCH-001` Launcher is its own window and process that starts the game (OWN-RULE) - looked at, left
- `LCH-002` Launcher settings pages keep their names but have this client's own contents (OWN-RULE) - looked at, left
- `LCH-003` Loading screen picture is tinted blue; zoning also shows it (OWN-RULE) - part done
- `LCH-004` Wide window keeps the 4:3 view height in the login hall and character creation (OWN-RULE) - looked at, left

**Settings and files** (5)

- `SET-001` Chat window layout kept in chatwindows.ini (OWN-RULE) - looked at, left
- `SET-002` Hotbars saved in hotbar.ini (OWN-RULE) - looked at, left
- `SET-003` New windows start at the original client's frame, scaled; another character's layout used (OWN-RULE) - part done
- `SET-005` Window places, sizes and open windows kept in the client's own ini files (OWN-RULE) - looked at, left
- `SET-032` Original's key editor not ported; Key bindings tab leads to the client's own settings (MISSING) - looked at, left

**Network and protocol** (1)

- `NET-003` Organisation and many other messages built from the decompile only (GUESS) - looked at, left

**Other** (1)

- `OTH-006` Tradeskill guide is a reading of prose, not a recipe table (GUESS) - looked at, left

### Additions waiting for your call

"Your call": a client addition read overnight; its entry recommends keeping or removing it.

- `RND-183` (medium) Anti-aliasing, anisotropic filtering, mips and hemisphere light: Addition: recommend KEEP: anti-aliasing, anisotropic filtering and mips are display-quality settings with no rule of the original's to break (its DirectX 7 devi
- `RND-184` (medium) Coloured boxes stand in for characters with no model: Addition: recommend KEEP: the original has a model for every character (its fallback is `unfinished-breed.cir`, see RND-028), so the boxes only show where this 
- `RND-185` (medium) Distance haze when the area has no fog: Addition: recommend REMOVE: the original's only fog is the area's VisualFog_t; with none it draws clear to its far clip, so the haze is a look of the client's o
- `RND-186` (medium) Name plates are the client's own: Addition: recommend KEEP: the original's name tags are VisualSprite_t in the world (HandleNametags 0x100268ee); until those are ported (RND-027) the client's pl
- `RND-187` (medium) Plants sway in the wind and bend away from the player: Addition: recommend REMOVE: the original has no plant flag and moves plants only through their meshes' own animations; the sway and the bend away from the playe
- `RND-188` (medium) Real sun shadows from the playfield's props, shadow-map options: Addition: recommend KEEP: it is an option and `shadows` off gives the original's blob shadows only (GfxVisualSimpleShadow_c); suggest it default to off if the o
- `RND-189` (low) Drawing scale with FSR 1 upscaling and a mip bias: Addition: recommend KEEP: a performance option for slow machines that defaults to full scale; it does not change the picture's content.
- `RND-190` (low) Frame limiter and background frame rate: Addition: recommend KEEP: frame limiting is a machine setting with no rule of the original's to break.
- `RND-191` (low) Generic emitter: dense clouds dimmed, jitter and clamps: Addition: recommend REMOVE: the dimming, jitter and clamps change the generic emitter's look away from the template's own numbers; if a template makes a cloud t
- `RND-192` (low) Hit effect fade-in and fade-out added: Addition: recommend REMOVE: the original's hit effects end as their own controls fade them; an extra fade in and out changes their look.
- `RND-193` (low) Textures block-compressed on the card, on by default: Addition: recommend KEEP: compression saves video memory and is a launcher option; the original's DX7 device also took compressed (DXT) textures where the files
- `RND-194` (low) View distance and character distance settings: Addition: recommend KEEP: the original has a view distance setting of its own in its options; the character distance is a performance option.
- `LCH-043` (medium) An account with no characters is sent back to the form: Addition: recommend REMOVE: the original goes to the character screen for every character list (GUI.dll `LoginModule_c::SlotLoginReply` 0x10012179: reply 0xe ->
- `LCH-044` (medium) Character screen figure is dressed from a remembered look (looks.ini): Addition: recommend KEEP: no store of looks was found in the original (GUI.dll's login strings name only the creation meshes for `LoginWorld_c`; the list's `Cha
- `LCH-046` (low) "No heads found" message in creation: Addition: recommend KEEP: it shows only when the game's files have none of the breed's head meshes, where the original would have offered no head and sent a cre
- `LCH-048` (low) Game folder is looked for in usual install places: Addition: recommend KEEP: the original runs from inside its own folder and never needs to find it; the client is installed apart from the game, so looking in th
- `LCH-049` (low) Launcher has its own backgrounds (night sky, own picture) and a news-page switch: Addition: recommend KEEP: the default (`background = game`) is the original's own pictures; the night sky and an own picture only show when chosen. The news-pag
- `LCH-050` (low) Launcher Visual page adds frame limit, shadows, monitor, presets: Addition: recommend KEEP: the Visual page's added rows are the wgpu counterparts of the original's Direct3D choices (see SET-042), which have no meaning here.
- `LCH-051` (low) Launcher window hidden at start, web view moved in after load: Addition: recommend KEEP: nothing a player sees differs from the original but the missing white flash of an empty web view; it is how a WebView2 child is shown 
- `SET-038` (medium) Character draw distance and auto-loot settings: Addition: recommend KEEP: the character draw distance is the original's own (`/chardist`, registered by GUI.dll FUN_100bb3e0), only reached here also from a set
- `SET-039` (medium) Frame limit, background limit and vsync choice: Addition: recommend KEEP: the original's only frame control is its fixed cap and the `/framerate` counter (`FramerateView_t`); a limit, a background limit and t
- `SET-040` (medium) Interface size setting: Addition: recommend KEEP: the original has no interface scale (no scale string in GUI.dll), and at today's resolutions its 1024x768-sized windows are unreadably
- `SET-041` (medium) Item finder files of what was seen: Addition: recommend KEEP: the item finder is one of the client's own tools the user asked for (memory "MAO own tools"); its files are only written beside the cl
- `SET-042` (medium) Own graphics and display options (presets, scale, FSR, shadows, monitor, distances): Addition: recommend KEEP: the original's graphics settings are Direct3D 7 device, colour depth and texture choices that have no meaning for wgpu (LCH-002); the 
- `SET-043` (medium) Themes, skin folders and the built-in palette: Addition: recommend KEEP: the default theme is `original` (the game's own skin and GUIColors.xml); the AOU themes and other folders only apply when chosen.
- `SET-044` (medium) Window modes with F11 / Alt+Enter and a chosen monitor: Addition: recommend KEEP, but note: Alt+Enter is one of the original's three chat keys (GUI.dll's key table at 0x101aab28: `COMMAND_TEXT_INPUT_START_TEXT_INPUT`
- `SET-045` (low) Account file with last-played character first: Addition: recommend KEEP the pre-selection, which is the original's own (`CharSelectWindow_c::SlotCharListReceived` 0x1000ebd5 selects the row of the value `Sel
- `SET-046` (low) AutoAFK controlled by /setoption: Addition: recommend KEEP: `/setoption` is the original's own command for setting any distributed value, and `AutoAFK` is one (LoginPrefs.xml), so the original's
- `SET-047` (low) Client brings its own PlanetMap folder: Addition: recommend KEEP: the game's own PlanetMap folder is still read; the client's folder only adds maps (memory: they reached the real folder 2026-11-15) wi
- `SET-048` (low) Crash log, run markers and error box: Addition: recommend KEEP: the crash log, run markers and error box only show where the client fails, which is where the original would have shown its own crash 
- `SET-049` (low) Extra key actions of the client: Addition: recommend KEEP, with one note: Alt+Enter (CycleWindowMode) is one of the original's chat-input keys in GUI.dll's key table (see SET-044); the other ex
- `SET-050` (low) Game folder is a setting with Browse picker and auto-detection: Addition: recommend KEEP: the client lives apart from the game's folder (LCH-048), so it needs to be told or to find where the game is.
- `SET-051` (low) Game log written to game-log.txt: Addition: recommend KEEP: a log file beside the settings that a player never has to look at; it carries nothing of the account.
- `SET-052` (low) Option panel folder list uses egui widgets with a "Client settings" link: Addition: recommend KEEP: the folder list is the original's (its folders and pages from the option panel files); the "Client settings" link is the only way from
- `SET-053` (low) Pet window, planet map and camera settings in mao.ini: Addition: recommend KEEP: the settings themselves are the original's (`PetWindowConfig`, `PlanetMapIndexFile`, `PreferredCameraMode`); only where they are kept 
- `SET-054` (low) Tradeskill guide keeps a record of what the window said: Addition: recommend KEEP: the tradeskill guide is one of the client's own tools; the window itself still shows each answer as the original does.
- `NET-012` (low) Help assistant: hosted model and guide crawler: Addition: recommend KEEP: off by default, and the hosted model and the guide crawler only run when the player sets them up; the original game never contacts the
- `NET-013` (low) Protocol read by a schema generated from SmokeLounge/AOSharp definitions: Addition: recommend KEEP: the schema is how the client reads and writes messages, not something a player sees; where the original's own readers have been read t
- `OTH-018` (medium) Help assistant: a small language model run inside the client: Addition: recommend KEEP: the help assistant is one of the client's own tools the user asked for; the model is fetched only on request and runs only while the w
- `OTH-019` (low) Help assistant: crafting texts as a source: Addition: recommend KEEP: part of the client's own help assistant; the texts are the game's own books and descriptions.
- `OTH-020` (low) Linux and macOS builds exist: Addition: recommend KEEP: other platforms change nothing for the Windows client.
- `OTH-021` (low) Screenshot key saves a PNG with a notes file: Addition: recommend KEEP the notes file (the user's tool for finding places and meshes); the original's screenshot key saves a picture too, so only the .txt bes

Low weight: see the counts above and each area's lists below.
<!-- /status -->

## The numbers

| Area | OWN-RULE | SIMPLIFIED | GUESS | SAFETY | MISSING | ADDITION | all | high |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| [Movement](#movement) (`MOV`) | 3 | 9 | 4 | 3 | 7 | 7 | 33 | 1 |
| [Combat](#combat) (`CMB`) | 2 | 12 | 7 | 0 | 15 | 2 | 38 | 2 |
| [Targeting and mouse](#targeting-and-mouse) (`TGT`) | 2 | 8 | 2 | 1 | 3 | 1 | 17 | 3 |
| [Death and revival](#death-and-revival) (`DTH`) | 5 | 1 | 4 | 2 | 2 | 0 | 14 | 2 |
| [NPCs and creatures](#npcs-and-creatures) (`NPC`) | 4 | 9 | 14 | 0 | 11 | 2 | 40 | 5 |
| [Inventory and items](#inventory-and-items) (`INV`) | 15 | 22 | 8 | 2 | 31 | 6 | 84 | 10 |
| [Nano programs](#nano-programs) (`NAN`) | 3 | 24 | 9 | 1 | 18 | 5 | 60 | 0 |
| [Zoning and playfields](#zoning-and-playfields) (`ZON`) | 2 | 3 | 7 | 4 | 7 | 5 | 28 | 2 |
| [Chat and commands](#chat-and-commands) (`CHT`) | 15 | 13 | 16 | 3 | 21 | 12 | 80 | 3 |
| [Windows and interface](#windows-and-interface) (`WIN`) | 21 | 61 | 22 | 4 | 42 | 30 | 180 | 12 |
| [Camera](#camera) (`CAM`) | 3 | 2 | 2 | 2 | 3 | 1 | 13 | 1 |
| [Rendering and world](#rendering-and-world) (`RND`) | 19 | 78 | 26 | 11 | 48 | 12 | 194 | 6 |
| [Sound and music](#sound-and-music) (`SND`) | 1 | 9 | 1 | 0 | 6 | 0 | 17 | 0 |
| [Launcher, login, character creation](#launcher-login-character-creation) (`LCH`) | 8 | 13 | 6 | 6 | 9 | 9 | 51 | 4 |
| [Settings and files](#settings-and-files) (`SET`) | 22 | 3 | 3 | 1 | 8 | 17 | 54 | 2 |
| [Network and protocol](#network-and-protocol) (`NET`) | 0 | 2 | 5 | 3 | 1 | 2 | 13 | 0 |
| [Other](#other) (`OTH`) | 2 | 3 | 4 | 0 | 8 | 4 | 21 | 1 |
| **all** | 127 | 272 | 140 | 43 | 240 | 115 | 937 | 54 |

## Start here: what a player notices most

The `high` entries that are not additions, by area. The full entries are below under the same numbers.

**Movement**

- [x] `MOV-020` Other characters' flight and crafts known only from spell lists (MISSING) **(done)**

**Combat**

- [x] `CMB-015` Special-attack list built from worn weapons (GUESS) **(done)**
- [ ] `CMB-022` Special attacks are not checked before sending (weapon bit, range, line of sight, queue) (MISSING) **(part done)**

**Targeting and mouse**

- [x] `TGT-001` The character itself is last on a click line (OWN-RULE) **(done)**
- [x] `TGT-003` Mouse picking uses cylinders, not the dynels' collision (SIMPLIFIED) **(done)**

**Death and revival**

- [x] `DTH-013` Corpse whose body is missing is not drawn (MISSING) **(done)**
- [x] `DTH-014` Own death: reasons, death music, resurrection line and windows' signal not ported (MISSING) **(done)**

**NPCs and creatures**

- [x] `NPC-005` Per-pet command never captured; /pet sends an empty list (SIMPLIFIED) **(done)**
- [ ] `NPC-006` Vehicle riders and other characters use plain/medium bodies only (SIMPLIFIED) **(part done)**
- [x] `NPC-014` Shop: double-click buys or removes, right click opens the page (GUESS) **(done)**
- [x] `NPC-028` Other characters and humanoid NPCs are drawn at scale 100 (MISSING) **(done)**
- [x] `NPC-029` Pet window: no running-nano list per pet, no Remember Pet Names (MISSING) **(done)**

**Inventory and items**

- [x] `INV-001` A shop is left when the character walks 8 m away (OWN-RULE) **(done)**
- [x] `INV-002` Item picked up by a press that moves (drag), not only on release (OWN-RULE) **(done)**
- [x] `INV-003` No use of an item by double click (OWN-RULE) **(done)**
- [x] `INV-004` Right click with an item in hand always does a tradeskill combine (OWN-RULE) **(done)**
- [x] `INV-016` Item let go over the world from a container is used on what is there (SIMPLIFIED) **(done)**
- [x] `INV-017` Item stat effects: only some spell types read (SIMPLIFIED) **(done)**
- [x] `INV-018` Mail terminals come from a fixed list (SIMPLIFIED) **(done)**
- [x] `INV-048` Backpack stacks cannot be joined or split inside a bag window (MISSING) **(done)**
- [x] `INV-049` Item use sent at once instead of after the item timer (MISSING) **(done)**
- [x] `INV-050` No item menu in container windows (MISSING) **(done)**

**Zoning and playfields**

- [x] `ZON-001` Doors open on approach within 4 m (OWN-RULE) **(done)**
- [x] `ZON-017` Doors: sound, own closing time and locked-door check not ported (MISSING) **(done)**

**Chat and commands**

- [x] `CHT-001` Chat tabs moved by menu entries instead of dragging (OWN-RULE) **(done)**
- [x] `CHT-048` /raid opens the raid window instead of running raid commands (MISSING) **(done)**
- [x] `CHT-049` Raid window shows only the own team and its buttons do nothing (MISSING) **(done)**

**Windows and interface**

- [x] `WIN-001` Shortcut bars are the client's own design (OWN-RULE) **(done)**
- [x] `WIN-022` Actions window lists a fixed catalogue, not the character's own list (SIMPLIFIED) **(done)**
- [x] `WIN-023` All non-own containers share one window named "loot" (SIMPLIFIED) **(done)**
- [x] `WIN-024` Stat bonuses are summed by the client from items, perks, nanos and organization (SIMPLIFIED) **(done)**
- [x] `WIN-109` Bag, bank, inventory and wear windows have no List Mode, Auto Arrange, Labels On Top or Unlock All menus (MISSING) **(done)**
- [x] `WIN-110` City controller window: no land-buying dialog, no item lock or pointer tip, no Escape option (MISSING) **(done)**
- [x] `WIN-111` Popup menus lack icons, sub-menus, sliders, separators, type-to-search (MISSING) **(done)**
- [ ] `WIN-112` Several character action tiles do nothing (MISSING) **(part done)**
- [x] `WIN-113` Toggling character actions always show the same tile (MISSING) **(done)**
- [ ] `WIN-114` Window chrome: no "?" help button, docking, auto-fade, original titles or art (MISSING) **(part done)**

**Camera**

- [x] `CAM-001` WoW-style camera controls (OWN-RULE) **(done)**

**Rendering and world**

- [x] `RND-020` Character meshes drawn fully opaque (SIMPLIFIED) **(done)**
- [x] `RND-021` Character skin colour is a plain tan (SIMPLIFIED) **(done)**
- [x] `RND-135` Other characters' crafts not drawn (MISSING) **(done)**
- [x] `RND-136` Statel effects with their own colour or radius not drawn (MISSING) **(done)**
- [ ] `RND-137` Statels of generated buildings and rendering effects get no effects (MISSING) **(part done)**
- [x] `RND-138` Weather: cloud layer (and other weather sprites) not drawn (MISSING) **(done)**

**Launcher, login, character creation**

- [x] `LCH-034` Character creation: figures and ships do not animate, no selection glow, no expansion rules (MISSING) **(done)**
- [ ] `LCH-035` No account activation window and no limbo window (MISSING) **(part done)**
- [ ] `LCH-036` Patching and licence dialog not done (MISSING) **(part done)**
- [x] `LCH-037` The picked character and the hall's figures cannot be turned or animated (MISSING) **(done)**

**Settings and files**

- [x] `SET-030` Option panel: many preferences kept but not acted on (MISSING) **(done)**
- [x] `SET-031` Per-character Prefs.xml key bindings not read (MISSING) **(done)**

**Other**

- [x] `OTH-010` Notum Wars windows absent (MISSING) **(done)**

## The client's own additions

Things the original does not have. "Port" has no meaning for these: the choice is **keep** or **remove**.

**Movement**

- [ ] `MOV-027` /follow is the client's own walk-after-target
- [ ] `MOV-028` Walk-to-place with its own obstacle detours
- [ ] `MOV-029` /sit command is the client's own
- [ ] `MOV-030` Auto-run key
- [ ] `MOV-031` Own character glides between server updates
- [ ] `MOV-032` Walk-through-props toggle (noclip) and key layout
- [ ] `MOV-033` Walk/run mode announced on entering each playfield

**Combat**

- [ ] `CMB-037` Damage meter window (client's own)
- [ ] `CMB-038` Floating combat numbers

**Targeting and mouse**

- [x] `TGT-017` Double click uses what the first click took; Shift/Ctrl/Alt clicks not ported **(left by decision)**

**NPCs and creatures**

- [ ] `NPC-039` Mission helper: "Looking for" lists and "Keep asking until one fits"
- [ ] `NPC-040` Clicked NPC answer echoed locally; client line for rejected items

**Inventory and items**

- [ ] `INV-079` Auto loot of fresh corpses
- [ ] `INV-080` Container and overflow windows have a "Take all" button
- [ ] `INV-081` Tradeskill guide made from item descriptions
- [ ] `INV-082` Backpack count heading
- [ ] `INV-083` Item finder (/find) is the client's own tool
- [ ] `INV-084` A swap by right click is judged without the item that comes off (added 2026-11-18)

**Nano programs**

- [ ] `NAN-056` Nano landing glow: three widening rings, colour from stats or school
- [ ] `NAN-057` Programs fade while a cast or recharge runs; locks from the client's own timing
- [ ] `NAN-058` "Nano programs available." chat line
- [ ] `NAN-059` Nano program and NCU tips are the client's own layout
- [ ] `NAN-060` Right-click "Cancel this nano" entry is the client's own

**Zoning and playfields**

- [ ] `ZON-024` Client says the Shadowlands expansion is missing at the Jobe booth
- [ ] `ZON-025` Whom-Pah destinations learned from play and remembered
- [ ] `ZON-026` Whom-Pah window with a Go button that walks to the booth
- [ ] `ZON-027` Interior exit offered like a Whom-Pah booth
- [ ] `ZON-028` Whom-Pah booths labelled by their sign texture

**Chat and commands**

- [ ] `CHT-069` Chat commands of the client's own
- [ ] `CHT-070` Use key adds a "Using X" line and a "Nothing targeted to use" notice
- [ ] `CHT-071` "Chat server is not connected" line
- [ ] `CHT-072` /fxscript with only a name gets a parameter mark added
- [ ] `CHT-073` /setoption names the client's own settings
- [ ] `CHT-074` Client's own chat status lines
- [ ] `CHT-075` Deposit to Market adds a client line
- [ ] `CHT-076` Inspect result also told as a chat line
- [ ] `CHT-077` New mail needs a recipient and subject (client's own check)
- [ ] `CHT-078` Own confirmation lines for several commands
- [ ] `CHT-079` Scripts folder beside the client's settings is searched first
- [ ] `CHT-080` Team invitation text is the client's own

**Windows and interface**

- [x] `WIN-151` Fog-of-war corner map and overlay map are the client's own **(done)**
- [x] `WIN-152` Smart hotbars with priority lists and queued presses **(done)**
- [ ] `WIN-153` Client settings window beside the original's option panel
- [ ] `WIN-154` Client's own message box for 14 seconds
- [ ] `WIN-155` Hotbar "is not ready: queued" message and own cooldowns
- [ ] `WIN-156` Map: dots for nearby characters and the character arrow
- [ ] `WIN-157` A skin without its own TextColors.xml takes the game's
- [ ] `WIN-158` Actions tooltip has the client's own text
- [ ] `WIN-159` AOU mod skins bundled and selectable
- [ ] `WIN-160` Creature inspect page is the client's own headline with health
- [ ] `WIN-161` Error boxes are queued, never repeated
- [ ] `WIN-162` Escape always closes the client's own Options, Whom-Pah, Browser and Help windows
- [ ] `WIN-163` Help assistant ("Ask the guides") is the client's own window
- [ ] `WIN-164` Help window (Ctrl+H) with find and chat input history
- [ ] `WIN-165` Hover breakdown tooltips on stats and skills
- [ ] `WIN-166` Inbox status line and "Sending ..." text
- [ ] `WIN-167` Info view file:// pages also search the game's text folder
- [ ] `WIN-168` Inspect window: title carries the character's name, right click opens the item page
- [ ] `WIN-169` Missing game texts fall back to the client's own words
- [ ] `WIN-170` Performance overlay
- [ ] `WIN-171` Plain strip drawn when there is no skin
- [ ] `WIN-172` Planet map window: map choice list inside the window
- [ ] `WIN-173` Shop status line "Trade complete."
- [ ] `WIN-174` Shop window title carries the shop's name
- [ ] `WIN-175` Skills window messages use the client's own words
- [ ] `WIN-176` The client can build planet maps of its own and list any map folder
- [ ] `WIN-177` The client's own tool windows (damage meter, item finder, guide)
- [ ] `WIN-178` Web pages: loading page and 8 s / 30 s proxy time-outs
- [ ] `WIN-179` Window icon is the client's own emblem on a slate square
- [ ] `WIN-180` Windows that can be docked include ones the original does not dock

**Camera**

- [ ] `CAM-013` Camera mode change prints a chat line

**Rendering and world**

- [ ] `RND-183` Anti-aliasing, anisotropic filtering, mips and hemisphere light **(your call)**
- [ ] `RND-184` Coloured boxes stand in for characters with no model **(your call)**
- [ ] `RND-185` Distance haze when the area has no fog **(your call)**
- [ ] `RND-186` Name plates are the client's own **(your call)**
- [ ] `RND-187` Plants sway in the wind and bend away from the player **(your call)**
- [ ] `RND-188` Real sun shadows from the playfield's props, shadow-map options **(your call)**
- [ ] `RND-189` Drawing scale with FSR 1 upscaling and a mip bias **(your call)**
- [ ] `RND-190` Frame limiter and background frame rate **(your call)**
- [ ] `RND-191` Generic emitter: dense clouds dimmed, jitter and clamps **(your call)**
- [ ] `RND-192` Hit effect fade-in and fade-out added **(your call)**
- [ ] `RND-193` Textures block-compressed on the card, on by default **(your call)**
- [ ] `RND-194` View distance and character distance settings **(your call)**

**Launcher, login, character creation**

- [ ] `LCH-043` An account with no characters is sent back to the form **(your call)**
- [ ] `LCH-044` Character screen figure is dressed from a remembered look (looks.ini) **(your call)**
- [x] `LCH-045` Logging out (/camp) restarts the launcher and logs in again **(done)**
- [ ] `LCH-046` "No heads found" message in creation **(your call)**
- [x] `LCH-047` Character list shows free slots and offers New Character only while one is free **(done)**
- [ ] `LCH-048` Game folder is looked for in usual install places **(your call)**
- [ ] `LCH-049` Launcher has its own backgrounds (night sky, own picture) and a news-page switch **(your call)**
- [ ] `LCH-050` Launcher Visual page adds frame limit, shadows, monitor, presets **(your call)**
- [ ] `LCH-051` Launcher window hidden at start, web view moved in after load **(your call)**

**Settings and files**

- [ ] `SET-038` Character draw distance and auto-loot settings **(your call)**
- [ ] `SET-039` Frame limit, background limit and vsync choice **(your call)**
- [ ] `SET-040` Interface size setting **(your call)**
- [ ] `SET-041` Item finder files of what was seen **(your call)**
- [ ] `SET-042` Own graphics and display options (presets, scale, FSR, shadows, monitor, distances) **(your call)**
- [ ] `SET-043` Themes, skin folders and the built-in palette **(your call)**
- [ ] `SET-044` Window modes with F11 / Alt+Enter and a chosen monitor **(your call)**
- [ ] `SET-045` Account file with last-played character first **(your call)**
- [ ] `SET-046` AutoAFK controlled by /setoption **(your call)**
- [ ] `SET-047` Client brings its own PlanetMap folder **(your call)**
- [ ] `SET-048` Crash log, run markers and error box **(your call)**
- [ ] `SET-049` Extra key actions of the client **(your call)**
- [ ] `SET-050` Game folder is a setting with Browse picker and auto-detection **(your call)**
- [ ] `SET-051` Game log written to game-log.txt **(your call)**
- [ ] `SET-052` Option panel folder list uses egui widgets with a "Client settings" link **(your call)**
- [ ] `SET-053` Pet window, planet map and camera settings in mao.ini **(your call)**
- [ ] `SET-054` Tradeskill guide keeps a record of what the window said **(your call)**

**Network and protocol**

- [ ] `NET-012` Help assistant: hosted model and guide crawler **(your call)**
- [ ] `NET-013` Protocol read by a schema generated from SmokeLounge/AOSharp definitions **(your call)**

**Other**

- [ ] `OTH-018` Help assistant: a small language model run inside the client **(your call)**
- [ ] `OTH-019` Help assistant: crafting texts as a source **(your call)**
- [ ] `OTH-020` Linux and macOS builds exist **(your call)**
- [ ] `OTH-021` Screenshot key saves a PNG with a notes file **(your call)**

## Movement

### Movement: the client knowingly does otherwise (3)

#### `MOV-001` Own flight also read from worn items and running programs

- OWN-RULE, medium
- Original: switches to fly mode (7) when the server's spell list runs spell 0xcf92 (FUN_100a8107) and back when that list ends
- Client: also treats a running program or a worn item with the fly spell as flying, since no capture shows a list for a worn item and it was not seen whether the server sends one at login or zoning; not tried live
- A player notices: a worn jetpack or Yalmaha may start flight where the original would wait for the server's list
- Where: `crates/mao-client/src/net.rs:341`; `crates/mao-client/src/net.rs:8688`; `docs/CLIENT_UI.md:3576`
- Decision: [ ] port  [ ] leave

#### `MOV-002` Sliding along a room's edge when a step leaves the rooms

- OWN-RULE, low
- Original: not stated (n3RoomSurface_t::VetoPosition refuses the place)
- Client: if still out of the rooms after stepping back, tries moving along either axis, else not at all
- A player notices: a character slides along an interior wall instead of stopping dead
- Where: `crates/mao-client/src/vehicle.rs:341`
- Decision: [ ] port  [ ] leave

#### `MOV-003` Yalmaha scale and run speed restored by the client

- OWN-RULE, low
- Original: reads RunSpeed in mode 3 (stored value plus modifier map plus attribute trickle, FUN_1006f2fc); what it keeps for a character that logs in on its craft is unknown
- Client: a worn Yalmaha sets Scale 20 and adds 300 RunSpeed as temporary stats; the client saves the old scale and gives it back when taken off, also when the server's list said 20 in between; with none saved (logging in on the craft) the character stays small after leaving it until the server says otherwise
- A player notices: size and flying speed come back by client bookkeeping; wrong size possible after login on a Yalmaha
- Where: `crates/mao-client/src/net.rs:12340`; `crates/mao-client/src/net.rs:1451`; `docs/CLIENT_UI.md:3990`
- Decision: [ ] port  [ ] leave

### Movement: an approximation or a stand-in (9)

#### `MOV-004` Own walking and collision where a playfield has no surfaces (and with noclip)

- SIMPLIFIED, medium
- Original: Vehicle_t::Run and EnsureSurfaceAlignment against n3TilemapSurface_t, in every playfield; collision from surfaces
- Client: in a generated dungeon, room playfields without surfaces (missions) and with noclip, the client's own steps are used: the props' visible meshes or triangles, wall sliding along the walls (a single axis tried after), fixed jump speed 7 and gravity 10, floor probing; MAO_VISUAL_COLLISION forces them
- A player notices: walking, jumping and collision feel and behave differently in such playfields
- Where: `crates/mao-client/src/net.rs:8771`; `crates/mao-client/src/net.rs:6661-6662`; `crates/mao-client/src/net.rs:8845-8870`; `docs/CLIENT_UI.md:2872`; `docs/CLIENT_UI.md:2875`; `docs/CLIENT_UI.md:2923`
- Decision: [ ] port  [ ] leave

#### `MOV-005` Walk or run of other characters told by speed

- SIMPLIFIED, medium
- Original: walk against run is the movement mode (WALK_MODE 0x18 / RUN_MODE 0x19), chosen in FUN_1006c370 over table FUN_100c0720
- Client: for other characters (and morphs) a measured speed over 2.2 is a run, else a walk (walking 1.5, running from 3 backwards or 5)
- A player notices: another's walk/run animation may be wrong at in-between speeds
- Where: `crates/mao-client/src/net.rs:5372-5377`; `crates/mao-client/src/net.rs:3748`; `docs/CLIENT_UI.md:2124`; `docs/ORIGINAL_AUDIT.md:57`
- Decision: [ ] port  [ ] leave

#### `MOV-006` Doors checked on the character's own steps

- SIMPLIFIED, low
- Original: asks IsDynelRoomTransitionAllowed as the dynel's room-space cell changes
- Client: asks it of each of the character's own steps and holds the character in its room if refused
- A player notices: a step across a closed doorway is undone; may feel different at the doorway
- Where: `crates/mao-client/src/interior.rs:8`
- Decision: [ ] port  [ ] leave

#### `MOV-007` Flight pitch: body drawn level, no camera tilt

- SIMPLIFIED, low (notes only)
- Original: not stated (pitch by N3.dll VehicleForwardUpdate)
- Client: the body is drawn level and the camera does not tilt while the button pitches (the pitch limit now matches the original; the sensitivity is under the mouse numbers entry)
- A player notices: a drag shows nothing until the character moves forward
- Where: `docs/CLIENT_UI.md:3588`
- Decision: [ ] port  [ ] leave

#### `MOV-008` Jump animation stretched over a fixed air time

- SIMPLIFIED, low
- Original: not stated
- Client: a jump is 1400 ms in the air and the landing plays 350 ms as constants; the jump animation plays over the fixed 1400 ms whatever the real jump's height and length
- A player notices: jump animation timing may not match the real jump
- Where: `crates/mao-client/src/net.rs:4161`; `crates/mao-client/src/net.rs:4995`
- Decision: [ ] port  [ ] leave

#### `MOV-009` Others count as moving for a fixed 700 ms

- SIMPLIFIED, low
- Original: not stated
- Client: a character is shown moving if its place changed in the last 700 ms (creatures 900 ms)
- A player notices: walk animation may linger or stop early
- Where: `crates/mao-client/src/net.rs:5372`
- Decision: [ ] port  [ ] leave

#### `MOV-010` Sneak animation played forwards when moving backwards

- SIMPLIFIED, low
- Original: plays the sneak action reversed when moving backwards
- Client: plays it forwards
- A player notices: backwards sneaking looks wrong
- Where: `docs/CAPTURE_GAPS.md:273`; `crates/mao-client/src/net.rs:4222`; `crates/mao-client/src/net.rs:12286`
- Decision: [ ] port  [ ] leave

#### `MOV-011` Tile-map ground hit test samples along the line

- SIMPLIFIED, low
- Original: n3TilemapSurface_t::GetLineIntersection walks the tiles the segment crosses (Intersect_Tile)
- Client: samples the segment every half tile and tests each sampled tile and its four neighbours
- A player notices: none visible expected
- Where: `crates/mao-client/src/vehicle.rs:136`
- Decision: [ ] port  [ ] leave

#### `MOV-012` Turn animations fall back as if stat 662 were never set

- SIMPLIFIED, low
- Original: Gamecode FUN_10010b3c: the turns 0xc4 and 0xc5 fall to the side steps for a client character with stat 662 set
- Client: the stat is taken as not set
- A player notices: a character with that stat set would not use the sidestep for turning
- Where: `crates/mao-formats/src/appearance.rs:184`
- Decision: [ ] port  [ ] leave

### Movement: the original was not read: inferred or measured (4)

#### `MOV-013` Flight: leaving fly mode resets body and pitch at once

- GUESS, low
- Original: moves the body up 0.1, lets it fall, resets the orientation mode; whether the pitch is zeroed at once was not found
- Client: resets its body and zeroes the pitch
- A player notices: the landing may look different
- Where: `docs/FLIGHT_ORIGINAL.md:251`; `crates/mao-client/src/net.rs:8735-8745`; `crates/mao-client/src/viewer.rs:1765`
- Decision: [ ] port  [ ] leave

#### `MOV-014` Seated character cannot move or jump

- GUESS, low
- Original: what a jump does while seated was not read (only the sit toggle's LeaveSit ends sit mode; turns are taken in any mode)
- Client: forward, back, sidesteps and jump do nothing while seated; the sit key is the only way up (taken from the user's word)
- A player notices: a seated character cannot walk or jump
- Where: `docs/CLIENT_UI.md:1530`; `crates/mao-client/src/net.rs:8650`; `crates/mao-client/src/net.rs:4355`; `docs/ORIGINAL_AUDIT.md:27`
- Decision: [ ] port  [ ] leave

#### `MOV-015` Speed of other characters smoothed from position reports

- GUESS, low
- Original: not stated
- Client: other characters' speed (for their animation) is 0.6 old plus 0.4 new, measured from distance over time between reports
- A player notices: walk/run animation of others follows measured speed and may lag or flicker
- Where: `crates/mao-client/src/net.rs:3701`
- Decision: [ ] port  [ ] leave

#### `MOV-016` Turn speed without the TurnSpeed stat

- GUESS, low
- Original: FUN_1006ca4f: TurnSpeed (stat 267) / 11000, halved while moving
- Client: with no such stat uses 3.5 standing and 1.5 moving
- A player notices: turning speed differs when the stat is missing
- Where: `crates/mao-client/src/net.rs:4360`
- Decision: [ ] port  [ ] leave

### Movement: a guard or fallback the original lacks (3)

#### `MOV-017` Client goes back to a place the server reports

- SAFETY, medium
- Original: not stated (a server place is put on the dynel)
- Client: a full report more than 5 m from us that is none of the places we told it in the last 4 s makes the client go back there (`own_report`)
- A player notices: the client snaps back when the server refuses a move
- Where: `docs/CLIENT_UI.md:3274`; `crates/mao-client/src/net.rs:3682`; `crates/mao-client/src/net.rs:7041`
- Decision: [ ] port  [ ] leave

#### `MOV-018` Stun flags default when stat 224 is missing

- SAFETY, low
- Original: FUN_1004496c starts the counts from stat 224
- Client: if the server did not send it, own flags start as 6 (free to move)
- A player notices: stuns work from a default instead of the server's value
- Where: `crates/mao-client/src/net.rs:1216`
- Decision: [ ] port  [ ] leave

#### `MOV-019` Take-off waits for the server to take the flying item

- SAFETY, low
- Original: flies when the server's spell list says so, which comes after the move
- Client: flight from worn items goes by where each item was before the moves still asked, so a take-off waits for the server's answer (about a tenth of a second); a move never answered leaves the item counted where it was
- A player notices: a short delay before take-off or landing after putting on or taking off a flying item
- Where: `crates/mao-client/src/net.rs:4260`; `docs/CLIENT_UI.md:4178`; `CHANGELOG.md:91`
- Decision: [ ] port  [ ] leave

### Movement: the original does it, the client does not (7)

#### `MOV-020` Other characters' flight and crafts known only from spell lists

- MISSING, high
- Original: another player's craft comes from its update (template and scale there for a player); MonsterData has its own handling in the temporary set; flight comes from the server's spell list
- Client: another character flies only when a spell list of the server's said so (cleared at every zoning, not seen whether the server runs it again); the client is not told what others wear, so another player's Yalmaha or craft is not known; the Yalmaha size (template 99470 at 20 %) was not checked against a picture
- A player notices: others on hoverbikes or crafts may be drawn walking, or not as a craft, until or unless a list arrives
- Where: `crates/mao-client/src/net.rs:347`; `crates/mao-client/src/net.rs:283-285`; `crates/mao-client/src/net.rs:7130`; `docs/CLIENT_UI.md:3594`; `docs/CLIENT_UI.md:3992`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): another player's craft, polymorph body and flight are taken from its full update (`MonsterData`, `Scale`, the movement mode in the block behind them: 7 flies, 8 sits), and a craft or flight item put on or taken off in view from the server's `TemplateAction` 6 / 7, whose wear list the client runs as the original does (`World::others_wear`). The line above is wrong in part: flight comes from a server's spell list only for a program with criteria on one's own character. The mount rests on one capture, the dismount on the code alone. Not ported: a hoverbike program of another player started in view; and found beside it: `Buff` is a program's removal, which the client takes for its running (see CLIENT_UI.md).

#### `MOV-021` Flight: entering effect and the ways flight ends missing

- MISSING, medium
- Original: entering fly mode makes effect 2000 (colour 0xfff0e0ff); flight ends by character action 0xac, a kind-5 area, state flag 0x2000000, or the list ending
- Client: the hover animation (0xb3), fly speeds (7 + RunSpeed/275), 0xcf92 and the Yalmaha craft are now ported; no entering effect and only the list's ending (and the client's own reading) end flight
- A player notices: no flash on take-off; flight may not end where the original ends it
- Where: `docs/FLIGHT_ORIGINAL.md:227`; `docs/CLIENT_UI.md:3596`; `crates/mao-client/src/net.rs:8688-8745`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): CharacterAction 0xac ends a flight a spell list began; one's own flight from a worn item ending by it, the other ways out and the entering effect 2000 are not done.

#### `MOV-022` Flight: sit, sneak and crawl are not refused

- MISSING, low
- Original: in fly mode refuses sit, sneak and crawl and disables the walk/run toggle (FUN_1006c6f4 case 7)
- Client: the sit key and the sneak command check no flight (sneak only refuses a vehicle)
- A player notices: a flying character may be able to sit or sneak
- Where: `docs/FLIGHT_ORIGINAL.md:256`; `crates/mao-client/src/net.rs:7765`; `crates/mao-client/src/net.rs:8184`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): the sit key, the walk/run key and the Sit/Stand, Walk/Run, Sneak and Crawl tiles say Feedback_ActionIsNotAvailable in fly mode and do nothing (actions::fly_refuses: the eight actions FUN_1006e4f8 marks); /camp says Feedback_YouMustBeSitting.

#### `MOV-023` Fly spell does not make a seated character get up first

- MISSING, low
- Original: 0xcf92 (FUN_100a8107) first makes a crawling, sitting, sleeping or lounging character get up (actions 0x28, 0x29, 0x2a, 0x25)
- Client: not ported
- A player notices: a seated or crawling character switched to fly mode may not stand first
- Where: `crates/mao-client/src/spelllist.rs:36`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): a character a fly spell switches gets up first; one's own at each switch of its fly mode. Nothing is told the server, as in the original. Crawling is not kept by this client.

#### `MOV-024` Mouse steering: turn limit missing and small message differences

- MISSING, low
- Original: limits the mouse's turn while stat 224's bits 2 or 4 are set and stat 267 is there; counts a movement straight up or down as one to the right; stops the key turn first and starts the strafe; sends both strafe stops where a converted key was held
- Client: no limit; the turn alone decides the side; the keys are counted as strafe keys and only the running strafe stop is sent (mouse turns 10 and 13 are ported)
- A player notices: mouse-steer turning is not limited by stats; very small differences in messages
- Where: `docs/CLIENT_UI.md:4295`; `docs/CLIENT_UI.md:4293`; `docs/ORIGINAL_AUDIT.md:76`; `crates/mao-client/src/net.rs:4272-4286`
- Decision: [ ] port  [ ] leave

#### `MOV-025` Operators "is flying" (70) and "flying allowed" (112) missing

- MISSING, low
- Original: operator 70 is "is flying" (mode 7); operator 112 fails when the playfield forbids flying; the jetpack's wear requirement uses both
- Client: neither is in requirements.rs, so they fall to "unknown"
- A player notices: a flight item's wear check cannot be fully evaluated
- Where: `docs/FLIGHT_ORIGINAL.md:253`; `crates/mao-client/src/requirements.rs:160-167`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): operators 70 and 112 are judged (requirements.rs; the playfield's flag from playfield::flag_words). A jetpack is now refused by the client where flying is not allowed (Arete Landing among them), with the game's line.

#### `MOV-026` Room surface transforms and vehicle turn not ported

- MISSING, low (notes only)
- Original: the room surface's own transforms, the vehicle's turn (orientation, FUN_1000c616); VetoPosition's room transitions are now handled (vehicle.rs)
- Client: room surface transforms and the vehicle's turn not ported
- A player notices: room-edge and turning behaviour may differ
- Where: `docs/CLIENT_UI.md:2933`; `crates/mao-client/src/vehicle.rs:328`
- Decision: [ ] port  [ ] leave

### Movement: the client has it, the original does not (7)

#### `MOV-027` /follow is the client's own walk-after-target

- ADDITION, medium
- Original: not stated (the original has no /follow in this form)
- Client: walks after the target to 3.5 m (2.0 m if fighting it, since the server answered "too far" at 3.5), restarts a metre past that, stops on any movement key, and says "Following X" / "Stopped following" / "The target is gone: stopped following"
- A player notices: automatic following with the client's own distances and chat lines
- Where: `crates/mao-client/src/net.rs:7616-7655`; `docs/CLIENT_UI.md:2690`; `docs/CLIENT_UI.md:2851`
- Decision: [ ] keep  [ ] remove

#### `MOV-028` Walk-to-place with its own obstacle detours

- ADDITION, medium
- Original: not stated
- Client: Whom-Pah Go button and /waypoint walk straight toward a point, turn aside for 1.6 s when no progress for 0.9 s (six angles), give up after 12 detours with "Can't find a way round (stopped)"; stop within 1.2 m
- A player notices: a click-to-walk feature with the client's own path-finding
- Where: `crates/mao-client/src/net.rs:7687-7698`
- Decision: [ ] keep  [ ] remove

#### `MOV-029` /sit command is the client's own

- ADDITION, low
- Original: the original's command list has no /sit command
- Client: /sit and X sit the character on the ground or stand it up and no longer log out; /camp while seated logs out without sitting again
- A player notices: an extra sit command; X does not log out
- Where: `docs/CLIENT_UI.md:2739`; `crates/mao-client/src/chat_commands.rs:60`; `crates/mao-client/src/chat_commands.rs:250`; `crates/mao-client/src/net.rs:7765`
- Decision: [ ] keep  [ ] remove

#### `MOV-030` Auto-run key

- ADDITION, low
- Original: not stated
- Client: an auto-run key keeps the character walking forward until it is pressed again or a walk key is used
- A player notices: an extra key
- Where: `crates/mao-client/src/viewer.rs:1357`; `crates/mao-client/src/settings.rs:240`; `docs/CLIENT_UI.md:1858`
- Decision: [ ] keep  [ ] remove

#### `MOV-031` Own character glides between server updates

- ADDITION, low
- Original: not stated
- Client: between updates the character, its name tag, shadow and the camera are moved on at the speed it was walking, up to 0.12 s ahead, using the step's own time
- A player notices: smoother motion; the picture can run slightly ahead of the server's position
- Where: `crates/mao-client/src/viewer.rs:1962-1970`; `docs/CLIENT_UI.md:1288`
- Decision: [ ] keep  [ ] remove

#### `MOV-032` Walk-through-props toggle (noclip) and key layout

- ADDITION, low
- Original: not stated (the original's own key for C is sidestep)
- Client: a toggle lets the character walk through props (unbound by default now; was C, then Ctrl+Shift+C); strafing is on Z/C
- A player notices: the player can walk through props, which the original does not offer
- Where: `crates/mao-client/src/viewer.rs:29`; `crates/mao-client/src/viewer.rs:1370`; `crates/mao-client/src/settings.rs:195-197`; `docs/CLIENT_UI.md:95`; `README.md:49`
- Decision: [ ] keep  [ ] remove

#### `MOV-033` Walk/run mode announced on entering each playfield

- ADDITION, low
- Original: the server remembers the movement mode between sessions
- Client: sends mode 0x19/0x18 on entering every playfield so the server does not pull back a running character
- A player notices: none visible
- Where: `docs/CLIENT_UI.md:2862`; `crates/mao-client/src/net.rs:7201`; `crates/mao-client/src/net.rs:8666`
- Decision: [ ] keep  [ ] remove

## Combat

### Combat: the client knowingly does otherwise (2)

#### `CMB-001` Damage meter fights split by a 12-second gap

- OWN-RULE, low
- Original: not stated (the original has no meter)
- Client: a fight ends after 12 s with no hit; per-second over first to last event (at least 1 s); 40 fights kept; a heal alone starts none
- A player notices: the meter's fight boundaries and rates follow the client's rule
- Where: `crates/mao-client/src/damage.rs:11`
- Decision: [ ] port  [ ] leave

#### `CMB-002` Health loss told only beyond recent hits

- OWN-RULE, low
- Original: the original's own rule was not read
- Client: a health loss is told only as far as it exceeds hits told in the last two seconds (World::recent_hits, untold_loss)
- A player notices: damage lines may differ from the original's
- Where: `crates/mao-client/src/net.rs:1166`; `crates/mao-client/src/net.rs:210`; `docs/CLIENT_UI.md:525`
- Decision: [ ] port  [ ] leave

### Combat: an approximation or a stand-in (12)

#### `CMB-003` Attack key checks only the first refusals

- SIMPLIFIED, medium
- Original: FUN_10069a9f checks the selection with more refusals
- Client: checks oneself, the dead, not a character, swimming and a pending attack only
- A player notices: other cases the original refuses are sent to the server
- Where: `crates/mao-client/src/combat.rs:82`
- Decision: [ ] port  [ ] leave

#### `CMB-004` Health drops in Stat messages become "unknown damage" lines

- SIMPLIFIED, medium
- Original: not stated
- Client: a drop in another's health stat with no matching hit is told as "was attacked with nanobots for N points of unknown damage"; a hit just told suppresses the duplicate
- A player notices: extra chat lines with generic wording
- Where: `crates/mao-client/src/net.rs:12191`; `crates/mao-client/src/net.rs:3514`
- Decision: [ ] port  [ ] leave

#### `CMB-005` Start Combat action attacks the target directly

- SIMPLIFIED, medium
- Original: the attack key goes through N3Msg_DefaultAttack (not stated for this action)
- Client: sends an attack at the target with the client's own "Attacking X" / "No target (Tab selects one)" lines
- A player notices: the action does not apply the attack key's rules (stop a fight, refusals)
- Where: `crates/mao-client/src/net.rs:8195`
- Decision: [ ] port  [ ] leave

#### `CMB-006` Attack swing variant picked by a stand-in for rand()

- SIMPLIFIED, low
- Original: FUN_10045976 and FUN_1006a782 pick one of the weapon's attack actions at random each time
- Client: a variant is picked per character (`pick` stands for rand())
- A player notices: less variety of swings
- Where: `crates/mao-client/src/net.rs:4888`; `crates/mao-formats/src/item.rs:327`; `docs/CLIENT_UI.md:2392`
- Decision: [ ] port  [ ] leave

#### `CMB-007` Damage floaters fixed 2 m above the target

- SIMPLIFIED, low
- Original: not stated
- Client: numbers appear 2.0 m above the feet, not by body height; zero amounts skipped
- A player notices: numbers may sit inside tall creatures or too high over small ones
- Where: `crates/mao-client/src/net.rs:9748`
- Decision: [ ] port  [ ] leave

#### `CMB-008` Damage meter leaves out what has no source

- SIMPLIFIED, low
- Original: not stated
- Client: a loss of health told only as a new health value, and a fall, are not counted; the owner of another's pet is unknown; nothing kept between runs, no log file
- A player notices: some damage never appears in the meter
- Where: `crates/mao-client/src/damage.rs:8`; `docs/CLIENT_UI.md:982`
- Decision: [ ] port  [ ] leave

#### `CMB-009` Hit as a creature dies is told with the remains' name

- SIMPLIFIED, low (notes only)
- Original: not stated
- Client: the name is the corpse's by then
- A player notices: wrong name in the line
- Where: `docs/CLIENT_UI.md:736`
- Decision: [ ] port  [ ] leave

#### `CMB-010` Own turn in a fight not sent by itself

- SIMPLIFIED, low (notes only)
- Original: CharTurn_t state machine
- Client: own turn is not sent to the server; the next movement message carries the heading
- A player notices: server sees the heading later
- Where: `docs/ORIGINAL_AUDIT.md:85`
- Decision: [ ] port  [ ] leave

#### `CMB-011` Perk use relies on the server's remembered target

- SIMPLIFIED, low
- Original: not stated
- Client: the perk message names the character itself as target and relies on the target the server knows from set_target
- A player notices: none visible
- Where: `crates/mao-net/src/n3.rs:683`
- Decision: [ ] port  [ ] leave

#### `CMB-012` Set-stat spell kept for one's own character only

- SIMPLIFIED, low
- Original: 0xcf22 (FUN_100a4f8a) sets stat 0 to stat 39 on the dynel
- Client: kept for the client's own character only (no stats of others)
- A player notices: other characters' and creatures' stats set by such spells are not tracked
- Where: `crates/mao-client/src/spelllist.rs:33`
- Decision: [ ] port  [ ] leave

#### `CMB-013` Tracer1 frame choice, caller's speed and streak life

- SIMPLIFIED, low (notes only)
- Original: FindPerpendicular's frame; the caller's speed; streaks have a one-second life
- Client: takes the frame's y nearest up; fixed flight; streaks longer than the flight
- A player notices: shot tracers differ
- Where: `docs/ORIGINAL_AUDIT.md:295`
- Decision: [ ] port  [ ] leave

#### `CMB-014` Tracer5 spread plane and hit timing

- SIMPLIFIED, low (notes only)
- Original: Cord4's spread in the view's screen plane; the hit's timing
- Client: spreads across the line and the line of sight; fixed flight
- A player notices: tracer ribbon and timing differ
- Where: `docs/ORIGINAL_AUDIT.md:297`
- Decision: [ ] port  [ ] leave

### Combat: the original was not read: inferred or measured (7)

#### `CMB-015` Special-attack list built from worn weapons

- GUESS, high
- Original: not stated (the server lists the specials); the source of a gun's attacks in the original's window was not traced
- Client: adds each wielded weapon's attacks and Reload to the server's list from item templates, drops Reload when the server says no ammunition, adds Backstab only after a server flag
- A player notices: the specials bar can differ from the original's
- Where: `crates/mao-client/src/net.rs:11380`; `crates/mao-client/src/net.rs:1121`; `docs/LIVE_TEST_LIST.md:758`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): not a guess any more. The original builds the list the same way: the server's `SpecialAttackWeapon` adds its three, and `WeaponItem_t` (Gamecode FUN_1009e84b) adds each wielded weapon's attacks by its stat 30 bits, Reload when stat 26 is not -1 and Backstab by the server's flag. Two small differences were made the original's: the server's list is added to and not put in the old one's place, and a weapon's rounds are taken from an attack only while the stored number is above -1. `CLIENT_UI.md` "The special attacks' list is the original's".

#### `CMB-016` Creature with weapon points but no weapon swings the unarmed swing

- GUESS, medium
- Original: a body with weapon attachment points swings its natural weapon's own attack (FUN_1006a014), an item made by FUN_1006b14c
- Client: does not make those items; plays unarmed-rswing (0x40a) in its place, a guess; where the list of natural weapons comes from not read
- A player notices: humanoid NPCs with natural weapons swing the generic unarmed attack
- Where: `crates/mao-client/src/net.rs:4515`; `docs/CLIENT_UI.md:4801`; `crates/mao-client/src/net.rs:4514`; `docs/LIVE_TEST_LIST.md:839`
- Decision: [ ] port  [ ] leave

#### `CMB-017` Backstab availability from the decompile only

- GUESS, low
- Original: server's CharacterAction 0xb5 sets bit 20 of the second flag word; weapons add Backstab beside sneak attack (FUN_1005d61f case 0x44, FUN_1009c5c8, FUN_10068976)
- Client: done from the decompile only (action 489); no capture has the message; which characters are sent it was not seen
- A player notices: Backstab shows by an unconfirmed rule
- Where: `crates/mao-client/src/net.rs:2581`; `docs/CLIENT_UI.md:504`; `docs/ORIGINAL_AUDIT.md:100`
- Decision: [ ] port  [ ] leave

#### `CMB-018` Duel and pet duel texts use the client's own words

- GUESS, low
- Original: duel and pet duel texts come from text.mdb category 110 by name (Gamecode FUN_1005ca5b)
- Client: English fallback lines for duel and pet duel are the client's own wording (Feedback_Duel* names); `/duel` with no target prints a line in the client's words
- A player notices: wording of duel messages may differ from the original's
- Where: `crates/mao-client/src/feedback.rs:80`; `crates/mao-client/src/feedback.rs:81`; `docs/CLIENT_UI.md:520`; `crates/mao-client/src/net.rs:8316`
- Decision: [ ] port  [ ] leave

#### `CMB-019` Full Auto uses a stat taken from public tables

- GUESS, low
- Original: not stated
- Client: Full Auto uses stat 167 (weapon-can bit 0x2000), which is the skill's stat in public tables but was not in a capture
- A player notices: Full Auto may not work as in the original
- Where: `docs/CLIENT_UI.md:1808`; `crates/mao-client/src/actions.rs:104`
- Decision: [ ] port  [ ] leave

#### `CMB-020` Special attacks by others use an assumed text layout

- GUESS, low
- Original: for others the original has a text of its own with four things fed
- Client: assumes they are the attacker, the victim, the amount and the skill
- A player notices: the line for another's special attack may read differently
- Where: `crates/mao-client/src/feedback.rs:243`
- Decision: [ ] port  [ ] leave

#### `CMB-021` Weapon's attack animation set default for an item with no AnimSet stat

- GUESS, low
- Original: FUN_1009d964 reads AnimSet stat 353 with its own default
- Client: an item without the stat is taken as unset (default not traced); sets other than 0,1,2,3,6,7,8 keep the template's own swings
- A player notices: weapons without the stat may swing the wrong animation
- Where: `crates/mao-formats/src/item.rs:332`
- Decision: [ ] port  [ ] leave

### Combat: the original does it, the client does not (15)

#### `CMB-022` Special attacks are not checked before sending (weapon bit, range, line of sight, queue)

- MISSING, high
- Original: N3Msg_SecondarySpecialAttack (0x100281fc) and FUN_100427d6/FUN_10042fc9 refuse some specials before sending: the held weapon's can bit (Burst ... Brawl 0x2000000, Dimach 0x4000000), Feedback_TargetIsOutsideSpecialAttackRange, Feedback_NoLineOfSight, a queue of unswung specials
- Client: only not-in-list, skill lock, no target/self/dead/swimming are checked; the weapon bit, range, line of sight and the queue are left to the server
- A player notices: Brawl and Dimach go out with any weapon; no client line for range, line of sight or queue
- Where: `crates/mao-client/src/net.rs:3821`; `crates/mao-client/src/net.rs:3822`; `docs/CLIENT_UI.md:3823`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18): the range is checked as the original checks it: each weapon in a hand with the bit, then the special's own item as the server lists it (Brawl's 3 m, Dimach's 10 m), the reach its stat 287 with the stat 380 share, cut and at most 40, against the 3D distance less 1 m a body (`combat::special_reaches`). Left: the line of sight (two flags not established, the client's collision not the original's). The 'queue' was not found: that check (FUN_1002a027) is of something the reload sets and refuses silently.

#### `CMB-023` Action tiles do not follow state; some tiles, CharacterAction 0xb2/0x14/0xfc, icon time text, special at attackable item undone

- MISSING, medium (notes only)
- Original: six state-dependent tiles (Sit/Stand, Walk/Run, Sneak/Stop, Start/End Combat, Suspended Animation/Disrupt, Crawl/stop); tiles Use, Pick-Up, Walk, Suspended Animation, Crawl; CharacterAction 0xb2, 0x14, 0xfc; icon time text; special at an attackable item (the weapon-bit, range and queue checks are the entry above)
- Client: none done
- A player notices: tiles do not change with state; some tiles do nothing
- Where: `docs/CLIENT_UI.md:3823`
- Decision: [ ] port  [ ] leave

#### `CMB-024` Action places not kept, toggles keep one name, profession action missing

- MISSING, low (notes only)
- Original: SpecialActionView_c keeps the actions' places (item_position_map), toggles have other names (sit for stand, run for walk), and the profession's own action (138, 139, FUN_1004326f)
- Client: not done; toggles keep one name
- A player notices: action window lacks these
- Where: `docs/CLIENT_UI.md:507`; `docs/GUI_DLL_PARITY.md:83`
- Decision: [ ] port  [ ] leave

#### `CMB-025` Actions with no known message only say so

- MISSING, low
- Original: not stated
- Client: an action with no known network message shows "<action>: its network message is not captured yet"
- A player notices: some actions in the action window do nothing
- Where: `crates/mao-client/src/net.rs:8214`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): the same port as WIN-112: only Pick-Up and Crawl still say so.

#### `CMB-026` Alien-XP level-cap adjustment not done

- MISSING, low (notes only)
- Original: stat 169 alien-XP level-cap adjustment
- Client: not done
- A player notices: alien XP lines differ
- Where: `docs/ORIGINAL_AUDIT.md:89`
- Decision: [ ] port  [ ] leave

#### `CMB-027` Attack swing ignores the target's height

- MISSING, low
- Original: FUN_1006a782's param_3 (target height), which FUN_1003cac9 does not pass on
- Client: not followed
- A player notices: none stated
- Where: `crates/mao-client/src/net.rs:4892`
- Decision: [ ] port  [ ] leave

#### `CMB-028` Character's own animation set, draw/holster, wield and attractor smoothing not done

- MISSING, low
- Original: type 0xfde97 picks among an action's _01_0n variants; the draw's start and holster's stop; wield and op1h/op2h; camera attractor smoothing
- Client: left
- A player notices: fewer animation variants
- Where: `crates/mao-client/src/net.rs:4892`; `docs/ORIGINAL_AUDIT.md:250`
- Decision: [ ] port  [ ] leave

#### `CMB-029` Crawling variants of attack swings are not used

- MISSING, low
- Original: crawling variants are used when the owner's stat 430 is 14
- Client: not followed
- A player notices: a crawling character swings the standing animations
- Where: `crates/mao-formats/src/item.rs:342`; `crates/mao-client/src/net.rs:4893`; `docs/CLIENT_UI.md:2392`
- Decision: [ ] port  [ ] leave

#### `CMB-030` Equip timer, restarted attack entry place, items-after-timer not ported

- MISSING, low
- Original: Gamecode timer list: kind 6 (equip, FUN_100482c7); a restarted attack entry is re-added in a new place; items used only after the timer
- Client: not ported (timers.rs has kinds 1 to 5)
- A player notices: no equip timer bar; restarted attack bars keep their place
- Where: `crates/mao-client/src/timers.rs:6`; `crates/mao-client/src/timers.rs:34`; `docs/GUI_DLL_PARITY.md:62`
- Decision: [ ] port  [ ] leave

#### `CMB-031` Four special-attack keys start unbound

- MISSING, low
- Original: GUI.dll 0x1006951c binds ActionSneakAttack, ActionFastAttack, ActionAimedShot, ActionBowSpecialAttack; the original's keys were not read
- Client: those four actions can be bound but start without a key
- A player notices: those four special attacks have no default key
- Where: `crates/mao-client/src/settings.rs:177`; `docs/CLIENT_UI.md:3821`
- Decision: [ ] port  [ ] leave

#### `CMB-032` Set-stat spell's selection and profession cases not ported

- MISSING, low
- Original: on a character stat 0x2f goes to the selection (FUN_1003e464) and 0x3c to FUN_10056552
- Client: those stats (and 0x25, 4, 0x3b, 0x16d, 0x1b, 0xd6) are not set
- A player notices: selection and profession changes from such a spell do not happen
- Where: `crates/mao-client/src/spelllist.rs:227`
- Decision: [ ] port  [ ] leave

#### `CMB-033` Sounds: special attack fields, item hits and "Sneak Attack!" texts not followed

- MISSING, low (notes only)
- Original: special attack's own fields (FUN_1006af0e), hits on items (FUN_1009b4b0), "Sneak Attack!" texts
- Client: not followed
- A player notices: no special-attack sounds/texts
- Where: `docs/CLIENT_UI.md:2450`
- Decision: [ ] port  [ ] leave

#### `CMB-034` Stun does not touch the vehicle

- MISSING, low
- Original: Stun (0xcf82, FUN_100a879c) also calls the vehicle's +0x9c/+0x98 with FUN_1005a02c(1)
- Client: only the state flags (stat 224) change
- A player notices: not stated beyond the stunned character's movement state
- Where: `crates/mao-client/src/spelllist.rs:30`
- Decision: [ ] port  [ ] leave

#### `CMB-035` Stun on a non-character gives no "select a target" line

- MISSING, low
- Original: on anything but a character the caster, when it is the client's, is told Feedback_YouNeedToSelectATargetForProgram (FUN_100a879c)
- Client: not ported; nothing is shown
- A player notices: no explanation when casting the program on a non-character
- Where: `crates/mao-client/src/spelllist.rs:223`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): no path was found on which the line can be said (a target that is neither a character nor a world item is an inventory entry, and such spells carry no caster): nothing to port.

#### `CMB-036` Weapon draw: busy count and idle-stand once not done

- MISSING, low (notes only)
- Original: a weapon without a start plays idle-stand once; the controller's busy count (+0x3c) holds the draw back
- Client: not done
- A player notices: weapon draw timing may differ
- Where: `docs/CLIENT_UI.md:2398`
- Decision: [ ] port  [ ] leave

### Combat: the client has it, the original does not (2)

#### `CMB-037` Damage meter window (client's own)

- ADDITION, medium
- Original: the original has no such window; players used programs reading the chat log
- Client: /damage, /dps window counting damage done, taken and healing by everyone near, split into fights, per attacker, with copy as text
- A player notices: a damage meter
- Where: `crates/mao-client/src/damage.rs:1`; `docs/CLIENT_UI.md:971`; `docs/LIVE_TEST_LIST.md:596`; `crates/mao-client/src/net.rs:1402`
- Decision: [ ] keep  [ ] remove

#### `CMB-038` Floating combat numbers

- ADDITION, medium
- Original: not stated
- Client: damage dealt, taken and healing rise 38 px a second over the target for 1.6 s, yellow, red or green, with a shadow
- A player notices: combat numbers over characters
- Where: `crates/mao-client/src/ui.rs:2179`; `docs/CLIENT_UI.md:1844`
- Decision: [ ] keep  [ ] remove

## Targeting and mouse

### Targeting and mouse: the client knowingly does otherwise (2)

#### `TGT-001` The character itself is last on a click line

- OWN-RULE, high
- Original: the camera keeps everything pickable on the line from the eye, nearest first (n3Camera_t +0x244); the source does not say the character is sorted last
- Client: the player's own character is put last on the line, behind whatever else the line meets (camera looks over its shoulder); it was left out before 2026-11-17
- A player notices: clicking at the character with something behind it takes the thing behind first; the character is taken only when what stands behind is already selected
- Where: `crates/mao-client/src/viewer.rs:1005`; `docs/CLIENT_UI.md:4850`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): one's own character is on the click line by its distance like anything else, and left out only in the first person (N3.dll FUN_10020a3c). The client's rule of putting it last is gone. `CLIENT_UI.md` "What the mouse points at".

#### `TGT-002` Mouse steering counts up/down movement differently

- OWN-RULE, low
- Original: N3Msg_MouseMovement (0x100196b3) counts a movement straight up or down as one to the right
- Client: goes by the turn alone
- A player notices: moving the mouse straight up or down does not start a right turn
- Where: `crates/mao-client/src/net.rs:4276`
- Decision: [ ] port  [ ] leave

### Targeting and mouse: an approximation or a stand-in (8)

#### `TGT-003` Mouse picking uses cylinders, not the dynels' collision

- SIMPLIFIED, high
- Original: casts the camera's line through the mouse against the dynels' collision (n3Camera_t::SetMousePos / GetObjectUnderColLine; the mouse's line against the dynel itself)
- Client: a cylinder per object standing on its feet (fixed size by kind, 1.0x2.6, 1.0x3.0, 0.8x2.2; characters from the posed points, not under 0.3/0.6, no wider than 3; built-in objects and a city controller sized from their mesh, half the greater width by the height); the player's own character is put last on the line
- A player notices: clicking near the edge of a model or through gaps may pick differently from the original
- Where: `crates/mao-client/src/net.rs:9790`; `crates/mao-client/src/main.rs:600`; `crates/mao-client/src/main.rs:2584`; `crates/mao-client/src/blobshadow.rs:55`; `crates/mao-client/src/viewer.rs:1275`; `docs/CLIENT_UI.md:4010` and 2 more
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the cylinders are gone: a character is met by the box of its posed vertices in the mesh's own axes (`BoundingBoxTargeting`, on by default), a built-in object by its mesh's triangles that face the line, and the line is cut at the first ground or surface (`picking.rs`; N3.dll `FineCollisionCheck`, `SetMousePos`). Not ported: the option's triangles for bodies, `IsVisible`, door leaves as they swing, and objects the server announces outside the playfield's table (they keep a post).

#### `TGT-004` Attack key: can-attack check is partial

- SIMPLIFIED, medium (notes only)
- Original: FUN_10069a9f check: swimming, suppression, surrender, PvP, 0x8000000 states
- Client: check only for oneself, the dead and non-characters; team/raid/pet friends and InPlay (194) not tracked
- A player notices: attacks permitted that the original refuses
- Where: `docs/ORIGINAL_AUDIT.md:86`
- Decision: [ ] port  [ ] leave

#### `TGT-005` Friendly means the same side only for targeting keys

- SIMPLIFIED, medium
- Original: a friend is the same side and also the team, the raid and their pets (FUN_10065d65 / FUN_10065dab / stat 196)
- Client: a friend is only the same side
- A player notices: team mates of another side and pets are treated as hostile by the targeting keys
- Where: `crates/mao-client/src/combat.rs:20`
- Decision: [ ] port  [ ] leave

#### `TGT-006` City map: only the left button; drag-scroll direction assumed

- SIMPLIFIED, low
- Original: BuildingMapView_c MouseDown never reads its button arguments: any button selects, drags a node or scrolls; ScrollBy's sign was not traced
- Client: only the primary button starts a selection, drag or scroll; scrolls by egui's drag delta (may be the opposite way)
- A player notices: right/middle presses on the map do nothing; scroll direction may be reversed
- Where: `crates/mao-client/src/cityterminal.rs:415`; `crates/mao-client/src/cityterminal.rs:444`
- Decision: [ ] port  [ ] leave

#### `TGT-007` Name tag height and picking size from the posed body, not the mesh at rest

- SIMPLIFIED, low
- Original: tag from VisualMesh_t::GetMeshHeight, the bounding sphere's diameter of the largest of the body's meshes at rest
- Client: taken from the posed body's bounding sphere including what it holds
- A player notices: a long weapon may lift a player's tag and target indicator
- Where: `crates/mao-client/src/blobshadow.rs:52`; `docs/CLIENT_UI.md:1187`; `crates/mao-client/src/nametag.rs:87`
- Decision: [ ] port  [ ] leave

#### `TGT-008` Objects are targeted only on the client

- SIMPLIFIED, low
- Original: not stated
- Client: targeting an object does not tell the server (no select, set-target or info request), because the server's select names characters and creatures
- A player notices: none visible
- Where: `crates/mao-client/src/net.rs:7834`
- Decision: [ ] port  [ ] leave

#### `TGT-009` Tab picks guards and guides

- SIMPLIFIED, low (notes only)
- Original: N3Msg_GetCloseTarget side counting (client matches)
- Client: Tab picks guards and guides; Q while "fighting" one stops instead of attacking next
- A player notices: Tab may select unattackable guards
- Where: `docs/CLIENT_UI.md:2771`
- Decision: [ ] port  [ ] leave

#### `TGT-010` Target frame and targeting window are partial

- SIMPLIFIED, low
- Original: TargetingModule_t, TargetView, friendly/hostile health bars, GFX_GUI_CC_TARGET_FRAME
- Client: ui.rs::target_frame only; with the skin the original frame, without it a plain box (checked offscreen only); distance written on it
- A player notices: target display differs
- Where: `crates/mao-client/src/ui.rs:2273`; `docs/GUI_DLL_PARITY.md:74`; `docs/CLIENT_UI.md:2042`
- Decision: [ ] port  [ ] leave

### Targeting and mouse: the original was not read: inferred or measured (2)

#### `TGT-011` Control center's two target buttons guessed

- GUESS, low (notes only)
- Original: not stated
- Client: "we guessed: self / next hostile"
- A player notices: target buttons may not do what the original's do
- Where: `docs/ORIGINAL_UI_PLAN.md:217`
- Decision: [ ] port  [ ] leave

#### `TGT-012` Mouse pointer hot spots and egui-driven choice

- GUESS, low (notes only)
- Original: not stated
- Client: pointer picture chosen from the pointer egui asks for; hot spots set by the client; not checked live
- A player notices: pointer shape/hot spot may be off
- Where: `docs/CLIENT_UI.md:2064`
- Decision: [ ] port  [ ] leave

### Targeting and mouse: a guard or fallback the original lacks (1)

#### `TGT-013` Target sent as "none" on entering and when lost

- SAFETY, low
- Original: sends its selection when it changes; what it does on entering was not read
- Client: sends LookAt with no target once on entering and whenever a target is lost, so a taken-over character's old target does not catch targetless casts
- A player notices: no stale target after a session takeover
- Where: `crates/mao-client/src/net.rs:8639`; `docs/CLIENT_UI.md:551`; `docs/LIVE_TEST_LIST.md:103`
- Decision: [ ] port  [ ] leave

### Targeting and mouse: the original does it, the client does not (3)

#### `TGT-014` Monster info carries no name or level

- MISSING, low (notes only)
- Original: not stated
- Client: monster level is meaningless in this layout; no name/level for monsters
- A player notices: no level on monster target frame
- Where: `docs/CLIENT_UI.md:1892`
- Decision: [ ] port  [ ] leave

#### `TGT-015` Name tags lack title, names, marks, colour table, ignored icon, side colours

- MISSING, low
- Original: Indicator_t title, first/last names, ** name **, = name =, "the <breed>", colour table, ignored icon, battle-station colours
- Client: not done (other characters' stats not known); a tag shows the name and the organization name in the side's colour
- A player notices: name tags lack these marks
- Where: `docs/ORIGINAL_AUDIT.md:87`; `crates/mao-client/src/nametag.rs:1`
- Decision: [ ] port  [ ] leave

#### `TGT-016` Unnamed usable objects shown as "Object xxxx"

- MISSING, low
- Original: shows object names
- Client: an object whose name is not decoded is shown as "Object <hex id>"
- A player notices: unnamed targets
- Where: `crates/mao-client/src/net.rs:3389`; `docs/CLIENT_UI.md:99`
- Decision: [ ] port  [ ] leave

### Targeting and mouse: the client has it, the original does not (1)

#### `TGT-017` Double click uses what the first click took; Shift/Ctrl/Alt clicks not ported

- ADDITION, high
- Original: GUI.dll FUN_1002cfac: a left click selects the one after the selected target on the line (no double click described); with Shift opens the info page; with Ctrl or Alt on a character switches the fight to it (SwitchTarget)
- Client: a second left click within half a second is a double click and uses what the first took, without stepping on (client's own); the Shift info page and Ctrl/Alt SwitchTarget clicks are not done
- A player notices: double-clicking a creature or object uses it where the original would step to the next thing behind it; modifier clicks do nothing
- Where: `crates/mao-client/src/viewer.rs:1290`; `docs/CLIENT_UI.md:4006`; `docs/ORIGINAL_AUDIT.md:71`
- Decision: [ ] keep  [x] remove
- Done (2026-11-18): the client's own double click is removed. A double click is the original's now (`N3Msg_DefaultActionOnDynel` at the second press, option `DoubleclickAction`), Shift + click opens the info page, Ctrl + click on a character is `N3Msg_SwitchTarget` (its attack flag 2 was never captured), and a right click no longer selects. `CLIENT_UI.md` "The clicks in the world".

## Death and revival

### Death and revival: the client knowingly does otherwise (5)

#### `DTH-001` Attack and use keys blocked while dead; nobody followed

- OWN-RULE, low (notes only)
- Original: interface is told of death through DeadMessage
- Client: while dead the attack and use keys do nothing and nobody is followed (client-side)
- A player notices: no actions while dead
- Where: `docs/CLIENT_UI.md:731`
- Decision: [ ] port  [ ] leave

#### `DTH-002` Corpse falls back to the nearest dead body within 4 m

- OWN-RULE, low
- Original: the corpse message names the one that died (corpse_of; corpse stats 415/416)
- Client: when the corpse names nobody, takes the nearest dead body within 4 m as its remains
- A player notices: a nameless corpse may be attached to the wrong body or none
- Where: `crates/mao-client/src/net.rs:3362`; `docs/CLIENT_UI.md:4751`
- Decision: [ ] port  [ ] leave

#### `DTH-003` Own character never a corpse's body; own corpse is an object of its own

- OWN-RULE, low
- Original: not stated
- Client: the client's own character is never a corpse's body (reworked 2026-11-17); its corpse is an object of its own named by the corpse's name
- A player notices: own corpse treated differently from creature corpses
- Where: `docs/CLIENT_UI.md:4826`; `crates/mao-client/src/net.rs:12629`
- Decision: [ ] port  [ ] leave

#### `DTH-004` Own death line told once in five seconds

- OWN-RULE, low
- Original: Gamecode FUN_1005b3d8 gives the game's line for the death
- Client: tells the death line only if none was told in the last 5 seconds
- A player notices: a repeated death message gives one line
- Where: `crates/mao-client/src/net.rs:3873`
- Decision: [ ] port  [ ] leave

#### `DTH-005` Whiteout clears over 1.5 s after arrival; only the world goes white

- OWN-RULE, low
- Original: not stated
- Client: the white clears over 1.5 s after the arrival; windows and chat are not whitened
- A player notices: how the screen returns after revival
- Where: `crates/mao-client/src/whiteout.rs:9`; `crates/mao-client/src/whiteout.rs:16`
- Decision: [ ] port  [ ] leave

### Death and revival: an approximation or a stand-in (1)

#### `DTH-006` Death animation: controller priority and loop arguments not followed

- SIMPLIFIED, low (notes only)
- Original: the controller's own priority and loop arguments (+0x18, not decoded)
- Client: holds the pose by direction instead
- A player notices: death pose hold implemented differently
- Where: `docs/CLIENT_UI.md:2401`
- Decision: [ ] port  [ ] leave

### Death and revival: the original was not read: inferred or measured (4)

#### `DTH-007` White fade at own death: that effect 3000 is the white is a guess

- GUESS, medium
- Original: CharDie_t (FUN_1007bad5) makes effect 3000 on the own character; the effect itself was not read and the original's code for a white was looked for and not found
- Client: from the user's description, the world fades to white over three seconds (RISE) once the death has played out (an older note said over the minute the body lies; the code now says three seconds, a minute until 2026-11-15)
- A player notices: the white screen on death may differ in look and timing from the original's
- Where: `crates/mao-client/src/whiteout.rs:7`; `crates/mao-client/src/net.rs:4304`; `docs/CLIENT_UI.md:4123`; `docs/CLIENT_UI.md:1541`; `docs/ORIGINAL_AUDIT.md:50`; `docs/ORIGINAL_AUDIT.md:77`
- Decision: [ ] port  [ ] leave

#### `DTH-008` Corpse naming a dead creature moves its body to the corpse's place

- GUESS, low
- Original: not stated
- Client: a corpse naming a dead creature (stats 415/416) becomes that creature's body and moves it to the corpse's place, even if it died twelve metres away; the body is renamed "Remains of ..."; one that dies with no corpse goes with its despawn
- A player notices: bodies jump to where the corpse appears
- Where: `crates/mao-client/src/net.rs:12116`
- Decision: [ ] port  [ ] leave

#### `DTH-009` Corpse's owner found by scanning for stat keys

- GUESS, low
- Original: not stated
- Client: the dead character is found by searching the announcement for the bytes of stats 415 and 416
- A player notices: a corpse may not link to its owner for an unusual message
- Where: `crates/mao-net/src/n3.rs:1692`
- Decision: [ ] port  [ ] leave

#### `DTH-010` Invented names for corpses

- GUESS, low
- Original: not stated
- Client: if the corpse has no name, calls it "Remains of <name>" ("the dead" if unknown), or "Object <hex>" for a corpse of one it never had
- A player notices: some corpses show a made-up name
- Where: `crates/mao-client/src/net.rs:3383`; `crates/mao-client/src/net.rs:3389`
- Decision: [ ] port  [ ] leave

### Death and revival: a guard or fallback the original lacks (2)

#### `DTH-011` Dead bodies remembered for two minutes

- SAFETY, low
- Original: not stated
- Client: bodies of the lately dead are forgotten after 120 s
- A player notices: bodies vanish from the client's bookkeeping after two minutes
- Where: `crates/mao-client/src/net.rs:1933`
- Decision: [ ] port  [ ] leave

#### `DTH-012` Unseen death counts as played out after 6 seconds

- SAFETY, low
- Original: CharDie_t (FUN_1007bad5, FUN_1003c6e7) waits for the death animation to play out, then effect 3000 and three seconds later CharacterAction 0x98
- Client: a death never seen to end (no death animation, or character not drawn) counts as played out after six seconds, then whitens and tells the server three seconds later
- A player notices: death screen and revival call come on a fixed timer in that case
- Where: `crates/mao-client/src/net.rs:12551`; `crates/mao-client/src/net.rs:4327`; `docs/CLIENT_UI.md:4121`
- Decision: [ ] port  [ ] leave

### Death and revival: the original does it, the client does not (2)

#### `DTH-013` Corpse whose body is missing is not drawn

- MISSING, high (notes only)
- Original: draws a corpse from its own data
- Client: a corpse whose body the client has not got (e.g. present before arrival, or after walking away and back) is named, not drawn; some dead entities carry impossible places
- A player notices: some corpses invisible
- Where: `docs/CLIENT_UI.md:1193`; `docs/CLIENT_UI.md:4773`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): The original's corpse has a CAT mesh of its own (the dead one's mesh, scale, clothes and textures are in its announcement) and shows it whenever the dead character is not there (Gamecode FUN_1007eaf0, FUN_1007ed2c). The client reads that (n3::corpse_look) and draws a corpse it has no body for, lying. The pose is a guess (the end of a death of that mesh's creature): no captured corpse names one. Checked beside the original's screenshot of corpses it arrived at. Not done: a humanoid corpse's skin and head from the corpse's own stats.

#### `DTH-014` Own death: reasons, death music, resurrection line and windows' signal not ported

- MISSING, high (notes only)
- Original: reason in the special-attack and health-damage messages; death music mode (SetDeathMusicMode); windows' reaction to the DeadMessage signal; resurrection line and stand-up (FUN_1003ec1f, shock text from level 15); meaning of CharacterAction 0x99
- Client: only the AttackInfo/health line and keys done; the rest not ported (no death-music or resurrection code found in crates/)
- A player notices: no death music, no resurrection text
- Where: `docs/ORIGINAL_AUDIT.md:51`; `docs/CLIENT_UI.md:733`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done with what was left. The input stop: while the own character is dead no key, mouse button, wheel step, pointer move or typed character is taken and the pointer is off, until the character is in play again, as the original (there is no timer in it; the eight seconds were its capture's). The resurrection shock: stat 247 at 5000 with the resurrection's line, counted down by the client 27 every two seconds, lowering every shown skill and ability by shock * base / 10000 meanwhile, with its end's line. Not ported: the AFK timer stopped while dead. The resurrection message is in no capture; a death that sticks would keep the input off (the window's close button still works).
- Done in part (2026-11-18): ported: the death's reason from the four other messages that carry it (`SpecialAttackInfo`, `HealthDamage`, `ShieldAttack`, `ReflectAttack`: the capture's own death, by a nano program, was told nothing) and from `CharacterAction` 0x99, which is that and nothing else; and the resurrection message (`ResurrectIIR_t`, in no capture): health, nano, the shock's line from level 15. Nothing to port: the death music mode is an empty function in the original. Left on purpose: the windows' reaction, which is not what the line above says (`DeadMessage` is the character object going at every zoning; what the original does at a death is stop all input for some eight seconds, which here could leave a stuck death with no way out). Not ported: the resurrection shock itself (stat 247).

## NPCs and creatures

### NPCs and creatures: the client knowingly does otherwise (4)

#### `NPC-001` Silent NPC is told its chat is closed and asked again after two seconds

- OWN-RULE, medium
- Original: not stated; the original has nothing of the kind
- Client: an NPC silent for two seconds after being asked is told the chat is closed and asked once more; the window then opens about two seconds late
- A player notices: NPC dialogue opens late in that case
- Where: `crates/mao-client/src/net.rs:6565`; `docs/CLIENT_UI.md:1574`; `docs/LIVE_TEST_LIST.md:165`
- Decision: [ ] port  [ ] leave

#### `NPC-002` A mission told again takes its old row; list emptied at zoning

- OWN-RULE, low
- Original: the original adds the message's missions to the character's list without looking (FUN_10056cdd); a zoning gives a new character
- Client: a mission already in the list takes its old row; the list is emptied at a zoning
- A player notices: none visible in normal play
- Where: `crates/mao-client/src/missionlog.rs:255`; `docs/ORIGINAL_AUDIT.md:74`; `docs/CLIENT_UI.md:4082`
- Decision: [ ] port  [ ] leave

#### `NPC-003` Candidate animations for a creature's action are an invented order

- OWN-RULE, low
- Original: not stated; a creature's record can name an animation the database does not hold
- Client: animation_candidates lists keys best first and the first that loads is taken
- A player notices: a creature may show another action's animation when its own is missing
- Where: `crates/mao-formats/src/npc.rs:146`
- Decision: [ ] port  [ ] leave

#### `NPC-004` Pet window: client's own menu; fixed buttons sorted by family

- OWN-RULE, low
- Original: PetView_c icon menu (FUN_1007e75e) with Show buttons and Fixed Pet Buttons, fixed rows by PetType (stat 512)
- Client: a menu button with two check boxes kept in mao.ini; fixed order sorts by NPCFamily (stat 455) as a stand-in
- A player notices: pet window menu looks different; fixed rows can differ
- Where: `crates/mao-client/src/pets.rs:191`; `docs/GUI_DLL_PARITY.md:94`
- Decision: [ ] port  [ ] leave

### NPCs and creatures: an approximation or a stand-in (9)

#### `NPC-005` Per-pet command never captured; /pet sends an empty list

- SIMPLIFIED, high
- Original: N3Msg_SendPetCommand (0x1001b2ea) puts the chosen pet in the list; PetCommandIIR_c writer FUN_100766ec
- Client: /pet sends an empty pet list (all pets) and lets the server take its known target; the window's per-pet commands are built from the decompile with no capture to compare
- A player notices: a typed pet command affects all pets; window buttons unverified
- Where: `crates/mao-net/src/n3.rs:696`; `crates/mao-net/src/n3.rs:709`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): `/pet` is read as Gamecode's FUN_100542ad reads it: the pets named before the command (none: every pet, a rename the target; `all`; names matched without regard to case), the commands with an argument (cycle, social by Gamecode's table of 70, rename, chat, script), `Feedback_YouHaveNoPet` without a pet; the message carries the value, and the window's buttons the tower word (PetType 15). The per-pet layout is still from the writer only, never captured; `/tower` left.

#### `NPC-006` Vehicle riders and other characters use plain/medium bodies only

- SIMPLIFIED, high
- Original: GUI.dll CCCharacter_t::ChangeMesh names thin, fat (and robed) bodies (0 small, 1 medium, 2 large); vehicle spells name them too
- Client: every character is drawn with its breed's plain/medium body; riders likewise; vehicles have no jump animation, trails (3039), particles (3028) or movement flags (0x2000/0x4000)
- A player notices: thin/fat players look medium; board/bike visuals incomplete
- Where: `crates/mao-client/src/net.rs:4041`; `crates/mao-client/src/net.rs:4633`; `docs/CLIENT_UI.md:2335`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18): Partly stale, checked 2026-11-18: the vehicles' trails (kind 3039, effects::visual_trails), thrown particles (kind 3028) and the movement flags 0x2000 / 0x4000 are drawn now. Still so: every character of the world is drawn with its breed's medium body (Characters::build is 1 outside character creation; the thin and fat bodies are only in the creation preview). Not checked: a vehicle's jump animation.

#### `NPC-007` A creature with no standing animation holds its walk's first frame

- SIMPLIFIED, medium
- Original: the original falls back by its own table (FUN_10010b3c) for characters
- Client: for creatures the walk stands in for a missing run; a missing idle falls to 127 then the walk, held at its first frame (polymorphs too)
- A player notices: such creatures stand stiffly in one pose
- Where: `crates/mao-client/src/net.rs:4494`; `crates/mao-formats/src/npc.rs:125`; `docs/CLIENT_UI.md:1987`
- Decision: [ ] port  [ ] leave

#### `NPC-008` Hoverboards/hoverbikes drawn from a template worked out from the program

- SIMPLIFIED, medium (notes only)
- Original: server sets MonsterData (stat 0x167) and the stat handler FUN_1005a3b1 loads the template
- Client: derives the template from the program (own running nano list, others' Buff messages, 30 s); the server's own MonsterData not read
- A player notices: a vehicle could show or not depending on guessed buffs
- Where: `docs/CLIENT_UI.md:2331`; `crates/mao-client/src/net.rs:5326`
- Decision: [ ] port  [ ] leave

#### `NPC-009` Creature animation variants fixed per creature, not re-rolled

- SIMPLIFIED, low
- Original: FUN_10045976 picks a variant with rand() at each start
- Client: the creature's id (seed modulo variants) picks the variant, for animations and sounds
- A player notices: a creature always plays the same variant of an action
- Where: `crates/mao-client/src/net.rs:4446`; `crates/mao-formats/src/npc.rs:113`; `docs/ORIGINAL_AUDIT.md:84`
- Decision: [ ] port  [ ] leave

#### `NPC-010` Creature size taken from posed points, not rest meshes

- SIMPLIFIED, low (notes only)
- Original: VisualMesh_t::GetMeshHeight uses each mesh at rest
- Client: sizes from the posed points
- A player notices: tag heights may differ slightly
- Where: `docs/ORIGINAL_AUDIT.md:29`
- Decision: [ ] port  [ ] leave

#### `NPC-011` Following creatures (FollowTarget type 2) stop where they are

- SIMPLIFIED, low (notes only)
- Original: FollowTargetIIR_c type 2 follows a target
- Client: live it is always identity 0, so the client stops the creature where it is
- A player notices: following creatures not shown following
- Where: `docs/CLIENT_UI.md:2705`
- Decision: [ ] port  [ ] leave

#### `NPC-012` Grounded creature's idle held at its end; announced-sitting players not done

- SIMPLIFIED, low (notes only)
- Original: the idle loops
- Client: the idle is held at its end where the original loops it; players announced sitting not done (their update's fields before the block were not read)
- A player notices: sitting players stand; wounded NPC pose static
- Where: `docs/CLIENT_UI.md:4683`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18): a player announced sitting now sits (the movement mode of its full update is read: mode 8 holds the ground's idle). The grounded creature's idle held at its end is as it was.

#### `NPC-013` Shop totals computed by the client

- SIMPLIFIED, low
- Original: PartnerCashView and OurCashView show costs via N3Msg_TradeGetInventoryCost
- Client: prices come from the client's own port of Gamecode's VendingMachine rules; the total is the sum of rows, painted into the field with a hand-drawn frame
- A player notices: totals may differ if a rule was ported wrongly
- Where: `crates/mao-client/src/shop.rs:15`
- Decision: [ ] port  [ ] leave

### NPCs and creatures: the original was not read: inferred or measured (14)

#### `NPC-014` Shop: double-click buys or removes, right click opens the page

- GUESS, high
- Original: items go in by dropping on a list (FUN_100e015f) or Ctrl+right click (FUN_100cae1b); per-row gestures are the generic item list's
- Client: double click on a stock row puts it in, double click on a bought/sold row takes it out, right click on stock shows its page
- A player notices: buying works by double click, which the original's list does not do
- Where: `crates/mao-client/src/shop.rs:494`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the entry's "Original" line was incomplete: the trade's lists have their own click handlers. One left or right click on a stock row buys one (`N3Msg_TradeAddItem`, GUI.dll FUN_100dffe8), one on a bought or sold row takes it out, Shift + left click shows the page, there is no double click; an inventory item goes in by Ctrl + right click (FUN_100cae1b) or a drop. All ported (`shop::row_click`); the client's Ctrl + double click is gone. Not ported: the original's own refusals before an add (cannot afford, trade full). `CLIENT_UI.md` "A shop's rows act at one click".

#### `NPC-015` Built-in machines, doors and mail terminals found by inferred rules

- GUESS, medium
- Original: not stated (the server does not announce mail terminals; the original's way to find the used terminal is not read)
- Client: a built-in machine is used by its record's place among its kind in the table; without the playfield message's runs an instance's doors are numbered along consecutive ids; mail terminals are the nearest built-in one, else four seen in captures, within 25 units
- A player notices: a machine or door may be refused or matched wrongly; the mail window may not open at an unlisted terminal
- Where: `crates/mao-client/src/net.rs:12817`; `crates/mao-client/src/net.rs:11983`; `crates/mao-client/src/mailbox.rs:17`
- Decision: [ ] port  [ ] leave

#### `NPC-016` Creature movement mode (ground/stand) and run speed read from measured bytes

- GUESS, medium
- Original: Gamecode's reader FUN_1007916d copies a 28-byte block out whole
- Client: the mode is block byte 12 (8 on the ground, lying flat in the idle; 3 stands) and run speed a byte after it, measured over 5,600 captured announcements
- A player notices: wounded creatures may lie or stand wrongly where the measurement misses a case
- Where: `crates/mao-net/src/n3.rs:1686`; `crates/mao-client/src/net.rs:11544`
- Decision: [ ] port  [ ] leave

#### `NPC-017` Creature run speed is the characters' rule, learnt or fallback, not read

- GUESS, medium
- Original: the characters' rule FUN_1006f9eb: RunSpeed / 275 + 5 within 1.5 and 13 (falling below 15% health); creatures are moved by FollowTarget (0x18 walk / 0x19 run)
- Client: a running creature uses RunSpeed/275+5 (clamped 1.5-13) if known, else the speed worked out from two paths in a row (first place of a path taken as where the server has it, clamped 0.5-15), else a flat 5 (RUN_SPEED); the fall below 15% health is not applied to creatures; a creature chasing the player stops 1.5 m short; fitted from captures
- A player notices: a creature may lag or lead the server's place; its pace approximates the original's
- Where: `crates/mao-client/src/net.rs:3746`; `crates/mao-client/src/net.rs:111`; `crates/mao-client/src/net.rs:129`; `crates/mao-client/src/net.rs:3260`; `crates/mao-client/src/net.rs:12689`; `crates/mao-net/src/n3.rs:1688` and 3 more
- Decision: [ ] port  [ ] leave

#### `NPC-018` Creature template found by scanning, not by the original's field order

- GUESS, medium
- Original: Gamecode FUN_1007916d reads the creature fields in order
- Client: the template is found by scanning for scale, a zero and 0x1f within 72 bytes after the name, with a looser second rule added after a boss drew as a box
- A player notices: a creature with another update shape may be drawn as the placeholder box
- Where: `crates/mao-net/src/n3.rs:1718`
- Decision: [ ] port  [ ] leave

#### `NPC-019` Creatures walk at a flat 1.5 m/s

- GUESS, medium
- Original: not stated; the server sends paths
- Client: a walked path goes at a fixed PATH_SPEED 1.5 m/s measured from wandering creatures
- A player notices: walking pace may differ from the server's
- Where: `crates/mao-client/src/net.rs:109`; `crates/mao-client/src/net.rs:3746`
- Decision: [ ] port  [ ] leave

#### `NPC-020` Mission request scope for an unknown terminal kind is a fixed 5

- GUESS, medium
- Original: Gamecode FUN_10086b3e reads the terminal's QuestOriginator_e (stat 490)
- Client: for a terminal of unknown kind sends scope 5 (a clan terminal's), the value seen in one capture; AOSharp's values did not work
- A player notices: requests at unlisted terminals may ask for the wrong mission kind
- Where: `crates/mao-client/src/mission.rs:18`
- Decision: [ ] port  [ ] leave

#### `NPC-021` Backing animation inferred from moving against the facing

- GUESS, low
- Original: not stated
- Client: a character is backing (walk-back/run-back) when its move is against its facing by more than 0.3 of the distance
- A player notices: back-pedalling animation may differ
- Where: `crates/mao-client/src/net.rs:3692`
- Decision: [ ] port  [ ] leave

#### `NPC-022` Corpse name found by scanning bytes

- GUESS, low
- Original: the original reads the message's fields
- Client: the corpse name is the first plausible length-prefixed string of letters after the stats; fallback "Remains of <name>" when none is found
- A player notices: a corpse's name could be wrong or missing on odd messages
- Where: `crates/mao-client/src/net.rs:119`; `crates/mao-client/src/net.rs:3383`; `docs/CLIENT_UI.md:2722`
- Decision: [ ] port  [ ] leave

#### `NPC-023` Mission terminal "need team" error box: signal slot not found

- GUESS, low (notes only)
- Original: QuestBooth_NeedTeam signalled through GlobalSignals_c +0x184; the slot that takes it was not found
- Client: shows an error box chosen by the signal's other uses
- A player notices: box may differ from the original's
- Where: `docs/ORIGINAL_AUDIT.md:36`; `docs/CLIENT_UI.md:996`
- Decision: [ ] port  [ ] leave

#### `NPC-024` NPC template record read by a heuristic when its layout is odd

- GUESS, low
- Original: Gamecode FUN_1004de5d reads the record's sections in order
- Client: when the sections do not read, the first 12 pairs after the header are taken as stats and animations are found by the count word 2018
- A player notices: a creature with an odd record may get a wrong mesh or animation
- Where: `crates/mao-formats/src/npc.rs:14`
- Decision: [ ] port  [ ] leave

#### `NPC-025` Other characters' polymorph form ends by a 30 s timeout; their movement names not done

- GUESS, low (notes only)
- Original: not stated
- Client: another character's polymorph (Buff message) keeps its form 30 s after last seen; whether the server announces a buff's end is unknown; the polymorphed character's own speed/animation names not done; not seen drawn live
- A player notices: a form may linger after a polymorph ends; movement animation may differ
- Where: `docs/CLIENT_UI.md:1975`; `docs/CLIENT_UI.md:1967`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18): the 30 s timeout is gone: a polymorph ends with its program's `Buff`, or when the character is announced without the body. The polymorphed character's own speed and animation names are still not done.

#### `NPC-026` Polymorph animation rate is a client-tuned 1.0 where not calibrated

- GUESS, low
- Original: Setupf/animcalibration.txt factors
- Client: MORPH_ANIM_RATE 1.0 as a further factor (the 0.75 constant is gone; the calibration is applied per mesh/animation, factor 1 when unlisted or file missing); the user found the uncalibrated speed a little fast
- A player notices: animation speed of unlisted creatures/polymorphs is client-tuned
- Where: `crates/mao-client/src/net.rs:4094`; `crates/mao-client/src/net.rs:4496`; `crates/mao-client/src/effects.rs:2692`; `docs/CLIENT_UI.md:1991`
- Decision: [ ] port  [ ] leave

#### `NPC-027` Server-announced shop (Junk Shop) model from template stat

- GUESS, low
- Original: not stated
- Client: a machine with no model gets its template's Mesh stat; a machine with no name template is named as its template; "Booth xxxx" when it has no name
- A player notices: placeholder names/meshes for such machines
- Where: `crates/mao-client/src/net.rs:1860`; `docs/CLIENT_UI.md:4662`
- Decision: [ ] port  [ ] leave

### NPCs and creatures: the original does it, the client does not (11)

#### `NPC-028` Other characters and humanoid NPCs are drawn at scale 100

- MISSING, high
- Original: the body's scale is the Scale stat (360) over 100 for every dynel (SetBodyScale, FUN_100892ca)
- Client: reads the Scale stat for one's own character and for creatures; every other player and humanoid NPC is 100; remains of a scaled creature not looked at
- A player notices: scaled players and humanoids show at normal size
- Where: `crates/mao-client/src/net.rs:5396`; `docs/CLIENT_UI.md:749`; `docs/ORIGINAL_AUDIT.md:59`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): a player's scale is read from its full update (a short after its MonsterData word; the original sets it as stat 360 for every character, FUN_1007835c) and other players and humanoid NPCs are drawn at it, not below 20. Most players are 90 or 110, not 100. A player with a monster body (a polymorph, a Yalmaha at 20) keeps 100, since the client does not draw that body for others yet (MOV-020, RND-135; the field is read now). Not followed: a change of the stat after the update (a size nano). `FUN_100892ca` in the line above is the items' SetStat, not a character's. Not seen live.

#### `NPC-029` Pet window: no running-nano list per pet, no Remember Pet Names

- MISSING, high
- Original: each pet has an icon list of its running nanos with time remaining (FUN_10045d42); "Remember Pet Names" renames on summoning
- Client: left out; the client does not know a pet's programs' times
- A player notices: no pet nano list and no automatic pet renaming
- Where: `crates/mao-client/src/pets.rs:22`; `docs/CAPTURE_GAPS.md:142`; `crates/mao-client/src/pets.rs:41`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): under each pet a row of its running programs' icons (the `NCUListView_c` the original puts there: smallest icons, by the time left, none when the pet has none or is away), read from the per-character program lists of the same day's Buff fix. `PetType` is in the entity message after all, so the pets' places are the original's. Remember Pet Names is in the menu: a pet's new name (the server's `SetNameIIR_t`) is kept for its place and given back to a pet of that kind that comes; `/pet rename <name>` now sends its name. Neither the rename nor the name change is in any capture: both are the code's alone, and nothing is sent by itself until a name change has come and been understood.

#### `NPC-030` "No missions available" line in the client's own words

- MISSING, low
- Original: "No missions available, please try again later!" (FUN_100d0e71); MustHave2FreeSlots line
- Client: says "The terminal has no missions just now; try again later." and "The booth offered no missions."; MustHave2FreeSlots is ported (mission.rs:673), only the no-missions wording differs
- A player notices: wording differs
- Where: `crates/mao-client/src/net.rs:6982`; `crates/mao-client/src/mission.rs:322`; `docs/CLIENT_UI.md:1005`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): the line is a chat line now and the old offers are emptied, as FUN_100d0e71 does; the sentence is a literal of GUI.dll's (not in text.mdb), so the words are still the client's own.

#### `NPC-031` CharacterAction 0x64 (NPC state) not decoded

- MISSING, low (notes only)
- Original: case 26 (0x1005ddba) FUN_1003c68e stores the value at +0x1c of the +0x1dc part
- Client: meaning unknown, not handled
- A player notices: not stated
- Where: `docs/CAPTURE_GAPS.md:287`
- Decision: [ ] port  [ ] leave

#### `NPC-032` Guards' Action messages toward an attacker not ported

- MISSING, low (notes only)
- Original: guards (0xc749) send Action messages (2049527c) toward a player who attacked them, with the PvP flag
- Client: not ported
- A player notices: guards do not react as the original's
- Where: `docs/ARETE_CAPTURE.md:153`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): a misreading: the 2049527c messages on 0xc749 identities in the capture are bags' and chests' open (0x64) and close (0x66) actions, not guards.

#### `NPC-033` Journal "Update Team Members" disabled

- MISSING, low
- Original: gives the mission to the team members (GiveQuestToMembers)
- Client: entry shown for team missions but disabled; the message was never captured
- A player notices: team missions cannot be shared again from the journal
- Where: `crates/mao-client/src/missionlog.rs:31`
- Decision: [ ] port  [ ] leave

#### `NPC-034` Male Watcher drawn without its gear

- MISSING, low (notes only)
- Original: not stated
- Client: the Male Watcher is drawn without its gear; unresolved
- A player notices: an NPC appears unequipped
- Where: `docs/CLIENT_UI.md:3483`
- Decision: [ ] port  [ ] leave

#### `NPC-035` Mission page: shadow-knowledge pay for level 200-220 not shown

- MISSING, low
- Original: for level 200-220 the page shows pay in shadow knowledge (GetSKForXP)
- Client: no experience row for those levels
- A player notices: no experience reward line at levels 200-220
- Where: `crates/mao-client/src/missionlog.rs:321`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): the row is IT_QuestSKReward with GetSKForXP in a Shadowlands playfield and absent elsewhere (missionlog::Reader, sk_for_xp).

#### `NPC-036` Mission request sliders not kept; advanced options button not done

- MISSING, low (notes only)
- Original: sliders kept between sessions, an advanced options button
- Client: not
- A player notices: sliders reset each time
- Where: `docs/ORIGINAL_AUDIT.md:72`
- Decision: [ ] port  [ ] leave

#### `NPC-037` Mission: character's flag +0x140 not read for the new-mission line

- MISSING, low (notes only)
- Original: the new-mission line also asks the character's flag +0x140
- Client: not read
- A player notices: new-mission line may show when the original would not
- Where: `docs/ORIGINAL_AUDIT.md:74`
- Decision: [ ] port  [ ] leave

#### `NPC-038` Other polymorphs do not sidestep-animate

- MISSING, low (notes only)
- Original: not stated
- Client: only the player's own sidestep is known; another character's direction not read from the server
- A player notices: other polymorphs do not sidestep-animate
- Where: `docs/CLIENT_UI.md:2011`
- Decision: [ ] port  [ ] leave

### NPCs and creatures: the client has it, the original does not (2)

#### `NPC-039` Mission helper: "Looking for" lists and "Keep asking until one fits"

- ADDITION, medium
- Original: no such helper (players used outside helpers); one request per button press, each costing credits equal to the level
- Client: lists of reward-name and place parts, a green frame on fitting offers; "Keep asking" re-requests every 1.2 s up to a limit (default 20, at most 200) and stops after 10 s with no answer; kept in missionhelper.txt
- A player notices: players can roll missions automatically, spending credits, until a wanted reward or place comes up
- Where: `crates/mao-client/src/mission.rs:101`; `crates/mao-client/src/mission.rs:102`; `crates/mao-client/src/mission.rs:183`; `docs/LIVE_TEST_LIST.md:579`; `docs/CLIENT_UI.md:934`; `docs/CLIENT_UI.md:944` and 1 more
- Decision: [ ] keep  [ ] remove

#### `NPC-040` Clicked NPC answer echoed locally; client line for rejected items

- ADDITION, low
- Original: lines come from KnubotAppendText; the client's own echo is not stated
- Client: a clicked answer is appended at once as the player's line before the server replies; rejected items add a system line "N item(s) given back."
- A player notices: possible duplicate or extra lines in the conversation
- Where: `crates/mao-client/src/npcchat.rs:291`
- Decision: [ ] keep  [ ] remove

## Inventory and items

### Inventory and items: the client knowingly does otherwise (15)

#### `INV-001` A shop is left when the character walks 8 m away

- OWN-RULE, high
- Original: where the original's client ends it was not found; Gamecode's range test FUN_1005a1e7 is called when a trade starts; the server leaves the trade open and answers "You are too far away"
- Client: declines the trade and closes the window once the character is more than 8 units from where the shop opened
- A player notices: shop windows close by themselves when walking away
- Where: `crates/mao-client/src/net.rs:7947`; `docs/CLIENT_UI.md:3535`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the rule is taken out. Nothing in the original's client ends a vending machine's trade by distance (`TradeView_c` has no slot on the character's place; `VendingMachine_t` runs the plain dynel's frame function; the range test FUN_1005a1e7 has no caller on a shop's path), and no capture shows the server doing it. The window stays until Decline, Escape, the server's word or another trade. `CLIENT_UI.md` "A shop stays open when the character walks away".

#### `INV-002` Item picked up by a press that moves (drag), not only on release

- OWN-RULE, high
- Original: The item lists pick up only on the left button's release on the cell it went down on; press, move, release on another cell does nothing (ItemListViewBase_c FUN_1004141b, MultiListView_c::MouseUp 0x10133d04).
- Client: A press that moves takes the item up in one stroke.
- A player notices: Dragging an item with the button held lifts it, where the original ignores such a gesture.
- Where: `crates/mao-client/src/invwindows.rs:17`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): a press that moves takes nothing up any more, in the inventory, the wear pages and every container: the lists tell of a click only when the button comes up on the cell it went down on (`multilist::click_only`; GUI.dll 0x10133d04, FUN_1004141b). The nano programs' and the actions' lists keep their drag. `CLIENT_UI.md` "An item is taken up by a click, not by a drag".

#### `INV-003` No use of an item by double click

- OWN-RULE, high
- Original: not stated
- Client: double click no longer uses an item; Ctrl+double click still puts an item into an open trade
- A player notices: double clicking an item does not use it
- Where: `docs/CLIENT_UI.md:3327`; `crates/mao-client/src/invwindows.rs:21`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): not a rule of the client's own: the original's item lists have no double-click handler. The first click takes the item up and the second puts it back on its own cell (FUN_100cc6e8, FUN_100cc855); an item is used by a right click. Nothing to change. `docs/INVENTORY_ORIGINAL.md` section 1.

#### `INV-004` Right click with an item in hand always does a tradeskill combine

- OWN-RULE, high
- Original: Right click with an item held: plain = UseItemOnItem (use message action 5); Shift (inventory only) = TradeskillCombine (CharacterAction 0x51); can also target worn items (FUN_100cc855, FUN_100e220f).
- Client: A plain right click on a backpack item with another in hand pushes a combine; both must be backpack items; worn items are not targets.
- A player notices: Using a held item on a worn item does not work, and a plain right click combines instead of using the item on it.
- Where: `crates/mao-client/src/invwindows.rs:577`; `docs/CLIENT_UI.md:3639`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): with an item in hand a plain right click on another item, in the inventory, on a wear page or in a container, is `UseItemOnItem` (the use message's action 5), and Shift + right click on an inventory item is the tradeskill combine (`CharacterAction` 0x51), as FUN_100cc855 and FUN_100e220f have it (`invwindows::held_right_click`). Not ported: the same with an item out of a bag or the bank in hand. `CLIENT_UI.md` "An item in hand and the right button".

#### `INV-005` Item tooltip is the client's own layout, with requirements and "When worn" list

- OWN-RULE, medium
- Original: Tooltip (FUN_10042193): the name in the item's rarity colour (N3Msg_GetRarityColor), and the description under it when ItemDescriptionTooltips is on.
- Client: Name in the theme accent colour, a small line with the slot label, QL and "(bag)", the stack count, the description, "To wear/wield/use" requirement lines in green/red and a "When worn" list of stat effects.
- A player notices: Tips look and read differently, and show requirements and stat effects the original's tip does not.
- Where: `crates/mao-client/src/gear.rs:199`; `docs/CLIENT_UI.md:1687`
- Decision: [ ] port  [ ] leave

#### `INV-006` Moves by use go one at a time with a wait

- OWN-RULE, medium
- Original: Sends both moves of a swap at once.
- Client: Wearing/taking off by use (right click swap) is queued; each move is sent only after the server answered the last plus 0.6 s (3 s at most); the swap ends if the wear place is still taken.
- A player notices: Swapping a worn item by use takes about a second per step.
- Where: `crates/mao-client/src/net.rs:8489`; `docs/CLIENT_UI.md:3661`
- Decision: [ ] port  [ ] leave

#### `INV-007` Right click takes an item out of a corpse

- OWN-RULE, medium
- Original: Right button up on a corpse's list does nothing (flag 0x400); only the held menu works there (FUN_10041526).
- Client: A right click on an item in any container takes it.
- A player notices: Right click loots a corpse where the original ignores it.
- Where: `crates/mao-client/src/loot.rs:316`
- Decision: [ ] port  [ ] leave

#### `INV-008` Splitting a stack asks by a number box

- OWN-RULE, medium
- Original: Ctrl+left press on a stack enters split mode; moving the mouse right/left changes the count, clamped 1..count-1, and the release raises the pick-up with that count (FUN_10041926, FUN_10041ca4).
- Client: Ctrl+click opens a small "Split stack" window where the player types a number, then OK; that many go in hand.
- A player notices: A different way of choosing how many to split off.
- Where: `crates/mao-client/src/invwindows.rs:29`; `docs/CLIENT_UI.md:3633`
- Decision: [ ] port  [ ] leave

#### `INV-009` Ctrl+double left click puts an item into the trade, instead of Ctrl+right click

- OWN-RULE, low
- Original: Ctrl + right click (the click signal FUN_100cae1b) adds the item to an open trade or shop (N3Msg_TradeAddItem); a drop on the trade window also adds it.
- Client: The second click of a Ctrl+double left click raises a trade add (documented as a shop sale only).
- A player notices: A different gesture is needed to put an item into a trade.
- Where: `crates/mao-client/src/invwindows.rs:20`
- Decision: [ ] port  [ ] leave

#### `INV-010` Delete confirmation uses the client's own wording

- OWN-RULE, low
- Original: The question is the text ReallyDeleteItemX (Warning, Yes/No) -> N3InterfaceModule_t::DeleteItem (FUN_100ca997).
- Client: Asks "Destroy <item>? It cannot be brought back." with its own text.
- A player notices: Different wording of the delete question.
- Where: `crates/mao-client/src/invwindows.rs:637`
- Decision: [ ] port  [ ] leave

#### `INV-011` Dragging out of a corpse or chest takes the item up onto the pointer

- OWN-RULE, low
- Original: A container that is not one's own never starts a drag: a left click loots at once with MoveItemToInventory (FUN_100cc6e8).
- Client: A press that moves out of a corpse or chest takes the item up, and it is taken when let go over the inventory window.
- A player notices: Looting can be done by dragging as well as clicking.
- Where: `crates/mao-client/src/loot.rs:485`
- Decision: [ ] port  [ ] leave

#### `INV-012` Over-full container grows a row

- OWN-RULE, low
- Original: Not stated.
- Client: An item past a container's room is given the first free cell beyond it, and the grid grows a row.
- A player notices: An over-full container shows extra rows.
- Where: `crates/mao-client/src/places.rs:117`
- Decision: [ ] port  [ ] leave

#### `INV-013` Shift+right-click inventory menu (Info, Use, Delete) is the client's own

- OWN-RULE, low
- Original: menu look is the original's popup; Info/Use/Delete entries not the original's
- Client: Info, Use, Delete are the client's own entries
- A player notices: menu contents differ
- Where: `docs/CLIENT_UI.md:1756`; `crates/mao-client/src/invwindows.rs:20`
- Decision: [ ] port  [ ] leave

#### `INV-014` Wear check ignores the 'wear' can flag

- OWN-RULE, low
- Original: FUN_10081d5f: an item needs 'wear' among its can flags.
- Client: Not checked, since a garden key with only carry is worn all the same; the server decides.
- A player notices: Items the original refuses may be tried.
- Where: `crates/mao-client/src/net.rs:1109`
- Decision: [ ] port  [ ] leave

#### `INV-015` Wear to another allowed place

- OWN-RULE, low
- Original: N3Msg_UseItem (0x10028883) moves the item to its wear page place (stat 88).
- Client: Another place the item's flags (stat 298) allow is taken when its own is taken and that one is free.
- A player notices: Item goes to an alternate slot.
- Where: `docs/CLIENT_UI.md:3613`; `crates/mao-client/src/net.rs:8585`
- Decision: [ ] port  [ ] leave

### Inventory and items: an approximation or a stand-in (22)

#### `INV-016` Item let go over the world from a container is used on what is there

- SIMPLIFIED, high
- Original: Letting an item go over the 3D world: terrain -> IsDropPossible / DropItem or the drop question; a character -> UseItemOnCharacter; other -> UseItemOnItem (FUN_100cbcb5).
- Client: dropped_out is used on what is under the pointer; nothing for the ground.
- A player notices: An item cannot be dropped on the ground by letting it go in the world.
- Where: `crates/mao-client/src/loot.rs:144`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): an item let go over a character is `N3Msg_UseItemOnCharacter` (the use message's action 0x20, captured), over another object `UseItemOnItem` as before, and over the ground the original's boxes by `N3Msg_IsDropPossible`'s answer (`dropitem.rs`): too far, "not a fixture, delete it instead?" for nearly every item, unique, no-drop, or the drop question, whose Yes sends `DropTemplateIIR_t`. That message was never captured (its bytes are the decompile's). Not ported: the tests that come after the fixture test, and anything inside, where there is no terrain to meet. `CLIENT_UI.md` "An item let go over the world".

#### `INV-017` Item stat effects: only some spell types read

- SIMPLIFIED, high
- Original: The original's event and spell tables apply every spell of every type.
- Client: Stat effects of worn items are found by scanning every byte offset for spell records of type 0xcf35, 0xcf14 and 0xcfb7; other spell types are not read.
- A player notices: Stat bonuses of worn items from other spell types are missing in the character sheet.
- Where: `crates/mao-formats/src/item.rs:35`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the byte scan is replaced by the wear list itself (`item::wear_spells`, event 14), every spell of it judged by the criteria evaluator and taken at the item's quality before it is judged. The entry's "other spell types" turned out not to exist: over the game's 120842 templates the wear lists' only spells that add to a shown stat are 0xcf35 and 0xcf14. What changes is that spells of an item's other lists (about 2600: use, vicinity, personal buffs) no longer count while it is worn, and criteria on more than the level are read. The sets of stored stats (0xcf22, 0xcf76, 0xcf93) are not applied. `CLIENT_UI.md` "What a worn item adds is its wear list".

#### `INV-018` Mail terminals come from a fixed list

- SIMPLIFIED, high
- Original: Not stated.
- Client: mailbox::KNOWN_TERMINALS (four entries from captures) stands in for announced terminals.
- A player notices: Mail works only at those terminals.
- Where: `crates/mao-client/src/mailbox.rs:18`; `docs/CLIENT_UI.md:137`; `crates/mao-client/src/net.rs:7881`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): No longer so, and was not when the catalog was made: mailbox::terminals reads the playfield's own dynel table (identity type 0xc773: 385 terminals in 203 playfields) and falls back on the four captured ones only for a playfield whose table has none. Found by the local model's first look at the open entries; checked in the code.

#### `INV-019` Item cells counted along rows and kept in one places.ini for all characters

- SIMPLIFIED, medium
- Original: Each item's grid cell is kept as a column and a row per container in the view's item_position_map (FUN_100ccefe / FUN_100cac66), in Inventory.xml, Bank.xml and Containers/Container_<type>x<instance>.xml.
- Client: A cell is one number counted along the rows (grids take as many columns as the window is wide); cells are kept in places.ini, one map for every character; nothing sent to the server.
- A player notices: Cell layouts change with window width and are shared across characters.
- Where: `crates/mao-client/src/places.rs:15`; `crates/mao-client/src/places.rs:13`; `docs/CLIENT_UI.md:3621`
- Decision: [ ] port  [ ] leave

#### `INV-020` Item dropped on a wear cell is sent without the original's drop handling

- SIMPLIFIED, medium
- Original: On a wear cell FUN_100e220f asks N3Msg_IsItemPossibleToWear first; if false nothing happens and the item stays in hand; on a cell that is not a slot the item's own placement (stat 0x58) is used; the pointer shows the "no" shape over cells that cannot take it (FUN_100e1ded).
- Client: Only labelled slot cells take a drop; the move is pushed straight away (refusal comes back from the server) and the pointer does not change.
- A player notices: A wrong drop empties the hand and prints a refusal instead of leaving the item in hand; no "no" pointer.
- Where: `crates/mao-client/src/invwindows.rs:516`
- Decision: [ ] port  [ ] leave

#### `INV-021` Loot windows keep the server's place order instead of sorting by name

- SIMPLIFIED, medium
- Original: A container not one's own (except reclaim), the 0xdeaa contract and trade lists are sorted by name with auto arrange on (FUN_10041258(1), FUN_100ce55d).
- Client: "The bank, corpses and chests are arranged as they were, in their places' order."
- A player notices: Loot lists are in server order, not alphabetical.
- Where: `crates/mao-client/src/loot.rs:20`
- Decision: [ ] port  [ ] leave

#### `INV-022` Right click sends Use; wear/take-off rule not in the window

- SIMPLIFIED, medium
- Original: The GUI calls N3Msg_UseItem(item,false); Gamecode (0x10028883) wears an item whose Can stat lacks the use bit, takes off a worn one into the first free place, and otherwise sends Use.
- Client: Every right click pushes events.uses; the window has no wear or take-off rule of its own.
- A player notices: Right clicking armour, weapons or implants depends on what the server does with a Use.
- Where: `crates/mao-client/src/invwindows.rs:583`
- Decision: [ ] port  [ ] leave

#### `INV-023` Spell texts: only listed spell types written, some English words fixed

- SIMPLIFIED, medium
- Original: N3Msg_ConvertSpellList writes each spell type it knows.
- Client: Writes the types listed in the file; other types write nothing; some lines ("Modify %s %d", "Cast ", "Taunt", "Spawn Item", "Root", "Pacify", "Fear", "Stun", "Charm") are fixed English.
- A player notices: Some item or nano spell lines are missing or not translated.
- Where: `crates/mao-client/src/gui/spells.rs:7`
- Decision: [ ] port  [ ] leave

#### `INV-024` City map: pending nodes dropped when their item changes place

- SIMPLIFIED, low
- Original: FUN_100e46fc rewrites a pending node's item when identity changes; FUN_100e4b49 removes the node only when the item is gone.
- Client: Pending nodes are matched to backpack items by slot and template; a node whose item left that slot is dropped, not re-pointed.
- A player notices: Moving a waiting building item may make its node vanish from the map.
- Where: `crates/mao-client/src/citymap.rs:192`
- Decision: [ ] port  [ ] leave

#### `INV-025` Deleted item leaves the window at once with a client line

- SIMPLIFIED, low
- Original: not stated
- Client: removes the item from the inventory window immediately and says "Item deleted"; a refused delete is not taken back (notes)
- A player notices: no wait for the server
- Where: `crates/mao-client/src/net.rs:8424`; `docs/CLIENT_UI.md:1862`
- Decision: [ ] port  [ ] leave

#### `INV-026` Drag and drop covers only part of the original's targets

- SIMPLIFIED, low (notes only)
- Original: DragObject_c drags items, nanos, actions onto bars, windows and other characters.
- Client: inventory and hotbar drag only.
- A player notices: some drops the original allows do nothing.
- Where: `docs/GUI_DLL_PARITY.md:50`
- Decision: [ ] port  [ ] leave

#### `INV-027` Inventory grid starts five columns wide

- SIMPLIFIED, low
- Original: No fixed column count; default places are three to a row for 21-place bags when no item_position_map exists.
- Client: GridView::new(ctx, 30, 5): a five-column default.
- A player notices: The inventory window opens with a different shape than the original.
- Where: `crates/mao-client/src/invwindows.rs:319`
- Decision: [ ] port  [ ] leave

#### `INV-028` Item action chunk read only when 2 to 9 actions fit whole

- SIMPLIFIED, low
- Original: Not stated.
- Client: The actions chunk is found by the words 22, 36 with a plausible count and ignored if any action is cut or has more than 40 criteria.
- A player notices: An item with a larger action chunk shows no requirements.
- Where: `crates/mao-formats/src/item.rs:404`
- Decision: [ ] port  [ ] leave

#### `INV-029` Item menu: Delete on wear pages, no Lock/Unlock, no Looter

- SIMPLIFIED, low
- Original: Menu (right button held 0.3 s): LookAtItem, CreateReference, DeleteItem (not on wear pages), Lock/Unlock Item on wear pages (EquipmentLocks), Looter submenu on a team corpse (FUN_100caf07).
- Client: Always the three entries Look At, Create Reference, Delete, on the inventory and the wear grids; no lock entries.
- A player notices: Worn items can be deleted from the menu and cannot be locked.
- Where: `crates/mao-client/src/invwindows.rs:310`
- Decision: [ ] port  [ ] leave

#### `INV-030` Item pages use the item's template stats for the reader

- SIMPLIFIED, low
- Original: N3Msg_GetSkill on the item object.
- Client: its template's stats.
- A player notices: per-item changes (charges used, modifications) may not appear on the page.
- Where: `crates/mao-client/src/gui/iteminfo.rs:2`
- Decision: [ ] port  [ ] leave

#### `INV-031` Mail and market deposit: a click takes the item out, not a drag

- SIMPLIFIED, low
- Original: An item dragged off a slot view leaves it (the mail/market item slots are drag sources).
- Client: A click on the item place of a new mail or of Deposit to Market empties it.
- A player notices: Removing the attached item needs a click, not a drag.
- Where: `crates/mao-client/src/mailmessage.rs:301`; `crates/mao-client/src/marketsend.rs:127`
- Decision: [ ] port  [ ] leave

#### `INV-032` Perk criteria: social-armour flag not tracked

- SIMPLIFIED, low
- Original: CanWear refuses a perk when the character's +0x138 bit 1 (social armour) is set unless flags 0x10000000 or equipment class 3 or 5.
- Client: Treated as clear.
- A player notices: Perks may show as trainable while social armour is on.
- Where: `crates/mao-client/src/perks.rs:181`
- Decision: [ ] port  [ ] leave

#### `INV-033` Requirements: an unreported stat reads as 0

- SIMPLIFIED, low
- Original: The stat is read from the holder through GetStat.
- Client: A stat the server has not sent reads as 0 for the client's own character.
- A player notices: A requirement on an unsent stat may pass or fail by that default.
- Where: `crates/mao-client/src/requirements.rs:183`
- Decision: [ ] port  [ ] leave

#### `INV-034` Tooltip description cut to 420 characters

- SIMPLIFIED, low
- Original: The tooltip shows the description (no length cut stated).
- Client: The description is cut at 420 characters (300 in container windows) and the "When worn" list at 10 effects.
- A player notices: Long item descriptions and effect lists are truncated in tips.
- Where: `crates/mao-client/src/gear.rs:220`
- Decision: [ ] port  [ ] leave

#### `INV-035` Tradeskill result named by its low template

- SIMPLIFIED, low
- Original: The result is named by the item the chosen quality interpolates to. TradeSkillView_c has two GlobalSignals (+0x1d4, +0x1d8).
- Client: Names the result by its low template's name for every quality; the two signals were not traced.
- A player notices: The "Result will be QL n name" line and the result slot's name show the lowest quality's item name, which may differ at higher quality.
- Where: `crates/mao-client/src/tradeskill.rs:24`; `docs/GUI_DLL_PARITY.md:90`
- Decision: [ ] port  [ ] leave

#### `INV-036` Wear refusal ignores the page

- SIMPLIFIED, low
- Original: Tests the place's bit with the page.
- Client: wear_refusal tests the place's bit (stat 298, slot = place % 16) without the page.
- A player notices: A drag onto a place of the wrong page is not refused locally.
- Where: `docs/CLIENT_UI.md:3644`; `crates/mao-client/src/net.rs:1111`
- Decision: [ ] port  [ ] leave

#### `INV-037` Worn items' wear lists run on the client

- SIMPLIFIED, low
- Original: not stated
- Client: Temporary stat sets of worn items are run by the client because the server sends no list for them.
- A player notices: Scale stats are set locally.
- Where: `crates/mao-client/src/net.rs:1515`
- Decision: [ ] port  [ ] leave

### Inventory and items: the original was not read: inferred or measured (8)

#### `INV-038` Using a bag again closes/shows its window locally, sending nothing

- GUESS, medium
- Original: The GUI sends the use again (FUN_100cae1b); the server answers with item action 0x66 which closes the bag window; picking a bag up also closes it (CloseContainer). Where the original closes a backpack on its second use was not found.
- Client: using a bag whose window is up closes the window, one whose window was closed shows it again; nothing is sent to the server
- A player notices: a second use of an open backpack closes its window instead of re-sending the use
- Where: `crates/mao-client/src/ui.rs:1392`; `crates/mao-client/src/ui.rs:1405`; `docs/CLIENT_UI.md:3332`
- Decision: [ ] port  [ ] leave

#### `INV-039` Given backpack recognised by capture bytes

- GUESS, low
- Original: Not stated.
- Client: backpack_given recognises a given backpack by two bytes 0x71 0x6f read from captures.
- A player notices: A new character's pack may be missed.
- Where: `crates/mao-client/src/net.rs:731`; `docs/CLIENT_UI.md:4593`
- Decision: [ ] port  [ ] leave

#### `INV-040` Hotbar item use picks the first stack, backpack before worn

- GUESS, low
- Original: Not stated.
- Client: An item on a hotbar fires from the first stack found, backpack before worn; says "<name> is not in your inventory" if none.
- A player notices: Which stack is used and the message wording may differ.
- Where: `crates/mao-client/src/viewer.rs:951`
- Decision: [ ] port  [ ] leave

#### `INV-041` Inventory list end found by a heuristic

- GUESS, low
- Original: Reads the world-entry message's tables in order.
- Client: Items are read until an entry "does not look like an item" (slot above 0xff, odd flags or no id); the stat tables that follow are not read here.
- A player notices: An item with unusual flags would cut the inventory short.
- Where: `crates/mao-net/src/inventory.rs:58`
- Decision: [ ] port  [ ] leave

#### `INV-042` Item template parsed by heuristics, not the original's reader

- GUESS, low
- Original: The item class reader reads fields in order.
- Client: Name/description length word searched as the first word whose halves measure printable text (name up to 80, description up to 4000); trailing tables found by scanning for chunk keys.
- A player notices: An item with an unusual record may show no name or no actions.
- Where: `crates/mao-formats/src/item.rs:442`
- Decision: [ ] port  [ ] leave

#### `INV-043` Place of an item moved or looted into a container is a guess

- GUESS, low
- Original: The server's own choice of place was not found.
- Client: An item taken from a corpse goes to the first free backpack place; into an open bag, its first free place; into a chest, the next place after the highest; into the bank, the first free place (as aosharp does).
- A player notices: Item may sit in a different slot than on the server until refreshed.
- Where: `crates/mao-client/src/net.rs:3021`; `crates/mao-client/src/net.rs:3048`; `docs/CLIENT_UI.md:1774`; `docs/CLIENT_UI.md:1776`
- Decision: [ ] port  [ ] leave

#### `INV-044` Slot names 1, 2, 8, 29, 46+ are guesses

- GUESS, low (notes only)
- Original: Not stated.
- Client: Slot names read off one capture; those slots guessed.
- A player notices: Some slot names may be wrong.
- Where: `docs/GAME_WINDOWS.md:31`
- Decision: [ ] port  [ ] leave

#### `INV-045` Worn places rule and place 15 are inferred

- GUESS, low
- Original: Not stated.
- Client: Place 15 counts as worn because "a Dreadloch booster sat there".
- A player notices: Bonuses from place 15 items may be wrong.
- Where: `crates/mao-client/src/derived.rs:15`
- Decision: [ ] port  [ ] leave

### Inventory and items: a guard or fallback the original lacks (2)

#### `INV-046` Dropped item waited for up to five seconds, then forgotten

- SAFETY, low
- Original: The place is remembered in +0x150 and used when the item arrives (FUN_100ce25d); no time-out stated.
- Client: The awaited cell expires after 5 s.
- A player notices: With a slow server the item appears in the first free cell instead of the drop cell.
- Where: `crates/mao-client/src/invwindows.rs:364`
- Decision: [ ] port  [ ] leave

#### `INV-047` Stat names fall back to the client's own table

- SAFETY, low
- Original: stat names come from text database category 2003.
- Client: without the game's text it uses the client's own `skills::stat_name`.
- A player notices: only without game files.
- Where: `crates/mao-client/src/gui/criteria.rs:44`
- Decision: [ ] port  [ ] leave

### Inventory and items: the original does it, the client does not (31)

#### `INV-048` Backpack stacks cannot be joined or split inside a bag window

- MISSING, high
- Original: Dropping a stack on a stack of the same container sends JoinItems; split-count drag onto an empty cell sends SplitItem (FUN_100cc855).
- Client: Two like stacks change cells like any two items.
- A player notices: Stacks cannot be merged or split inside a bag.
- Where: `crates/mao-client/src/loot.rs:17`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): a stack let go on a like stack of its backpack is joined (`N3Msg_JoinItems`), Ctrl+click splits (the inventory's count box) into an empty cell of the same backpack (`N3Msg_SplitItem`, the client's list first); a bag's item goes out as `(0x6b, handle << 16 | place)`. The captures have joins and a split in the inventory only: the server's answer inside a bag, and its place for the new stack, are Gamecode's reading and want one live try. Not done: the bank's stacks.

#### `INV-049` Item use sent at once instead of after the item timer

- MISSING, high
- Original: Uses the item once the first phase of the item timer is over (FUN_1007ccb3 / FUN_1003bb5a).
- Client: Sends the use at once while showing the timer.
- A player notices: Item takes effect before its timer bar finishes.
- Where: `crates/mao-client/src/net.rs:1365`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the line above is a misreading: the original also sends the use at the click (the engine sends the command in that frame), and it is the server that waits the ItemDelay (in the captures a one-second stim works 1.1 s after the use is sent). Sending late would have doubled every delay, so that was not ported. What did differ is: the timer bar now starts with the server's answer that allows the use (FUN_1007ccb3), not at the click. Not ported: the silent refusal of a second use while a timer runs, and `CharacterAction` 0x92, which the client takes for the end of this timer and the original for the equip timer's.

#### `INV-050` No item menu in container windows

- MISSING, high
- Original: Every container list has the held-right-button menu: Look At, Create Reference, Delete (not on read-only), Looter submenu for a corpse in a team (FUN_100caf07).
- Client: Bag, bank, corpse and chest windows have a tip and Shift+click info only; no menu, no Looter, no Set Loot Access.
- A player notices: No Create Reference, Delete or loot-rights choice from a container window.
- Where: `crates/mao-client/src/loot.rs:333`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the container windows have the item's menu by the held right button: Look at Item, Create Reference, Delete Item (in every view that is not read-only or a wear page, a corpse too: more than the line above says; not on the organization's contract here), and on a corpse for one in a team the Looter submenu (`N3Msg_SetLootAccess`). Delete sends the item's own identity and the server's answer takes it out; a capture has that for a backpack. The bank's, the overflow's and a corpse's deletes and every loot access are the code's alone. Create Reference makes a chat link as the inventory's menu does (the original puts the item on the reference board), and a plain right click still takes the item (the original uses a backpack's item and does nothing on a corpse): both left as they were.

#### `INV-051` Client does not send bank use (0x6a), corpse slot action (0x8e), pet command (0x80)

- MISSING, medium (notes only)
- Original: N3Msg_UseItem's bank branch sends 0x6a; corpse slot actions send 0x8e (with progress); pet commands 0x80.
- Client: Listed under "Client messages the original sends that this client does not"; no send of 0x6a/0x8e found in net.rs.
- A player notices: Bank use, corpse slot progress, pet commands may not work as the original's.
- Where: `docs/CAPTURE_GAPS.md:302`
- Decision: [ ] port  [ ] leave

#### `INV-052` Inventory behaviours left from the original reading (ground drop, use on a character, and others)

- MISSING, medium (notes only)
- Original: dropping on the ground, using on a character, a bag window's close sends the use again, Ctrl+right click adds to any trade, an unfit wear place leaves the item in hand, a corpse right click and "Take all", windows' own menus (List Mode, Auto Arrange, Labels On Top, Unlock All Items), tooltip and count look
- Client: none done; the client only hides the bag window. (Ground drop and use-on-a-character were not found in the code; several other parts are separate entries here.)
- A player notices: several inventory behaviours differ
- Where: `docs/CLIENT_UI.md:3640`
- Decision: [ ] port  [ ] leave

#### `INV-053` Item effects with requirements other than level are left out

- MISSING, medium
- Original: Not stated; the original tests each requirement.
- Client: modifiers_at keeps a record only when its requirements are level tests (stat 0x36) with operators 1 or 2; any other requirement or operator drops the effect. (Notes say computed stats still matched the original in three checked characters.)
- A player notices: Effects that depend on another stat never count.
- Where: `crates/mao-formats/src/item.rs:61`; `docs/STAT_ENGINE.md:76`
- Decision: [ ] port  [ ] leave

#### `INV-054` Item pages: shop price, nano radius, armour/implant/nano sections, fixtures, towers, buildings missing

- MISSING, medium
- Original: Info view's item pages (FUN_10031fda shop price from a vendor's stat 74; N3Msg_GetFormularRadius nano radius; FUN_1003826b and FUN_10038586 armour and implant sections; fixtures, towers, buildings).
- Client: "Not yet" those.
- A player notices: Those item/nano pages are shorter or lack lines.
- Where: `crates/mao-client/src/gui/iteminfo.rs:39`; `docs/CLIENT_UI.md:2680`
- Decision: [ ] port  [ ] leave

#### `INV-055` Player trade: partner's items not named; no out-of-range check

- MISSING, medium (notes only)
- Original: Feedback_TargetOutsideRangeForTrade; partner's items named.
- Client: Not named (never captured); no range check found in code.
- A player notices: Partner's items may appear unnamed or not at all.
- Where: `docs/GUI_DLL_PARITY.md:89`; `docs/ARETE_CAPTURE.md:121`
- Decision: [ ] port  [ ] leave

#### `INV-056` Requirement checks: only the client's own character, no apply-on target/user/secondary

- MISSING, medium
- Original: "On Target", "On User" and "On Secondary Item" pick another dynel (FUN_10001fba cases 3, 2, 4) and read its stats; FUN_100065d8 checks below/above on any character.
- Client: Only "On Self" is known; the others give unknown. Below/above comparisons (1, 2, 0x82..0x85) check only the client's own character and pass for anyone else.
- A player notices: Requirements on a target, user or second item cannot be checked; others' effects with criteria pass by default.
- Where: `crates/mao-client/src/requirements.rs:193`; `crates/mao-client/src/requirements.rs:21`; `docs/CAPTURE_GAPS.md:68`
- Decision: [ ] port  [ ] leave

#### `INV-057` Requirement operators needing world state give "unknown"

- MISSING, medium
- Original: FUN_100065d8 decides every operator: timers, items in slots, nanos, perk locks, teams, organizations, playfields, positions, vehicles.
- Client: Those operators give "unknown"; the caller decides (tips show no met/not-met colour or read "condition N on stat value"; perk checks count an unknown operator as failing). The stat-against-stat operators 0x82-0x85 and has-perk are now ported (requirements.rs), so that part of the older notes is out of date.
- A player notices: Some item, perk and spell requirements are not enforced or shown (no green/red mark, perks possibly red or untrainable wrongly).
- Where: `crates/mao-client/src/requirements.rs:24`; `crates/mao-client/src/gear.rs:306`; `crates/mao-client/src/perks.rs:172`; `docs/ORIGINAL_AUDIT.md:114`; `docs/CAPTURE_GAPS.md:88`
- Decision: [ ] port  [ ] leave

#### `INV-058` Requirement texts: special criteria, breed/gender names, apply-on reordering not ported

- MISSING, medium
- Original: ConvertCriteria writes special criteria (items, perks, pets, places, flag lists), breed and gender names; N3Msg_CriteriaListToString reorders apply-on criteria.
- Client: Special criteria are written as a plain stat criterion; no breed or gender names; no reordering.
- A player notices: Some item requirement lines read differently or oddly.
- Where: `crates/mao-client/src/gui/criteria.rs:12`
- Decision: [ ] port  [ ] leave

#### `INV-059` Bag cell placement: no join or split inside a bag

- MISSING, low
- Original: Not stated (the original joins/splits stacks within a container, FUN_100cc855).
- Client: Join is sent only for inventory items (0x68); an inventory item put into a bag takes the first free cell.
- A player notices: Stacking inside bags unsupported.
- Where: `docs/CLIENT_UI.md:3625`; `crates/mao-client/src/net.rs:8412`
- Decision: [ ] port  [ ] leave

#### `INV-060` Bank item dropped on a bag window: only the inventory rectangle is tested

- MISSING, low
- Original: Out of the bank only to the inventory (message ItemsDraggedFromBankCanOnlyBeDroppedToBank); into a bag at the cell clicked (FUN_100cc855).
- Client: Let go over the inventory window it is taken; no message when let go on a bag.
- A player notices: No refusal text for a bank item dropped elsewhere.
- Where: `crates/mao-client/src/loot.rs:382`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): read (GUI FUN_100cc855: text 10003 ItemsDraggedFromBankCanOnlyBeDroppedToBank as a System chat line, on a bag, chest or corpse window or a wear cell); not ported, it wants a drag to try.

#### `INV-061` City map: waiting building item not locked in the inventory

- MISSING, low
- Original: Dropping a building item on the map marks it in use on InventoryItemLocker_c (FUN_1003d103); it cannot be picked up, clicked or dropped until the node is removed.
- Client: The item is only not taken a second time by the map; the inventory does not lock it.
- A player notices: The item can still be moved or used in the backpack while it waits on the map.
- Where: `crates/mao-client/src/cityterminal.rs:33`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done with WIN-110: the waiting item shows the lock's picture and is not taken up, split, traded or used; a drop on it is told (DroppedItemWasLocked). The locked places are the map's waiting ones each frame.

#### `INV-062` Closing a bag window does not tell the server

- MISSING, low
- Original: The window's close sends N3Msg_UseItem(container,false) again, which is how the server is told the bag is closed (FUN_100ca891).
- Client: Nothing is sent when a bag window closes ("the original's message was not identified"); the loot window's close sends nothing either (docs/CLIENT_UI.md:1774).
- A player notices: The server may still think the bag is open.
- Where: `crates/mao-client/src/loot.rs:20`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): confirmed (GUI FUN_100ca891 uses the container again; the server answers with the container's close action, 2049527c 0x66, and every use flips open and closed); a faithful port has to follow the server's open and close actions and wants a live session.

#### `INV-063` Inventory item not shown at the cell it was dropped on in a bag window

- MISSING, low
- Original: The drop remembers the place (+0x150) and the new item appears there (FUN_100cc855, FUN_100ce25d).
- Client: Not ported for bag windows.
- A player notices: An item put into a bag appears at the first free cell, not under the pointer.
- Where: `crates/mao-client/src/loot.rs:18`
- Decision: [ ] port  [ ] leave

#### `INV-064` Item page "Fabric" line only in the original's debug mode, not built

- MISSING, low
- Original: Armour's "Fabric" line shows in a debug input mode (FUN_1003826b).
- Client: Not built.
- A player notices: None for normal play.
- Where: `crates/mao-client/src/gui/iteminfo.rs:573`
- Decision: [ ] port  [ ] leave

#### `INV-065` Item pages: stats not interpolated between the low and high templates

- MISSING, low
- Original: the page shows the item's stats at its quality, interpolated between the low and high templates.
- Client: uses the low template's stats (the requirements are interpolated).
- A player notices: stat values on a page at a quality between the templates may be the low template's.
- Where: `crates/mao-client/src/gui/iteminfo.rs:41`
- Decision: [x] port  [ ] leave
- Looked at, left (2026-11-18, overnight): read (FUN_100cc5a1, FUN_100cbfc8, FUN_100cb909: 27 stats interpolated, the price by a curve, the high template whole at its level); the page is unchanged. The rounding fault it showed in the client's effects and requirements is mended (derived::between).
- Done (2026-11-18, overnight, later the same night): the page and the inventory's names take the item as it is at its quality (derived::item_at: the high template at its level, 26 stats and the price between both); the page's modifier and skill lists still read the low template.

#### `INV-066` Locks not done: item-own wait, nano points, locked-special lines

- MISSING, low (notes only)
- Original: Not stated.
- Client: An item with a wait of its own that locks no skill; a program not shown locked for want of nano points; the locked-special lines ("Action is not available.") not looked into.
- A player notices: Some locked states not shown.
- Where: `docs/CLIENT_UI.md:4882`
- Decision: [ ] port  [ ] leave

#### `INV-067` Mail: total cost not shown

- MISSING, low (notes only)
- Original: Mail window shows a total cost.
- Client: The total cost is left to the server (C.O.D. itself is now ported in mailmessage.rs, so only this part of the note is left; no total-cost code found).
- A player notices: No total cost shown when sending mail.
- Where: `docs/CLIENT_UI.md:141`; `crates/mao-client/src/mailbox.rs:3`
- Decision: [ ] port  [ ] leave

#### `INV-068` Organization contract items cannot be taken or put in

- MISSING, low
- Original: The contract is an ordinary container; a click or drop is handled by the generic handlers (not read in detail by the client).
- Client: The contract is only looked at; takes and puts are filtered out; a click does nothing.
- A player notices: Nothing can be moved in or out of an organization contract window.
- Where: `crates/mao-client/src/loot.rs:460`; `docs/LIVE_TEST_LIST.md:511`
- Decision: [ ] port  [ ] leave

#### `INV-069` Overflow window: unique-item refusal, title from game text, list mode

- MISSING, low (notes only)
- Original: refusal of a unique item one already has; window title from the game's own text; list mode
- Client: refusal left to the server; no title from game text; no list mode
- A player notices: minor differences in the overflow window
- Where: `docs/CLIENT_UI.md:3700`
- Decision: [ ] port  [ ] leave

#### `INV-070` Points shops: cost unlabelled

- MISSING, low (notes only)
- Original: The money shown at such a shop is points, not credits.
- Client: The cost is drawn as a plain number with nothing saying it is points (no code for a label found).
- A player notices: Points vendors show a bare number.
- Where: `docs/CLIENT_UI.md:4387`; `docs/LIVE_TEST_LIST.md:268`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): nothing to port: the original's shop shows the same Credits label at a points shop (TradeGUI.xml's ShopBuy view); the notes that said otherwise are wrong.

#### `INV-071` Requirement evaluator's flag arguments and feedback lines not ported

- MISSING, low
- Original: The evaluator takes a level-compare flag (ebp+0x18), a pass-most flag (ebp+0x28) and a feedback flag that writes lines to the player.
- Client: None of the three is ported; both callers pass 0.
- A player notices: No requirement feedback lines come from the evaluator.
- Where: `crates/mao-client/src/requirements.rs:28`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): the evaluator's feedback lines are ported for below, above and the equalities of breed, side, sex and profession (requirements::evaluate_told, commit c9ff224), used by the wear and wield checks; the other operators' lines and the two flag arguments (level compare, pass-most) are not, and no ported caller sets those flags.

#### `INV-072` Server CharacterAction 0x6e (end of an action's progress) not handled

- MISSING, low (notes only)
- Original: Case 32 (0x1005ddf4) FUN_10046e7a -> FUN_1003f61c: the end of an action's progress.
- Client: Not listed as handled; no handler found.
- A player notices: No progress end on corpse/chest actions.
- Where: `docs/CAPTURE_GAPS.md:288`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): the note is a misreading: 0x6e closes the open reclaim container (0xdeae; 0x6a the bank's), FUN_10046e7a; no timer ends. Nothing to port until there is a reclaim window.

#### `INV-073` Shops: list configs not read

- MISSING, low (notes only)
- Original: shop_listview_config kept columns; ShopBuy sold list.
- Client: Not read; no sold list (shop.rs notes nothing is sold for points).
- A player notices: Shop lists not in the saved layout.
- Where: `docs/GUI_DLL_PARITY.md:88`
- Decision: [ ] port  [ ] leave

#### `INV-074` Some wear refusals left to the server

- MISSING, low
- Original: Refuses locally with feedback: implants need bit 0x10 of stat 224 (FUN_10044dd8, Feedback_AccessingImplants); weapons on places 9 to 14 need stat 45 deck places (FUN_10046f26, Feedback_ThisItemRequiresComputerDeck); social armour against other armour (FUN_10081d5f, FUN_10080aff); an item not one's own (Feedback_ItemNotOwnedByYou).
- Client: None of these four is checked; the server decides.
- A player notices: No local refusal message for these cases.
- Where: `crates/mao-client/src/net.rs:1095`; `crates/mao-client/src/net.rs:1097`; `crates/mao-client/src/net.rs:1099`; `crates/mao-client/src/net.rs:1101`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): the deck places rule is ported (a unit on places 9 to 14 needs that many deck places, Feedback_ThisItemRequiresComputerDeck), and with it the belt's (Feedback_DeckMustBeEmptyBeforeUnequipped); the implant's surgery bit, social armour and an item not one's own are still the server's.

#### `INV-075` Wear check leaves out per-tab locks, implant rule, deck places, social armour

- MISSING, low
- Original: Gamecode N3Msg_IsItemPossibleToWear also checks EquipmentLocks per tab, implants (stat 224 bit 0x10; Feedback_AccessingImplants), deck places (stat 45; Feedback_ThisItemRequiresComputerDeck), social armour rules (stat 702) and an item that is not one's own
- Client: only the global LockEquipment is checked; the wear window has no per-tab locks; the rest is knowingly left to the server (it refuses and the client takes the move back)
- A player notices: wear refusals come from the server after a round trip; a single wear tab cannot be locked
- Where: `crates/mao-client/src/net.rs:1014`; `crates/mao-client/src/net.rs:1085`; `docs/CLIENT_UI.md:4315`; `docs/CLIENT_UI.md:1573`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): the deck places rule is ported (a unit on places 9 to 14 needs that many deck places, Feedback_ThisItemRequiresComputerDeck), and with it the belt's (Feedback_DeckMustBeEmptyBeforeUnequipped); the implant's surgery bit, social armour and an item not one's own are still the server's. The wear window's single tab locks are not done.

#### `INV-076` Wielded weapon: mirrored left-hand side and social places not ported

- MISSING, low
- Original: Gamecode FUN_1009cda2 attaches weapon meshes by wear place
- Client: the other side's mirroring for a left-hand weapon with the can-flag 0x800000 and the social places 0x3d and 0x3f are not ported
- A player notices: off-hand and social-slot weapons are not drawn as in the original
- Where: `crates/mao-client/src/net.rs:5403`; `docs/ORIGINAL_AUDIT.md:24`; `docs/CLIENT_UI.md:714`
- Decision: [ ] port  [ ] leave

#### `INV-077` Worn items' temporary stat sets with criteria are left out

- MISSING, low
- Original: not stated
- Client: worn_sets keeps only spells with no criteria; one with criteria is ignored
- A player notices: such worn stat changes never apply
- Where: `crates/mao-formats/src/item.rs:216`
- Decision: [ ] port  [ ] leave

#### `INV-078` Worn meshes: hiding modes and loading stand-in not ported

- MISSING, low (notes only)
- Original: FUN_10073517 hiding modes (+0x74); a bit-2 mesh in place while it loads.
- Client: Not ported.
- A player notices: Some worn items hide/show differently; no stand-in while loading.
- Where: `docs/ORIGINAL_AUDIT.md:39`
- Decision: [ ] port  [ ] leave

### Inventory and items: the client has it, the original does not (6)

#### `INV-079` Auto loot of fresh corpses

- ADDITION, medium
- Original: Not stated.
- Client: With the auto_loot setting on (off by default) a corpse that just opened is emptied automatically.
- A player notices: Corpses loot themselves when the option is on.
- Where: `crates/mao-client/src/net.rs:8436`; `crates/mao-client/src/settings.rs:548`; `docs/CLIENT_UI.md:1906`
- Decision: [ ] keep  [ ] remove

#### `INV-080` Container and overflow windows have a "Take all" button

- ADDITION, medium
- Original: GUI.dll has no "Take all" / "Loot all" button or handler (INVENTORY_ORIGINAL.md section 4); the overflow is an ordinary non-own container window with no "Take all"; a click takes one item.
- Client: Corpses, chests and the overflow list (anything not one's own, not bank or organization) have a "Take all" button.
- A player notices: All items can be looted with one click.
- Where: `crates/mao-client/src/loot.rs:276`; `docs/CLIENT_UI.md:3701`
- Decision: [ ] keep  [ ] remove

#### `INV-081` Tradeskill guide made from item descriptions

- ADDITION, medium
- Original: No such guide; which items combine is the server's to say a pair at a time.
- Client: Guide built by reading item descriptions ("it will make", "to create" ...), linking item names into steps; no skill requirements.
- A player notices: A tradeskill guide window the original lacks, complete only as far as descriptions say.
- Where: `crates/mao-formats/src/crafting.rs:29`; `crates/mao-client/src/craftguide.rs:1`; `docs/CLIENT_UI.md:895`; `docs/CLIENT_UI.md:908`; `docs/LIVE_TEST_LIST.md:554`
- Decision: [ ] keep  [ ] remove

#### `INV-082` Backpack count heading

- ADDITION, low (notes only)
- Original: Not stated.
- Client: "Backpack 12/30" heading showing used places (not found in code).
- A player notices: Extra count.
- Where: `docs/CLIENT_UI.md:1957`
- Decision: [ ] keep  [ ] remove

#### `INV-083` Item finder (/find) is the client's own tool

- ADDITION, low
- Original: The original has nothing like it; players kept lists by hand or with outside programs.
- Client: A window (/find <part of a name>) searching every character played with this client by words in an item's name, saying which character, where (worn, inventory, bag, bank) and how old the knowledge is; clicking a result opens the item's description. "Look into my bags" uses each unseen bag of the played character half a second apart without a window; the character's items are written to disk every few seconds. It cannot see bags never looked into, a bank never seen, a bag inside the bank, or characters not yet played with this client.
- A player notices: A player can look up where an item is, even on a character not logged in; bags are opened silently on request.
- Where: `crates/mao-client/src/itemindex.rs:2`; `crates/mao-client/src/itemindex.rs:14`; `crates/mao-client/src/net.rs:8010`; `docs/CLIENT_UI.md:949`; `docs/CLIENT_UI.md:962`; `docs/LIVE_TEST_LIST.md:586`
- Decision: [ ] keep  [ ] remove

#### `INV-084` A swap by right click is judged without the item that comes off

- ADDITION, medium (added by hand on 2026-11-18, at the user's wish; the count in the file's head is one more for it)
- Original: N3Msg_UseItem asks N3Msg_IsItemPossibleToWear with the character as it stands, the old item still on; it then takes the old item off and wears the new one, and the server may refuse the new one for want of what the old one gave, which leaves the old item in the backpack.
- Client: When the wear place is taken, the new item's requirements are judged on the shown values less the effects of the item that will come off (derived::modifiers_without, net.rs Stripped); a no ends the use with the game's line and nothing moves.
- A player notices: A swap that would strand the old item is refused at the click; a swap the original would have tried (and the server refused) is not tried.
- Where: `crates/mao-client/src/net.rs` (`World::wear_refusal`, the use of an item that is worn instead); `crates/mao-client/src/derived.rs` (`modifiers_without`); `docs/CLIENT_UI.md` ("An item is judged before anything is taken off")
- Decision: [ ] keep  [ ] remove

## Nano programs

### Nano programs: the client knowingly does otherwise (3)

#### `NAN-001` Target NCU windows are turned on with /setoption

- OWN-RULE, medium
- Original: The options are in the control center's settings (both off in the game's settings).
- Client: Since nothing in the game's files offers the options, `/setoption ShowFightTargetNCU on` turns a window on.
- A player notices: The target NCU windows are reached by a typed command.
- Where: `crates/mao-client/src/ui.rs:1266`; `docs/CLIENT_UI.md:424`
- Decision: [ ] port  [ ] leave

#### `NAN-002` Cast effect lasts a fixed time (1.2 s at someone, 1.95 s at oneself)

- OWN-RULE, low
- Original: The effect lasts as long as the cast.
- Client: Takes the cast to last as long as the casting animation it plays (1.2 s at someone, 1.95 s at oneself).
- A player notices: Effect length can differ from the real cast time.
- Where: `crates/mao-client/src/effects.rs:536`; `crates/mao-client/src/net.rs:9671`; `docs/CLIENT_UI.md:2309`; `docs/ORIGINAL_AUDIT.md:267`; `crates/mao-client/src/effects.rs:535`; `crates/mao-client/src/effects.rs:587`
- Decision: [ ] port  [ ] leave

#### `NAN-003` Repeated aura self-casts are silent

- OWN-RULE, low
- Original: not stated
- Client: A self-cast the server repeats within 10 minutes (or while the nano runs) says nothing, shows nothing and plays no animation unless the player just asked; "built, not confirmed" with the Keeper.
- A player notices: Repeated aura casts are silent.
- Where: `crates/mao-client/src/net.rs:2348`; `docs/ORIGINAL_UI_PLAN.md:36`
- Decision: [ ] port  [ ] leave

### Nano programs: an approximation or a stand-in (24)

#### `NAN-004` NCU memory in use is summed by the client

- SIMPLIFIED, medium
- Original: The NCU in use is stat 180 (title "NCU (%u/%u)" from stats 180 and 181, FUN_100d7689); the note says the original works it out as the sum of the running nanos' quality stat.
- Client: The server does not send stat 180 for one's own character, so the running nanos' costs are summed for the strip and the NCU window; a missing stat 181 reads 0.
- A player notices: The NCU figure may differ from the server's when a nano is missing from the list.
- Where: `crates/mao-client/src/ui.rs:1088`; `crates/mao-client/src/ncu.rs:244`; `docs/CLIENT_UI.md:1695`
- Decision: [ ] port  [ ] leave

#### `NAN-005` Others' nano times are the client's estimate, counted from first announcement

- SIMPLIFIED, medium
- Original: not stated
- Client: The buff message carries no time, so a nano announced without its time is shown with its whole duration (item stat 8) counted from when it was first seen.
- A player notices: Time left of another character's buff in the target NCU window is wrong for a nano not seen being cast.
- Where: `crates/mao-client/src/net.rs:1176`; `crates/mao-client/src/net.rs:9535`; `docs/CLIENT_UI.md:429`; `docs/ORIGINAL_AUDIT.md:108`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): another character's programs have the time the server told (0x62's, or the update's list), in hundredths of a second as it comes; no estimate from a first sighting any more.

#### `NAN-006` Spell criteria the client cannot decide count as failing

- SIMPLIFIED, medium
- Original: FUN_100065d8 decides every criterion; vehicle (ChangeMesh) criteria are tested in full.
- Client: A criterion giving "unknown" counts as failed and spends a hit; for vehicle programs only breed, sex and mesh are known, every other stat compares false.
- A player notices: Spells with criteria on stats the client lacks do not show (a vehicle program keyed on another stat would not put the body on the vehicle).
- Where: `crates/mao-client/src/spelllist.rs:186`; `crates/mao-formats/src/item.rs:156`
- Decision: [ ] port  [ ] leave

#### `NAN-007` Auras limited to six per character; others' body fixed to normal fatness

- SIMPLIFIED, low
- Original: not stated
- Client: At most 6 program auras per character are drawn; the ring's body uses fatness 1 for everyone but the own character.
- A player notices: Other characters' auras may not fit their body or may be cut off.
- Where: `crates/mao-client/src/net.rs:9635`; `crates/mao-client/src/net.rs:9626`
- Decision: [ ] port  [ ] leave

#### `NAN-008` Buff pulse retry wait is a hash, not the original's random number

- SIMPLIFIED, low
- Original: A buff that finds the gate shut waits rand() / 16384 (0 to 2 s).
- Client: A fixed hash of character, nano and try gives the 0 to 2 s wait.
- A player notices: Pulse timing is deterministic rather than random.
- Where: `crates/mao-client/src/pulses.rs:37`
- Decision: [ ] port  [ ] leave

#### `NAN-009` Buff ring anchors from a standing-body table, not the posed skeleton

- SIMPLIFIED, low (notes only)
- Original: Placeholder word 7 names a body node (FUN_10105e6d tables).
- Client: Places the ring from a table of where each node sits on a standing body; posed skeleton bones would place it exactly, animation included.
- A player notices: Rings do not follow animation.
- Where: `docs/CLIENT_UI.md:2086`
- Decision: [ ] port  [ ] leave

#### `NAN-010` Buff ring placeholders' frame step taken at 30 a second

- SIMPLIFIED, low
- Original: Children's clocks follow the frame rate.
- Client: Taken at 30 a second.
- A player notices: Ring step timing is fixed.
- Where: `crates/mao-client/src/effects.rs:119`; `docs/ORIGINAL_AUDIT.md:268`
- Decision: [ ] port  [ ] leave

#### `NAN-011` Buff ring: attach height guessed and shield stood in by a climbing ring

- SIMPLIFIED, low (notes only)
- Original: _GfxControlShield_t draws the character's CAT mesh again with a material; attach point 1006 is the head bone.
- Client: Attach point placed 2.1 m above the feet (a guess), bone not resolved; shield drawn as a ring climbing the legs every 20 s; the second mesh pass with additive band shader left to do.
- A player notices: Buff visuals differ from the original's.
- Where: `docs/CLIENT_UI.md:2082`
- Decision: [ ] port  [ ] leave

#### `NAN-012` Buff rings laid level; stat-360 body scale of the offset not applied

- SIMPLIFIED, low (notes only)
- Original: The ring tilts with the bone's own frame; the offset is scaled by stat 360.
- Client: Lays the rings level; no stat-360 scale.
- A player notices: Rings do not tilt with limbs.
- Where: `docs/ORIGINAL_AUDIT.md:268`; `docs/CLIENT_UI.md:2305`
- Decision: [ ] port  [ ] leave

#### `NAN-013` Cancelled nano leaves the window at once

- SIMPLIFIED, low
- Original: not stated
- Client: The nano is removed from the running list and NCU window and its stat bonuses are taken off at once, with a client line "Nano cancelled"; the server's update is not waited for.
- A player notices: The program disappears immediately; the display can run ahead of the server.
- Where: `crates/mao-client/src/net.rs:8392`; `crates/mao-client/src/net.rs:8401`; `docs/CLIENT_UI.md:1888`
- Decision: [ ] port  [ ] leave

#### `NAN-014` Cast and tracer effects use fixed heights

- SIMPLIFIED, low
- Original: not stated
- Client: A tracer goes from 1.1 m above the caster's feet to 1.2 m above the target's, whatever the body size.
- A player notices: Tracers always start and end at the same heights.
- Where: `crates/mao-client/src/net.rs:9705`
- Decision: [ ] port  [ ] leave

#### `NAN-015` Cast animation is only self or directed

- SIMPLIFIED, low
- Original: not stated (the library also has spell-gen, spell-sys, spell-self_02, unarmed-spell1-3)
- Client: Plays spell-self or spell-dir only; creatures cast without an animation.
- A player notices: Cast pose is generic.
- Where: `crates/mao-client/src/net.rs:2390`; `docs/CLIENT_UI.md:1971`
- Decision: [ ] port  [ ] leave

#### `NAN-016` Cast sound phases end at the nano's cast time, not the animation's end

- SIMPLIFIED, low
- Original: The travel sound runs while the cast animation plays and the hit's sound plays when the animation ends (CharCastNano_t).
- Client: The animation's end is taken as the nano's cast time (stat 294 in hundredths); the client does not ask the animation.
- A player notices: Travel and hit sounds may be timed slightly differently from the animation.
- Where: `crates/mao-client/src/sounds.rs:362`; `docs/ORIGINAL_AUDIT.md:96`
- Decision: [ ] port  [ ] leave

#### `NAN-017` Casting animation timed by the client itself

- SIMPLIFIED, low (notes only)
- Original: 0x6b FinishNanoCasting clears the cast's still-casting byte.
- Client: The client times the casting animation itself; 0x6b draws nothing.
- A player notices: Cast end follows a client timer rather than the server message.
- Where: `docs/CAPTURE_GAPS.md:225`
- Decision: [ ] port  [ ] leave

#### `NAN-018` Effect meshes: only some drawn, head attractor used when the body lacks one

- SIMPLIFIED, low (notes only)
- Original: GfxControlEffectMesh_t via FUN_1010ce9c.
- Client: Draws via attractor frames; the head's attractor when the body lacks it, nothing without a body; only some of the 28 are done.
- A player notices: An effect mesh may sit at the head instead of the intended attractor.
- Where: `docs/CLIENT_UI.md:2325`
- Decision: [ ] port  [ ] leave

#### `NAN-019` Hidden nano programs kept in a file, not a seventh school

- SIMPLIFIED, low
- Original: Keeps hidden programs in a seventh list (school 6).
- Client: A set kept in hiddennanos.ini beside mao.ini.
- A player notices: None visible.
- Where: `crates/mao-client/src/nanoprograms.rs:88`; `docs/CLIENT_UI.md:438`
- Decision: [ ] port  [ ] leave

#### `NAN-020` Other characters' cast times use the nano's time alone; flag 0x80000 not followed

- SIMPLIFIED, low
- Original: Gamecode FUN_1005141d works the time from nano initiative, aggression and least time; a nano with flag 0x80000 has no time whatever the stats.
- Client: Done for one's own casts (cast_seconds); for anyone else the casting stance follows the nano's own attack time because their skills are not known; the 0x80000 flag is not followed.
- A player notices: Others' casting stances can play for casts that would be instant; flagged nanos show a cast time they should not have.
- Where: `crates/mao-client/src/net.rs:673`; `crates/mao-client/src/net.rs:2370`; `docs/CLIENT_UI.md:1236`
- Decision: [ ] port  [ ] leave

#### `NAN-021` Plasma tracer wobbles up and down

- SIMPLIFIED, low (notes only)
- Original: Wobbles the band across the view.
- Client: Wobbles it up and down.
- A player notices: Tracer wobble direction differs.
- Where: `docs/CLIENT_UI.md:2311`
- Decision: [ ] port  [ ] leave

#### `NAN-022` Spell 0xcf22 stat set only for one's own character

- SIMPLIFIED, low
- Original: FUN_100a4f8a sets stat 0 on any character (pets, NPCs).
- Client: Kept for one's own character only (the client keeps no stats of others).
- A player notices: Pets and NPCs keep their own mesh stat.
- Where: `crates/mao-client/src/spelllist.rs:32`; `docs/CAPTURE_GAPS.md:73`
- Decision: [ ] port  [ ] leave

#### `NAN-023` Spell 0xcfd4 effect drawn without colours, scale and repetitions

- SIMPLIFIED, low
- Original: FUN_100a9159 makes the effect with repetitions (stat 0xa1), start and stop colours (0x99-0xa0) and a scale (stat 0x32 / 100 + 1).
- Client: Ported as the effect only.
- A player notices: Environment effects (smoke, sparks) look plainer.
- Where: `crates/mao-client/src/spelllist.rs:22`; `docs/CAPTURE_GAPS.md:66`
- Decision: [ ] port  [ ] leave

#### `NAN-024` Spell lists with an unreadable spell, or spells 0xcf17/0xcf20, are dropped whole

- SIMPLIFIED, low
- Original: SpellFormats_c reads types 0xcf17 and 0xcf20 with more values after the format
- Client: A list whose spells do not all read is left out with all its spells (spell lists are found by scanning for key 2); 0xcf17 and 0xcf20 carry more after their values and are not read.
- A player notices: All effects of a list are lost if one spell is unknown.
- Where: `crates/mao-formats/src/spell.rs:151`; `crates/mao-formats/src/spell.rs:6`
- Decision: [ ] port  [ ] leave

#### `NAN-025` Stars effect: only some modes as the original

- SIMPLIFIED, low (notes only)
- Original: _GfxControlStars_t with 29 modes.
- Client: Decompile is mangled so modes follow its shape rather than exact numbers; later modes 1, 2, 5, 9, 13, 15, 16-18, 20, 22 added besides 6 and 19; unlisted modes keep a spiral.
- A player notices: Unlisted modes show a spiral instead of the original's effect.
- Where: `docs/CLIENT_UI.md:2265`
- Decision: [ ] port  [ ] leave

#### `NAN-026` Stars mode 22 edges measured across the client's attach table; mode 15 uses blob-shadow points

- SIMPLIFIED, low (notes only)
- Original: Body edges between bones; mode 15 samples thirty of the target's posed vertices.
- Client: Mode 22 edges are measured between the client's attach-point table, so they run across the body's x rather than its heading; mode 15 takes vertices from the target's blob-shadow points.
- A player notices: Effects are oriented to the world axis and sparks come from approximate surface points.
- Where: `docs/CLIENT_UI.md:2281`; `docs/CLIENT_UI.md:2291`; `crates/mao-client/src/effects.rs:2075`
- Decision: [ ] port  [ ] leave

#### `NAN-027` Volume grid: fan gradient replaced by flat quads, slices added instead of alpha-blended

- SIMPLIFIED, low (notes only)
- Original: Fan gradient within a slice; alpha-blended slices.
- Client: Two flat quads stand in for the gradient; slices are added to the scene.
- A player notices: Volume effects look different.
- Where: `docs/CLIENT_UI.md:2273`
- Decision: [ ] port  [ ] leave

### Nano programs: the original was not read: inferred or measured (9)

#### `NAN-028` Cast, tracer, hit and spiral effects: partial decode with chosen sizes and timings

- GUESS, medium (notes only)
- Original: Effect trees in gfxtweak.bin; buff spiral data radius path numbers 0.06-0.15.
- Client: Decode is partial and checked in pictures only; radius (0.42 m), height (1.9 m), life of spirals, trail, sprite scale, hand height of the cast, flight time 0.45 s and hit size are chosen rather than decoded. (An early note that effects cannot be drawn at all is out of date: a partial decode is drawn.)
- A player notices: Effects look different from the original's.
- Where: `docs/CLIENT_UI.md:1999`; `docs/CLIENT_UI.md:2004`; `docs/CLIENT_UI.md:1969`
- Decision: [ ] port  [ ] leave

#### `NAN-029` Others' morph, vehicle and aura programs are guessed from repeated buffs for 30 seconds

- GUESS, medium
- Original: not stated
- Client: Since the server repeats others' buffs, one seen in the last 30 s counts as still running.
- A player notices: Another character's morph, mount or aura can linger up to 30 s after the program ends, or vanish if the server stops repeating.
- Where: `crates/mao-client/src/net.rs:9280`; `crates/mao-client/src/net.rs:9302`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the guess is gone: `Buff` turned out to be a program's removal, not its repeat. The programs on other characters are kept from their start (`CharacterAction` 0x62 for the character) or their update's list to their `Buff` (`World::others_nanos`), and morphs, vehicles and auras are read from there.

#### `NAN-030` Active nano list found by its shape

- GUESS, low
- Original: SimpleCharFullUpdate reads ActiveNanos at its place in the field order.
- Client: The list is found by scanning for a count word followed by entries beginning with 0xcf1b.
- A player notices: The nano list may be misread for odd updates.
- Where: `crates/mao-net/src/n3.rs:2156`
- Decision: [ ] port  [ ] leave

#### `NAN-031` Flare effect burns where the character stood

- GUESS, low (notes only)
- Original: not stated
- Client: A flare burns where the character stood and does not follow it.
- A player notices: Fire on cleaning robots does not move with them.
- Where: `docs/CAPTURE_GAPS.md:64`
- Decision: [ ] port  [ ] leave

#### `NAN-032` Hoverbike exhaust direction not checked against the original

- GUESS, low (notes only)
- Original: not stated
- Client: Exhaust is blown along the attractor's z; whether it lies so in the original is not checked.
- A player notices: Exhaust direction may differ.
- Where: `docs/CLIENT_UI.md:2340`
- Decision: [ ] port  [ ] leave

#### `NAN-033` Left limb buff rings made to vanish into the body

- GUESS, low (notes only)
- Original: A one-sided ring seen in the original on the thigh.
- Client: Left-limb offsets are sent the other way so rings vanish; inferred from the rig and one observation.
- A player notices: Left limb rings are hidden.
- Where: `docs/CLIENT_UI.md:2088`
- Decision: [ ] port  [ ] leave

#### `NAN-034` Nano cast message laid out from captures

- GUESS, low
- Original: not stated
- Client: Layout taken from captured casts, not from the original's writer.
- A player notices: None while the server accepts it.
- Where: `crates/mao-net/src/n3.rs:213`
- Decision: [ ] port  [ ] leave

#### `NAN-035` Nano page: attack time 10 treated as none

- GUESS, low
- Original: FUN_10086142 sets NanoItem_t defaults; FUN_10032582.
- Client: Reproduces the defaults and "here takes 10 as none".
- A player notices: A nano page attack time of 10 is hidden.
- Where: `crates/mao-client/src/gui/iteminfo.rs:31`
- Decision: [ ] port  [ ] leave

#### `NAN-036` Which spell tail words count was found by checking live

- GUESS, low
- Original: not stated
- Client: The tail word is accepted as 2 or 3 for items and 1, 2 or 3 for nano programs.
- A player notices: Nano buffs with other tail values would not count.
- Where: `crates/mao-formats/src/item.rs:66`
- Decision: [ ] port  [ ] leave

### Nano programs: a guard or fallback the original lacks (1)

#### `NAN-037` Gather sound ends after 5 seconds without a server answer

- SAFETY, low
- Original: not stated
- Client: The gather sound stops after 5 s without an answer from the server.
- A player notices: None visible.
- Where: `crates/mao-client/src/sounds.rs:371`
- Decision: [ ] port  [ ] leave

### Nano programs: the original does it, the client does not (18)

#### `NAN-038` Cast effect: hand cords draw nothing; phases 3-4 (projectile, impact) not played

- MISSING, medium (notes only)
- Original: _GfxControlSpell1_t phases 3-4: projectile _GfxControlNano0_t from the hands to the target, impact _GfxControlNano1_t over the bones; hand cords (kind 1003) lay links.
- Client: Only hand flares and the line between hands; cords draw nothing; timeline fixed (phase 1 0-6 s, beam 4-5 s), so in practice a cast shows the hand flares only.
- A player notices: No projectile or impact from casts.
- Where: `docs/ORIGINAL_AUDIT.md:267`
- Decision: [ ] port  [ ] leave

#### `NAN-039` Nano programs window: programs cannot be placed by hand; radius value, widths and sort not kept

- MISSING, medium
- Original: Programs can be placed by hand (NanoView_c item_position_map, "Don't Auto Arrange"); the radius column has a value; column widths and the list sort are kept (listview_config<n>).
- Client: Not ported; "Don't Auto Arrange" is shown but dead; the radius is always "---"; widths and sort reset each session.
- A player notices: Programs cannot be arranged by hand; list layout resets each session.
- Where: `crates/mao-client/src/nanoprograms.rs:166`; `crates/mao-client/src/nanoprograms.rs:71`; `docs/CLIENT_UI.md:452`; `docs/GUI_DLL_PARITY.md:81`; `docs/ORIGINAL_AUDIT.md:103`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): widths and the list's sort are kept in lists.ini for the NCU windows, the nano programs and the mission journal (layout::ListConfig, multilist::Keeper); hand placing and the radius not done.

#### `NAN-040` NCU list: no Auto Arrange submenu, radius value or kept widths and sort

- MISSING, medium
- Original: The window's menu has an Auto Arrange submenu; the radius column comes from N3Msg_GetFormularRadius; column widths and sort are kept (listview_config).
- Client: Not ported; the radius column shows "---"; widths and sort are not kept.
- A player notices: No Auto Arrange choice in the NCU window, radius never shown, columns reset each session.
- Where: `crates/mao-client/src/ncu.rs:179`; `crates/mao-client/src/ncu.rs:181`; `docs/CLIENT_UI.md:412`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): widths and the list's sort are kept in lists.ini for the NCU windows, the nano programs and the mission journal (layout::ListConfig, multilist::Keeper); the Auto Arrange sub-menu and the radius not done.

#### `NAN-041` Cast refusal "NCUErrorNanoprogramCantReplaceOther" not ported

- MISSING, low (notes only)
- Original: N3Msg_CastNanoSpell check (FUN_1004fbe4 -> FUN_1004fb8d) refuses a nano that cannot replace another of its line.
- Client: Not ported (no cast list with the nanos' lines).
- A player notices: No refusal message when casting a nano that cannot replace another.
- Where: `docs/CAPTURE_GAPS.md:257`
- Decision: [ ] port  [ ] leave

#### `NAN-042` Crazy cones: texture offset scrolling not drawn

- MISSING, low (notes only)
- Original: Tracks scroll texture offsets.
- Client: Not drawn.
- A player notices: Cones lack texture scroll.
- Where: `docs/CLIENT_UI.md:2362`
- Decision: [ ] port  [ ] leave

#### `NAN-043` Effect sequencer entry end and looping not followed; some rare classes not drawn

- MISSING, low (notes only)
- Original: _GfxControlSequencer_t entry end (cuts the child's duration to end - start in 56 of 522 entries) and flag 0x400 looping.
- Client: Not followed; some rare effect classes are not drawn.
- A player notices: Some effects last longer or do not loop.
- Where: `docs/CLIENT_UI.md:2007`
- Decision: [ ] port  [ ] leave

#### `NAN-044` Highlights: mode 3, flag 0x400 priority and hide word not drawn

- MISSING, low (notes only)
- Original: Mode 3 shows the worn items alone with specular; flag 0x400 render priority; a hide word.
- Client: Not drawn (hide word set in no template; mode 3 templates unnamed by programs).
- A player notices: None visible by the notes.
- Where: `docs/CLIENT_UI.md:2376`
- Decision: [ ] port  [ ] leave

#### `NAN-045` Nano type "Self" never shown

- MISSING, low
- Original: Type 3 (self only) is a flag of Gamecode's nano object (fields +0xbc radius and +0xb8 self only).
- Client: The fields were not traced to the record, so no nano is told to be Self; self-only nanos are listed as Friendly.
- A player notices: Self-only nanos show the wrong type; radius also absent.
- Where: `crates/mao-client/src/ncu.rs:39`; `docs/ORIGINAL_AUDIT.md:109`
- Decision: [ ] port  [ ] leave

#### `NAN-046` Server's answer to a nano cast not shown

- MISSING, low (notes only)
- Original: The server answers with the nano effect or a refusal (range, NCU memory).
- Client: The note says refusal or answer is not shown, only "Casting <name>"; a later "Executing Nano Program" feedback line was added (feedback.rs maps it); refusals not found in code.
- A player notices: No feedback on a failed cast.
- Where: `docs/CLIENT_UI.md:1762`; `crates/mao-client/src/feedback.rs:119`
- Decision: [ ] port  [ ] leave

#### `NAN-047` Several spell types are not run on the client

- MISSING, low
- Original: 0xcf4c (FUN_100a7c79, state flag of stat 0x49), 0xcf44 (FUN_100a7aa2, stat 0x167 MonsterData), 0xcf7d (FUN_100a83d8, feedback lines), 0xcf35 (FUN_100a517a, an item's modify) and 0xcf16 runs each do something; 0xcf92 first makes a crawling, sitting, sleeping or lounging character get up (0x28/0x29/0x2a/0x25).
- Client: None of these is run; 0xcf16 spells are left out of the list. (The ending-list effect delete and the 0xcf92 fly mode are now done.)
- A player notices: Their state flags, template mesh changes, feedback lines, item stat changes and 0xcf16 effects do not happen; a hoverbike program does not make a sitting character stand.
- Where: `crates/mao-client/src/spelllist.rs:38`; `crates/mao-client/src/spelllist.rs:34`; `crates/mao-client/src/spelllist.rs:284`; `docs/CAPTURE_GAPS.md:84`; `docs/ORIGINAL_AUDIT.md:113`
- Decision: [ ] port  [ ] leave

#### `NAN-048` Single-quad sprite effects: body-fixed quad, billboard height, alpha blending not done

- MISSING, low (notes only)
- Original: GfxVisualSprite3 standing quad fixed in the body frame, separate billboard height, alpha blending.
- Client: Not done.
- A player notices: Tower beams and halos are approximated.
- Where: `docs/CLIENT_UI.md:2277`
- Decision: [ ] port  [ ] leave

#### `NAN-049` Smoke streams: force push of other kinds and sprite spin not done

- MISSING, low
- Original: The effect handler's force pushes other kinds; sprites spin.
- Client: Not done (the client keeps no force).
- A player notices: Smoke does not respond to force or spin.
- Where: `crates/mao-client/src/effects.rs:2189`; `docs/CLIENT_UI.md:2275`
- Decision: [ ] port  [ ] leave

#### `NAN-050` Spell types 0xcf0a, 0xcf56, 0xcf4a, 0xcf70, 0xcf1b, 0xd003 do nothing

- MISSING, low
- Original: Gamecode FUN_100a5f4b handles those spell types (per the note; the code's own comment says they fall outside the table or its default)
- Client: Nothing on the client.
- A player notices: Those spell effects are absent.
- Where: `crates/mao-client/src/spelllist.rs:27`; `docs/CAPTURE_GAPS.md:75`
- Decision: [ ] port  [ ] leave

#### `NAN-051` Spells aimed at the use target (0x17) never run

- MISSING, low
- Original: Target 0x17 is the character's use target (+0x1e4's +0x14).
- Client: The run handles 0x17, but the scene never has a use target (net.rs answers None), so such spells are not run.
- A player notices: Spells aimed at the use target do not run.
- Where: `crates/mao-client/src/net.rs:1078`; `crates/mao-client/src/spelllist.rs:300`; `docs/CAPTURE_GAPS.md:79`
- Decision: [ ] port  [ ] leave

#### `NAN-052` Stun: the call to the vehicle is not made

- MISSING, low
- Original: FUN_1005a02c(1) call to the vehicle (+0x9c/+0x98) on a stun; effects of the state flags 0x400 / 0x100 / 0x4200.
- Client: The flags themselves are set and one's own character cannot move; the vehicle call is not made.
- A player notices: A stun has no effect on a vehicle.
- Where: `crates/mao-client/src/spelllist.rs:29`; `docs/CAPTURE_GAPS.md:86`
- Decision: [ ] port  [ ] leave

#### `NAN-053` Target NCU windows: no borderless style, own tab strip or places beside the target bars

- MISSING, low
- Original: Alt and a double click makes the window borderless (FUN_100dd7ea); the window has its own tab strip; the windows sit beside the target bars.
- Client: Not ported.
- A player notices: Target NCU windows cannot be made borderless and lack the tab strip.
- Where: `crates/mao-client/src/ncu.rs:363`; `docs/CLIENT_UI.md:431`
- Decision: [ ] port  [ ] leave

#### `NAN-054` Thrown billboard particles: spin, bounce, height and daylight fades, body turn not done

- MISSING, low (notes only)
- Original: BParticle2 with spin, ground bounce, fades by height and by daylight, body turn.
- Client: Not done.
- A player notices: Debris and embers are approximate.
- Where: `docs/CLIENT_UI.md:2279`
- Decision: [ ] port  [ ] leave

#### `NAN-055` Tracer cords: ribbon and locator not done

- MISSING, low (notes only)
- Original: A ribbon between the links and a locator riding along the line.
- Client: Draws each link as a sprite; ribbon and locator not done.
- A player notices: Tracer cords look dotted.
- Where: `docs/CLIENT_UI.md:2271`
- Decision: [ ] port  [ ] leave

### Nano programs: the client has it, the original does not (5)

#### `NAN-056` Nano landing glow: three widening rings, colour from stats or school

- ADDITION, medium
- Original: not stated (the original's effects come from gfxtweak.bin templates)
- Client: When a nano lands, three coloured rings widen and fade round the target's chest for 1.1 s on top of the effect visuals; colour from the program's EffectRed/Green/Blue stats, else one of six fixed by school. The notes call it a stand-in for the original's effects.
- A player notices: An extra ring effect on nano targets; nanos without colour stats glow in invented colours.
- Where: `crates/mao-client/src/ui.rs:2164`; `crates/mao-client/src/ui.rs:45`; `crates/mao-client/src/net.rs:9738`; `docs/CLIENT_UI.md:1973`
- Decision: [ ] keep  [ ] remove

#### `NAN-057` Programs fade while a cast or recharge runs; locks from the client's own timing

- ADDITION, medium
- Original: Not stated for the nano window; the nano program locks are not seen live.
- Client: A nano lock (cast time plus the nano's RechargeDelay 210, or a skill lock the run locks) fades every program in the windows; the locks come from the client's own cast as the server starts it.
- A player notices: Program icons grey out after a cast, possibly differing from the server's lock.
- Where: `crates/mao-client/src/net.rs:1308`; `crates/mao-client/src/net.rs:1317`; `crates/mao-client/src/nanoprograms.rs:332`; `docs/CLIENT_UI.md:4866`; `docs/CLIENT_UI.md:4877`; `crates/mao-client/src/net.rs:1343`
- Decision: [ ] keep  [ ] remove

#### `NAN-058` "Nano programs available." chat line

- ADDITION, low
- Original: Not stated; the client calls the line its own.
- Client: After a cast and recharge of two seconds or more, chat says "Nano programs available."
- A player notices: An extra chat line after long waits.
- Where: `crates/mao-client/src/net.rs:1339`
- Decision: [ ] keep  [ ] remove

#### `NAN-059` Nano program and NCU tips are the client's own layout

- ADDITION, low
- Original: A click shows the nano's page (FUN_10045259); a hover tip is not described; the program menu has "Look at" and "Hide".
- Client: A hover tip with name, NCU (or "x of y left"), description (420 characters) and up to ten stat changes.
- A player notices: Extra hover text.
- Where: `crates/mao-client/src/nanoprograms.rs:450`; `crates/mao-client/src/ncu.rs:281`
- Decision: [ ] keep  [ ] remove

#### `NAN-060` Right-click "Cancel this nano" entry is the client's own

- ADDITION, low
- Original: The window's menu (FUN_100d758a) has "List Mode" only.
- Client: Over a nano the menu has an extra entry that cancels it.
- A player notices: Extra way to cancel a running nano.
- Where: `crates/mao-client/src/ncu.rs:178`; `docs/GUI_DLL_PARITY.md:80`
- Decision: [ ] keep  [ ] remove

## Zoning and playfields

### Zoning and playfields: the client knowingly does otherwise (2)

#### `ZON-001` Doors open on approach within 4 m

- OWN-RULE, high
- Original: opens a door on a character's touch (FUN_1007fccc, unless a lock or owned building keeps them out: FUN_1007fad8)
- Client: Whom-Pah doors (and doors not locked) open as the character comes within 4 m horizontally and 3 m vertically and close 5 s later; other doors open on a use
- A player notices: door opening trigger distance differs from a touch
- Where: `crates/mao-client/src/net.rs:806`; `crates/mao-client/src/main.rs:550`; `docs/CLIENT_UI.md:3249`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): a door opens when the character's body sphere meets the door's (both from their meshes' packed words) and shuts on the first frame nothing touches it, unless the server said it stays; the 4 m, 3 m and five seconds are gone. Locked is now bit 0x40 of stat 0 (684 doors, against 13 by stat 259's bit), and a shut door that is not locked no longer stops the character between rooms. Left out on purpose: the frame's stat 437 test, read as it stands it would stop a door answering after two seconds, and no door has that stat. `CLIENT_UI.md` "A door opens at the character's touch".

#### `ZON-002` Zoning ends following and walking

- OWN-RULE, low (notes only)
- Original: not stated
- Client: a zoning ends following and walking, since the followed target's number could stand for something else
- A player notices: auto-follow stops after a zone
- Where: `docs/CLIENT_UI.md:1560`
- Decision: [ ] port  [ ] leave

### Zoning and playfields: an approximation or a stand-in (3)

#### `ZON-003` Door zoning by distance to the table door's place

- SIMPLIFIED, medium
- Original: a collision callback FUN_100a944a with gate FUN_10058ae4 starts StartTeleportTry; TeleportTrier_t gives up after 30 s; the zone outline is n3Zone_t::IsPosInTeleportal (FUN_10058b35)
- Client: sends one full report (0x16) when within a fixed reach (DOOR_REACH, measured live: 1.1 m off ignored, 0.24 m zoned; notes say 2 m) of a table door's place, or inside a teleportal, then waits; a new try needs the door left and touched again
- A player notices: zoning trigger distance differs from a collision contact
- Where: `crates/mao-client/src/net.rs:9151`; `crates/mao-client/src/net.rs:9154`; `docs/CLIENT_UI.md:2114`; `docs/ORIGINAL_AUDIT.md:53`
- Decision: [ ] port  [ ] leave

#### `ZON-004` Own door use opens the door for a fixed 5 seconds

- SIMPLIFIED, medium
- Original: the door follows the server's open flag; its state machine (FUN_10080477 / FUN_10080319) closes it a while after
- Client: using a door opens it locally at once for 5 s, not cut short by a closed word, in case the server never answers; a used or approached door is held open a fixed 5 s
- A player notices: door swings before the server answers and closes on a fixed timer
- Where: `crates/mao-client/src/net.rs:1626`; `crates/mao-client/src/net.rs:803`; `docs/CLIENT_UI.md:1715`; `crates/mao-client/src/net.rs:12248`
- Decision: [ ] port  [ ] leave

#### `ZON-005` Generated buildings: doors linked and rooms placed by reconstructed rules

- SIMPLIFIED, low
- Original: building generator FUN_100c92b3 links doors; Gamecode FUN_100ca6f8 and FUN_100cacbc compute each room's place
- Client: doors that face each other within 0.1 in x and z and 0.3 in height are paired, any that meets none is a way out; corner and centre offsets written out per turn (90, 180, 270)
- A player notices: doors outside the tolerance become exits
- Where: `crates/mao-formats/src/rooms.rs:229`; `crates/mao-formats/src/rooms.rs:270`
- Decision: [ ] port  [ ] leave

### Zoning and playfields: the original was not read: inferred or measured (7)

#### `ZON-006` Instanced playfield object ids read from live bytes

- GUESS, medium
- Original: not stated
- Client: runs of doors, terminals and machines are found by searching for the 0xc77d word, runs of five words decoded from captures, or at a fixed byte 90 for owned buildings; table id mapped through the runs
- A player notices: instanced shops' machines/objects may keep table ids if the layout differs
- Where: `crates/mao-net/src/n3.rs:2115`; `docs/CLIENT_UI.md:3279`
- Decision: [ ] port  [ ] leave

#### `ZON-007` City terminal's third word is a guess

- GUESS, low
- Original: FUN_1012faab reads the signal; the word's meaning is not stated
- Client: treated as "a rank in the organisation" that lets the window build and demolish when 0 or 1
- A player notices: who may build or demolish in the city terminal may differ
- Where: `crates/mao-net/src/city.rs:52`
- Decision: [ ] port  [ ] leave

#### `ZON-008` Door number guessed from the run of ids

- GUESS, low
- Original: not stated
- Client: a door's number is its place in the run of consecutive announced ids, in table order, a guess that fits one capture
- A player notices: a wrong door may animate in odd instances
- Where: `crates/mao-client/src/net.rs:1555`; `docs/CLIENT_UI.md:1715`
- Decision: [ ] port  [ ] leave

#### `ZON-009` Owned-building playfield number taken at byte 58

- GUESS, low (notes only)
- Original: OwnedBuildingGeneratorData_t read order (FUN_1011f23e)
- Client: playfield number taken at byte 58 by the original's order of reads, no captured message confirmed it then (a later note says tests of real messages read it)
- A player notices: possibly wrong building inside
- Where: `docs/CLIENT_UI.md:3859`
- Decision: [ ] port  [ ] leave

#### `ZON-010` Server travel-destination list: two flag bytes not understood

- GUESS, low
- Original: not stated
- Client: the two bytes after each name in the FightModeUpdate list are kept as flags (1,1 for most, 1,2 for one) and unused; the message name comes from AOSharp sources
- A player notices: none visible
- Where: `crates/mao-net/src/destinations.rs:14`
- Decision: [ ] port  [ ] leave

#### `ZON-011` Teleport message's extra identities and tail not used

- GUESS, low
- Original: n3TeleportIIR_t::ReadSubClass (0x10029e7f) reads two identities at +0x4c and +0x54 (the second 186a1 before most zonings, 186a7 in a mission) and a byte string
- Client: keeps them as unknown and does nothing with them
- A player notices: none visible
- Where: `crates/mao-net/src/n3.rs:746`; `docs/CAPTURE_GAPS.md:117`
- Decision: [ ] port  [ ] leave

#### `ZON-012` Whom-Pah sign-to-playfield table is inferred

- GUESS, low
- Original: not stated
- Client: the playfield for each sign texture name was worked out from the game's own data ("the network is the same both ways"), the playfields' names and one booth checked against the server; three playfields have two stations
- A player notices: a booth's label may be wrong where the inference is
- Where: `crates/mao-client/src/whompah.rs:111`; `docs/CLIENT_UI.md:4332`
- Decision: [ ] port  [ ] leave

### Zoning and playfields: a guard or fallback the original lacks (4)

#### `ZON-013` City house blob refused when its length does not match

- SAFETY, low
- Original: does not check the count against the length
- Client: a blob that does not hold exactly its count of houses is refused
- A player notices: with odd data no houses show
- Where: `crates/mao-net/src/landcontrol.rs:145`
- Decision: [ ] port  [ ] leave

#### `ZON-014` Redirect hop logs in again and retries

- SAFETY, low
- Original: connects to the redirect address with the kept game key (0x1b), no new login
- Client: hop_direct does the same; only as the fallback hop() logs in again at the default server then the redirect address, retrying up to 90 times a second apart with a "waiting for the server" line (the README's older text describes only the log-in-again route)
- A player notices: when the fallback is used, a possibly long wait
- Where: `crates/mao-client/src/net.rs:75`; `crates/mao-client/src/net.rs:6742`; `crates/mao-client/src/net.rs:10759`; `docs/ARETE_CAPTURE.md:66`; `docs/CLIENT_UI.md:1989`; `README.md:76`
- Decision: [ ] port  [ ] leave

#### `ZON-015` Shuttle pad teleport with every id zero taken as a move

- SAFETY, low
- Original: the original client crashed on a c7a0 teleport with every id zero into an instance not made yet; whether a redirect would have followed is unknown
- Client: treats it as a move within the playfield to the pad's place and nothing more
- A player notices: no crash; the player is placed at the pad instead of zoning
- Where: `crates/mao-client/src/net.rs:12709`; `docs/ARETE_CAPTURE.md:131`
- Decision: [ ] port  [ ] leave

#### `ZON-016` Zoning hold released after 30 seconds

- SAFETY, low
- Original: stays stopped (TeleportTrier_t, StopPlayfield, no timer) until the playfield message
- Client: after 30 s without a playfield message the character moves again
- A player notices: no permanent freeze if the message never comes
- Where: `crates/mao-client/src/net.rs:7258`; `docs/CAPTURE_GAPS.md:116`
- Decision: [ ] port  [ ] leave

### Zoning and playfields: the original does it, the client does not (7)

#### `ZON-017` Doors: sound, own closing time and locked-door check not ported

- MISSING, high
- Original: door has a sound (stat 131, else 100, FUN_10080477), its own closing time (stat 335), and a locked-door question whether the character may pass (FUN_1007fad8: key, owned building, team's mission) with a feedback line
- Client: not done; closing time is five seconds; locked doors stay shut until the server says they are open; keypad side not compared
- A player notices: doors silent, close after a fixed 5 s, a player the original lets through finds a locked door shut, no feedback
- Where: `crates/mao-client/src/main.rs:553`; `docs/CLIENT_UI.md:831`; `docs/ORIGINAL_AUDIT.md:40`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done as far as the client keeps the question's inputs: the server's word locks and unlocks a door and gives its key (DoorStatusUpdate's locked byte and stat 195), and a locked door in reach asks the original's question (FUN_1007fad8) and opens for one who may pass: a door with an exit, a key door for who holds an item with its key, any door inside an owned building. Not asked: the door's own access (its requirements and its list of identities and access cards, which are not kept), so the full-backpack line is not said either. Only 13 of the game's 5,082 table doors are locked and no captured door is: not seen against a server.
- Done in part (2026-11-18, overnight): the doors' opening and closing sounds are played (from the template's sound set: classes 0x83/0x64 and 0x84/0x66; the entry's 'stat 131' is wrong). The entry's closing time is a misreading: the original has no timer (a door closes when no character touches it; stat 335 is a count of seconds before it lets through), so the client's five seconds are kept. The locked door's question is not done.

#### `ZON-018` City signals not checked against the city id

- MISSING, low
- Original: holds the opening's CityID_t against the playfield's own
- Client: the server's signal is taken as it comes
- A player notices: signals for another city may be applied
- Where: `crates/mao-client/src/net.rs:3162`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): the compare is in city.dll (CityClientInterface_c::SlotOpenGUISession, code 5 only) and a mismatch only leaves the session deaf; no capture has a city signal, so the id's place on the wire is not proven.

#### `ZON-019` City signals with unknown codes are dropped

- MISSING, low
- Original: Gamecode dispatch FUN_1012fdaa handles every code
- Client: signal returns None for any code other than 2, 5 to 10, 13 to 16
- A player notices: other city events are ignored
- Where: `crates/mao-net/src/city.rs:105`
- Decision: [ ] port  [ ] leave

#### `ZON-020` Owned building: fixtures not read

- MISSING, low
- Original: OwnedBuildingGeneratorData_t reader (FUN_101217c0) reads a list of fixtures (what an organisation has put in the building) and more
- Client: reads only version, template and outside place
- A player notices: organisation-placed fixtures not shown
- Where: `crates/mao-net/src/n3.rs:2067`; `docs/CLIENT_UI.md:3861`
- Decision: [ ] port  [ ] leave

#### `ZON-021` Planet map window: no Build/War buttons, coordinate row or own PF map view

- MISSING, low (notes only)
- Original: MapControlView_c Build/War buttons, coordinate/waypoint row; the window's PF map view
- Client: not done
- A player notices: map lacks those controls
- Where: `docs/GUI_DLL_PARITY.md:101`
- Decision: [ ] port  [ ] leave

#### `ZON-022` Playfield definitions older than version 7 are refused

- MISSING, low
- Original: RDBPlayfield_t::ReadBlob reads older formats
- Client: version below 7 gives an error
- A player notices: such playfields have no rooms
- Where: `crates/mao-formats/src/rooms.rs:362`
- Decision: [ ] port  [ ] leave

#### `ZON-023` Tower update of other kinds left out

- MISSING, low
- Original: FUN_1012b350 adds an empty entry for kinds other than 1 and 2
- Client: other kinds are ignored
- A player notices: none visible
- Where: `crates/mao-net/src/landcontrol.rs:84`
- Decision: [ ] port  [ ] leave

### Zoning and playfields: the client has it, the original does not (5)

#### `ZON-024` Client says the Shadowlands expansion is missing at the Jobe booth

- ADDITION, medium
- Original: the server answers such a try with nothing (the game's text Feedback_YouMustHaveExpansionpack1 exists for stat 389 bit 1; what the original shows a free account was not seen)
- Client: shows an error box with that text when a character without the expansion touches a Jobe booth, and still sends the try
- A player notices: a free player gets a message at the Jobe booth where the original gives none
- Where: `crates/mao-client/src/whompah.rs:207`; `crates/mao-client/src/net.rs:9160`; `docs/CLIENT_UI.md:4189`
- Decision: [ ] keep  [ ] remove

#### `ZON-025` Whom-Pah destinations learned from play and remembered

- ADDITION, medium
- Original: the server decides a booth's destination; the original shows no list of destinations
- Client: learns where a booth leads when the server zones a character standing at it (within 20 s), keeps it in whompah.ini, tells "Whom-Pah: this booth leads to X (remembered)", and seeds one booth checked live; others show "Whom-Pah" until used
- A player notices: booth labels and a chat line the original does not show
- Where: `crates/mao-client/src/whompah.rs:3`; `crates/mao-client/src/net.rs:7142`; `docs/CLIENT_UI.md:208`
- Decision: [ ] keep  [ ] remove

#### `ZON-026` Whom-Pah window with a Go button that walks to the booth

- ADDITION, medium
- Original: not stated
- Client: a window lists booths with distance and a Go button that auto-walks the character to the booth
- A player notices: auto-walk to booths
- Where: `docs/CLIENT_UI.md:117`; `crates/mao-client/src/net.rs:5837`; `crates/mao-client/src/ui.rs:1892`; `crates/mao-client/src/net.rs:5898`; `docs/GUI_DLL_PARITY.md:113`; `crates/mao-client/src/whompah.rs:1`
- Decision: [ ] keep  [ ] remove

#### `ZON-027` Interior exit offered like a Whom-Pah booth

- ADDITION, low
- Original: not stated
- Client: an interior's door blocker is offered in the booth list (labelled, with a Go button that walks there)
- A player notices: the interior's way out appears in the Whom-Pah window
- Where: `crates/mao-client/src/net.rs:5837`
- Decision: [ ] keep  [ ] remove

#### `ZON-028` Whom-Pah booths labelled by their sign texture

- ADDITION, low
- Original: not stated; a sign statel sits over each booth but is not read as a destination
- Client: an unused booth is labelled from the texture name of the sign statel within two units
- A player notices: booths show where they lead before first use
- Where: `crates/mao-client/src/whompah.rs:9`
- Decision: [ ] keep  [ ] remove

## Chat and commands

### Chat and commands: the client knowingly does otherwise (15)

#### `CHT-001` Chat tabs moved by menu entries instead of dragging

- OWN-RULE, high
- Original: chat tabs are dragged between frames
- Client: window menu entries "Move to a window of its own" / "Move into the window of X"
- A player notices: no dragging of tabs
- Where: `crates/mao-client/src/chatwindow.rs:540`; `crates/mao-client/src/chatwindow.rs:3`; `docs/CLIENT_UI.md:364`; `docs/GUI_DLL_PARITY.md:75`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): a chat tab is taken up by the right button let go on it (Tab FUN_10146ef3, as the decompile reads; not seen live), its picture follows the pointer, and the left button puts it before a tab of a strip, last on a strip, or into a window of its own at the pointer (not the only tab of a window); Escape lets go (the client's own). The menu entries stay as the client's own.

#### `CHT-002` /help and /version show the client's own text

- OWN-RULE, medium
- Original: /version also asks the server for its version (action 0x9d); /help is the original's own
- Client: /help lists the client's own command descriptions; /version says only the client's version
- A player notices: different help text; no server version shown
- Where: `crates/mao-client/src/chat_commands.rs:83`
- Decision: [ ] port  [ ] leave

#### `CHT-003` /bug sends no report

- OWN-RULE, low
- Original: /bug asks in a box and hands the text to BugReport_t::SendUserMadeBugReport (the makers' bug database)
- Client: left out on purpose; parsed as Bug, says so, sends nothing
- A player notices: no bug report goes to the makers
- Where: `crates/mao-client/src/chat_commands.rs:144`; `crates/mao-client/src/chat_commands.rs:578`; `docs/CLIENT_UI.md:4270`
- Decision: [ ] port  [ ] leave

#### `CHT-004` /cc without a word prints the client's usage line

- OWN-RULE, low
- Original: says its syntax is wrong
- Client: prints the usage line "/cc <command> [words]"
- A player notices: different message
- Where: `crates/mao-client/src/chat_commands.rs:610`; `crates/mao-client/src/chat_commands.rs:614`; `docs/CLIENT_UI.md:4251`
- Decision: [ ] port  [ ] leave

#### `CHT-005` /inspect with no number is the client's own

- OWN-RULE, low
- Original: /inspect with no argument does nothing (it wants the number)
- Client: /inspect alone (or with a name) asks about the target and opens the info page
- A player notices: /inspect alone does something
- Where: `crates/mao-client/src/chat_commands.rs:377`; `docs/CLIENT_UI.md:4566`
- Decision: [ ] port  [ ] leave

#### `CHT-006` /items delete is left out

- OWN-RULE, low
- Original: /items delete <number> (action 0xf5) has the server delete one of the account's items (Gamecode FUN_1003fe10)
- Client: only /items and /items list are carried out; delete is Unsupported on purpose
- A player notices: /items delete does nothing
- Where: `crates/mao-client/src/chat_commands.rs:467`; `crates/mao-client/src/chat_commands.rs:699`; `docs/GUI_DLL_PARITY.md:202`; `docs/SLASH_COMMANDS_STATUS.md:16`
- Decision: [ ] port  [ ] leave

#### `CHT-007` /tell alone opens a window for the target

- OWN-RULE, low
- Original: does nothing when /tell has no name
- Client: /tell alone opens a tell window with the current target; with no target it prints a usage line of the client's own
- A player notices: typing /tell alone gives a window or a usage line; the original does nothing
- Where: `crates/mao-client/src/chat_commands.rs:94`; `crates/mao-client/src/chat_commands.rs:348`; `crates/mao-client/src/chatwindow.rs:1123`; `docs/CLIENT_UI.md:1150`; `docs/ORIGINAL_AUDIT.md:31`
- Decision: [ ] port  [ ] leave

#### `CHT-008` Chat filter rules take the next free number

- OWN-RULE, low
- Original: FUN_100b933d files a new rule under the count of rules, which can be a name already in use; the rule is then never shown or tried
- Client: gives the rule the next number that is free
- A player notices: a rule added after a deletion works, where the original's is dead
- Where: `crates/mao-client/src/chatfilter.rs:16`; `crates/mao-client/src/chatfilter.rs:211`; `docs/ORIGINAL_CODE_QUESTIONS.md:153`
- Decision: [ ] port  [ ] leave

#### `CHT-009` Chat window tab config source differs

- OWN-RULE, low
- Original: the character's own Chat/Windows config
- Client: reads the original client folder's character, else the account's newest character, else the original's two defaults; kept in chatwindows.ini
- A player notices: tabs may come from another character's files
- Where: `crates/mao-client/src/chatgroups.rs:20`; `crates/mao-client/src/chatgroups.rs:512`; `docs/CLIENT_UI.md:297`
- Decision: [ ] port  [ ] leave

#### `CHT-010` Friend online/offline lines are the client's own notice

- OWN-RULE, low
- Original: not stated
- Client: tells "<name> is now online." / "has gone offline." only for a listed non-temporary friend whose state changed; the first listing and recent messages are not told
- A player notices: extra chat lines, in the client's wording
- Where: `crates/mao-client/src/net.rs:6228`; `docs/CLIENT_UI.md:1730`; `docs/CLIENT_UI.md:1963`
- Decision: [ ] port  [ ] leave

#### `CHT-011` Lines typed while the chat was down are dropped on reconnect

- OWN-RULE, low
- Original: not stated
- Client: discards what was typed while there was no connection instead of sending it late
- A player notices: a line typed during an outage is lost
- Where: `crates/mao-client/src/net.rs:6031`
- Decision: [ ] port  [ ] leave

#### `CHT-012` Macro text cut to 99 bytes, not 100

- OWN-RULE, low
- Original: CreateMacro (0x1002338f) cuts a name or command to 100 bytes, and Load (0x1002350b) gives up at the first one over 99, dropping it and every macro after it
- Client: cuts to 99 so what was saved is read again
- A player notices: a 100-byte macro is kept here; the original would lose it and later ones
- Where: `crates/mao-client/src/macros.rs:16`; `crates/mao-client/src/macros.rs:26`; `docs/ORIGINAL_CODE_QUESTIONS.md:144`
- Decision: [ ] port  [ ] leave

#### `CHT-013` Macros: own file, carried on the pointer; no macro from a chatcmd link

- OWN-RULE, low (notes only)
- Original: right button on a chatcmd:// link makes a macro (FUN_10065798); drag via the original's drag
- Client: file beside the client's settings; macro carried as egui drag payload and set down by a click; link right-click not done
- A player notices: macros dragged differently; cannot make one from a link
- Where: `docs/GUI_DLL_PARITY.md:144`
- Decision: [ ] port  [ ] leave

#### `CHT-014` Mail terminal looked up within 25 units

- OWN-RULE, low
- Original: not stated
- Client: nearest dynel-table terminal within 25 units, captured list as fallback
- A player notices: mail range is the client's
- Where: `crates/mao-client/src/mailbox.rs:20`; `crates/mao-client/src/mailbox.rs:41`; `docs/CLIENT_UI.md:1872`
- Decision: [ ] port  [ ] leave

#### `CHT-015` Texts fall back to the client's own words

- OWN-RULE, low
- Original: texts come from the game's text.mdb (10001, ChatCmdFeedback_*, text 110, 501)
- Client: where the install's file lacks an entry, the client uses its own wording (menus, chat settings, AFK, /org help, team lines, group commands); "Error: Chat group <name> is currently not available." is its own wording
- A player notices: wording may differ without the game files or where an entry is absent
- Where: `crates/mao-client/src/chatwindow.rs:403`; `crates/mao-client/src/chatwindow.rs:419`; `crates/mao-client/src/chatconfig.rs:33`; `crates/mao-client/src/afk.rs:28`; `crates/mao-client/src/orgcommands.rs:40`; `crates/mao-client/src/feedback.rs:130` and 3 more
- Decision: [ ] port  [ ] leave

### Chat and commands: an approximation or a stand-in (13)

#### `CHT-016` Fight lines cannot tell a pet hit or a player attacker

- SIMPLIFIED, medium
- Original: the feedback formatter (FUN_10012c3e) picks between two groups by whether the one hit is the client's pet, or whether the attacker is a player
- Client: does not know, takes the other group
- A player notices: some fight lines (pet fights) land in the wrong combat group and so the wrong chat tab
- Where: `crates/mao-client/src/chatgroups.rs:206`; `docs/CLIENT_UI.md:291`
- Decision: [ ] port  [ ] leave

#### `CHT-017` /inspect on objects and booths shows the client's own text

- SIMPLIFIED, low
- Original: a built-in terminal's description is shown as an itemid:// link
- Client: shows an itemref link for table things, a plain white name for other objects, and "Nothing targeted to inspect (Tab selects)" with no target
- A player notices: simpler info for objects than the original's
- Where: `crates/mao-client/src/net.rs:7468`
- Decision: [ ] port  [ ] leave

#### `CHT-018` /voice speaks only in the vicinity

- SIMPLIFIED, low
- Original: FUN_100b88b1 says the line in the chat window's selected group (Vicinity unless another is picked); also tags' doings, a tell's voice text
- Client: always the vicinity (no group selection; an earlier note said the organization channel, since replaced)
- A player notices: /voice cannot go to the team, guild or another channel
- Where: `crates/mao-client/src/net.rs:6418`; `crates/mao-client/src/voices.rs:22`; `docs/CLIENT_UI.md:2545`; `docs/ORIGINAL_AUDIT.md:98`
- Decision: [ ] port  [ ] leave

#### `CHT-019` Always behind / always on top stored with the window

- SIMPLIFIED, low
- Original: window flags 0x200/0x100 in VisualFlags
- Client: drawn in egui's background or foreground layer, kept with the window (the frame's: every window in it)
- A player notices: none visible
- Where: `crates/mao-client/src/chatwindow.rs:559`; `crates/mao-client/src/chatwindow.rs:622`; `docs/CLIENT_UI.md:394`
- Decision: [ ] port  [ ] leave

#### `CHT-020` Borderless chat window rule simplified

- SIMPLIFIED, low
- Original: FUN_10099a56 also compares the frame's name with a second, unidentified name
- Client: "alone in its frame" is the whole rule
- A player notices: borderless allowed in slightly different cases
- Where: `crates/mao-client/src/chatwindow.rs:550`; `crates/mao-client/src/chatwindow.rs:1607`; `docs/CLIENT_UI.md:380`
- Decision: [ ] port  [ ] leave

#### `CHT-021` Chat line plain-text conversion is a simplified markup reader

- SIMPLIFIED, low
- Original: the game's HTMLParser_c reads and renders the markup (fonts, links)
- Client: chat::plain strips tags and decodes only a few entities (lt, gt, quot, amp, decimal numbers), turns line breaks and tabs into spaces
- A player notices: other entities and formatting of a line are lost where plain text is used
- Where: `crates/mao-net/src/chat.rs:392`
- Decision: [ ] port  [ ] leave

#### `CHT-022` Chat server reconnect timing and give-up

- SIMPLIFIED, low
- Original: FUN_1008a2c1 makes the connection again and counts its attempts (the numbers are not stated)
- Client: retries after 5 s plus 10 s per failed try in a row, and stops for good after five tries that never logged in; "the timing is this client's"
- A player notices: after five failures the chat stays dead until the client is restarted (a line says so)
- Where: `crates/mao-client/src/net.rs:6018`; `docs/CLIENT_UI.md:1165`
- Decision: [ ] port  [ ] leave

#### `CHT-023` Chat transparency, fading and mode are simplified

- SIMPLIFIED, low
- Original: two sliders in the Visual menu; fading rules; mode rule
- Client: two lists in steps of ten; fading follows option names only; mode's rule simplified
- A player notices: coarser control of window transparency
- Where: `crates/mao-client/src/chatwindow.rs:198`; `docs/CLIENT_UI.md:383`; `docs/GUI_DLL_PARITY.md:75`
- Decision: [ ] port  [ ] leave

#### `CHT-024` Emotes: lounge/sleep need the camp's sitting; swimming and vehicle checks absent

- SIMPLIFIED, low (notes only)
- Original: emote checks for swimming and vehicles
- Client: sitting is the camp's; swimming and vehicle checks not done (client knows neither)
- A player notices: emotes allowed where the original refuses
- Where: `docs/ORIGINAL_AUDIT.md:112`
- Decision: [ ] port  [ ] leave

#### `CHT-025` Tells of other kinds shown as tells

- SIMPLIFIED, low
- Original: shows only the kinds 0, 2 and 6
- Client: a tell of another kind is shown as a tell
- A player notices: extra tells shown
- Where: `crates/mao-client/src/net.rs:6186`; `docs/GUI_DLL_PARITY.md:291`
- Decision: [ ] port  [ ] leave

#### `CHT-026` Text database strings read byte by byte

- SIMPLIFIED, low
- Original: not stated
- Client: every text in text.mdb becomes a string by turning each byte into the character of the same number
- A player notices: non-ASCII letters in game texts may show wrong
- Where: `crates/mao-formats/src/ldb.rs:40`
- Decision: [ ] port  [ ] leave

#### `CHT-027` Text formatting fills arguments by a simplified rule

- SIMPLIFIED, low
- Original: LDBformat::Feed prints each argument by its full printf conversion and handles time and float arguments
- Client: fills by argument order with texts and integers only; %s %d %i %u print plainly (width and flags ignored); a choice picks flags only for 0 and 1; no time or float feeds
- A player notices: texts with widths, %x, times or floats print plainly or wrongly
- Where: `crates/mao-formats/src/ldb.rs:219`; `crates/mao-formats/src/ldb.rs:74`; `docs/ORIGINAL_AUDIT.md:20`
- Decision: [ ] port  [ ] leave

#### `CHT-028` Unknown sender shown by hex id

- SIMPLIFIED, low
- Original: not stated
- Client: a sender whose name is not known yet is shown as its id in hex
- A player notices: some chat lines show a number instead of a name
- Where: `crates/mao-client/src/net.rs:6182`
- Decision: [ ] port  [ ] leave

### Chat and commands: the original was not read: inferred or measured (16)

#### `CHT-029` /cc info and /cc commands built without a capture

- GUESS, low
- Original: GUI.dll FUN_100ba6f4, FUN_100baba6 and FUN_10089940
- Client: /cc info (packet 110) and /cc commands (120) built from the disassembly; what the server answers to /cc info was never seen
- A player notices: /cc info may show nothing
- Where: `crates/mao-net/src/chat.rs:290`
- Decision: [ ] port  [ ] leave

#### `CHT-030` /group and /ch say "There is no chat group named" in the client's words

- GUESS, low
- Original: not stated
- Client: a fixed English line for an unknown group name
- A player notices: possibly different wording from the original
- Where: `crates/mao-client/src/chatwindow.rs:1003`
- Decision: [ ] port  [ ] leave

#### `CHT-031` /viewdist, /chardist and /char&viewdist limits and wording

- GUESS, low
- Original: not stated (GUI.dll FUN_100b6a54 cited for both distances)
- Client: view distance clamped to 200-4000, character distance to at most 4000; own lines "View distance ..., characters up to ... away."
- A player notices: values over the limits are cut
- Where: `crates/mao-client/src/chatwindow.rs:1265`
- Decision: [ ] port  [ ] leave

#### `CHT-032` AFK box answers use fixed English lines

- GUESS, low
- Original: not stated (texts come from text.mdb where the install has them)
- Client: "AFK off.", "Using default AFK message." and the emote "is back" are fixed strings
- A player notices: wording may differ from the original
- Where: `crates/mao-client/src/afk.rs:162`; `crates/mao-client/src/afk.rs:166`
- Decision: [ ] port  [ ] leave

#### `CHT-033` Channel message sent with a one-space blob

- GUESS, low
- Original: GUI.dll FUN_1008a2c1 sends the group message with a voice record as blob
- Client: a plain channel line sends a one-space text as blob
- A player notices: none visible
- Where: `crates/mao-net/src/chat.rs:360`
- Decision: [ ] port  [ ] leave

#### `CHT-034` Chat message fading is the client's reading

- GUESS, low
- Original: how FadingTextView_c uses ChatTextFadeDelay/Time was not read
- Client: while a window is not active, lines fade after the delay over the fade time, by the client's own reading of the option names
- A player notices: fade may differ
- Where: `crates/mao-client/src/chatwindow.rs:1516`; `crates/mao-client/src/ui.rs:1122`; `docs/CLIENT_UI.md:391`
- Decision: [ ] port  [ ] leave

#### `CHT-035` Command list is drawn and completed in the client's own way

- GUESS, low
- Original: the original lists commands when / is typed (FUN_100a44e8)
- Client: its own egui list with fixed row size, colours and keys (Up, Down, Tab, Enter takes the highlighted one) and a hint text "Type to speak, / for commands"
- A player notices: look and keys of the command list may differ
- Where: `crates/mao-client/src/chatwindow.rs:1442`
- Decision: [ ] port  [ ] leave

#### `CHT-036` Declined mail gets the client's own explanation

- GUESS, low
- Original: not stated
- Client: a notice from the server right after a send is shown as "sending is limited to paying accounts"; action 0xba shows a "message code" line
- A player notices: the player sees the client's guess at why the mail failed, not the original's text
- Where: `crates/mao-client/src/net.rs:6797`
- Decision: [ ] port  [ ] leave

#### `CHT-037` Leave-organization box line is the client's own words

- GUESS, low
- Original: the line is in the program, not in the game's texts (its wording not stated)
- Client: "Do you want to leave the organization? For six hours afterwards you cannot join another."
- A player notices: wording may differ
- Where: `crates/mao-client/src/orgboxes.rs:22`; `docs/CLIENT_UI.md:4228`
- Decision: [ ] port  [ ] leave

#### `CHT-038` Mail return-to-sender built from the code only

- GUESS, low
- Original: N3Msg_ReturnMail (Gamecode 0x10023a8c)
- Client: no capture of a return exists; the unread fields of an opened mail (two words after the item) are skipped
- A player notices: return may not work
- Where: `crates/mao-net/src/mail.rs:10`
- Decision: [ ] port  [ ] leave

#### `CHT-039` Name link qualifier keys are a guess

- GUESS, low
- Original: FUN_100ac4c4 uses the qualifiers' low two bits (FUN_100ac65f handlers)
- Client: taken as Shift and Ctrl, "a guess at which keys they are"
- A player notices: modifier clicks on names may do different things
- Where: `crates/mao-client/src/chatwindow.rs:11`; `docs/CLIENT_UI.md:1158`; `docs/ORIGINAL_AUDIT.md:32`
- Decision: [ ] port  [ ] leave

#### `CHT-040` Private group line sent with a zero-byte blob

- GUESS, low
- Original: not stated
- Client: a private group line is "laid out as a tell is" with a single zero byte blob
- A player notices: none visible
- Where: `crates/mao-net/src/chat.rs:347`
- Decision: [ ] port  [ ] leave

#### `CHT-041` Script words are split at blanks only

- GUESS, low
- Original: String::Tokenize treats quotes in some way not read in the script path
- Client: words are split at blanks
- A player notices: quoted words in a script call may split differently
- Where: `crates/mao-client/src/scripts.rs:19`
- Decision: [ ] port  [ ] leave

#### `CHT-042` Sent-line history of 100 lines and chat buffer of 600 lines

- GUESS, low
- Original: not stated
- Client: Up and Down recall up to 100 lines; 600 chat lines are kept
- A player notices: older lines are lost
- Where: `crates/mao-client/src/chatwindow.rs:313`; `crates/mao-client/src/chatwindow.rs:952`
- Decision: [ ] port  [ ] leave

#### `CHT-043` Team leader guessed as the first member when none is named

- GUESS, low
- Original: the leader is the one the server's message 0x17 names
- Client: until it is heard, the first member is shown as the leader
- A player notices: the leader colour and Recruit button may be on the wrong person for a moment
- Where: `crates/mao-client/src/team.rs:91`
- Decision: [ ] port  [ ] leave

#### `CHT-044` Team level before the server says it is the highest member's

- GUESS, low
- Original: GetTeamLevel from interface message 0x10
- Client: before it is told, shows the highest member's level
- A player notices: header level may differ briefly
- Where: `crates/mao-client/src/team.rs:224`
- Decision: [ ] port  [ ] leave

### Chat and commands: a guard or fallback the original lacks (3)

#### `CHT-045` Chat lines with control characters are dropped

- SAFETY, low
- Original: not stated
- Client: any chat line whose text holds a control character is dropped (a packet of another layout read wrongly)
- A player notices: such a server line never shows
- Where: `crates/mao-client/src/net.rs:6301`
- Decision: [ ] port  [ ] leave

#### `CHT-046` Mail inbox list capped at 64 entries

- SAFETY, low
- Original: not stated
- Client: parse_list reads at most 64 inbox entries
- A player notices: a mailbox of more than 64 mails shows only the first 64
- Where: `crates/mao-net/src/mail.rs:119`
- Decision: [ ] port  [ ] leave

#### `CHT-047` Script names with a backslash, colon or dots are refused

- SAFETY, low
- Original: not stated
- Client: a name must be a plain file name in the folder
- A player notices: none visible normally
- Where: `crates/mao-client/src/scripts.rs:127`
- Decision: [ ] port  [ ] leave

### Chat and commands: the original does it, the client does not (21)

#### `CHT-048` /raid opens the raid window instead of running raid commands

- MISSING, high
- Original: /raid <command> (create, list, listlocal, move, lootaccess, locks) sends RaidCmdIIR_c (4e525e58); it does not open a window
- Client: /raid (with or without words) opens the client's raid window; the raid commands are not carried out
- A player notices: /raid create, /raid list and the others do nothing; a window appears instead
- Where: `crates/mao-client/src/chat_commands.rs:382`; `docs/GUI_DLL_PARITY.md:255`; `docs/SLASH_COMMANDS_STATUS.md:12`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): `/raid` is the raid's commands as Gamecode's interpreter reads them (`raidcommands.rs`): create, list and locks send `RaidCmdIIR_c` with no check, move and lootaccess with their numbers, listlocal says the team, another word does nothing; the server's `RaidIIR_c` is read (the raid made; the lock list, said in chat). No raid message was ever sent to a server: the bytes are the code's alone. The raid window is `/open raid` and the control center's menu now.

#### `CHT-049` Raid window shows only the own team and its buttons do nothing

- MISSING, high
- Original: raid window with six teams and working "Convert To Raid" and "Raid Locks" buttons; raid bars view (RaidBar.xml)
- Client: shows own team as Team 1, others empty; the two buttons send nothing; the raid bars are not drawn
- A player notices: raids cannot be made or seen
- Where: `crates/mao-client/src/raid.rs:6`; `crates/mao-client/src/raid.rs:5`; `docs/GUI_DLL_PARITY.md:92`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the two buttons send the raid command (`N3Msg_CreateRaid` for a team's leader, `N3Msg_RequestRaidLocks`), "Convert To Raid" is enabled in a team that is no raid, and in a raid all six teams are shown from the server's lists (`team.rs`: the world-entry message's raid part, `TeamMember`'s place, 0x20's place, `RaidIIR_c` type 0); outside a raid the teams are empty, as the original's. Found and mended: zoning in a team emptied the perk list (the world-entry message's team flags were misread), and a team member's profession was read wrongly. No capture has a raid: its data is the code's alone. Not ported: a line's menu, dragging a member to another team, the raid bars.

#### `CHT-050` Friends list: no flashing row for unread tells, no red folders, tell windows not under Chat Windows, no tell-window menu

- MISSING, medium
- Original: FriendListView_c flashes a row for an unread tell, recolours the Online/Offline/Recent folder labels red (0xff6666) for unread messages (FUN_100a86f6), lists tell windows under "Chat Windows", has the tell window's own menu (FUN_100a8a69), and makes group names links (chatgroup://, FUN_100ac4f0)
- Client: none of these; the "Chat Windows" folder lists the client's one chat window; no messages are kept per friend
- A player notices: an incoming tell does not flash in the friends list
- Where: `crates/mao-client/src/friends.rs:15`; `crates/mao-client/src/friends.rs:20`; `docs/CLIENT_UI.md:1174`; `docs/CLIENT_UI.md:1738`; `docs/GUI_DLL_PARITY.md:85`; `docs/ORIGINAL_CODE_QUESTIONS.md:61` and 1 more
- Decision: [ ] port  [ ] leave

#### `CHT-051` Private group invitation box has no chat-window list

- MISSING, medium
- Original: the question (FUN_100a76c9) offers a list of chat windows to put the group in; at ChatPGInviteAction 2 it only marks the invitation in the friends list
- Client: the box always comes, with no list; windows that take new channels by themselves show the group
- A player notices: you cannot choose the window for a private chat group
- Where: `crates/mao-client/src/chatwindow.rs:1737`; `docs/CLIENT_UI.md:349`
- Decision: [ ] port  [ ] leave

#### `CHT-052` /bank info and /items show no answer

- MISSING, low
- Original: the server answers (it may be a message the client does not show)
- Client: /bank info and /items are sent and not refused; nothing is shown in answer. /items with a word other than list is not carried out
- A player notices: no output for those commands
- Where: `crates/mao-client/src/chatwindow.rs:1243`; `crates/mao-client/src/chat_commands.rs:468`; `docs/LIVE_TEST_LIST.md:496`; `docs/LIVE_TEST_LIST.md:517`
- Decision: [ ] port  [ ] leave

#### `CHT-053` /bank open does nothing

- MISSING, low
- Original: /bank open is a game master's command (Gamecode FUN_1003fe10)
- Client: nothing happens
- A player notices: none visible for a normal player
- Where: `crates/mao-client/src/chatwindow.rs:1239`
- Decision: [ ] port  [ ] leave

#### `CHT-054` /cc info <name> gets no answer

- MISSING, low
- Original: says in a line of the program's that it is asking; what the server answers is not known
- Client: sends the request (packet 110) and prints a line in the client's words; nothing reads an answer, and a live test (2026-11-16) got none; probably not a player's command
- A player notices: /cc info prints nothing useful
- Where: `crates/mao-client/src/chatwindow.rs:1283`; `crates/mao-client/src/net.rs:6369`; `docs/CLIENT_UI.md:4254`; `docs/CLIENT_UI.md:4455`; `docs/LIVE_TEST_LIST.md:289`
- Decision: [ ] port  [ ] leave

#### `CHT-055` /reclaim is not carried out

- MISSING, low
- Original: opens the reclaim window (N3Msg_ToggleReclaim) at a reclaim terminal
- Client: the command is listed but falls to Unsupported; the client has no reclaim window
- A player notices: /reclaim does nothing
- Where: `crates/mao-client/src/chat_commands.rs:621`; `docs/LIVE_TEST_LIST.md:527`; `docs/SLASH_COMMANDS_STATUS.md:14`
- Decision: [ ] port  [ ] leave

#### `CHT-056` /start does not ask first when full screen

- MISSING, low
- Original: /start asks first when the game is full screen
- Client: opens an http/https address without a question
- A player notices: link opens without a question
- Where: `crates/mao-client/src/chatwindow.rs:1184`; `docs/GUI_DLL_PARITY.md:160`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): in the exclusive full-screen mode a Warning box with text 10000 ReallyMinimizeGame asks first (chatwindow::EXCLUSIVE); the usage line for a missing address is not done.

#### `CHT-057` /tower, /terminate (Notum Wars) are not carried out

- MISSING, low
- Original: /tower needs a service tower and runs a script (FUN_100b375a; words such as create); /terminate asks in a yes/no box first (FUN_100b3a69)
- Client: both parse as an unsupported command; /tower <word> and /terminate do nothing (docs also list /option beyond /setoption as missing)
- A player notices: typing /tower or /terminate does nothing useful
- Where: `crates/mao-client/src/chat_commands.rs:416`; `crates/mao-client/src/chat_commands.rs:759`; `docs/GUI_DLL_PARITY.md:170`; `docs/LIVE_TEST_LIST.md:498`; `docs/SLASH_COMMANDS_STATUS.md:13`; `crates/mao-client/src/chat_commands.rs:621` and 2 more
- Decision: [ ] port  [ ] leave

#### `CHT-058` Away does not leave a battle station queue; AFK box title is the client's

- MISSING, low
- Original: SetAFK (0x10087669) leaves a battle station's queue; the AFK window's flags 0x800 not followed to what the bar shows
- Client: nothing (the client has no battle station queue); the box title "AFK" is the client's own
- A player notices: none visible
- Where: `crates/mao-client/src/afk.rs:6`; `docs/GUI_DLL_PARITY.md:283`
- Decision: [ ] port  [ ] leave

#### `CHT-059` Chat configuration: click-through, deactivate on send, default-window options, menu separators

- MISSING, low (notes only)
- Original: the configuration rows' click-through, deactivate on send, default window options, menu separators
- Client: not found in the code (the friends-list note also lists click-through as not done)
- A player notices: missing options
- Where: `docs/CLIENT_UI.md:397`; `crates/mao-client/src/friends.rs:20`
- Decision: [ ] port  [ ] leave

#### `CHT-060` Chat configuration: no window-row menu, no disbanding one's own private group

- MISSING, low
- Original: a window row's own menu (style, show, rename, remove) and disbanding one's own private group
- Client: not done
- A player notices: missing options in the configuration window
- Where: `crates/mao-client/src/chatconfig.rs:17`; `docs/CLIENT_UI.md:340`; `crates/mao-client/src/chatconfig.rs:19`
- Decision: [ ] port  [ ] leave

#### `CHT-061` Chat indicator's head flash not done

- MISSING, low (notes only)
- Original: the chat indicator flashes the head on vicinity speech (the %t/%f text macros are done since: chat_commands.rs:272)
- Client: no head flash found in the code
- A player notices: no head flash
- Where: `docs/ORIGINAL_AUDIT.md:99`; `crates/mao-client/src/chat_commands.rs:272`
- Decision: [ ] port  [ ] leave

#### `CHT-062` chatgroup:// links not made

- MISSING, low (notes only)
- Original: chat lines carry chatgroup:// links
- Client: not made (no "chatgroup://" found in crates)
- A player notices: group names in chat are not clickable
- Where: `docs/ORIGINAL_AUDIT.md:32`
- Decision: [ ] port  [ ] leave

#### `CHT-063` One combined chat log, no message-log window

- MISSING, low
- Original: each chat window logs by itself; "Show message log" shows a log window
- Client: every line goes to one text file a day (UTC) in a chat folder beside mao.ini; "Show the message log" is drawn disabled
- A player notices: no per-window logs and no in-game log viewer
- Where: `crates/mao-client/src/chatconfig.rs:18`; `crates/mao-client/src/chatlog.rs:1`; `docs/CLIENT_UI.md:334`; `docs/CLIENT_UI.md:1816`
- Decision: [ ] port  [ ] leave

#### `CHT-064` Other chat frames' places not taken from the original's files

- MISSING, low (notes only)
- Original: its own files give the other frames' places
- Client: the windows and tab order are read from the original's Config.xml; the frames' places could not be confirmed
- A player notices: frame placement differs
- Where: `docs/CLIENT_UI.md:368`; `crates/mao-client/src/chatgroups.rs:453`
- Decision: [ ] port  [ ] leave

#### `CHT-065` Server lines for a named window go to the system lines

- MISSING, low
- Original: ChatGUIModule_c::HandleSystemMessage (0x1008633c) sends a line to the window it is for
- Client: the window is not followed; the line goes where system lines go
- A player notices: a server line meant for a given window shows in the system lines instead
- Where: `crates/mao-client/src/net.rs:6277`; `docs/CLIENT_UI.md:4468`
- Decision: [ ] port  [ ] leave

#### `CHT-066` Unknown chat server packets are ignored

- MISSING, low
- Original: GUI.dll's parser (FUN_1016d6f4) reads every chat server packet
- Client: packets not decoded (e.g. 40/41 forms, 61, the channel membership flag) are returned as "Other" and only logged
- A player notices: chat server events with no decoder show nothing
- Where: `crates/mao-client/src/net.rs:6295`; `crates/mao-net/src/chat.rs:16`
- Decision: [ ] port  [ ] leave

#### `CHT-067` Voice lines ignore the ignore list and the tags' doings

- MISSING, low
- Original: a sender on the ignore list is heard only when record 2's flag has bit 0; tag T flashes the speaker's name in the team window, M signals GlobalSignals_c +0x150
- Client: not done
- A player notices: voice lines of ignored players may still play; no name flash in the team window
- Where: `crates/mao-client/src/voices.rs:25`
- Decision: [ ] port  [ ] leave

#### `CHT-068` Words-per-command table only covers the commands that were read

- MISSING, low
- Original: each command is registered with a number of pieces its line is cut into
- Client: only the commands read are in the table; the organization's and the team's were not read
- A player notices: /org and /team lines may be cut differently from the original's
- Where: `crates/mao-client/src/chat_commands.rs:317`
- Decision: [ ] port  [ ] leave

### Chat and commands: the client has it, the original does not (12)

#### `CHT-069` Chat commands of the client's own

- ADDITION, medium
- Original: not in the original's command list (the original's Befriend file is a script `/cc addbuddy %1`, a /cc line)
- Client: adds /befriend, /sit, /pet, /perf, /damage (/dps), /find, /crafting (/craft), /follow, /inspect, /open, /close, /items, /waypoint, /selectself and window open/close by name ("No window called ..." when unknown); /follow walks client-side and the server is told nothing; view and character distance commands
- A player notices: extra commands that open the client's own tools or act on the client only
- Where: `crates/mao-client/src/chat_commands.rs:250`; `crates/mao-client/src/viewer.rs:2812`; `docs/CLIENT_UI.md:1910`; `docs/ORIGINAL_AUDIT.md:120`
- Decision: [ ] keep  [ ] remove

#### `CHT-070` Use key adds a "Using X" line and a "Nothing targeted to use" notice

- ADDITION, medium
- Original: not stated
- Client: every use shows "Using <name>" or "Nothing targeted to use (Tab selects)"
- A player notices: extra chat lines on each use
- Where: `crates/mao-client/src/net.rs:7758`
- Decision: [ ] keep  [ ] remove

#### `CHT-071` "Chat server is not connected" line

- ADDITION, low
- Original: not stated
- Client: a system line says "The chat server is not connected."
- A player notices: extra message
- Where: `crates/mao-client/src/net.rs:1047`
- Decision: [ ] keep  [ ] remove

#### `CHT-072` /fxscript with only a name gets a parameter mark added

- ADDITION, low
- Original: a script runs only when a parameters part follows the name; the reader's complaints go to the original's log
- Client: when the name stands alone, a lone "#" is added so the script runs; started scripts and reader complaints are reported in chat
- A player notices: /fxscript <name> runs the script where the original would not; extra feedback lines
- Where: `crates/mao-client/src/chat_commands.rs:434`; `docs/CLIENT_UI.md:1253`
- Decision: [ ] keep  [ ] remove

#### `CHT-073` /setoption names the client's own settings

- ADDITION, low
- Original: /setoption sets distributed values; the option panel has no control for AutoAFK
- Client: /setoption sets vsync, render_scale, sharpness, antialiasing, ui_scale, view_distance, char_distance, auto_loot, camera_mode, log_messages (and AutoAFK)
- A player notices: display options and AutoAFK changeable from chat
- Where: `crates/mao-client/src/settings.rs:668`; `crates/mao-client/src/afk.rs:29`; `docs/CLIENT_UI.md:3554`; `docs/GUI_DLL_PARITY.md:306`
- Decision: [ ] keep  [ ] remove

#### `CHT-074` Client's own chat status lines

- ADDITION, low
- Original: not stated
- Client: says "Connected to the chat server again.", "The chat connection ended: ...", "The chat server cannot be reached: no more tries..." and "Asking the chat server about <name>." in the chat window
- A player notices: extra system lines in chat
- Where: `crates/mao-client/src/net.rs:6033`
- Decision: [ ] keep  [ ] remove

#### `CHT-075` Deposit to Market adds a client line

- ADDITION, low
- Original: not stated
- Client: shows "Sent N items and X credits to the market."
- A player notices: an extra chat line
- Where: `crates/mao-client/src/net.rs:8247`
- Decision: [ ] keep  [ ] remove

#### `CHT-076` Inspect result also told as a chat line

- ADDITION, low
- Original: opens the character info page
- Client: also adds a system chat line with name, level and health
- A player notices: extra line in chat after inspecting
- Where: `crates/mao-client/src/net.rs:2669`
- Decision: [ ] keep  [ ] remove

#### `CHT-077` New mail needs a recipient and subject (client's own check)

- ADDITION, low
- Original: Sending (FUN_1010733d) checks only negative credits, text/subject length, and asks for money/C.O.D.
- Client: refuses an empty recipient or subject with "A mail needs a recipient and a subject."
- A player notices: a mail with no subject cannot be sent from the client
- Where: `crates/mao-client/src/mailmessage.rs:278`
- Decision: [ ] keep  [ ] remove

#### `CHT-078` Own confirmation lines for several commands

- ADDITION, low
- Original: the original says nothing (e.g. /gfx); not stated for the others
- Client: /macro, /waypoint, /befriend, /viewdist, /chardist, /char&viewdist, /ignore, /bug, /reply with nobody add a system line of the client's own
- A player notices: extra feedback lines in chat
- Where: `crates/mao-client/src/chatwindow.rs:1170`
- Decision: [ ] keep  [ ] remove

#### `CHT-079` Scripts folder beside the client's settings is searched first

- ADDITION, low
- Original: scripts folder under %LOCALAPPDATA%/Funcom/Anarchy Online/<install>/Anarchy Online/scripts
- Client: looks in a `scripts` folder beside the client's settings first, then the original's
- A player notices: scripts can come from a different folder
- Where: `crates/mao-client/src/scripts.rs:17`; `crates/mao-client/src/scripts.rs:53`; `docs/GUI_DLL_PARITY.md:152`; `docs/CLIENT_UI.md:854`; `docs/LIVE_TEST_LIST.md:474`
- Decision: [ ] keep  [ ] remove

#### `CHT-080` Team invitation text is the client's own

- ADDITION, low
- Original: not stated
- Client: 'X invites you to join a team.' with fallback name 'Character 0x...'
- A player notices: invitation line may differ from the original
- Where: `crates/mao-client/src/net.rs:2853`
- Decision: [ ] keep  [ ] remove

## Windows and interface

### Windows and interface: the client knowingly does otherwise (21)

#### `WIN-001` Shortcut bars are the client's own design

- OWN-RULE, high
- Original: shortcutbar_window in the original's behaviour (not otherwise stated).
- Client: hotbar.rs is the client's own smart hotbar: queued presses, cooldown sweeps, Alt+1-6 pick a bar and number keys press slots, the bar is dragged by a grip, turned vertical, edited by right click, items dropped onto it, with a 46 px look without a skin.
- A player notices: Shortcut bars look and behave differently; key assignments may differ.
- Where: `crates/mao-client/src/ui.rs:1922`; `crates/mao-client/src/ui.rs:1962`; `crates/mao-client/src/settings.rs:255`; `docs/GUI_DLL_PARITY.md:84`; `docs/CLIENT_UI.md:2060`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the original's shortcut bars are the default (`shortcutbar.rs`): up to ten bars of ten slots over ten pages, a primary bar, the game's keys (1-0, Shift for the page, Alt for the primary), its art, moving, turning upright, the wheel, dropping and dragging out, the timer text and the first run's shortcuts. The smart hotbars stay as a setting. Not ported: reading the original's ShortcutBar_<n>.xml, Shift+click info, the red unusable fade, stack counts, new nanos put on the bar, the bars' close.

#### `WIN-002` Dark themes draw controls in the border colour instead of the skin's Default

- OWN-RULE, medium
- Original: colour code 0x1000000 is the skin's Default colour (GUIColors.xml); not checked against the original running a dark skin
- Client: on a dark theme (the AOU Translucent's Default is near black) colour code 1 is drawn with the palette's border colour (chrome::code1), TEXTHOVER maps to the hover colour, and inactive tab text turns the theme's text colour
- A player notices: buttons, check boxes and tabs under dark skins are drawn lighter than the skin states
- Where: `crates/mao-client/src/gui/controls.rs:50`; `crates/mao-client/src/chrome.rs:528`; `docs/CLIENT_UI.md:2036`
- Decision: [ ] port  [ ] leave

#### `WIN-003` Grid columns centred in the width

- OWN-RULE, medium
- Original: The first column is BORDER (six pixels) in from the left whatever the width, leaving the remainder at the right.
- Client: Columns that fill the width are centred ("this client's own", from the user's complaint 2026-11-11).
- A player notices: Bags and inventory grids are centred instead of left-aligned.
- Where: `crates/mao-client/src/gui/multilist.rs:197`; `docs/CLIENT_UI.md:1265`
- Decision: [ ] port  [ ] leave

#### `WIN-004` In-game web pages are a WebView2 overlay behind a local proxy

- OWN-RULE, medium
- Original: GUI.dll BrowserWindow_c shows an Awesomium page and sends the game's headers through AddHTTPHeaderRule
- Client: a WebView2 (wry) page laid over the game window; every request goes through an `aomarket://` proxy that forwards it with the game's headers and rewrites addresses, cookies and compressed bodies; Home, back, forward buttons; a failed site shows the client's own "The market could not be reached" page
- A player notices: pages look and behave as a modern browser, with extra buttons
- Where: `docs/CLIENT_UI.md:243`; `crates/mao-client/src/browser.rs:220`; `crates/mao-client/src/browser.rs`; `docs/GUI_DLL_PARITY.md:64`
- Decision: [ ] port  [ ] leave

#### `WIN-005` Map: interiors and instanced playfields get the client's own explored-grid map

- OWN-RULE, medium
- Original: for a playfield of rooms the map draws through FUN_100474b7, and for ground the ground from above (FUN_10047964); a planet map for each playfield
- Client: where there is no ground the map is the client's own (fog of war, in the style of Diablo 2 and 3): nothing drawn until walked past, walls as bright outlines on a faint floor; it also stays for playfields without a planet map
- A player notices: interiors show a self-revealing outline map instead of the original's picture
- Where: `docs/ORIGINAL_AUDIT.md:211`; `crates/mao-client/src/automap.rs:8`; `crates/mao-client/src/automap.rs:412`; `docs/ORIGINAL_AUDIT.md:63`
- Decision: [ ] port  [ ] leave

#### `WIN-006` Wear window grid scaled to fill the window

- OWN-RULE, medium
- Original: the wear cells keep their size (3 x 5 grid, 47 by 47, icon spacing 11 x 9) and the window is exactly as wide as three of them
- Client: cells, spacing and pictures are grown by a scale (GridMode::scale, "this client's own", asked by the user 2026-11-11) so the squares fill the window or rollup page; the inspect window not photographed
- A player notices: wear and inspect cells are larger than the original's
- Where: `crates/mao-client/src/gui/multilist.rs:157`; `crates/mao-client/src/invwindows.rs:203`; `docs/CLIENT_UI.md:1119`
- Decision: [ ] port  [ ] leave

#### `WIN-007` Closed web pages are kept hidden and opened private instead of clearing cookies

- OWN-RULE, low
- Original: clears cookies each time it sets its browser up (awe_webcore_clear_cookies after awe_webcore_initialize, GUI.dll Reinitialize 0x1010bbd7)
- Client: closing a web window hides the WebView2 page instead of dropping it (dropping ended the session and the site refused the ticket a second time); the view is opened incognito
- A player notices: web pages persist while hidden; reopening shows the home page
- Where: `docs/CLIENT_UI.md:3231`; `docs/CLIENT_UI.md:3463`; `crates/mao-client/src/browser.rs:402`; `crates/mao-client/src/browser.rs:456`
- Decision: [ ] port  [ ] leave

#### `WIN-008` Fixed dialogs and the client's own windows keep no place

- OWN-RULE, low
- Original: DialogBox_c has no config; other windows keep theirs
- Client: Login, LoginBrowser, Progress, NewCharacter, Delete Character, Options and Whom-Pah keep no place
- A player notices: those windows always open at the centre
- Where: `crates/mao-client/src/layout.rs:27`
- Decision: [ ] port  [ ] leave

#### `WIN-009` Inbox size shows the server's count, not the list's row count

- OWN-RULE, low
- Original: FUN_10109489 writes InboxSize as "%d/%d" with the rows in the list view and the constant 30
- Client: shows the server's count when it has one
- A player notices: the count may differ when rows are not yet loaded
- Where: `docs/ORIGINAL_CODE_QUESTIONS.md:39`; `crates/mao-client/src/mailbox.rs:232`
- Decision: [ ] port  [ ] leave

#### `WIN-010` Info view window opens at the client's own size and title

- OWN-RULE, low
- Original: not stated
- Client: The XML window opens at 360x480, titled "Info".
- A player notices: Window size on first open.
- Where: `crates/mao-client/src/gui/infoview.rs:156`
- Decision: [ ] port  [ ] leave

#### `WIN-011` Land control view: district text placed under the button

- OWN-RULE, low
- Original: puts the "Lvl Rng" text at 5, 5 of the map
- Client: puts it under the land-control button in the map overlay
- A player notices: district range text in a different place
- Where: `docs/CAPTURE_GAPS.md:214`; `crates/mao-client/src/automap.rs:699`
- Decision: [ ] port  [ ] leave

#### `WIN-012` Link words stretched so a click anywhere chooses

- OWN-RULE, low
- Original: not stated
- Client: a link's words are stretched over the spaces between them so a click anywhere in an NPC answer chooses it
- A player notices: bigger click targets
- Where: `docs/CLIENT_UI.md:1562`; `crates/mao-client/src/gui/controls.rs:818`
- Decision: [ ] port  [ ] leave

#### `WIN-013` Mail without a recipient or subject is not sent

- OWN-RULE, low
- Original: not stated
- Client: a mail without a recipient or a subject is not sent (this client's own)
- A player notices: mail cannot be sent without a subject
- Where: `docs/CLIENT_UI.md:4369`; `crates/mao-client/src/mailmessage.rs:277`
- Decision: [ ] port  [ ] leave

#### `WIN-014` Mission terminal with no offers gets the client's own sentence

- OWN-RULE, low
- Original: GUI.dll FUN_100d0e71 says so in the window (in its own words).
- Client: Shows "The terminal has no missions just now; try again later."
- A player notices: Different wording from the original's window text.
- Where: `crates/mao-client/src/net.rs:6981`; `docs/CLIENT_UI.md:941`
- Decision: [ ] port  [ ] leave

#### `WIN-015` Mission window uses the terminal first when opened another way

- OWN-RULE, low
- Original: The Request Missions button sends the request alone (FUN_100d09cb); the terminal was used to open the window.
- Client: Sends a use of the terminal (and a heading report) before the request when the window was opened another way.
- A player notices: Missions can be requested from a window not opened at the booth.
- Where: `crates/mao-client/src/net.rs:8089`; `docs/CLIENT_UI.md:4038`
- Decision: [ ] port  [ ] leave

#### `WIN-016` Nano programs window docked layout is the client's own

- OWN-RULE, low
- Original: SchoolSelector_c lays its buttons in rows as wide as the window (FUN_100d6051) and the grid takes as many columns as fit.
- Client: Docked in the rollup column it uses two buttons a row and four grid columns; free it uses one row and six columns.
- A player notices: The layout does not follow the window's width.
- Where: `crates/mao-client/src/nanoprograms.rs:290`
- Decision: [ ] port  [ ] leave

#### `WIN-017` Original's shortcut bar frames not used

- OWN-RULE, low
- Original: Containers/ShortcutBar_<n>.xml hold the shortcut bars' frames
- Client: "not used: the bars here are the client's own"
- A player notices: hotbars do not start where the original's did
- Where: `crates/mao-client/src/aoprefs.rs:11`
- Decision: [ ] port  [ ] leave

#### `WIN-018` Scroll bars tinted with the theme's accent colour

- OWN-RULE, low
- Original: not stated
- Client: the XML scroll bars' pictures are tinted with the palette's accent colour
- A player notices: scroll bar colours follow the client's theme
- Where: `crates/mao-client/src/gui/scroll.rs:174`
- Decision: [ ] port  [ ] leave

#### `WIN-019` TextButton sized to the widest of its three texts

- OWN-RULE, low
- Original: TextButton_c::CalculatePreferredSize (0x10004e3f) is the shown text's own size.
- Client: Takes the widest of the raised, pressed and hover texts so the size does not jump.
- A player notices: Text buttons keep a fixed size as the pointer comes and goes.
- Where: `crates/mao-client/src/gui/textbutton.rs:69`
- Decision: [ ] port  [ ] leave

#### `WIN-020` Tip patterns: `*x` matched as "ends with x" instead of a prefix match

- OWN-RULE, low
- Original: TipSystem_t matcher FUN_100539be compares `*x` from the start of the data (a prefix match, same as `x*`)
- Client: tips.rs `matches` treats `*x` as "ends with x", the evident intent
- A player notices: eleven tips fire on items/playfields ending with the text, where the original fires only on data starting with it
- Where: `docs/ORIGINAL_CODE_QUESTIONS.md:23`; `crates/mao-client/src/tips.rs:69`
- Decision: [ ] port  [ ] leave

#### `WIN-021` Windows can be dragged anywhere; pin/dock hints in the client's words

- OWN-RULE, low
- Original: Windows are moved by their title bar.
- Client: Dragging anywhere on a skinned window moves it; pin, dock and close buttons carry the client's own hover texts.
- A player notices: A window can be dragged by its body.
- Where: `crates/mao-client/src/chrome.rs:4`
- Decision: [ ] port  [ ] leave

### Windows and interface: an approximation or a stand-in (61)

#### `WIN-022` Actions window lists a fixed catalogue, not the character's own list

- SIMPLIFIED, high
- Original: The list is built from the character's special-action holder (character actions, the server's SpecialAttackWeapon list, weapons' attacks when worn, perk actions 0xb4).
- Client: A catalogue (actions_data.rs) read from a running client is filtered by what the server listed; before the list arrives everything shows; a weapon's attacks are recomputed and de-duplicated.
- A player notices: Which tiles show can differ (before the lists arrive, two weapons with the same attack).
- Where: `crates/mao-client/src/actions.rs:1`; `crates/mao-client/src/actions.rs:184`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the window lists what the original's list holds, in its order: the nine character actions, then the special attacks (the server's, then the weapons' and Reload), then the server's perk actions; nothing else before the server's lists, one tile for an action listed twice (`actions::listed_templates`). The tiles' kept places (SpecialActions.xml) are not ported.

#### `WIN-023` All non-own containers share one window named "loot"

- SIMPLIFIED, high
- Original: Each container has its own window (TempContainer_%u, many open at once, keyed by identity).
- Client: Corpses, chests, bank and overflow use one shared window name "loot" rebuilt for the container shown.
- A player notices: Only one corpse/chest/bank window at a time, sharing a remembered position.
- Where: `crates/mao-client/src/loot.rs:105`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): one window per container identity (SlotContainerOpened 0x100c7bef), named as the original names them: `bank_window`, `TempContainer_<n>` by the lowest free number (FUN_100cb7f9), own backpacks `Container_<type>x<instance>`; the client keeps every open container (`World::loots`). Not ported: the X's `N3Msg_UseItem(container, false)` (bytes not traced), the list mode per number, a chest closing when walked away from. The 'Take all' button is the client's own.

#### `WIN-024` Stat bonuses are summed by the client from items, perks, nanos and organization

- SIMPLIFIED, high
- Original: the stat container (FUN_10059399 -> FUN_10064d46) holds the modifier map
- Client: derived.rs rebuilds it: only worn places 1-15, 17-31, 33-45 count; item effects are interpolated between low and high templates by quality
- A player notices: shown values can differ from the original's where a source is missing or a place rule is wrong
- Where: `crates/mao-client/src/derived.rs:3`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the summing is the original's own (the server sends stored stats and lists only), so it stays; ported from the stat container: the movement's mode 3 without the trickle (FUN_1006f2fc, FUN_10064d46: `derived::mode3_one`), every place under 0x30 counting (FUN_10080fc3), max health never under 1 (FUN_10064a0e). Not ported: 0xcff5 percentage spells, nano/perk/benefit criteria, stats 0x20d-0x274, mode 4.

#### `WIN-025` Chat windows of a frame are tabs of one window, others get windows of their own

- SIMPLIFIED, medium
- Original: each chat window (ChatGroupWindow_c) is its own window, tabs share a frame
- Client: the first frame is "the client's chat window", each other frame a window of its own
- A player notices: window behaviour may differ from the original's
- Where: `crates/mao-client/src/chatgroups.rs:18`
- Decision: [ ] port  [ ] leave

#### `WIN-026` Control center menus are the client's own hand-written lists

- SIMPLIFIED, medium
- Original: Menus come from CommandMenu.xml, CharMenu.xml, MapMenu.xml, CameraMenu.xml and Windows.xml (pets, tradeskill kit, faction, research, planet map, Team menu, 3rd person modes); left and right strips from LeftMenu.xml and RightMenu.xml.
- Client: Hand-written menus "as far as this client has the windows" with English labels, a "< Back" entry, an "Ask the guides" entry and a camera submenu; entries for things the client lacks are left out; Command opens the Actions window for both Emotes and Control panel.
- A player notices: Menu entries differ from the original's.
- Where: `crates/mao-client/src/vitals.rs:94`; `docs/CLIENT_UI.md:2016`
- Decision: [ ] port  [ ] leave

#### `WIN-027` Fonts: Verdana drawn regular whatever the weight, sizes by egui points

- SIMPLIFIED, medium
- Original: FontSystem_t makes Verdana bold 20/13/24 and italic 16 (CreateTTFont) and scales by the screen's height (GetScaleFromLowestResolution)
- Client: egui's Verdana at the size whose ascent plus descent is the cell height, regular whatever the weight; no screen-height scaling; CHAT is the chat bitmap font
- A player notices: bold and italic UI text appears regular; text size does not follow screen height
- Where: `crates/mao-client/src/gui/font.rs:7`
- Decision: [ ] port  [ ] leave

#### `WIN-028` Inventory and wear windows share one key and one window

- SIMPLIFIED, medium
- Original: inventory (I) and wear (Ctrl+1) are separate windows
- Client: Ctrl+1 opens the worn gear, "which is the same window here"
- A player notices: pressing Ctrl+1 and I toggle the same window
- Where: `crates/mao-client/src/settings.rs:217`
- Decision: [ ] port  [ ] leave

#### `WIN-029` Mission selection window drawn by hand with its own words

- SIMPLIFIED, medium
- Original: MissionSelectionView_c is built by GUI.dll FUN_100d10fc (seven sliders, offer boxes, Accept/Request buttons).
- Client: An egui window with six fixed boxes, plain sliders, an offer list, English button texts ("Request Missions", "Accept Mission", "Cancel") and status lines ("Asking the booth ...").
- A player notices: The window looks and reads differently from the original's.
- Where: `crates/mao-client/src/mission.rs:409`; `crates/mao-client/src/mission.rs:458`
- Decision: [ ] port  [ ] leave

#### `WIN-030` NPC give-trade bar drawn by hand with its own texts

- SIMPLIFIED, medium
- Original: NPCChatTradeBar_c (FUN_10057d8b) docked as a Trade tab (FUN_10059236).
- Client: A bar above the conversation with "GIVE ITEM(S)", slot squares, a credits field and V/X buttons in egui; a click on a slot takes the item out.
- A player notices: The trade bar looks and behaves differently and is not a docked tab.
- Where: `crates/mao-client/src/npcchat.rs:316`
- Decision: [ ] port  [ ] leave

#### `WIN-031` Only some menu entries open windows

- SIMPLIFIED, medium (notes only)
- Original: skin menus drive GUI.dll windows
- Client: a menu entry's value is a window's open flag; only Inventory, Skills, Stats, Nano, NCU, Missions, Friends, Map, Actions, Mail and Options open client windows
- A player notices: other menu entries do nothing
- Where: `docs/CLIENT_UI.md:182`
- Decision: [ ] port  [ ] leave

#### `WIN-032` Option panel pages come from an AOU skin's Root.xml, not the original's built-in pages

- SIMPLIFIED, medium
- Original: GUI.dll fills the option panel from code (the game's own Root.xml is empty)
- Client: loads the pages from any AOU skin at hand (themes, the user's Gui folders), with the current skin's art
- A player notices: option pages depend on AOU skins being installed; without one "No option pages were found"
- Where: `crates/mao-client/src/gui/options.rs:267`
- Decision: [ ] port  [ ] leave

#### `WIN-033` Option window "Key bindings" tab is not the original's editor

- SIMPLIFIED, medium
- Original: "Key bindings" is the editor of every key (FUN_100bcfb3) and an "enter a key" box (FUN_100bf286).
- Client: The tab says this client's keys are set in its own settings window and leads there; "Fixed keys" shows this client's keys and is not editable; commands without a client key show none.
- A player notices: Different key-binding UI.
- Where: `crates/mao-client/src/gui/options.rs:14`; `docs/CLIENT_UI.md:4401`
- Decision: [ ] port  [ ] leave

#### `WIN-034` Rollup area: no drag to reorder, client's own widths, heights and docking controls

- SIMPLIFIED, medium
- Original: RollupController_c; the player drags a page out to make it a window again or to reorder it; docked containers are 175 wide; cc_rollup_controller_fade_level; the control center has docking of entries
- Client: dragging not done; pages are 196 wide with buttons to close, undock and roll; an undock and a dock button replace the drag; page heights start from fixed guesses (Wear 330, NCU 110 ...); the list and rolled state are kept in rollup.ini; the fade is not done
- A player notices: the rollup column looks and is operated differently; pages cannot be dragged
- Where: `crates/mao-client/src/rollup.rs:10`; `docs/CLIENT_UI.md:2046`; `docs/GUI_DLL_PARITY.md:72`
- Decision: [ ] port  [ ] leave

#### `WIN-035` Shop lists are hand-drawn rows with fixed sizes, not item-container lists

- SIMPLIFIED, medium
- Original: the three docks hold ItemContainerView_c lists (columns Icon, Name, Count, Price, Quality), sized by the list (FUN_100ce55d), with the generic item handlers (menu, hold, sorting)
- Client: rows are painted by hand with fixed dock heights (176/84/84) and column positions as fractions of the width; no list mode, sorting or item menu
- A player notices: the shop's lists look and behave differently (no sorting, no held-button menu)
- Where: `crates/mao-client/src/shop.rs:271`
- Decision: [ ] port  [ ] leave

#### `WIN-036` Stats not in the server's list shown as a dash

- SIMPLIFIED, medium
- Original: the original has every stat in its stat container
- Client: stats the server has not sent and the client cannot work out show "-"; armour classes, attack rating and bonuses are built by the client from worn items, perks, nanos and organization benefits
- A player notices: some values are absent or computed by approximation
- Where: `crates/mao-client/src/stats.rs:16`
- Decision: [ ] port  [ ] leave

#### `WIN-037` Tool tip frame drawn as lines, not the original's surfaces

- SIMPLIFIED, medium
- Original: ToolTip_c (0x10149c20) with InfoContainer_c's surfaces, MoveToMouse; a click-to-keep tip
- Client: frame pieces are lines round a translucent dark ground, the title ruled off from the body; placed 12 right and 16 below the pointer; body width limited to 400; original's exact surface sizes not followed; no click-to-keep tip; egui hover text for non-XML windows
- A player notices: tips look different from the original's; outside XML views they cannot be kept
- Where: `crates/mao-client/src/gui/tooltip.rs:8`; `docs/GUI_DLL_PARITY.md:49`; `docs/CLIENT_UI.md:2989`
- Decision: [ ] port  [ ] leave

#### `WIN-038` Alien experience bar counts from zero

- SIMPLIFIED, low
- Original: bar runs between the previous alien level's threshold and the next
- Client: counts from zero because the previous threshold is not known; current nano, alien XP and NextXP show a dash when the server does not send them
- A player notices: alien XP bar fill wrong above alien level 1
- Where: `docs/CLIENT_UI.md:1667`; `crates/mao-client/src/stats.rs:202`
- Decision: [ ] port  [ ] leave

#### `WIN-039` AOU skins show the game's straight bars, not their curved bars

- SIMPLIFIED, low (notes only)
- Original: AOU skin wings carry curved ACTIONVIEW bars (43 x 161) at places the data does not state; CC*BarConfig rects hint at a bar dock beside the wing.
- Client: An AOU skin shows the game's straight bars at the game's offsets. (The bitmap fonts and mouse pointers listed as not done in the notes now exist: font.rs, pointer.rs.)
- A player notices: Bars look different under AOU skins.
- Where: `docs/ORIGINAL_UI_PLAN.md:203`; `docs/CLIENT_UI.md:2024`
- Decision: [ ] port  [ ] leave

#### `WIN-040` Bitmap fonts drawn as a mesh with a 4-pixel line gap, behind a setting

- SIMPLIFIED, low
- Original: DisplaySystem draws FontSystem_t's bitmap fonts
- Client: each line is a mesh of glyph quads; line height is glyph height plus 4; the `original_font` setting (a checkbox, "The original's chat font") turns the bitmap fonts off for egui's font; a glyph not in the font takes a space's width
- A player notices: line spacing and fallback fonts are the client's
- Where: `crates/mao-client/src/font.rs:65`; `docs/CLIENT_UI.md:2056`; `crates/mao-client/src/settings.rs:565`
- Decision: [ ] port  [ ] leave

#### `WIN-041` BorderView three-slice and single-picture modes: ends scaled to the rect's height

- SIMPLIFIED, low
- Original: BorderView_c has nine-slice, three-slice and single-picture modes
- Client: the three-slice and single-picture modes exist only as a check box's button and for the control-center strip buttons (Skin::three_slice scales the end pieces by rect height over the middle's height)
- A player notices: none visible unless a skin uses them elsewhere; strip buttons stretch with their height
- Where: `crates/mao-client/src/skin.rs:130`; `docs/GUI_DLL_PARITY.md:35`
- Decision: [ ] port  [ ] leave

#### `WIN-042` Browser window frame is the client's own (Home, Back, Forward, status line)

- SIMPLIFIED, low
- Original: HomeButton, BackButton, ForwardButton, URL bar and StatusBar of BrowserWindow_c.
- Client: Three buttons (Home, <, >) and a small status line in the client's chrome; resizable, default 820x620.
- A player notices: Browser frame looks different.
- Where: `crates/mao-client/src/browser.rs:161`
- Decision: [ ] port  [ ] leave

#### `WIN-043` Buttons in right-click menus and the launcher left as egui's

- SIMPLIFIED, low (notes only)
- Original: Button_c everywhere except right-click menus
- Client: the launcher's buttons and right-click menu buttons left as egui's (the skinned Button_c falls back to egui::Button without a skin)
- A player notices: some buttons look different
- Where: `docs/CLIENT_UI.md:2034`; `crates/mao-client/src/chrome.rs:554`
- Decision: [ ] port  [ ] leave

#### `WIN-044` Character viewer is only the inventory's figure

- SIMPLIFIED, low (notes only)
- Original: CharacterViewerModule_c / CharacterViewer_c
- Client: the inventory's figure
- A player notices: no separate viewer
- Where: `docs/GUI_DLL_PARITY.md:115`
- Decision: [ ] port  [ ] leave

#### `WIN-045` Chat input line and launcher text fields stay egui's

- SIMPLIFIED, low (notes only)
- Original: TextInputView_c; the chat input is the control center's CONTROLCENTER_INPUT_BAR
- Client: the chat input stays egui's because the bar "looked foreign in the window"; launcher fields likewise; the text field colour is "taken to be Default, not checked"
- A player notices: input field look differs
- Where: `docs/CLIENT_UI.md:2040`; `docs/CLIENT_UI.md:2050`
- Decision: [ ] port  [ ] leave

#### `WIN-046` Chat sub-menus replace their parent

- SIMPLIFIED, low (notes only)
- Original: sub-menus open beside their parent
- Client: one menu replaces the other
- A player notices: menu navigation looks different
- Where: `docs/CLIENT_UI.md:308`
- Decision: [ ] port  [ ] leave

#### `WIN-047` Chat Visual menu: mode rule, sliders, fading simplified

- SIMPLIFIED, low (notes only)
- Original: FUN_1009a00d mode, two transparency sliders, FadingTextView_c fading
- Client: the mode's rule, sliders as lists, the fading's behaviour (not read from FadingTextView_c)
- A player notices: chat window transparency/fading behave differently
- Where: `docs/ORIGINAL_AUDIT.md:110`
- Decision: [ ] port  [ ] leave

#### `WIN-048` City map: the visible part is not capped at 128 tiles

- SIMPLIFIED, low
- Original: CalculatePreferredSize (FUN_100e36a7) caps each component at 1023 pixels so the visible part never spans more than 128 tiles
- Client: the dock takes whatever room there is (up to 16000 pixels) and each tile is painted separately
- A player notices: a very large window shows more of the city map at once than the original could
- Where: `crates/mao-client/src/cityterminal.rs:290`
- Decision: [ ] port  [ ] leave

#### `WIN-049` Control center strip texts and tips are fixed English

- SIMPLIFIED, low
- Original: Strip labels and tips come from the game's texts.
- Client: "NCU", "CRED", "DEF", "AGG" and tooltips are fixed strings.
- A player notices: The control center is not translated.
- Where: `crates/mao-client/src/vitals.rs:428`
- Decision: [ ] port  [ ] leave

#### `WIN-050` Controller window's next bill date always in English

- SIMPLIFIED, low
- Original: UpkeepDateView shows strftime("%#x", localtime(...)), the long date of the user's locale.
- Client: Always writes the English long date, "Sunday, November 15, 2026".
- A player notices: Players with another system language see an English date.
- Where: `crates/mao-client/src/cityterminal.rs:117`
- Decision: [ ] port  [ ] leave

#### `WIN-051` Deposit to Market closes silently when nothing is deposited

- SIMPLIFIED, low
- Original: with items or credits it sends N3Msg_SendMarketItem (FUN_10009c40); behaviour with nothing is not stated
- Client: Send with no items and no credits just closes the window
- A player notices: no message when pressing Send on an empty deposit
- Where: `crates/mao-client/src/marketsend.rs:141`
- Decision: [ ] port  [ ] leave

#### `WIN-052` Escape closing: the chest option is not told apart from an own backpack

- SIMPLIFIED, low
- Original: InventoryView_c FUN_100ccefe: a container's window follows esc_backpacks for a backpack and the option of chests (esc_chests) for one in the world; esc_* options also exist for raid, LFT, vehicle, tower menu, city terminal
- Client: an own backpack uses esc_backpacks, other containers none; esc_chests is absent; the escape_option list has no raid, LFT, vehicle, tower menu or city terminal windows
- A player notices: Escape does not close a world chest (and those other windows) by their options
- Where: `crates/mao-client/src/ui.rs:408`; `docs/GUI_DLL_PARITY.md:60`
- Decision: [ ] port  [ ] leave

#### `WIN-053` Faction bands and titles built into the code

- SIMPLIFIED, low
- Original: N3Msg_GetFactionRange reads the database's 0xf4265 records.
- Client: The twelve bands are written into the code.
- A player notices: None visible unless the database differs.
- Where: `crates/mao-client/src/factions.rs:20`
- Decision: [ ] port  [ ] leave

#### `WIN-054` Inbox Delete unchanged from before

- SIMPLIFIED, low (notes only)
- Original: mail fill FUN_10107b87 enables Delete only when nothing is left to take
- Client: the inbox's own Delete is as it was (a mail with an untaken item is refused with a game line)
- A player notices: inbox Delete enabling differs
- Where: `docs/CLIENT_UI.md:3682`; `crates/mao-client/src/mailbox.rs:123`
- Decision: [ ] port  [ ] leave

#### `WIN-055` Interface font falls back to egui's own font without Verdana

- SIMPLIFIED, low
- Original: CreateTTFont makes Verdana via the Windows font system
- Client: reads verdana.ttf from the system's fonts folder (Linux and macOS search their font folders); egui's proportional font until it is in or when the system has none
- A player notices: without Verdana the interface text uses another font
- Where: `crates/mao-client/src/font.rs:222`; `docs/PLATFORMS.md:18`
- Decision: [ ] port  [ ] leave

#### `WIN-056` Knowledge window keeps items only, no character references

- SIMPLIFIED, low
- Original: a click shows the item's page (itemid://low/high), and a character on the board shows charref://
- Client: only items are dropped and shown (itemref://low/high/ql); charref:// references are not dropped; the drag source is the inventory window; an item read back shows its name until inventory lends its picture
- A player notices: characters cannot be kept on the board
- Where: `crates/mao-client/src/knowledge.rs:7`; `docs/GUI_DLL_PARITY.md:98`
- Decision: [ ] port  [ ] leave

#### `WIN-057` Knowledge window: the board's size is fixed

- SIMPLIFIED, low
- Original: the grid (MultiListView_c in grid mode) grows
- Client: a fixed 6 by 5 board in a window that cannot be resized; a full board refuses a drop
- A player notices: at most 30 items fit
- Where: `crates/mao-client/src/knowledge.rs:22`
- Decision: [ ] port  [ ] leave

#### `WIN-058` Logout timer bar drawn plainly; sit loop not played

- SIMPLIFIED, low
- Original: TimerBar_c with bar pictures 0x1a8/0x1a9; sitting characters play ground-start then idle-ground loop
- Client: bar drawn plainly (pictures not read); sitting shown by held ground-start only, idle-ground not played; bar runs 40 s while text says 30
- A player notices: logout bar and sitting pose look different
- Where: `docs/CLIENT_UI.md:2731`; `crates/mao-client/src/ui.rs:456`
- Decision: [ ] port  [ ] leave

#### `WIN-059` Mouse pointers are mapped from the toolkit's cursor kinds

- SIMPLIFIED, low
- Original: the original's interface chooses among its own GFX_GUI_POINTER_* sprites
- Client: egui says which pointer it wants and each kind is mapped to the nearest sprite (several kinds share one); a kind with no sprite keeps the system pointer; without the skin the system pointers stay
- A player notices: some pointer shapes differ from the original or are the system's
- Where: `crates/mao-client/src/pointer.rs:21`
- Decision: [ ] port  [ ] leave

#### `WIN-060` Name tag text falls back to the toolkit's font and a width guess

- SIMPLIFIED, low
- Original: the tag's text is in the game shell's bitmap font
- Client: without the shell font a 7 pixels a letter width and a proportional font are used
- A player notices: only when the original font is missing
- Where: `crates/mao-client/src/nametag.rs:141`
- Decision: [ ] port  [ ] leave

#### `WIN-061` Organization boxes at the pointer; invitation timer not kept

- SIMPLIFIED, low
- Original: leave and promotion boxes come up in the screen's middle (Window::MoveToCenter); before an invitation's yes the original checks a timer (number 5, FUN_1006414f) and says the time left instead of asking the server
- Client: all six boxes come up at the pointer; no timer, the server is asked every time; the four server message layouts are from the reader's code alone
- A player notices: box position differs; invitation timer absent
- Where: `docs/CLIENT_UI.md:4238`; `crates/mao-client/src/orgboxes.rs:17`; `crates/mao-client/src/orgboxes.rs:20`
- Decision: [ ] port  [ ] leave

#### `WIN-062` Page language is always English

- SIMPLIFIED, low
- Original: AnarchyOnline.AcceptLanguage is the locale.
- Client: The script object is made with "en" every time.
- A player notices: Pages that follow AcceptLanguage always get English.
- Where: `crates/mao-client/src/browser.rs:458`
- Decision: [ ] port  [ ] leave

#### `WIN-063` Player trade: partner status is painted text

- SIMPLIFIED, low
- Original: GUI.dll fills PartnerStatusDock with a view of its own
- Client: paints "Partner accepted" / "Partner trading" as plain text in the dock
- A player notices: the partner-status area looks different
- Where: `crates/mao-client/src/shop.rs:366`
- Decision: [ ] port  [ ] leave

#### `WIN-064` Popup menus longer than the screen are clipped, with no scroll bar

- SIMPLIFIED, low
- Original: PopupMenu_c holds its menu in a scroll view with a vertical bar on auto; the menu is at most the screen's height less 100
- Client: the menu is cut to that height and clipped; no scroll view or bar
- A player notices: a very long menu cannot be scrolled
- Where: `crates/mao-client/src/gui/popup.rs:72`
- Decision: [ ] port  [ ] leave

#### `WIN-065` RadioButton drawn as a check box with radio pictures

- SIMPLIFIED, low
- Original: RadioButton_c is its own class
- Client: a check box with the radio pictures
- A player notices: none visible
- Where: `crates/mao-client/src/gui/controls.rs:347`; `docs/GUI_DLL_PARITY.md:37`
- Decision: [ ] port  [ ] leave

#### `WIN-066` Rename box is the client's own small window

- SIMPLIFIED, low
- Original: The original's text input dialog.
- Client: A small window with a field, OK and Cancel.
- A player notices: Look of the rename box differs.
- Where: `crates/mao-client/src/chatconfig.rs:308`
- Decision: [ ] port  [ ] leave

#### `WIN-067` Research lists rebuilt whenever the data changed

- SIMPLIFIED, low
- Original: lists are rebuilt when stats 53 and 60 change and when a personal rank finishes (LAB_100f712a, LAB_100f7151, FUN_100f70b0)
- Client: rebuilt whenever what they show changed
- A player notices: none visible
- Where: `crates/mao-client/src/research.rs:46`
- Decision: [ ] port  [ ] leave

#### `WIN-068` Reset all skills count counts unknown stats as nothing

- SIMPLIFIED, low
- Original: reads stats 348 and 691 whatever they hold (FUN_100fae7a)
- Client: a stat the server has not told of counts nothing
- A player notices: the "Reset all skills (n)" count may differ before the server sends the stats
- Where: `crates/mao-client/src/skillwindow.rs:362`
- Decision: [ ] port  [ ] leave

#### `WIN-069` Selected list row filled at 45 percent of the original's colour

- SIMPLIFIED, low
- Original: a selected row is filled with 0x88aadd behind its cells (FUN_100f1f68)
- Client: the fill is blended at 0.45 opacity
- A player notices: selected rows are paler
- Where: `crates/mao-client/src/gui/multilist.rs:402`
- Decision: [ ] port  [ ] leave

#### `WIN-070` Stat names in tips use the client's own English table

- SIMPLIFIED, low
- Original: Names come from text.mdb categories 2003 and 2002.
- Client: stat_name has a hand-written English table and "Stat N" for stats not in it.
- A player notices: Names in some tips differ; unnamed stats show as "Stat N".
- Where: `crates/mao-client/src/skills.rs:126`; `crates/mao-client/src/skills.rs:159`
- Decision: [ ] port  [ ] leave

#### `WIN-071` Tabs wider than the strip are narrowed in proportion, not by SpaceOut

- SIMPLIFIED, low
- Original: TopBorderView_c (FUN_10147438) narrows tabs by View::SpaceOut's share
- Client: all tabs are scaled in proportion (not below 0.2), "not followed exactly" and not checked against the original; text cut at the right margin; inventory tab names cut
- A player notices: crowded tab strips may be split differently
- Where: `crates/mao-client/src/chrome.rs:621`; `docs/CLIENT_UI.md:2032`
- Decision: [ ] port  [ ] leave

#### `WIN-072` Target frame without the skin falls back to a plain box

- SIMPLIFIED, low
- Original: the target frames are the control center's bars (CreateTargetMenus) with the skin's pictures
- Client: without the skin's target pictures a plain "Target" box with the name replaces them
- A player notices: only when the skin lacks the pictures
- Where: `crates/mao-client/src/ui.rs:2242`
- Decision: [ ] port  [ ] leave

#### `WIN-073` Team window header leaves the team side empty

- SIMPLIFIED, low
- Original: header text "Team level: %u Team side: %s"
- Client: the %s (team side) is replaced with nothing
- A player notices: the side name is never shown in the team window header
- Where: `crates/mao-client/src/team.rs:226`
- Decision: [ ] port  [ ] leave

#### `WIN-074` Teleport target window: the single tab is not drawn

- SIMPLIFIED, low
- Original: the window has one tab, "Target List"
- Client: the tab is not drawn, as the client's other windows' single tabs are not
- A player notices: no tab header on the window
- Where: `crates/mao-client/src/teleport.rs:7`; `docs/CLIENT_UI.md:3843`
- Decision: [ ] port  [ ] leave

#### `WIN-075` Text editing keeps plain text; Home and End use new-line bounds; selection colour is the client's

- SIMPLIFIED, low
- Original: TextRenderer_c keeps the text as markup, escaping <, >, & as typed; Home and End go to the start and end of the wrapped line
- Client: an editable view's text is plain characters; Home and End go to the start and end of the line between new lines; the selection colour is the client's
- A player notices: Home and End in a wrapped multi-line field jump to the paragraph's ends, not the visible line's
- Where: `crates/mao-client/src/gui/edit.rs:8`; `docs/CLIENT_UI.md:2951`
- Decision: [ ] port  [ ] leave

#### `WIN-076` Tips of the day: only a few events, no reset

- SIMPLIFIED, low
- Original: TipSystem_t has all events (other code reports them to TriggerEvent), /tipoftheday and reset
- Client: derives only entering a playfield, opening inventory, skills and shop windows and minutes played (others from `trigger` calls elsewhere); kept in tips.ini; no reset
- A player notices: some tips of the original may never appear
- Where: `docs/GUI_DLL_PARITY.md:61`; `crates/mao-client/src/tips.rs:259`
- Decision: [ ] port  [ ] leave

#### `WIN-077` Vertical layouts place a top or bottom aligned child at the left

- SIMPLIFIED, low
- Original: VLayoutNode knows left, right and centre; for top or bottom the original reads a stale value and prints an error
- Client: places such a child at the left
- A player notices: none visible with the shipped skins
- Where: `crates/mao-client/src/gui/layout.rs:190`
- Decision: [ ] port  [ ] leave

#### `WIN-078` Wheel: egui's own step, vertical bars only

- SIMPLIFIED, low
- Original: ScrollBar_c WheelMoved (0x10142820): 10 a notch
- Client: converts egui's 50-point notch to the original's step; the wheel only moves the vertical bar
- A player notices: horizontal bars cannot be moved with the wheel
- Where: `docs/CLIENT_UI.md:2030`; `crates/mao-client/src/gui/scroll.rs:277`
- Decision: [ ] port  [ ] leave

#### `WIN-079` Wide words go on a line of their own instead of splitting at a character

- SIMPLIFIED, low
- Original: a word wider than the line is split where it overflows, at a character (GetBufferLocation)
- Client: the word goes on a line of its own with the overflow left to the edge
- A player notices: very long words (URLs, names) run past the view's edge
- Where: `crates/mao-client/src/gui/text.rs:511`
- Decision: [ ] port  [ ] leave

#### `WIN-080` Window transparency in steps of ten

- SIMPLIFIED, low
- Original: two sliders for how solid a borderless chat window is
- Client: a menu list of steps of 0, 10 ... 100 percent for inactive and for active
- A player notices: no slider; ten steps
- Where: `crates/mao-client/src/chatwindow.rs:198`
- Decision: [ ] port  [ ] leave

#### `WIN-081` X-Platform header is always Native

- SIMPLIFIED, low
- Original: `Native` (`Steam` when RunningSteam)
- Client: always "Native"
- A player notices: Steam players are told Native
- Where: `crates/mao-client/src/browser.rs:89`
- Decision: [ ] port  [ ] leave

#### `WIN-082` XML window keyboard handling covers only a few keys

- SIMPLIFIED, low
- Original: TextRenderer_c::KeyDown handles its full key set.
- Client: Only Enter, Backspace, Delete, Home, End, arrows, Insert, A, V, Tab and Escape reach a focused view.
- A player notices: Other editing keys do nothing in XML text fields.
- Where: `crates/mao-client/src/gui/host.rs:301`
- Decision: [ ] port  [ ] leave

### Windows and interface: the original was not read: inferred or measured (22)

#### `WIN-083` Skill values for other characters are computed and unchecked

- GUESS, medium
- Original: the original's shown value (checked on one character only)
- Client: all 69 trickle skills get a computed value even when the server sent none; formula verified on Conjecture only; an untrained skill starts at 5
- A player notices: skill numbers for other characters may be off
- Where: `docs/CLIENT_UI.md:1673`; `crates/mao-client/src/derived.rs:186`
- Decision: [ ] port  [ ] leave

#### `WIN-084` Window frame and panel translucency, title size are the client's own

- GUESS, medium
- Original: not stated
- Client: the frame's panel colour is drawn at alpha 225 (fixed), the title at 15 points at the top left, borders tinted with the theme's accent
- A player notices: window opacity and title size may differ from the original's
- Where: `crates/mao-client/src/chrome.rs:305`
- Decision: [ ] port  [ ] leave

#### `WIN-085` Bitmap font line metrics not understood

- GUESS, low (notes only)
- Original: .fnt files with three trailing words
- Client: the three trailing words (line metrics) are not understood; the note also says tooltips are not in the bitmap font (the code now calls a tooltip bitmap font in ui.rs:466, so that part may be out of date)
- A player notices: text spacing in the bitmap font may differ
- Where: `docs/CLIENT_UI.md:2052`
- Decision: [ ] port  [ ] leave

#### `WIN-086` Chat window sizes, places and tell-window stacking

- GUESS, low
- Original: not stated
- Client: first window 560x230 at a fixed spot, others 420x170, tell windows 380x190 offset 26 px each over the chat window
- A player notices: window first positions differ until the layout is saved
- Where: `crates/mao-client/src/chatwindow.rs:1610`
- Decision: [ ] port  [ ] leave

#### `WIN-087` City cloak button switches the cloak at once

- GUESS, low (notes only)
- Original: not stated
- Client: pressing it switches the cloak at once
- A player notices: the cloak state flips before any server answer
- Where: `docs/LIVE_TEST_LIST.md:246`; `crates/mao-client/src/net.rs:8376`
- Decision: [ ] port  [ ] leave

#### `WIN-088` City map: building item's identity in the build request is a guess

- GUESS, low (notes only)
- Original: not stated
- Client: The building item's identity in the request is its inventory place.
- A player notices: A build request may be wrong.
- Where: `docs/CLIENT_UI.md:3920`
- Decision: [ ] port  [ ] leave

#### `WIN-089` Container window sizes, columns and start positions are the client's own

- GUESS, low
- Original: container windows are made with an empty Rect, no default size or position found; no fixed column count; the bank's columns not stated
- Client: bank eight columns, others three; six rows at most; a position chosen from the bag's instance number
- A player notices: windows open in different places and shapes
- Where: `crates/mao-client/src/loot.rs:178`
- Decision: [ ] port  [ ] leave

#### `WIN-090` Control center experience bars ignore shadowknowledge and show nothing past the next level

- GUESS, low
- Original: the Stats window experience bar switches to shadowknowledge from level 200 (FUN_1008038f); what the control center's bar does is not stated
- Client: the wing bars use experience only; a value past the next level's figure shows no bar
- A player notices: at level 200 and above, or when over the figure, the XP wing bar is empty or wrong
- Where: `crates/mao-client/src/vitals.rs:14`
- Decision: [ ] port  [ ] leave

#### `WIN-091` Default chat window look values taken from the player's files

- GUESS, low
- Original: not stated (the original's configs carry the values)
- Client: solid 80 percent active and 30 percent inactive, taken from files seen
- A player notices: none visible if right
- Where: `crates/mao-client/src/chatgroups.rs:305`
- Decision: [ ] port  [ ] leave

#### `WIN-092` List double-click is a fixed 400 ms and sorting is by plain text keys

- GUESS, low
- Original: MultiListView_c's click count and sort comparison are the original's
- Client: a second click on the same row within 400 ms is a double click; rows sort by comparing key strings
- A player notices: double-click timing and sort order may differ
- Where: `crates/mao-client/src/gui/multilist.rs:455`
- Decision: [ ] port  [ ] leave

#### `WIN-093` Market text field widths forced to the old browser's

- GUESS, low
- Original: Awesomium (a 2012 WebKit) lays out text fields narrower than today's engines (size times average character width plus widest minus average).
- Client: Injects a script that gives sized text fields the old width; the two figures (5 and 27 px) are worked out from Arial's tables, not measured in the original.
- A player notices: Market fields look like the original's, approximately.
- Where: `crates/mao-client/src/browser.rs:212`; `docs/CLIENT_UI.md:3458`
- Decision: [ ] port  [ ] leave

#### `WIN-094` Mission booth chosen by the client within 40 m

- GUESS, low
- Original: not stated
- Client: Picks the booth last used or the nearest 0xdac1 table dynel within 40 m, else "No mission booth in reach (walk up to one)."
- A player notices: Missions can be requested from up to 40 m away.
- Where: `crates/mao-client/src/net.rs:8068`; `crates/mao-client/src/net.rs:8105`
- Decision: [ ] port  [ ] leave

#### `WIN-095` Mission window opened by the client on a double click on a booth

- GUESS, low
- Original: not stated (the server only echoes the use)
- Client: A double click on a mission booth opens the mission window itself and remembers the booth.
- A player notices: The window opens on use though the server sends nothing for it.
- Where: `crates/mao-client/src/net.rs:8300`
- Decision: [ ] port  [ ] leave

#### `WIN-096` New-mission condition: only the message's last byte

- GUESS, low (notes only)
- Original: SlotGotNewMission; another condition for the new-mission line (a flag of the character, +0x140)
- Client: new mission by the message's last byte; the +0x140 flag condition not looked at
- A player notices: waypoint/new-mission line may differ
- Where: `docs/CLIENT_UI.md:4086`
- Decision: [ ] port  [ ] leave

#### `WIN-097` Panel colour for a light theme is the Default colour scaled to 14 percent

- GUESS, low
- Original: not stated
- Client: a theme whose Default is light gets a panel fill of that colour times 0.14 and borders in Default; a dark one fills panels with Default and borders in Selected
- A player notices: window backgrounds are the client's derived colour
- Where: `crates/mao-client/src/theme.rs:95`
- Decision: [ ] port  [ ] leave

#### `WIN-098` Perk action icons are chosen from the perk's name

- GUESS, low
- Original: the perk's icon is its item template's icon
- Client: when the template icon is absent the icon family (perk_*) is picked by words in the perk's name ("heal", "bash", "blood" ...); Reload uses the Aimed Shot picture
- A player notices: some perk or Reload tiles show an unrelated picture
- Where: `crates/mao-client/src/actions.rs:56`; `docs/CLIENT_UI.md:1655`
- Decision: [ ] port  [ ] leave

#### `WIN-099` Planet map window: a new map opens at its second level

- GUESS, low
- Original: not stated
- Client: a newly chosen map opens at its second zoom level (or its first if it has one) and follows the character
- A player notices: the starting zoom is the client's choice
- Where: `crates/mao-client/src/planetwindow.rs:81`
- Decision: [ ] port  [ ] leave

#### `WIN-100` Planet map window: size and place taken from screenshots

- GUESS, low
- Original: PlanetMapView_c (FUN_1004d92d): its own default frame; starts small at the top left
- Client: opens about 400 pixels square (a twelfth of the screen width, 200 to 400) at 1.8 % across and 5.3 % down, fitted to the user's screenshots on a 5120 by 1440 display
- A player notices: the window may open at a slightly different size and place
- Where: `crates/mao-client/src/planetwindow.rs:90`; `docs/CLIENT_UI.md:3144`
- Decision: [ ] port  [ ] leave

#### `WIN-101` Planet maps: which map a playfield is shown on

- GUESS, low
- Original: not stated (PlanetMapView_c chooses the Shadowlands set by the expansion flag)
- Client: a playfield is shown on the Rubi-Ka map when it has a place there, else on the Shadowlands map, else on Rubi-Ka's
- A player notices: which of the two maps opens for a playfield is the client's rule
- Where: `crates/mao-client/src/planetmaps.rs:73`; `docs/ORIGINAL_AUDIT.md:63`
- Decision: [ ] port  [ ] leave

#### `WIN-102` Team window's own bars come from the character's own stats

- GUESS, low
- Original: the original's member view was not read for it
- Client: the character's own health and nano in the team window come from its own stats because no TeamMemberInfo came for oneself
- A player notices: own bars in the team window may differ from the original's
- Where: `crates/mao-client/src/net.rs:7306`; `docs/CLIENT_UI.md:567`
- Decision: [ ] port  [ ] leave

#### `WIN-103` Text selection drawn in a fixed blue shade

- GUESS, low
- Original: not stated
- Client: selection band is a fixed colour (0x4070c0 at alpha 0xa0)
- A player notices: selection colour may differ from the original's
- Where: `crates/mao-client/src/gui/controls.rs:762`
- Decision: [ ] port  [ ] leave

#### `WIN-104` Title level is worked out from the level

- GUESS, low
- Original: stat 37 title level is not sent by the server; the original reports 3 for level 86 and 7 for 220
- Client: derives it from level bands ("checked against characters of level 5 to 220"; 1 below 15, 2 below 50, ...)
- A player notices: title-level requirement lines depend on a fitted table, may be wrong at band edges
- Where: `docs/CLIENT_UI.md:1689`; `crates/mao-client/src/derived.rs:187`; `crates/mao-client/src/derived.rs:300`
- Decision: [ ] port  [ ] leave

### Windows and interface: a guard or fallback the original lacks (4)

#### `WIN-105` A bar's current value above its computed maximum counts as the maximum

- SAFETY, low
- Original: not stated (the original computes the maximum through GetSkill detail 2)
- Client: when a current value exceeds the computed maximum (active nano buffs are not all known) the maximum is raised to the current value
- A player notices: bars show full instead of overflowing, the tooltip shows a different maximum, and a wrong maximum may be hidden
- Where: `crates/mao-client/src/stats.rs:127`; `docs/CLIENT_UI.md:1669`; `docs/STAT_ENGINE.md:263`
- Decision: [ ] port  [ ] leave

#### `WIN-106` Control center falls back to a plain strip without the skin's pictures

- SAFETY, low
- Original: not stated: the original always has its pictures
- Client: without the pictures a plain dark strip with plain bars and button menus is drawn
- A player notices: only when the skin is missing
- Where: `crates/mao-client/src/vitals.rs:3`
- Decision: [ ] port  [ ] leave

#### `WIN-107` Skills window without the cost tables charges one IP a point

- SAFETY, low
- Original: Costs and caps come from Gamecode's tables.
- Client: When the game's tables are not at hand, a point costs one IP and caps are not checked.
- A player notices: Only if the game's files are missing.
- Where: `crates/mao-client/src/skillwindow.rs:544`
- Decision: [ ] port  [ ] leave

#### `WIN-108` Windows wait for the screen's real size before showing

- SAFETY, low
- Original: not stated
- Client: A movable window returns without drawing while egui reports a placeholder screen.
- A player notices: Windows appear a frame or two later at start.
- Where: `crates/mao-client/src/chrome.rs:251`
- Decision: [ ] port  [ ] leave

### Windows and interface: the original does it, the client does not (42)

#### `WIN-109` Bag, bank, inventory and wear windows have no List Mode, Auto Arrange, Labels On Top or Unlock All menus

- MISSING, high
- Original: The container window's menu offers ListMode and AutoArrange (sort by column); the inventory window's menu has ListMode and an AutoArrange submenu; the wear window's menu has LabelsOnTop and UnlockAllItems (FUN_100cbace, FUN_10041637, FUN_100e20d6).
- Client: The only container menu entry is Rename, on own backpacks; the inventory and wear windows have no menu and are plain icon grids.
- A player notices: No list view or column sorting of bags, bank, corpses or inventory; no labels-on-top switch, no unlock-all.
- Where: `crates/mao-client/src/loot.rs:19`; `crates/mao-client/src/invwindows.rs:321`; `docs/CLIENT_UI.md:1797`; `docs/ORIGINAL_AUDIT.md:101`; `docs/GUI_DLL_PARITY.md:76`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done with what was left: a worn item's Lock Item / Unlock Item and the wear window's Unlock All Items (the lock on the wear place: its picture, nothing taken up or used, nothing worn on it or taken off it); the bank placed by hand, with Don't Auto Arrange; the inspect window's own Labels On Top; the frame's menu button at the left of every window's title (the window's entries, then Close Window). Not there: Autofade Window (the client's windows do not fade), joining stacks inside the bank by the mouse, the lock entry in the inspect window. Looked at in offscreen pictures; no mouse path was tried.
- Done in part (2026-11-18): Done: the window menus of the inventory, own backpacks, the bank, corpses, chests and the overflow (right button beside the items): List Mode (columns Icon, Name, Count, Quality; sort and widths kept) and Auto Arrange with its sub-menu (Don't Auto Arrange, Unsorted, By Name / Count / Quality, Sort Ascending; another's container only the By entries, and it starts sorted by name as in the original); the wear window's Labels On Top. Left: Unlock All Items (it empties per-item locks the client does not have), the bank placed by hand, the Inspect window's own Labels On Top, the frame's menu button. Looked at in offscreen pictures; the mouse paths in list mode and an arranged grid were not tried.

#### `WIN-110` City controller window: no land-buying dialog, no item lock or pointer tip, no Escape option

- MISSING, high
- Original: GUI type 1 land-buying dialog; lock on an inventory item that waits on the map; pointer's place as a tip over the map; Escape option esc_cityterminal.
- Client: Not done ("Not here": GUI type 1, the lock (such an item is not taken a second time instead), the tip); no esc_cityterminal found; the map itself is built.
- A player notices: A city's land cannot be bought; no Escape option; item not visibly locked.
- Where: `crates/mao-client/src/cityterminal.rs:35`; `docs/CLIENT_UI.md:3890`; `docs/CLIENT_UI.md:3919`; `crates/mao-client/src/cityterminal.rs:33`; `docs/GUI_DLL_PARITY.md:111`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): The land-buying dialog (GUI type 1: the text with the price and upkeep, Purchase in lease mode 3 only and sent at its press with no question as in the original, Cancel, the map shown only), the inventory's lock on an item that waits on the map (picture, no take-up, use, trade or drop on it) and the pointer's place as a tip over the map; the Escape option came with SET-030. A type 1 opening used to leave a session with no window that blocked every later opening. No capture has a type 1 opening or a purchase: from the code, looked at in offscreen pictures. Purchase is never to be pressed in a test.

#### `WIN-111` Popup menus lack icons, sub-menus, sliders, separators, type-to-search

- MISSING, high
- Original: PopupMenu_c has icons, sub-menus, slider items, separators, type-to-search
- Client: string entries with check marks only
- A player notices: right-click menus are flat
- Where: `docs/GUI_DLL_PARITY.md:48`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): gui/popup.rs is PopupMenu_c with separators, the check and sub-menu arrow in the tail column, sub-menus (300/200 ms, placed beside their entry, flipped at the screen's edge), slider entries and type-to-search; the chat window's menu is one menu with its sub-menus and the two transparency sliders, the container item menu's Looter and the nano programs' Auto Arrange are sub-menus. Icons and more than one choice are used by no menu of the original and were left. Looked at in offscreen renders, not live. See CLIENT_UI.md, 'Popup menus like the original'.

#### `WIN-112` Several character action tiles do nothing

- MISSING, high
- Original: Use, Pick-Up, Walk/Run, Suspended Animation and Crawl run UseItem/GetItem, the walk/run switch, StartCamping/StopCamping and CrawlToggle (FUN_100427d6); Stand is SitToggle
- Client: these tiles fall to "its network message is not captured yet"; Stand only sends STAND
- A player notices: clicking those tiles shows a not-captured line and does nothing
- Where: `crates/mao-client/src/net.rs:8214`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): Use, Start/End Combat, Sit/Stand, Walk/Run and Suspended Animation / Disrupt do what their keys do (the functions FUN_100427d6 calls for them); Pick-Up (ClientGetItem, in no capture) and Crawl (no crawling kept) still say they are not known.

#### `WIN-113` Toggling character actions always show the same tile

- MISSING, high
- Original: Sit/Stand, Walk/Run, Sneak/Stop Sneaking, Start/End Combat, Suspended Animation/Disrupt and Crawl/Crawl-stop swap by state (FUN_1004300e)
- Client: the catalogue always shows Stand, Walk, Sneak, Start Combat, Suspended Animation and Crawl
- A player notices: the tile does not change when you sit, run, fight or sneak
- Where: `crates/mao-client/src/actions.rs:45`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): a pair's tile shows what a click does next (FUN_1004300e: `actions::shown_number`) by the states the network thread sets every frame (fighting, sitting, walking, sneaking, camping; crawling is not kept), named and pictured by its template; the shortcut bars and the smart hotbars follow; Sneak and Stop Sneaking go by number. Pictures only, not seen live.

#### `WIN-114` Window chrome: no "?" help button, docking, auto-fade, original titles or art

- MISSING, high (notes only)
- Original: GuiSystem_c / WindowController_c docks windows (DockTabbedWindow, dock_config), fades window groups (AutofadeWindow) and gives windows a "?" title button; original titles (Mission, Wear), tabs/buttons in original art, launcher windows.
- Client: egui windows; chrome::window draws frame, pin and close only; without pictures a plain egui window.
- A player notices: Windows cannot be docked into tab groups, never auto-fade, have no help button; look differs.
- Where: `docs/GUI_DLL_PARITY.md:32`; `docs/CLIENT_UI.md:2028`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18): windows fade when the pointer leaves them (0.3 after 2 s over 1 s, solid over 0.2 s: FadeWindows), the pin is the fade switch alone and kept in pins.ini, the frame menu has Autofade Window, Escape closes pinned windows too, and the nine windows with a help page have the "?" button opening it. Left: docking windows as tabs of one frame (DockTabbedWindow, DockAreas); the chat windows' own fading.

#### `WIN-115` A locked perk action click is ignored without the wait line

- MISSING, medium
- Original: prints the wait line (UnableToPerformActionSkill, category 1000) with the perk's name and the time left; nothing is sent
- Client: the click is ignored silently
- A player notices: no text is shown when clicking a locked perk action
- Where: `crates/mao-client/src/actions.rs:365`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): a locked action's click goes on and says its wait line (the perk's UnableToPerformActionPerk, the skill's UnableToPerformActionSkill; the catalog's Original line named the skill text for a perk); perk locks were being read from the wrong words of 0xcf / 0xce, mended with it.

#### `WIN-116` Character info page leaves out several of the original's parts

- MISSING, medium
- Original: FUN_1003482c's page includes the profession title's name, the defender rank's name, the side experience bonus, the city, the PvP ranks' names, own tokens and veteran points, specializations and the account's items; the item info view also shows the shop price (FUN_10031fda)
- Client: shows only a number for the title level and the defender rank and omits the rest; the item page has no shop price
- A player notices: /inspect info page is shorter than the original's; item pages in shops show no price
- Where: `crates/mao-client/src/charinfo.rs:11`; `crates/mao-client/src/gui/iteminfo.rs:39`; `docs/LIVE_TEST_LIST.md:88`; `docs/CLIENT_UI.md:1611`; `docs/GUI_DLL_PARITY.md:77`
- Decision: [ ] port  [ ] leave

#### `WIN-117` Inline pictures (img) parsed but never sized or drawn

- MISSING, medium
- Original: HTMLParser_c reads img src=rdb://<n>|tdb://<name> and draws the picture
- Client: the markup is parsed into an image item, but words are measured with no image sizes so it is not shown
- A player notices: pictures in info pages, chat and NPC text do not appear
- Where: `crates/mao-client/src/gui/controls.rs:567`; `crates/mao-client/src/gui/text.rs:399`; `docs/GUI_DLL_PARITY.md:46`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done: an img in the text takes its picture's own room and is drawn at the line's top, as the original's (rdb:// an icon of the database, tdb://id:NAME a picture of the skin, a number of 0 the default icon); text beside a taller picture hangs from the line's top too. The NPC answers' bullets and a mission's reward icons show. Of bare tdb:// numbers only the four the original's own markup writes are known. Looked at offscreen and beside the original's screenshot of an NPC's answers; not in the live game.

#### `WIN-118` Land control Build and War buttons and placement circle not ported

- MISSING, medium
- Original: MapControlView_c has Build and War buttons (the land-control/tower build and war menus) beside the Info toggle; a placement circle at the character (MapData_c +0x75, 0xdcdcdc)
- Client: only the Info (land-control) toggle exists
- A player notices: the land control build and war menus cannot be opened from the map
- Where: `crates/mao-client/src/automap.rs:620`; `docs/CAPTURE_GAPS.md:217`
- Decision: [ ] port  [ ] leave

#### `WIN-119` Mission journal does not keep columns, sort or active mission, and has no Update Team Members

- MISSING, medium
- Original: MissionViewConfig's listview_config keeps widths, order and sort; active_mission is kept and its place uploaded again at the start; "Update Team Members" (GiveQuestToMembers) is a row action.
- Client: Not done (GiveQuestToMembers was never captured, so the entry is not sent).
- A player notices: The journal forgets column layout, sort and the active mission after a restart; no Update Team Members.
- Where: `crates/mao-client/src/missionlog.rs:29`; `crates/mao-client/src/missionlog.rs:408`; `docs/CLIENT_UI.md:3118`; `docs/GUI_DLL_PARITY.md:87`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): the journal's widths and the list's sort are kept in lists.ini for the NCU windows, the nano programs and the mission journal (layout::ListConfig, multilist::Keeper); the active mission and Update Team Members not done.

#### `WIN-120` Playfield map: shadow textures absent, no second patch texture, no rooms' pictures

- MISSING, medium
- Original: PFMapGroundRenderer_c shadow texture per patch (FUN_100b7c29, type 0xf4247), a second texture, rooms' pictures FUN_100474b7
- Client: ground tile textures under the veil's dimming only; no shadow maps because today's game files have none
- A player notices: the playfield map lacks shadow detail and room pictures
- Where: `docs/ORIGINAL_AUDIT.md:106`; `crates/mao-client/src/automap.rs:417`
- Decision: [ ] port  [ ] leave

#### `WIN-121` Text views that are not editable cannot be selected by mouse

- MISSING, medium
- Original: TextView_c/TextRenderer_c support selection and copying, bold/italic faces, shadows
- Client: only editable text views select; others only click links and scroll; bold, italic, shadows and scroll bar art not done
- A player notices: info and chat text in XML views cannot be selected by drag
- Where: `crates/mao-client/src/gui/controls.rs:916`; `docs/GUI_DLL_PARITY.md:45`
- Decision: [ ] port  [ ] leave

#### `WIN-122` Buttons lack icons, drag and auto-repeat

- MISSING, low (notes only)
- Original: Button_c/ButtonBase_c support icons, drag and repeat
- Client: gui::controls::Button has size, clicks, toggles only
- A player notices: buttons with icons, drag or held-key repeat behave plainly
- Where: `docs/GUI_DLL_PARITY.md:36`
- Decision: [ ] port  [ ] leave

#### `WIN-123` City map: no tip with the pointer's place

- MISSING, low
- Original: The base map view provides a tooltip (FUN_100478c7) titled PointerPosition with the world x and z under the pointer; whether the tooltip machinery asks this view was not traced.
- Client: No tip over the map.
- A player notices: Hovering over the city map shows no coordinates.
- Where: `crates/mao-client/src/cityterminal.rs:34`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done with WIN-110: the title PointerPosition and the world's x and z as %.1fx%.1f, through the general tip. The map view does answer the tips' question (BuildingMapView_c's vtable slot +0x98, FUN_100478c7).

#### `WIN-124` Dialog boxes: no text-field form, no close time-out, not modal

- MISSING, low (notes only)
- Original: DialogBox_c family: DialogBoxForm_c with a text field, close time-out, modal.
- Client: chrome::dialog/question: none of these.
- A player notices: Dialogs can be ignored while playing; no timed closing.
- Where: `docs/GUI_DLL_PARITY.md:51`
- Decision: [ ] port  [ ] leave

#### `WIN-125` Factions: IsFactionTitleShown not kept between runs

- MISSING, low (notes only)
- Original: IsFactionTitleShown persisted
- Client: not kept
- A player notices: faction title/standing view resets each run
- Where: `docs/GUI_DLL_PARITY.md:99`; `crates/mao-client/src/factions.rs:12`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): kept in tabs.ini under its name; the default is 1 (titles), as the original's.

#### `WIN-126` Friends window: chat-windows folder lists one window, no row menu, no flashing

- MISSING, low
- Original: Lists every chat window; each row has its own menu (FUN_100a8a69: visual settings, click-through, text input); a row flashes with unread tells.
- Client: One row for the client's chat window; no menu; no flashing.
- A player notices: No flashing friend rows for unread tells; fewer chat window entries.
- Where: `crates/mao-client/src/friends.rs:19`
- Decision: [ ] port  [ ] leave

#### `WIN-127` In-game web pages unavailable where there is no web view (Linux)

- MISSING, low
- Original: Shows item store, market, customer service and daily rewards pages in Awesomium.
- Client: With no web view the window says so and gives the page's address.
- A player notices: No store/market/rewards pages on Linux.
- Where: `crates/mao-client/src/browser.rs:185`; `docs/PLATFORMS.md:15`
- Decision: [ ] port  [ ] leave

#### `WIN-128` Item slots: icon flags, open slot picture, dragging from a slot not ported

- MISSING, low (notes only)
- Original: ItemSlotView_c icon flags, open slot picture 0x11e, dragging from a slot
- Client: not ported (no drag handling found in gui/itemslot.rs)
- A player notices: slots in XML windows cannot be dragged from
- Where: `docs/CLIENT_UI.md:3008`
- Decision: [ ] port  [ ] leave

#### `WIN-129` LayoutGrid not ported; StringListView a plain view

- MISSING, low (notes only)
- Original: The view tree has LayoutGrid and the XML registry has StringListView.
- Client: No LayoutGrid; StringListView stands in as a plain view (no skin uses it).
- A player notices: None visible with the shipped skins.
- Where: `docs/GUI_DLL_PARITY.md:33`
- Decision: [ ] port  [ ] leave

#### `WIN-130` List views: no column swapping by dragging, no horizontal scroll bar, no saved column layouts

- MISSING, low
- Original: ColumnHeaderView_c lets a label be dragged to swap columns; MultiListView_c restores hidden flag and width from listview_config (LoadColumnInfo 0x10136a6d); ListViewBase_c has a horizontal scroll bar.
- Client: Column sort by pressing a label and width dragging are done (list mode exists for the NCU and nano programs windows); dragging a label to swap columns is not done, and widths/sort are not kept (listview_config).
- A player notices: Columns cannot be reordered and list layouts reset each session.
- Where: `crates/mao-client/src/gui/multilist.rs:11`; `crates/mao-client/src/ncu.rs:181`; `crates/mao-client/src/nanoprograms.rs:168`; `docs/GUI_DLL_PARITY.md:47`; `docs/ORIGINAL_CODE_QUESTIONS.md:50`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): widths and the list's sort are kept in lists.ini for the NCU windows, the nano programs and the mission journal (layout::ListConfig, multilist::Keeper); column swapping, hidden columns not done.

#### `WIN-131` Mail: column widths and sort not remembered

- MISSING, low (notes only)
- Original: the mail window remembers widths and sort
- Client: not remembered
- A player notices: columns reset
- Where: `docs/GUI_DLL_PARITY.md:86`
- Decision: [ ] port  [ ] leave

#### `WIN-132` Menu ItemTemplate (special action menu styling) not kept

- MISSING, low
- Original: an `<ItemTemplate>` styles the special actions' menus
- Client: not kept
- A player notices: those menus are styled plainly
- Where: `crates/mao-client/src/gui/menu.rs:5`
- Decision: [ ] port  [ ] leave

#### `WIN-133` Mission journal pays level 200-220 readers in experience, not shadow knowledge

- MISSING, low
- Original: Pays a reader of level 200 to 220 in shadow knowledge (GetSKForXP).
- Client: The experience row is not adapted ("not ported").
- A player notices: The reward row is wrong for high-level characters.
- Where: `crates/mao-client/src/missionlog.rs:320`; `docs/CLIENT_UI.md:3120`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): the same port as NPC-035.

#### `WIN-134` Mission offer: no right-click menu and no new-mission tip

- MISSING, low
- Original: Offer's right-click menu (its page, "Upload to map" again) and the tip at a new mission (OnReceiveMission). The window closing at Accept is now done.
- Client: Menu and tip not done.
- A player notices: No offer menu or new-mission tip.
- Where: `crates/mao-client/src/mission.rs:592`; `docs/CLIENT_UI.md:4060`; `docs/ORIGINAL_AUDIT.md:73`
- Decision: [ ] port  [ ] leave

#### `WIN-135` Mission request window does not keep its sliders or hide the advanced ones

- MISSING, low
- Original: Keeps the sliders between sessions (MissionSelectionViewConfig); shows the six others only with its "advanced options" button down.
- Client: Not done; the sliders start at fixed values each time the window is made.
- A player notices: Sliders reset each time; all seven always show.
- Where: `crates/mao-client/src/mission.rs:315`; `docs/CLIENT_UI.md:4040`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): read (MissionSelectionViewConfig: show_advanced_options, difficulty, dim_good, dim_ctrl, dim_secret, dim_mystery, dim_stealth, dim_reward; the toggle hides sliders 1 to 6); changes the window's layout, so left for a time when it can be looked at.
- Done in part (2026-11-18, overnight, later the same night): the seven sliders are kept in tabs.ini under the config's names; the advanced options button is still not done.

#### `WIN-136` Notification window: links, saved frame not ported

- MISSING, low
- Original: links in a notification line open in the info view when ClickableLinks is set; the window uses its saved config (NotificationWindowConfig)
- Client: links not ported; the lines are plain text without markup; the frame is always the top third of the screen; text colour 0 drawn white
- A player notices: no clickable links in notifications; a saved window shape is not used
- Where: `crates/mao-client/src/notifications.rs:17`; `docs/GUI_DLL_PARITY.md:103`
- Decision: [ ] port  [ ] leave

#### `WIN-137` PowerbarView draws no labels

- MISSING, low
- Original: PowerbarView_c in XML windows (limbo/progress/raid bars) has labels.
- Client: Draws background, caps and fill only; label, left_label, right_label are not drawn.
- A player notices: Bars in XML windows show no label text.
- Where: `crates/mao-client/src/gui/powerbar.rs:11`; `docs/CLIENT_UI.md:3048`
- Decision: [ ] port  [ ] leave

#### `WIN-138` Reclaim window absent

- MISSING, low (notes only)
- Original: reclaim_window, /reclaim
- Client: none
- A player notices: no reclaim
- Where: `docs/GUI_DLL_PARITY.md:105`
- Decision: [ ] port  [ ] leave

#### `WIN-139` Research window: the research blink is not cleared and requirement text differs

- MISSING, low
- Original: The window clears the "got_tech" value on opening (control center research blink) and describes requirements with N3Msg_GetCriteriaString.
- Client: Left out; the requirements text is N3Msg_GetUseByString's. (ResearchConfig placement is now mapped by aoprefs.rs:38.)
- A player notices: The research blink stays on; requirement wording may differ.
- Where: `crates/mao-client/src/research.rs:52`; `docs/GUI_DLL_PARITY.md:100`
- Decision: [ ] port  [ ] leave

#### `WIN-140` Reward claims are asked for each time; lock 0x8c not kept

- MISSING, low
- Original: CharacterAction 0x107/0x108 are held back while its lock 0x8c runs (FUN_1006414f) and it says so
- Client: asks the server every time
- A player notices: a claim can be requested while the original would refuse and say so
- Where: `docs/ORIGINAL_AUDIT.md:34`; `crates/mao-client/src/net.rs:7384`; `docs/GUI_DLL_PARITY.md:106`; `docs/CLIENT_UI.md:1098`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): the claims wait while the lock of stat 0x8c runs and say UnableToPerformAction (World::claims_refusal); the claims also name the character as their target now, as the original's captures do.

#### `WIN-141` Score bars have no maximum; attack rating is the client's own sum

- MISSING, low
- Original: the score bars ask Gamecode for the next rank's score as their maximum; the attack rating is asked of Gamecode (stat 22)
- Client: duel, solo and team score bars have no maximum; the attack rating is the client's own sum
- A player notices: those bars show only a number with an empty fill; attack rating may differ
- Where: `crates/mao-client/src/stats.rs:17`; `crates/mao-client/src/stats.rs:52`; `docs/GUI_DLL_PARITY.md:79`
- Decision: [ ] port  [ ] leave

#### `WIN-142` Skills window does not refuse a "temporarily changed" skill

- MISSING, low (notes only)
- Original: FUN_100642cd refuses to raise a temporarily changed skill.
- Client: Refusal not ported (no trace of it in the code).
- A player notices: A temporarily changed skill can be raised where the original refuses.
- Where: `docs/GUI_DLL_PARITY.md:78`; `docs/ORIGINAL_AUDIT.md:88`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-18, overnight): read: N3Msg_GetSkillMax gives 0 for a stat a 0xcf76 spell holds, with Feedback_CantChangeTemporarilyChangedSkill; the client keeps the set (World::saved_stats) but the window is not handed it. Rare; left.

#### `WIN-143` Slider has no gfxid_slider art choice and no steps

- MISSING, low (notes only)
- Original: Slider_c honours the `gfxid_slider` art choice and steps
- Client: only the option slider's pictures, continuous
- A player notices: sliders all look alike and move freely
- Where: `docs/GUI_DLL_PARITY.md:40`
- Decision: [ ] port  [ ] leave

#### `WIN-144` Slider knob and track: picture-number overrides not read, plain fallbacks drawn

- MISSING, low
- Original: gfxid_slider / gfxid_background picture numbers
- Client: not read ("no skin gives them"); when the skin lacks the slider pictures a dark bar and a plain knob rectangle are drawn
- A player notices: sliders look plain with skins lacking the art
- Where: `docs/CLIENT_UI.md:3040`; `crates/mao-client/src/gui/controls.rs:427`; `crates/mao-client/src/gui/controls.rs:447`
- Decision: [ ] port  [ ] leave

#### `WIN-145` Team: raid window's menu and team search's invite not ported

- MISSING, low
- Original: raid window menu; team search's invite
- Client: not ported (the looking-for-team window's Search, Invite, Tell send nothing)
- A player notices: missing menus, Invite does nothing
- Where: `docs/ORIGINAL_AUDIT.md:105`; `crates/mao-client/src/lft.rs:86`
- Decision: [ ] port  [ ] leave

#### `WIN-146` Team: team side message 0x36, leader's own button, grey of members away not done

- MISSING, low (notes only)
- Original: message 0x36 (TeamSide_isX); leader's own button; members away shown 0x888888
- Client: left out
- A player notices: missing team window features
- Where: `docs/CAPTURE_GAPS.md:299`; `docs/GUI_DLL_PARITY.md:91`; `docs/CLIENT_UI.md:573`
- Decision: [ ] port  [ ] leave

#### `WIN-147` Text input: no right-click menu, no IME, no name-entry dialog

- MISSING, low (notes only)
- Original: TextInputView_c has a right-click menu, IME, and TextInputDialogForm_c ("type a name" dialog).
- Client: None of these; chat input plain.
- A player notices: No context menu in text fields, no IME composition.
- Where: `docs/GUI_DLL_PARITY.md:44`
- Decision: [ ] port  [ ] leave

#### `WIN-148` Tradeskill window: build feedback signals not handled

- MISSING, low
- Original: The window listens to two global signals (GlobalSignals +0x1d8, FUN_100de45e, and +0x1d4) that feed its build feedback text.
- Client: Neither is handled; their senders were not traced.
- A player notices: Some build feedback lines never appear.
- Where: `crates/mao-client/src/tradeskill.rs:23`
- Decision: [ ] port  [ ] leave

#### `WIN-149` ViewSelector: no list view link; HotSpot_t etc. not ported

- MISSING, low (notes only)
- Original: ViewSelector_c list link; HotSpot_t, NewGuiSpot_t, ClearRect_t, RenderInfoView_t
- Client: not ported
- A player notices: none visible
- Where: `docs/GUI_DLL_PARITY.md:57`
- Decision: [ ] port  [ ] leave

#### `WIN-150` Wear and Shop icons do not fit the control-center strip

- MISSING, low (notes only)
- Original: CCMenu lays the icons out.
- Client: Note says six 48 px icons do not fit the 214 px strip as drawn; the strip draws four icons a side and a separate MINI_TOOLBAR holds Wear (no Shop entry found).
- A player notices: Some control-center icons missing or misplaced.
- Where: `docs/ORIGINAL_UI_PLAN.md:206`; `crates/mao-client/src/gui/controlcenter.rs:91`
- Decision: [ ] port  [ ] leave

### Windows and interface: the client has it, the original does not (30)

#### `WIN-151` Fog-of-war corner map and overlay map are the client's own

- ADDITION, high
- Original: not stated; the original's map is a square PF Map window (the client's round map is the user's choice); the target buttons' real function was a guess in the notes
- Client: Diablo-style fog-of-war map built from walked cells and collision data; a round corner map turning with the camera, draggable, resizable (120-600 pts), zoomed with the wheel, marking north, with place and size saved in automap.ini; a large translucent overlay (toggle key, drag to pan, wheel to zoom, Space re-centres); the explored cells are not saved between sessions
- A player notices: a minimap and full-screen map unlike the original's; explored area forgotten each session
- Where: `crates/mao-client/src/automap.rs:708`; `docs/CLIENT_UI.md:1657`; `docs/CLIENT_UI.md:2048`; `docs/CLIENT_UI.md:2016`
- Decision: [x] keep  [ ] remove
- Done (2026-11-18): kept as a setting (own_map, off): the original's PF Map window is the default (AutoMap::pf_window): north up at one pixel a unit over black, following the character, dragged at 1.5 and back on release, land control and towers, characters by side as squares, its arrow, the pointer's place; map_window in the control center and WINDOW_MAP now open it. Not done: room floors, vendors and booths, the facing arrows of others, the mission square, the withheld maps.

#### `WIN-152` Smart hotbars with priority lists and queued presses

- ADDITION, high
- Original: the shortcut bar slots (ShortcutBarView) hold one shortcut each (FUN_100d89bd, FUN_100d833f); a macro has no wait of its own
- Client: 6 bars of 10 slots, each slot a priority list of actions with its own cooldown (5 seconds per dropped action, 0 for a macro, editable by right click); a press fires the first one that is ready; when nothing is ready the press is queued and fires by itself later
- A player notices: a slot can hold several actions and presses are remembered
- Where: `crates/mao-client/src/hotbar.rs:1`; `crates/mao-client/src/ui.rs:2071`; `docs/CLIENT_UI.md:103`
- Decision: [x] keep  [ ] remove
- Done (2026-11-18): kept as a setting: since WIN-001 (2026-11-18) the original's shortcut bars are the default and the smart hotbars the client settings' choice (smart_hotbar).

#### `WIN-153` Client settings window beside the original's option panel

- ADDITION, medium
- Original: the option panel is the skin's OptionPanel/Root.xml
- Client: when the skin has no panel (or from the panel's "Client settings") the client's own settings window opens, with key bindings, themes, window mode and size, presets
- A player notices: a settings window the original does not have
- Where: `crates/mao-client/src/ui.rs:943`
- Decision: [ ] keep  [ ] remove

#### `WIN-154` Client's own message box for 14 seconds

- ADDITION, medium
- Original: not stated
- Client: the client's own messages (screenshot saved, not in inventory, Broken macro, camera mode and others) show in a box at the left bottom for 14 s, the last 8 kept
- A player notices: transient status lines outside the chat window
- Where: `crates/mao-client/src/ui.rs:2250`
- Decision: [ ] keep  [ ] remove

#### `WIN-155` Hotbar "is not ready: queued" message and own cooldowns

- ADDITION, medium
- Original: prints the skill-wait line (UnableToPerformActionSkill) for a locked skill and sends nothing (SPECIALS_ORIGINAL.md 2.2)
- Client: the hotbar tells "<name> is not ready (x.xs): queued" and holds the press; each entry has a fixed cooldown number stored in hotbar.ini
- A player notices: different messages and a press that fires later
- Where: `crates/mao-client/src/hotbar.rs:171`
- Decision: [ ] keep  [ ] remove

#### `WIN-156` Map: dots for nearby characters and the character arrow

- ADDITION, medium
- Original: not stated
- Client: booths as cyan rings; players (blue), friendly (green), hostile (red) and the target (yellow, larger) as coloured dots; the character as a gold arrow pointing the way it faces
- A player notices: the map shows creatures and players as live coloured dots
- Where: `crates/mao-client/src/automap.rs:572`; `crates/mao-client/src/net.rs:9581`
- Decision: [ ] keep  [ ] remove

#### `WIN-157` A skin without its own TextColors.xml takes the game's

- ADDITION, low
- Original: not stated
- Client: falls back to the game's Default TextColors.xml; the built-in look carries it too
- A player notices: markup colour names resolve under any skin
- Where: `crates/mao-client/src/theme.rs:108`
- Decision: [ ] keep  [ ] remove

#### `WIN-158` Actions tooltip has the client's own text

- ADDITION, low
- Original: not stated
- Client: name, "Available in N s" and a kind line ("Weapon attack", "Perk action", "Character action") or the description
- A player notices: extra hover text on action tiles
- Where: `crates/mao-client/src/actions.rs:386`
- Decision: [ ] keep  [ ] remove

#### `WIN-159` AOU mod skins bundled and selectable

- ADDITION, low
- Original: not stated
- Client: bundles four AOU GUI mod skins (`assets/gui`, GPL) and lets the player switch theme
- A player notices: optional alternate skins
- Where: `docs/CLIENT_UI.md:164`; `README.md:94`; `crates/mao-client/src/theme.rs:40`
- Decision: [ ] keep  [ ] remove

#### `WIN-160` Creature inspect page is the client's own headline with health

- ADDITION, low
- Original: Not stated.
- Client: For a creature the page is the client's own headline with health.
- A player notices: Creature info page differs.
- Where: `crates/mao-client/src/net.rs:2699`
- Decision: [ ] keep  [ ] remove

#### `WIN-161` Error boxes are queued, never repeated

- ADDITION, low
- Original: GlobalSignals_c +0x184 raises an error text for a refusal
- Client: error boxes shown one after another; a text already waiting is not queued twice
- A player notices: a repeated refusal shows one box
- Where: `crates/mao-client/src/chrome.rs:59`
- Decision: [ ] keep  [ ] remove

#### `WIN-162` Escape always closes the client's own Options, Whom-Pah, Browser and Help windows

- ADDITION, low
- Original: the esc_* options decide which windows Escape closes; windows the original has not made read none
- Client: the client's own windows always close with Escape, with no option
- A player notices: Escape closes windows the original does not have
- Where: `crates/mao-client/src/ui.rs:406`
- Decision: [ ] keep  [ ] remove

#### `WIN-163` Help assistant ("Ask the guides") is the client's own window

- ADDITION, low
- Original: The original has nothing like it (only its help files and the Ctrl+H keys window).
- Client: A window (/open assistant, Help menu) ranks pieces of the game's help pages and optionally AO-Universe guides by word matching; optionally a downloaded local model (Qwen2.5 1.5B, 1.1 GB, frame rate dips) or an OpenAI/Anthropic-protocol endpoint under [help]; reads sites with the system's curl, off by default.
- A player notices: An extra window and extra network use when sources are turned on.
- Where: `crates/mao-client/src/helpwindow.rs:2`; `docs/HELP_ASSISTANT.md:3`; `docs/CLIENT_UI.md:3295`; `README.md:91`
- Decision: [ ] keep  [ ] remove

#### `WIN-164` Help window (Ctrl+H) with find and chat input history

- ADDITION, low
- Original: not stated (the original's help is separate, see helpwindow.rs)
- Client: a window listing keys as bound now, mouse controls and the chat commands the client carries out, with a Find field; Up/Down history of the last 100 chat lines
- A player notices: a help window and input history the original does not have
- Where: `crates/mao-client/src/help.rs:1`; `docs/CLIENT_UI.md:1848`; `docs/GUI_DLL_PARITY.md:109`
- Decision: [ ] keep  [ ] remove

#### `WIN-165` Hover breakdown tooltips on stats and skills

- ADDITION, low
- Original: Not in the original: health and nano bars show text.mdb's tip (TooltipHealth ...).
- Client: Resting on a value shows Trained / Equipment / Perks / Nano buffs / Attributes / Organization parts and each modifier's source.
- A player notices: Extra tooltip with value breakdown.
- Where: `crates/mao-client/src/stats.rs:17`; `docs/CLIENT_UI.md:1675`; `docs/STAT_ENGINE.md:420`
- Decision: [ ] keep  [ ] remove

#### `WIN-166` Inbox status line and "Sending ..." text

- ADDITION, low
- Original: MailWindow_c shows InboxSize "n/30" only
- Client: a status line under the list ("The inbox holds N messages.", "Sending ...", the delete refusal)
- A player notices: extra text the original does not show
- Where: `crates/mao-client/src/mailbox.rs:115`
- Decision: [ ] keep  [ ] remove

#### `WIN-167` Info view file:// pages also search the game's text folder

- ADDITION, low
- Original: FUN_100ee973 `file://<path>`: the path as it is, else under the game's help folder
- Client: tries the path, then the help folder, then the game's text folder (an extra place)
- A player notices: more places searched for text files
- Where: `crates/mao-client/src/gui/infoview.rs:71`
- Decision: [ ] keep  [ ] remove

#### `WIN-168` Inspect window: title carries the character's name, right click opens the item page

- ADDITION, low
- Original: the Inspect window is the wear view built with the other identity, titled Inspect; a list with no click listeners opens the item menu at once on a right press (FUN_100e24b8, 10041c04)
- Client: title "Inspect: <name>"; a right click shows the item's info page; no menu
- A player notices: a different title and a different right-click result in the Inspect window
- Where: `crates/mao-client/src/invwindows.rs:265`
- Decision: [ ] keep  [ ] remove

#### `WIN-169` Missing game texts fall back to the client's own words

- ADDITION, low
- Original: the text database (LDBface::GetText; text.mdb 506 for character info labels, 508 for the organization page, 2005 for side names) gives the wording
- Client: where the game's text is missing, the client's own words are used (own labels, "neutral/clan/omni")
- A player notices: windows show the client's wording when the game's text files are absent
- Where: `crates/mao-client/src/charinfo.rs:45`; `crates/mao-client/src/net.rs:771`; `crates/mao-client/src/gui/xml.rs:365`
- Decision: [ ] keep  [ ] remove

#### `WIN-170` Performance overlay

- ADDITION, low
- Original: not in the original
- Client: /perf or Options > Graphics > Show performance shows frame rate, frame-time graph, processor, memory, GPU time, adapter, draw counts, window and render size, anti-aliasing
- A player notices: an overlay at the top left
- Where: `crates/mao-client/src/perf.rs:1`; `crates/mao-client/src/viewer.rs:2668`
- Decision: [ ] keep  [ ] remove

#### `WIN-171` Plain strip drawn when there is no skin

- ADDITION, low
- Original: not stated
- Client: Without a skin the old plain strip (vitals::show) is drawn.
- A player notices: Non-original look when there is no skin.
- Where: `crates/mao-client/src/ui.rs:1109`; `docs/CLIENT_UI.md:201`
- Decision: [ ] keep  [ ] remove

#### `WIN-172` Planet map window: map choice list inside the window

- ADDITION, low
- Original: the map is picked by the options PlanetMapIndexFile and ShadowlandMapIndexFile (FUN_1004cb15 lists the indexes for the choice)
- Client: a drop-down under the buttons lists every map of the planet's kind and saves the pick as the option
- A player notices: a player can change the planet map from the window itself
- Where: `crates/mao-client/src/planetwindow.rs:199`
- Decision: [ ] keep  [ ] remove

#### `WIN-173` Shop status line "Trade complete."

- ADDITION, low
- Original: not stated
- Client: a status string "Trade complete." is set when the trade completes
- A player notices: minor extra text
- Where: `crates/mao-client/src/shop.rs:302`
- Decision: [ ] keep  [ ] remove

#### `WIN-174` Shop window title carries the shop's name

- ADDITION, low
- Original: The skin's ShopTrade view leaves PartnerName out.
- Client: The window title carries the shop's name (or "Booth <id>").
- A player notices: A name shows in the title bar that the original does not show.
- Where: `crates/mao-client/src/shop.rs:426`
- Decision: [ ] keep  [ ] remove

#### `WIN-175` Skills window messages use the client's own words

- ADDITION, low
- Original: Refusals come from text.mdb (Skill_LackIP); no status line is described for the others.
- Client: Own status texts such as "This skill has been disabled and can not be increased.", "Changes sent.", "Reset asked for."
- A player notices: Extra text under the skills window.
- Where: `crates/mao-client/src/skillwindow.rs:553`
- Decision: [ ] keep  [ ] remove

#### `WIN-176` The client can build planet maps of its own and list any map folder

- ADDITION, low
- Original: the original's planet map comes from the fixed PlanetMap index files shipped with the game
- Client: a planet map can be made of one's own pictures (planetmap::pack, index_text, `planetmapbuild` example, `tools/planetmap_fit.py`); discover lists every index file one folder deep in any maps' folder so maps made here (Atlas of Rubi-Ka) show beside the game's
- A player notices: the planet map window shows maps the original does not have
- Where: `CHANGELOG.md:76`; `crates/mao-formats/src/planetmap.rs:12`
- Decision: [ ] keep  [ ] remove

#### `WIN-177` The client's own tool windows (damage meter, item finder, guide)

- ADDITION, low
- Original: The original has none of these windows.
- Client: Damage meter (/damage, /dps), item finder (/find), tradeskill guide and others are opened, closed (/close) and drawn by name from extras.rs.
- A player notices: Extra windows and chat commands.
- Where: `crates/mao-client/src/extras.rs:1`; `crates/mao-client/src/ui.rs:343`
- Decision: [ ] keep  [ ] remove

#### `WIN-178` Web pages: loading page and 8 s / 30 s proxy time-outs

- ADDITION, low
- Original: not stated
- Client: A "Loading ..." page shows at once; proxy requests time out after 8 s to connect and 30 s in all.
- A player notices: A loading line instead of a blank pane; a silent site is reported in seconds.
- Where: `crates/mao-client/src/browser.rs:302`
- Decision: [ ] keep  [ ] remove

#### `WIN-179` Window icon is the client's own emblem on a slate square

- ADDITION, low
- Original: not stated
- Client: the original GUI's steel emblem (GFX_GUI_AO_LOGO) on a slate square as the executable's resource and every window's icon
- A player notices: the window icon differs from the original's
- Where: `README.md:87`; `crates/mao-client/src/icon.rs:1`
- Decision: [ ] keep  [ ] remove

#### `WIN-180` Windows that can be docked include ones the original does not dock

- ADDITION, low
- Original: docks the wear, NCU, nano program, team and faction windows by default
- Client: also lets Friends, PetWindow, Stats, Missions and MissionView be docked
- A player notices: more windows have a dock button
- Where: `crates/mao-client/src/rollup.rs:23`
- Decision: [ ] keep  [ ] remove

## Camera

### Camera: the client knowingly does otherwise (3)

#### `CAM-001` WoW-style camera controls

- OWN-RULE, high
- Original: not stated
- Client: left drag orbits, right drag turns camera and character, both buttons walk forward, camera swings behind the character while moving, cursor hidden and confined while a button is held
- A player notices: camera behaves like WoW
- Where: `docs/CLIENT_UI.md:83`; `crates/mao-client/src/viewer.rs:1095-1125`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the WoW scheme had already given way to the original's ActionViewMouseHandler_c; now also its sensitivity (MouseTurnSensitivity / 1000 radians a count), the wheel's ZoomSpeed / 10 step, the look of the last button pressed ended by either release, and the turn keys strafing once any look moves. Not ported: the first-person head turn handed to the character at a look's end, the RMBMouseLook1st/3rd prefs, the default camera mode (Lock in the game, trail here). The middle-button reset is the client's own.

#### `CAM-002` Camera mode comes from the client's setting; unknown names give rubber

- OWN-RULE, low
- Original: the default when nothing is preferred is mode 1, the free camera (FUN_10021859); no /setoption
- Client: `camera_mode` (free/trail, rubber, lock; default trail = free) set by /setoption or the interface; any name other than "lock" and "trail" gives the rubber camera (mode 2); whether the jitter is still there in the free camera is unverified
- A player notices: an old or odd setting gives the rubber camera instead of the free one
- Where: `crates/mao-client/src/viewer.rs:1042-1049`; `crates/mao-client/src/settings.rs:641-654`; `docs/LIVE_TEST_LIST.md:368`
- Decision: [ ] port  [ ] leave

#### `CAM-003` Camera smoothing scaled to sixty frames a second

- OWN-RULE, low
- Original: moves its look target a fifth of the way and its snapped camera a tenth every frame, whatever the frame took (FUN_10022345 turn keys 0.02 rad a frame)
- Client: shares are taken per sixtieth of a second (`keep_for`, `camera::ORIGINAL_FPS`; "sixty is this client's choice"); turn keys, locked camera easing and look target scale by elapsed time; per frame only with a developer switch
- A player notices: the camera feels the same at any frame rate; the original's would be tighter or looser
- Where: `crates/mao-client/src/camera.rs:113-119`; `crates/mao-client/src/viewer.rs:2085-2089`; `docs/CLIENT_UI.md:1298`
- Decision: [ ] port  [ ] leave

### Camera: an approximation or a stand-in (2)

#### `CAM-004` Camera line of sight: props' meshes stand in without surfaces

- SIMPLIFIED, low
- Original: n3Playfield_t::LineOfSight uses the surfaces' GetLineIntersection
- Client: the props' meshes stand in where a playfield has no surfaces
- A player notices: camera collision differs on such playfields
- Where: `docs/ORIGINAL_AUDIT.md:202`; `crates/mao-client/src/viewer.rs:776`
- Decision: [ ] port  [ ] leave

#### `CAM-005` Zooming in always ends in first person

- SIMPLIFIED, low
- Original: closer than 0.8 while zooming in switches to first person only when ZoomTo1stPerson is set
- Client: the setting is not checked: any zoom in past 0.8 switches to first person
- A player notices: players who turned ZoomTo1stPerson off are still thrown into first person
- Where: `crates/mao-client/src/camera.rs:482`; `crates/mao-client/src/camera.rs:24`
- Decision: [ ] port  [ ] leave

### Camera: the original was not read: inferred or measured (2)

#### `CAM-006` Mouse turn, zoom speed and wheel step are fixed numbers

- GUESS, medium
- Original: MouseTurnSensitivity and ZoomSpeed are interface values (settings); rotate keys 0.02 rad a frame; flying pitch scaled by MouseTurnSensitivity; a mode-3 eased zoom (+0x204, 3 units a second) which nothing in the decompile sets
- Client: sensitivity fixed at 1 and zoom 10 units a second ("feel like the original"), mouse look 0.005 rad per pixel (the client's), each wheel notch a quarter second of zoom, distance 0.8 to 25; no settings for them (MouseLookInverted is now supported)
- A player notices: camera and flight turn and zoom speed cannot be changed and may not match the original's defaults
- Where: `crates/mao-client/src/camera.rs:199-200`; `crates/mao-client/src/camera.rs:183`; `crates/mao-client/src/viewer.rs:972-973`; `docs/CLIENT_UI.md:2142`; `docs/ORIGINAL_AUDIT.md:208`; `docs/FLIGHT_ORIGINAL.md:249` and 2 more
- Decision: [ ] port  [ ] leave

#### `CAM-007` Camera near and far planes computed from the scene

- GUESS, low
- Original: not stated (the view is 90 degrees across, as SetViewPlaneWindow)
- Client: far plane is the camera distance plus four scene radii, near plane a thousandth of it between 0.01 and 0.2
- A player notices: how far the world is drawn before the far plane cuts it
- Where: `crates/mao-render/src/lib.rs:895`
- Decision: [ ] port  [ ] leave

### Camera: a guard or fallback the original lacks (2)

#### `CAM-008` Camera frame time clamped to a quarter second

- SAFETY, low
- Original: Vehicle_t::Run takes substeps of at most 0.4 s; no cap on the frame time is stated
- Client: a frame's time is clamped to 0.25 s for the camera (and the vehicle's integration to 4 s) so a long stall does not fling it
- A player notices: after a hitch the camera catches up more slowly
- Where: `crates/mao-client/src/camera.rs:524`
- Decision: [ ] port  [ ] leave

#### `CAM-009` Camera pulled in to stay above the ground

- SAFETY, low
- Original: not stated (the camera's own vehicle settles against the surface in the original's way)
- Client: after the camera is placed, the distance is shrunk by 12 % up to 24 times until the eye is 0.6 above the heightmap
- A player notices: the camera comes nearer the character on slopes so the ground does not hide the view
- Where: `crates/mao-client/src/viewer.rs:2548-2570`
- Decision: [ ] port  [ ] leave

### Camera: the original does it, the client does not (3)

#### `CAM-010` Free camera for playfield 0x380 not ported

- MISSING, low (notes only)
- Original: SetMainDynel also makes playfield 0x380 use the free camera
- Client: not ported
- A player notices: that playfield uses the normal camera
- Where: `docs/CLIENT_UI.md:2788`
- Decision: [ ] port  [ ] leave

#### `CAM-011` Room camera attractors: manual cycling and collider visibility not ported

- MISSING, low
- Original: GetNextVisibleAttractor manual cycling; a two-step door search over the collider for the camera's "visible" test; manual next/previous camera view keys
- Client: none ported (old CHGA tile-map branch not ported either)
- A player notices: camera attractors chosen without the original's visibility test; no view-cycling keys
- Where: `docs/ORIGINAL_AUDIT.md:206`; `docs/CLIENT_UI.md:1711`; `crates/mao-client/src/camera.rs:39`
- Decision: [ ] port  [ ] leave

#### `CAM-012` Two-shot and secondary-target camera framing not ported

- MISSING, low
- Original: the original's camera (n3Camera_t) has a two-shot and a secondary-target framing
- Client: not ported
- A player notices: the camera never frames two subjects together
- Where: `crates/mao-client/src/camera.rs:47`; `docs/CLIENT_UI.md:2144`; `docs/ORIGINAL_AUDIT.md:62`
- Decision: [ ] port  [ ] leave

### Camera: the client has it, the original does not (1)

#### `CAM-013` Camera mode change prints a chat line

- ADDITION, low
- Original: not stated
- Client: choosing a third-person camera mode in the interface says "Camera: third person locked/trailing (free)/rubber"
- A player notices: a status line the original may not print
- Where: `crates/mao-client/src/viewer.rs:2954`
- Decision: [ ] keep  [ ] remove

## Rendering and world

### Rendering and world: the client knowingly does otherwise (19)

#### `RND-001` Client's own sky and light when the scripts give none

- OWN-RULE, medium
- Original: Direct3D fog; sky and light come from the effect scripts and the area
- Client: fog density itself unused; without scripts (interior, or no sky scripts) a fixed day or dark sky pair and the client's own light stand in, scaled by day intensity, keeping a fifth at night; terrain water's reflected sky is the client's DAY_SKY; interiors ignore the clock
- A player notices: lighting in interiors and sky colour differ from the original
- Where: `crates/mao-render/src/lib.rs:465`; `docs/CLIENT_UI.md:2134`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The stand-in sky and light are used only where no effect script gives them (interiors, playfields without sky scripts). What the original shows there is set by the environment scripts it runs for every playfield kind (playfield.rs sky_scripts by environment kind) and by the rooms' light maps; which of those an interior gets, and what colour the original clears to when there is no sky, is not stated in what was read tonight and can only be settled against original screenshots of an interior (the capture folders) with the game hour matched. Left for a comparison session.

#### `RND-002` Plants kept cut out at half alpha instead of blended

- OWN-RULE, medium
- Original: draws plants as their material's states say; why the original's trees do not thin with distance was not found; no alpha test (randy31 state 15 = 0)
- Client: foliage is alpha-tested at a half, never blended (blended crowns thin to a veil with distance)
- A player notices: tree crowns stay dense at a distance; edges may differ
- Where: `crates/mao-render/src/lib.rs:239`; `docs/CLIENT_UI.md:686`; `docs/CLIENT_UI.md:3723`; `docs/ORIGINAL_AUDIT.md:22`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original draws plants by their materials' states with no alpha test (randy31 render state 15 = 0) and blends; the reason its crowns do not thin to a veil with distance (most likely how its D3D7 mip chain treats alpha, not a rule in the program) was not found. Switching the client to blending brought the veil back before, and the cut-out at half alpha is a stand-in for an unfound cause; deciding needs a side-by-side look at a distant tree. Left.

#### `RND-003` Rain, fallout and sand dust are the client's own particles

- OWN-RULE, medium
- Original: VisualRain_t (FUN_100b9da8 / FUN_100b9ba5) draws the rain; streak layout ported in the drop-in; dust rule not stated
- Client: rain, red and green fallout and sand dust are particles in a 14 m box round the eye, at most 700 each, fixed colours, fall speeds and wind drift; dust is faint tan motes along the wind
- A player notices: rain, sandstorms and fallout look different from the original's streaks
- Where: `crates/mao-client/src/precipitation.rs:7`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): precipitation.rs is a port of VisualRain_t now (see RND-138): the original's 350 lines per cloud, its colours, falls and alphas; the rain is entry 0, the storms entries 3 to 6. Still the client's own: lines drawn as thin sheets.

#### `RND-004` Spell flight 0.45 s and landing 1.4 s are the client's own timings

- OWN-RULE, medium
- Original: not stated (the tracer's flight speed in the original)
- Client: tracer flies FLIGHT 0.45 s (streak models too), hit effect stays LANDING 1.4 s (longer rings up to 10 s)
- A player notices: spell flight and hit durations are the client's
- Where: `crates/mao-client/src/effects.rs:2919`; `crates/mao-client/src/effects.rs:2931`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The flight time is not one number in the original: _EffectHandler_t::CreateGfxControlTracer (Gamecode 0x100d176c) builds a different control for each tracer kind (0x3f5 FUN_100fe22f, 0x3fb FUN_100fec4e, 0x3fd FUN_100ff48e, 0x3fe FUN_100ff933, 0x400 FUN_10100268, 0x401 FUN_10100b0d, 0x402 FUN_101018ef, 0x403 FUN_101021eb, 0xbd2 FUN_10114e83), each with its own travel words, and the hit effect is started by the spell's own message timing; replacing the client's FLIGHT/LANDING means porting each tracer control's run and the hit's start, a set of effect ports (several catalogued on their own: RND-092, RND-110) well over the time box. Left.

#### `RND-005` Statels farther than 50 radii not drawn

- OWN-RULE, medium
- Original: LODManager swaps a low-detail mesh beyond its distance
- Client: a statel more than 50 of its radii from the camera is not drawn at all
- A player notices: far scenery vanishes at a set distance
- Where: `crates/mao-formats/src/model.rs:211`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Already the original's rule, the catalog line is wrong: DisplaySystem's LODManager::Process (drop-in lodmanager.rs, globals at 0x100ae1a4) hides a watched statel mesh whose squared distance over its squared radius exceeds 2500 (50 radii) and swaps 0xf6951/0xf696a at 6000/4000 squared units beyond the radius, the same three numbers as mao-render's LOD_CULL/LOD_FAR/LOD_NEAR. Small differences remain: the original looks at 20 meshes a frame round the set (so a change can lag a few frames) and measures from the mesh's position with its bounding volume's radius, where the client measures every frame from the world box's centre and half diagonal. Not changed.

#### `RND-006` Terrain keeps slope lighting and shadow map instead of the original's flat-lit ground

- OWN-RULE, medium (notes only)
- Original: AnarchyGround_t gives every vertex an up normal, no slope shading; relief from 0xf4247 shadow PNGs (absent here), flat-lit ground plus FXS ground-light projection
- Client: keeps its slope lighting and shadow map rather than flatten
- A player notices: terrain is shaded by slope where the original is flat-lit
- Where: `docs/ORIGINAL_AUDIT.md:186`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only. The original's flat-lit ground gets its relief from the 0xf4247 ground shadow pictures, which are not in this installation's database; flattening the client's normals without them would lose the relief the original does show. Left until those pictures (or their source) are found.

#### `RND-007` Blended parts sorted per part, faint pixels dropped

- OWN-RULE, low (notes only)
- Original: render list 6 drawn far to near in distance buckets; writes depth for any alpha
- Client: sorts blended parts far to near by each part's own distance; skips pixels with alpha under 0.02 so they write no depth
- A player notices: slight differences in blended sorting
- Where: `docs/CLIENT_UI.md:688`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the original draws list 6 far to near in distance buckets and writes depth for any alpha; skipping faint pixels' depth is the client's guard against halos that the original's buckets avoid differently. Changing it is the same pipeline port as RND-033, judged by eye. Left.

#### `RND-008` Door record without its own mesh draws nothing (template door stands in at shuttle hatches)

- OWN-RULE, low (notes only)
- Original: a door has its record's mesh or its template's
- Client: notes disagree: earlier, doors without a mesh draw nothing instead of the template's white door; later the rule is gone but the plain 'Door' template's small white leaf stands in shuttle hatches at ICC's landing platforms
- A player notices: a small white leaf in some shuttle hatches, or no door
- Where: `docs/CLIENT_UI.md:3787`; `docs/CLIENT_UI.md:2098`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the notes disagree about the client's own behaviour (whether a door without a mesh draws nothing or the template's leaf); the original's rule (record's mesh, else the template's) needs the shuttle hatches seen in game to settle. Left.

#### `RND-009` Interiors use a dark sky backdrop, not black

- OWN-RULE, low
- Original: drew interiors against black
- Client: with no ground the scene uses the dark sky and fog (mao_render::DARK_SKY, nearly black), not pure black
- A player notices: backdrop outside rooms is a near-black colour chosen by the client
- Where: `crates/mao-client/src/viewer.rs:2516`; `crates/mao-render/src/lib.rs:467`; `docs/CLIENT_UI.md:1925`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The original draws interiors against black (the catalog's reading; DisplaySystem's device clears with colour 0, drop-in d3d.rs Clear, slot 10). mao_render::DARK_SKY, the backdrop and fog colour of a playfield with no ground, was a near-black blue of the client's own; it is now black at the horizon and the zenith, so an interior's fog fades into black. A constant; no unit test; built.

#### `RND-010` Left-limb attach offsets set by hand

- OWN-RULE, low
- Original: left limbs' bone frames are mirrored in the Biped rig
- Client: offset directions set by hand per attach id
- A player notices: none visible
- Where: `crates/mao-client/src/effects.rs:3152`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The hand-set left-limb directions belong to the buff rings' fixed figures (RND-022): with the rings on the posed bones' frames the mirrored left frames come from the rig itself. Left with RND-022.

#### `RND-011` Map: the planet map stands in for a missing ground picture

- OWN-RULE, low
- Original: not stated (the planet map is the planet map window's own picture)
- Client: when a playfield on the planet map has no ground picture, the corner map and overlay draw the planet map's picture, with its own zoom scales
- A player notices: some playfields show the planet map picture in the minimap
- Where: `crates/mao-client/src/automap.rs:6`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Original not stated: the original's corner map has no picture where a playfield has none; the stand-in from the planet map is the client's own. Removing it would leave those maps empty; a choice for the user, not a rule to port. Left.

#### `RND-012` Name tag height held at the first height seen

- OWN-RULE, low
- Original: measures the mesh at rest once when the tag is made (N3Msg_GetMeshHeight in FUN_10025603)
- Client: keeps the first posed height seen of a character, replaced only by one a fifth off or more; a character first seen mid-stride keeps that stride's height
- A player notices: tag does not bob with a run, but may be a few hundredths off
- Where: `crates/mao-client/src/nametag.rs:38`; `docs/CLIENT_UI.md:4160`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original measures the mesh at rest once (N3Msg_GetMeshHeight in FUN_10025603); the client's plates are its own (RND-186) and are drawn from the posed body, which has no separate rest measure in the client's tag code. Left with RND-186.

#### `RND-013` Outline map: reach and walls by the client's own rule

- OWN-RULE, low
- Original: not stated (the original has no walked-grid map)
- Client: reveals cells within 75 world units, at most 3000 per step; a wall when the ground rises more than 1.2 cell widths or collision blocks
- A player notices: the outline map's walls and reach are the client's rule
- Where: `crates/mao-client/src/automap.rs:29`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The walked outline map is the client's own feature (the original has no walked grid); its rules have no original counterpart. Left.

#### `RND-014` Planet map defaults to aftermarket maps

- OWN-RULE, low
- Original: the game's own planet map and Shadowlands map (index files by Type)
- Client: defaults to the aftermarket Saavick's Map of Rubi-Ka 3.3 and SLmap 2.22 in the client's own PlanetMap folder; the corner map uses the chosen map at up to 2048 pixels a side
- A player notices: different map art by default
- Where: `crates/mao-client/src/settings.rs:580`; `docs/CLIENT_UI.md:3152`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The default map is a settings choice the user made (the aftermarket maps); the game's own planet map is selectable. Not a rule to port. Left.

#### `RND-015` Renderer uses wgpu instead of Direct3D 7

- OWN-RULE, low (notes only)
- Original: randy31 Direct3D (DX7) renderer
- Client: wgpu picks Direct3D 12 or Vulkan on Windows, Vulkan on Linux, Metal on macOS
- A player notices: a different graphics backend
- Where: `docs/PLATFORMS.md:12`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the renderer API is the client's platform choice. Left.

#### `RND-016` Stray statels of room playfields dropped

- OWN-RULE, low
- Original: not stated
- Client: keeps only statels in or near a room (within the room's larger side plus 10)
- A player notices: props the data places outside rooms are not drawn
- Where: `crates/mao-client/src/main.rs:838`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): n3StatelController_t::ReadGlobalData (N3.dll 0x100250ac) reads every one of the playfield's global statels (FUN_1002777a each) into its list with no test of where they stand. The client dropped a room playfield's statels that stood farther than a room's larger side plus 10 from every room (main.rs placed_statels); the filter is gone, so they are drawn where the file puts them. No unit test (it reads the game's files); built.

#### `RND-017` Sun held above the horizon for the shadow map

- OWN-RULE, low
- Original: not stated
- Client: the shadow direction is lifted to at least 0.15 of height so low or night suns still shade the ground
- A player notices: long, steep shadows at dawn and dusk
- Where: `crates/mao-render/src/lib.rs:2204`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The shadow map is the client's own (RND-188); holding its sun above the horizon is part of that feature. Left with RND-188.

#### `RND-018` Wielded weapon not hung where worn meshes fill that hand

- OWN-RULE, low
- Original: not stated
- Client: skips hanging the weapon if the server's worn-mesh list already has something at that hand
- A player notices: a wielded weapon may not show in that hand
- Where: `crates/mao-client/src/net.rs:5409`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_1009cda2 hangs the wielded weapon's mesh with CharacterMesh::AddAttractorMesh(place 1 or 2, mesh, flags 2 (3 mirrored), 0): a bit-1 mesh, so it is shown alone at that hand, and of two bit-1 meshes at one place the last added wins (FUN_10071e5d keeps them in front). Which comes last, the wield or the server's appearance list (AddAttractors), depends on the order the messages arrive, so the faithful rule cannot be written without knowing that order for a wield; the client's skip avoids a double weapon where the list already holds one. Left.

#### `RND-019` Wind steps at 30 a second with its own random generator

- OWN-RULE, low
- Original: steps once a frame (FUN_100bff68 / FUN_100bfc80); CRT rand()
- Client: steps at 30 a second whatever the frame rate; xorshift generator, the original's sequence not reproduced
- A player notices: gust strength same at any frame rate; gust pattern differs
- Where: `crates/mao-client/src/wind.rs:14`; `crates/mao-client/src/wind.rs:38`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The wind steps once a drawn frame in the original (FUN_100bff68 / FUN_100bfc80) with rand(); the client's fixed 30 a second and own generator are the same law (frame-rate dependence and RND-063). Left.

### Rendering and world: an approximation or a stand-in (78)

#### `RND-020` Character meshes drawn fully opaque

- SIMPLIFIED, high
- Original: a material with alpha transparency is drawn blended unless flag 8 is set, with an opacity value
- Client: build_model gives every character part opacity 1.0 and ignore_alpha true
- A player notices: glass, hair edges or see-through parts drawn solid
- Where: `crates/mao-formats/src/catmesh.rs:435`; `docs/CLIENT_UI.md:56`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the texture list's flag is read as the original reads it: alpha that blends (clear) or alpha that is light (set), by whether the texture has alpha; the material's opacity is used. Two listed textures with a full alpha channel are drawn plain where the original lights them fully. Compared offline, not seen live.

#### `RND-021` Character skin colour is a plain tan

- SIMPLIFIED, high
- Original: the game paints a character's skin colour into the pure-green areas of its textures at runtime
- Client: the green areas get a fixed tan (214,164,128) shaded by the green
- A player notices: every character, where skin colour is not applied elsewhere, has the same tan skin
- Where: `crates/mao-client/src/main.rs:297`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the original has no skin colour: pure green (565 0x07e0) shows the breed's, sex's and Solitus race's naked texture, for hands, body, feet, arms and legs of a body with a head. Remains and morphs take breed and sex from the head's file name. Compared offline, not seen live.

#### `RND-022` Attach points use fixed figures for a standing body, not the posed skeleton

- SIMPLIFIED, medium
- Original: locators from the posed skeleton's bones
- Client: fixed heights/offsets per attach id
- A player notices: effects sit at typical heights, not on the real body part when animated or of non-standard size
- Where: `crates/mao-client/src/effects.rs:3139`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original places a buff ring through FUN_10105e6d (Gamecode): attach 1000-1018 is the posed bone of that name from the table at 0x102c63a8 (`Bip01 Pelvis_ac`, `Bip01 Spine_ac` ... `Bip01 R Hand_ac`, exact match in RCATMesh_t::GetBoneMatrix), falling back to the head attractor; then `_GfxControlBuffPlaceHolder_t`'s Run (0x100d52c3) lays each step at the locator's origin plus cos/sin times the radius along the locator frame's axes (FUN_1010640a origin, FUN_10106439 / FUN_1010646a the frame's x and y axes, all overridden by fixed axes when the locator is the dynel itself). The bone lookup itself is ported under RND-139 (see there). Moving the rings from the client's per-attach figures (`effects.rs` anchor(), with its left-limb mirror rule tuned against screenshots) onto the posed bone frames changes every buff ring's placement and orientation and can only be judged by eye against the original, which cannot be done tonight; the placeholder's own offset/step fields (0x6c, 0x90, 0x94, 0x98) would also need reading against the template words. Left for a session with screenshots.

#### `RND-023` Buff ring ribbon drawn as sprites, clocks tied to 30 frames per second

- SIMPLIFIED, medium
- Original: GfxVisualCord4 draws a ribbon through the newest 32 links; clocks follow the frame rate
- Client: sprites along each segment (at most 8 per segment); clocks fixed at 30 fps
- A player notices: buff rings look beaded and run at a fixed speed regardless of frame rate
- Where: `crates/mao-client/src/effects.rs:2740`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Read again: `_GfxControlBuffPlaceHolder_t` Run (Gamecode 0x100d52c3) takes `FUN_1013f240` (ftol) whole steps of 0.45 rad a frame and runs its cord child once per step with the frame's own step time, so the cord's clocks (link life, drift held to 0.01..0.025, 0x10161994/0x1016c558) depend on how many frames the machine draws; `GfxVisualCord4` draws a ribbon through the newest 32 links. The client's `ring()` (effects.rs) is a stateless function of the pulse time at a fixed 1/30 s frame. Following the original needs (a) a per-ring state kept across frames and fed the real frame time, and (b) a new ribbon (triangle strip) primitive in the renderer instead of sprites; both change only how the rings look and can only be judged by eye against the original on the same frame rate. Left.

#### `RND-024` Flat grey floor under interiors whose ground is not read

- SIMPLIFIED, medium
- Original: not stated (interior ground is of a tile-map layout the client cannot read)
- Client: draws a 200-unit flat grey square at a given height as the floor
- A player notices: interiors without a readable floor show a plain slab
- Where: `crates/mao-client/src/net.rs:3947`; `crates/mao-client/src/net.rs:7166`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The slab (net.rs floor_part, laid only when a playfield has neither a readable tile-map ground nor readable rooms) stands in for a floor the original reads from the old CHGA tile-map layout of rooms, which this client cannot parse (catalogued separately as RND-166). The original has no stand-in floor of its own, so the only faithful change is reading that layout (RND-166, a format port with no reader in the drop-ins to copy from); taking the slab away alone would leave the player standing over nothing, which the original never shows either. Left with RND-166.

#### `RND-025` Flat water sheet where a playfield has no water triangles

- SIMPLIFIED, medium
- Original: the playfield's own water polygons (liquid surfaces from triangles)
- Client: a playfield without polygons gets one flat translucent guessed sheet at a detected seabed's level plus 1.5
- A player notices: a plain sheet rather than liquid passes; level may be wrong
- Where: `crates/mao-render/src/lib.rs:2233`; `crates/mao-formats/src/terrain.rs:163`; `docs/CLIENT_UI.md:70`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The original's liquid is only the playfield's own water triangles (the Anarchy part of its definition object); a playfield without them has none. Terrain::water (mao-formats terrain.rs) no longer guesses a sheet over a big flat low floor; a probe over every playfield showed the guess fell on Omni-1 Trade 710 (at 6.5), Garden_of_unredeemed 4011, Pandemonium Tower 4900, Uturn Canyon/Forest 6550/6551 and Foundry of Nightmares 8040-8050, all of which have no water triangles; every other playfield the guess touched has triangles, which already replaced the sheet. MAO_WATER=level still lays a sheet for tests. Test: terrain::tests::seabed_ground_has_no_guessed_water.

#### `RND-026` Map: ground picture shrunk and averaged

- SIMPLIFIED, medium
- Original: PFMapGroundRenderer_c draws each ground patch's own hi texture (CreateHiTexture) at full resolution
- Client: each tile's texture averaged down to 8, 4, 2 or 1 pixels a side so the ground fits 2048 pixels across; a tile with no texture is flat grey (70, 70, 70)
- A player notices: the map's ground looks blurrier than the original's
- Where: `crates/mao-client/src/automap.rs:74`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's PFMapGroundRenderer_c draws each ground patch with its own hi texture (CreateHiTexture) at the texture's resolution, patch by patch as the map is drawn. The client builds one picture of the whole ground (automap.rs ground_picture) that must fit one texture, so it averages each tile to 8/4/2/1 pixels. Following the original means drawing the map ground as many textured patches (a new map renderer path with per-patch textures and their own memory budget), not a rule that can be checked as a function; the result is only judged by looking at the map. Left.

#### `RND-027` Name tags hidden by a line-of-sight test through the surfaces, a fourth of them per frame

- SIMPLIFIED, medium
- Original: name tags are VisualSprite_t placed in the world (HandleNametags 0x100268ee, FUN_100258b3), so walls hide them naturally
- Client: tags drawn over the picture; each tag in view tested against the zones' collision surfaces every 4th frame, the answer kept in between; off-screen tags not tested
- A player notices: a name can show or vanish up to three frames late behind a wall; non-surface props may not hide it
- Where: `crates/mao-client/src/nametag.rs:55`; `docs/CLIENT_UI.md:3207`; `docs/CLIENT_UI.md:1334`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): In the original the name tags are VisualSprite_t placed in the world (HandleNametags 0x100268ee, FUN_100258b3) and the depth buffer hides them behind anything drawn, walls and props alike. The client's tags are its own egui plates drawn over the picture (catalogued as the addition RND-186), so the only way to get the original's hiding is to draw the tags as world sprites in the renderer's depth-tested pass, which is the RND-186 question (the tags' look) and a renderer change judged only by eye. Left with RND-186.

#### `RND-028` Player body and head mesh names listed for seven breed/sex pairs only

- SIMPLIFIED, medium
- Original: the player-visual table at Gamecode RVA 0x57a2e
- Client: only seven pairs listed, head number 01; an Atrox only as sex 1; any other pair has no mesh
- A player notices: other combinations are not dressed
- Where: `crates/mao-formats/src/appearance.rs:25`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The player-visual table (Gamecode FUN_10057a2e) was read again: the client's seven pairs are exactly the original's, plus breed 5 sex 1, which the original dresses with `unfinished-breed.cir` and head mesh 0 (no head); every other pair gets nothing (the function returns 0 and sets neither stat 12 nor 64). player_meshes now returns the head as optional and has the breed-5 pair; the body cache (net.rs ensure_body) loads a headless body for it. Test: appearance::tests::player_visual_table.

#### `RND-029` Precipitation streak counts and sizes are the client's own

- SIMPLIFIED, medium
- Original: VisualRain_t streak counts and sizes
- Client: rain and snow as particles with the client's own numbers
- A player notices: rain and snow differ in density and size
- Where: `crates/mao-client/src/precipitation.rs:7`; `docs/ORIGINAL_AUDIT.md:179`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): The counts and movement are the original's since the VisualRain_t port (see RND-138); the sheets' width is the client's own (about a picture element).

#### `RND-030` Animation cross-fade uses a normalised weighted sum

- SIMPLIFIED, low
- Original: CATAnimBlend_t blends by the weight in its chain (FUN_10072562; rotations slerped per its comment)
- Client: rotations mixed by weighted sum (the shorter way) and renormalised, translations lerped; not a true slerp
- A player notices: slightly different turn speed mid-fade between animations
- Where: `crates/mao-formats/src/catmesh.rs:326`; `crates/mao-formats/src/animstack.rs:421`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): CATAnimBlend_t's mode 3 slerps a track's two rotations by the blend's weight (FUN_10050586 with FUN_10053d0b: the shorter way, linearly when within 1e-6) and lerps the translations (FUN_1005063c); the weight comes from FUN_10072562. The animation stack's `mix` (animstack.rs) and CatMesh::pose_blend (catmesh.rs) used a renormalised weighted sum; both now call catanim::slerp, the existing port of FUN_10053d0b. Test: animstack::tests::blends_slerp_the_rotation (a quarter blend to a half turn is 45 degrees; the old sum gave 36.9).

#### `RND-031` Attractor looked up by part of its name

- SIMPLIFIED, low
- Original: RCATMesh_t::GetAttractor compares the whole name (FUN_10054df1)
- Client: takes the first whose lower-case name contains the given text
- A player notices: wrong attachment point possible when two names share a text
- Where: `crates/mao-formats/src/catmesh.rs:444`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): RCATMesh_t::GetAttractor compares the whole name (FUN_10054df1, drop-in catrender.rs attractor_matrix), the head is asked for as `Attractor01_head`, and a worn mesh's place names its attractor from AttractorMesh::GetName's table (DisplaySystem 0x100af7f0, drop-in attractor.rs NAMES: Attractor01_head, 02_righthand, 03_lefthand, 04_rightshoulder, 05_leftshoulder, 06_back, 07_hip, 08_rightthigh, 09_leftthigh, 10_rightcrus, 11_leftcrus, 12_rightarm, 13_leftarm, 14_rightforearm, 15_leftforearm). The client took the first attractor whose lower-case name contained "head" or "AttractorNN"; net.rs (PLACE_ATTRACTORS, attractor_name, the three head lookups) and anim.rs now use the whole names through CatMesh::attractor_named, and the substring CatMesh::attractor is removed. A place past the table finds nothing (the original indexes it unchecked). Test: net::door_tests::worn_places_name_whole_attractors.

#### `RND-032` Billboard particles never spin (suns, smoke)

- SIMPLIFIED, low
- Original: the suns' quads (FUN_100fc425) and the smoke's Type0 quads turn
- Client: the client's billboards do not turn
- A player notices: suns and smoke puffs do not rotate
- Where: `crates/mao-client/src/effects.rs:642`; `crates/mao-client/src/effects.rs:2190`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The client's particles have no rotation field: billboards are drawn by the renderer's sprite pass with fixed corners (mao-render Particle has position, size, colour, sprite rect and world axes only). Spinning the suns' quads (FUN_100fc425) and the smoke's Type0 quads needs a rotation per particle in the renderer's vertex layout and shader, plus each effect's spin words; the result is only seen, not testable as a rule. Left.

#### `RND-033` Blended parts ordered by own distance; per-visual sort, frame fade and environment-mapped pass missing

- SIMPLIFIED, low
- Original: the transparent list (6) is drawn after the others, far to near, in RVisual_t::AddToRenderList's distance buckets; meshes of one visual sorted far to near where it asks (`sorted`); the frame's transparency fade; an environment-mapped pass; run-time alpha modes (FUN_10040645)
- Client: blended parts sorted far to near by each part's own centre distance; the per-visual sort, the frame fade, the environment-mapped pass and the run-time alpha modes not done
- A player notices: transparent things can overlap in a different order or look wrong in rare cases
- Where: `crates/mao-render/src/lib.rs:2376`; `docs/CLIENT_UI.md:689`; `docs/ORIGINAL_AUDIT.md:22`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's transparent ordering (RVisual_t::AddToRenderList's distance buckets, the per-visual `sorted` flag, the frame's fade, the environment-mapped pass and the run-time alpha modes FUN_10040645) is a renderer pipeline port across randy31 and DisplaySystem; the client's per-part far-to-near sort differs only in rare overlaps, judged by eye. Left.

#### `RND-034` BParticle2 integrated in steps with hashed rebirth

- SIMPLIFIED, low
- Original: each frame velocity pulled by gravity and damped, particle reborn in mode 1
- Client: 60 steps/s (max 240), at most 400 particles, rebirth as a hashed chain of lives
- A player notices: particle paths and rebirth rhythm approximate
- Where: `crates/mao-client/src/effects.rs:2440`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original integrates each BParticle2 once per drawn frame (gravity pull and damping) and rebirths by rand() in mode 1, so its paths depend on the frame rate and the C library's random sequence; the client's stateless 60-steps-a-second integration with hashed rebirth gives the same law at a fixed step. Following it exactly needs per-effect state across frames and the original's rand() stream (RND-063), and is only judged by eye. Left.

#### `RND-035` City map door and guard dots use the whole picture

- SIMPLIFIED, low
- Original: the marker atlas copies only the top-left 4 x 4 texels of each GUI picture, so a marker is a 4 x 4 dot
- Client: draws the whole skin picture squeezed into 4 x 4, or a plain blue or red square
- A player notices: dots may look slightly different
- Where: `crates/mao-client/src/cityterminal.rs:387`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The city map's markers come from an atlas (GUI U:0x10046d0b, docs/CITY_MAP_ORIGINAL.md) that copies only the top-left 4 x 4 texels of each GUI picture into its cell, so a door or guard dot is those 4 x 4 texels. The client drew the whole GFX_GUI_MAPDOT_BLUE/_RED picture squeezed into 4 x 4 pixels; it now draws just its top-left 4 x 4 texels (cityterminal.rs show_map); the plain blue/red squares stay for a skin without the pictures. No unit test (drawing); built.

#### `RND-036` Client's own light stands in without the scripts' light

- SIMPLIFIED, low
- Original: the world is lit by the FXS scripts' ambient and ground light
- Client: the client's own sky and sun colours, scaled by the day's intensity, stand in only without the scripts' lighting
- A player notices: lighting differs where scripts give none
- Where: `crates/mao-render/src/lib.rs:505`; `docs/ORIGINAL_AUDIT.md:161`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Same question as RND-001: the stand-in light is used only where no script gives one, and what the original shows there needs a comparison against original screenshots, not a reading. Left with RND-001.

#### `RND-037` Collision from visual meshes where no collision surfaces exist

- SIMPLIFIED, low
- Original: collision from the zones' surfaces (0xf424d, n3Zone_t::LoadSurface 0x1001a947)
- Client: for playfields with none (generated dungeons) uses the props' opaque triangles; see-through parts left out
- A player notices: blocking can differ in such playfields
- Where: `crates/mao-client/src/net.rs:5681`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Playfields without 0xf424d surfaces are the generated dungeons, whose collision in the original comes from the room pieces through the rooms' own data (the N3 room code), not read tonight; the client's props-triangle stand-in is the only collision it has there, and replacing it needs the rooms' collision format and a live walk to check. Left.

#### `RND-038` Collision surfaces keep only triangles

- SIMPLIFIED, low
- Original: n3SurfaceResource_t keeps a KD tree whose nodes sort the volumes for its queries
- Client: only the triangles are kept
- A player notices: no visible change; different cost and edge cases
- Where: `crates/mao-formats/src/surface.rs:10`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): No visible change: the original's KD tree (n3SurfaceResource_t) only orders the volumes for its queries; the triangles and the answers are the same. Porting the tree changes cost, not behaviour. Left.

#### `RND-039` Cords without a shot are a short cord upward

- SIMPLIFIED, low
- Original: cords span a line between two ends
- Client: alone, a short cord up from the point (one metre)
- A player notices: stand-in shape
- Where: `crates/mao-client/src/effects.rs:3026`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): A cord spans two ends in the original; a cord drawn with no shot has no second end in the client (it comes from statel or aura places with only one point), so there is no original rule for this case to port; the stand-in shape stays until the place that starts such a cord gives the second end. Left.

#### `RND-040` Crazy cone tracks in closed form, texture offsets not drawn

- SIMPLIFIED, low
- Original: FUN_100d6dcc moves values by track speed and acceleration frame by frame
- Client: closed form v t + a t^2/2; texture offsets not drawn
- A player notices: cone motion slightly off; texture does not scroll
- Where: `crates/mao-client/src/effects.rs:1739`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_100d6dcc steps the cone's tracks by speed and acceleration once per drawn frame, so the original's motion depends on the frame rate; the closed form is that motion at an infinitely small step. Following it needs per-effect state fed the real frame time, and the texture offsets need a texture-scroll input in the renderer's shell pass; both only judged by eye. Left.

#### `RND-041` Default water light and ambient without the effect scripts

- SIMPLIFIED, low
- Original: the scripts' WaterLightCurrent and WaterLightDirection light the liquid
- Client: without them the water is lit by the sun as bright as 0.2 + 0.8 of the day, with half of that as ambient
- A player notices: liquid brightness may differ where scripts give no water light
- Where: `crates/mao-render/src/lib.rs:2319`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Same as RND-001: the default water light only stands in where the scripts give no WaterLightCurrent/Direction; what the original's liquid looks like there needs an original screenshot. Left.

#### `RND-042` Effect scripts' eye is the character, with a fixed camera rotation

- SIMPLIFIED, low
- Original: the frame block's eye is the camera (FUN_100b05b4); GAME variables hold storm, precipitation, wind, suns and the camera
- Client: the character's place stands in as the eye with a fixed rotation [0, 1, 0, 0] instead of the camera's
- A player notices: sky and cloud effects ignore where the camera is turned
- Where: `crates/mao-client/src/net.rs:9015`; `docs/ORIGINAL_AUDIT.md:93`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The scripts run on the network thread (net.rs, crate::fxs::set_game), which has the character's place but not the camera: the camera's eye and turn live in the viewer. Feeding the frame block the camera (FUN_100b05b4) means sharing the viewer's camera with the script thread each frame (a new shared slot) and checking that the sky and clouds then follow the camera's turn, which can only be seen. Left.

#### `RND-043` Effect spell 0xcfd4 draws the effect only

- SIMPLIFIED, low
- Original: FUN_100a9159 makes the effect with repetitions (stat 0xa1), start and stop colours (stats 0x99-0xa0 over 255) and a scale (stat 0x32 / 100 + 1)
- Client: draws the effect only, no colours, repetitions or scale
- A player notices: such effects look plainer
- Where: `crates/mao-client/src/spelllist.rs:23`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_100a9159 starts the effect with repetitions (stat 0xa1), start and stop colours (stats 0x99-0xa0 over 255) and a scale (stat 0x32 / 100 + 1); the client's effect path has no colour or scale inputs for a running effect (pulses and visuals are drawn as their templates say), so this needs a tint and scale carried through effects.rs to every visual kind. Over the time box for a low entry. Left.

#### `RND-044` Eruption rocks: one mesh kind, unlit, no scene collision

- SIMPLIFIED, low
- Original: VisualTinyRock_t draws a lit triangle mesh; other material kinds make rocks; collision with statics; 50-controls limit (DAT_102ea974)
- Client: only kind 4 rock01.abiff; terrain height only; white unlit shells
- A player notices: eruption rocks look flat and differ
- Where: `crates/mao-client/src/effects.rs:1168`; `docs/ORIGINAL_AUDIT.md:299`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): VisualTinyRock_t's lit triangle meshes, the other material kinds and the collision with statics need lit effect meshes (RND-154) and the statics' collision in the effect code; judged by eye. Left.

#### `RND-045` Fire and global smoke seen without a frame use heading 0

- SIMPLIFIED, low
- Original: flames placed in the dynel's frame; smoke frame turned by the locator
- Client: heading/facing taken as 0
- A player notices: flames and smoke sit in the world's frame, not the caster's
- Where: `crates/mao-client/src/effects.rs:3041`; `crates/mao-client/src/effects.rs:3091`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The fires and smokes seen without a dynel are the statels' and the environment's; their locator in the original is the connector's frame on the statel's mesh, and the client's statel effect list (main.rs statel_effects) keeps only the point, not the frame. Carrying the connector's turn needs that list widened and the fire/smoke code fed a heading; small but only checkable by eye. Left.

#### `RND-046` Fixed sun direction when no script gives one

- SIMPLIFIED, low
- Original: the way to the sun comes from the effect scripts
- Client: a fixed direction (0.4, -0.5, 0.8) is the light and shadow direction without scripts
- A player notices: light and shadows do not move with the hour unless scripts supply a sun
- Where: `crates/mao-render/src/lib.rs:771`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As RND-001: the fixed direction stands in only where no script gives a sun; the original's light there has to be compared on screenshots. Left.

#### `RND-047` Flare streak drawn as three sprites

- SIMPLIFIED, low
- Original: GfxVisualFlareType0 draws a streak from head to tail
- Client: three sprites along it; at most 400 particles
- A player notices: streaks look beaded/shorter
- Where: `crates/mao-client/src/effects.rs:573`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): GfxVisualFlareType0 draws one streak quad from head to tail; the client has no streak (stretched quad) primitive for particles, only billboards and world quads with fixed axes. A world quad along the streak could stand in but its facing to the camera is the renderer's to do; judged by eye. Left.

#### `RND-048` Flying craft plays the walk animation

- SIMPLIFIED, low
- Original: not stated
- Client: the Yalmaha craft (template 99470, scale 20) has no animation for the fly action (179), so the walk is played
- A player notices: craft shows a walk loop while flying
- Where: `crates/mao-client/src/net.rs:12378`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): It is the original's rule: Gamecode FUN_10010b3c, the action fallback table, sends 0xb3 (hover-norm, the fly action) and 0x66 (sneakcool) to 100, the walk; a craft template with no animation for 0xb3 therefore plays its walk in the original too. The client follows that chain (appearance.rs action_fallback, npc.rs animation_candidates). Nothing to change; the catalog line should say so.

#### `RND-049` Fogged sky backdrops faded with a reconstruction

- SIMPLIFIED, low
- Original: a draw whose blob turns fog on fades into the fog by distance from the eye (a fixed-function fog); whether by distance or depth is unknown
- Client: vertex colours multiplied by what the fog leaves and an additive fog-colour layer laid over, for opaque or black-fog backdrops only; fog not done on blended sky draws whose fog is not black
- A player notices: the sky dome's fade toward the horizon may differ (Shadowlands)
- Where: `crates/mao-client/src/viewer.rs:3333`; `docs/CLIENT_UI.md:1210`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original fades a fogged sky draw with Direct3D's fixed-function fog, and whether that fog is by distance or by depth (range fog or not) is not known from the reading; the client's reconstruction can only be checked against original screenshots of the Shadowlands sky. Left.

#### `RND-050` FXS [LENGTH] double operand and unscannable literal give 0

- SIMPLIFIED, low
- Original: a double operand's multiplier is the entry's address; an unscannable unsigned literal is the address of its text (FUN_1000ca20)
- Client: both give 0
- A player notices: malformed or quirky script values read as 0
- Where: `crates/mao-client/src/fxs/mod.rs:1980`; `crates/mao-client/src/fxs/parser.rs:510`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's values here are memory addresses (a double operand's multiplier is the formula entry's address; an unscannable unsigned literal is the address of its text, FUN_1000ca20): numbers that differ every run and that no script can rely on. There is nothing to reproduce; 0 stays. Left.

#### `RND-051` FXS [STORE] into an array does nothing

- SIMPLIFIED, low
- Original: writes over the array's element list
- Client: skips arrays
- A player notices: scripts that store into an array leave it unchanged
- Where: `crates/mao-client/src/fxs/mod.rs:1460`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FXS.dll's [STORE] (FUN_10005600, drop-in fxs/src/eval.rs) copies the value's bytes to the variable's value block +8 whenever the variable's kind matches, without looking at whether it is an array; for an array that block holds the element list's own pointers, so the original overwrites its list with the value's bits (a corrupt list the next read follows). Rust cannot write over the list's pointers; the client's skip is the only safe reading. Left.

#### `RND-052` Global smoke births modelled at 60 per second

- SIMPLIFIED, low
- Original: frame-by-frame births
- Client: fixed 60 a second slots
- A player notices: birth rhythm differs with frame rate
- Where: `crates/mao-client/src/effects.rs:2269`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's global smoke births run once per drawn frame, so their rhythm follows the frame rate; reproducing it needs per-effect state fed the real frame time, and the result is only seen. Left.

#### `RND-053` Helmet face takes the head's whole material

- SIMPLIFIED, low
- Original: puts only the head's texture on the face's own material (FUN_1006d3e2)
- Client: replaces the whole face material with the head's, because the face's own colour is the modeller's plain blue
- A player notices: a helmet's face colour may differ
- Where: `crates/mao-client/src/net.rs:4111`; `docs/CLIENT_UI.md:4645`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_1006d3e2 puts only the head's texture on the face's own material; the face material is the modeller's plain blue, so how the original's face looks depends on how its D3D7 stage modulates that colour with the texture (the client found it a blue patch when it followed the reading). Needs an original screenshot of a helmeted face to settle. Left.

#### `RND-054` Lightning has no limit on effects at once

- SIMPLIFIED, low
- Original: effects come from pools that can refuse a new one when full
- Client: each strike gets its own effects, none refused
- A player notices: in a storm the client never drops a strike the original might
- Where: `crates/mao-client/src/lightning.rs:24`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The pools' sizes (the 50-control limit DAT_102ea974 and the effect handler's own pools) were not read through, and a storm rarely fills them; porting the refusal needs the pool counts of every effect kind a strike starts. Low value against the time box. Left.

#### `RND-055` Lightning rock bounce normal estimated from heights

- SIMPLIFIED, low
- Original: the ground probe FUN_100ada15 gives the normal
- Client: estimates it from four heights half a unit either side
- A player notices: rocks bounce slightly differently
- Where: `crates/mao-client/src/lightning.rs:103`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The probe FUN_100ada15 asks the playfield's collision object (its +0x60 member's virtual calls) and the previous liquid test, so its normal is the collision surface's (statels included), not the tile-map's; the client's lightning code has only a height function. Porting needs the collision surfaces' normal query handed to the lightning code and is only seen in a rock's bounce. Left.

#### `RND-056` Liquid tests ignore the zones' lists

- SIMPLIFIED, low
- Original: a zone's list of water infos is tested in order and stops at its first hit
- Client: every info of the playfield is tested in the order made
- A player notices: in overlapping liquids a different surface may be found
- Where: `crates/mao-client/src/liquid.rs:13`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original tests a zone's own list of water infos (n3Zone_t's liquid data, per zone) in order; the client builds one list for the playfield from the definition's water triangles and has no per-zone split of them (the zones' liquid lists are made when N3 loads each zone). Splitting needs the zone each triangle belongs to, which was not read. Left.

#### `RND-057` Liquid types 0x10 and 0x11 not drawn; wave offsets and lava second pass left out

- SIMPLIFIED, low
- Original: frame block's wave offsets (set to 0 by scripts) and lava second rippling pass (alpha 1 contributes nothing)
- Client: left out on purpose; types 0x10 and 0x11 not drawn; wide-layer refraction copy left out
- A player notices: none visible by the notes
- Where: `crates/mao-formats/src/liquid.rs:39`; `docs/CLIENT_UI.md:2180`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes say nothing visible: the wave offsets are set to 0 by the scripts and the lava's second pass has alpha 1 (contributes nothing); types 0x10 and 0x11 do not occur in the playfields' triangles the client reads. Nothing a player would see to port. Left.

#### `RND-058` Map ground dimmed by a plain veil, no shadow textures

- SIMPLIFIED, low
- Original: the ground is multiplied over each patch's shadow texture lightened by a white 0xbb veil (shadow textures from six stored maps, type 0xf4247)
- Client: the shadow maps are not in today's game files, so the ground is drawn at 0xbb of its brightness over black ('taking the background for black (not checked)')
- A player notices: the map ground has no shadow shading; brightness may differ
- Where: `crates/mao-client/src/automap.rs:421`; `docs/CLIENT_UI.md:593`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The shadow textures (type 0xf4247) are not in this installation's database, so the original's multiply over them cannot be done; the plain veil at 0xbb is the reading without them. Left until the textures are found.

#### `RND-059` Material render states read only in part

- SIMPLIFIED, low
- Original: DirectX materials and texture stage states apply their full effect
- Client: only alpha blend (add or alpha), z write off, alpha test with a reference, two-sided and the "alpha as light" setup are read; other states of a delta state are ignored; vertex colours are skipped
- A player notices: materials with other states draw plainly
- Where: `crates/mao-formats/src/model.rs:346`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The remaining render states and texture stage states are a renderer port (randy31's state blocks onto wgpu pipelines), many combinations, judged only by eye. Left.

#### `RND-060` Mesh vertex formats: only the first texture-coordinate pair kept

- SIMPLIFIED, low
- Original: not stated
- Client: only the first texture coordinate pair of a vertex is kept
- A player notices: multi-texture materials use one set of coordinates
- Where: `crates/mao-formats/src/model.rs:411`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Keeping the second texture-coordinate pair only matters with a second texture stage, which the client's renderer does not draw (RND-059); reading the pair alone changes nothing on screen. Left with RND-059.

#### `RND-061` Nano effect: direction not turned by the locator (also pool size, sprite spin)

- SIMPLIFIED, low
- Original: turns the sprite direction by the locator's orientation; also a rand() gate, the pool's size and sprite spin
- Client: uses the world's frame; spin not done
- A player notices: nano sprites fly in world directions, not relative to the caster/locator
- Where: `crates/mao-client/src/effects.rs:160`; `docs/ORIGINAL_AUDIT.md:294`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Turning the nano effect's directions by the locator needs the locator's frame passed into the particle code (pulse_particles has the feet and heading only), the rand() gate is RND-164, the pool size and the sprite spin need the renderer's spin (RND-032); seen only. Left.

#### `RND-062` Nano effect: wind push in 12 steps, at most 600 sprites

- SIMPLIFIED, low
- Original: push added to position each frame
- Client: summed in 12 steps; at most 600 sprites considered per frame
- A player notices: negligible
- Where: `crates/mao-client/src/effects.rs:183`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes say negligible: the push summed in 12 steps differs from a per-frame sum only by the frame rate, and 600 sprites is above what the templates make. Left.

#### `RND-063` Particle effects use hashed random numbers, not the original's rand()

- SIMPLIFIED, low
- Original: Sparks (FUN_100f1bbe), Nano (FUN_100e6f76), Stars (FUN_100f8491) draw values with rand()
- Client: values come from a hash of the particle's number, so every cast plays the same pattern
- A player notices: particles are identical each time instead of random
- Where: `crates/mao-client/src/effects.rs:114`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's rand() is one CRT stream shared by every effect and everything else in the process (msvcr100's state times 214013 plus 2531011), so a particle's numbers depend on everything that called rand() before it; the client cannot reproduce that sequence, only its law. Hashing per particle gives fixed patterns; a per-cast seed would vary them but would still not be the original's numbers. Left for the user's choice (a seeded stream per cast is a small change if variety is wanted).

#### `RND-064` Parts drawn in a different order

- SIMPLIFIED, low (notes only)
- Original: parts drawn in upload order
- Client: opaque parts sorted and instanced; exactly coplanar surfaces may change winner
- A player notices: z-fighting outcome may change
- Where: `docs/CLIENT_UI.md:1414`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: drawing in upload order instead of the client's sorted, instanced order only changes which of two exactly coplanar surfaces wins; giving up the instancing for that is not worth it. Left.

#### `RND-065` Plasma band: 8 sprites displaced up, no line sampling

- SIMPLIFIED, low
- Original: GfxVisualPlasma (FUN_1001bbf2) draws 75 segments displaced across the view, sampling one line of its texture, with rare random phase nudges
- Client: 8 sprites a tenth apart, displaced up (world z); the line sampling and nudges not done
- A player notices: plasma band and orbs look different
- Where: `crates/mao-client/src/effects.rs:783`; `docs/ORIGINAL_AUDIT.md:272`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): GfxVisualPlasma (FUN_1001bbf2) draws 75 segments displaced across the view, sampling one line of its texture: a strip primitive with a one-texel-line sampling and per-frame random nudges, a renderer feature judged by eye. Left.

#### `RND-066` Playfield-level environment taken as zero when the database has none

- SIMPLIFIED, low
- Original: the constructor makes an empty table when none is given
- Client: the playfield's own table is zero here; the blend starts from zero
- A player notices: none visible if the original does the same
- Where: `crates/mao-formats/src/areas.rs:15`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The note itself says the original makes an empty table when none is given, which is what zero is; nothing differs. Left.

#### `RND-067` Room floor drawn apart, unbaked colour

- SIMPLIFIED, low
- Original: draws the floor from the height map, apart from the room's pieces and not light-mapped
- Client: a quad per cell with an unbaked colour, texture from the cell's first texture layer; wall piece winding flipped for the client's frame
- A player notices: none stated
- Where: `crates/mao-formats/src/rooms.rs:733`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original draws the room floor from the height map without light-mapping, as the client does; the remaining difference (a quad per cell, colour unbaked, the first texture layer) is in RND-117's open question of the texture layer. Nothing further to port without a comparison. Left.

#### `RND-068` Room left stays drawn two seconds

- SIMPLIFIED, low
- Original: hides the room when its locality monitor loses it
- Client: keeps the room just left for two seconds
- A player notices: the old room lingers briefly
- Where: `crates/mao-client/src/interior.rs:12`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original hides a room when its locality monitor loses it; the monitor's rules (which rooms it keeps, from where) were not read tonight, and the client's two seconds stand in for them to hide the gap while the next room's locality comes in. Removing the linger without the monitor's rule would make rooms blink. Left.

#### `RND-069` Room shell corner heights from a smoothing rule

- SIMPLIFIED, low (notes only)
- Original: not stated
- Client: each corner averages neighbouring cells within a 6-step slope; a bigger step or missing neighbour is a wall; verified only against shop booth heights
- A player notices: room floors and ramps approximate the original's geometry
- Where: `docs/CLIENT_UI.md:1705`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the original's rule for the room shell's corner heights was not stated; the client's smoothing was checked against shop booths only. Needs the original's room shell reader to compare. Left.

#### `RND-070` Scripted environment draws honour a subset of render states

- SIMPLIFIED, low
- Original: DisplaySystem applies the state blob's render states, texture stages and multiple textures to D3D
- Client: honours only depth test, alpha/additive blend (SRCBLEND ignored), lighting, ambient, texture factor alpha, MODULATE2X, first-stage texture transform, fog, one (last) texture and material
- A player notices: scripted sky visuals may blend or light differently
- Where: `crates/mao-client/src/fxs/visuals.rs:164`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The remaining state blob render states and multi-texture stages are a renderer port for the scripted sky (SRCBLEND, more texture stages), judged by eye. Left.

#### `RND-071` Shield shells ignore the character's heading

- SIMPLIFIED, low
- Original: model frame (Y up, origin the feet) used for the mapping, point and shimmer (FUN_1001c94f / FUN_1001d569)
- Client: the frame is the world's, unturned by the heading
- A player notices: shield texture and shimmer do not turn with the character
- Where: `crates/mao-client/src/effects.rs:804`; `docs/ORIGINAL_AUDIT.md:275`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The shield shell's mapping, point and shimmer are in the model frame (FUN_1001c94f / FUN_1001d569); the shell code gets the character's surface in world space and no heading (visual_shells takes the surface and translation only). Turning needs the heading carried into shield_shell; seen only in the texture's direction. Left.

#### `RND-072` Shield2 texture mappings 3-5 use a hash

- SIMPLIFIED, low
- Original: random place per vertex overwrites the sphere map
- Client: hash of the vertex index
- A player notices: fixed instead of random texture scatter
- Where: `crates/mao-client/src/effects.rs:895`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The random place per vertex comes from the process-wide rand() (see RND-063); the hash gives a fixed scatter of the same law. Left with RND-063.

#### `RND-073` Shock wave rings follow the feet

- SIMPLIFIED, low
- Original: keeps a ring's centre from its first frames
- Client: ring centre and cones follow the feet
- A player notices: ring moves with a walking caster instead of staying put
- Where: `crates/mao-client/src/effects.rs:1328`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original keeps a ring's centre from its first frames; the client's shock wave is a stateless function of the current feet, so keeping the centre needs per-effect state (the place at the effect's start) kept in the pulse scheduler. Seen only with a walking caster. Left.

#### `RND-074` ShockWave: blending without 0x800 drawn additive; ring centre follows feet

- SIMPLIFIED, low
- Original: one onto the inverse source colour blend without 0x800; ring centre kept from the first frames; Color_t byte arithmetic
- Client: drawn added with alpha forced to 1; centre follows the feet; floats
- A player notices: shock rings look brighter or different
- Where: `crates/mao-client/src/effects.rs:1321`; `docs/ORIGINAL_AUDIT.md:301`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The blend without 0x800 is ONE onto INVSRCCOLOR, which the renderer's shell pass does not offer (it has alpha and additive only); the ring centre is RND-073. A new blend mode in the renderer, judged by eye. Left.

#### `RND-075` Single clouds only the realistic 10 by 10 mesh

- SIMPLIFIED, low
- Original: FUN_10053681 builds clouds at a size the quality settings choose
- Client: always the 10 by 10 grid, 30 puffs
- A player notices: no lower cloud-quality variant
- Where: `crates/mao-client/src/fxs/clouds.rs:6`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_10053681 builds the cloud mesh at a size chosen by the quality settings; the client has no cloud-quality setting and keeps the realistic 10 by 10 mesh, the highest. A lower variant is an option to add, not a rule to port. Left.

#### `RND-076` Skin pictures linearly filtered and stretched

- SIMPLIFIED, low
- Original: not stated
- Client: skin sprites loaded with linear filtering (egui) and stretched by rectangle
- A player notices: scaled pictures look smoothed
- Where: `crates/mao-client/src/skin.rs:113`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Original not stated: the GUI's sprites go through DisplaySystem with its own filtering, not read. Seen only. Left.

#### `RND-077` Skinned normals use the first bone only; unknown bones fall back to rest pose

- SIMPLIFIED, low
- Original: not stated
- Client: the normal is turned by the first bone only; a vertex whose bones are missing is drawn at its rest position
- A player notices: slight shading differences while limbs bend
- Where: `crates/mao-formats/src/catmesh.rs:40`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Original not stated: the original skins on the card through randy31's CAT render, whose normal blend was not read tonight; the client's first-bone normal only changes shading at joints, seen only. Left.

#### `RND-078` Sky flash: always follows the point, torso radius taken as 0.5, texture extents not read as floats

- SIMPLIFIED, low
- Original: _GfxControlSkyFlash_t (FUN_100ef141) uses the torso's radius when word 18 is negative; following the point is optional; cone texture extents read as floats
- Client: takes 0.5; always follows; extents not read as floats
- A player notices: flash width differs on bodies not 0.5 wide; moves with the caster when it would stay
- Where: `crates/mao-client/src/effects.rs:1623`; `crates/mao-client/src/effects.rs:1624`; `docs/ORIGINAL_AUDIT.md:290`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): _GfxControlSkyFlash_t (FUN_100ef141) needs the body's torso radius for a negative word 18 (the client has a torso sphere in hazards.rs but not in the effect's call), the follow flags 0x800/0x1000 need the effect's start place kept (state), and the texture extents read as floats change only the cone's texture repeat. All seen only. Left.

#### `RND-079` Sparks: alpha blend of flag 0x800 not done (client adds)

- SIMPLIFIED, low (notes only)
- Original: alpha-blend for 0x800
- Client: particles add
- A player notices: sparks brighter/different
- Where: `docs/ORIGINAL_AUDIT.md:270`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: flag 0x800's alpha blend for sparks needs a blended particle pass beside the additive one in the renderer; seen only. Left.

#### `RND-080` Sparks: burst frame gate and wind push approximated

- SIMPLIFIED, low
- Original: FUN_100f24c8 gate lets frames through; push of word 38 added to position every frame
- Client: later sparks born at a fixed spacing computed from the gate; push summed over 12 sample steps
- A player notices: spark timing and wind drift slightly different
- Where: `crates/mao-client/src/effects.rs:118`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_100f24c8's frame gate and the per-frame push both follow the drawn frames; the client models them at fixed steps. Per-effect state fed the frame time would be needed; seen only. Left.

#### `RND-081` Spiral (coil) strips drawn as sprites, texture repeat and scroll not done

- SIMPLIFIED, low
- Original: GfxVisualSpiral (FUN_10021924) draws strips with the texture's repeat and scroll along the strip
- Client: sprites along the visible part of each strip; texture repeat and scroll not done
- A player notices: coil is a string of dots instead of a ribbon, texture static
- Where: `crates/mao-client/src/effects.rs:741`; `docs/ORIGINAL_AUDIT.md:271`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): GfxVisualSpiral (FUN_10021924) draws textured strips with repeat and scroll; the client has no strip with scrolling texture coordinates in the particle path (the shells could carry it, but the coil's run would need porting onto them). Seen only. Left.

#### `RND-082` Sprite effect: standing templates all face the camera, billboards square

- SIMPLIFIED, low
- Original: standing templates set in the locator's frame (flag 0x2000 turns to face the camera); billboard has its own height
- Client: all standing quads face the camera; billboards are square (the width); alpha blend 0x1000 not done
- A player notices: standing quads turn with the view; sprites differently shaped
- Where: `crates/mao-client/src/effects.rs:2349`; `docs/ORIGINAL_AUDIT.md:280`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Standing templates in the locator's frame need the locator passed to the sprite effect (as RND-061), and billboards of their own height need a non-square particle in the renderer (Particle has one size); seen only. Left.

#### `RND-083` Stars mode 15: stand-in ring and stars riding the sample

- SIMPLIFIED, low
- Original: FUN_100f740b callback keeps 30 skinned vertices and normals; each star keeps its place at birth
- Client: samples every fourth posed vertex of the blob-shadow caster; a star rides its sample's current place; without a body a ring of eight points round the attach point stands in
- A player notices: stars follow the moving body instead of staying where born; off-body shape differs
- Where: `crates/mao-client/src/effects.rs:278`; `crates/mao-client/src/effects.rs:1984`; `docs/ORIGINAL_AUDIT.md:266`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_100f740b keeps 30 skinned vertices and normals and each star keeps its birth place; the client samples the blob caster's posed points each frame. Keeping birth places needs per-star state across frames; seen only. Left.

#### `RND-084` Stars mode 2 keeps each star's colour mix instead of re-rolling every frame

- SIMPLIFIED, low (notes only)
- Original: re-rolls the mix every frame
- Client: keeps each star's mix
- A player notices: star colours do not flicker
- Where: `docs/ORIGINAL_AUDIT.md:266`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: re-rolling each star's colour mix every frame draws from the process rand() (RND-063); with a hash it would flicker in a fixed pattern. Left with RND-063.

#### `RND-085` Stars mode 22: heading unknown, edges run across x

- SIMPLIFIED, low
- Original: edges between the character's attach points in its pose
- Client: heading not known; edges run across x from the feet plus fixed anchors; spring in closed form
- A player notices: edge stars not shaped to the body's facing or pose
- Where: `crates/mao-client/src/effects.rs:2082`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Mode 22's edges run between the character's attach points in its pose; the client's stars code is given the feet only (no posed attach points), so the edges need the body's attractor/bone frames passed in (now kept for the bones, RND-139) and the spring stepped per frame. Seen only. Left.

#### `RND-086` Stars mode 3 swirl gain approximated

- SIMPLIFIED, low
- Original: swirl gains a tenth a frame
- Client: closed form 1 + 0.1*age*30*0.1
- A player notices: minor
- Where: `crates/mao-client/src/effects.rs:385`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The swirl gains a tenth a drawn frame in the original, so its growth follows the frame rate; the closed form is that at 30 frames a second. Left (frame-rate dependent, seen only).

#### `RND-087` Stars: births and spring simulated at a fixed 30 fps

- SIMPLIFIED, low
- Original: slot of 128 reborn per frame while free; damped spring runs per frame for the star's life
- Client: closed-form stagger/period at an assumed 30 frames a second; spring simulated at 30 fps, at most 60 steps
- A player notices: star birth rhythm differs at other frame rates
- Where: `crates/mao-client/src/effects.rs:319`; `crates/mao-client/src/effects.rs:511`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's births and spring run once per drawn frame; the client's fixed 30 fps simulation is the same law at a set rate. Per-effect state fed the frame time would follow the original's dependence on the frame rate; seen only. Left.

#### `RND-088` Statel level of detail decided every frame, radius from world box

- SIMPLIFIED, low (notes only)
- Original: LODManager::Process looks at 20 meshes a frame (switches lag a little); the bounding volume radius is the original's
- Client: every mesh decided every frame; radius is the mesh's world box half diagonal
- A player notices: LOD switching distances slightly differ
- Where: `docs/ORIGINAL_AUDIT.md:82`; `docs/CLIENT_UI.md:2261`; `crates/mao-formats/src/model.rs:209`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only (see RND-005): the original looks at 20 meshes a frame and uses the bounding volume's radius; the client decides every mesh every frame from the world box. The 20-a-frame lag is a cost-saving of the original that only delays switches by a few frames. Left.

#### `RND-089` Statels in a zone without a room are skipped

- SIMPLIFIED, low
- Original: not stated
- Client: a statel zone with no matching room is not drawn
- A player notices: those props missing
- Where: `crates/mao-client/src/main.rs:831`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The original does the same: a zone's statels are made only when n3StatelController_t::EnableRoomStatels (N3.dll 0x10025ae8) is called with that zone's number, which the rooms' code does for a room that comes into view (the calls at n3.c 25846, 25958, 26022, each with a room index); a zone whose number has no room is never enabled, so its statels are never drawn. The client's skip of a zone without a room (main.rs placed_statels) matches. Nothing to change.

#### `RND-090` Things with no animation message stand in their first pose

- SIMPLIFIED, low
- Original: an AnimPlay/AnimPos told to the thing plays its model
- Client: without them a thing stands in its first pose
- A player notices: none stated
- Where: `crates/mao-client/src/net.rs:5451`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original plays a thing's model when an AnimPlay/AnimPos is told to it; what its VisualMesh shows before any such message (its first frame, or its own animation on the clock as LODManager does for statel meshes) was not found for dynels. Needs the dynel visual's start read. Left.

#### `RND-091` Toggle effect: playfield test replaced by flag 0x800

- SIMPLIFIED, low
- Original: toggle (kind 3036, FUN_10112952) tests the current playfield id/flags against lists; restarts under 0x400 without 0x2000; delayed children
- Client: passes only with flag 0x800 (the client knows no playfield flags or ids here)
- A player notices: toggled effects may show in the wrong playfields
- Where: `crates/mao-client/src/effects.rs:2775`; `docs/ORIGINAL_AUDIT.md:296`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): The playfield test is ported: GfxControlToggle_t's run (Gamecode FUN_10112952) starts its child only when FUN_101125ed passes: the current playfield record's first flag word (RDBPlayfield_t +0x50, playfield::flag_words()[0]) must meet the toggle's mask (any playfield when the mask is 0), then with flag 0x800 the playfield's number (+0x1c) must not be in the toggle's list, without it it must be. New effects::toggle_passes; resolve_toggles takes the current playfield (number, flag word), which the viewer reads once per playfield. Before, only toggles with 0x800 ever passed, so effects listed for given playfields never showed there. Left: the restart under 0x400 without 0x2000 and the delayed children. Test: effects::tests::a_toggle_starts_its_child_as_the_vehicle_moves_off (extended).

#### `RND-092` Tracer4 cords: hashed flips and jitter, sprite ribbon, no locator travel

- SIMPLIFIED, low
- Original: _GfxControlTracer4_t winds three cords (FUN_100ffa7b); the step's sign flips at random a quarter of the frames and links jitter randomly; a cord ribbon; links ride a locator moving at 100 a second
- Client: flips and jitter decided by a hash of the link; fixed 20 links, each a sprite; links span the whole line for the flight
- A player notices: tracers look and wobble differently
- Where: `crates/mao-client/src/effects.rs:47`; `docs/ORIGINAL_AUDIT.md:277`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): _GfxControlTracer4_t (FUN_100ffa7b) flips and jitters with the process rand() (RND-063), draws a cord ribbon (no ribbon primitive, RND-023) and rides a locator moving at 100 a second; all three need renderer or state work and are seen only. Left.

#### `RND-093` Trail sampling fixed at 30 a second

- SIMPLIFIED, low
- Original: a sample every frame (trails as long as its frames are many)
- Client: a fixed 30 a second; ring's first copies and locator turn not done
- A player notices: hover exhaust trail length differs with frame rate
- Where: `crates/mao-client/src/effects.rs:1218`; `docs/ORIGINAL_AUDIT.md:300`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): A sample every drawn frame makes the original's trails as long as its frames are many; the client's 30 a second is that at 30 fps. Following the frame rate is a one-line change of TRAIL_FRAME to the real frame step but makes trail length depend on the machine, as the original's does; the ring's first copies and the locator turn are not read. Left for the user's choice.

#### `RND-094` Universal effects of meshes ignore the connector's place and turn

- SIMPLIFIED, low
- Original: the effect stays on its connector on the mesh (its own place and turn, object's rotation, parts that need a body)
- Client: the effect goes with the object's position only; the connector's own place on the mesh is not taken; fill square (Tweak_ProgressiveLava) not drawn
- A player notices: effects on larger meshes sit at the mesh's origin, not the connector
- Where: `crates/mao-client/src/net.rs:9038`; `docs/CLIENT_UI.md:1223`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The universal effects of a mesh stay on their connector (its place and turn on the mesh); the client's list of these effects keeps the object's position only (net.rs), as for statels (RND-045). Carrying the connector's frame needs the mesh's connectors read for each such object and the effect placed in that frame; seen only. Left.

#### `RND-095` View drawing goes through egui

- SIMPLIFIED, low
- Original: ViewSurface_c sprites drawn through DisplaySystem
- Client: drawn through egui's painter; one unit is one egui point
- A player notices: none visible beyond small rendering differences
- Where: `crates/mao-client/src/gui/host.rs:6`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The views are drawn through egui by design of this client (one unit one point); drawing them through the renderer as DisplaySystem sprites would replace the whole GUI drawing path. Not a rule to port. Left.

#### `RND-096` VolGrid: slices are two flat-coloured quads

- SIMPLIFIED, low
- Original: GfxVisualVolGrid (FUN_1002f86d) slices have per-vertex colour fans (a gradient within a slice); the locator's rotation, alpha blend and the 0x400 turned texture
- Client: each standing slice is two single-colour quads at a quarter and three quarters; blend is added; random initial turn by salt
- A player notices: volume effects flatter, coarser gradients
- Where: `crates/mao-client/src/effects.rs:2141`; `docs/ORIGINAL_AUDIT.md:278`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): GfxVisualVolGrid (FUN_1002f86d) slices are per-vertex colour fans with the locator's rotation, alpha blend and a turned texture; the client's shells could carry vertex colours but the slice geometry, the blend and the locator are a visual port judged by eye. Left.

#### `RND-097` Weather fade rolls the old state at full strength

- SIMPLIFIED, low
- Original: Environment_t keeps a current and next table and lerps by the fade (FUN_100bd555 / FUN_100bf96e)
- Client: the new state is blended with the previous state at full strength, weighted by one minus the fade
- A player notices: the fade between weather states may differ in detail
- Where: `crates/mao-formats/src/weather.rs:287`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's Environment_t keeps a current and a next 22-float table (FUN_100bd555) and lerps them by the fade (FUN_100bf96e: the first 18 floats by the weight, 0x12-0x14 by the weights' share of +0x15). Those tables are built by FUN_100bcec6 from the area's environment, a different shape from the client's 14 rolled weather values (weather.rs roll); porting the fade means porting the table building too. Over the time box. Left.

### Rendering and world: the original was not read: inferred or measured (26)

#### `RND-098` Atlas of Rubi-Ka map coordinates measured

- GUESS, medium (notes only)
- Original: not stated (the Atlas' own index and coordinates file not at hand)
- Client: rectangles and coordinates measured by cross-correlation against Saavick's map, about a pixel (5 m) off in most playfields; alien sectors and the Unicorn outpost have no coordinates, so no arrow
- A player notices: arrow position on the Atlas map approximate or absent
- Where: `docs/CLIENT_UI.md:3177`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only. The Atlas of Rubi-Ka is a player-made map, not part of the original client's program: there is no rule in Gamecode/GUI/DisplaySystem to port, and the Atlas' own index and coordinates file is not at hand. Measuring better coordinates needs the Atlas' source files or screenshots, nothing that can be read from the original's code.

#### `RND-099` Shadowlands sky still wrong, cause not found

- GUESS, medium (notes only)
- Original: not stated
- Client: nothing drawn off the game showed a difference; what is wrong is still to be asked
- A player notices: unresolved Shadowlands sky issue
- Where: `docs/LIVE_TEST_LIST.md:674`; `docs/CLIENT_UI.md:1307`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: no rule named. The cause of the Shadowlands sky difference was never found; finding it needs the original's sky and the client's side by side on the same Shadowlands playfield at the same game hour (a live comparison), which cannot be done tonight.

#### `RND-100` Cargo box doors never open

- GUESS, low (notes only)
- Original: the capture shows no stat told when the box is opened, so whether its doors open is not known
- Client: the box is announced with AnimPlay and AnimPos 0 and stays shut
- A player notices: unknown
- Where: `docs/CLIENT_UI.md:625`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: whether a cargo box's doors open is told by a stat no capture shows changing; it needs a capture of a box being opened. Left.

#### `RND-101` Character textures and clothes found by byte scan

- GUESS, low
- Original: the original reads the message field by field
- Client: the texture and cloth lists are searched byte by byte for a count marker followed by plausible names
- A player notices: a character whose update has marker-like bytes earlier may be dressed wrongly
- Where: `crates/mao-net/src/n3.rs:1887`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Reading the texture and cloth lists field by field means porting the rest of Gamecode's full-update reader (FUN_1007916d) for the fields before them, which the client skips by the scan; the scan finds what the captures hold. A reader port across many flag-dependent fields with no failing case known tonight. Left.

#### `RND-102` Darkness dimming of sky and light is a reading

- GUESS, low
- Original: darkness [8] sets VisualEnvFX_t +0x1134 to 1 - [8]/2.333; what the field does is not stated
- Client: takes it as a dimming of the sky and its light
- A player notices: how dark storms make the sky
- Where: `crates/mao-client/src/environment.rs:16`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): VisualEnvFX_t +0x1134 is one of the values DisplaySystem hands to the FXS scripts (drop-in fxvisual.rs maps script variable 0x2f8 to it), so what it does is decided by the scripts that read it, not by a rule in the program; the client's reading as a dimming is what the scripts seemed to do. Needs the scripts' use traced and a comparison. Left.

#### `RND-103` Deformer ripple phase constants guessed; modes 0 and 4 not done

- GUESS, low
- Original: _GfxControlDeformer_t callback FUN_100d776f uses two phase constants on the vertex's place (not legible); mode 0 pulses round named bones; mode 4 fades a second body mesh
- Client: ripple runs up the body with invented factors (frequency*age*TAU*4 + y*6); modes 0 and 4 not done
- A player notices: hit-deform ripple shaped differently
- Where: `crates/mao-client/src/effects.rs:101`; `docs/ORIGINAL_AUDIT.md:276`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The phase constants in FUN_100d776f are not legible in the decompile (noted), and modes 0 (pulses round named bones) and 4 (a second body mesh faded) need the bones' frames in the deformer and a second mesh; seen only. Left.

#### `RND-104` Effect lifetimes use the client's own default durations

- GUESS, low
- Original: not stated (the scheduler's period and life for effects without a duration)
- Client: stand-in lifetimes: stars 4.0 s, deform 1.5 s, cords 1.0 s, volume grid 3.0 s, unknown visuals (4.0, 1.5); endless effects restarted every ten minutes
- A player notices: effects without a duration end or restart at the client's times
- Where: `crates/mao-client/src/effects.rs:3231`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original runs an effect until its controls end themselves; the client's scheduler needs a period and life up front, which the stand-in figures give for kinds whose end is not computed. Replacing them means computing each kind's own end, a per-kind port. Left.

#### `RND-105` Electra effect: birth point scaling a guess; modes 0 and 2 not done

- GUESS, low (notes only)
- Original: decompile unclear whether the birth point is made unit before the scale
- Client: scales the sphere point; modes 0 and 2 (unused) not done
- A player notices: electra shape may differ
- Where: `docs/ORIGINAL_AUDIT.md:274`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: whether the birth point is made unit before the scale cannot be read from the decompile; modes 0 and 2 are unused by the templates. Left.

#### `RND-106` Face culling not checked in other playfield kinds

- GUESS, low (notes only)
- Original: D3DCULL_CCW, two-sided materials
- Client: mission dungeons, city interiors, shops, vehicles and the Shadowlands not checked; liquids, particles, shells, sky domes, blob shadows and the shadow map draw both sides
- A player notices: possible missing faces in unchecked places
- Where: `docs/CLIENT_UI.md:1442`; `crates/mao-render/src/lib.rs:265`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: a check of face culling in the other playfield kinds is a look in the client, not a port. Left.

#### `RND-107` Fall effect and water compared with one original shot

- GUESS, low (notes only)
- Original: pool (69, 71, 79), mist (154, 161, 169), sheet opaque dark grey (44, 45, 46)
- Client: pool (67, 66, 68), mist (193, 196, 200), sheet (96, 90, 86) showing the sand cliff through 20% and taking a glint; the original's hour unknown
- A player notices: fall sheet looks lighter and more transparent
- Where: `docs/CLIENT_UI.md:2233`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: one original shot at an unknown hour cannot settle the colours; needs another shot at a known hour. Left.

#### `RND-108` FXS quaternion [ROT] order and sky mesh z fitted

- GUESS, low
- Original: FUN_10005600 calls FUN_10004f91 at 0x10008027 with the turn so far first (o*acc); mesh pool data is the mesh as stored, frames not applied (FUN_10061505)
- Client: [ROT] composed the other way (ROT_REVERSED true by default) and mesh z negated, because only that lays the horizon's dish level; other readings behind a developer switch
- A player notices: sky domes oriented by what looked right, not a read rule
- Where: `crates/mao-client/src/fxs/mod.rs:2070`; `crates/mao-client/src/viewer.rs:3271`; `docs/ORIGINAL_AUDIT.md:93`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The decompile says FUN_10005600 composes [ROT] with the turn so far first (o*acc), and the mesh pool keeps meshes as stored; the client's reversed order and negated z are what lays the horizon's dish level. Following the decompile reopens the sky's look, which can only be judged against original screenshots; the switch to compare exists. Left.

#### `RND-109` Haze start is a fixed fraction of the view distance

- GUESS, low
- Original: an area's fog runs from the near plane 0.2 to the far plane drawn in by the density (VisualFog_t); the view distance setting is not stated
- Client: the plain haze begins at a seventh of the view distance (at least 40) and ends at the view distance
- A player notices: the distance haze may differ from the original's
- Where: `crates/mao-client/src/viewer.rs:854`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The haze is the client's own (RND-185, recommended for removal); with no area fog the original has none, so there is no start distance to port. Left with RND-185.

#### `RND-110` Hit and tracer effects at fixed offsets

- GUESS, low
- Original: not stated
- Client: hit effect at target z-1.0; tracer at point z-0.25; plasma +0.25
- A player notices: effects may sit off the original's height
- Where: `crates/mao-client/src/effects.rs:2986`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The hit and tracer places come from the effects' locators in the original (a hit at the target's attach point, a tracer along its locator's path), see RND-004 and RND-139; the fixed offsets stand in for those per-kind paths. Left with RND-004.

#### `RND-111` Land control bitmap texture filter unknown

- GUESS, low (notes only)
- Original: not read (none is set)
- Client: takes Direct3D's default, the nearest texel
- A player notices: map overlay edge look
- Where: `docs/CAPTURE_GAPS.md:218`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the original sets no filter for the land control bitmap, and Direct3D 7's default is point sampling, which the client takes. Nothing to change.

#### `RND-112` Passable props chosen by mesh name

- GUESS, low
- Original: not stated (collision from the zone surfaces)
- Client: when collision is built from visual meshes, parts whose mesh name says whompah entrance, door or gate are walk-through
- A player notices: such doors and gates never block in playfields without collision surfaces
- Where: `crates/mao-client/src/net.rs:5724`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Only used where collision is built from visual meshes (generated dungeons, RND-037); the original's collision there comes from the rooms' own data, not read. Left with RND-037.

#### `RND-113` Plants are told by whole-word endings in the mesh file name

- GUESS, low
- Original: not stated
- Client: what is a plant is decided by word endings (and a few starts) in the mesh file name
- A player notices: some plants may not sway, some non-plants might
- Where: `docs/CLIENT_UI.md:44`; `crates/mao-formats/src/model.rs:102`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The plant test feeds the client's own sway (RND-187, recommended for removal) and its cut-out (RND-002); the original has no plant flag. Left with those.

#### `RND-114` Plants' thinning rule chosen from screenshots

- GUESS, low
- Original: not read (whether the plant textures have smaller mip levels)
- Client: blended plants' crowns thin with distance, a rule chosen from comparing screenshots
- A player notices: tree and plant crowns may look thinner at distance than in the original
- Where: `crates/mao-render/src/lib.rs:241`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The thinning rule stands in for the unknown cause in RND-002; left with it.

#### `RND-115` Room cell height is 8 where the tile-map has none

- GUESS, low
- Original: not stated; the THSA table gives each type's height
- Client: takes 8 for the roof height where there is no entry
- A player notices: a character may or may not count as inside a room where a type has no height
- Where: `crates/mao-formats/src/rooms.rs:194`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The THSA table gives each tile type's height; a cell whose type has no entry gets the client's 8. What the original does with a missing entry (its lookup's default) was not read in the N3 room code tonight. Left.

#### `RND-116` Room light map colours more numerous than vertices: coloured from the map's start

- GUESS, low
- Original: DepackLightmap 0x10010eb7 applied by FUN_100082a7 as vertex emissive; the original colours the stream from the map's start regardless
- Client: the same; the cause (baked from other geometry) is a guess; floor not in the stream keeps scene light
- A player notices: room lighting may not match the original's if the original behaves otherwise
- Where: `crates/mao-formats/src/rooms.rs:32`; `docs/ORIGINAL_AUDIT.md:231`; `docs/CLIENT_UI.md:2249`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The client does what the original does (colours the stream from the map's start, DepackLightmap 0x10010eb7 applied by FUN_100082a7); only the explanation is a guess. Nothing to change.

#### `RND-117` Room texture layer choice is a guess

- GUESS, low (notes only)
- Original: not stated
- Client: red palette index for a cell top, green for walls; blue unused
- A player notices: some room surfaces may be textured wrongly
- Where: `docs/CLIENT_UI.md:1709`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: which palette channel picks the cell top's and the walls' texture layer was not read from the original; needs the N3 room drawer. Left.

#### `RND-118` Room turn direction chosen by checking against booths

- GUESS, low
- Original: VisualRoom_t::SetPosition(position, turn x pi/2)
- Client: the sense of the turn (cell offset becomes (dy, -dx)) was chosen by checking against shop booths the server places; "likely right, not proven"
- A player notices: things in turned rooms might be mirrored
- Where: `crates/mao-formats/src/rooms.rs:29`; `docs/CLIENT_UI.md:1830`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Checked against the original's math: VisualRoom_t::SetPosition (drop-in displaysystem room.rs) turns the room by axis_angle((0,1,0), turn x pi/2) and randy31's quat_rotation (math.rs) makes the row-vector matrix with m[2] = -sin and m[8] = sin, so a point (x, z) goes to (x cos + z sin, -x sin + z cos): a quarter turn takes (x, z) to (z, -x). The client's cell offset (dx, dy) to (dy, -dx) is that turn. Nothing to change; the catalog's "likely right, not proven" can say proven by the matrix.

#### `RND-119` Stars mode 13 ray duration uses the client's landing time

- GUESS, low
- Original: every template's duration is -1, so the hit's window must set it
- Client: uses LANDING (1.4 s)
- A player notices: ray timing guessed
- Where: `crates/mao-client/src/effects.rs:2026`; `docs/ORIGINAL_AUDIT.md:266`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Every mode-13 template's duration is -1, so the hit's window (the spell's hit effect run) sets it in the original; that window is the hit timing of RND-004, not read. Left with RND-004.

#### `RND-120` Stars modes 16/17/18 follow the decompile's shape, not its arithmetic

- GUESS, low (notes only)
- Original: the Ghidra text of the run function is mangled (register reuse)
- Client: modes follow its shape, not its arithmetic word for word
- A player notices: those star patterns approximate
- Where: `docs/ORIGINAL_AUDIT.md:266`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the run function's Ghidra text is mangled for these modes; reading the assembly (xdis) for three rarely used modes is over the time box. Left.

#### `RND-121` Untextured parts drawn in material colour

- GUESS, low (notes only)
- Original: no rule against them was found
- Client: 1,086 untextured parts in 643 meshes drawn in their material's colour
- A player notices: possibly odd coloured parts
- Where: `docs/CLIENT_UI.md:3387`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: no rule against untextured parts was found, and drawing them in the material's colour is what D3D7 does with no texture stage. Nothing to change.

#### `RND-122` Waterfall feet placed 1.2 under the liquid without surfaces

- GUESS, low
- Original: not stated
- Client: without surfaces the tile-map ground is held 1.2 under the liquid to place the fall effect
- A player notices: fall effects may sit at a slightly wrong height
- Where: `crates/mao-client/src/net.rs:5848`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The 1.2 under the liquid is the closest surface point's rule where the playfield has collision surfaces; without surfaces the client holds the tile-map ground there. The original always has its surfaces, so this case has no original rule. Left.

#### `RND-123` Zone layout told apart by which one reads to the zone's end

- GUESS, low
- Original: not stated
- Client: tries both layouts and keeps the one that ends at the zone's end; a zone with rooms is taken as it stands
- A player notices: a zone could be taken for the wrong layout
- Where: `crates/mao-formats/src/statels.rs:253`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): How N3's zone reader tells the two layouts apart is in the zone worker's run (see RND-140), not read tonight. Left with RND-140.

### Rendering and world: a guard or fallback the original lacks (11)

#### `RND-124` Statel effects limited to 150 units of the eye

- SAFETY, medium
- Original: the waterfall effect is hidden past 80 units; no limit stated for statels' own effects
- Client: a statel's own effect (campfire, chimney smoke, lamp halo) is drawn only within 150 units of the eye; start times are also the client's
- A player notices: far fires and lamps show no sparks or smoke
- Where: `crates/mao-client/src/viewer.rs:2344`; `docs/ORIGINAL_AUDIT.md:47`; `crates/mao-client/src/viewer.rs:2343`; `docs/CLIENT_UI.md:3434`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Removed the client's 150-unit limit: the original's statel effects run with the statel's visual (_EffectHandler_t::EnableEffect 0x100ce0dd, reached from N3 through n3EngineClientAnarchy_t::EnableEffect 0x10016906) and no distance of their own is set for them in Gamecode. The client now makes them out to its view distance (the fog's end, past which nothing shows). The start times staggered per place remain the client's own. viewer.rs only; no unit test (a draw loop), built clean.

#### `RND-125` Bone tree depth cut at 64

- SAFETY, low
- Original: not stated
- Client: pose_bone stops at depth 64
- A player notices: none visible
- Where: `crates/mao-formats/src/catmesh.rs:354`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Safety kept: without the depth cut a bone tree with a cycle would recurse until the stack overflows and the client ends, where the original would also run away; real bodies are a few dozen bones deep, so the cut changes nothing on screen. Left.

#### `RND-126` Collision volumes over 4000 triangles refused

- SAFETY, low
- Original: the reader takes up to 999 triangles a volume
- Client: a volume over 4000 triangles is refused as a broken stream
- A player notices: none visible
- Where: `crates/mao-formats/src/surface.rs:174`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Safety kept: the volume decoder (FUN_10012af0) has no count of its own (999 is the end mark's odds, s.bit(1, 999), not a limit); on a broken stream that never reads as broken it would grow without end, where the original would hang. Real volumes stay far below 4000 triangles (none was refused in the playfields walked so far), so the cap changes nothing a player sees. Left.

#### `RND-127` Frame trees deeper than 64 refused

- SAFETY, low
- Original: not stated
- Client: a frame tree deeper than 64 returns an error
- A player notices: such a mesh does not draw
- Where: `crates/mao-formats/src/model.rs:472`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Safety kept, as RND-125: a frame tree with a cycle would recurse until the stack overflows; real meshes are shallow. Left.

#### `RND-128` Missing liquid texture leaves the pass white

- SAFETY, low (notes only)
- Original: not stated
- Client: a missing texture leaves the pass white
- A player notices: white water if a texture is missing
- Where: `docs/CLIENT_UI.md:2183`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: a missing liquid texture does not happen with the game's files; what the original shows then (its texture stage with no texture: white by D3D7's default) is the same. Nothing to change.

#### `RND-129` Name tags not drawn under 1 or over 400 screen pixels tall

- SAFETY, low
- Original: not stated
- Client: a tag whose 0.3 m height projects to under 1 or over 400 pixels is left out
- A player notices: tags vanish when very far or very close
- Where: `crates/mao-client/src/nametag.rs:136`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The name plates are the client's own (RND-186); the original's tags are world sprites with no size test, which this client does not have to follow until they are ported (RND-027). The size limits belong to the client's own plates. Left with RND-186.

#### `RND-130` Rock thrower capped at 10 s, own step and random

- SAFETY, low
- Original: rocks stepped once a frame; random from rand()
- Client: stops after ROCKS_LAST 10 s; fixed 1/30 s step; xorshift seeded
- A player notices: rock throw timing and paths differ
- Where: `crates/mao-client/src/effects.rs:1073`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original steps a rock once a drawn frame with rand(); the client's fixed step and seeded generator are the same law (RND-063, frame-rate dependence), and its 10 s stop is past the seven bounces a rock makes before it rests, so it changes nothing seen. Left.

#### `RND-131` Shield2 skipped for dense meshes with flag 0x40000

- SAFETY, low
- Original: not stated (the flag test is a client guard)
- Client: returns nothing when flag 0x40000 is set and the mesh has more than 1000 vertices
- A player notices: no shield on dense meshes
- Where: `crates/mao-client/src/effects.rs:868`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Not a client guard: the original has the same test. DisplaySystem's GfxVisualShield2 (drop-in gfxshield2.rs too_big, and the draw FUN_1001d3ee) leaves a mesh alone when flag 0x40000 is set and it has more than 1000 vertices. The client's shield2_shell test matches. Nothing to change.

#### `RND-132` Soft dots for particles whose sprite sheet is not loaded

- SAFETY, low
- Original: not stated
- Client: a particle whose sprite sheet is not loaded is drawn as a soft dot
- A player notices: soft dots instead of the effect's sprites until the sheet loads
- Where: `crates/mao-render/src/lib.rs:2636`; `crates/mao-render/src/lib.rs:541`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Particles' sprite sheets are always in the game's files, so this stand-in never shows in practice; the original has no such case. Left.

#### `RND-133` Statel moving parts animate only within 250 m

- SAFETY, low
- Original: not stated
- Client: parts whose animation moves anything are drawn each frame only within 250 units of the character
- A player notices: far machines or doors (Whom-Pah glow, propellers) stand still
- Where: `crates/mao-client/src/net.rs:5527`; `docs/CLIENT_UI.md:3256`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The 250 m range does change what is seen: past it a statel's moving part is not drawn at all (net.rs overlay_model skips it). The original draws statel meshes under the LODManager's rule (hidden past 50 radii, RND-005) and gives every 0xf6951 mesh the time each frame. Removing the range alone would put every moving part of the playfield into the overlay rebuilt each frame on the network thread; the faithful change is to cull them by the LODManager's 50-radii rule (each part's radius from its vertices), which needs a measure of the cost on a big city before it ships. Left, with that as the suggested port.

#### `RND-134` Unreadable global statel ends the global data

- SAFETY, low
- Original: not stated
- Client: a global statel that fails to read stops the global list
- A player notices: later global statels are missing
- Where: `crates/mao-formats/src/statels.rs:238`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Safety kept: a global statel that cannot be read leaves the reader at an unknown place in the file; the original would read the rest as garbage statels. Stopping is the only way not to make up statels from garbage; no playfield's global data fails today. Left.

### Rendering and world: the original does it, the client does not (48)

#### `RND-135` Other characters' crafts not drawn

- MISSING, high
- Original: not stated
- Client: only one's own craft is known; the client is not told what another character wears
- A player notices: other players' crafts are not drawn
- Where: `crates/mao-client/src/net.rs:285`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): other players' crafts are drawn: the body and scale of their full update (a Yalmaha XL is template 99470 at 20), or of the item they put on in view; put away when they are announced without it, take it off, or go. See MOV-020.

#### `RND-136` Statel effects with their own colour or radius not drawn

- MISSING, high
- Original: the connector name gives colour and radius to the effect (DisplaySystem FUN_1006d49a table; AttractorEffectData_t words go to Gamecode's effect handler); how it applies them was not read
- Client: those effects are filtered out (180 of Jobe Research's 181 halos, 24 of Andromeda's 138); lamps such as Jobe's have no halo
- A player notices: lamps, campfires and chimneys without their glow or effect
- Where: `crates/mao-client/src/net.rs:5867`; `crates/mao-client/src/net.rs:5868`; `docs/CLIENT_UI.md:3430`; `docs/LIVE_TEST_LIST.md:387`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): How the original applies them was read: EnableEffect makes the effect, then the control's SetStartColor and SetStopColor get the same colour (the name's fourth byte is the alpha) and SetSize the size; on the single quad the colour replaces both colours and the size scales it. gfx::Visual::with_overrides does that for the quad, and every effect with a colour or size of its own in Jobe Research (180 of 181) and Andromeda (24 of 138) is that quad, so the lamps have their halos. Not done: the colour on fire, smoke and sparks (none seen with one). Looked at in an offscreen picture.

#### `RND-137` Statels of generated buildings and rendering effects get no effects

- MISSING, high
- Original: a generated building's statels are moved room by room so their connector effects go with them; the set-up function creates the rendering effects
- Client: statel effects are not placed for a generated building (mission dungeon); eff_hologram, eff_space, eff_clear not made; templates 12200, 12570, 96003 draw nothing
- A player notices: no fire, lamp or smoke effects in mission rooms and some places
- Where: `crates/mao-client/src/main.rs:762`; `crates/mao-formats/src/attractors.rs:66`; `docs/CLIENT_UI.md:3437`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18): Done: statel effects in a generated building (a mission's) and in an interior of rooms take the props' placing by their rooms; the Arete capture's alien ship has its 21 effects inside its rooms (none before, and a shop's stood at places inside their rooms' own frames). Left: eff_hologram, eff_space and eff_clear are ways a mesh is drawn (DisplaySystem's rendering effects 1 to 3), which this client has no draw for; the templates 12200, 12570 and 96003 are not in the effects' file and where the original has them was not found.

#### `RND-138` Weather: cloud layer (and other weather sprites) not drawn

- MISSING, high (notes only)
- Original: draws rain/dust sprites, cloud layer (VisualEnvFX_t +0x114), wind, lightning, thunder
- Client: fog and sky dimming from the weather object; rain, dust, fallout (client's own streak count and fall) and lightning are drawn now; no cloud layer found in code
- A player notices: no weather cloud layer
- Where: `docs/CLIENT_UI.md:2152`; `docs/ORIGINAL_AUDIT.md:67`; `docs/CLIENT_UI.md:2134`; `crates/mao-client/src/precipitation.rs:1`; `crates/mao-client/src/lightning.rs:1`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): The cloud layer was a misreading: +0x114 is the cloud cover's number in the frame block, and the layer is the scripts' ThickClouds mesh, which was drawn already. Ported what the weather really draws and the client did not: the rain or snow (entry 0, nothing was drawn for it), the four storms as wind-blown line clouds in the original's colours and numbers, each storm's flickering screen tint, all on a port of VisualRain_t (precipitation.rs); the storms' lines follow UseStormParticles. A fault mended with it: the sky's dimming and the fog's light went by entry 8 where the original reads entry 2 (the clouds). Cloud shadows left. Looked at in offscreen pictures only.

#### `RND-139` Effect locators: bone, weapon and mesh attractors; effect mesh details not drawn

- MISSING, medium
- Original: FUN_10105e6d finds locators 1000-1018 (bones), 3000 (weapon), 3001 (mesh attractors); effect meshes also use lighting and depth writes, the body's scale at the feet, vehicle flags 0x2000/0x4000 (shown while the vehicle moves forward), the mesh's animation 0x1000
- Client: only attractors 2000-2023 and the head fallback; the vehicle flags and the other listed parts not played or drawn
- A player notices: effects attached to a bone, weapon or mesh attractor do not show or sit elsewhere; vehicle effect meshes not toggled
- Where: `crates/mao-client/src/effects.rs:996`; `crates/mao-client/src/effects.rs:1856`; `docs/ORIGINAL_AUDIT.md:302`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): The bone locators are ported: FUN_10105e6d (Gamecode) resolves 1000-1018 by the names at 0x102c63a8 (`Bip01 Pelvis_ac` ... `Bip01 R Hand_ac`, whole name, RCATMesh_t::GetBoneMatrix), falling back to `Attractor01_head`. The characters' frames (net.rs, three places) now carry each bone's posed matrix (new CatMesh::bone_frames) beside the attractors, and effects::Locator::attractor looks 1000-1018 up there (BONE_NAMES), so effect meshes and trails on a bone are drawn at it instead of not at all. Left: 3000 (the weapon's frame, FUN_1009c337 on a WeaponItem_t), 3001 (the mesh's attractor 0), the effect meshes' lighting and depth writes, the 0x200 body scale at the feet, the vehicle flags 0x2000/0x4000 and the mesh animation 0x1000 (catalogued separately as RND-152..154). The buff rings still use the client's own figures (RND-022). Test: effects::tests (the attractor test now checks a bone, the head fallback for a missing bone and 1019 refused).

#### `RND-140` Statel zones of the second layout are not read

- MISSING, medium
- Original: n3Zone_t loader reads a zone's lights and packed block; room objects at the end of each zone
- Client: a zone that begins with a packed block and lights has its lights read past and no statels kept; a zone fitting neither layout is left out
- A player notices: lights and statels of such zones are missing
- Where: `crates/mao-formats/src/statels.rs:19`; `docs/ROADMAP.md:73`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The zone is read off the main thread in the original: n3StatelController_t::EnableRoomStatels (N3.dll 0x10025ae8) builds a zone object (FUN_1002896c) and schedules a WorkerRequest_c (FUN_100274b1) that reads the zone from the .pf file; the statel reader inside it is FUN_1002777a (read for RND-141). The reader of the zone's own header (the packed block and the 18-byte lights) is in that worker's run function, reached only through its vtable, which the scratchpad N3 decompile (n3.c) does not have in a readable form tonight. Porting it means a second zone reader in statels.rs checked against every playfield's file, and how the two layouts are told apart (RND-123); over 40 minutes with no reader to copy. Left.

#### `RND-141` Statels' own matrix (second flag layout) not applied

- MISSING, medium
- Original: N3.dll's statel reader builds a further matrix (SetAnimMatrix, FUN_1002435c) for statels with flag bit 0; elevators have sound and effect
- Client: only the rotation about Y and the stretch along X are read; the extra matrix is left out (flagged as unread_rotation); elevators have no sound or effect
- A player notices: such statels are not tilted or placed as in the original; elevators silent
- Where: `crates/mao-formats/src/statels.rs:61`; `crates/mao-formats/src/statels.rs:64`; `docs/ROADMAP.md:77`; `docs/CLIENT_UI.md:3963`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): The matrix is ported: N3.dll's statel reader FUN_1002777a, for flag bit 0, after the Y turn (v % 630 × 2π/630) and the stretch s = (v/630 % 211)/100 + 0.5, builds an anim matrix scaled by (1, 1/s, 1/s) (FUN_10026cdc) and sheared by p = (v/630/211)/100 − 1.25 of x into y (FUN_10026d5c, constants read from N3.dll: 100.0, 0.5, 1.25), handed to VisualMesh_t::SetAnimMatrix (FUN_1002435c); the frame applies it before its uniform scale s×C, turn and place (randy31 refframe update_world_matrix). Statel::shear and Statel::matrix carry it, and both statel placings in main.rs use it. Left: the elevators' sound and effect. Test: statels::rotation_tests::second_layout_shears.

#### `RND-142` Worn meshes: the original's two hiding modes not ported

- MISSING, medium
- Original: DisplaySystem hides meshes by two modes at +0x74
- Client: shown_meshes follows the flags only; not seen live
- A player notices: some worn items the original hides may be drawn
- Where: `crates/mao-net/src/n3.rs:1949`; `docs/CLIENT_UI.md:1058`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Found the rule: VisualCATMesh_t::SetShape (DisplaySystem, stores +0x74) is called by Gamecode FUN_100590b8 only with 0 or 1: 1 when the character's MonsterData (stat 359) is set (a polymorph or a craft), 0 otherwise; shape 2 is never set by Gamecode. FUN_10073517 (drop-in catrun.rs attach_attractors) then hides, in shape 1, every worn mesh except those at place 0 with flag bit 2 (the character's own head), and in shape 2 every place outside 1..4. The client draws a MonsterData body with no worn meshes at all (net.rs overlay_model, creature_parts with an empty gear list), so the faithful change is to hang the character's own bit-2 head on the creature body's head attractor while it is morphed. Whether that head then sits on (or doubles) the creature's own head on the morph and craft bodies can only be judged by eye, and the gear list the client keeps (World::equipment) has already lost the flags (shown_meshes), so it needs a second list kept from the raw one. Left for a session that can look.

#### `RND-143` Animation rest pose of a frame not used

- MISSING, low
- Original: a sliding door's closed pose is the first keys of its animation
- Client: local_matrix reads the rest pose and drops it ("the key layout is not settled")
- A player notices: some statels and doors may stand in the saved (open) pose
- Where: `crates/mao-formats/src/model.rs:318`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The key layout of a frame's rest pose is not settled in the client's mesh reader (model.rs local_matrix drops it), and the drop-in's frame reader (randy31 refframe.rs, `anim_matrix` and the animation holder) keeps the rest pose as the anim matrix only when the frame has no animation; tying the two needs the stored layout checked on a sliding door's mesh and seen in game. Left.

#### `RND-144` Area control percentages, level range, sounds and styles dropped

- MISSING, low
- Original: AreaData_t records carry control percentages, level range, five sounds and two style words
- Client: read past and dropped; only name, environment and centre are kept
- A player notices: area sounds and the like are not used from here
- Where: `crates/mao-formats/src/areas.rs:168`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The dropped fields (control percentages, level range, five sounds, two style words) have no use in the client yet: keeping them is only worth it with the land-control and area-sound features that read them. Left.

#### `RND-145` Area environment lists of unknown format or version not read

- MISSING, low
- Original: GameData reads the area list in whatever format it comes
- Client: only list formats 5 to 7 and area version 4; any other gives no areas and a log message
- A player notices: such a playfield has no area light, fog or weather blend
- Where: `crates/mao-formats/src/areas.rs:143`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Only list formats 5 to 7 and area version 4 occur in this installation's data; the other formats GameData reads have no example to test a reader against. Left.

#### `RND-146` BParticle2: spin, ground bounce, height and daylight fades, locator turn not done

- MISSING, low
- Original: FUN_1010b499 spins particles (words 23-24), bounces them on the ground (flag 0x8000), fades by height and daylight (0x800000 / 0x400000), turns by the locator (0x1000000); word 15 births several a frame
- Client: none of these; the billboards do not turn; a particle is reborn at its death
- A player notices: particles do not bounce on the ground or fade with daylight
- Where: `crates/mao-client/src/effects.rs:2388`; `docs/ORIGINAL_AUDIT.md:281`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_1010b499's spin needs the renderer's particle spin (RND-032), the ground bounce and the fades need per-particle state and the daylight, the locator turn the locator passed in; a set of visual ports judged by eye. Left.

#### `RND-147` BParticle: only kind 8, no turn, distance fade or ground flag

- MISSING, low
- Original: GfxVisualBParticle quad turns, fades by distance (words 28-30), can lie on the ground (flag 0x400); kind 1 is tall turned beams with scrolled texture
- Client: only kind 8; no turn, no distance fade, no ground flag
- A player notices: other BParticle kinds (three programs' beams) show nothing; those shown do not turn
- Where: `crates/mao-client/src/effects.rs:2612`; `docs/ORIGINAL_AUDIT.md:298`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The other BParticle kinds (kind 1's tall turned beams with scrolled texture), the turn, the distance fade and the ground flag are visual ports needing turned quads and texture scroll in the renderer; seen only. Left.

#### `RND-148` CAT meshes' own level of detail by distance not done

- MISSING, low (notes only)
- Original: run_function sets levels 0/1/3/6 at 16, 32, 48 units (RCATMesh_t +0x21c)
- Client: not ported
- A player notices: none visible (skinning cost)
- Where: `docs/ORIGINAL_AUDIT.md:256`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: a CAT mesh's own levels of detail at 16, 32 and 48 units choose lower-detail triangle lists that the client's mesh reader does not keep; reading them and switching is a format and renderer port. Left.

#### `RND-149` Character mesh colour word not applied

- MISSING, low
- Original: the colour word is shown through a curve
- Client: stored but not applied
- A player notices: a mesh's tint colour does not show
- Where: `crates/mao-formats/src/catmesh.rs:14`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The colour word's curve is not described in what was read (catmesh.rs keeps the word); applying it needs the curve from the CAT renderer. Left.

#### `RND-150` Collision surfaces of versions other than 5 not read

- MISSING, low
- Original: n3SurfaceResource_t::ReadBlob (0x10016179) reads several versions
- Client: only version 5 is read; any other gives none
- A player notices: a zone with an older surface has no collision from it
- Where: `crates/mao-formats/src/surface.rs:377`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Only version 5 surfaces occur in this installation's 0xf424d objects; the other versions n3SurfaceResource_t::ReadBlob (0x10016179) reads have no example to test against. Left.

#### `RND-151` Corner map: patch's second texture and room floors missing

- MISSING, low (notes only)
- Original: patch's second texture; rooms' floor pictures (FUN_100474b7)
- Client: not done; interiors keep the client's walked map
- A player notices: interior maps are the walked map
- Where: `docs/CLIENT_UI.md:593`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the patch's second texture and the rooms' floor pictures (FUN_100474b7) are drawing ports of the corner map; seen only. Left.

#### `RND-152` Effect mesh: body scale at the feet (flag 0x200) not applied

- MISSING, low
- Original: flag 0x200 scales by body scale
- Client: not applied at the feet
- A player notices: mesh wrong size on scaled bodies when held at the feet
- Where: `crates/mao-client/src/effects.rs:1856`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Flag 0x200 grows the mesh by the frame's scale, and at the feet that is the body's scale (stat 360 / 100); the viewer's aura loop has the body's breed, sex and fatness but not its scale, which lives in the network thread's World. Small plumbing, but only seen. Left.

#### `RND-153` Effect mesh: own animation and attractor effects not played

- MISSING, low
- Original: flag 0x1000 mesh animation; FUN_1010ca78 attractor effects
- Client: not played
- A player notices: animated effect meshes stand still, no sub-effects
- Where: `crates/mao-client/src/effects.rs:1857`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The effect mesh's own animation (0x1000) and its attractor effects (FUN_1010ca78) need the effect meshes run through the animated-mesh path and their connectors' effects started; seen only. Left.

#### `RND-154` Effect meshes unlit and without depth writes

- MISSING, low
- Original: the plain effect (0) is lit and writes depth
- Client: unlit, no depth writes
- A player notices: effect meshes look flat
- Where: `crates/mao-client/src/effects.rs:1855`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Lighting and depth writes for the plain effect mesh need a lit, depth-writing pipeline for effect shells in the renderer (they are drawn in the blended shell pass); seen only. Left.

#### `RND-155` Effect scripts: screen fills (0x3d, the fill square) and some kinds not drawn

- MISSING, low
- Original: draws all effect kinds
- Client: the visuals module does not draw the render targets, the screen fills (0x3d), the debug line monitor, the occluders and the attractors' effects
- A player notices: some scripted effects (the attack's flash) show nothing
- Where: `crates/mao-client/src/fxs/visuals.rs:26`; `docs/CLIENT_UI.md:1258`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Screen fills (0x3d, the fill square), render targets, the debug line monitor and the occluders are renderer features of their own (full-screen passes, offscreen targets); judged by eye. Left.

#### `RND-156` Far ground patches not averaged to one texel

- MISSING, low (notes only)
- Original: averages far patches to one texel
- Client: not stated as done
- A player notices: far terrain texture differs
- Where: `docs/ORIGINAL_AUDIT.md:185`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: averaging far ground patches to one texel is a level-of-detail of the ground's textures; mips do the same work in the client. Left.

#### `RND-157` Fire effect: rand() gate (word 31) and attractor frames not played

- MISSING, low
- Original: _GfxControlFire_t gates flames with rand() (word 31); attractor frames
- Client: gate not played, random numbers hashed; the body's own frame is used
- A player notices: flame count may differ from the original
- Where: `crates/mao-client/src/effects.rs:2500`; `docs/ORIGINAL_AUDIT.md:286`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The rand() gate (word 31) draws from the process-wide rand() (RND-063) and the attractor frames need the locator passed into the fire code (as RND-045); seen only. Left.

#### `RND-158` Generic scripted visual: points not drawn

- MISSING, low
- Original: BlueprintRender supports other visualizations including points
- Client: only Visualization 1 (a strip) is drawn
- A player notices: point-based scripted visuals do not appear
- Where: `crates/mao-client/src/fxs/visuals.rs:382`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): BlueprintRender's other visualizations (points among them) need a point primitive in the scripted visuals' draw; no script seen tonight uses them. Left.

#### `RND-159` Ground grid: locator's turn and laid-once (0x800) not done; random turn is a fixed hash

- MISSING, low
- Original: locator's turn; laid once with 0x800; flag 0x20000 turns the grid at random
- Client: world's frame, follows the point; the 0x20000 turn is a fixed hash, the same every time
- A player notices: ground grid orientation/following differs, turn not random
- Where: `crates/mao-client/src/effects.rs:1528`; `docs/ORIGINAL_AUDIT.md:284`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The locator's turn needs the locator passed in, laid-once (0x800) needs the place kept from the start (state), and the random turn draws from rand() (RND-063); seen only. Left.

#### `RND-160` Hall's nano effect and figure animations do not run

- MISSING, low
- Original: the hall's nano effect runs and each mesh loops its animation
- Client: static meshes
- A player notices: the profession figures and hall stand still
- Where: `crates/mao-client/src/hall.rs:28`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The hall's nano effect and the figures' looping animations need the hall's meshes run through the animated path and the effect started at the hall's place; seen only in the login hall. Left.

#### `RND-161` Highlight effect: render priority 0x400 and hide word not done

- MISSING, low (notes only)
- Original: flag 0x400 render priority; a hide word
- Client: not done (mode 3, items only, is handled)
- A player notices: highlight variants missing
- Where: `docs/ORIGINAL_AUDIT.md:287`; `crates/mao-formats/src/gfx.rs:262`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: render priority 0x400 orders the highlight among the transparent draws, and the hide word was not read; seen only. Left.

#### `RND-162` House building effect not shown

- MISSING, low
- Original: the building effect when a house is constructed
- Client: not done
- A player notices: no construction effect
- Where: `crates/mao-client/src/houses.rs:11`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The building effect plays when a house is put up live (see RND-167); without the live signal handled there is nothing to start it from. Left with RND-167.

#### `RND-163` Lightning: the bolt's ground-place note not made

- MISSING, low
- Original: FUN_100b8b35 notes the bolt's ground place (radius 50)
- Client: not done
- A player notices: none visible known
- Where: `crates/mao-client/src/lightning.rs:23`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_100b8b35's note of the bolt's ground place (radius 50) feeds whatever reads it (the note's consumers were not read); nothing visible follows from making the note alone. Left.

#### `RND-164` Nano effect: rand() gate (word 24) not played

- MISSING, low
- Original: FUN_100e6f76 gates sprite making with rand() (word 24)
- Client: ignored because it is zero in the templates examined
- A player notices: a template with the gate set would emit more sprites than the original
- Where: `crates/mao-client/src/effects.rs:163`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The gate word is 0 in the templates examined, so the gate never closes; when it is not, it draws from rand() (RND-063). Nothing visible to port. Left.

#### `RND-165` Occluder bodies give no occlusion culling

- MISSING, low (notes only)
- Original: hands the [OCC] bodies to HMOccluder_t, which leaves out what stands behind them
- Client: no occlusion culling; the bodies are not drawn and not used
- A player notices: none visible (performance)
- Where: `docs/CLIENT_UI.md:3361`; `docs/ORIGINAL_AUDIT.md:49`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: occlusion culling changes cost, not the picture (what it leaves out is hidden anyway). Left.

#### `RND-166` Old CHGA tile-maps for rooms not ported

- MISSING, low
- Original: the cell constructor has a branch for old CHGA tile-maps
- Client: rooms drawn only from the GNDA tile-map form
- A player notices: none now; a room playfield on an old tile-map would not draw
- Where: `crates/mao-formats/src/rooms.rs:1364`; `docs/ORIGINAL_AUDIT.md:68`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The old CHGA tile-map branch of the room cell constructor is a format port with no reader in the drop-ins to copy; the playfields that need it are the ones the flat floor stands in for (RND-024). Over the time box. Left.

#### `RND-167` Player city houses added or removed live not handled

- MISSING, low
- Original: AOTransportSignal 0x62741e15 codes 2 and 9 with building effect 0x30e8 for a house built or taken down in the city
- Client: houses are drawn from PlayfieldAllCities at login; a house added or taken away while in the city, and the building effect, are not done; the signal's lead byte is a guess
- A player notices: houses do not appear or vanish live
- Where: `crates/mao-client/src/houses.rs:10`; `docs/CLIENT_UI.md:3754`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): AOTransportSignal 0x62741e15 codes 2 and 9 have no capture (its lead byte is a guess), so a live house change cannot be decoded with confidence; needs a capture of a house going up or down. Left.

#### `RND-168` Scatter effect: original's random numbers, matrix and connector placements not done

- MISSING, low (notes only)
- Original: hashed random numbers; matrix and connector placements
- Client: not done
- A player notices: scatter placement differs
- Where: `docs/ORIGINAL_AUDIT.md:304`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the scatter's random numbers are rand()'s (RND-063), its matrix and connector placements need the locator passed in; seen only. Left.

#### `RND-169` Script-started effects: several visual kinds not drawn

- MISSING, low
- Original: FXS visuals include render targets, screen fills (kind 0x3d), debug line monitor (7), occluders, attractors' effects and projected lights
- Client: none of these are drawn (state blobs with a render target are skipped; only kinds 1, 4, 5, 0x2a, 0x38 are kept)
- A player notices: no scripted full-screen flashes or tints, and some sky effects absent
- Where: `crates/mao-client/src/fxs/visuals.rs:25`; `crates/mao-client/src/fxs/visuals.rs:26`; `docs/ORIGINAL_AUDIT.md:93`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As RND-155: render targets, screen fills, the line monitor, occluders, attractors' effects and projected lights are renderer features each; judged by eye. Left.

#### `RND-170` Sequencer effects: end cutting child's duration, loop 0x400, flag 0x800

- MISSING, low
- Original: _GfxControlSequencer_t end above 0 cuts the child's duration to end - start (56 of 522 entries); 0x400 loops; 0x800 runs without the dynel
- Client: not applied (children made at their starts only)
- A player notices: some sequencer children play too long or do not loop
- Where: `crates/mao-formats/src/gfx.rs:2449`; `docs/ORIGINAL_AUDIT.md:269`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The sequencer's end cutting a child's duration needs a new limited-duration visual handled by every visual function (particles, meshes, trails, shells, timing) in effects.rs; 0x400 looping and 0x800 (running without the dynel) need the scheduler's state. Over the time box for a low entry. Left.

#### `RND-171` Shield2 effect: mech meshes, own material and depth modes not drawn

- MISSING, low
- Original: GfxVisualShield2 shields the EP03 mechs' meshes (word 20 from 0), flag 0x10000 draws the mesh's own material, flags 0x4000/0x8000 give depth modes, and the sequencer has later children
- Client: the mech meshes, the mesh's own material and the depth modes are not drawn; the sequencer's later children are not done
- A player notices: shield variants look different or are absent
- Where: `crates/mao-client/src/effects.rs:861`; `docs/ORIGINAL_AUDIT.md:303`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The mech meshes (word 20), the mesh's own material (0x10000) and the depth modes (0x4000/0x8000) need the shield drawn over other meshes and new depth states in the renderer; seen only. Left.

#### `RND-172` Smoke effects: quads' spin, alpha blend, mode 1 wandering emitter not done

- MISSING, low
- Original: quads spin; mode 5 blends under alpha; mode 1 has a wandering emitter (kind 6 random wander); original's random is one-sided
- Client: no spin; mode 5 added like the rest; mode 1 taken as mode 0; births at 60 a second with a hash for random numbers
- A player notices: smoke and waterfall mist look brighter and differ from the original's
- Where: `crates/mao-client/src/effects.rs:2274`; `crates/mao-client/src/effects.rs:2191`; `docs/ORIGINAL_AUDIT.md:279`; `docs/CLIENT_UI.md:2228`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The quads' spin needs the renderer's particle spin (RND-032), mode 5's alpha blend a blended particle pass (RND-079), mode 1's wandering emitter per-effect state with rand(); seen only. Left.

#### `RND-173` Spell 0xcf76 on MonsterData does not reload the mesh

- MISSING, low
- Original: 0xcf76 (FUN_100a95c7) on stat 0x167 MonsterData also has the mesh taken anew (FUN_10052686)
- Client: not ported
- A player notices: a body set by that stat is not redrawn
- Where: `crates/mao-client/src/spelllist.rs:241`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): FUN_100a95c7 reloads the mesh when 0xcf76 sets MonsterData (stat 0x167) (FUN_10052686). The client already draws the body a MonsterData names from the full update (World::crafts) and from the worn craft's wear list (refresh_mods, a_worn_yalmaha_is_its_craft), and for a polymorph from the running program; making 0xcf76 itself switch the body would be a third path for the same change, and only one's own temporary sets are applied (World::temp_stat). Needs a live check of which arrives first before adding it. Left.

#### `RND-174` Spiral2 effect: body's torso scale not applied

- MISSING, low
- Original: radius, rise and turn scaled by the body's torso scale
- Client: scale taken as 1
- A player notices: spirals on non-standard-size bodies are the wrong size
- Where: `crates/mao-client/src/effects.rs:1577`; `docs/ORIGINAL_AUDIT.md:289`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The body's torso scale is not passed to the effect code (as RND-078); seen only. Left.

#### `RND-175` Star water drawn as plain water; its star field and mists not ported

- MISSING, low
- Original: star water (types 0xe/0xf) has its own star field and mists
- Client: the star water layer is drawn with the water passes; star field and mists not ported (no playfield has star water)
- A player notices: none visible so far
- Where: `crates/mao-render/src/lib.rs:2568`; `docs/ORIGINAL_AUDIT.md:65`; `docs/CLIENT_UI.md:74`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): No playfield in this installation has star water (types 0xe/0xf), so there is nothing to see or test. Left.

#### `RND-176` Stars modes 12, 14, 28 show the spiral child

- MISSING, low
- Original: _GfxControlStars_t modes 12, 14, 28 (6, 6 and 3 program uses)
- Client: no mode of its own; shows the kind-1001 child's spiral
- A player notices: those few effects look wrong
- Where: `docs/ORIGINAL_AUDIT.md:266`; `crates/mao-client/src/effects.rs:312`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): _GfxControlStars_t modes 12, 14 and 28 (15 uses in all) have run functions not ported; each is a visual port judged by eye. Left.

#### `RND-177` Statels' moving parts all play on the clock

- MISSING, low
- Original: statels' parts follow their state (AnimPlay/AnimPos) the way things do
- Client: the playfield statels' moving parts are drawn at the clock's time; things the server announces do follow their AnimPlay/AnimPos
- A player notices: some built-in animated props play when they should stand
- Where: `docs/CLIENT_UI.md:626`; `crates/mao-client/src/net.rs:5498`; `crates/mao-client/src/net.rs:5451`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Partly a misreading: DisplaySystem's LODManager::Process gives every 0xf6951 statel mesh the time each frame when its animate flag is set (drop-in lodmanager.rs, global `animate` = 1), so statels' moving parts do run on the clock in the original. Whether a statel's part can also be stopped by a state (as things are by AnimPlay/AnimPos) depends on the statel's own messages, which were not seen. Left.

#### `RND-178` Sun rays do not flicker

- MISSING, low
- Original: VisualSunRays_t (FUN_1005a3a9) flickers the rays by a table
- Client: brightness set only by the horizon
- A player notices: sun rays steady
- Where: `crates/mao-client/src/fxs/visuals.rs:459`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): VisualSunRays_t's flicker table (FUN_1005a3a9) changes only the rays' brightness frame by frame; porting the table is possible but the effect is seen only and the table's indexing by time was not read through tonight. Left.

#### `RND-179` Suns effect: modes 1-3 not done

- MISSING, low (notes only)
- Original: _GfxControlSuns_t modes 1-3 (21 uses)
- Client: not done (modes 0 and 4 are drawn)
- A player notices: a few effects missing
- Where: `docs/ORIGINAL_AUDIT.md:273`; `crates/mao-client/src/effects.rs:4463`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the suns' modes 1-3 are visual ports of their own; seen only. Left.

#### `RND-180` TParticle2: fade by facing and texture options not done

- MISSING, low (notes only)
- Original: flag 0x40000 fades the strips by how squarely they face the camera; flags 0x100, 0x80000
- Client: not done
- A player notices: streak particles differ
- Where: `docs/ORIGINAL_AUDIT.md:288`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the facing fade and texture options need the strips' facing to the camera in the renderer; seen only. Left.

#### `RND-181` TParticle: frame turn and move not played; kind 3 draws nothing

- MISSING, low
- Original: frame turned by FUN_10112ff1 and moved by FUN_100d6a3d; kind 3; 0x400 ground
- Client: not played; random numbers hashed; kind 3 draws nothing
- A player notices: streak effect does not turn/move with its frame; some bursts missing
- Where: `crates/mao-client/src/effects.rs:1425`; `docs/ORIGINAL_AUDIT.md:292`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The frame turn (FUN_10112ff1) and move (FUN_100d6a3d) need per-effect state, and kind 3 is a visual of its own; seen only. Left.

#### `RND-182` Water refraction: byte 0x188 skip not followed

- MISSING, low (notes only)
- Original: frame block's byte 0x188 skips the pass when set
- Client: not followed (what sets it not found); refraction made with a screen copy; MAO_NO_REFRACTION turns it off
- A player notices: refraction could show where the original skips it
- Where: `docs/CLIENT_UI.md:2198`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: what sets the frame block's byte 0x188 was not found, so there is no rule to follow yet. Left.

### Rendering and world: the client has it, the original does not (12)

#### `RND-183` Anti-aliasing, anisotropic filtering, mips and hemisphere light

- ADDITION, medium
- Original: a DirectX 7 device with no anti-aliasing mentioned
- Client: 4x MSAA by default or temporal anti-aliasing, mip chains, 16x anisotropic filtering, hemisphere ambient light with a warm sun
- A player notices: smoother, differently lit picture
- Where: `crates/mao-render/src/lib.rs:1292`; `crates/mao-render/src/lib.rs:1690`; `docs/CLIENT_UI.md:55`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: anti-aliasing, anisotropic filtering and mips are display-quality settings with no rule of the original's to break (its DirectX 7 device simply had none); the hemisphere ambient is the part that changes the look, and it only stands in where the scripts give no light (see RND-001).

#### `RND-184` Coloured boxes stand in for characters with no model

- ADDITION, medium
- Original: not stated
- Client: a character with no loadable model is a box (red own, blue creature, green player) with a yellow nose
- A player notices: boxes appear in place of unmodelled characters
- Where: `crates/mao-client/src/net.rs:5428`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the original has a model for every character (its fallback is `unfinished-breed.cir`, see RND-028), so the boxes only show where this client cannot yet load one; without them such characters would be invisible and unclickable.

#### `RND-185` Distance haze when the area has no fog

- ADDITION, medium
- Original: fog only from the area's VisualFog_t
- Client: a haze toward the sky's horizon colour from a start to an end distance when no area fog applies
- A player notices: far scenery fades to the sky colour
- Where: `crates/mao-render/src/lib.rs:2237`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend REMOVE: the original's only fog is the area's VisualFog_t; with none it draws clear to its far clip, so the haze is a look of the client's own.

#### `RND-186` Name plates are the client's own

- ADDITION, medium
- Original: not stated
- Client: names within 40 units, nearest forty, colour by kind; no health bars or levels (level added later for targeted characters); an Org after the name
- A player notices: floating names over characters
- Where: `docs/CLIENT_UI.md:1840`; `crates/mao-client/src/main.rs:1760`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the original's name tags are VisualSprite_t in the world (HandleNametags 0x100268ee); until those are ported (RND-027) the client's plates are the only way names show, and removing them would leave none.

#### `RND-187` Plants sway in the wind and bend away from the player

- ADDITION, medium
- Original: not stated; the original does not name a plant flag
- Client: a mesh whose file name has a word ending in tree, bush, palm, grass, flower and so on is marked foliage; the renderer sways it in the wind (more towards the top) and bends it away from the player within 3 units, in the shader rather than by the mesh animation
- A player notices: trees and grass move in the wind and bend when the player walks near
- Where: `docs/CLIENT_UI.md:46`; `crates/mao-formats/src/model.rs:82`; `crates/mao-client/src/main.rs:920`; `crates/mao-render/src/lib.rs:177`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend REMOVE: the original has no plant flag and moves plants only through their meshes' own animations; the sway and the bend away from the player are the client's own and the plant test itself is a guess by file name (RND-113).

#### `RND-188` Real sun shadows from the playfield's props, shadow-map options

- ADDITION, medium
- Original: characters have only blob shadows (GfxVisualSimpleShadow_c under CAT meshes); nothing else throws a shadow
- Client: the playfield's statels/meshes throw real shadows into a sun shadow map (1024, 2048 or 4096 texels, a setting), kept and redrawn only when the player moved 8 m, the sun turned, shadows toggled or playfield changed; render scale, sharpness, anti-aliasing and texture compression options; Renderer::shadows=false gives the original's
- A player notices: shadows from trees and buildings the original never drew; they update in steps
- Where: `crates/mao-render/src/lib.rs:2367`; `crates/mao-render/src/lib.rs:455`; `crates/mao-client/src/viewer.rs:2573`; `docs/CLIENT_UI.md:2253`; `docs/CLIENT_UI.md:2754`; `docs/ORIGINAL_AUDIT.md:69` and 3 more
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: it is an option and `shadows` off gives the original's blob shadows only (GfxVisualSimpleShadow_c); suggest it default to off if the original's look is wanted out of the box.

#### `RND-189` Drawing scale with FSR 1 upscaling and a mip bias

- ADDITION, low
- Original: not stated
- Client: the scene can be drawn at 40 to 100 percent of the window and upscaled with FSR 1 plus sharpening; textures are read with a bias of log2 of the scale (liquids and particles unchanged)
- A player notices: a render-scale setting; softer or sharper picture
- Where: `crates/mao-render/src/lib.rs:1458`; `docs/CLIENT_UI.md:1452`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: a performance option for slow machines that defaults to full scale; it does not change the picture's content.

#### `RND-190` Frame limiter and background frame rate

- ADDITION, low
- Original: not stated
- Client: fps_limit and background_fps with a 2 ms spin-wait before a limited frame
- A player notices: client can be capped lower when not in front
- Where: `crates/mao-client/src/settings.rs:589`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: frame limiting is a machine setting with no rule of the original's to break.

#### `RND-191` Generic emitter: dense clouds dimmed, jitter and clamps

- ADDITION, low
- Original: not stated
- Client: alpha scaled down by the square-metre coverage of the cloud; 0.12 m jitter; life clamped 0.1-6 s; size min 0.04; at most 400 particles
- A player notices: dense particle clouds look dimmer and softer than the template says
- Where: `crates/mao-client/src/effects.rs:3347`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend REMOVE: the dimming, jitter and clamps change the generic emitter's look away from the template's own numbers; if a template makes a cloud too dense, that is how the original draws it.

#### `RND-192` Hit effect fade-in and fade-out added

- ADDITION, low
- Original: not stated
- Client: hit particles' alpha multiplied by a fade over the last 0.4 s and first 0.1 s
- A player notices: hit particles ease in and out
- Where: `crates/mao-client/src/effects.rs:2981`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend REMOVE: the original's hit effects end as their own controls fade them; an extra fade in and out changes their look.

#### `RND-193` Textures block-compressed on the card, on by default

- ADDITION, low
- Original: not stated (the DirectX 7 device takes the textures as loaded)
- Client: textures are re-encoded to BC7/BC3 at upload when the card allows (BC3 for mesh textures); sides not multiples of four stay plain; "Compressed textures" is a launcher option, default on
- A player notices: less video memory, slightly lossy colours, possible banding or stutter
- Where: `crates/mao-render/src/lib.rs:2908`; `crates/mao-client/src/settings.rs:585`; `crates/mao-client/src/launchwindow.rs:484`; `docs/CLIENT_UI.md:1388`; `docs/LIVE_TEST_LIST.md:678`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: compression saves video memory and is a launcher option; the original's DX7 device also took compressed (DXT) textures where the files had them.

#### `RND-194` View distance and character distance settings

- ADDITION, low
- Original: not stated
- Client: view distance slider (fog ends there) and character distance (beyond it characters not drawn)
- A player notices: extra settings
- Where: `crates/mao-client/src/settings.rs:545`; `docs/CLIENT_UI.md:1880`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the original has a view distance setting of its own in its options; the character distance is a performance option.

## Sound and music

### Sound and music: the client knowingly does otherwise (1)

#### `SND-001` Sound plays through the client's own mixer

- OWN-RULE, low
- Original: SandyInterface.dll's SoundEngineInterface_t over the Miles sound system
- Client: Sandy's logic ported over a rodio-fed software mixer (13 channels, same allocation rule, one 44.1 kHz stereo source), samples decoded whole, linear interpolation, output clamped; sound options applied by audio.rs setting its volumes and mutes
- A player notices: same rules for which sound plays, a different mixing quality
- Where: `crates/mao-client/src/audio.rs:1`; `crates/mao-client/src/mixer.rs:1`; `crates/mao-client/src/gui/controlcenter.rs:179`; `docs/CLIENT_UI.md:2409`; `docs/GUI_DLL_PARITY.md:63`
- Decision: [ ] port  [ ] leave

### Sound and music: an approximation or a stand-in (9)

#### `SND-002` Indoors the zone is always 0 (music, district info)

- SIMPLIFIED, medium
- Original: PlayfieldDistrictInfo_t::GetDistrictData(GetZoneInstanceID()) finds the district of the character's zone, rooms included
- Client: indoors the zone is 0, no rooms' zones kept; indoor music does not follow rooms
- A player notices: district music and land control info indoors always come from the first zone
- Where: `crates/mao-client/src/sounds.rs:204`; `docs/CLIENT_UI.md:2468`
- Decision: [ ] port  [ ] leave

#### `SND-003` Large variant index plays nothing

- SIMPLIFIED, low
- Original: reads past the variant array for a large index
- Client: nothing is played
- A player notices: none visible in normal play
- Where: `crates/mao-client/src/audio.rs:632`
- Decision: [ ] port  [ ] leave

#### `SND-004` Liquid depth probed at the character, not under it

- SIMPLIFIED, low
- Original: probe tests at the ground point under the character with the character's height for the depth, keeping the last depth within 0.1 above the surface
- Client: tests at the character itself; the 0.1 grace is out
- A player notices: splash/wade step sounds may change at slightly different heights
- Where: `crates/mao-client/src/sounds.rs:141`
- Decision: [ ] port  [ ] leave

#### `SND-005` Music player's random choices start from a fixed seed

- SIMPLIFIED, low
- Original: seeds the CRT random generator with the tick count
- Client: generator seeded 1 every time
- A player notices: the music's random choices repeat from run to run
- Where: `crates/mao-client/src/music.rs:146`
- Decision: [ ] port  [ ] leave

#### `SND-006` Music streams start with no driver latency

- SIMPLIFIED, low
- Original: AIL_digital_latency is the driver's latency
- Client: latency is 0
- A player notices: none visible
- Where: `crates/mao-client/src/music.rs:14`
- Decision: [ ] port  [ ] leave

#### `SND-007` Sample memory budget is fixed

- SIMPLIFIED, low
- Original: not stated how the budget is set
- Client: fixed 0xf00000 byte budget, least recently used freed
- A player notices: samples may be re-read from disk sooner in a big session
- Where: `crates/mao-client/src/audio.rs:321`
- Decision: [ ] port  [ ] leave

#### `SND-008` Simultaneous sound plays ordered by creation

- SIMPLIFIED, low
- Original: the frame's play map is ordered by the sources' addresses
- Client: ordered by the order the sources were made
- A player notices: none visible
- Where: `crates/mao-client/src/audio.rs:237`
- Decision: [ ] port  [ ] leave

#### `SND-009` Sound preferences read once from LoginPrefs.xml

- SIMPLIFIED, low
- Original: GUI.dll's slots pass the preferences on as the player changes them
- Client: reads LoginPrefs.xml once when the sound system starts and passes the values on
- A player notices: volume changes depend on the options window feeding the same module
- Where: `crates/mao-client/src/audio.rs:1627`
- Decision: [ ] port  [ ] leave

#### `SND-010` Thunder only on playfields with a tile map

- SIMPLIFIED, low
- Original: not stated
- Client: the strike cell needs the zone side from a tile map; none without
- A player notices: no thunder on room playfields
- Where: `crates/mao-client/src/ambience.rs:163`
- Decision: [ ] port  [ ] leave

### Sound and music: the original was not read: inferred or measured (1)

#### `SND-011` Music districts' heads found by their shape

- GUESS, low (notes only)
- Original: district spawn data read
- Client: districts' heads found by their shape (spawn data not read)
- A player notices: district music may be wrong in places
- Where: `docs/ORIGINAL_AUDIT.md:92`
- Decision: [ ] port  [ ] leave

### Sound and music: the original does it, the client does not (6)

#### `SND-012` Alert and noise radii not sent to the game's AI

- MISSING, low
- Original: module 9's messages 0xbf and 0x119 tell AFCM the alert and noise radii of sounds, for the game's AI
- Client: not sent (music is done since)
- A player notices: creatures' reaction to noise is not driven by the sound system's radii
- Where: `crates/mao-client/src/audio.rs:16`; `docs/ORIGINAL_AUDIT.md:90`
- Decision: [ ] port  [ ] leave

#### `SND-013` Environment sounds: zones' own lists, env FX pools, water level

- MISSING, low (notes only)
- Original: the zones' lists; env FX pools
- Client: all infos tested; each strike's effects always had; no water level in the ground's height
- A player notices: none stated
- Where: `docs/ORIGINAL_AUDIT.md:94`
- Decision: [ ] port  [ ] leave

#### `SND-014` Environment type's indoor sounds and effects not followed

- MISSING, low (notes only)
- Original: environment type steers indoor sounds and effects (FUN_100b2478's switch)
- Client: not followed
- A player notices: indoor ambient sounds/effects by environment type absent
- Where: `docs/CLIENT_UI.md:2605`
- Decision: [ ] port  [ ] leave

#### `SND-015` Fly-mode step sound not played

- MISSING, low
- Original: FUN_100452d3 plays a step in fly mode (movement mode 6) at 0.25 volume
- Client: the client keeps no such mode, so no flying step sound
- A player notices: no quarter-volume flight step sound
- Where: `crates/mao-client/src/sounds.rs:101`; `docs/CLIENT_UI.md:2438`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): the entry was misnamed: the quarter volume is for sneaking (movement mode 6), which is ported; a flyer makes no step sound in the original either.

#### `SND-016` Item-hit sound and "Sneak Attack!" texts not done

- MISSING, low (notes only)
- Original: FUN_1009b4b0 item hit; "Sneak Attack!" texts
- Client: not done (the special attack's own fields, FUN_1006af0e, are since handled in feedback.rs)
- A player notices: missing sounds/texts for those
- Where: `docs/ORIGINAL_AUDIT.md:91`; `crates/mao-client/src/feedback.rs:241`
- Decision: [ ] port  [ ] leave

#### `SND-017` Lightning strike at the character has no sender

- MISSING, low
- Original: FUN_100bde3c's strike at the character (AFCM message 0x7f, FUN_100b2d7e)
- Client: no sender, so a strike on the character does not happen
- A player notices: only distant thunder and near jolts
- Where: `crates/mao-client/src/ambience.rs:8`; `docs/ORIGINAL_AUDIT.md:94`
- Decision: [ ] port  [ ] leave

## Launcher, login, character creation

### Launcher, login, character creation: the client knowingly does otherwise (8)

#### `LCH-001` Launcher is its own window and process that starts the game

- OWN-RULE, medium
- Original: the login runs inside the game's own window (LoginModule_c, ServerLogin3DModule_t) and goes straight on to the playfield
- Client: the launcher is its own process: own login form and character list, Play starts the game as a new process (`net <game> enter <name> --live`) with the account in its environment (only the account name saved); the loading screen stays up in the launcher until the game's window has drawn
- A player notices: a second window and a few seconds' hand-over
- Where: `crates/mao-client/src/launcher.rs:300`; `docs/CLIENT_UI.md:1828`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original runs the login, the character list and the game in one process and one window (GUI.dll `LoginModule_c`, `ServerLogin3DModule_t`); the client's launcher is a separate program because the game's window, event loop and renderer are set up for one playfield per process (winit allows one event loop per process: launcher.rs "an event loop cannot be made twice in one"). Putting the login into the game's window would rebuild the launcher and the game's start; far beyond a night's entry. What can be matched inside the split is matched (LCH-009, LCH-038, LCH-045).

#### `LCH-002` Launcher settings pages keep their names but have this client's own contents

- OWN-RULE, medium
- Original: three settings pages (dialog templates 153, 161, 107): Direct3D device, colour depth, texture quality, compressed textures, texture colour depth, skin folder, font, report to Funcom, folders
- Client: wgpu adapter, edge smoothing, shadows, drawing scale, view distance, character distance, frame limits, VSync, theme, interface size, own folders; texture quality, compressed textures, colour depth and report to Funcom gone; drawn inside the launcher window
- A player notices: settings offered differ from the original's
- Where: `crates/mao-client/src/launchwindow.rs:21`; `docs/CLIENT_UI.md:789`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's three pages (dialog templates 153, 161, 107 in Anarchy.exe) choose a Direct3D 7 device, colour depths and texture formats that wgpu has no counterpart for (see SET-037, SET-042); keeping the page names with the client's own rows is the nearest the client can be. Nothing to port without those choices meaning something.

#### `LCH-003` Loading screen picture is tinted blue; zoning also shows it

- OWN-RULE, medium
- Original: the picture gfx/ai_loading_login.png is shown as it is; no loading screen noted at zoning
- Client: each channel multiplied (0.35, 0.6, 1.45) so the picture is blue; a zoning shows the screen too (fades out over 1.5 s); picture kept in proportion. (The welcome picture note is out of date: welcome_to_rubika.jpg is now drawn, loading.rs:93.)
- A player notices: loading picture is blue; loading screen at zoning
- Where: `crates/mao-client/src/loading.rs:9`; `crates/mao-client/src/loading.rs:93`; `docs/CLIENT_UI.md:2796`; `crates/mao-client/src/loading.rs:10`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): The blue tint was the client's own (added with the first loading screen, 2026-10-29, not a user's request): GUI.dll `ServerLogin3DModule_t::SetupScreen` 0x10017331 draws `gfx/ai_loading_login.png` (or `welcome_to_rubika.jpg` after a creation) scaled from 1024x768 to the display with no colour change. `loading.rs` now shows the pictures as they are (`TINT` and `tint` removed); test `loading::tests::the_screen_fades_in_then_out`. Left: the screen over a zoning, since whether the original covers a zoning with a screen of its own (and which) was not read: `ServerLogin3DModule_t` is the login's module, and the zoning's flow in GUI.dll/Gamecode is still to be traced.

#### `LCH-004` Wide window keeps the 4:3 view height in the login hall and character creation

- OWN-RULE, medium
- Original: the view is 90 degrees across the window (VisualCamera_t::SetViewPlaneWindow), so a wide window sees less up and down and cuts the figures at the knees and head
- Client: keeps the height a 4:3 window has: a wide window sees the same height and more to the sides, showing space outside the hall's walls (the user's choice of full width over black bars; also for the character list)
- A player notices: wide windows show whole smaller figures and space outside the hall
- Where: `crates/mao-render/src/lib.rs:841`; `docs/ORIGINAL_AUDIT.md:26`; `docs/CLIENT_UI.md:809`; `docs/CLIENT_UI.md:1490`; `docs/CLIENT_UI.md:4524`; `crates/mao-client/src/hall.rs:20` and 1 more
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The 4:3 view height on wide windows is the user's own choice (full width over black bars, docs/ORIGINAL_AUDIT.md and CLIENT_UI.md), and `MAO_HALL_ORIGINAL=1` already shows the original's rule (90 degrees across the window, `VisualCamera_t::SetViewPlaneWindow`) for comparison. Not changed against the user's decision.

#### `LCH-005` A successful login puts away the error page

- OWN-RULE, low
- Original: keeps a failed login's error page until its Browser window is closed, so it stands over the login again after Back
- Client: a login that works clears the page
- A player notices: error page does not come back after Back
- Where: `crates/mao-client/src/launcher.rs:355`; `docs/CLIENT_UI.md:1028`; `docs/ORIGINAL_AUDIT.md:37`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The client's own rule is gone: GUI.dll keeps the error's address until the Browser window is closed (`SlotBrowserQuitPressed` 0x10011c33 empties it), and `LoginModule_c::Show(0)` 0x1001175a shows that window over the login again whenever it has an address, so after a working login and Back the page stands over the login again. The launcher no longer clears `error_page` when the list comes (only closing the Browser window does, and the client's own empty-account case of LCH-043). No unit test (screen state); launcher tests pass.

#### `LCH-006` Launcher window title reads "Modern Anarchy Online"

- OWN-RULE, low
- Original: title with local and remote versions
- Client: same shape with this client's name
- A player notices: different title text
- Where: `crates/mao-client/src/launchwindow.rs:253`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The title keeps the original's shape (name and the local and remote versions); only the name is the client's, which is what the program is. Writing "Anarchy Online" there would present the client as Funcom's own launcher.

#### `LCH-007` Login dialog keeps a list of used accounts with Remove

- OWN-RULE, low (notes only)
- Original: shows the Server row only when not Rubi-Ka (as the client does); username list with Remove
- Client: list of previously used accounts with a Remove button kept in account.ini; default theme "original"
- A player notices: stored accounts list
- Where: `docs/CLIENT_UI.md:2070`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Notes only, and the original has it: GUI.dll `LoginWindow_c` has the username list with `remove_btn`, kept as `LauncherConfig`'s `NumAccounts`, `Account%d` and `SelectedAccount` (`SaveAccounts`); the client's list with Remove is that, kept in account.ini (never a password). Nothing changed.

#### `LCH-008` Progress bar time-out comes from the launcher's Connection page

- OWN-RULE, low
- Original: bar over the game's GeneralNetworkTimeout (90 s)
- Client: uses the launcher.ini time-out when ever saved, else the game's preference
- A player notices: bar length may differ
- Where: `crates/mao-client/src/launcher.rs:248`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Whether the original launcher's Connection page writes the game's `GeneralNetworkTimeout` (which the login's progress window reads) was not read in Anarchy.exe; if it does, the client's "the launcher's value when saved, else the game's" is the original's. Left until Anarchy.exe's page handler is read.

### Launcher, login, character creation: an approximation or a stand-in (13)

#### `LCH-009` A new character is not logged straight in; the list is fetched again

- SIMPLIFIED, medium
- Original: logs a new character straight in (Client_t::ProcessMessage 0x11)
- Client: logs in again to fetch the list with the new character selected, then plays that name
- A player notices: an extra login round-trip
- Where: `crates/mao-client/src/launcher.rs:211`; `docs/CLIENT_UI.md:2824`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Checked the original: Interfaces.dll `Client_t::ProcessMessage` 0x10002a9e, system message 0x11 (0x10002ee3) reads the new character's id into `s_nCharID` and calls `LoginCharacter` 0x10001c5b at once. The launcher now starts the game for the character as soon as the creation's walk to the shuttle ends (`Outcome::Created`): it is written as the account's last character, picked and played with the new-character loading screen; the extra login that fetched the list again (and the `play_new` field) is gone. No unit test (the launcher's screen state has none); built.

#### `LCH-010` A click does not skip the creation intro flight, only Escape

- SIMPLIFIED, low
- Original: Escape (command 0x31) puts the camera at the transition's end
- Client: only Escape is wired (the comment says "or a click"); skip works during the intro and entering the breed scene only
- A player notices: clicking does not skip
- Where: `crates/mao-client/src/ccscene.rs:413`; `crates/mao-client/src/charcreate.rs:571`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The client already does what the original does: GUI.dll `CharCreateModule_t::SlotEscPressed` 0x1011b69e is the only skip (command 0x31, `FUN_10115c71`), and the module's other input, `CameraControlMessage` 0x1011b428, maps keys only (the debug camera); no click skips the flight in the original either. Only the comment of `ccscene::Director::skip`, which said "or a click", was wrong; corrected (comment-only commit).

#### `LCH-011` Character list screen falls back to a lone turning figure

- SIMPLIFIED, low
- Original: the character stands in the hall where the original's viewer puts it (hall::stand)
- Client: when the hall is not found, or a character is made without the creation scene, the figure is alone on a dark blue sky with fixed fog and a camera turning at a fixed rate
- A player notices: a plain turntable instead of the hall
- Where: `crates/mao-client/src/launcher.rs:788`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The turntable is only the fallback for when the hall's meshes are not found in the game's files (or a picture is taken without the creation scene); with a complete game the character stands in the hall as the original's (`hall::stand`). The original has no such case to port.

#### `LCH-012` Character screen and login dialog draw their own lists

- SIMPLIFIED, low (notes only)
- Original: XML TextButton / ComboBox views
- Client: launcher's character screen draws its own list; the login dialog draws its own list
- A player notices: launcher built from the client's own widgets
- Where: `docs/CLIENT_UI.md:3023`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the character screen and login dialog are drawn on the skin's own views (`loginviews.rs`: `LoginWindow.xml`, `CharacterSelectionWindow.xml`); only the two lists inside them are egui widgets because the client's XML views have no `TextButton` list / `ComboBox` yet. A port is the GUI.dll list widgets, a GUI_DLL_PARITY item larger than this entry.

#### `LCH-013` Creation choices are kept only while the launcher runs

- SIMPLIFIED, low
- Original: keeps the choices in its preferences
- Client: keeps them in memory for the launcher's run; lost when it is closed
- A player notices: creation restarts after the launcher is closed
- Where: `crates/mao-client/src/launcher.rs:214`; `CHANGELOG.md:47`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The original keeps the creation's choices in `IndependentPrefs_t` (`CCSelectedBreed`, `CCSelectedProfession`, `CCSelectedHead`, `CCSelectedHeight`, `CCSelectedSize`, `CCSelectedName`), which outlive the program; GUI.dll `CharCreateModule_t::InitialiseMessage` 0x1011bdaa reads them back and first resets them to 0, 0, 0, 1, 1 and "" when `WasCharacterCreated` is set (set by `NameScene_t::SetState` 0x1007 once a character was made). The client now keeps them in `creation.ini` beside the settings: written when the creation is left (Back), read when New Character starts a creation (a creation left in the same run still goes on where it was), and reset when a character is made (`charcreate::keep_choices`, `kept_choices`, `forget_choices`). Test: `charcreate::tests::the_choices_are_kept`. The choices are written on leaving rather than at each click (the original sets each value as it is chosen); the difference shows only if the launcher is killed in the middle of a creation.

#### `LCH-014` Creation picking uses figure boxes; head point is the figure's top less 0.12

- SIMPLIFIED, low
- Original: picking by the figure's mesh; the head's place from its attractor
- Client: the figure's box is used; the face camera's head point is the figure's top less a fixed 0.12
- A player notices: slightly different pick area and face close-up
- Where: `crates/mao-client/src/launcher.rs:587`; `docs/CLIENT_UI.md:4538`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Picking by the figure's mesh and the head's place from its attractor need the creation figures' meshes ray-tested and their attractors read through the CCCharacter_t path (`CCCharacter_t::MoveToConnector`, the head attractor); the boxes are within a few pixels for the seven breed figures. Too much for its worth tonight.

#### `LCH-015` Delete confirmation compares names case-insensitively

- SIMPLIFIED, low
- Original: OK only once the name is typed (else MatchError); case rule not stated
- Client: compares trimmed text ignoring ASCII case
- A player notices: not known whether the original ignores case
- Where: `crates/mao-client/src/loginviews.rs:455`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Read: GUI.dll `CharDeleteWindow_c::SlotOkayButtonActivated` 0x1000db27 compares the field's text (`TextInputView_c::GetText`) with the character's name by `std::string::compare`: exact, case-sensitive, nothing trimmed; any difference shows `MatchError`. The client compared trimmed text ignoring case; now `loginviews::delete_name_matches` is the exact comparison. Test: `loginviews::tests::deleting_asks_for_the_name_exactly`.

#### `LCH-016` Login hall loaded once instead of four stages

- SIMPLIFIED, low
- Original: loads the five meshes four times, a stage two units above the last, shows one stage
- Client: one copy of the hall
- A player notices: none visible
- Where: `crates/mao-client/src/hall.rs:7`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original loads the hall's five meshes four times, a stage two units above the last, and shows one stage; the client's single copy draws the same picture (only one stage is ever shown). Nothing a player can see.

#### `LCH-017` Meta-Physicist description tags taken out

- SIMPLIFIED, low (notes only)
- Original: description with <font> tags and entities
- Client: tags and entities taken out
- A player notices: plain text description
- Where: `docs/CLIENT_UI.md:2821`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the Meta-Physicist's description is GUI.dll's `DescriptionMetaPhysicists` with `<font>` tags and entities, which the creation's text area (drawn by the client's own painter) shows as plain text; drawing the tags' colours needs the creation's text areas moved onto the GUI text view (`gui/text.rs`), which the creation scenes do not use yet.

#### `LCH-018` Name case is set by the client

- SIMPLIFIED, low
- Original: FUN_10123cdc's code 3 asks for the case; verdict text shown by StartServerCreation
- Client: the client sets capital then small letters itself before sending
- A player notices: names forced to Capital-then-lower
- Where: `crates/mao-client/src/charcreate.rs:94`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The original sets the case itself too: GUI.dll `NameScene_t::StartServerCreation` 0x1012015d refuses an empty field (`MustEnterName`), then runs FUN_10124093 on the text: the character at place 0 `toupper`, every other `tolower`, and every one that is not a letter or digit (FUN_10123bbb) dropped; only then FUN_10123cdc judges it (its code 3, the case, can no longer come and is not refused). The client did the case but trimmed spaces instead of dropping every other character, and said `MustEnterName` for a name that became empty. Now `charcreate::proper` drops non-alphanumerics after setting the case by the place in the typed text, and `charcreate::checked_name` is `StartServerCreation`'s order (empty field first, then the made name's length / digit / letter checks); the creation screen and the `net ... create` command use it. Test: `charcreate::tests::the_rules`.

#### `LCH-019` News page in WebView2, can be switched off, links open in the system browser

- SIMPLIFIED, low
- Original: embeds a browser control at SHOWURL; the upper page is always shown
- Client: WebView2 child (wry); a `page` setting leaves it out; links asking for a new window go to the system browser
- A player notices: page behaves as in a normal browser; optional
- Where: `crates/mao-client/src/launchwindow.rs:568`; `docs/CLIENT_UI.md:776`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The page shows by default (`page = true`), as the original's always-shown browser control; the switch and sending new-window links to the system browser are what a WebView2 child needs (it has no window of its own to open them in). Nothing of the original's left to port.

#### `LCH-020` No colour depth; exclusive full screen when neither box is ticked

- SIMPLIFIED, low
- Original: Resolution: predefined, custom, Run In Window, Borderless, with a colour depth
- Client: same boxes; neither ticked is exclusive full screen; no colour depth
- A player notices: no colour-depth choice
- Where: `crates/mao-client/src/launchwindow.rs:27`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The boxes and their meaning are the original's (neither ticked is full screen); only the colour depth is missing, which wgpu cannot choose (the swap chain's format is the display's; see SET-037).

#### `LCH-021` Unknown refusal codes show "ServerProblem"

- SIMPLIFIED, low
- Original: SetState's codes with texts of category 600
- Client: any other code is shown as "ServerProblem"
- A player notices: unknown refusals look generic
- Where: `crates/mao-client/src/charcreate.rs:143`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Read: GUI.dll `NameScene_t::SetState` 0x1011fc0f has cases for codes 1-10, 0x13, 0x14, 0x15, 0x1c, 0x1e, 0x1f (each a category-600 text; 0x1e/0x1f also the `SM_Sandy_CC_Nick` sound); any other code goes to its default: no message, the waiting flag cleared and the buttons updated. `charcreate::refusal` now returns `None` for those codes and the creation shows nothing (and plays nothing) for them, instead of "ServerProblem". Test: `charcreate::tests::refusals_have_the_originals_texts_or_none`. (The test command `net ... create` prints an empty text for such a code.)

### Launcher, login, character creation: the original was not read: inferred or measured (6)

#### `LCH-022` Breed, sex and profession numbering on the character rows is inferred

- GUESS, low (notes only)
- Original: not stated
- Client: "usual" numbering (9 Enforcer, 15 Shade, ...) checked against some characters; area not shown
- A player notices: a label could be wrong for some professions
- Where: `docs/CLIENT_UI.md:1836`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only. The original's character row names breed, sex and profession through Gamecode's name tables (`CharSelectItem_c::SetupView` 0x1000e48e calls `N3InterfaceModule_t::GetBreedStr` / `GetSexStr`, Gamecode `n3EngineClientAnarchy_t::GetBreedStr` 0x10016168 → FUN_10036b15, "Missing breed: %d"), which FUN_100368d0 fills from the game's data at run time rather than from a table in the program; reading that loader and its data file was more than this entry's worth tonight. The client's numbers are the Profession stat's values every capture agrees with.

#### `LCH-023` Character deletion messages named from AOSharp and a capture

- GUESS, low
- Original: not stated
- Client: delete character (0x14, 0x15) is named from AOSharp's messages and the Arete capture, not the original's code
- A player notices: none visible
- Where: `crates/mao-net/src/create.rs:20`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Checked against the original: Interfaces.dll `Client_t::DeleteCharacter` 0x10001a67 builds system message 0x14 with `s_nCharID` (push 0x14 at 0x10001aa9), and `Client_t::ProcessMessage` 0x10002a9e handles 0x15 at 0x10002e71 (with `s_nCharID`, telling its listeners). The client's 0x14 / 0x15 are the original's; only the comment in `mao_net::create` now names the original's functions (comment-only commit).

#### `LCH-024` Creation timings and text details are guesses or not done

- GUESS, low
- Original: camera transitions are built by the original (durations not shown as read); a half second a scene waits; DockingCompleted and ContinuingDNA lines; TextWindowFade_t timing 5 and 4; ambience asked every frame; profession text area 156 units
- Client: fixed durations 1.9 s, 3.4 s, 3.7 s, 0.3 s; walk-to-shuttle fade at 1/8 per second; a second of caption fade; ambience started once; none of the other lines
- A player notices: small timing differences in creation
- Where: `crates/mao-client/src/ccscene.rs:494`; `crates/mao-client/src/ccscene.rs:528`; `crates/mao-client/src/ccscene.rs:283`; `docs/CLIENT_UI.md:4539`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The durations are built by the original's transition code (CharCreateModule_t's camera keys, `FUN_10115210`, `PlainSpriteFade_t` with `_DAT_101b0848`, `TextWindowFade_t` with `_DAT_101a8ba0` / `_DAT_101a9fa4` / `_DAT_101b20b0` / `_DAT_101c41e8`, all in `CharCreateModule_t::FrameProcess` 0x1011c355) from several constants and the camera file; each of the six items is its own reading of a 17 kB function. Too many parts for one entry tonight; the constants named here are the place to start.

#### `LCH-025` Launcher background picture is picked by the clock

- GUESS, low
- Original: one of the ten pictures; which one was not read
- Client: one picked by the clock (seconds modulo count)
- A player notices: background may differ from the original's choice
- Where: `crates/mao-client/src/launchwindow.rs:137`; `docs/CLIENT_UI.md:761`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Read in Anarchy.exe: the launcher lists the files of `\backgrounds\` (0x41525b on, `\backgrounds\01.bmp` the one named when there is none, 0x415196), then seeds a Mersenne twister (mt19937: 624 words, 0x6c078965 at 0x415498) with `_time64(NULL)` (0x41546a) and draws one index between 0 and count-1 (0x4154b4..0x415503, the distribution at 0x415ec1). So the original's choice is a random picture per start, seeded by the clock; the client's "seconds modulo count" is the same in effect (another picture each start, chosen by the clock). Matching the very picture for a given second would need MSVC 2010's `uniform_int` draw reproduced bit for bit, which no player could tell apart; left.

#### `LCH-026` Launcher control rectangles measured, list drawn as a plain white box

- GUESS, low
- Original: not stated
- Client: control rectangles measured on the original's window; the list is a plain white box
- A player notices: layout may be a pixel off
- Where: `crates/mao-client/src/launchwindow.rs:82`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The launcher's controls are in Anarchy.exe's dialog templates (resources 153, 161, 107 and the main dialog); reading their exact rectangles needs a dialog-template reader over the program's resources, and the measured rectangles are within a pixel. The list's white box is how the original's plain Win32 list box looks with the default colours. Left.

#### `LCH-027` Profession scene text area height is a guess

- GUESS, low
- Original: height is 156 units of the lowest resolution, scaled
- Client: 0.2 of the window, by the original's pictures
- A player notices: text box a bit different in size
- Where: `crates/mao-client/src/charcreate.rs:306`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Read: GUI.dll `ProfessionScene_t` 0x101216d1 takes `GetScaleFromLowestResolution` (the display over 640x480; the y share, [ebp-0x34]), brings it 0.4 of the way from 1 (`(s - 1) * 0.4 + 1`, `_DAT_101a9150`), multiplies by 156 (`_DAT_101c4f98`) and floors: the text area's height in display pixels; the width is `floor(width * 0.6)` and the middle at 0.325 / 0.8125 of the display (as the client already had). `charcreate::place::profession_text_height` is that rule and the profession scene's text area uses it (in the display's pixels, turned into points); test `charcreate::tests::the_profession_text_height` (480 → 156, 768 → 193, 1080 → 234).

### Launcher, login, character creation: a guard or fallback the original lacks (6)

#### `LCH-028` Dimension list is cached and falls back to a built-in Rubi-Ka; only the first port is used

- SAFETY, low
- Original: downloads the list from DimensionServer.url
- Client: keeps the last list in dimensions.txt for when the server does not answer; with none, a built-in Rubi-Ka entry; an answer listing no dimension is refused; of a dimension's ports only the first is used
- A player notices: launcher works offline
- Where: `crates/mao-client/src/launchwindow.rs:1206`; `crates/mao-client/src/dimensions.rs:98`; `crates/mao-client/src/dimensions.rs:45`; `docs/CLIENT_UI.md:781`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The cache is the original's own: the original launcher keeps the last list in dimensions.txt and says it is using the cached data when the server does not answer (its message, SET-013), as the client does. What is the client's alone is the built-in Rubi-Ka entry when there is no list at all, and the refusal of an answer that lists no dimension; without them a first start with the dimension server down would leave no dimension to log in to. Kept for the user to decide (removing them could make the morning's tests look like a broken launcher). Whether the original uses more than a dimension's first port was not read.

#### `LCH-029` Hand-over gives up after 180 seconds

- SAFETY, low
- Original: not stated
- Client: the launcher stops waiting for the game's window after 180 s (or when the game ends) and closes
- A player notices: none unless the game hangs
- Where: `crates/mao-client/src/launcher.rs:126`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: the 180 s limit only ends the launcher's own wait for the game's window when the game hangs before having one; the original has no second process to wait for. Nothing a player can do or see changes.

#### `LCH-030` Launcher guards, lenient graphics choice, error box and crash log

- SAFETY, low
- Original: not stated
- Client: falls back down to the system's software device; pictures and page run guarded so a failure leaves the plain window; a box says why it cannot go on; the launcher keeps a crash log
- A player notices: launcher survives odd graphics setups and says why it failed
- Where: `crates/mao-client/src/launchwindow.rs:49`; `docs/CLIENT_UI.md:4709`; `CHANGELOG.md:31`; `CHANGELOG.md:33`; `crates/mao-render/src/lib.rs:89`; `crates/mao-client/src/crash.rs:50`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: the software-device fallback, the guarded pictures and the error box keep the launcher from ending (a Rust panic) where a graphics device or a picture fails; the original's equivalents are Windows' own (Direct3D falls back, a missing bitmap draws nothing). The crash log is SET-048.

#### `LCH-031` Login error browser shrinks on low screens

- SAFETY, low
- Original: BrowserView_c of 780 by 410
- Client: on a screen too low for the two windows the page gives way (smaller, minimum 200x120)
- A player notices: smaller error page on small screens
- Where: `crates/mao-client/src/loginviews.rs:203`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: the launcher's window is 1000 by 640, lower than the original's game window the login ran in; the original's 780x410 page plus the login window and their gaps need about 700 lines, so without the shrink the page would run off the window's bottom. The page keeps the original's size wherever it fits.

#### `LCH-032` Login error shown as words when no page can be opened

- SAFETY, low
- Original: LoginModule_c::ShowError shows the error's page of the game's site in the Browser window (BrowserView_c in LoginBrowserWindow_c)
- Client: the page is a WebView2 child over the window; with no WebView2 (or non-Windows) the refusal is said in words, in red under the fields, with "Logging in..." shown while logging in
- A player notices: players without WebView2 see words, not the site's page
- Where: `crates/mao-client/src/launcher.rs:623`; `crates/mao-client/src/loginviews.rs:182`; `crates/mao-client/src/loginviews.rs:134`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: the words are only shown where no WebView2 (or another system's web view) can be made, where the original's page could not be shown at all; with WebView2 the page is the original's.

#### `LCH-033` Plain orange buttons when the game has no Anarchy.exe

- SAFETY, low
- Original: uses bitmaps from the program (resources)
- Client: plain buttons when the folder has no Anarchy.exe
- A player notices: none visible with the game installed
- Where: `crates/mao-client/src/launchwindow.rs:826`; `docs/CLIENT_UI.md:786`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: plain buttons are only drawn when the game folder has no Anarchy.exe to take the original's bitmaps from (a Linux/Mac install or a stripped folder); with the game installed the buttons are the original's.

### Launcher, login, character creation: the original does it, the client does not (9)

#### `LCH-034` Character creation: figures and ships do not animate, no selection glow, no expansion rules

- MISSING, high
- Original: the profession figures and the intro's ships animate; the hovered/chosen breed has a selection glow; expansion rules apply; camera debug display
- Client: figures and ships stand still; the hovered breed is only named in the text; no glow; no expansion rules; no debug display
- A player notices: static creation scenes, no highlight on the chosen figure
- Where: `crates/mao-client/src/ccscene.rs:31`; `docs/CLIENT_UI.md:4537`; `docs/GUI_DLL_PARITY.md:71`; `CHANGELOG.md:46`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): what the original really does is ported, and the entry's "Original" line says more than that. Ported: the meshes' animations (the far suns, the stars, the intro's shuttle), the chosen character's idle and fidgets with its size (0.95 / 1.05) and set-down by build, and the heads by the Shadowlands flag with the numbers the original leaves out. Not in the original: animated profession figures (plain meshes) and a drawn selection glow (turned on and off in the same call, GUI.dll 0x10113baa). The camera's debug display is developer text and was left. `CLIENT_UI.md` "The character creation's scenes move".

#### `LCH-035` No account activation window and no limbo window

- MISSING, high
- Original: CharActivateWindow_c and LimboWindow_c exist
- Client: neither exists (no mention in launcher.rs, loginviews.rs, charcreate.rs)
- A player notices: accounts needing activation or limbo cannot proceed in the client
- Where: `docs/GUI_DLL_PARITY.md:70`; `crates/mao-client/src/launcher.rs`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): Play on an inactive character asks (CharacterActivateWindow.xml, ActivateCharacterWarning) or, with no slot free, shows CharacterSlotsExhausted, as SlotLoginPressed 0x1000dcf0 does; test play_asks_for_an_inactive_character. The limbo window (system message 0x4e) is left: no capture of it.

#### `LCH-036` Patching and licence dialog not done

- MISSING, high
- Original: PLAY turns into PATCH when the versions differ; an end-user licence dialog; (error pages and a background pick too)
- Client: a version difference is only told (logged); no licence dialog; the launcher was built from resources and the user's pictures, not from the decompiled program
- A player notices: the game cannot be patched from the launcher; no licence prompt
- Where: `crates/mao-client/src/launchwindow.rs:52`; `docs/ORIGINAL_AUDIT.md:25`; `docs/CLIENT_UI.md:1517`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): Version compare with labels in the list, PLAY becomes PATCH (bitmaps 183-185) and says to patch with Funcom's launcher, an older server is refused with a line, and the anarchy.eula licence is asked until accepted (kept in launcher.ini, the client's own rule). Running AnarchyPatcher.exe is left: Funcom's servers and the game folder at stake, its arguments guessed.

#### `LCH-037` The picked character and the hall's figures cannot be turned or animated

- MISSING, high
- Original: the picked character in the list can be turned by the mouse; each hall mesh loops its animation; hall nano effect runs
- Client: not done
- A player notices: cannot rotate the character on the list screen; static hall figures
- Where: `crates/mao-client/src/hall.rs:28`; `docs/CLIENT_UI.md:1492`; `docs/ORIGINAL_AUDIT.md:26`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done, and the entry corrected: of its three points only one is in the original. The picked character is not turned by the mouse there (CharacterViewer_c::SetRot has no caller) and the login's hall does not animate its meshes (that, with the nano effect's mesh, is the character creation's module: ccscene.rs's gap). What the original does and the client did not: the picked figure plays its idle and, about one time in five, one of 23 social animations (CCCharacter_t::RunFunction); ported (net::ListFigure). Seen moving in my own test launcher's window.

#### `LCH-038` No progress window between Play and the loading screen; the loading screen goes up at the press of Play

- MISSING, medium
- Original: LoginModule_c::SlotSelectCharacter shows a progress window ("Please wait...", Show(4), ProgressWindow_c) until the character's ticket has come, then the loading screen
- Client: the progress window runs only while logging in; at Play the loading screen goes up at once (a client choice)
- A player notices: no progress window when entering a character; loading screen appears earlier
- Where: `crates/mao-client/src/launcher.rs:227`; `crates/mao-client/src/launcher.rs:341`; `docs/GUI_DLL_PARITY.md:116`; `docs/ORIGINAL_AUDIT.md:38`; `docs/CLIENT_UI.md:1042`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Ported from GUI.dll `LoginModule_c::SlotSelectCharacter` 0x10011be1 → `Show(4)` 0x1001175a: the login and the character list are put away and a `ProgressWindow_c` of `DimensionNetworkTimeout` seconds (MainPrefs.xml, 30) is shown until the character's ticket has come; its time running out (or Cancel) is `SlotSelectCharacterTimeout` 0x10011523 → `Show(0)`, the login form again. In the client: Play starts the game's process as before, but the launcher shows the progress window over the hall; the game's process writes the file named by `MAO_TICKET_UP` with the clock as soon as `login_character` has its ticket (net.rs `enter`), and the launcher then puts the loading screen up from that moment (`loading::begin_since`), the game's own window going on from the same moment (`loading::set_since` / `since`, replacing `MAO_LOADING_SINCE`). On the time-out or Cancel the game's process (started by the launcher) is ended and the login form shown. No unit test (window flow); built. Not tried live.

#### `LCH-039` Expansions' extra heads missing (head counts without the expansion flag)

- MISSING, low
- Original: the head counts per set are raised by ExpansionFlags & 2 for some sets
- Client: uses the counts without the expansion flag only
- A player notices: fewer heads offered to expansion accounts
- Where: `crates/mao-client/src/charcreate.rs:60`; `CHANGELOG.md:47`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Already ported: `charcreate::head_sets` (GUI.dll FUN_1011d692) raises the eight counts when the account's `ExpansionFlags & 2` is set, and `heads_of` passes the login's flags (`set_expansions`, from the list's trailer). The entry's Where line is out of date. Nothing changed.

#### `LCH-040` Keeper/Shade gate ignores nSLProfsEnabled

- MISSING, low
- Original: also lets them through when the client's nSLProfsEnabled is set (where it is set was not read)
- Client: only the expansion flag bit 2 counts; an account with unknown flags is let through
- A player notices: possible difference for accounts the original lets through by that setting
- Where: `crates/mao-client/src/charcreate.rs:208`; `docs/CLIENT_UI.md:4553`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Found where `nSLProfsEnabled` is set: Interfaces.dll `Client_t::ProcessMessage` 0x10002a9e, the character list (message 0xe), reads three numbers after the characters and sets `Client_t::s_nSLProfsEnabled` to whether the third is not zero (0x1000319b; the setter `SetSLProfsEnabled` 0x100011a2 has no other caller in Interfaces.dll). GUI.dll `ProfessionScene_t` 0x101216d1 switches the Keeper and Shade buttons on with `ExpansionFlags & 2` or that switch, and `SlotNextPressed` 0x10121035 refuses them only when neither is set. Now `mao_net::session::sl_professions_enabled` reads the list's third number (test in `session::tests::the_account_trailer`), `login_account` returns it with the slots and flags, the launcher keeps it (`charcreate::set_sl_professions`) and the creation's gate is `charcreate::keeper_and_shade(flags, switch)` (test `charcreate::tests::keeper_and_shade_by_the_flag_or_the_lists_switch`). The head counts still go by the flag alone, as FUN_1011d692 does. The captures have the third number as zero, so nothing changes for the accounts seen so far.

#### `LCH-041` Login error pages for 0x10 / 0x21 after the list and a lost connection not ported

- MISSING, low (notes only)
- Original: pages for a character that cannot be entered (0x10, 0x21 after the list) and for a lost connection
- Client: not ported (launcher.rs handles 0xd, 0x10, 0x21 from the login reply only)
- A player notices: those failures are not shown as the original's pages
- Where: `docs/ORIGINAL_AUDIT.md:37`; `docs/CLIENT_UI.md:1030`; `crates/mao-client/src/launcher.rs:113`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): Ported the refusals after the list: GUI.dll `LoginModule_c::SlotLoginReply` 0x10012179 sends 0x10 and 0x21 (as 0xd) to `ShowError` with their kind and code, the error's page over the login. `mao_net::session::login_character` now returns a system message in the ticket's place as `Error::Refused { kind, code }` (it was a type error without the code); the game's process writes it to the `MAO_TICKET_UP` file (`launcher::TicketNote::Refused`), and the launcher, whose progress window is up (LCH-038), waits for that process to end, puts the window away and shows the error's page (`dimensions::error_page(kind, code)`) over the login form, or the words where no page can be shown. Test: `launcher::tests::the_ticket_note_round_trips`. Left: the page for a connection lost after the list (the game's process would have to tell that apart from its other failures), not ported.

#### `LCH-042` play.sh does not search Steam libraries on other drives

- MISSING, low (notes only)
- Original: not stated
- Client: a Steam library listed in libraryfolders.vdf is not searched: pass the folder
- A player notices: Linux/Mac launch script needs the folder
- Where: `docs/PLATFORMS.md:63`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only, and nothing of the original's: play.sh is the client's own Linux/Mac starter; reading Steam's libraryfolders.vdf is a feature of the script, not a departure from the original game. Left for a platform pass.

### Launcher, login, character creation: the client has it, the original does not (9)

#### `LCH-043` An account with no characters is sent back to the form

- ADDITION, medium
- Original: not stated
- Client: sent back to the login form with "This account has no characters."
- A player notices: a player with an empty account cannot reach character creation from the list
- Where: `crates/mao-client/src/launcher.rs:351`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend REMOVE: the original goes to the character screen for every character list (GUI.dll `LoginModule_c::SlotLoginReply` 0x10012179: reply 0xe -> `Show(3)`, no count checked), so an empty account reaches New Character there; the client's "This account has no characters." on the form blocks creation for a new account.

#### `LCH-044` Character screen figure is dressed from a remembered look (looks.ini)

- ADDITION, medium
- Original: the server's character list carries no look; the original shows gear only after a first login
- Client: the live client writes the last-played clothes and gear to looks.ini whenever the look changes and dresses the figure from it; a character never played wears the base clothes (head/height/width unused)
- A player notices: list screen shows the last outfit; a first-time character shows the default body
- Where: `crates/mao-client/src/look.rs:2`; `crates/mao-client/src/launcher.rs:222`; `crates/mao-client/src/net.rs:9503`; `docs/CLIENT_UI.md:1977`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: no store of looks was found in the original (GUI.dll's login strings name only the creation meshes for `LoginWorld_c`; the list's `CharacterData_t` carries breed, sex, head and size), so the original's figure wears the list's look; the client's looks.ini only dresses the same figure in the last-seen outfit and is harmless, but it is not the original's. REMOVE if the screen must match the original exactly.

#### `LCH-045` Logging out (/camp) restarts the launcher and logs in again

- ADDITION, medium
- Original: not stated
- Client: the game ends with exit code 42 and the launcher restarts with MAO_RELOGIN, logs in at once with the account and password from its environment and shows the character list
- A player notices: back at the character list without typing the password
- Where: `crates/mao-client/src/launcher.rs:936`; `crates/mao-client/src/launcher.rs:1038`; `docs/CLIENT_UI.md:2018`; `docs/ORIGINAL_UI_PLAN.md:35`
- Decision: [x] keep  [ ] remove
- Done (2026-11-19, overnight): The original has it: GUI.dll `LoginModule_c::SlotInitialize(bool)` 0x100123ee keeps the account and password (+0x40, +0x5c) and, when called for a return from the game (its bool), calls `SlotLoginAccount` with them instead of `Show(0)`, so a camp lands on the character list without typing the password, as the client's restart with MAO_RELOGIN does. Nothing changed.

#### `LCH-046` "No heads found" message in creation

- ADDITION, low
- Original: not stated
- Client: if no heads exist for the breed it says "No heads were found for this breed in the game's files." instead of sending
- A player notices: only if game files lack heads
- Where: `crates/mao-client/src/charcreate.rs:864`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: it shows only when the game's files have none of the breed's head meshes, where the original would have offered no head and sent a creation with a head that does not exist; with a complete game it never appears.

#### `LCH-047` Character list shows free slots and offers New Character only while one is free

- ADDITION, low
- Original: AvailableCharSlots text with the slots left and in all (CharSelectWindow_c::SlotCharListReceived); whether it limits Create not stated
- Client: shows free slots, New Character only while one is left; where the account's slots are not known it writes "N characters" instead
- A player notices: a slot-count line on the list
- Where: `crates/mao-client/src/loginviews.rs:405`; `crates/mao-client/src/loginviews.rs:49`; `CHANGELOG.md:45`
- Decision: [x] keep  [ ] remove
- Done (2026-11-19, overnight): The original has it: GUI.dll `CharSelectWindow_c::SlotCharListReceived` 0x1000ebd5 enables the view at +0xbc only while the count at +0xd8 is below the slots at +0xd4, and the constructor 0x1000efb1 shows +0xbc is `create_btn` (0x1000f28d, `SlotCreatePressed`); the slot line is its `AvailableCharSlots` text. Only the "N characters" line where the slots are not known is the client's own (the original always has the trailer). Nothing changed.

#### `LCH-048` Game folder is looked for in usual install places

- ADDITION, low
- Original: not stated
- Client: looks through usual install folders (Funcom, Steam) when none was chosen, checks the folder, opens the settings if none
- A player notices: first start finds the game on its own
- Where: `crates/mao-client/src/launchwindow.rs:222`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the original runs from inside its own folder and never needs to find it; the client is installed apart from the game, so looking in the usual install places is what makes its first start work.

#### `LCH-049` Launcher has its own backgrounds (night sky, own picture) and a news-page switch

- ADDITION, low
- Original: one of the pictures of cd_image/data/launcher/skins/backgrounds over the whole window (no own-picture choice stated)
- Client: by the launcher's `background` setting: the game's pictures, a generated night sky (cloud band, 1500 twinkling stars) with the game's steel emblem keyed out of green, or a picture of the user's
- A player notices: optional different launcher look
- Where: `crates/mao-client/src/launchwindow.rs:6`; `crates/mao-client/src/launchskin.rs:1`; `docs/CLIENT_UI.md:762`; `docs/LIVE_TEST_LIST.md:395`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the default (`background = game`) is the original's own pictures; the night sky and an own picture only show when chosen. The news-page switch only leaves the page out.

#### `LCH-050` Launcher Visual page adds frame limit, shadows, monitor, presets

- ADDITION, low
- Original: not stated (100 fps is where the original stopped)
- Client: frame limit (none, 60-240) with a lower background rate (30), shadow quality, monitor choice, four presets, upscaler steps
- A player notices: extra options
- Where: `docs/CLIENT_UI.md:1462`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the Visual page's added rows are the wgpu counterparts of the original's Direct3D choices (see SET-042), which have no meaning here.

#### `LCH-051` Launcher window hidden at start, web view moved in after load

- ADDITION, low
- Original: not stated
- Client: web view made beside the window and moved into place 300 ms after it says it has loaded (or after 8 s); window shown with its first frame (or after 2 s)
- A player notices: no white flash
- Where: `crates/mao-client/src/launchwindow.rs:75`; `docs/CLIENT_UI.md:4698`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: nothing a player sees differs from the original but the missing white flash of an empty web view; it is how a WebView2 child is shown without one.

## Settings and files

### Settings and files: the client knowingly does otherwise (22)

#### `SET-001` Chat window layout kept in chatwindows.ini

- OWN-RULE, medium
- Original: each chat window is kept in the character's Chat/Windows/Window<n>/Config.xml
- Client: windows are kept in chatwindows.ini beside mao.ini; the original's files are only read as a start; "always on top/behind" is not in the original's file and is not read from it
- A player notices: chat layout is separate from the original client's
- Where: `crates/mao-client/src/chatgroups.rs:19`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original keeps each chat window in the character's `Chat/Windows/Window<n>/Config.xml` under the original client's Prefs folder; the client reads those files as its start (chatgroups.rs) but never writes into the original's folder (writing would change the installed original client's own layout, and under %APPDATA% the client's writes are not even the user's files). Writing the same XML into the client's own folder instead of chatwindows.ini would change only the file's format, for no player-visible gain; left.

#### `SET-002` Hotbars saved in hotbar.ini

- OWN-RULE, medium
- Original: Shortcut bars are kept in the original's own config files.
- Client: hotbar.ini beside mao.ini, with its own line format and a layout line.
- A player notices: Hotbar layout is not shared with the original client.
- Where: `crates/mao-client/src/hotbar.rs:273`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As SET-001: the original's shortcut bars live in its own per-character config files, which the client does not write; hotbar.ini is the client's copy. Writing the original's archive format into the client's folder would change only the format; left.

#### `SET-003` New windows start at the original client's frame, scaled; another character's layout used

- OWN-RULE, medium
- Original: the character's own frames from its own Prefs.xml (each character has its own)
- Client: a window with no place starts at the original client's frame for the character (read from its Prefs files, scaled to this window); when the original has no folder for the character, the player's most recently written character's layout is used; shipped CharPrefs.xml has no frames; AutoCloseWindows (false in prefs) not done
- A player notices: windows start near where the original had them; a new character gets another character's layout
- Where: `crates/mao-client/src/layout.rs:96`; `crates/mao-client/src/aoprefs.rs:52`; `docs/ORIGINAL_UI_PLAN.md:31`; `docs/CLIENT_UI.md:2044`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): The fallback to another character's layout is gone: the original reads a character's window frames from that character's own Prefs files (GUI.dll `ControlCenterModule_c::LoadUserConfig` 0x1006c3b1), and the shipped `CharPrefs.xml` has no frames, so a character it never played starts with every window at its view's own place. `aoprefs::prefs_dirs` now returns only the character's own `derived/Char<id>` folder (`prefs_dirs_in`, test `aoprefs::tests::a_characters_layout_is_its_own_or_none`); the chat panes follow it. The newest folder is kept only for the install's `scripts` folder (`newest_prefs_root`, used by scripts.rs when the character has no folder). Left: the client still takes the original's frames as starting places and keeps its own ini files (SET-005), and `AutoCloseWindows` is not done.

#### `SET-004` Original chat font is a setting

- OWN-RULE, medium
- Original: the original draws chat (and the UI) in its own 9 px bitmap font
- Client: a setting (original_font) switches between the bitmap font and the toolkit's font, since 9 px glyphs are small on today's screens
- A player notices: text may be drawn in another font unless the option is on
- Where: `crates/mao-client/src/ui.rs:2477`; `crates/mao-client/src/settings.rs:565`; `docs/ORIGINAL_UI_PLAN.md:99`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The rule a player meets is the original's: `original_font` is on by default (settings.rs `Settings::default`, `original_font: true`), so chat is drawn in the original's own bitmap font unless the player turns it off; the switch is the only addition (recommend keeping it for high-resolution screens). Nothing changed.

#### `SET-005` Window places, sizes and open windows kept in the client's own ini files

- OWN-RULE, medium
- Original: each window's place is kept in its `*Config` value (`Window::SaveWndConfig`); layout in DistributedValue prefs (CharPrefs.xml, Prefs.xml, *WindowConfig archives); WindowFrame is place and size together
- Client: layout.ini, sizes.ini (windows the player can size; docked/dialog windows keep no size), tabs.ini, settings.ini and open.ini (the windows open at the last run, restored at start) beside the client's settings; the wear grids' labels_on_top is not kept
- A player notices: window places, sizes and open windows come back at start, but do not round-trip with the original client
- Where: `crates/mao-client/src/layout.rs:353`; `crates/mao-client/src/layout.rs:5`; `docs/CLIENT_UI.md:1824`; `docs/CLIENT_UI.md:1949`; `docs/GUI_DLL_PARITY.md:59`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As SET-001: the original keeps window places in its `*Config` values inside the character's own Prefs files, which the client reads as starting places (SET-003, `aoprefs.rs`) but does not write; its own ini files hold what it changes. Round-tripping with the original would mean writing into the installed original client's folder; left.

#### `SET-006` Backpack names kept in backpacks.ini; rename by right click on empty space

- OWN-RULE, low
- Original: The window's menu (FUN_100cbace) has Rename; the dialog (FUN_100cb852) saves container_name in the bag's own file Containers/Container_%ux%u.xml.
- Client: A right click on empty space of an own backpack window opens a one-entry menu; names are kept in backpacks.ini beside mao.ini; the reset button is labelled "Its own name" by default.
- A player notices: Bag names live in a different file and are renamed from a different gesture.
- Where: `crates/mao-client/src/loot.rs:14`; `docs/CLIENT_UI.md:1793`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original renames from its backpack window's own menu (FUN_100cbace's Rename, dialog FUN_100cb852) and keeps the name in the bag's `Containers/Container_%ux%u.xml`; the client's backpack windows have no window-menu button yet, so the right click stands in, and the original's files are not written (SET-001). Porting needs the window menu of the container windows.

#### `SET-007` Default key map replaced; key table version resets custom keys

- OWN-RULE, low
- Original: keymap from Prefs.xml KeyBindings
- Client: the keymap is the original's defaults; a settings file written for another key-table version (KEYMAP_VERSION) has its [keys] replaced by the defaults
- A player notices: custom old bindings lost once when the table version changes
- Where: `docs/CLIENT_UI.md:2013`; `crates/mao-client/src/settings.rs:286`; `crates/mao-client/src/settings.rs:288`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The key-table version is how the client brings its own mao.ini up to date; the original's keys come from its Prefs.xml archive, which the client reads (`keybindings.rs`) but does not write. A player's own changes made in the original are still taken over each session. Nothing to port.

#### `SET-008` Folders page shows this client's folders

- OWN-RULE, low
- Original: Folders: skins, scripts, screenshots
- Client: settings, screenshots and chat logs folders plus the game's folder
- A player notices: different folder list
- Where: `crates/mao-client/src/launchwindow.rs:31`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's Folders page lists its skins, scripts and screenshots folders, which are inside the original's install; the client's own files live elsewhere, so its page lists those. The scripts folder of the original is still read (scripts.rs).

#### `SET-009` Friends window open folders kept in tabs.ini

- OWN-RULE, low
- Original: FriendsWindowConfig (is_chat_window_list_open, is_online_list_open and others)
- Client: a line of tabs.ini
- A player notices: friends folder state is not shared with the original
- Where: `crates/mao-client/src/friends.rs:4`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As SET-001: the state is the original's `FriendsWindowConfig` values, kept in the client's own file because the original's Prefs files are not written by this client.

#### `SET-010` Hidden nano programs kept in hiddennanos.ini

- OWN-RULE, low
- Original: HideNano moves a program to list 6 (kept in the original's own config).
- Client: The hidden set is stored in the client's own file hiddennanos.ini beside mao.ini.
- A player notices: Hidden programs are kept per client install, not in the original's files.
- Where: `crates/mao-client/src/nanoprograms.rs:166`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As SET-001: `HideNano`'s list 6 is the original's rule; only where the hidden set is kept differs.

#### `SET-011` Knowledge window board kept in knowledge.ini

- OWN-RULE, low
- Original: The board is saved in KnowledgeViewConfig (item_id and item_pos per item) as the window closes and read back as it opens (FUN_100cf66a, FUN_100cf779).
- Client: Keeps it in knowledge.ini beside layout.ini, one line (low high quality x y name), written as soon as it changes; an entry read back shows only its name until the inventory lends a picture again.
- A player notices: Boards read back from the file may show names instead of pictures at first.
- Where: `crates/mao-client/src/knowledge.rs:12`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As SET-001 for the file; the original reads the board back as it opens with each item's id and place (FUN_100cf66a, FUN_100cf779), and the client keeps the same fields. The names-before-pictures moment is the client's picture loading, not a rule.

#### `SET-012` Land-control button state in automap.ini

- OWN-RULE, low
- Original: state archived as RenderLandControlData (FUN_100ebfdc)
- Client: kept in `automap.ini`
- A player notices: stored elsewhere than original
- Where: `crates/mao-client/src/automap.rs:232`; `docs/CAPTURE_GAPS.md:217`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As SET-001: `RenderLandControlData` is the original's state, kept in the client's own file.

#### `SET-013` Launcher dimension list copy in the client's folder, own wording

- OWN-RULE, low
- Original: the launcher keeps a copy (dimensions.txt) and says "Could not retrieve dimension information. Using cached data."
- Client: keeps dimensions.txt beside the client's settings; says "The dimension server did not answer: this is the list kept from before."
- A player notices: different wording and file place
- Where: `crates/mao-client/src/launchwindow.rs:769`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The cache is the original's (dimensions.txt); the wording must be the client's own by the user's rule (no literal strings from the original's program), and the file is beside the client's settings because the game folder is not written.

#### `SET-014` Launcher keeps its own launcher.ini with clamped time-outs

- OWN-RULE, low
- Original: Client-to-Server and Dimension-check time-outs in the original's own storage
- Client: launcher.ini holds the dimension chosen last, the time-outs (clamped to 5-600 s and 2-300 s), the background and the page switch
- A player notices: extra file beside the settings
- Where: `crates/mao-client/src/launchwindow.rs:381`; `crates/mao-client/src/dimensions.rs:104`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The time-outs are the original launcher's Connection page values; whether its spin boxes clamp at 5-600 and 2-300 was not read in Anarchy.exe's dialog template (resource 161). Left until the template is read.

#### `SET-015` Macros saved beside the client's settings, one for all characters

- OWN-RULE, low
- Original: TextMacro.bin is kept in the character's preferences folder (Init 0x100236ec)
- Client: the file is the client's own, beside its settings (one for all characters)
- A player notices: macros are not shared with the original client or per character
- Where: `crates/mao-client/src/macros.rs:8`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original keeps TextMacro.bin per character in its preferences folder (`Init` 0x100236ec); making the client's macros per character means keying macros.rs's file by the character id at load and save, a storage change of SET-001's kind that also loses players' current macros unless migrated. Left for a storage pass.

#### `SET-016` Option panel "Fixed keys" shows the client's own keys

- OWN-RULE, low
- Original: each fixed hot key is shown with the key the command is on (`InputConfig_t::GetHotkey`).
- Client: the key is this client's own for the same command; a command it has no key for shows none.
- A player notices: fixed-key list shows the client's bindings.
- Where: `crates/mao-client/src/gui/options.rs:18`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The rule is the original's: each fixed key is shown with the key the command is on now (`InputConfig_t::GetHotkey`), which here is the client's binding for the same command; and since SET-026 the client's default fixed keys are GUI.dll's own key table, so an unchanged client shows the original's keys. Nothing changed.

#### `SET-017` Option panel changes saved to prefs.xml; archives not kept

- OWN-RULE, low
- Original: preferences in the Funcom Prefs folders; DistributedValue prefs include `<Archive>` entries and the user's Prefs.xml
- Client: changes saved to prefs.xml beside mao.ini; `<Archive>` entries not kept
- A player notices: preferences and window archives are not shared with the original
- Where: `docs/CLIENT_UI.md:2650`; `crates/mao-client/src/gui/dvalue.rs:7`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): As SET-001: the option panel's values go to the client's own prefs.xml because the original's Prefs folders are not written; the `<Archive>` entries are the windows' configs, kept in the client's own ini files (SET-005).

#### `SET-018` Remembered looks cache in looks.ini

- OWN-RULE, low
- Original: shows gear only after a first login (as the client does)
- Client: own looks.ini cache, written by the live client (its writing path was never run live per the note)
- A player notices: stored looks may not appear
- Where: `crates/mao-client/src/look.rs:2`; `docs/CLIENT_UI.md:1979`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The looks cache is LCH-044's matter (an addition, recommendation there).

#### `SET-019` Screenshot capture off Windows renders the scene offscreen

- OWN-RULE, low
- Original: not stated
- Client: on Windows the window as shown (`xcap`), web page included; on Linux/macOS the scene and HUD rendered offscreen (also the Windows fallback)
- A player notices: screenshots on those systems lack the web page
- Where: `crates/mao-client/src/screenshot.rs:12`; `docs/PLATFORMS.md:16`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Platform matter: on Windows the picture is the window as shown, as the original's; elsewhere there is no window capture to take, so the scene is drawn offscreen.

#### `SET-020` Settings kept under the client's own folder in mao.ini

- OWN-RULE, low
- Original: Prefs under %LOCALAPPDATA%/Funcom/Anarchy Online (not stated for settings, hotbars, crash.log, screenshots, chat logs)
- Client: `mao.ini` in the working directory or under `%APPDATA%\ModernAnarchyOnline` (Linux `$XDG_CONFIG_HOME/ModernAnarchyOnline`, else `~/.config/ModernAnarchyOnline`), `MAO_SETTINGS` override, written with defaults when missing; `--window-mode`, `--width`, `--height` override; values clamped on read (e.g. width 320..16384, render scale 0.4..1)
- A player notices: settings are not in the original's preference folders
- Where: `crates/mao-client/src/settings.rs:790`; `crates/mao-client/src/settings.rs:726`; `docs/PLATFORMS.md:19`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The client's settings cannot live in the original's Prefs folders without writing into the installed original client's files; the clamps keep a hand-edited file from making the window unusable.

#### `SET-021` Tip settings in one tips.ini for all characters

- OWN-RULE, low
- Original: switched-off tips are kept per character (DisabledTipsMap.xml) and IsTipEnabled with the preferences
- Client: both are in tips.ini beside the settings
- A player notices: disabled tips are shared by all characters and not shared with the original
- Where: `crates/mao-client/src/tips.rs:19`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original keeps switched-off tips per character (DisabledTipsMap.xml); making tips.ini per character means keying it by the character id at load and save (tips.rs) and migrating today's file; a storage change of SET-015's kind, left for that pass.

#### `SET-022` Tradeskill window key Shift+T is the user's word

- OWN-RULE, low
- Original: not stated
- Client: the key is the user's word for the original's (2026-11-12); the action's name is the client's.
- A player notices: Shift+T opens the tradeskill window.
- Where: `crates/mao-client/src/settings.rs:230`; `docs/CLIENT_UI.md:3341`; `crates/mao-client/src/settings.rs:231`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The key is the original's own default: the game's shipped `cd_image/gui/Default/CharPrefs.xml` binds provider 2073189714 (`WINDOW_TS`, `keybindings::provider`) to input 131173, which is Shift (0x20000) + key 101, T (`keybindings::binding`). Nothing changed; the user's word was right.

### Settings and files: an approximation or a stand-in (3)

#### `SET-023` Only 20 windows' original frames mapped

- SIMPLIFIED, low
- Original: every window keeps its own `*Config` archive.
- Client: maps 20 config names to its windows (NAMES) and the chat startup window to "Chat"; others start at the client's own place; rollup pages have no frame.
- A player notices: windows not in the list start elsewhere.
- Where: `crates/mao-client/src/aoprefs.rs:28`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The 20 names are the windows the client has a counterpart for and that the original's Prefs files were seen to hold (`aoprefs::NAMES`); the original's other `*Config` archives belong to windows the client draws differently (rollups, docked bars) or not at all. Mapping more needs each window's original name checked against a real Prefs file; left.

#### `SET-024` Screenshot file names fall back to UTC off Windows

- SIMPLIFIED, low
- Original: not stated
- Client: local time when the `time` crate can read the offset; on Linux and macOS the name falls back to the UTC stamp
- A player notices: screenshot names in UTC on those systems
- Where: `crates/mao-client/src/screenshot.rs:5`; `docs/PLATFORMS.md:20`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original is a Windows client whose names use the local time; on Windows the client does the same. Off Windows the `time` crate refuses to read the local offset in a program with threads (a soundness rule of the crate), so the UTC stamp is the fallback; a port would need another way to read the zone (libc `localtime_r`), a platform matter outside the original.

#### `SET-025` Stats window bars kept in tabs.ini

- SIMPLIFIED, low
- Original: StatViewConfig's AIXPBar, XPBar, DuelScoreBar, TeamScoreBar and SoloScoreBar are ticked in a menu (FUN_1007fe87), kept in the player's config files.
- Client: The right-button menu ticks them and they are kept in tabs.ini.
- A player notices: The choice is stored in the client's own file.
- Where: `crates/mao-client/src/stats.rs:10`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The bars and their menu are the original's (FUN_1007fe87, `StatViewConfig`'s AIXPBar, XPBar, DuelScoreBar, TeamScoreBar, SoloScoreBar); only where the ticks are kept differs, as for SET-001: the original's config files are its own client's and are not written by this one.

### Settings and files: the original was not read: inferred or measured (3)

#### `SET-026` Some default keys are assumed, not read from the original

- GUESS, medium
- Original: the default keys are stored as hashed provider ids in a player's Prefs.xml (hash not decoded); not recorded in its data files
- Client: uses defaults the players know, marked "assumed" where they could not be checked
- A player notices: some default keys may differ from the original client's
- Where: `crates/mao-client/src/settings.rs:20`; `docs/CLIENT_UI.md:38`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The defaults are no longer assumed: the rebindable keys were already the shipped `CharPrefs.xml`'s (`keybindings.rs`, the provider hash decoded, 2026-11-18), and the fixed keys (camera, targets, chat, coordinates, perk/vehicle windows) are in GUI.dll's built-in key table, the text at 0x101aab28 that `ParseFile` 0x1001b058 reads. Compared line by line, every client default matched it but two: `COMMAND_TEXT_INPUT_START_TEXT_INPUT` is also on Shift+Enter (and Alt+Enter, left to the client's window-mode key, see SET-044) and `COMMAND_START_CHAT_CMD_INPUT` also on NUMPAD_DIV; both added as defaults (`chat_input = Enter, Shift+Enter`, `chat_command = Slash, NumpadDivide`). The head comment of settings.rs now names the sources. Test: `settings::tests::defaults_parse_and_resolve` (13 settings tests pass). Left: an existing mao.ini (key table version 3) keeps the chat keys it has written; only new files and reset keys get the second keys. Not ported from the table: Shift+F9 (extended coordinates), Ctrl+F10 (camp to login), Shift+| (control center), Shift+C (chat config): the client has no such actions.

#### `SET-027` Chat filter rules kept in chatfilter.ini

- GUESS, low
- Original: rules are texts under the ChatFilterRules value; behaviour with a pattern that does not compile is not stated
- Client: rules stored in chatfilter.ini; a pattern that does not compile matches nothing
- A player notices: rules are not shared with the original client
- Where: `crates/mao-client/src/chatfilter.rs:15`; `docs/GUI_DLL_PARITY.md:157`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The rules are the original's `ChatFilterRules` texts; how the original treats a pattern that does not compile was not found (its filter's matcher is not read), and keeping them in chatfilter.ini instead of the original's prefs is SET-001's matter. Left until the matcher is read.

#### `SET-028` Ignore list kept in the client's own ignore.ini

- GUESS, low
- Original: not stated how the original keeps or announces ignoring
- Client: the list (lower-case names) is stored in ignore.ini beside mao.ini; hides tells and channel lines; says "X is ignored: their tells and channel lines are hidden."
- A player notices: ignoring is local to this client and not shared with the original
- Where: `crates/mao-client/src/chatwindow.rs:267`; `docs/CLIENT_UI.md:1834`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-19, overnight): Read the original's `/ignore` (GUI.dll FUN_100bafeb, registered by FUN_100bb3e0; `IgnoreSystem_t` keeps the list in `IgnoreList.bin` by character identity): more than one word is `ChatCmdFeedback_Ignore_ToManyArguments`; `list` prints a line of its own and one per ignored character; a number is a character's id; no word takes the selected target, else `ChatCmdFeedback_Ignore_NoTargetOrNick`; a target that is not a character is `CanOnlyIgnoreCharacters`; the own character `CantIgnoreYourself`; otherwise it toggles and says `IgnoringCharacter` / `UnignoringCharacter` with the name (text.mdb 10001, read at run time). Ported: `chatwindow::ignore_request` (the words: too many, `list`, a name, the target or nobody; test `chatwindow::tests::ignore_takes_one_name_the_list_or_the_target`) and `toggle_ignore`'s texts and the own-character refusal (test `an_ignored_character_is_not_shown_and_the_command_toggles` updated). Left: ignoring by character id (the client's list is by name, and the window has no ids), the target's kind check, and keeping the list in the original's `IgnoreList.bin` (SET-001's matter).

### Settings and files: a guard or fallback the original lacks (1)

#### `SET-029` Layout file writes throttled

- SAFETY, low
- Original: `Window::SaveWndConfig` writes on close.
- Client: places written when something moved, at most every two seconds, and a size is noted only if sensible (40 to 8000).
- A player notices: none visible
- Where: `crates/mao-client/src/layout.rs:287`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: writing places at most every two seconds only spaces the client's own file writes (the original writes once on close); what a player sees and gets back is the same. The 40-8000 check only keeps a broken file from making a window unusable.

### Settings and files: the original does it, the client does not (8)

#### `SET-030` Option panel: many preferences kept but not acted on

- MISSING, high
- Original: every setting acts (options for view distance, fog mode, water, clouds, wildlife, names, character fading)
- Client: effect switches, shadows, mouse look and sound act; view distance, fog mode, water, clouds, wildlife, names and character fading are "not used yet" (no use found in code beyond the stored value); pages come from an AOU skin's Root.xml since the game's is empty
- A player notices: those option controls do nothing
- Where: `docs/GUI_DLL_PARITY.md:108`; `docs/CLIENT_UI.md:2658`; `crates/mao-client/src/prefs.rs:1`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done with what was left, each read in N3.dll or DisplaySystem.dll by disassembly: ZoomTo1stPerson (gates both ways of the first-person switch), MouseWheel (0 zooms, 2 does nothing), ShowMyCharacter (the own body hidden in first person unless on), the FadeCharacter four (the own body fades as the camera nears its head), and the ViewDistance and DisplayCharViewDistance sliders, which set this client's own two distances. Not done: MouseWheel 1, the control center's side bar scrolled by the wheel. Kept as the client's own: its default of drawing every character (the original's is 80 m) and its view distance's wider range. Not seen running.
- Done in part (2026-11-18): Acted on now, each confirmed in the decompile or the drop-in ports: FogMode (0 = a tenth of the fog), ToggleAllEffects (sets thirteen preferences), RealisticClouds/SimpleClouds (both off: no single clouds), WaitForVertSync (sets the client's vsync), the seven VisualFlags boxes (a fault: each overwrote the whole number; shown and kept per bit now, still not sent), esc_raid / esc_lft / esc_cityterminal, and esc_chests / esc_backpacks put right. Left because the code that reads them is in none of the decompiles at hand: ViewDistance, DisplayCharViewDistance, ZoomTo1stPerson, MouseWheel. Left as ports of their own: FadeCharacter*, ShowMyCharacter. Nothing to switch here: RealisticWater, Wildlife, IsSpaceShipsShown, SmoothAnimations.

#### `SET-031` Per-character Prefs.xml key bindings not read

- MISSING, high
- Original: InputConfig_t/hotkey_config and the KeyBindings archive per character.
- Client: default keymap in settings::ACTIONS; per-character Prefs.xml not read.
- A player notices: keys rebound in the original client do not carry over.
- Where: `docs/GUI_DLL_PARITY.md:58`; `crates/mao-client/src/gui/dvalue.rs:7`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): the character's `KeyBindings` archive (`<Prefs>/<user>/Char<id>/Prefs.xml`) is read at the game's start and the keys it changed from the shipped default are laid over the client's for the session (`keybindings.rs`), unless the key was changed here; never saved, the original's folder only read. No character on this machine has rebound a key there, so nothing changes yet. Found: the client's defaults differ from the shipped file in five keys (walk/run on Backspace there; sneak attack J, fast attack N, aimed shot O, bow special Period have no key here; O is the map overlay here): changed the same day at the user's word ("match the original"; a settings file's untouched old defaults are carried over, the map overlay moves to Ctrl+P). Not read: the fixed keys of `Login.cfg`, mouse buttons, actions the client has none for.

#### `SET-032` Original's key editor not ported; Key bindings tab leads to the client's own settings

- MISSING, medium
- Original: FUN_100bcfb3 is the key editor of every key.
- Client: not ported; the tab says "This client's keys are set in its own settings window" with a "Client settings" button.
- A player notices: keys cannot be rebound in the original's editor.
- Where: `crates/mao-client/src/gui/options.rs:442`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's key editor is `BindView_c` (GUI.dll FUN_100bcfb3, 15 kB of decompile): a `MultiListView_c` with Category / Function / key columns, bind and clear buttons (FUN_100bd8ff, FUN_100bd94e, FUN_100bc713) and key capture, writing the per-character `KeyBindings` archive. Porting it needs the multi-column list widget, the key capture and writing the original's `Prefs.xml` archive (the client never writes the original's folder: `keybindings.rs`), well past the 40 minutes; the client's own key list in its settings window rebinds every key meanwhile.

#### `SET-033` "Automatically report disconnects to Funcom" setting gone

- MISSING, low
- Original: "Automatically report disconnects to Funcom" setting
- Client: gone
- A player notices: no such option
- Where: `crates/mao-client/src/launchwindow.rs:33`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The setting only switched the original's reports of disconnects to Funcom's own servers; the client sends nothing to Funcom (the user's rule: no Funcom services but the game's), so there is nothing for the box to switch.

#### `SET-034` Criteria context `id:` not ported

- MISSING, low
- Original: criteria context (anarchyonline.exe FUN_00408268) includes `id:` and `module:`; the user's Prefs.xml.
- Client: `module:` now handled as a distributed value; no `id:` context in the list; user's Prefs.xml is the client's own prefs.xml
- A player notices: some XML criteria with `id:` evaluate differently.
- Where: `crates/mao-client/src/gui/dvalue.rs:8`; `crates/mao-client/src/gui/dvalue.rs:220`; `docs/GUI_DLL_PARITY.md:73`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Nothing a player can meet is missing: the context's `id:<name>` (anarchyonline.exe FUN_00408268: `DynamicID_t::GetID(name, true)` as an Int32) is used in the game's GUI files only by the action menus' `bgicon="id:GFX_..."` pictures (LeftMenu.xml and RightMenu.xml, 6 each; no `criteria` or `activate_criteria` anywhere has `id:`), and those pictures are already read by name in `gui/menu.rs`. A criteria with `id:` would need DynamicID's numbering, which only matters when compared with another such number; none exists. Nothing changed.

#### `SET-035` Equipment lock: only by /setoption, single-tab locks not done

- MISSING, low
- Original: LockEquipment value (false in CharPrefs.xml); a list of locked tabs `EquipmentLocks`
- Client: LockEquipment is read from the settings store and acts (the wear move is refused with Feedback_EquipmentLocked); set with /setoption LockEquipment true; the wear window has no tab locks (EquipmentLocks)
- A player notices: lock only by command; no per-tab locks
- Where: `crates/mao-client/src/net.rs:1007`; `crates/mao-client/src/net.rs:1014`; `crates/mao-client/src/ui.rs:1232`; `docs/CLIENT_UI.md:4311`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Read: the locks are per worn slot, not per tab. Gamecode FUN_100266c9 (asked by `N3Msg_IsItemPossibleToWear` / `ToUnWear`, `Feedback_EquipmentLocked`) says locked when `LockEquipment` is true, else when the slot's identity is among the `Locks` identities of the message value `EquipmentLocks`; GUI.dll FUN_100ca3ae toggles one slot's identity in that list (from a view's identity at +0x130; it is an action of the item context menu built by FUN_100caf07), and FUN_100e1f9b unlocks every slot of the four wear pages (types 0x65, 0x66, 0x67, 0x73, 16 slots each) and empties the list. Porting needs the item menu's lock entry (its label and which views offer it were not read), keeping `EquipmentLocks` per character, and the check in the wear moves: more than this night's 40 minutes, and the menu entry's text and conditions are still unread.

#### `SET-036` List view column widths and sort not kept

- MISSING, low
- Original: list views keep column widths and sort (listview_config)
- Client: not done (mission log, nano programs, NCU lists)
- A player notices: columns reset each session
- Where: `docs/CLIENT_UI.md:3136`; `crates/mao-client/src/missionlog.rs:29`; `crates/mao-client/src/nanoprograms.rs:168`; `crates/mao-client/src/ncu.rs:181`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): widths and the list's sort are kept in lists.ini for the NCU windows, the nano programs and the mission journal (layout::ListConfig, multilist::Keeper).

#### `SET-037` Texture quality, compressed textures and colour depth settings dropped

- MISSING, low
- Original: Texture Quality, Compressed Textures, Texture Color Depth settings
- Client: gone (textures are always drawn whole); a different "Compressed textures" box keeps them compressed on the graphics card
- A player notices: no texture quality choice
- Where: `crates/mao-client/src/launchwindow.rs:28`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's Texture Quality, Compressed Textures and Texture Color Depth chose Direct3D 7 texture formats and mip levels on the card; wgpu has no 16-bit texture formats to choose and the client draws every texture whole (its own "Compressed textures" box keeps them BC-compressed on the card). A port would be a texture-downscaling option of the client's own, not the original's rule.

### Settings and files: the client has it, the original does not (17)

#### `SET-038` Character draw distance and auto-loot settings

- ADDITION, medium
- Original: not stated
- Client: a setting hides characters farther than a chosen distance (0 draws all); another takes everything from a corpse when it opens
- A player notices: far characters can be hidden; fewer clicks at corpses
- Where: `crates/mao-client/src/net.rs:9277`; `crates/mao-client/src/viewer.rs:102`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the character draw distance is the original's own (`/chardist`, registered by GUI.dll FUN_100bb3e0), only reached here also from a setting; auto-loot is not in the original (no such string in GUI.dll or Gamecode.dll) but is off by default (`auto_loot: false`), so a player who does not turn it on sees the original's corpse window.

#### `SET-039` Frame limit, background limit and vsync choice

- ADDITION, medium
- Original: the original stopped at 100 fps
- Client: a frame limit, a lower one while the window is not in front, and a vsync choice (Immediate/Mailbox/Fifo); pacing sleeps until 2 ms before then waits awake (`LIMIT_SPIN_MICROS`)
- A player notices: the player can cap or uncap the frame rate and save power in the background; the limiter uses some CPU
- Where: `crates/mao-client/src/viewer.rs:1133`; `crates/mao-client/src/settings.rs:590`; `docs/LIVE_TEST_LIST.md:442`; `docs/CLIENT_UI.md:1228`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the original's only frame control is its fixed cap and the `/framerate` counter (`FramerateView_t`); a limit, a background limit and the vsync choice are what a wgpu client on today's monitors needs (unlimited by default, as the old cap has no meaning at 144 Hz).

#### `SET-040` Interface size setting

- ADDITION, medium
- Original: not stated
- Client: a UI scale 0.6-2.0 (slider, Ctrl+= / Ctrl+- keys) multiplies the window's own scale; the slider's value is taken when it is let go (a typed value at once)
- A player notices: the whole interface can be made larger or smaller
- Where: `crates/mao-client/src/ui.rs:1045`; `crates/mao-client/src/ui.rs:549`; `docs/CLIENT_UI.md:1961`; `docs/CLIENT_UI.md:1866`; `docs/CLIENT_UI.md:1498`; `crates/mao-client/src/settings.rs:127`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the original has no interface scale (no scale string in GUI.dll), and at today's resolutions its 1024x768-sized windows are unreadably small without it; at 1.0 (the default) nothing differs from the original.

#### `SET-041` Item finder files of what was seen

- ADDITION, medium
- Original: Not in the original.
- Client: Writes a plain-text file per character and dimension in an items folder beside the settings, with sections for worn, inventory, each bag and the bank and when each was last seen; a bag's contents are only as they were when last opened, and the bank's as when a bank terminal was last used.
- A player notices: Search results can be out of date, and cannot see bags never opened, a bag inside the bank, or a character never played with this client.
- Where: `crates/mao-client/src/itemindex.rs:5`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the item finder is one of the client's own tools the user asked for (memory "MAO own tools"); its files are only written beside the client's settings and change nothing the original does.

#### `SET-042` Own graphics and display options (presets, scale, FSR, shadows, monitor, distances)

- ADDITION, medium
- Original: not present (the original's settings are Direct3D era; options are the skin's option panel)
- Client: own options window and mao.ini keys: window mode, size, monitor, graphics adapter, Low/Medium/High/Ultra presets, anti-aliasing (MSAA/TAA), shadows, texture compression, drawing scale with FSR (0.4-1.0) and sharpening, view and character distance, VSync, frame limits, performance overlay, auto-loot, interface size, theme, skin folder, key rebinding, Quit game; some wait for a zone or the launcher
- A player notices: many settings (and a rebindable key list) with no counterpart in the original
- Where: `crates/mao-client/src/ui.rs:2362`; `crates/mao-client/src/launchwindow.rs:269`; `crates/mao-client/src/settings.rs:515`; `docs/CLIENT_UI.md:1359`; `docs/CLIENT_UI.md:78`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the original's graphics settings are Direct3D 7 device, colour depth and texture choices that have no meaning for wgpu (LCH-002); the client's options are the counterparts a modern renderer needs, and the skin's option panel is still there.

#### `SET-043` Themes, skin folders and the built-in palette

- ADDITION, medium
- Original: one skin from `Gui/<skin>` with the game's GUIColors.xml.
- Client: built-in dark warm silver-and-gold palette, themes `original`, `aou_translucent`, `aou_cyt`, `aou_slim_cni`, `aou_slim_cnt` (bundled), or any folder; skin folder stack (gui_dir, theme folder, game default); `MAO_THEME`; choosable in the options
- A player notices: the interface can be themed from a setting.
- Where: `crates/mao-client/src/theme.rs:78`; `crates/mao-client/src/viewer.rs:3063`; `docs/CLIENT_UI.md:2024`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the default theme is `original` (the game's own skin and GUIColors.xml); the AOU themes and other folders only apply when chosen.

#### `SET-044` Window modes with F11 / Alt+Enter and a chosen monitor

- ADDITION, medium
- Original: not stated
- Client: windowed, borderless or exclusive full screen at the monitor's best mode, switched with mao.ini, F11, Alt+Enter or the options window; chosen monitor; falls back to borderless where there is no exclusive mode
- A player notices: own window-mode controls
- Where: `crates/mao-client/src/viewer.rs:2`; `docs/CLIENT_UI.md:13`; `crates/mao-client/src/settings.rs:280`; `README.md:48`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP, but note: Alt+Enter is one of the original's three chat keys (GUI.dll's key table at 0x101aab28: `COMMAND_TEXT_INPUT_START_TEXT_INPUT` on Enter, Shift+Enter, Alt+Enter), so the window-mode switch takes a key from the original; F11 and the options are free. Moving the switch to F11 alone would give Alt+Enter back to the chat.

#### `SET-045` Account file with last-played character first

- ADDITION, low
- Original: keeps a username list; a played-last ordering of the character list is not stated
- Client: account.ini stores the account, the character played last (listed first and pre-selected) and the other accounts; only the name is written, never the password
- A player notices: the character list starts with the last-played character selected
- Where: `crates/mao-client/src/launcher.rs:103`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP the pre-selection, which is the original's own (`CharSelectWindow_c::SlotCharListReceived` 0x1000ebd5 selects the row of the value `SelectedCharacter`, and the account list is `LauncherConfig`'s `Account%d` / `SelectedAccount`); REMOVE only the moving of that character to the top of the list, which the original does not do (it keeps the server's order).

#### `SET-046` AutoAFK controlled by /setoption

- ADDITION, low
- Original: no control for AutoAFK in the option panel (it is in LoginPrefs.xml)
- Client: /setoption AutoAFK off sets it
- A player notices: you can turn going away by itself on or off in game
- Where: `crates/mao-client/src/afk.rs:29`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: `/setoption` is the original's own command for setting any distributed value, and `AutoAFK` is one (LoginPrefs.xml), so the original's `/setoption AutoAFK false` does the same.

#### `SET-047` Client brings its own PlanetMap folder

- ADDITION, low
- Original: Reads index files under the game's textures/PlanetMap.
- Client: Looks first in a PlanetMap folder beside mao.ini (aftermarket maps the client brings), then in the game's cd_image/textures/PlanetMap.
- A player notices: Extra planet maps can be supplied without touching the game folder.
- Where: `crates/mao-client/src/planetmaps.rs:3`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the game's own PlanetMap folder is still read; the client's folder only adds maps (memory: they reached the real folder 2026-11-15) without writing into the game's folder.

#### `SET-048` Crash log, run markers and error box

- ADDITION, low
- Original: not stated
- Client: writes crash.log and run-<part>.txt markers, notices a run that ended without a clean exit, and shows a system message box with the panic message (packaged build, or by MAO_ERROR_BOX)
- A player notices: an error box when the launcher or login fails
- Where: `crates/mao-client/src/crash.rs:1`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the crash log, run markers and error box only show where the client fails, which is where the original would have shown its own crash dialog (BugReport_t); they change nothing in a run that works.

#### `SET-049` Extra key actions of the client

- ADDITION, low
- Original: not stated
- Client: NoClip (debug aid), CycleWindowMode (Alt+Enter), ToggleFullscreen (F11), ToggleOptions (F10), Quit (Ctrl+Q), MapOverlay (O), whompah, mail and help window keys (empty by default), plus its own movement actions
- A player notices: extra key commands exist.
- Where: `crates/mao-client/src/settings.rs:278`; `docs/CLIENT_UI.md:25`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP, with one note: Alt+Enter (CycleWindowMode) is one of the original's chat-input keys in GUI.dll's key table (see SET-044); the other extra actions use keys the original leaves free or have no key.

#### `SET-050` Game folder is a setting with Browse picker and auto-detection

- ADDITION, low
- Original: not stated
- Client: Game folder row in settings (Browse via rfd, Open, check), detection of usual install places, launcher starts without a folder; changing folder restarts the launcher
- A player notices: extra setup flow
- Where: `crates/mao-client/src/launchwindow.rs:587`; `docs/CLIENT_UI.md:1505`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the client lives apart from the game's folder (LCH-048), so it needs to be told or to find where the game is.

#### `SET-051` Game log written to game-log.txt

- ADDITION, low
- Original: not stated
- Client: game-log.txt beside the settings (previous run kept as game-log.previous.txt); log_messages = on adds the server's messages ("message log: on", "server n3" lines)
- A player notices: extra log files appear beside the settings
- Where: `crates/mao-client/src/launcher.rs:318`; `docs/CLIENT_UI.md:3501`; `docs/LIVE_TEST_LIST.md:372`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: a log file beside the settings that a player never has to look at; it carries nothing of the account.

#### `SET-052` Option panel folder list uses egui widgets with a "Client settings" link

- ADDITION, low
- Original: a StringListView of folders and pages (FUN_100c3d62).
- Client: egui collapsing headers and selectable labels, plus a "Client settings" link for window mode, resolution, keys and theme.
- A player notices: the panel has an extra link to the client's own settings.
- Where: `crates/mao-client/src/gui/options.rs:482`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the folder list is the original's (its folders and pages from the option panel files); the "Client settings" link is the only way from the original's panel to the client's own window, keys and theme.

#### `SET-053` Pet window, planet map and camera settings in mao.ini

- ADDITION, low
- Original: `PetWindowConfig` keeps ShowButtons/EnableFixedPetButtons; PlanetMapIndexFile/ShadowlandMapIndexFile; PreferredCameraMode.
- Client: kept in mao.ini (`pet_buttons`, `fixed_pet_buttons`, `planet_map`, `shadowlands_map`, `camera_mode` free/rubber/lock).
- A player notices: those settings are in the client's file.
- Where: `crates/mao-client/src/settings.rs:568`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the settings themselves are the original's (`PetWindowConfig`, `PlanetMapIndexFile`, `PreferredCameraMode`); only where they are kept differs, as for every other value the client keeps (the original's Prefs files are not written by this client).

#### `SET-054` Tradeskill guide keeps a record of what the window said

- ADDITION, low
- Original: The tradeskill window shows each server answer once and keeps nothing.
- Client: Every result, quality range and skill requirement shown for a pair of items is written to tradeskills.txt and shown in the guide under "What the tradeskill window told you".
- A player notices: A player-made recipe table grows with use and is kept between sessions.
- Where: `crates/mao-client/src/craftguide.rs:14`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the tradeskill guide is one of the client's own tools; the window itself still shows each answer as the original does.

## Network and protocol

### Network and protocol: an approximation or a stand-in (2)

#### `NET-001` Character creation logs in on a new connection per request

- SIMPLIFIED, low
- Original: not stated (the original's creation module stays on its login connection)
- Client: every suggest/create request logs in again on a new connection
- A player notices: a short wait per request
- Where: `crates/mao-client/src/charcreate.rs:182`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's creation runs on the login connection it already has (the character list's), but the client's launcher closes that connection as soon as the list is read, because the game runs in a process of its own that must make its own login (LCH-001; "the login connection must be gone before the game's own login", launcher.rs `log_in`). Keeping one connection open through the creation means holding it across the launcher's stages and closing it before Play: a restructuring of the launcher's threads for a wait of under a second per request; left.

#### `NET-002` Ping reply uses one clock value for receive and transmit

- SIMPLIFIED, low
- Original: not stated
- Client: the receive and transmit stamps of the ping reply are the same millisecond clock value
- A player notices: none visible
- Where: `crates/mao-net/src/frame.rs:209`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The original does the same: the capture of the original's own reply (the test `frame::ping_tests::ping_reply_matches_the_capture`) has the receive and transmit stamps equal (0x044fdd67 twice), so one clock value for both is what the original sent. Nothing changed.

### Network and protocol: the original was not read: inferred or measured (5)

#### `NET-003` Organisation and many other messages built from the decompile only

- GUESS, medium
- Original: Gamecode N3Msg_* functions (duel, pet duel, claims, /played /born /team loot /bank info, /rp, org commands, reset skill, stacks, team invite/kick/leader) and the organisation boxes' messages (invitation, tax, promotion, leave)
- Client: built from the disassembly; no capture of the original has one (none of the six organisation boxes seen live)
- A player notices: any could be refused or mean something else on the real server
- Where: `crates/mao-net/src/n3.rs:274`; `crates/mao-client/src/orgboxes.rs:23`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The messages are already written from the original's own builders (Gamecode `N3Msg_*` and the organisation boxes' senders); what is not known is only whether the live server accepts them, and no capture of the original holds any of them (docs/CAPTURE_GAPS.md). Nothing more can be read from the decompile; it needs a capture or a live try of each.

#### `NET-004` Entity message position and rotation read where captures put them

- GUESS, low
- Original: not stated
- Client: position() and rotation() read fixed offsets "where the captures put it"; the fields before are not understood
- A player notices: an unseen layout could misplace a character
- Where: `crates/mao-net/src/n3.rs:168`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The entity message's layout before the place is the original's `SimpleCharFullUpdate` reader in Gamecode (NET-005), not read field by field yet; the place and rotation offsets agree with every capture. Porting the reader is NET-005's work (well over 40 minutes); left.

#### `NET-005` Entity message's unknown number guessed as health; stats and rest not decoded

- GUESS, low
- Original: SimpleCharFullUpdateIIR_t reader reads stats, textures, clothing and more
- Client: a number after the flags is guessed to be health and its presence tells a character from a creature; after the name only a few fields are decoded, stats in the update are ignored
- A player notices: stats carried in the update are ignored
- Where: `crates/mao-net/src/n3.rs:1651`; `crates/mao-net/src/n3.rs:1632`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original's `SimpleCharFullUpdateIIR_t` reader reads stats, textures, clothing and more in a long chain of optional blocks; porting it field by field (and checking it against the captures) is far past 40 minutes, and the client's partial decode serves what it draws. Left.

#### `NET-006` Organisation member's info packet layout never seen; tower list skipped

- GUESS, low
- Original: not stated
- Client: a member's info packet (types 65, 67, 71) is read from the schema; types 67 and 71 list towers first, which is not read, so their records are not shown
- A player notices: info page of an organisation member may be wrong or lack the kill and duel records
- Where: `crates/mao-net/src/n3.rs:556`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The member info packets 67 and 71 (tower list first) appear in no capture of the original, so the tower list's layout can only be guessed from the schema; reading the original's handler would still leave it untested. Needs a capture.

#### `NET-007` Research goal choice kept by the window until the stat changes

- GUESS, low
- Original: not stated whether the server answers SetStat 265/266 with the stat
- Client: the window keeps its own choice until the stat changes
- A player notices: shown goal may differ from the server's if a SetStat is ignored
- Where: `crates/mao-client/src/research.rs:54`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Whether the server answers SetStat 265/266 with the stat cannot be read from the client's code (it is the server's behaviour); no capture of a research goal change exists. Needs a capture.

### Network and protocol: a guard or fallback the original lacks (3)

#### `NET-008` Network time-outs and non-blocking polling

- SAFETY, low
- Original: not stated
- Client: 20 s time-out on sends and on waits for login or chat replies; the world connection reads non-blocking and polls because read time-outs on Windows left the socket undefined
- A player notices: a slow server is given up on after 20 s
- Where: `crates/mao-net/src/socket.rs:37`; `crates/mao-net/src/chat.rs:480`; `docs/CLIENT_UI.md:2693`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: the non-blocking world socket is there because blocking reads with time-outs left the socket undefined on Windows (the client would stop reading the world), and the 20 s waits stop the login from hanging the launcher forever where the original shows its progress window's own time-out (which the launcher now also shows, LCH-038). Removing them would make the client hang where the original gives up.

#### `NET-009` Places that look like garbage are dropped

- SAFETY, low
- Original: not stated
- Client: ignores any position that is not finite, over 1e6, or tiny but not zero
- A player notices: a creature stays where it was instead of the world's corner
- Where: `crates/mao-client/src/net.rs:3672`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: the garbage places come from the client's own guessed offsets in the entity message (NET-004/005), not from the server; the original reads the real layout and never meets them. Without the guard such a misread draws a creature at a corner of the world or puts NaN into the renderer's matrices. It goes once the reader is the original's.

#### `NET-010` Waits at most 3 seconds for the own place at entry

- SAFETY, low
- Original: not stated
- Client: reads frames for up to 3 s hoping to learn the own place, then goes on
- A player notices: none visible
- Where: `crates/mao-client/src/net.rs:5984`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Kept: the wait only bounds how long the game's process reads before opening its window when the server is slow to send the own character; nothing a player can see or do differs (the original waits for the same message before its world shows, the client's loading screen covers both).

### Network and protocol: the original does it, the client does not (1)

#### `NET-011` Special-action server messages 0xb2, 0x14, 0xfc not handled; special attack target always a character

- MISSING, low (notes only)
- Original: 0xb2 marks the action available, 0x14 starts or lengthens a lock, 0xfc clears all perk locks; the attack names the target's own identity (can be an attackable item)
- Client: not handled; n3::special_attack hard-codes the character type 50000 (0xc350)
- A player notices: some lock states not tracked; attackable items cannot be special targets
- Where: `docs/SPECIALS_ORIGINAL.md:247`
- Decision: [x] port  [ ] leave
- Done (2026-11-18, overnight): 0x14 lengthens or starts a skill's lock, 0xfc clears the perk locks and stat 577; 0xb2 has nothing to change here (no mark apart from the locks). None is in a capture. The attack's target type (an attackable item) is not done.

### Network and protocol: the client has it, the original does not (2)

#### `NET-012` Help assistant: hosted model and guide crawler

- ADDITION, low (notes only)
- Original: not in the original
- Client: with a hosted endpoint set, the question and top results' text go to the model's provider; when the player turns the source on, a crawler fetches AO-Universe's 676 guides through curl (one page per two seconds or slower, robots.txt obeyed), off by default
- A player notices: data leaves the machine or web requests are made only if the player sets it up
- Where: `docs/HELP_ASSISTANT.md:162`; `docs/HELP_ASSISTANT.md:35`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: off by default, and the hosted model and the guide crawler only run when the player sets them up; the original game never contacts them.

#### `NET-013` Protocol read by a schema generated from SmokeLounge/AOSharp definitions

- ADDITION, low
- Original: each message is read by its own class reader
- Client: a schema generated from the SmokeLounge/AOSharp definitions is the generic reader and writer, with one size taken from captures; some names (mission fields) set by hand from a capture's values
- A player notices: none visible
- Where: `crates/mao-net/src/schema.rs:1`; `docs/CLIENT_UI.md:3111`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the schema is how the client reads and writes messages, not something a player sees; where the original's own readers have been read they win (the hand-set names, the captured size).

## Other

### Other: the client knowingly does otherwise (2)

#### `OTH-001` Game clock starts from the real clock until the server speaks

- OWN-RULE, low
- Original: starts its clock at 0 and waits for the server
- Client: the real clock at the game's pace stands in until a GameTime message comes
- A player notices: the hour on login before the server's word differs from the original's midnight
- Where: `crates/mao-client/src/gametime.rs:5`; `docs/CLIENT_UI.md:2131`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): Gamecode `GameTime_t::GameTime_t` 0x1000afdb zeroes the time of day (+0x40, +0x44, +0x48) and the day count (+0x4c); only the server's `GameTime` (`Update` 0x1000b590) sets them. `gametime::Clock::default` now starts at midnight of day 0 instead of the real clock at the game's pace (the server's time offset was already 0 until told). Test: `gametime::tests::the_clock_runs_at_the_games_pace` (asserts 00:00 of day 0 before the server's word).

#### `OTH-002` Resource database read with full 64-bit positions

- OWN-RULE, low (notes only)
- Original: opens files without c-tree's huge-file mode, cannot reach positions past 4 GiB
- Client: full 64-bit positions
- A player notices: objects the original cannot load are readable
- Where: `docs/ROADMAP.md:17`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: reading past 4 GiB only lets the client load what the original could not; it changes nothing the original can show.

### Other: an approximation or a stand-in (3)

#### `OTH-003` Camp countdown is a fixed 30 seconds

- SIMPLIFIED, low
- Original: not stated (the logout bar starts with CampStartedMessage)
- Client: the logout message is given a fixed 30 seconds
- A player notices: a camp message always says 30 seconds
- Where: `crates/mao-client/src/net.rs:7723`
- Decision: [x] port  [ ] leave
- Done (2026-11-19, overnight): The original does the same: GUI.dll `FlowControlModule_t::CampStartedMessage` 0x1002a86c gets the category-200 text and feeds it the constant 0x1e (30) (`LDBformat::Feed(..., 0x1e)`) whatever the bar's length, which is `CreateTimer(40000, ..., "Logout", 0xffffff)`; the client says `LogoutStartedXseconds` with 30 and runs its bar for `ui::CAMP_MS` = 40000 ms, the same two numbers. Nothing changed (the Where line points at an old place; the code is net.rs's `/camp` handling and ui.rs `CAMP_MS`).

#### `OTH-004` Server time from the real clock

- SIMPLIFIED, low
- Original: GameTime_t +0xc0/+0xc4 counted on by the frames' times from Update's
- Client: real clock plus server offset
- A player notices: none visible
- Where: `crates/mao-client/src/gametime.rs:85`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original counts its game time on from the server's Update by the frames' times (`GameTime_t` +0xc0/+0xc4); the client's real clock plus the server's offset gives the same time while frames are steady. Porting means carrying the frames' own times through the game time, with no difference a player could see.

#### `OTH-005` Spell runner clock is a client stopwatch

- SIMPLIFIED, low
- Original: GameTime_t +0x28
- Client: seconds since first use of the runner stand in
- A player notices: spell timing approximate
- Where: `crates/mao-client/src/net.rs:411`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The spell runner's clock (`GameTime_t` +0x28) and the client's stopwatch both count seconds from a start; the runner only uses differences between them, so the timings are the same. Left.

### Other: the original was not read: inferred or measured (4)

#### `OTH-006` Tradeskill guide is a reading of prose, not a recipe table

- GUESS, medium (notes only)
- Original: the server holds the table of tradeskill processes; the client learns an answer only when two items are put in the window
- Client: items and links read from names and descriptions (2,002 steps from sentences with a result phrase); skills a text gives and quality arithmetic are not read out
- A player notices: links and steps in the guide can be incomplete or wrong
- Where: `docs/TRADESKILL_GUIDE.md:48`; `docs/TRADESKILL_GUIDE.md:103`; `docs/TRADESKILL_GUIDE.md:104`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The original has no recipe table to port: its tradeskill window learns a result only from the server's answer when two items are put in (the process table is the server's). The guide is the client's own tool built from item texts; making it exact needs data the original never had.

#### `OTH-007` Features stat assumed present when missing

- GUESS, low
- Original: not stated
- Client: a character without stat 224 is taken to have bit 2 set, as the captured player did
- A player notices: timers may run for characters that lack the stat
- Where: `crates/mao-client/src/net.rs:1352`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): What the original does for a character without stat 224 is in its stat defaults, which no capture shows for a character lacking it (every captured player has it). Needs a capture of such a character.

#### `OTH-008` Kept temporary stats and login-kept craft assumed

- GUESS, low
- Original: what the original has kept for a character that logs in on its craft is not known; FUN_100a95c7 spell 0xcf76 with criteria, MonsterData mesh anew
- Client: kept values live only for the session; criteria on the spell and the mesh anew not ported
- A player notices: odd Yalmaha scale after login
- Where: `crates/mao-client/src/net.rs:1453`; `docs/ORIGINAL_AUDIT.md:78`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): What the original keeps for a character that logs in on its craft is not known (memory "MAO flight capture"); FUN_100a95c7's spell 0xcf76 criteria and making the MonsterData mesh anew need a capture of a login on a Yalmaha to be checked. Left.

#### `OTH-009` New-level message only partly decoded

- GUESS, low
- Original: not stated
- Client: first 28 bytes read as the trace showed, the rest laid out as the original's capture has it
- A player notices: experience bar span may be wrong if the layout guess is off
- Where: `crates/mao-client/src/net.rs:12839`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The new-level message's layout beyond the first 28 bytes is laid out as the original's capture has it; the reader in Gamecode was not found under a name. Left for a capture of a second level-up.

### Other: the original does it, the client does not (8)

#### `OTH-010` Notum Wars windows absent

- MISSING, high (notes only)
- Original: tower build menu, laser target, airstrike, battlestation, signup terminal, mech select
- Client: only tower sites and land-control view on the map
- A player notices: no Notum Wars interface
- Where: `docs/GUI_DLL_PARITY.md:110`
- Decision: [x] port  [ ] leave
- Done (2026-11-18): Done: all six windows, from the decompile alone (no message of any of them is in a capture; each that comes goes to the log whole). The signup terminal and the vehicles window (earlier the same day); the laser target and airstrike window (terminals of window kinds 4 and 5; no such terminal was found in the game's data); the battlestation's view with its invitation and end boxes; the tower build menu (the map's Build and War buttons, and using a tower, controller or shield disabler item opens it with its checklist). Not ported: BattleOver's two side effects on the character (not understood), the tower item's mark in the inventory, the use of such an item from a hotbar or dropped on the world.

#### `OTH-011` Help assistant: other sites and sources not done

- MISSING, low (notes only)
- Original: not stated
- Client: no first-run setup, scheduled reading, Discourse/MediaWiki readers, database source, whole help page in the info view, built-in HTTP client; Funcom forums, aodb.us, Fandom, auno.org not used
- A player notices: answers only from the game's help, crafting texts and AO-Universe
- Where: `docs/HELP_ASSISTANT.md:284`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: other sources for the client's own help assistant; nothing of the original's.

#### `OTH-012` Level-up: reset-points line and hints not ported

- MISSING, low
- Original: Gamecode FUN_10075f55 NewLevel prints the reset points' line (with the level's title) and the got_ip, got_perk, got_tech hints
- Client: stats and two lines done; the rest not ported
- A player notices: missing chat lines/hints at level up
- Where: `crates/mao-client/src/net.rs:2033`; `docs/CLIENT_UI.md:662`; `docs/ORIGINAL_AUDIT.md:21`; `crates/mao-client/src/net.rs:2032`
- Decision: [ ] port  [ ] leave
- Done in part (2026-11-18, overnight): the experience line and the title line are said (the title's from the code alone: no capture); there is no reset points line in the original; the hints are the notice buttons and are not done.

#### `OTH-013` Looking for team: Search finds nobody

- MISSING, low
- Original: LFT list from the chat server
- Client: view only; Search/Invite/Tell send nothing
- A player notices: LFT window does nothing
- Where: `crates/mao-client/src/lft.rs:5`; `docs/GUI_DLL_PARITY.md:93`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The Looking For Team search is a chat-server exchange whose messages appear in no capture of the original and are not in the client's chat protocol; porting it needs that capture.

#### `OTH-014` Mine/hazard signal after the bleep not sent

- MISSING, low
- Original: FUN_10085faa sends a signal (GlobalSignals_c +0x14c with 3.0, FUN_10011a4b) after the bleep
- Client: not done
- A player notices: none visible known
- Where: `crates/mao-client/src/hazards.rs:21`; `docs/ORIGINAL_AUDIT.md:97`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): The signal after the bleep (FUN_10085faa: `GlobalSignals_c` +0x14c with 3.0, FUN_10011a4b) has no traced receiver: which window or effect listens on +0x14c is unknown, so there is nothing to wire it to. Left until the receiver is found.

#### `OTH-015` Organization: no module views; contract window read-only

- MISSING, low (notes only)
- Original: OrganizationGUIModule_c views; raises GlobalSignals_c +0x18c with the text; ItemContainerView_c shows the contract and handles clicks
- Client: /org commands, info page, container window; contract window titled "Organization contract" in this client's words, click or drop sends nothing; signal receiver not traced
- A player notices: organization UI is chat-driven; contract window is read-only
- Where: `docs/GUI_DLL_PARITY.md:95`; `docs/GUI_DLL_PARITY.md:239`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the organization module's views and the contract window's clicks are a large GUI.dll part (`OrganizationGUIModule_c`, its ItemContainerView_c); out of reach tonight.

#### `OTH-016` Server CharacterAction cases with unknown meaning not handled

- MISSING, low (notes only)
- Original: Gamecode cases 12, 23, 3, 61, 90, 39/44/46, 49 (actions 0x2f, 0x61, 0x15, 0xab, 0xd0, 0x81/0x8a/0x8c, 0x93)
- Client: meanings unknown; 0x84 is handled since (unable-to-perform line)
- A player notices: those server actions have no effect (e.g. 0xd0 a perk's effect)
- Where: `docs/CAPTURE_GAPS.md:316`; `crates/mao-client/src/net.rs:2544`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: the CharacterAction cases' meanings are unknown; without a capture of each nothing faithful can be written.

#### `OTH-017` Tradeskill guide: no outside recipe table

- MISSING, low (notes only)
- Original: not stated
- Client: nothing reads a recipe table from a player's export or community database
- A player notices: no recipes beyond the game's texts and what the player tried
- Where: `docs/TRADESKILL_GUIDE.md:106`
- Decision: [ ] port  [ ] leave
- Looked at, left (2026-11-19, overnight): Notes only: an outside recipe table is not the original's (the original has none; OTH-006).

### Other: the client has it, the original does not (4)

#### `OTH-018` Help assistant: a small language model run inside the client

- ADDITION, medium (notes only)
- Original: not in the original
- Client: Qwen2.5 1.5B Instruct (1,117 MB) fetched on request and run by candle; search is plain word matching, so a small model may answer wrongly from the nearest pieces; about a gigabyte held, frame rate drops while answering
- A player notices: answers appear in the window and can be off; frame rate dips
- Where: `docs/HELP_ASSISTANT.md:221`; `docs/HELP_ASSISTANT.md:259`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: the help assistant is one of the client's own tools the user asked for; the model is fetched only on request and runs only while the window is asked something, so a player who never opens it sees nothing of it.

#### `OTH-019` Help assistant: crafting texts as a source

- ADDITION, low (notes only)
- Original: nothing like the "Ask the guides" window or an answer from item texts
- Client: books and item descriptions that speak of making (3,476 pages) are a source, on by default
- A player notices: "How do I make a nano crystal" is answered from the game's own book; one question joined two texts wrongly
- Where: `docs/TRADESKILL_GUIDE.md:91`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: part of the client's own help assistant; the texts are the game's own books and descriptions.

#### `OTH-020` Linux and macOS builds exist

- ADDITION, low (notes only)
- Original: a Windows client
- Client: builds for Linux x86-64 and Apple-silicon macOS; neither run on a real machine
- A player notices: new platforms, untested
- Where: `docs/PLATFORMS.md:3`; `docs/PLATFORMS.md:104`
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP: other platforms change nothing for the Windows client.

#### `OTH-021` Screenshot key saves a PNG with a notes file

- ADDITION, low
- Original: not stated
- Client: F12 saves a picture with HUD (on Windows the monitor's picture of the window, including web pages over it), plus a text file with place, position, camera and the looks of characters around
- A player notices: extra feature and .txt file beside screenshots
- Where: `crates/mao-client/src/screenshot.rs:1`; `crates/mao-client/src/shotnotes.rs:1`; `crates/mao-client/src/viewer.rs:2645`; `docs/CLIENT_UI.md:1820`; `crates/mao-client/src/viewer.rs:2654`; `docs/CLIENT_UI.md:3472` and 1 more
- Decision: [ ] keep  [ ] remove
- Recommendation (2026-11-19, overnight): Addition: recommend KEEP the notes file (the user's tool for finding places and meshes); the original's screenshot key saves a picture too, so only the .txt beside it is extra.

