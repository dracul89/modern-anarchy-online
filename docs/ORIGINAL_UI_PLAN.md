# Plan: make the new client look, feel and operate like the original Anarchy Online client

This is a handoff document. It says what the goal is, what is already done (with file pointers), how the
original's GUI is defined on disk, and the ordered list of work that remains. Anyone (person or LLM) should be
able to pick up at any phase from this file, `docs/CLIENT_UI.md` (the feature log) and the code.

## 1. The goal, in one paragraph

A returning or existing Anarchy Online player sits down at the new client and it feels like the game they know:
the same keys, the same windows in the same places with the same chrome, the same control center and chat,
the same skins (a skin folder of the original or of the AOU mod drops in and is used). Everything *behind*
the GUI is new and modern and may differ freely: 64-bit, Rust, wgpu/Vulkan, no memory leaks, fast zoning,
shadows, FSR, big view distances. **The rule: changes go on top of the existing egui UI; nothing of the
client's own features is removed, developer aids are hidden behind the `debug_overlay` setting, and
anything the original did not have is either dropped from the default view or made to look native.**

## 2. What is done (and where)

| Area | State | Code / docs |
|---|---|---|
| Original default keys (movement, camera on the numpad, Tab targets, F1 self, Q attack, X sit, E use, I/U/Ctrl+1..0 windows, Shift+P perks, B/M/K/L/,/V/R/H quick actions, F8 camera, F9 coordinates, F12 screenshot, Alt+1-6 shortcut bars, 1-0 slots) | done, `KEYMAP_VERSION = 2` replaces older files' keys | `crates/mao-client/src/settings.rs` (`ACTIONS`, `QUICK`), handlers in `viewer.rs`; key codes decoded in `docs/CLIENT_UI.md` §"original keys" |
| Developer overlay hidden (location, coordinates, fps, hotbar hint) | done, `debug_overlay = off` default, `MAO_DEBUG_HUD=1` forces on | `ui.rs::debug_overlay()`, settings `[interface] debug_overlay` |
| Escape closes dialogs like the original | done | `Ui::close_with_escape`, action `CloseWindows` |
| Bottom control center from the original skin: wings, health/nano/XP/alien-XP bars, left/right menu icons, compass, two target buttons, sub-menus (Actions, Character, Map; Camera and Help inside Actions) | done against the shipped Default skin | `crates/mao-client/src/vitals.rs` (`Skin`, `show_skinned`, `compass`, `target_buttons`, `sub_menu`, `menu_items`); layout taken from `cd_image/gui/Default/Views/ControlCenter.xml` and `ActionMenu/*.xml` |
| Skin layer (Phase A.1): `gui_dir` setting / options field / `MAO_GUI_DIR` → theme folder → game Default, per-sprite fallthrough, lazy decode | done, checked offscreen with Default and AOU_CYT | `crates/mao-client/src/skin.rs` (`Skin`, `dirs`, `configure`, `skin`); research tool `tools/uvga_extract.py <skin folder> <out>` |
| `TextColors.xml` chat colours (Phase A.3) | done | `theme.rs::text_colors`, `Palette::text`, `chatwindow.rs` |
| Window chrome (Phase B.1-2): original frame, title, pin and close on every window | done, checked offscreen | `crates/mao-client/src/chrome.rs` |
| Bitmap fonts (Phase A.2): `.fnt` reader, glyph mesh painter, word wrap; chat lines in `CHAT9` | done, checked offscreen; `original_font` setting | `crates/mao-client/src/font.rs` |
| Target frame (Phase D.2), chat tabs (Phase E.1) | done, checked offscreen | `ui.rs::target_frame`, `chatwindow.rs` |
| Theme colours from any `GUIColors.xml` folder (`theme = original`, `aou_cyt`, ... or a path; `aou_mod =` points at the AOU mod folder) | done | `crates/mao-client/src/theme.rs` (`theme_dir`, `Palette`) |
| Window positions remembered between sessions | done (this client's own `layout.ini`, not the original's `Prefs.xml`) | `layout.rs` |
| `/camp` returns to the character screen | built, not tried live | `viewer.rs` (`logged_out`), `launcher.rs` (`LOGGED_OUT`, `MAO_RELOGIN`) |
| Aura recasts the server repeats are silent | built, not confirmed with the Keeper | `net.rs` (`self_casts`, `asked_casts`) |
| Character screen: list left, figure in the window, Play at the bottom, cached looks | done | `launcher.rs`, `look.rs` |

What the windows are built with today: plain egui (`egui::Window`) with the theme's colours. None of the
windows except the control center use the original's art yet.

## 3. How the original defines its GUI (the data a skin is)

All of this is in the game folder `cd_image/gui/Default/` (the shipped skin) and, for a user skin, in
`<LocalAppData>/Funcom/Anarchy Online/<hash>/Anarchy Online/Gui/<SkinName>/` (the user's copy for reading is
`C:\Users\danny\Workspace\rea_ao\70dad3e6\Anarchy Online\Gui\` with four AOU skins: `AOU_CYT`,
`AOU_Slim_CNI`, `AOU_Slim_CNT`, `AOU_Translucent`). The per-character preferences live beside it under
`Prefs/derived/Char<id>/Prefs.xml` and the account ones in `Prefs/derived/Login.cfg`.

| File | Format | What it holds | Reader status |
|---|---|---|---|
| `Graphics.uvgi` | text: title line, a count, then `NAME offset length` per line | index into the atlas; 789 names in Default, 287 in AOU_CYT | read (control-center names only) |
| `Graphics.uvga` | concatenated PNGs at the index's offsets; pure green `(0,255,0)` is the transparent key | every sprite: window borders (`GFX_GUI_WINDOW3_BORDER_*` 10 pieces, `GFX_GUI_INFO_BORDER_*` 15), title buttons (`WINDOW_CLOSE/PIN/MAXIMIZE/QUESTIONMARK` 3 states each), tabs (`TAB_BORDER_*`, `TAB_ACTIVE/INACTIVE`), scrollbars (`SIMPLE_SB_*` 18, `SCROLLBAR_GRAY_*`), buttons (`BUTTON_ACCEPT/QUIT/CHECK/EXPAND`), progress bars (`PROGRESS01_BORDER_*`, `HOR_BAR_*`), toolbar (`TOOLBAR_BORDER/BUTTON/CONTROL_H/V`), control center (`CONTROLCENTER_*`, `CC_*`, `COMPASS*`, `ACTIONVIEW_*BAR*`), skill window art (`SKILL_BUTTON_*`, `SKILL_MAIN_*`), profession icons (28), pointers (`POINTER_STANDARD_*` 7), fonts (`GFX_FONT_CHAT9`, `GFX_FONT_GAMESHELL12`, `GFX_FONT_TOOLTIP9`: bitmap fonts as PNG strips), map icons | partial |
| `GUIColors.xml` | `<Color name= color=0x00rrggbb>`: Default, Selected, Hover, Text, TextSelected, TextHover | the skin's six colours | read (`theme.rs`) |
| `TextColors.xml` | named HTML colours, then chat kinds (`ct_system`, `ct_otell`, `ct_itell`, `ctch_clan`, ...) and text classes | chat and text colours | not read |
| `Views/*.xml` (27 files, 80 kB) | a layout language: `View view_layout=vertical/horizontal/stacked`, `HLayoutSpacer`, `layout_borders=Rect(l,t,r,b)`, `activate_criteria="dvalue:... && stat:..."`, `fade_group`, controls `Button`, `TextView`, `CCMenu`, `CCMiniToolbar`; fonts `LARGE`/`HUGE`/`CC17` | the control center, skills, trade, mail, login, character selection, research, raid, LFT, info, tips | `ControlCenter.xml` read by hand |
| `ActionMenu/*.xml` | `Menu`/`MenuEntry` with `label`, `name` (a window), `bgicon=id:GFX_GUI_CC_ICONS_*`, `sub_script`, `criteria`, `Actions criteria=dvalue:AutoCloseWindows` lists | the control center's menus; `Windows.xml` the help/windows list; `SocialMenu.xml` emotes; `PetMenu.xml`; `CameraMenu.xml` 1st / 3rd trail / lock / rubber | Left/Right/Char/Command/Map/Camera read by hand |
| `CharPrefs.xml` (shipped defaults) | `<Value name= value=>` flags (`cc_*` visibility, `AutoCloseWindows`, ...), `<Archive name="<X>WindowConfig">` with `Rect name="WindowFrame"`, pin/visible flags, `KeyBindings` archive (`Input` key code + modifier bits, `Provider` hashed action name) | window default positions, control center parts, default keys | keys decoded; rects not used |
| `Prefs.xml` (per character) | same archives with the player's values | where the player put every window, which are open, bar configs (`CC*BarConfig`), muted chat groups | not read |
| `OptionPanel/Root.xml`, `HotKeys.xml` | the options window's pages | options layout | not read |
| `MainPrefs.xml`, `Variables.xml`, `LoginPrefs.xml` | more defaults | | not read |

Key enum and provider hash (for `KeyBindings`): see `docs/CLIENT_UI.md` §"original keys" (codes 14 ESC ...
118 SLASH; modifiers Shift 0x20000, Ctrl 0x40000, Alt 0x80000; provider = `x ^= (x<<6)+(x>>2)+c+0x9e3779b9`
over the action name, e.g. `MOVEMENT_FORWARD_V3`, `WINDOW_INVENTORY`, `SHORTUCT_BAR_SELECT_n`).

## 4. The skin-directory contract (what "accept a custom GUI from a directory" means)

Add one setting, `[interface] gui_dir = <folder>`, and make **every** piece of original art, colour and
layout resolve through one function with this search order:

1. `gui_dir` from the settings (or `MAO_GUI_DIR` in the environment) — the user's skin folder, e.g.
   `.../Gui/AOU_CYT`.
2. `aou_mod/Gui/<theme folder>` when `theme =` names an AOU theme (existing `theme_dir`).
3. The game's `cd_image/gui/Default` (always present).

Each lookup is **per file and per sprite**: a skin that ships only `Graphics.uvga` + `GUIColors.xml` (that is
what the AOU skins are: 287 of the 789 sprites, no `Views`) gets the rest from Default. A sprite missing from
the user atlas falls through to Default's atlas. This is what the original does (`SetSkinData` in
DisplaySystem overlays the skin on the defaults).

Implementation: move `vitals.rs::Skin` into a new `crates/mao-client/src/skin.rs` holding the *whole*
atlas of each layer lazily (load the index up front, decode a PNG on first use, cache the
`TextureHandle`), with `sprite(name) -> Option<&Sprite>`, `nine_slice(prefix)`, `font(name)`. The control
center then stops owning the loader. `theme.rs` keeps the colours and gains `text_colors()` from
`TextColors.xml`. Expose `/skin <folder>` (or the options window's theme page) to switch at run time: drop
the cache and reload.

Note the AOU wings are 440x220 / 320x220 against Default's 189x194: the control-center layout must be
driven by the sprite sizes, never by constants (it mostly is; check `wing_bar` offsets when the AOU skin is
active).

## 5. The phases, in order

Each phase ends with: offscreen preview (`MAO_UI_PREVIEW=...`, see `docs/CLIENT_UI.md`), `cargo test -p
mao-client`, a live look if the user is around, a paragraph in `docs/CLIENT_UI.md`, one commit.

### Phase A — the skin layer (foundation for everything else)
*(A.1 and A.3 done 2026-10-26; A.2 done (fonts, `font.rs`) and A.4 done (pointers, `pointer.rs`), both 2026-10-26. A.2's data is decoded: `cd_image/textures/fonts/*.fnt` = header (256, 256) + 318 entries of (x0, y0, x1, y1) per character code into the `GFX_FONT_*` strip — see `docs/CLIENT_UI.md` "bitmap fonts". What remains is a glyph painter for egui (a mesh of textured quads per line, word wrap by summing glyph widths), used for chat lines and tooltips, behind a setting since 9 px glyphs are small on today's screens.)*
1. `skin.rs` as in §4; `gui_dir` setting + options-window field + `MAO_GUI_DIR`.
2. Decode the bitmap fonts `GFX_FONT_CHAT9/GAMESHELL12/TOOLTIP9` (PNG strips; glyph metrics are either a
   fixed grid or a table in the PNG's trailing bytes — inspect with `tools/uvga_extract.py`, compare to the
   original's rendering). Register them as egui fonts, or map LARGE/HUGE/CC17 to sized proportional fonts if
   the bitmaps resist. The chat and tooltips are where the font makes the "feel".
3. `TextColors.xml` → chat kind colours (`ct_*`, `ctch_*`) replacing the hard-coded ones in `chatwindow.rs`.
4. Mouse pointers from `GFX_GUI_POINTER_STANDARD_*` (winit custom cursors) — optional, cheap, very "original".

### Phase B — window chrome
*(B.1 and B.2's conversion done 2026-10-26: `chrome.rs`, all 17 windows. Scrollbars and tabs in the skin's art done 2026-10-28 (`chrome::scroll`, `chrome::tab_view`). Open: buttons in the skin's art, the ? button, the launcher's windows, B.3 titles.)*
1. A `skin_window()` helper that draws an `egui::Window` with the original's nine-slice frame
   (`GFX_GUI_WINDOW3_BORDER_*`: corners, edges, title strip), the title in the skin's text colour, and the title
   buttons (`WINDOW_CLOSE`, `WINDOW_PIN` = stays open when Escape closes dialogs, `WINDOW_MAXIMIZE`,
   `WINDOW_QUESTIONMARK` = opens the help window on that topic). Pass-through to plain egui when no skin.
2. Convert every window to it: inventory/wear (`gear.rs`), skills, stats, NCU, nano programs, perks, friends,
   missions, mail, shop, loot, help, actions, options, whompah, chat. Tabs use `TAB_ACTIVE/INACTIVE`, scrollbars
   `SIMPLE_SB_*`, buttons `BUTTON_ACCEPT/QUIT`, progress bars `PROGRESS01_BORDER_*`/`HOR_BAR_*`.
3. Rename windows to the original's titles (Wear, Inventory, Skills, Stats, NCU, Nano Programs, Perks,
   Friends, Mission, Mail, Shop, Help, Actions, Options, Whom-Pah).

### Phase C — default window positions and the original's prefs
*(Looked at 2026-10-26: the shipped CharPrefs.xml window archives are empty and a character's Prefs.xml only has frames for moved windows (5120x1440, no resolution saved); `AutoCloseWindows` defaults to false. C.1/C.2 are not worth doing; C.3 only if a player asks.)*
1. Read `CharPrefs.xml`'s `<X>WindowConfig` rects for the default positions (and `Prefs.xml` when a
   character's folder is found under `<LocalAppData>/Funcom/Anarchy Online/*/Anarchy Online/Prefs/derived/
   Char<id>/`): map config names to our windows (`WearViewConfig` → inventory, `StatWindowConfig` → stats,
   `NCUWindowConfig`, `NanoViewConfig`, `PerkWindowConfig`, `FriendsWindowConfig`, `MissionViewConfig`,
   `ShopViewConfig`, `InfoViewConfig` → help, `ChatGroupWindowConfig` → chat, `OptionWindowConfig`,
   `CompassWindowConfig`, `PFMapWindowConfig` → map overlay). Rects are in the original's screen pixels;
   scale by our window size when the saved resolution differs (the rects above 2000 come from a 2560-wide
   screen).
2. Our `layout.ini` stays the store for the new client; the original's file is the *default* when ours has no
   entry. Optionally write back a `Prefs.xml`-compatible file so the original client sees the same layout
   (low priority).
3. `cc_*` flags: honour `cc_compass`, `cc_left_menu`, ... from the prefs; `AutoCloseWindows` → close the
   other windows when a menu entry opens one (the `Actions criteria=dvalue:AutoCloseWindows` lists).

### Phase D — the rest of the control center
*(D.2 target frame, D.4 shortcut bar art and D.5 camera modes done 2026-10-26. Open: D.1 Wear/Shop icons (the Default strip is 214 px: six icons do not fit as drawn, check how the original's CCMenu lays them out), D.3's mini toolbar and agg-def slider (the rollup is done).)*
1. Wear and Shop icons (`GFX_GUI_CC_ICONS_WEAR`, `STORE`); Vehicle/Perks/Skills/Shortcuts/Team/Factions
   icons exist too — add entries as windows appear. Team menu (Team, Raid, Find Team) once a team window
   exists.
2. Friendly/hostile target health bars (`cc_friendly_health_bar`, `cc_hostile_health_bar`, art
   `GFX_GUI_TARGET_HB_*`, `GFX_GUI_CC_TARGET_*`) replacing the plain target box at the top; the target's name,
   level and health (`hud.target` already carries HP when known).
3. The rollup controller (`ROLLUP_EXPAND/COLLAPSE`) that hides the wings; the mini toolbar
   (`TOOLBAR_*`) when `cc_mini_toolbar`; the agg/def slider (`CONTROLCENTER_AGGDEF_SLIDER*`) wired to the
   `AggDef` stat (51) — find the set-stat message in a capture before claiming it works.
4. Shortcut bar in the original's look (`CONTROLCENTER_BUTTON01/02/03_*` strips, `BIGARROW_*` bar
   selectors, `SHORTCUTS` icon); keep our smart-hotbar behaviour (queues, cooldown sweeps) underneath.
5. The three 3rd-person camera modes (trail / lock / rubber) as real camera behaviours
   (`CAMERA_3RD_LOCK` = yaw follows heading rigidly, `RUBBER` = our current smoothed follow, `FREE/TRAIL` =
   no auto-turn), switchable from the Camera menu and keys.

### Phase E — chat like the original
*(E.1 tabs done 2026-10-26; the input bar kept plain. Open: the original's tab set/groups from ChatGroupWindowConfig, links/macros, the font.)*
1. Chat window with the original's tabs (All/Team/Vicinity/… groups from `ChatGroupWindowConfig`,
   `MutedChatGroups`), the `INPUT_BAR_*` input line, the font from Phase A, colours from Phase A.3.
2. Chat-text macros/HTML subset (`<font color=...>`, item/chat links `text://`, `chatcmd://`) as the
   original renders them; clickable links.
3. Keep our additions (command list on `/`, history) but hide the hints unless `debug_overlay`.

### Phase F — remaining original windows (as the network work allows)
Team window (Shift+F1-6 pets, team list), Knowledge/Faction, planet map (`PLANETMAP_*` art), tradeskill,
trade (`TradeGUI.xml`), research, raid, LFT, tips. Each: original title and layout from its `Views/*.xml`
where one exists, our data model behind it.

### Phase G — polish and parity checks
1. A side-by-side check list: screenshots of the original (through the capture proxy, see
   `tools/capture_proxy.py`) and the new client for each window; fix size, spacing, colours.
2. Options window pages mirroring `OptionPanel/Root.xml` (video pages are ours: resolution, vsync, FSR,
   view distance; GUI page: skin folder, theme, interface size; key page from `HotKeys.xml` ordering).
3. Remove or hide any remaining "new client" wording in on-screen text (`(F10: options)` etc.).

## 6. Things that must *not* change (the modern backend)

64-bit Rust client, wgpu (Vulkan/DX12/Metal), the particle pass, shadows, FSR, mip-mapped texture prefetch,
direct-hop zoning (`hop_direct`), the capture-driven protocol (`crates/mao-net`), the stat engine
(`derived.rs`), smart hotbars, remembered layout. The GUI work sits on top of these; if a UI change needs a
backend change, do the backend change in the backend crate with its own test and commit.

## 7. Gotchas already paid for

- Never build pictures of the UI from guessed data: demo characters once had invented breeds/levels and the
  user noticed at once. Use capture data (`MAO_CAPTURE=<log>`, `docs/live_*.txt`).
- 2D "aura" rings over characters looked bad; the nano effects are 3D particles now (`effects.rs`), the rings
  are off unless `MAO_AURAS=1`.
- The server re-sends an aura's self-cast every few seconds; treat as a running nano, not a cast.
- `widgets`/positions in egui must come from sprite sizes (AOU vs Default differ).
- Python edit scripts: write them to a file with the Write tool (bash heredocs mangle quotes and
  backslashes), keep CRLF (`rw()` helper pattern), `encoding='utf-8'`.
- Build to `--target-dir target-dev` while the game is running (the release exe is locked); kill
  `mao-client.exe` only to restart for the user.
- Keymap: bump `KEYMAP_VERSION` when the defaults change so old settings files yield.
- Preview everything offscreen first: `MSYS_NO_PATHCONV=1 MAO_UI_PREVIEW=1 ./target-dev/debug/mao-client.exe
  "<game>" map 566 --image out.png` (+ `MAO_CC_MENU=camera`, `MAO_UI_PREVIEW=stats MAO_CAPTURE=<log>`, ...).

## 7b. Where it stands after 2026-10-26, and what needs the original as a reference

Done: A.1-A.4, B.1-B.2, D.2, D.4, D.5, the rollup of D.3, E.1. Looked at and dropped: C.1/C.2 (no data), G.2's
option pages (`OptionPanel/Root.xml` is empty, `HotKeys.xml` lists only the fixed keys).

The next items cannot be done well from the data alone; each needs a screenshot of the original client (the user has it
and uses the AOU skins):
- **AOU skins' bars**: the AOU wings (440 x 220) carry curved `GFX_GUI_ACTIONVIEW_*BAR*` bars (43 x 161) at places the
  data does not state; a character's `CC*BarConfig` rects (10 x 124 stacked in a 23 x 268 dock) hint at a bar dock
  beside the wing. Today an AOU skin shows the game's straight bars at the game's offsets.
- **D.1 Wear/Shop icons**: six 48 px icons do not fit the 214 px strip as drawn; see how the original's CCMenu lays them.
- **E.2 chat groups/links**: the default tab set and the look of item/command links.
- **G.1 parity**: side-by-side of every window.
Without a reference, prefer the items that need only network work (Phase F windows) over guessing the look.

Done from the user's screenshots (2026-10-26): login dialog and character screen (`launcher.rs`), default theme `original`. Open there: the 3D hall behind them (find the room: not playfield 896), New Character / Delete.

## 8. Open questions for the user (ask when relevant, do not block on them)

- Which skin should be the default when none is set: the shipped Default, or an AOU one if the mod folder is
  found?
- Should the new client write the original's `Prefs.xml` format so layouts round-trip to the old client?
- The two target buttons' real functions in the original (we guessed: self / next hostile).
- Compass sense: confirm N on the compass matches the mini map's N in game.
