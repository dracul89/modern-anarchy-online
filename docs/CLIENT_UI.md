# Client visuals, window and interface

*The pictures this file names under `shots/` are the client's own offline renders. They are not kept in the repository (the folder is ignored since 2026-11-14): the names record what was looked at.*

What was built for "enhance the visuals, foliage, AOU mod, key bindings, window
modes, whompah labels", what was verified and what is still open.

## 1. Window modes (done)
`windowed`, `borderless` (a window covering the screen) and `fullscreen`
(exclusive, at the monitor's best video mode; borderless where the monitor has
none). Choose in `mao.ini` (`[window] mode =`), on the command line
(`--window-mode borderless`, `--width`, `--height`), with **F11** (windowed <->
borderless), **Alt+Enter** (cycle all three) or the **options window** (Esc).
The options window saves what you pick. `mao.ini` lives in the working
directory if it has one, else `%APPDATA%\ModernAnarchyOnline` (`~/.config/...`
elsewhere); `MAO_SETTINGS=path` points elsewhere.

## 2. Key bindings (framework done; original defaults not verified)
Every action is rebindable (options window: click a key, press the new one; or
edit `[keys]` in `mao.ini`; `Shift+`, `Ctrl+`, `Alt+` combine). The action
names are the original's own (found in its DLLs): `KEY_COMMAND_ROTATE_CAMERA_*`
-> `camera_rotate_*`, `KEY_COMMAND_ZOOM_CAMERA_IN/OUT`, `CAMERA_RESET`,
`CAMERA_SET_PREFERRED_POS`, `KEY_NEXT/PREV_FRIENDLY/HOSTILE_TARGET`,
`START_CHAT_INPUT/CMD_INPUT/REPLY_INPUT`, `CHAT_HISTORY_PAGE_UP/DOWN`,
`OPEN_PERK_WINDOW`, `OPEN_VEHICLE_WINDOW`, plus this client's movement.
The camera, window, movement and targeting actions work (Tab / Shift+Tab step
through nearby hostile NPCs nearest first, Ctrl+Tab through players and friendly
NPCs, `F` attacks the target; a target frame shows its name and distance; the
same select / set-target / attack messages as the tested `attack` command, not
yet verified in the live window). Chat and the perk / vehicle windows are
reserved (no interface for them yet).

**Open:** the default *keys* of the original's commands are not in its data
files. The AOU mod's `HotKeys.xml` is byte-identical to the original's and only
lists the action names. A player's `Prefs.xml` stores 99 bindings as
`(input code, provider id)` pairs; the provider ids are 32-bit values I could
not match to any common hash (CRC32, FNV, djb2, sdbm, one-at-a-time, with
case variants) of the ~115k identifier strings in the DLLs. Decoding them (most
likely a hash inside Gamecode.dll / GUI.dll) would give the true defaults. Until
then the defaults marked "assumed" in `settings.rs` follow AO's well-known
layout and should be checked against a real client.

## 3. Foliage (done)
Meshes whose file name is a plant (`is_foliage_name`: tree, bush, palm, grass,
fern, flower, shrub, vine, cactus, ivy ... as whole-word endings, so
`street_lights` and `Inferno_trophy` are not plants) are flagged at load. In the
vertex shader they sway in the wind (more towards the top) and bend away from
the player within 3 units, low plants lying over more than tall trees. In
Old Athens 21 of the 156 prop meshes are plants (2,339 placed). Seen in
motion only in the live window; offscreen: `MAO_PLAYER=x,y,z` (Z-up world).

## 4. Visual enhancements (done)
Sky gradient, distance fog into the horizon colour (live view), hemisphere
ambient light with a warm sun, 4x MSAA (where the device supports it), mip
chains and 16x anisotropic filtering for every texture. A character texture's alpha is
not a cut-out mask: it blends or it is light (see "What a character texture's alpha does"). The sky and fog are the effect scripts' per playfield now (see "The sky"). Bloom is not a look the original
had and is not planned.

## 4b. Shadows and water
**Shadows (done):** a 4096x4096 sun shadow map is drawn each frame around the
player (70 units; the whole scene in an overview), snapped to texels so it
doesn't shimmer, with a 3x3 soft comparison, a normal offset against acne, a fade
at the map's edge, and plants bending and cut-out leaves in the shadow pass as
in the main one. Parts outside the box are skipped. Verified offscreen (the
whompah plaza); `Renderer.shadows = false` turns them off.

**Water (done):** the playfield's own water polygons, shaped and drawn as the original's `VisualWater_t` and
`VisualLiquid_t` do (see "The water and the playfield's own environment" below; `mao_formats::liquid`, `liquid.wgsl`).
A playfield without polygons falls back to the old guessed sheet (`Terrain::water`: a flat floor holding a fifth of the
ground's points at the bottom of its range is a seabed, the water 1.5 above it; `MAO_WATER=<level>` overrides the
level), which no playfield with a readable tile-map needs any more. The refraction pass, the liquid raised 0.2 while the
camera is under water (with the under-water fog) and the fall effect a falling polygon starts are done too (below). Not
done: the star water of types 0xe/0xf (no playfield has any).

## 4c. Performance
* **Frame rate:** uncapped by default (`[window] vsync = off`; `on` waits for the monitor).
* **Frustum culling:** the camera pass skips parts outside the view (a plaza view draws 376 of 5,698 parts); the HUD shows the counts.
* **FSR 1 upscaling:** `[window] render_scale = 0.4..1.0` (also a slider in the options window, `MAO_RENDER_SCALE` for headless) draws the scene at that share of the window and upscales it with AMD FidelityFX Super Resolution 1 (EASU + RCAS, ported to WGSL from AMD's MIT-licensed `ffx_fsr1.h`; the notice is in `fsr.wgsl`), then sharpens (`sharpness`, in stops). At 60% a 960x720 frame is drawn at 576x432 and still looks sharp. Verified offscreen; the live gain isn't measured.
* **Not possible:** DLSS (NVIDIA-only SDK; this machine has an AMD GPU). FSR 2/3 need motion vectors, depth and jitter (a larger renderer change; TAA is the first step).
* **Next:** instancing the repeated props (about 3,000 placed from 156 meshes), cheaper shadow casters at a distance, TAA.

## 4d. Camera (live view)
The mouse is the original's (GUI.dll's `ActionViewMouseHandler_c`; corrected 2026-11-18, "The mouse's camera as the
original's"): **left button drag** looks round with the camera (`LMBMouseLook`), **right button drag** turns the
character (only the character: the camera follows it as its mode says, and the drag's up and down tilt the camera by
`RMBMouseLook1st` / `RMBMouseLook3rd`), with Ctrl only the camera; the last button pressed is the look's and letting go of either ends it; the turn
keys strafe while a look moves; the **wheel** zooms `ZoomSpeed` / 10 a notch. The cursor is hidden while a button is
held (the original moves it back to where the look began). The middle button resets the camera (this client's own; the
original has Numpad 5). Strafing is on `Z` / `C` (A/D turn, as the original's defaults do; see 4e).

## 4e. Controls added for playing
`Z` / `C` strafe left / right (the protocol's `StrafeLeftStart` 7 / `Stop` 8 and `StrafeRightStart` 5 / `Stop` 6 are
sent as the original sends its movement types; the captures carry them for the other characters, who are drawn strafing); `Ctrl+Shift+C` toggles walking through props (was `C`);
`X` sits or stands (sit = action `0x78`, which the protocol names **Logout**: the server starts a ~30 s logout
countdown and broadcasts `0x7a`; stand = `0x57` cancels it; moving stands you up, and the log says so); `E` uses the target (message `52526858`, action 3, decoded from the capture and
byte-exact in a test; not yet confirmed live because the test character stands in an interior with no
announced objects). `Tab` cycles creatures, `Ctrl+Tab` players, friendly NPCs and the usable objects the
server announces (`4f474e05`, identity type `0xc76a`: doors, mail terminals, booths) shown as "Object xxxx"
until their names are decoded.

## 4f. Smart hotbars
Six bars of ten slots (keys `1`-`0` for slots, `F1`-`F6` for bars, all rebindable; mouse click works too).
A slot is a **priority list** of actions, each with its own cooldown. A press fires the first ready action
and starts its cooldown, so while it recovers the slot offers the next one (opener, filler, finisher). If
nothing in the slot is ready the press is **queued** (yellow frame) and a "not ready" message shows; the
slot fires the moment something frees up. The slot shows the action that would fire, a cooldown sweep and
the seconds left. Edit: drag items from the inventory window onto a slot to append them; right-click a
slot to set cooldowns, reorder, remove, add Attack or clear. Saved in `hotbar.ini` beside `mao.ini`.
The rules are in `hotbar.rs` with tests (priority, skipping, queueing, round trip).
Actions today: use an inventory item (message 52526858 with the inventory slot as target, type 0x68, as
the capture shows for backpack items; not yet confirmed live) and attack. Nano programs and special attacks
wait for their messages to be decoded. Cooldowns are client-side (the server's timer messages aren't decoded).

## 4g. Whom-Pah window (`B`)
Lists the booths of the area (their floating labels say where each leads, once learned), the distance, and a
**Go** button that walks the character to the booth (the server zones a character that walks into a booth: no
menu or message is involved, as the capture and the live tests show). Any movement key stops the walk; a new
playfield cancels it. (`371d0542` is `FightModeUpdate`, a list of area names, not destinations: it is no longer shown.) Tried live (character Beetlez, level 8, in Old Athens, `MAO_GOTO=x,z` starts the same walk as the Go button): it walks 70+ units across the plaza, steers round an obstacle (it stalled once at a wall before the detour logic) and arrives within a unit of the booth centre. The server did **not** zone that character at any of the booths tried ((444.2, 313.0) and (454.0, 313.0)), with `net walk` either, so whether booths need something this character lacks (level, a previous visit) or the trigger is elsewhere is open. Another test character is needed, or a capture of a booth being used in the original.

## 4h. Mission booth window (`M`)
Built from the real exchange in the third capture (see `GAME_WINDOWS.md`): six offer boxes with the mission icons,
seven sliders (Easy/Hard, Good/Bad, Order/Chaos, Open/Hidden, Physical/Mystical, Head on/Stealth, Credits/XP),
**Request Missions**, a detail pane for the selected offer (text, credits, xp, reward items with their icons, the
destination) and **Accept Mission** / **Cancel**, laid out as in the original. `M` opens it; Request uses the nearest
announced booth (within 40 units: the vending machines the server announces share their serial with the booth),
then sends `QuestAlternative` (scope 5, the value in the capture; `MAO_MISSION_SCOPE` overrides) and shows the answer;
Accept sends `CreateQuest`. The captured offers render correctly offscreen (`MAO_UI_PREVIEW=mission MAO_CAPTURE=<log>`).
**Not confirmed live**: the only test character (level 8, probably the wrong faction) gets a rejection from the booth,
so the request has only been checked against the capture; the slider ranges (difficulty 0-7, others 0-255) and the
scope's meaning are guesses from one capture; the mission log (`QuestFullUpdate`) isn't shown yet.

## 4i. Mail windows (`Ctrl+M`)
Two windows laid out as in the original's screenshots: the **inbox** (Messages, "Size: n/30", a Subject / Sent
list, Check mail, New, Delete, Close) and the **compose window** (To, Subject, body, Credits, C.O.D., an item picked
from the backpack, Standard / Express, Total cost, Send, Close). Check mail uses the nearest *known* mail terminal
(`mailbox::KNOWN_TERMINALS`: the server doesn't announce mail terminals, so the two in Old Athens come from
the captures) and asks for the inbox (`Mail`, action 1, byte-exact with the capture); the reply's message count is
shown. Send builds the captured message (action 6, recipient, subject, body, the item as `(0x68, slot)`, credits,
express; byte-exact in a test). Free accounts are refused by the server (the window says so when a notice follows
the send). **Not done:** (now done:) the inbox rows, reading, taking and deleting come from the fourth capture. Still not done: C.O.D. (the captured message has no field for it), the total cost (the server's). Needs a capture with a
paying account sending to, and the receiver opening, a mail with an item and credits.

## 5. The AOU mod against the original GUI
The original GUI folder (`cd_image/gui/Default`) is 50 files: 27 small XML
views (login, character selection, mail, trade, research, raid ...), 10 action
menus, the option panel, colour tables (`GUIColors.xml`, `TextColors.xml`) and
the skin archive `Graphics.uvga/.uvgi`. The big windows (chat, hotbars,
inventory, character) are built in code, not XML.

Each of the four AOU themes (Translucent, CYT, Slim CNI, Slim CNT) overrides the
same things: 30 original files changed (colour tables, 20 of the 27 views, 4
menus, the option panel, a gfx file), 20 new files (its own `Graphics.UVGA/.UVGI`
skin and 18 extra menus: AOU menu, pet, social, lag config, custom maps ...) and
17 original files left alone.

Its colours differ a lot (README claims silver and gold; the files say
otherwise): original `Default 0x80e9f3` light cyan, `Selected 0xffffcc` cream,
`Hover 0xa5ffdb` mint, `Text 0x99ccaa`; AOU Translucent `Default 0x051a2a` dark
navy, `Selected/Text 0x30d2ff` neon cyan, `Hover 0x40e6ff`.

**Integrated:** `theme.rs` reads any `GUIColors.xml` folder: `theme = original`
(the game's own), `aou_translucent | aou_cyt | aou_slim_cni | aou_slim_cnt`, or any
folder. It is switchable in the options window. The four AOU skins are bundled in
`assets/gui/` (copied from the user's install, `70dad3e6/Anarchy Online/Gui`, on
request, 2026-10-28) and are used before an `aou_mod = <folder>` copy. Note: the
AOU mod is GPL while this client is MIT; bundling it was the user's call.

**The skin's control center** (`gui/controlcenter.rs`): the skin's
`Views/ControlCenter.xml` is laid out over the screen as GUI.dll does
(FUN_1006f911), its views shown by their `activate_criteria`. The criteria are
Utils' expression language (`gui/expression.rs`, vendored from the drop-ins'
`ao-utils`: the scanner and parser run from Utils.dll's tables, over
`gui/variant.rs`) against the distributed values (`gui/dvalue.rs`: the game's
`Gui/Default/{Variables,MainPrefs,LoginPrefs,CharPrefs}.xml`, then the skin's) and
the player's stats (`stat:level`, named by Gamecode's `N3Msg_StringToStat` map,
`gui/stat_names.rs`). Its `CCMenu` views are the skin's menu scripts
(`ActionMenu/*.xml`, `gui/menu.rs`): a row of buttons (the AOU skins' View, Team,
Maps, Options, Skills, AOU; the Default skin's icon menus at the wings' ends),
whose menus open above them, entries shown by their criteria, with their actions
(`SetValue`, `CloseMenu`, `RunChatScript`). A menu entry's value is a window's
open flag: Inventory, Skills, Stats, Nano, NCU, Missions, Friends, Map, Actions,
Mail and Options open the client's windows. The docks GUI.dll fills itself
(FUN_1006f911) take the size of their pictures and are laid out with the rest;
`vitals::show_docked` draws the wings, the strips, the target buttons and the rollup
there. The health, nano, experience and alien experience bars are windows of their
own, as GUI.dll's `CharBarWindow_c`s are (FUN_1006b8d1): dragged anywhere, their
places kept in `layout.ini` under the original's names (`CCHealthBarConfig`,
`CCNanoBarConfig`, `CCXPBarConfig`, `CCAlienXPBarConfig`), shown by `cc_health_bar`,
`cc_nano_bar`, `cc_xp_bar`, `cc_alienxp_bar`, drawn from the skin's `GFX_GUI_HOR_BAR_*`
pictures with `value / max`. The compass (`compass_window`, by `cc_compass`) can be
dragged too and keeps its place. (2026-11-21, the user: the bars did not stay where they were put. A place asked for before the screen's size is known (egui's first frames) is put aside and the area marked to be moved once it is (`layout::relocate`), and `layout::remember` notes no area so marked; only `chrome::window` took the mark, so the bars shown from the first frames and the compass were never noted again and `layout.ini` kept only the alien experience bar, shown later. `char_bar` and `compass` take the mark now, go to their place, and are noted from then on. Not tried live. 2026-11-22, the user: the compass still did not keep its place; `layout.ini` held it at (2452, 1388). The game window's first frames have the settings' size (5120x1400 for the borderless 5120x1440 monitor), and egui keeps an area on the screen by default (`Area::constrain`): the compass, 1388 plus its height, was pushed up, and that place stuck and was noted. The compass and the bars are `constrain(false)` now; `vitals::keep_reachable` brings one back only when its whole rect lies off the screen. Not tried live.) The strips hold what GUI.dll's bar views hold: the
left one (`LeftBarView_c`) the NCU in use of the most (stats 180/181) and the credits
(stat 61, grouped), the right one (`RightBarView_c`) the AGG/DEF slider: the
aggression setting (stat 51, -100 to 100), which when let go is sent to the server as
`SetStat` (`6e5f566e`, `N3Msg_SetAggDef` → `N3Msg_SetStat(51)`, the format from a
capture). The wings lie under the strips (the AOU skins put the right strip on the
right wing). The rollup's dock
grows (it holds the `RollupArea` windows dock into), which puts the right wing at
the screen's bottom; the rollup folds the wings by `cc_left_wing`/`cc_right_wing`.
Each skin puts them where its XML says (the AOU skins' left strip is at the top
left). Without a skin the old plain strip (`vitals::show`) is drawn. Preview: `MAO_THEME=aou_cyt MAO_UI_PREVIEW=chat ... --image
x.png`, with `MAO_CLICK=x,y;x,y` to click (e.g. `25,707` opens View at 960x720).

## 6. Whompah labels (done for the mechanism; destinations learned)
The booth meshes (`whompa_entrance_default`) get a floating label. Which
playfield a booth leads to is decided by the server and is not in the client's
game data (the zone-line records for playfield 540 do not hold the booth's
coordinates as plain floats), so the client learns it as a player does: when the
server zones a character standing at a booth, the playfield it arrives in is
that booth's destination, kept in `whompah.ini`. Verified live: Old Athens'
booth at (444.2, 313.0) leads to Tir (640); the other three booths show
"Whom-Pah" until used. The three destination sign meshes
(`sign_jobe-whompa_borealis / old_athend / rome_green`) were rendered and show
matching names.

## 4j. Shop window (right-click a vending machine)

Vending machines (identity `0xc75b`) are the playfield's own (its dynel table, drawn with their template's mesh: the
server's `VendingMachineFullUpdate` names the same identity with no mesh, only `StaticInstance`). Using one makes the server
send `ShopUpdate` (the stock: low id, high id, quality) and a `Trade` open. The window is GUI.dll's `TradeView_c` in shop
mode (FUN_100e121e): the skin's `Views/TradeGUI.xml` view `ShopTrade` (`shop.rs`), with the three `ItemContainerView_c`
lists (FUN_100ce55d: Icon, Name, Count, Price, Quality) in `ShopInventoryDock`, `PartnerInventoryDock` (Bought Items) and
`OurInventoryDock` (Sold Items), each side's cost in `PartnerCashView` / `OurCashView`, and Accept / Decline.

* **Double-click** a stock item: `Trade` action 5 `(0xc75b, booth, 0x6f, index)`; **drag** an inventory item onto Sold
  Items: action 5 `(0xc350, you, 0x68, slot)`; double-click a row of either list: action 6 (remove). The server echoes 5
  and 6 (`capture-225436.log`), and the lists follow the echoes. Accept is action 1; the server answers 4 (complete) and
  the booth is used again; Decline (or closing) is action 2.
* **Prices** are the client's own, Gamecode's `VendingMachine_t`: an item's value is stat 74 between its low and high
  templates at its quality (FUN_10099991: FUN_100cb909's line, then FUN_100cbb68's curve). Buying costs value x stat 427
  / 100 less a Computer Literacy discount (min(CL, 3000) / 10 x 0.25 points, at most 90, in percent; only at shops of
  type 61, stat 156, the default) (FUN_10099e9b). Selling fetches value (divided by stat 212 when set) plus the same points
  in percent, x count x stat 426 / 100 (FUN_1009a12d). The shop's stats are its template's: Old Athens' booths
  (0x184f1) sell at 105 % and buy at 4 %.
* Preview: `MAO_UI_PREVIEW=shop MAO_CAPTURE=capture-221539.log` (two stock items in Bought Items).

## 4j2. The market (web page) and Deposit to Market

**Opening it:** using the market terminal (template 0x3eea9) runs its use event's spell 0xcf7b "open a client window" with
window 0 (Gamecode FUN_100a9cf4 → GUI.dll FUN_1002e2fe; `main::client_window` reads it from the template; window 8 is the
mail window). That opens GUI.dll's `BrowserWindow_c` in its market mode: an Awesomium page of
`http://aomarket.funcom.com/market/`. Here (`browser.rs`) it is a WebView2 page (wry) laid over the game window where the
"Market" window's frame leaves room, with Home, back and forward.

**Headers:** the original sends its site's every request the game's headers (`BrowserWindow_c::Initialize` 0x1010fdb8:
`X-Anarchy-CharacterID`, `-PlayerID`, `-DimensionType`, `-Cookie`, `-ServerID`, `-Appearance`, `X-Platform`). They come
from the login ticket (Interfaces.dll's reader 0x10002d11: the key is `s_nCookie1 s_nCookie2`, then `s_nEventServerType`
and `s_nPlayerID`; `mao_net::session::last_ticket`), the game server's frames' source, and stats 59, 4, 47, 64. The site
answers every request without them with nginx's 400. WebView2's own request hook is a COM call (and the workspace forbids
`unsafe`), so the page is served through a wry custom protocol instead: `http://aomarket.localhost/...` is passed on to the
site by `ureq` with the headers, and the site's addresses in its pages, redirects and cookies are made the proxy's (links
to the site's full address are sent through too). The page also gets the original's script object `AnarchyOnline`
(`AcceptLanguage`, `IsInGame`, `OpenMarketSendWindow`).

**Deposit to Market** (`marketsend.rs`, `MarketSendWindow_c`, `Views/MarketSendWindow.xml`): opened by the page's
`OpenMarketSendWindow` with the Alien Invasion expansion (stat 389 bit 8; else its feedback line). Eight item places take
backpack items dragged from the inventory (not equipped ones, NoDrop items or containers: the original's feedback lines)
and the credits field (held to the credits you have; negative refused); Send sends `MarketSend` (`470b2e14`: the character,
the credits, the items `(0x68, slot)`) and closes the window.

**The menus' pages:** the skins' "Item Shop" and "Daily Rewards" entries (`itemshop_window`, `dailylogin_window`; and
`petition_window`) open the same window in the original's other modes (1 the item store `http://aoshop.funcom.com/shop/`,
4 the daily rewards, 3 customer service), with the same headers (kept by the network thread, `browser::set_game_headers`).
At the time of writing the shop answers "temporarily unavailable" and the daily rewards site 502.

**Test without the game:** `mao-client <game> web store` (the item store page, through the same proxy) and `web market`
(the market's page with zero headers: the site's 400). `MAO_UI_PREVIEW=marketsend` draws the deposit window.

## 4k. Chat server and the chat window

A second connection to `chat.d1.funcom.com:7105` (`mao_net::chat`, decoded from the captures, login tested live): salt, login with the same challenge response, then names, channel lists, notices, tells and channel messages. Packet 30 is a **tell** (`u32 sender, str16 text, str16 blob`; the server's own "Players Online" notice arrives the same way, with a sender that has no name), 20 and 21 are names (21 is the answer to a lookup), 60 a channel, 65 a channel message. The client packets are the public AO chat protocol's, which agree with every number seen: 21 looks a name up, 30 sends a tell, 65 sends to a channel. **A tell to oneself came back as the same text from one's own id** (`net <game> chat-tell <from> <to> <text>`), which proves the lookup, the send and the receive layouts live. `MAO_NO_CHAT=1` disables the connection, `net <game> chat [seconds]` prints what arrives.

**The chat window** (`chatwindow.rs`, open at the start; `Enter` or `/` starts typing in it): tabs All, Tells, Channels and System; lines coloured by kind (system yellow, tells blue, channels pale, what you said green); an input line. Typing a `/` opens the original's list of commands above the input, filtered as you type (the 88 commands of the original's own list, in its order, `chat_commands::COMMANDS`; Up and Down pick, Tab or Enter takes the highlighted one). A line is sent with Enter:
* `/tell <name> <text>` (`/t`): looks the name up, then sends the tell; it is echoed as "To name: text". `/reply <text>` (`/r`) answers the last character that told you something.
* `/org <text>` (`/o`): the organization channel (the channel of kind 3 the server announces; "You are not in an organization." without one; not tested live, none of the account's characters is in an organization).
* `/help` lists what this client carries out; `/version`.
* Every other command of the list says it is not carried out yet; a word that is not in the list says it is unknown; plain text and `/say`, `/shout` (`/s`), `/whisper` (`/w`) are vicinity chat, sent on the game connection (see "Speaking in the vicinity" below); `/me` says it is not available yet.

The lines also appear in the on-screen log as before, and now survive a zoning (the connection belongs to the run, not to one playfield). Preview: `MAO_UI_PREVIEW=chat` (`MAO_CHAT_INPUT=/te` types into the input and shows the list; set `MSYS_NO_PATHCONV=1` in Git Bash).

**Chat groups and windows as the original's (2026-11-10, `chatgroups.rs`, `chatwindow.rs`).** In the original every
line belongs to a *chat group* and a chat window shows the groups selected in it. Read from GUI.dll:

- The client's own groups (FUN_10084436): System, Vicinity (the only one of them that can be talked to), Tell Messages,
  Your Pets, Other Pets and 23 combat groups ("Me hit by monster", "You hit other", "Your misses", "Me got XP", "Me Cast
  Nano", ...), ids 0x40000001 on; the chat server's channels are groups by their names.
- Gamecode's feedback formatter (FUN_10012c3e) sets the group with each fight line: `chatgroups::feedback_group` is that
  table by the text's name (a hit on the client by a monster or by a player, the client's hits, the others', misses,
  healing, experience, a cast). `feedback::line` notes the group and `Line::new(Kind::Combat, ...)` takes it. Where the
  original picks by whether the one hit is the client's pet, the client does not know and takes the other group.
- A window's config (FUN_1009aecc; the character's `Chat/Windows/Window<n>/Config.xml`): `name`, the selected groups'
  names, `output_group`, `show_timestamps`, `is_autosubscribe_window`. With none saved the original makes two
  (FUN_100950df): "Default Window" (everything but Other Pets and the fight) and "Combat" (the client's own fight).

The chat window's tabs are those windows now, in place of the client's own All / Tells / Channels / System / Combat: the
character's from the original client's folder when it has some (the account's newest character's otherwise, as the
window places), else the original's two; kept in `chatwindows.ini` beside `mao.ini` from then on. A group that turns up
later (a channel) is shown by the windows that select new ones by themselves. The right button over the window opens
its menu (FUN_100abb98) as the original's popup: "Talk to Channel" (the output group, among the window's groups that
can be talked to), "Subscribe Channels" (then "Public Groups" or "Combat Messages", each group with its check mark; the
list stays up while groups are ticked), "Autosubscribe new channels", "Show timestamps" (each line led by `(hh:mm) `,
the original's `(%H:%M) `), "New Window" (`Window<n>`, nothing selected) and "Delete Window". A plain typed line goes to
the window's output group when that is a channel the server has told of this session (`Out::Channel`), else it is
spoken in the vicinity as before. Seen live: the test character came up with "Default Window" and "Combat" from the
original's files. Preview: `MAO_UI_PREVIEW=chat` with `MAO_CHAT_TAB=<n>` and `MAO_CHAT_MENU=main|talk|public|combat`.

Not done: the original's sub-menus open beside their parent (here one menu replaces the other). (Several chat
windows on screen: see "Chat windows of their own" below; `/ch`, `/group` and the team's chat: "Chat group
commands"; the Visual menu: "Chat window looks and input bars".)

**Chat group commands (2026-11-10, `chat_commands.rs`, `chatgroups.rs`, `chatwindow.rs`).** As GUI.dll registers them
(FUN_1009e379): `/ch <group name>` makes a group the window's output group (FUN_1009c5b8), `/group` or `/g <group
name> <text>` sends one line to a group (FUN_1009c977), `/o <text>` goes to the organization's channel (FUN_1009cdfe:
the first channel of kind 3) and `/t <text>` to the team's (FUN_1009d028: kind 0x82). **`/t` was a tell here before;
the original's `/t` is the team, `/tell` the private message.** A group is named by the start of its name in any
case, a name with spaces in quotes (FUN_10083df7; the whole name wins alone). `/ch` takes only a group the window
shows; a group that takes no lines (the client's own but Vicinity, and a channel whose flags have bit 1, as the tower
battle outcome's 0x8006) is refused; of several that match, the read-only ones (and for `/ch` those not shown) do not
count, and when more than one is left they are listed in the original's colours (FUN_1009b144). A line sent to a
group the window does not show is warned of while the option `ChatWarnWhenSpeakingToUnsubGroups` is on. The answers
are the game's own texts (text.mdb 10001, `ChatCmdFeedback_*`), with this client's words without the game's files.
The team's channel is the chat server's: it tells of it when the team forms (packet 60, a four-letter name, flags
0x8004) and takes it away when the team is left (packet 61, kind and id: `Packet::ChannelGone`, new), as two captures
show; so it is a group like any other, can be shown in a window and be its output group. Tested on built packets and
typed lines; **not tried live** (the team's needs a second character).

**Chat configuration window and group colours (2026-11-10, `chatconfig.rs`).** The window menu's "Chat Configuration"
opens the original's `GroupControlView_c` (FUN_100ad81c, FUN_100a19ff): the chat windows listed at the left, and for
the selected one the tree of chat groups in three folders (Public Channels, Private Channels, Combat Messages), each
group with the original's check picture while the window shows it; a press turns it on or off. A channel the server
has not told of this session is in red, as the original's. Below: "Hide channels subscribed to other windows", the
four buttons New / Rename / Clone / Remove Window, "Log messages" (the client's one chat log: the original logs each
window by itself) and "Show message log" (dead: there is no log window). Rename asks for the name in a small window.
The selected window and the folders' state are kept (`tabs.ini`), Escape closes it with `esc_chatconfig`. The folder
rule is the original's (FUN_1008537d): the loot messages and Research count as public, only the client's groups under
0x4200001a as combat messages; the window menu's lists follow it too. A channel's lines are now in the colour
TextColors.xml gives its kind (FUN_1008582a: clan, tower, newbie, team and so on; vicinity speech in the vicinity's),
seen live on the organization's channel. Preview: `MAO_UI_PREVIEW=chat MAO_CHAT_MENU=config`. The buttons and the
tree were not clicked through live. Not done: a window row's own menu (style, show, rename, remove) and disbanding
one's own private group.

**Private chat groups (2026-11-10, `mao_net::chat`, `net.rs`, `chatwindow.rs`).** The friends window's "Invite" sent
the invitation but the client showed nothing of the group. Now, as GUI.dll's chat library reads and sends them
(FUN_1016d6f4, FUN_1016c84a's callers): an invitation that comes (packet `50`) is handled by the player's option
`ChatPGInviteAction` (FUN_100a7986): declined without a word from an ignored character, declined and said so at 0,
else told of in the chat and asked about in a message box ("Private Group Invitation"; Yes joins with `52`, No
declines with `53`). The original's box also lists the chat windows to put the group in, and at 2 it only marks the
invitation in the friends list: here the box always comes and the windows that take new channels show the group. A
group goes by its owner's name and counts as a channel of kind 14: its lines (`57`) show as `[owner] who: text` in
the private group colour, with the comings and goings (`55`, `56`), a declined invitation (`58`) and the end of the
membership (`51`, `53`) in the original's words; it appears under "Private Channels" in the chat configuration,
where the right button offers "Leave <name>" (FUN_100a1102), and a tab whose output group it is sends typed lines
to it. One invitation and its end are in a capture (packets `50` and `51` with the owner's id); the rest is from
the decompile and **not tried live** (it needs a second character on line).

**Chat windows of their own (2026-11-10, `chatgroups.rs`, `chatwindow.rs`).** In the original every chat window's
config names the frame it is in (`window_name`, "Window1") and its place among the frame's tabs (`tab_index`): windows
that share a frame are its tabs, and a tab dragged out gets a frame of its own. Here a window keeps its frame too
(`Pane::frame`, read from the original's config, a `frame <name>` line in `chatwindows.ini`; a file of before has
every window in "Window1"). The frame of the first window is the client's chat window with the input line; each other
frame is a window on screen with its own tabs and lines (named `Chat:<frame>` for its place in `layout.ini`), without
an input line at first (each has its own bar since "Chat window looks and input bars" below). In place of dragging tabs, the window menu has this
client's own "Move to a window of its own" (when the tab shares its frame) and "Move into the window of <first tab>"
for each other frame; "New Window" and the configuration's New and Clone make their window in a frame of its own, as
the original does (FUN_10094701: the first `Window<n>` free among names and frames). Closing such a window puts its
tabs back into the first one. Preview: `MAO_UI_PREVIEW=chat MAO_CHAT_MENU=split` (the second window in its own
frame). Checked in the preview only, not clicked through live. Not done: the places the original's own files give
the other frames.

**Chat window looks and input bars (2026-11-10, `chatgroups::Look`, `chatwindow.rs`, `chrome::bare_window`).** The
window menu is now in the original's order (FUN_10097a40 over FUN_100abb98): Visual, Talk to Channel, Subscribe
Channels, Autosubscribe, Chat Configuration (marked while its window is up; choosing it again closes it), Rename
Window (opens the configuration with the name box), Delete Window, New Window, then this client's two "move" entries.
"Visual" is the original's sub-menu (FUN_10097756 over FUN_1009a00d), each setting kept with the window (`look` line
in `chatwindows.ini`) and read from the original's config:
- **Mode** (`visual_mode`): Normal, or Borderless (2): the window without frame and title, only a background
  (`chrome::bare_window`, same place and size). Only for a window alone in its frame. The original's rule
  (FUN_10099a56) also compares the frame's name with a second name of the window that was not identified; here
  "alone in its frame" is the whole rule.
- **Transparency** (only when borderless; `window_transparency_active` 0.8 and `_inactive` 0.3 in the player's own
  files): how solid the background is while the window is active (typed in or under the pointer) and otherwise. The
  original has two sliders in the menu; here two lists in steps of ten.
- **Show timestamps** (moved here from the main menu, where the original does not have it).
- **Disable text input** (`is_textinput_enabled`) and **Hide input bar when inactive** (`hide_input_when_inactive`):
  every chat window has its own input bar now, with its own line, command list and history keys; what is typed goes
  to that window's shown tab (its output group, `/ch`). A bar that hides shows while typing; a press on the window
  starts typing in it. Enter still starts a line in the first chat window, even with its bar turned off.
- **Fade messages** (`is_message_fading_enabled`): while the window is not active a line fades once it is
  `ChatTextFadeDelay` old (8 s in the game's settings) over `ChatTextFadeTime` (0.3 s). The two options are the
  original's; how its `FadingTextView_c` uses them was not read, so the behaviour is this client's reading of the
  names.
- **Always behind / Always on top** (the window's flags 0x200 and 0x100 in the original): the frame drawn in egui's
  background or foreground layer; kept with the window here, not in the original's `VisualFlags`.
Previews: `MAO_UI_PREVIEW=chat MAO_CHAT_MENU=visual|mode|solid|borderless`. Seen in the previews (the menu with the
game's texts, a borderless window with its bar, the transparency list) and one live login (the two tabs and the bar as
before); **the switches were not clicked through live**. Not done: the configuration rows' own menu (click-through,
deactivate on send, default window), the menus' separators.

**NCU window's list mode (2026-11-10, `ncu.rs`, `gui/multilist.rs`).** The original's NCU window is the shared list
view (`NCUView_c` FUN_100d774b over `NCUListView_c` FUN_10045d42) and its right-click menu (FUN_100d758a) has "List
Mode": the grid of icons becomes a list under a column header. Ported: the menu entry with its mark (the choice kept
as `NCUWindowConfig` in `tabs.ini`, the original's `listview_mode`), and the eleven columns in the original's order
with the game's labels (text.mdb 10000 `Tab_*`): icon, name, type, nano cost (stat 407), NCU cost (stat 54, `---` for
999), time and time left (`hh:mm:ss`), radius, attack range (stat 287, `%dm`), attack delay and recharge delay (stats
294 and 210, seconds with two places), written as `InventoryListViewItem_c` writes them (FUN_1003d7d0). The list
starts sorted by the time left and a pressed label sorts by its column; a hostile nano (flags 0x8000) is "Hostile"
and its row red, a nano with the team cast spell (0xcf4a) "Team", the others "Friendly" (FUN_1003f4cf, FUN_1003d5a5).
Nanos whose flags have 0x1000000 are left out of the window in both modes, as the original does (FUN_100458b0). The
list view itself gained `set_list_mode` (`SetLayoutMode`), a row colour and coloured cells. Preview:
`MAO_UI_PREVIEW=ncu MAO_NANOS=<id:ms,...> MAO_NCU_LIST=1`. Seen in the preview only. Not done: the radius column
(`N3Msg_GetFormularRadius` reads a field of Gamecode's nano object that was not traced back to the record: `---`), the
"Self" type (the same object's self-only flag), the menu's "Auto Arrange" sub-menu, and keeping column widths and
the sort.

**Target NCU windows (2026-11-10, `ncu.rs::TargetNcu`, `net.rs`).** The original makes two more NCU windows beside
the target bars (`ControlCenterModule_c::CreateTargetMenus` FUN_1006a9b8, `TargetNCUView_c` FUN_100dda60): "Nano
Target NCU" for the selected target and "Fight Target NCU" for the one fought. Each shows while its option is on
(`cc_section1 && ShowNanoTargetNCU` / `ShowFightTargetNCU`, both off in the game's settings) and closing it turns the
option off (FUN_100dd390). Ported: the same list view, 167 by 112 at first, in list mode with the three columns icon,
name and time left, sorted by the time left; the menu's "List Mode" and "Hostile Only" (FUN_100dd94d), the latter on
to start with as in the original, so that only nanos flagged hostile and with an NCU cost are listed (FUN_100458b0);
no target, no list (FUN_100dd68f); a click on a nano shows its page. The two choices are kept per window (`tabs.ini`,
under the original's config names). Since nothing in the game's files offers the two options, `/setoption
ShowFightTargetNCU on` (or `ShowNanoTargetNCU`) turns a window on: `/setoption` now sets a yes/no value of the
original's own where the control center keeps them.
What runs on a target: one's own list for oneself; for another character the nanos its full update listed (with
their times) and those the server announces about every twenty seconds (`Buff`, which has no time: the time left is
counted from the nano's whole duration at the first announcement, so it is only right for a nano seen being cast).
Preview: `MAO_UI_PREVIEW=ncu MAO_NANOS=<id:ms,...> MAO_NCU_TARGET=1`. Seen in the preview only, **not tried live**. Not
done: the borderless style (Alt and a double click, FUN_100dd7ea), the windows' tab strip, and their places beside
the target bars. A window's title is now cut where its buttons begin (`chrome::window`), for these narrow windows.

**Nano programs window: list mode, menus and hidden programs (2026-11-10, `nanoprograms.rs`).** Read from
`NanoView_c` (FUN_100d5328) and its menus:
- The right button over a program opens its menu (FUN_100d49a0): "Look at" (its page; before, the right button showed
  the page at once) and "Hide", or "Unhide" in the hidden list. The original keeps hidden programs in a seventh list
  (school 6); here they are a set kept in `hiddennanos.ini` beside `mao.ini`.
- The right button elsewhere in the window opens the window's menu (FUN_100d4f90): "List Mode", "Show Hidden" (shows
  the hidden list, FUN_100d4b49(6); a school's button leads back) and "Auto Arrange".
- List mode puts the school's programs under a column header with the original's ten columns and the game's labels:
  icon, name, nano cost, NCU cost, time, radius, attack range, attack delay, recharge delay and the stacking line
  (the nano strain's name from text.mdb 2009, sorted by strain and place in it, FUN_1003dee6), written as the NCU
  list's are. It starts sorted by name; a pressed label sorts by its column.
- "Auto Arrange" (FUN_10041637) orders the grid: "Unsorted", "By <column>" for each column with the current one
  marked, and "Sort Ascending". "Don't Auto Arrange" is shown dead: programs are not placed by hand here.
- Kept between starts (`tabs.ini`): list mode (`NanoViewConfig`), the school shown (`NanoViewSchool`, the original's
  `selected_school`) and the grid's order (`NanoViewGridSort`).
The original has no find field in this window (the earlier note here that it was missing was wrong). Previews:
`MAO_UI_PREVIEW=nanos MAO_PROGRAMS=<id,...>` with `MAO_PROGRAMS_LIST=1`, `MAO_PROGRAMS_SCHOOL=<0..6>` and
`MAO_PROGRAMS_HIDDEN=<id,...>`. The list was seen in a preview; **the menus were not clicked, and nothing was tried
live**. Not done: programs placed by hand (`item_position_map`), the radius column's value (`---`), keeping column
widths and the list's sort (`listview_config<n>`).

**Perk window's status icon and timer (2026-11-10, `perks.rs`).** The two pictures the window drew as stand-ins are
the original's now. The numbers the decompile gives for them are named by AFCM.dll's table (the `gfxid.py` scratch
tool): 0x121 is `GFX_GUI_PERK_STATUS_ICON` and 0x14f to 0x158 the `GFX_GUI_PROGRESS01` set. A perk row's status icon
(`StatusIcon_c` FUN_1005ffe0) is that 11 by 11 picture in the status' colour, placed as a button places its icon
(`Button_c::LayoutLabel` 0x10127d88: 5 in from the border's 5, a pixel above the middle, the label a pixel and 5
after it, so the label moved 5 to the right). The untraining timer is `TimeBar_c` (FUN_10060920): the progress bar's
border pieces over its background, the front picture inside the border from the left over the share of the wait
(FUN_10060354), and the time in white on it (a plain `TextView_c`, FUN_1006063d). Seen in the preview
(`MAO_UI_PREVIEW=perks MAO_PERKS=<ids>`); a running wait was not seen (it needs a perk untrained live). The parity
table's note that criteria operator 108 fails was out of date: it passes, as on the original's client
(`requirements.rs`).

**Special actions from the server's list (2026-11-10, `actions.rs`, `net.rs`).** The Action window's list was this
client's catalogue for every character. How the original's list comes to be was read from Gamecode:
- Every special action has a number and an item template that names and pictures it (FUN_10042b17's table, now
  `actions::TEMPLATES`): character action codes (use, pick up, start/stop combat, walk/run, sneak, crawl, sit/stand,
  camp, reload, search) and the stats of the special attack skills (brawl 142, dimach 144, fast attack 147, burst
  148, fling shot 150, aimed shot 151, full auto 167, sneak attack 121 and two more).
- The character's own actions are added when the character is made (FUN_1004326f), in the order the window already
  shows them.
- The server's `SpecialAttackWeapon` message (`1d3c0f1c`, sent for every character about) lists a character's special
  attacks: an item, a stat and a four-letter code each. Each of one's own whose stat is in the table becomes an action
  (FUN_1006b14c). Every capture lists the plain attack (100, no action), Dimach and Brawl. The client now reads it:
  Dimach and Brawl show only when the server lists them, and a listed special the catalogue does not have (aimed
  shot, fast attack, sneak attack, ...) appears named and pictured from its action's template and is sent as the
  others are (`n3::special_attack` with its stat).
- The perk actions come with the server's 0xb4 list, as before.
- **A weapon's own attacks and Reload (2026-11-10, second pass).** The server lists none of them: a soldier holding
  a gun is sent the same three as everyone (seen live). They come from the weapon: when the client's own character
  puts a weapon in a hand, `WeaponItem_t` (FUN_1009e84b) adds an action for each of these bits of the weapon's "can"
  flags (stat 30), with the weapon as the owner: 0x800 burst (148), 0x1000 fling shot (150), 0x2000 full auto (167),
  0x4000 aimed shot (151), 0x20000 sneak attack (146), 0x8000 the bow's special attack (121), 0x40000 fast attack
  (147); and Reload (110), owned by the character, when the weapon's ammunition stat (26) is not -1. Taking the
  weapon off (FUN_1009d39a) removes what the weapon in the other hand (places 6 and 8) does not give too. The client
  does the same (`actions::WEAPON_CAN`, `World::special_numbers`): the Action window shows Burst, Fling Shot, Full
  Auto, Aimed Shot and the others only for a weapon in a hand that has them, and Reload unless every weapon held
  says it takes no ammunition (the stat the server tells of the weapon in the hand, else the template's). Before,
  Burst, Fling Shot, Full Auto and Reload were shown to every character.
- **A special attack's hit line.** The server answers a special with `CharSecSpecAttack` and `SpecialAttackInfo`
  (place, amount, rounds left, target, the skill's stat) and no plain attack message, so the hit was told only by
  the health that followed, as nanobots of unknown damage, and not at all to the attacker. Gamecode FUN_1006af0e
  tells it as a weapon hit with the skill's name (text category 2003) where the damage type is (a burst's 176 points
  are "Burst" damage). Now `feedback::special`; the rounds left go to the weapon as the original's do (FUN_10068869).
Tried live between the two test characters in a duel (2026-11-10): a Neutrino Flash gives Burst and Reload; a Kyr'Ozch
Energy Pistol with an Electrical Surge Pistol gives Burst, Fling Shot and Reload; Burst and Fling Shot hit and are
told on both sides, a second Burst inside its recharge gets the server's "wait" text, Reload reloads. Backstab
(489) is added beside a weapon's sneak attack for a character the server has allowed it: the server's
`CharacterAction` 0xb5 with its second parameter not zero (FUN_1005d61f's case 0x44, FUN_10068976) sets bit 20 of the
character's second flag word and has each hand's weapon add it (FUN_1009c5c8; putting such a weapon on later adds it
too, FUN_1009e84b), and nothing takes the bit away again. Done the same way (`World::backstab`,
`actions::BACKSTAB`), from the decompile only: no capture has the message and neither test character is sent it, so
which characters get it and whether the attack goes off as the others do is unknown. Not tried: aimed shot, sneak attack, the bow's attack, fast attack and full auto are sent as the others are but
no character had a weapon for them; the actions' places kept (`item_position_map`), the toggles' other names (sit
for stand, run for walk), and the profession's own action (numbers 138 and 139, FUN_1004326f).

**Duel (2026-11-10, `chat_commands.rs`, `n3::duel`, `net.rs`, `ui.rs`).** `/duel` asks the targeted player
(a line saying so without one, in this client's words), `/duel accept`, `reject`, `stop` and `draw` answer and end one:
all five are Gamecode's `N3Msg_Duel_*` (0x1001d335 on), one `CharacterAction` 0x106 with the challenged character as
the target and 0, or no target and 1 accept, 2 refuse, 3 stop, 4 draw. The server sends each of the two the same
action back with the other character, what and a flag, which Gamecode FUN_1005bd68 turns into the boxes and the
texts: a challenge (0) with flag 0 is one received (GUI.dll `DuelChallengeReceived` 0x100308ca: the "Duel Challenge"
box with Accept and Reject, closing it rejects, and the text as a chat line; with the `AutoRejectDuel` option on, a
refusal with the second parameter 1 and nothing shown), with flag 1 one's own (`DuelChallengeSent` 0x10030aae: the
box with Cancel, which sends a refusal, not a stop); accepted (1) and refused (2: by the other, taken back, by the
other's option) take the box down; ended (3: lost, won, a draw) and a draw proposed (4: by the other, one's own) are
texts only. The texts are the game's (`Feedback_Duel*`), with this client's own words when the game's are missing.
The box comes up at the pointer, as every message box does. Tried live between two of this client: every message
above but the automatic refusal was seen with the flags as read. What the live fight showed: the server sends a
weapon hit between players twice, as the attack (`AttackInfo`) and as the health it left (`Stat` 27), and the second
was told as "attacked with nanobots ... unknown damage"; now a health loss is told only as far as it goes beyond the
hits told in the last two seconds (`World::recent_hits`, `untold_loss`: this client's rule, the original's own was
not read). The test script got `target@<character id>` (targets a character without a click, for a second client
that must only ever challenge the first) and `pginvite@<name>`; a `move@x;y` before a box is due puts the box there,
so that its buttons can be clicked (Accept 90 px left and 83 px below the middle, Reject 64 px right).

**Pet duel (2026-11-10, `chat_commands.rs`, `n3::pet_duel`, `net.rs`, `feedback::pet_duel`).** `/petduel` asks the
targeted character for a duel of pets, `/petduel accept`, `reject` and `stop` answer and end one (GUI.dll
FUN_100b8d78, which asks only that the target is a character). Gamecode's `N3Msg_PetDuel_*` (0x1001d083 on) send a
`CharacterAction` each: 0xef with the challenged character as the target; 0xf0 with the first parameter 1 to accept
and 0 to refuse; 0xf1 to stop. There is no box: the server's messages are told as texts (Gamecode FUN_1005ca5b,
category 110 by name, with the other's name when the client knows the character): a challenge received (0xef); an
answer (0xf0) by its parameter, turned down 0, taken 1, not someone to ask 2, in a duel already 3, nobody asked 4, no
duel pet 5, the opponent has none 6; a result (0xf3), won 0, lost 1, the opponent left 2; one's own challenge on its
way (0xf8). Tried live between two of this client: a challenge gets "no duel pet" (5) and an accept or a reject with
nothing asked gets "nobody asked" (4), both with the game's texts; a stop with no pet duel on gets no answer. An
engineer's automaton does not count as a duel pet (the same answer with it out), and the game's item and nano
names have nothing called one, so a pet duel itself could not be had: the challenge received, the answers 0 to 3
and 6 and the results are from the decompile only. The test script got `cast@<nano id>` and `useitem@<place>`
(the automaton's program leaves a shell in the inventory, which is used to bring the pet).

**No target on entering, and text boxes that take the keys (2026-11-10, `net.rs`, `chrome::focus_when_new`).** Two
things the third round of live tests found. A nano run with nothing selected goes out with no target, as the
original's does (`N3Msg_CastNanoSpell` 0x1001b5d4 passes the selection as it is; only its own check, FUN_1004fbe4,
takes the caster for a harmless nano's missing target), and the server then uses the target it knows of. A
character whose client stopped without logging out stands in the world for a while with its target, so a session
that takes it over could buff somebody it did not show as selected. The client now sends `LookAt` with no target
once on entering and whenever a target is lost; what the original sends on entering was not read. And a box that
asks for a text (a bag's or a chat window's new name) takes the keyboard when it comes up, as GUI.dll's
`TextInputDialogForm_c::AllAttached` (0x101483d8) gives its field the focus: before, the name could only be typed
after a click in the field.

**Team: asking somebody in (2026-11-10, `team.rs`, `n3::team_invite`, `net.rs`).** The original has no chat command
for it (`/invite`, `/kick` and `/leave` are the private chat group's, handed to the chat server as they are typed;
`/team` is a menu script). It is the team window's Recruit button: GUI.dll FUN_100785d7 builds the window's button
row, without a team Recruit at the left, in a team Leave at the right and Recruit after it for the leader, and
Recruit (FUN_100782b2) asks the selected character in with Gamecode's `N3Msg_TeamJoinRequest` (0x1001b9b2: a
`CharacterAction` 0x1a naming them; nothing is sent for one who is in the team already). Its three lines are the
game's texts: nobody selected, oneself selected, and the request sent with the name after it. The client does the
same. Tried live between two of this client: the three lines; the invitation arrives on the other side; with its
Yes the window lists both members with the team's level, the leader's name in its colour and Leave and Recruit;
`/t` lines arrive on both sides as the team's channel; after Leave the window is back to "no team" and `/t` gets
the game's text for not being in one. Found on the way: the server tells the other members' health and nano but not
one's own, so one's own bars stood empty; they are filled from the character's own stats now (this client's
reading). A member's view takes the mouse as the original's does (FUN_1007a23a): a click makes the member the target,
a right click brings a menu with the game's texts, "info on" the member (its page in the info window) and, for the
leader on another member, making it the leader (`N3Msg_TransferTeamLeadership`, action 0x19) and putting it out of
the team (`N3Msg_KickTeamMember`, 0x16; Gamecode sends neither for oneself). Tried live: the leader's menu has the
three entries and a member's only the first; after "make leader" the other's name takes the leader's colour and one's
own Recruit button goes; after the kick the window is back to "no team" (a team of two is gone with it). Not done:
the team search window's invite (FUN_100f08c8), the raid window's same menu (FUN_10102740) and the team's side in the
header. The test script got `teamreply@yes|no` and `teamleave@`.

**The playfield map shows the ground (2026-11-10, `automap.rs`).** The round map in the corner showed the planet
map's picture for a playfield that is on it. The original's playfield map (`playfield_map_view`, the "PF Map" window)
is not that: GUI.dll's `PFMapGroundRenderer_c` (FUN_10047964) draws the ground from above, each ground patch's own
texture (`AnarchyGroundData_t::CreateHiTexture`, with a second texture of the patch's when it has one) multiplied
over the patch's shadow texture (`AnarchyGround_t::GetShadowTexture`) lightened with white at 0xbb. The client now
builds that picture when a playfield loads (`automap::ground_picture`: every cell's tile texture, turned as the
ground's model turns it, shrunk so that the whole ground is at most 2048 pixels across; Old Athens is 1792 by 1792
and takes about 23 ms) and the corner map and the overlay show it. The planet map's picture stays the planet map
window's, and stands in on the corner map only if a playfield on it has no ground; interiors keep the walked map.
The shadow layer (second pass, same day): the patches' shadow textures are made by `AnarchyGround_t::SetShadowMap`
from a 4-bit map that Gamecode blends, by the time of day, from six stored maps of the playfield (FUN_100b7c29:
resource type 0xf4247, six PNGs behind a table of their ends; FUN_100b7fca picks two by eight spans of the 97200
second day and `BlendShadowMaps` mixes them). Today's game files have no object of that type at all (the database's
types were counted), so the original finds no stream, never sets a shadow map, and no patch has a shadow texture:
there are no ground shadows to draw, on the map or in the world. What is left of the layer is the white veil at 0xbb
over the map's background, which the ground's textures are multiplied over: the ground at 0xbb of its brightness,
taking the background for black (not checked). The corner map is dimmed so (`automap::VEIL`). Not done: the patch's
second texture, and the rooms' floor pictures the same view draws for playfields made of rooms (FUN_100474b7). Seen
live in Old Athens: paths, lawns and the plaza where they are around the character.

**A built-in thing's root animation (2026-11-10, `mao_formats::model::walk`).** The ring that marks a shuttle's door
(Arete Landing's "Shuttle Door", mesh 258035, one frame) stood on its edge and still. Its mesh is modelled upright and
its root frame's animation lays it down (a quarter turn about X, 5 cm up) and scrolls its texture once round in 3.33
seconds, which shows as a band running outwards over the ground. The client left the root's whole transform out for a
playfield's built-in things, because the game overwrites the root's local position and rotation with the thing's place;
but the animation's matrix is no part of that: randy31's `UpdateWorldMatrix` puts it in front of the local transform
(`refframe.rs`), and a frame saved with an `anim_matrix` and no animation keeps the matrix too. The root's matrix and
keys are kept now. Thirteen of the 656 meshes the built-in things use have one (`dynelroots` lists them): this ring,
three posters, wall panels, two buttons, a terminal, a tower's teleporter; they were drawn turned or upside down.
Seen live: the ring lies under the shuttle's door and its band runs outwards. Not right yet: the band's edges are
hard. The material asks for alpha blending (render state 27) and its texture's alpha fades from 5 to 208, but the
client cuts every textured mesh out at half alpha; 2327 of the 11228 meshes have such a material (`blendcount`), 259
of them adding their light (source alpha, one). Blended world materials are a piece of the renderer of their own.
New tools in `mao-formats/examples`: `meshframes` (a mesh's frame tree and animation keys), `meshparts` (its parts
and materials), `meshstates` (its materials' render states), `dynelroots`, `blendcount`.

**When a thing's model plays (2026-11-10, `net::prop_anim_time`, `World::prop_anim`, `main::static_prop_anims`).**
The client played every machine's and thing's animation round and round on one clock, so Arete Landing's cargo
crates swung their doors open and shut by themselves. The original steps a thing's model from two of its stats
(Gamecode FUN_10088970, called each frame from the things' step FUN_10089f0a): `AnimPlay` (501) 2 goes forward for
ever and 4 backward, 3 goes forward once and stays at the end, 5 backward once and stays at the start (then
`AnimPlay` becomes 1), and any other value leaves the model alone, in its first pose or, with `AnimPos` (500) 2, its
last (FUN_100877ec). Opening a thing is `AnimPos` 1 with `AnimPlay` 3 and closing it `AnimPos` 2 with `AnimPlay` 5
(FUN_1008563d, FUN_10085684). The client does the same: the stats are the built-in record's own, else its item
template's; for a thing the server announces, the message's, else the template's; and a later `Stat` message naming
either starts the play from then. In Arete Landing nineteen built-in things have `AnimPlay` 2 from their templates
(the shuttle door's ring, the signs, the shop machines) and go on playing; the cargo box is announced with both at 0
and stands shut. Seen live: the crates' doors are shut. The original's capture shows no stat told of the box when it
is opened (the box is taken away about 110 seconds later), so whether its doors ever open in the original is not
known. Doors keep their own way (their state's time). Not checked: the statels' moving parts still all play on the
clock, and the `Scale` stat (360) the same class reads for the body's scale (the shuttle door's template has 50, which
would make its ring 2 m across, not 4).

**The item database is there before the first message (2026-11-10, `net::live`).** Found on the way to the cargo box:
the things the server announces while the character enters are named and drawn by their item templates, and the
database those are read from was attached only after the entering. In the first playfield of a run they had neither
name nor model and could not be picked or used (the cargo box and eight pieces of junk about it); after a zoning they
were there. It is attached before the first message is read now. Seen live: the box is named, selected and opened
(the test hook's `usenear=<name>` also finds the announced things), and the quest moves on.

**A text's choices (2026-11-10, `mao_formats::ldb::format`).** A quest's reward line showed its text's markup, the
part that picks the singular or the plural by the number of credits. The original's `LDBformat` (the ldb drop-in's
`format.rs`: `Init`, `FeedInternal`) cuts a text into pieces, each filled by one argument: a `%` conversion by the
next argument, and `#N{flags:text|text}` by argument `N`, which picks the alternative with the most of its flags (a
number 0 has the flag `0`, 1 the flag `1`, any other none; the last of equals wins); `#N:` makes the next `%`
argument `N`. `format` is that now (it filled the `%` pieces in order and knew no `#`). The spaces about the bar
belong to the alternatives, so a plural comes out with two spaces before it, as the original's code makes it. Not
seen live yet (the reward was taken before the fix).

**A built-in machine's id (2026-11-10, `World::used`).** A use of one of a playfield's own machines was answered
"too far away" from 4 m (Arete Landing's ammunition machine; the mission booth of an earlier test may be the same
fault, not tried again). Where the server gives the playfield's built-in things ids of its own (the playfield
message's runs: a kind, the first record's place in the table, how many, the first id), the client took a record's
id as the first plus the number in its table id (`0xc0NNpppp`). It is the first plus the record's place among its
kind in the table: Arete Landing's machines are numbered 0 to 4 and 6 to 8, so machine 6 is the sixth record, and the
use went to the seventh, 90 m off. The original's capture has the machines announced with their places, which shows
the order. Seen live: the shop's window opens with its list and prices, and a double click (one click since 2026-11-18) on a row and Accept buys
it (100 arrows for 105 credits, in the inventory). Left: the on-screen line names the machine by the id used
("Booth d831") and not by its name.

**A new level (2026-11-10, `World::add`, Gamecode FUN_10075f55).** The experience bar was empty after a level until
the next login. The server tells a level with a message of its own (`NewLevel`, 0x7f405a16: the level, the points to
spend, the experience, where the level began and what the next takes, the reset points gained, the range of levels
that give experience, the last award), and the original puts six of them into the character's stats (54, 53, 52, 57,
350, 275); no `Stat` message tells 57 and 350. The client only played the jingle. It sets the stats now and tells
the level (`Feedback_NewLevel`) and, below level 4, that there are points to spend. Not ported: the line for reset
points with the level's title, the `got_ip`, `got_perk` and `got_tech` hints. Not seen live yet (the next level is
3100 experience away); the test replays the message of the level it was caught at.

**Things that go, NPCs that are busy, and following (2026-11-10, `net.rs`).** From the same quests: a thing the server
told of is removed when its `Despawn` comes (a gas fire put out went on burning on its crate); an NPC that answers
a use with a spoken line and no window is not asked a second time (the client took the silence for a chat left open
by an earlier run, closed and opened it again, and the NPC said its line twice); a walk to a place lets go of a
target followed until then (the walk never left the target's side); and the walk's on-screen line no longer calls
every goal "the booth". Checked against the original and left as it is: two lines an NPC says one after the other are
joined without a break (FUN_10058ee7 appends a second line of the same kind with neither a break nor the name).

**Blended materials (2026-11-10, `mao_formats::model::Blend`, `mao-render` `blend_of`, `mesh.wgsl`).** Every textured
mesh was cut out at half alpha, so what the original blends had hard edges: the shuttle door's ring, glass, glows, the
landing decks' floor panels. The original: a material read from a mesh is transparent when its stored delta state
turns `ALPHABLENDENABLE` (27) on (randy31 `material.rs`); the device's blend is source alpha over one less source
alpha, depth writes on, no alpha test (`randy.rs`), unless the material's states say otherwise (20 = 2 adds it, 14 = 0
writes no depth, 15 with 24 tests the alpha); a transparent visual goes on render list 6, which DisplaySystem draws
after the others and far to near (`AddToRenderList`'s distance buckets, `Render(5, 6, .., 0x707, 0)`). 2327 of the
11228 meshes have such a material, 259 of them added. The client reads those states into the material and draws the
blended parts after the opaque ones, far to near by each part's own distance (not in the original's buckets, and not
by the visual's origin), with the alpha of the texture times the material's opacity. Checked against the original's
capture screenshots: the ring is a soft disc with a brighter rim growing from its middle in both, and the landing
decks' floor panels are see-through in both. Two things are this client's own. Plants stay cut out as before:
blended, their crowns thinned to a veil with distance (the texture's smaller levels average the alpha down) where
the original's trees stand dense; why the original's do not thin was not found (whether its plant textures have
smaller levels was not read). And a blended pixel whose alpha is under 0.02 is not drawn, so that it writes no depth;
the original writes it. `MAO_NO_BLEND=1` draws everything cut out as before, to compare. Not done: the meshes of one
visual drawn far to near where the visual asks for it (`sorted`), the frame's own transparency fade through the same
list, the environment-mapped pass.

**The control center's bars (2026-11-11, `vitals.rs::char_bar`).** The health, nano and experience bars were lying
bars under the top of the screen with the coloured part off its frame. They were a guess: the original's
`CharacterBar_c` (GUI.dll FUN_100673ba) is a `PowerbarView_c` that fills upwards, built from four pictures each
(`GFX_GUI_ACTIONVIEW_HEALTHBAR`, `_BACKGROUND`, `_LEFT`, `_RIGHT`, and the same for `NANOBAR`, `XPBAR` and
`ALIENXPBAR`: its bitmap numbers 9 to 0x18, named by AFCM.dll's table), 11 by 125 with the end pieces, and it shows
the value and the maximum as its tooltip (FUN_10066f7a), not as a text on it. The client draws that now, the full
picture in the middle of the background's size as `PowerbarView_c::Recalculate` puts it. With no place kept they
stand in the wings of the control center, health and nano in the left and the experience bars in the right, on the
menu strip's top, following the control center as it is laid out (their first frame's place was the not yet laid
out wing's); a place the player dragged one to is kept as before. A place kept for the old lying bars is dropped
once (`layout.rs`, the `upright-bars` mark). Where the original's own default layout stands them was not read: the
user's original client runs another skin, whose screenshots have no wings.

**A wielded weapon in the hand (2026-11-11, `net.rs::overlay_model`, `mao_net::n3::EntityInfo::meshes`).** A character's
weapon was not drawn at all unless the server's appearance update happened to list a mesh for the hand, so one's own
rifle was missing from the moment of entering. Gamecode's FUN_1009cda2, run when a weapon is wielded, adds the mesh
the weapon's stat 209 names to the character's mesh at attractor place 1 for wear place 6 and 2 for place 8
(`CharacterMesh::AddAttractorMesh`), in a fight or out of one. The client does that for every character whose
weapons it knows (the server's weapon updates, or one's own inventory). Stat 12, tried first, is the box an item
lies on the ground as. Also read now: the meshes a character's full update lists after its clothes (ten bytes each:
position, mesh, texture, layer), which were only taken from the appearance updates, so hair and shoulder pieces are
there from the first sight of a character. Seen live: the rifle in both hands on entering. Not ported: the mirrored
side for a left-hand weapon with the can-flag 0x800000, the social places.

**One's own death (2026-11-11, `World::own_died_of`, `feedback::death`, `net.rs`).** What happens, from two live
deaths of the test character with the trace on: the hit that takes the last health comes with a death reason; the
character's `CharacterAction` 99 follows with its death animation, and the creatures stop fighting it; it lies dead
for 60 seconds; then the server sends `CharacterAction` 0x99, the remains of its body, a teleport and a playfield
message, and it stands on the arrival pad with part of its health. (An earlier note here took the last step for the
death.) The original: the reason is the word after the target in `AttackInfo` (the schema's `Unk1`; Gamecode
FUN_1009f257 and FUN_1009b6b7, and the same in the special attack's and the health damage's messages, FUN_1006af0e
and FUN_100a0611), handed to FUN_1005b3d8, which tells the client's own character one of seven of the game's texts
(1 terminated, 2 reflected damage, 3 a damage shield, 4 a weapon, 5 a nano program, 6 a fall, 7 a liquid) and sets its
health to nothing; the interface is told of the death and of the life again (GUI.dll `FlowControlModule_t::DeadMessage`
and `AliveMessage`: the alive flag of the input, a signal to the windows), and the sound module gets a death music
mode. The client had the dead state already (the death's animation, no moving until health or a playfield comes) but
told nothing, kept the health the last message said, and let the attack and use keys through: in the first live death
the test's attack orders went out for the whole minute and the server took them, shots, kills and experience
included. Now the reason's line is told once (seen live: the game's own text for a weapon's kill), the health is
nothing, and while dead the attack and use keys do nothing and nobody is followed. Seen live: the body lying with
its rifle beside it and the health bar empty for the minute, then the pad. Not ported: the reason in the special
attack's and the health damage's messages, the death music mode, the windows' reaction to the dead signal, the
line and stand-up of a resurrection (FUN_1003ec1f: from level 15 the game's text about the resurrection shock),
what `CharacterAction` 0x99 itself means. A hit that lands as a creature dies is still told with the remains'
name, since the name is the corpse's by then.

**A creature's size, and the bosses that were boxes (2026-11-11, `mao_net::n3::npc_template_and_scale`,
`net.rs::overlay_model`).** The junkyard's boss robot of Arete Landing was drawn as the client's placeholder, a blue
box; the user said what it was and sent the original's screenshot of its remains (a big yellow box of a robot with a
red rod). Its full update, caught with a new trace, has an ordinary cleaning robot's template followed by 600: the
two bytes after a creature's template are its body's scale in percent (in the original's capture 75 to 200 for the
ordinary creatures, 600 for these bosses, nine times), and the template was found by a pattern that took the scale's
high byte for a zero, so a creature of 256 percent or more had no template and no model. The pattern takes any scale
now, and the scale is kept and used: a creature is drawn at that share of its template's size, as the original's
dynel takes its `Scale` stat for the body's scale (`SetBodyScale`, Gamecode FUN_100892ca). Until now every creature
was drawn at its template's size, the cleaning robots at half of theirs. Seen live: the boss as in the original's
picture. (Other players and the humanoid NPCs have theirs since 2026-11-18: a player's update is read for it, see
"Other players are drawn at their own size".) Not done: the remains of a scaled creature (not looked at). `MAO_COMBAT_LOG=1` prints a "no template" line with the bytes of
any creature's update that gives the client neither a template nor a humanoid body, and a "no model" line for
anything drawn as a marker; `creaturecheck` says whether every creature record's body reads (all but two do).

**The launcher, its settings and the hall behind the login (2026-11-11; `launchwindow.rs`, `dimensions.rs`,
`hall.rs`, `mao_formats::pe`).** `mao-client <game> launch` opens the window the original's `Anarchy.exe` opens before
the game, built from that program's own parts, all read from the game folder as it runs (none is in this
repository):

- The window is the original's 794 by 575, not resizable, one of the ten pictures of
  `cd_image/data/launcher/skins/backgrounds` over all of it. Which one the original shows was not read (one by the
  clock here).
- **A background of this client's own (2026-11-12, asked for by the user; `launchskin.rs`).** The settings' GUI
  page has a Launcher group: Background is "The game's pictures" (as before, and the default), "Stars and the steel
  emblem", or "A picture of your own" (a file, drawn over the whole window); and a box for the game's news page,
  which can be left out. The star background is drawn by the client from a seed, the same every time and at the
  window's own pixels: a dark sky, a band of blue and violet cloud across it, some fifteen hundred stars with the
  brightest few spiked, and 48 more that change their brightness slowly. On it is the game's steel emblem,
  `GFX_GUI_AO_LOGO` of the game's own skin (`cd_image/gui/Default`), read from the game folder when the launcher
  opens, its green key taken out, with a blue light behind it: large in the page's place when the page is left
  out, small at the left of the strip under the page otherwise. Nothing of it is a file of this repository. Kept in
  `launcher.ini` as `background = game | stars | <a picture's path>` and `page = on | off`. Without a game folder
  the sky has no emblem; a picture that cannot be read gives the game's. Seen in pictures of the test launcher
  (both layouts and the settings' page); not yet by the user.
- Its upper 428 pixels are a web page: the original embeds a browser control at `SHOWURL` of
  `cd_image/data/launcher/AnarchyLauncher.url`, and the banner, the row of links, the customer service messages and
  the tip of the day are all that page's. Here the same page in a WebView2 child (wry); a link that asks for a new
  window opens in the system's browser. The page is opened after the window's third frame: opened before the first,
  the window's own drawing never showed under it; and the window asks for its frames from `about_to_wait`, since
  asking from inside a frame alone stopped once the page was there.
- The dimensions' list is downloaded as the original does it, from the address in `DimensionServer.url`, and a
  copy kept beside the settings (`dimensions.txt`) for when the server does not answer. Each dimension has a login
  server and a version; the one chosen last is selected (`launcher.ini`). The window's title has the game folder's
  version (without the expansion's mark after the underscore) and the dimension's, as the original's has.
- SETTINGS and PLAY are drawn with the pictures in the program's resources (`RT_BITMAP` 176, 178 and 187 for PLAY
  as it is, under the pointer and switched off; 181, 182 and 186 for SETTINGS), read by a small reader of a program
  file's resources (`mao_formats::pe`); plain buttons when the folder has no `Anarchy.exe`.
- PLAY starts the login (`mao-client <game> play`) with the dimension's server and the launcher goes.

The settings are the original's three pages (the program's dialog templates 153, 161 and 107) with this client's
own contents where the original's mean nothing any more: the rendering device is the graphics device wgpu draws
with (`mao_render::adapter_names`, a new `adapter` setting that every `Gpu` honours; empty is the system's choice);
the resolution keeps its predefined list (the monitor's modes), the custom size and the two boxes (Run In Window,
Borderless; neither is exclusive full screen); texture quality, compressed textures and the colour depth are gone
and the edges' smoothing, the wait for the monitor, the drawing scale and the view distance stand in their place;
the skin is the interface's theme and skin folder; the font is the original's bitmap font on or off and the
interface's size; the folders are this client's (settings, screenshots, chat logs); the two time-outs are kept in
`launcher.ini` (the login's progress bar and the dimension download use them); the report to Funcom is gone. The
settings window is drawn inside the launcher's (the page is put away while it is open), where the original opens a
window of its own.

The login now opens at the size and in the window mode of the settings, as the original's login is already the
game's window, and behind it is a hall (corrected the same day, see the paragraph above: this first try drew the
creation's corridor): the six meshes GUI.dll's `CharCreateModule_t` loads by
name (`InitialiseMessage` 0x1011bdaa, the table at 0x102724e4), seen from the first place of the game folder's
`CharCreateCamera.dat` with the original's 60 degrees. **The stretch the user showed** (the original at 5120 by
1440: the hall's upper part cut off): the original's angle is across the window, so a wide window sees less up and
down. Here the angle is kept as a 4:3 window has it and a wider window sees the same height and more to the sides
(`hall::half_height`; the renderer's camera took an optional view height for it). In a very wide window that shows
past the hall's walls to the space outside it (kept so: the user chose the full width over black bars at the sides). Seen: the launcher beside the user's picture of the original's, the
three settings pages, PLAY into the login, the hall at 16:9 and at 32:9.

**A shop's doorways: no door to be seen, no way through (2026-11-11, the user: "adjoining rooms are inaccessible
and not visible"; then "compare the mappings of the original for these shops").** Compared with the original's
data and code for the neutral shop (playfield 1186), as its captures hold no picture from inside one:
- Its dynel table has seven doors (type 0xc748, template 0x49604 "Door", no mesh of their own). Six stand in the
  doorways between its rooms and the seventh at the way out (`interior::Interior::doorway_at` over the table's
  places: a test with the game's data).
- A room is drawn, and may be entered, only through an open door (`n3RoomMonitor_t`,
  `IsDynelRoomTransitionAllowed`: unchanged here). So everything hangs on the doors opening.
- A door opens when a character touches it unless it is locked (`Door_t`'s FUN_1007fccc: shut, flags' bit 0x40 clear,
  vtable +0xfc at 0x10085635 → FUN_10080477 → FUN_10080319: a sound, the flag, the mesh's animation; nothing to the
  server). Here only the Whom-Pahs' doors opened so, the others on a use, and a shop's doors could be neither seen nor
  used. `approach_doors` is now every door of the table that is not locked (stat 259, its own or its template's), and
  a door the server announces by its flags.
- The door itself: the table's doors without a mesh were not drawn (outdoors such a door is a shuttle's or a
  Whom-Pah's, whose template mesh put a white door there). Among rooms the template's mesh is the door: the shop's
  `door_icc_standard1` fills the doorway's frame exactly (picture `shop-door.png`), and its animation takes the leaf
  away in a third of a second. `static_props` gives a rooms playfield's doors their template's mesh; the table's door
  prop follows its door's number when the server's announcement takes the slot (`World::door_props`), and gives way
  to an announced door that brings its own model.
Not done: the door's sound (stat 131, else 100), its own closing time (stat 335; five seconds here), the locked
door's question whether this character may pass (FUN_1007fad8: a key, an owned building, the team's mission) with its
feedback line. The doors' heading is as the table stores it (which side the keypad is on was not compared). Not
walked live: no test character stands near a shop.

**A shop's walls with holes (2026-11-11, the user after the walls came back: "shop walls are not complete").**
In the neutral shop (playfield 1186) the wall behind each row of terminals was a hole into the next room. Not the
faces turned away this time: the holes were there with every face drawn. Those walls' textures (the dark panels
with the lamps, 300536 and its like) carry an alpha channel, and the client cut every room texture out by its
alpha. The original does not: `VisualRoom_t::Init` (DisplaySystem) makes the room's one material and turns its
alpha off (`RMaterial_t::UseAlpha(0)`), and that material is not two-sided either (its faces turned away are left
out, as here). The rooms' material now has `ignore_alpha`. This was so before today as well; the faces drawn from
behind used to hide some of it. Seen from inside in pictures (`inside.py` in the scratch folder: a place at four
headings, `MAO_CULL=show`): the neutral, Omni and clan basic shops have their walls whole. `rooms::describe_room`
and the `roomcells` example say a room's cells, pieces and textures. Not seen live.

**Macros, scripts and more commands (2026-11-12, the user: "Work on /macro and any other missing / commands").**
What each does in the original is in `docs/GUI_DLL_PARITY.md`, section 3.
`/macro <name> <command>` makes a macro and puts it on the pointer: click a hotbar slot to set it down (the click
does not press the slot). The slot shows the macro's name in yellow; a press carries its line out as if typed;
its tip says the line. Macros are kept in `TextMacro.bin` beside the settings, in the original's format.
`/script <file> [words]`, or just `/<file>`, runs a file of chat lines from a `scripts` folder beside the
settings or from the original's own scripts folder: one line a frame, `%1` `%2` filled with the words, `#` lines
skipped, `/delay 500` waiting half a second. The files' names are in the list a `/` opens. **A line of a script
that is not a command is spoken**, as it would be typed (two of the files of the install looked at begin with
`<a`). `/filter add <pattern>` hides the lines of channels and the vicinity that the pattern matches (`list`,
`del <number>`, `enable`, `disable`, `clear`; it starts disabled, as the original's; the patterns are the old
regular expressions: `. [ ] * + ? ( ) | ^ $`, kept in `chatfilter.ini`). `/dvalue <name> [value]` shows or sets
one of the original's stored settings. `/start <http address>` opens it outside the game. `/char&viewdist <a>
<b>` sets both distances. `/played` says the time and date and asks the server for the time played; `/born` asks
when the character was made; `/team loot [all|leader|alpha]` asks for the team's loot order or, from its leader,
sets it; `/bank info` asks the server (these four are Gamecode's own text commands, each a character action:
`mao_net::n3::text_command`). **`/org <command>` is the organization's commands** (`info`, `ranks`, `contract`,
`invite`, `kick [name]`, `promote`, `demote`, `bank [add|remove <n>]`, `tax <n>`, `name`, `description`,
`objective`, `history`, `governingform`, the votes, `debt`, `city`; `orgcommands.rs`), and **`/o <text>` is its
chat**: before, `/org <text>` went to the chat. `/org info` is answered with a page in the info window (name,
leader, governing form, description, objective, history, your rank, the areas it controls) and `/org contract`
with a container window of the organization's contract, its advantages, to look at only
(`MAO_UI_PREVIEW=infopage MAO_INFO_URL=org` shows a made-up organization's page). **The
commands that set a text (`history`, `description`, `objective`, `name`, `governingform`) are sent as typed, as
the original sends them: with nothing after the word they empty that text.** `/items` asks the server for the account's items (it opened
the inventory before; `/open inventory` does that). `/tower`'s commands are not carried out (`/raid`'s are since 2026-11-18, see "`/raid` is the raid's commands").
Previews:
`MAO_UI_PREVIEW=macro` (a bar with two macros). Tried live: a macro made, set down by a click and pressed; a
script found and one not found; the filter's and `/dvalue`'s lines; `/played` and `/born` with the server's
answers. Not tried: the filter on a real channel line, a script with `/delay`, `/start`, `/team loot` in a team;
`/bank info` was sent and nothing was shown in answer.

**Away from the keyboard (2026-11-12, the user: "Do the afk and auto away").** `/afk` as the original's
(`afk.rs`; what was read is in `docs/GUI_DLL_PARITY.md`, section 3). `/afk <text>`: away with that message, said
to you and, as an emote, to those around. `/afk` alone: away with the default message and a box to change it
within 30 seconds (Enter or Ok takes it, Escape ends being away); alone while away: back. `/afk status` says which.
A tell that comes while away is shown with "(AFK reply sent)" and answered with the character's name, how long it
has been away and the message; that answer is a tell marked as such (its blob's first byte is 6), which the client
that gets it shows without a name in front and never answers. **Going away by itself:** three minutes without a key
or button press or a movement of the mouse over the window and the character is away with "Auto-away"; with that
setting on (`AutoAFK`, on in the game's own settings, `/setoption AutoAFK off` to change it) any input ends being
away, also one asked for with `/afk`. That is the original's rule, not a shortcut here. The texts are the
install's own (text.mdb), with this client's words where it has none. `MAO_AFK_IDLE=<seconds>` shortens the three
minutes for a trial and `MAO_AFK_QUIET=1` leaves the lines to the vicinity unsent; `MAO_UI_PREVIEW=chat
MAO_CHAT_AFK=<text, or empty for the box>` shows it in a picture. Tried live between the two test characters, the
box by its keys; the box was looked at only in the picture.

**The tradeskill guide (2026-11-12, the user: "I'd like a tradeskill helper too, a guide to all the creations a
user can craft in game").** A window of the client's own (`/crafting [text]`, `/craft`, window name `crafting`;
`craftguide.rs` over `mao_formats::crafting`; the whole account is `docs/TRADESKILL_GUIDE.md`). **The game's files
hold no table of the tradeskill processes**: no record kind for them, no "use on an item" action on the templates
(their action kinds are 0, 1, 3, 5, 6, 7, 8, 111), and the original client asks the server pair by pair
(`CharacterAction` 0xdc to 0xe4). So the guide is what Funcom wrote on the items of the player's own install, read
when the window first opens (two seconds, away from the frame): 16,306 entries, of which 416 are books and manuals
(the book on nano crystals, a construction manual for each of hundreds of weapons) shown as pages to read; the
other items a text names as links; the 2,002 steps a text states outright ("with this, that makes that"); the kin
of a thing made in several steps (the same name in brackets); the books that speak of a thing or its kind; and the
way back from a thing to its parts as far as the texts say. Two things come from the player's side: a name is
green where the item finder knows the player has it (which character, which place, on hover), and every answer the
server gives in the tradeskill window (result, quality range, each skill requirement) is written to
`tradeskills.txt` beside the settings and shown on the pages of the items involved. That last part is the only
exact one and grows only with use. The links and steps are read from prose: a thing named in passing is linked like
one that is needed, and a sentence built unusually is missed. The crafting texts are also a source of the help
window (`crafting`, on by default: 3,476 pages), so the small model answers "how do I make a nano crystal" from the
game's own book. Seen in preview pictures over the game's data (`MAO_UI_PREVIEW=crafting`, `MAO_CRAFT`,
`MAO_CRAFT_PAGE`, `MAO_CRAFT_SAMPLE=1`). Live (Brasscasings), `/crafting nano crystal` opened the window and the
guide was read in 2.3 s; the window was not looked at on the screen, and the writing down of the tradeskill
window's answers was not tried against the server.

**The help window's own small model (2026-11-12, the user: "We need a small llm, only needs to interact with our
data really").** The "Ask the guides" window can now write its answer with a model the client runs itself:
Qwen2.5 1.5B Instruct, a 1.1 GB file the player fetches once from the window ("Sources and the model", the tick
"Write an answer in words with the small model inside the client"), run by the `candle` library inside the client on
a few of the processor's threads. It sees the question and the pieces found for it and nothing else, and is told to
say "The sources do not say." when they do not hold the answer. Tried live in the running game: an answer in 6 s,
the frame rate from about 195 down to 120 to 150 meanwhile. `docs/HELP_ASSISTANT.md` ("The model inside the
client") has the choice of the model, the trial against the smaller one that was rejected, how it is asked, and what
it gets wrong. `mao-help model fetch` and `mao-help ask --local here <question>` do the same from the command line;
`MAO_HELP_MODEL=<folder>` runs the model's own test.

**The mission helper (2026-11-12, the user: "clone the mission-buddy to help users roll missions").** The old
outside helpers read the mission terminal's offers off the screen and told the player when a wanted reward or place
came up. Here it is part of the terminal's own window (`mission.rs`), since the client has the offers as data. Under
the Request Missions button every offer now stands in a line: its reward items with their quality, the playfield it
sends the player to (by name: the network thread fills it in) and its credits; a click selects the offer. "Looking
for" holds two lists, parts of reward items' names and parts of places' names, a part a line. An offer fits when
one of its rewards holds one of the item parts and its place one of the place parts (a list left empty asks
nothing; with both empty nothing fits): its line and its box are marked green, and the line's tip says what fitted.
With "Keep asking until one fits" ticked, Request Missions asks again by itself, 1.2 s after each answer that does
not fit, until an offer fits (it is selected, and the window says after how many requests), the limit is reached
(20 unless set, at most 200) or the terminal does not answer within 10 s; "Stop asking" ends it, and so does
closing the window. The lists, the limit and the tick are kept in `missionhelper.txt` beside the settings. Each
request costs as many credits as the character has levels (the original's own rule, above), which the window says
under the lists; a request the credits do not cover ends the asking with the error box. A terminal with nothing to
offer now says so (GUI.dll FUN_100d0e71's line, in this client's words). **Not tried live**: asking a terminal costs
credits, which the tests on the user's characters may not spend, so the asking-again has only its rule's tests
(`next_step`) and the window a picture (`MAO_UI_PREVIEW=mission MAO_CAPTURE=capture-221539.log`,
`MAO_MISSION_ITEMS` / `MAO_MISSION_PLACES` with `;` between the parts). Not there yet: a sound or a flash when an
offer fits, fitting by the kind of mission, and accepting by itself (left to the player on purpose).

**The item finder (2026-11-12, the user: "devise a means for players to find items in their bags or bank on each
character you might have to tell them which character it is on").** `/find <part of a name>` (or `/find`,
`/open find`) opens Find items, this client's own (`itemindex.rs`). The server tells a character's things only while
that character is played, and a container's contents only while it is open, so the client writes down what it sees:
a text file for each character played on this machine, in the settings folder's `items` folder
(`<Name>.<the dimension's port>.txt`), with a section for what is worn, the inventory, each bag of the inventory (as
it was when last opened) and the bank (as it was when a bank terminal was last used), each with when it was seen. The
network thread looks every three seconds whether the character's things changed and writes then. The search takes
every word typed, in any order and case, goes through all the characters' files, and lists each thing found with its
quality and count, the character, the place on it ("Bank", the bag's name) and how long ago that was seen; names
that begin with what was typed come first, the better quality before the lesser. A click on a row opens the item's
description. Under the rows the window says what it cannot see: the bags never looked into and a bank never seen,
by character. "Look into my bags" has the client use each bag of the character played that it has not seen into
yet, half a second apart and without opening their windows (`silent_bags`), so one press makes the bags known; a bag
the player then opens by hand gets its window as before. Bags are remembered by their place in the inventory and
their kind, so a bag moved to another place is taken for a new one until it is opened again. Not known: a bag inside
the bank (never opened there), and anything on a character not yet played with this client. Tried live with
Ballbearing: the file written with what is worn, the inventory and an opened bag, eleven more bags listed as not
looked into; then `scanbags@` (the button's script step) made all twelve known in one run. That the bags' windows
stay shut during it was not looked at on the screen. `MAO_UI_PREVIEW=find` (`MAO_FIND=<text>`) draws the window over
two made-up characters.

**The damage meter (2026-11-12, the user: "a parsing project that a player can use to determine damage output of
themselves and other players (should be a new window in the gui)").** `/damage` (or `/dps`, `/open damage`) opens
the Damage window, this client's own (`damage.rs`; the original has none, players read the chat log's combat lines
with other programs). It is not fed by text but by the fight's messages as the client reads them: a weapon's hit
(`AttackInfo`: critical and glancing by its hit type), a special attack's (`SpecialAttackInfo`, named by its skill), a
miss (`MissedAttackInfo`), a loss or gain of health with its source (`HealthDamage`: a nano program's damage by the
program's name, a heal), a damage or reflect shield's hit. The server tells these of everyone near, so other players,
their pets and the creatures are counted with the client's own character. The events fall into fights (a fight ends
after 12 s with nothing hit; 40 are kept); a row is a fighter: its damage, its damage a second over the fight's
length, its share, a bar against the largest. The window shows the latest fight, any kept one or all together;
damage done, damage taken or healing done; players only; a pet's figures with its owner's (the client's own pets:
the owner of another's pet is not known) or apart. A row's tip says hits, misses, the largest and the average and
the critical share; a click lists what the damage was done with. Copy puts the rows on the clipboard as lines of
text, Reset forgets the fights. Not counted: a loss of health told only as a new health value (no source), and a
fall. Tried live (Maotesttwo against Arete's cleaning robots): its own hits, the robots' on it, and another player's
fight near by were all counted. Nothing is kept between runs yet, and there is no log file. `MAO_UI_PREVIEW=damage`
(`MAO_DAMAGE_ROW=<n>`) draws the window with a sample fight; the script's `log` step prints the meter's figures.
This client's own tool windows are kept in `extras.rs`, which the interface opens and draws by name.

**A team's mission terminal, and the error box (2026-11-12, the user: "Team Missions should give an error dialog that
you're not in a team").** A mission terminal's stat 490 is the kind of mission it gives (`QuestOriginator_e`): by the
game's data (`terminals`, a new example in mao-formats) 1 and 2 the neutral terminals' (one's own missions, a team's),
3 and 4 Omni-Tek's, 5 and 6 the clans'; the even ones are a team's (`GameData::IsTeamOriginator`). Using a team's
terminal while in no team, the original says text.mdb's `QuestBooth_NeedTeam` (category 102) through its message
signal and does not use the terminal (Gamecode FUN_10086b3e). Here the same (`mission::use_refused`, the terminals'
kinds from `mission_terminals`), in an error box: `chrome::alert` queues a text from any thread and
`chrome::show_alerts` shows the boxes one after the other, titled text.mdb's `Error` with an OK button. That the
original's message signal (`GlobalSignals_c` +0x184) ends in such a box is taken from its other uses (the web view's
"has crashed and will now be restarted", the login's "Unknown error"), not read from the slot that takes it, which was
not found. Two more things came with it. The request named kind 5 whatever the terminal, the clan terminal's of the
capture: it now names the terminal's own kind (`MissionGenerateInfo_t`'s eighth number, `N3Msg_GenerateMissions`
0x10016276), so a neutral or Omni-Tek terminal is asked for its own missions. And asking costs as many credits as the
character has levels: with fewer, the Request Missions button says `NotEnoughCredits_NeedX` in an error box and asks
nothing (GUI.dll FUN_100d09cb; `mission::request_refused`). Tried live: Ballbearing, in no team, at Old Athens' clan
team terminal gets the box and the terminal is not used (`useobject@dac1;<number>`, a new script step for the things
of a playfield's table). Not tried: a request at a team terminal while in a team, and at an Omni-Tek or neutral one.
`MustHave2FreeSlots` (fewer than two free places of the inventory's thirty, on asking and on accepting; `mission::slots_refused`) followed the same day. Not ported then: "No
missions available, please try again later!" (FUN_100d0e71). A message box with no pointer on the window yet is put
in the screen's middle every frame (it stayed outside an offline picture). `MAO_ALERT=<text>` shows an error box in
a preview.

**A wrong password, and the wait after Play (2026-11-12, the user: "I put the wrong password and got a huge login box
not the traditional error window", "when we select a character and login for several seconds there's nothing being
displayed this is where I expect the loading screen the most").**
*The error.* The login's refusal was written into the login window in one unbroken line, the whole of the client's
own sentence about it, and the window grew as wide as the line. The original says a login's error with a page of the
game's site: `Client_t::ProcessMessage` (Interfaces.dll 0x10002a9e) reads one number of system message 0xd and hands
both to `LoginModule_c::SlotLoginReply` (0x10012179), which for 0xd, 0x10 and 0x21 calls `ShowError` (0x10011fd5):
the address `errorurl` (from `data/launcher/AnarchyLauncher.url`, whose `name=value` lines the module reads at its
making, the names in small letters), the message's number in decimal, a dash and the code when it is not 0, `.html`.
A login server that is not reached is error 1 with no code (`SlotLoginAccount` 0x100120f6). The page goes into
`LoginBrowserWindow_c` (0x10010ead): a window named "Browser" without a title, 780 by 410, links to the system's
browser; `Show(0)` (0x1001175a) centres it and the login window and moves the login down by half the browser's height
and 5, the browser up by half the login's and 5. Closing it empties the address and leaves the login alone
(`SlotBrowserQuitPressed`). Here the same: `session::Error::Refused` carries the code, `dimensions::error_page` makes
the address, `loginviews::LoginBrowser` is the window and the page is a WebView2 child over its inside, as the
launcher's page is (`launcher.rs` `place_page`; the keyboard stays with the login's fields). Tried live with the
free test account and a wrong password, once: the server answers 0xd with 106, the page is `13-106.html` and asks to
enter the name and the password again. Where no page can be shown (no WebView2, another system) the window says the
error in words, and a message in the login window is now wrapped to the window's width. This client's own: a login
that works puts the page away (the original keeps it until its window is closed, so it would stand over the login
again after Back). `MAO_LAUNCHER_DEMO=error` (`MAO_LAUNCHER_ERROR=13-106`) shows the window without a login. Not
ported: the pages of a character that cannot be entered (0x10, 0x21 after the list) and of a lost connection.
*The wait.* The game is a process of its own, started when Play is pressed; it logs in, reads the playfield and
makes its window, and only then is there anything on the screen: the character list's window went at once, and for
seven or eight seconds the desktop showed. The original has its loading screen up for that time
(`ServerLogin3DModule_t`). Now the list's window stays with the loading screen (`loading.rs`: the black, the picture
fading in over 4 s, the line of text) until the game's window has presented three frames, which the game says by
writing the file named in `MAO_WINDOW_UP`; the game's window opens where the list's was (`MAO_WINDOW_AT`, a plain
window only) and goes on with the same screen from where it had come (`MAO_LOADING_SINCE`, `loading::begin_since`),
so the fade in is one. The screen's going was begun with the first world update, in a window not yet shown whose
first frames take seconds, so it was half gone when it was first seen: it now begins once the window shows the world
(`viewer.rs` `LOAD_CLOSED`). Tried live through the login to a character twice: the list's window had the screen for
7.6 s, the game's window came up in its place with the world under the screen's fade. This client's own: the screen
goes up at the press of Play; the original shows its progress window ("Please wait...", `Show(4)`) until the
character's ticket has come and the loading screen from then. In the exclusive full-screen mode the two windows
change places with whatever the system does between two such windows: not tried.

**Hair through a helmet, a shop's wall gone, the run that stops (2026-11-11, the user's three notes after playing).**
*Hair through a helmet.* A character's listed meshes were all drawn, in the order of what the lists call a layer.
That number is a set of flags, and DisplaySystem shows by them (`CharacterMesh::AddAttractorMesh` keeps each
place's meshes with bit 1 first, FUN_10071e5d, and drops one already there, FUN_100734b0; FUN_10073517 attaches):
at a place with a bit-1 mesh that one alone, at a place without one every mesh. The captures agree: a helmet
(`opifex_male_helmet_cyberarmour`, a whole head made for the breed) is on the head with flags 2 beside the
character's own head (`head_opifexmale02`, hair and all) with 4, which the original attaches only while the
helmet's mesh is not loaded yet; glasses come with 0 beside the head; a weapon in a hand with 2, shoulder pads and
back pieces with 0. `mao_net::n3::shown_meshes` is that rule, used for the full update's list and the appearance
update's; a helmet stands in the head's place (`HEAD_REPLACED`) and the body's head is not drawn with it. Checked in
a picture (`MAO_WEAR=0:9d3a,0:4e5f` against `128:4e5f` on `player 2 2`): the hair's spikes through the helmet are
gone. Not seen live. The original's hiding modes (+0x74, `VisualCATMesh_t::SetShape`): Gamecode FUN_100590b8 sets 1 while
the character's `MonsterData` (stat 359) is set and 0 otherwise. In shape 1 FUN_10073517 hides every listed mesh but the
character's own head (place 0, flag bit 2). Hanging that head on a polymorphed or riding character's body was ported on
2026-11-19 (RND-142) and taken out on 2026-11-20: the user's polymorphed character showed a human face over the creature's,
which the original does not show. What in the original keeps that head off the body was not found; such bodies carry
nothing worn again.
*A shop's wall gone.* Since the faces turned away are no longer drawn (this morning), every interior's own walls and
ceilings were gone from inside it and the next room showed through; the floors stayed. The rooms' pieces are put from
the game's upright frame into the viewer's, which mirrors them, and given as parts in the viewer's own frame, so
their corners' order has to be turned round for that frame (`rooms::room_parts`); the floor, made here, was right
already. Checked in pictures from inside the clan's basic shop (playfield 1180, `MAO_CAMERA=208,202,7,1,1.57,0.05`
with `MAO_CULL=show`): no wall or ceiling is among the faces left out now. This was every interior (shops, flats,
mission buildings), not one shop. `MAO_CAMERA=print` says the fitted camera of a picture.
*The run that stops.* With a rifle held, the walk and the run are the weapon's own (`walk-2h`, `run-2h`,
FUN_1009cda2), and those files were looped whole: `run-2h` is four seconds of a start, the strides and a stop, its
cycle from 1166 to 1966 ms between its `loopstart` and `loopend` notes. The plain run was looped between its notes
already; the weapon's now is too (`Characters::file_loop_points`, as FUN_10010929's modes take the notes of whatever
animation is set). By the files' notes (`animnotes`, a new example); not watched live.

**Deleting a mail (2026-11-11, the user: "deleting mail doesn't work", a shop's mail with its item still in it).**
The original's delete (FUN_101096f1, which the inbox's Delete, FUN_101098a0, and the message window's, FUN_1010686c,
both come to after their "delete?" question) does not delete a mail that has an item (flag 8) not yet taken (flag
2): it says `Feedback_MailDeleteAttachments` instead. Else it sends the delete, takes the row out at once and closes
the message window showing that mail. This client sent the delete whatever the flags; the server deletes no such
mail and answers nothing, so nothing happened and nothing was said. Now `MailWindow::delete` does as the original:
the game's line in the chat window and the mail window's status line, or the delete with the row gone at once. The
message window's Delete asks the question too (it deleted without asking) and stays open on a No. By its test; a
delete was not tried live (deleting the user's mail is outside what the unattended tests may do).

**The item store's buttons, the daily rewards' claim, the pending rewards (2026-11-11, the user: "doesn't work yet",
"the buttons don't work for the item shop").** Three things, found by tracing the pages' requests
(`MAO_BROWSER_TRACE=<folder>`: each request said on the error output and what came back kept in the folder, the
page's errors and clicks said too; `MAO_BROWSER_EVAL=<script>` runs a script on the page ten seconds after it opens,
a test's way to press something without a place on the screen):
*What a site sends compressed.* The item store's lists (`/pools/`, `/items/`) come gzipped whether asked so or not.
The proxy does not pass `Content-Encoding` on and had no way to open them, so the page got the compressed bytes
(mangled besides, by the rewriting of the site's addresses in them), its script had no lists, and every button
threw. `browser::decoded` opens gzip and deflate before the addresses are rewritten.
*A POST with nothing in it.* The daily rewards' Claim is one (`/claim`), and it went out without a length. It is
sent with a length of 0 now. The page then answers the press and marks the day.
*The game's side of the claims.* The original asks the server to hand over what was claimed when the daily
rewards' window goes (`~BrowserWindow_c` 0x1010fcc1, mode 4: `N3Msg_RequestClaims`, `CharacterAction` 0x107), and
the menus' "Pending Rewards" (`checkrewards_window`, `BrowserModule_c::SlotCheckRewardsActivated` 0x1010a9e1) asks
it to look for what is waiting (`N3Msg_RequestClaimsDBCheck`, 0x108) and puts the value back. Neither was sent.
`n3::claims`, `net::request_claims`; the entry is in the control center's list of windows without being one. The
original holds either request back while its lock 0x8c runs (FUN_1006414f) and says so: not ported, the server is
asked each time.
Seen live (2026-11-11): the item store lists its items and its tabs and a price's purchase page answer (free
account); on a paid account the day's reward was claimed on the page (`/claim` answered with its hash, the
description loaded), and after the request an item came into the backpack; closing the window sent 0x107 and the
menu's value 0x108. Not seen: a purchase in the item store carried through, "Claim Items" there, customer service.
The free account's daily rewards page is only a notice that it is for members.

**A character written by its number (2026-11-11, the user: a bot's page showed `Mocham&#39;s Gift`).** The bot's
page now arrives (the paragraph below) and showed its apostrophes as `&#39;`. The game's markup reader takes its
characters through Utils.dll's `String::GetUCChar` (FUN_1000db51), which besides the five named entities reads a
character by its decimal number: `&#`, a digit, digits up to a `;` before the thirteenth byte, the number `atol`'s,
0 no character; no hexadecimal form. Ported as `mao_net::chat::numbered_character`, used by the markup
(`gui/text.rs`) and by the lines' plain text (`chat::plain`). By its tests; not yet seen on a bot's page.

**The wear window's squares fill the window (2026-11-11, the user: "the wear window squares should scale to fill the
whole window").** The original's wear cells keep their size (47 by 47, 11 and 9 apart) and its window is exactly as
wide as three of them. This client's window is as wide as its four tabs in its own fonts, or as its rollup page, and
the grid at its own size left empty room beside it (centred since the morning, which the user did not want either).
Now the grid is grown until its three columns reach from side to side: `GridMode::scale` (this client's own) grows
the cells, the room between them, the slot pictures and the picture under them alike, in whole pixels (a size a
hair over its room, as a fraction gives, brought the scroll bars out; the few pixels left over are halved to the
sides); never smaller than the original's size. `invwindows::filled_grid`, used by the wear window and the inspect
window; the labels keep their font. Seen live: the wear window's four tabs on two characters' layouts (one as
wide as its tabs, one wider). The inspect window was not photographed (the script's click on "Inspect Equipment"
only ran the inspection again).

**A bot's page of buffs did not arrive (2026-11-11, the user: "the response isn't coming back when talking to a bot").**
The bot's first tell showed and its second, the list, never did. The list is a link whose target is a whole page of
markup (`<a href="text://...">`), with tags, links and line breaks inside the quotes. `mao_net::chat::plain` ended a
tag at the first `>`, so the page's inside leaked into the line's plain text, line breaks with it; and the chat thread
drops a line whose text has control characters (its guard against a packet read wrongly). Now a tag ends at the first
`>` outside quotes, as `HTMLParser_c::_ParseTag` has it, and a line break or tab in the text is a space; a line still
dropped is said on the error output. Worked out from the code and a test with a made-up page: not tried against a bot
(a tell to someone else's bot is outside what the unattended tests may send).

**Tells: `%t`, the tell windows, the menu on a name (2026-11-11, asked for by the user after playing).**
The user typed `/tell %t help` and got "To %t: help" and, a moment later, "The chat connection ended". Four things
were missing or wrong, all read from GUI.dll first:
*The `%` arguments.* `ChatGUIModule_c::ExpandChatTextArgs` (0x100866c6), which every command that sends text runs it
through: `%t` the target's name, `%f` the name of the one fought, `%m` the player's own, `%%` a percent sign, any
other `%x` left alone; without a target the original's own marked-up words. Ported as `chat_commands::expand` and
used for `/tell` (name and text), `/reply`, `/o`, `/t`, `/group`, the speech commands and plain lines. The names
come from the target bars each frame (`ChatWindow::names`).
*The tell windows.* `/tell <name>` without a text opens the tell window for the character (FUN_100ba8d5 sends the
empty text on, FUN_10089940 calls `ChatGUIModule_c::OpenTellWindow`); so do the left button on a name and the
friends list's left button (FUN_100a7ca7; this client typed `/tell <name> ` there before). The window is asked of the
chat server first (`Out::TellWindow`): it opens once the name is known, as the original's does, and is then listed
among the recent characters (FUN_100a6487). It is a window titled with the character's name over the chat window,
with the tells from and to them (the ones from before it opened too: the original keeps each character's,
FUN_100a7394) and an input bar whose plain lines are tells to them; commands work there as anywhere. The chat windows
that show the tells go on showing them. A tell to the player's own character is refused (FUN_10089940), in this
client's words. `/tell` alone opens the window for the target: **this client's own** (the user asked for it; the
original does nothing without a name), and without a target it says how the command is used.
*The names as links.* The original writes a character's name in a chat line as a link (`user://<name>`, FUN_1009bc20
for the groups, FUN_100a681d for a tell, which is `[name]: text`; the echo of one's own is `ChatTellMsgToField`,
FUN_100a5f39). The lines are now written so (`chatwindow::name_link`): the look is the same but for the tells, which
were "<name> tells you: " and are `[<name>]: ` now. The chat view's `MouseDown` (FUN_1008fa92) and
`GroupChatView_c`'s handlers (FUN_100ac65f): the left button on a name opens the tell window (FUN_100ac2fa), with
Shift or Ctrl held it types `/tell <name> ` instead (FUN_100ac4c4: the qualifiers' low two bits, taken as those two
keys, which is a guess at which keys they are); the right button opens a menu of three (FUN_1008e5ea: `IgnoreUser`,
`OpenChat`, `SendTell`): ignore or stop ignoring, the tell window, and `/tell <name> ` typed into the bar of the
window clicked in. The right button anywhere else in a chat window is still the window's own menu.
*A name the chat server does not know.* Its answer carries an id of all ones; this client took that for an id and
sent the tell to it, and the chat server closed the connection: that is what happened to the user (`%t` was the
"name"). Now it says the game's `ChatCmdFeedback_UnknownUser` and drops what waited for the name (FUN_1008a2c1).
And a chat connection that ends is made again (the original retries too): after 5 seconds, 10 more after each try
that does not get as far as logging in, and no more after five such in a row; the timing is this client's.
*Found on the way.* `HTMLParser_c`'s tag reader (FUN_1015cb03) steps past each attribute's value; the port did not,
so a second attribute after a quoted one was lost (`style` then `href`: the names' links showed as text). A chat
window laid out every marked-up line every frame; with every line with a name marked up now, the height each took is
kept (by its text, width and font) and a line out of sight is no longer laid out.
Tried live with two of the user's own characters (Ballbearing and Brasscasings, next to each other): `/tell` alone
with the other selected opened the window; a line typed in it went out with `%t` filled in and came back answered
with `/r`; the right button on the name gave the menu, "Send Tell" typed into the window's bar, "Ignore" twice
ignored and stopped ignoring; `/tell Qqzzxxnope hi` and `/tell %x hi` got the unknown-user line and the connection
stayed. Not done: the tell window's own menu, the friends list's flashing row for a tell not yet seen and the tell
windows in its "Chat Windows" folder; the group's name in a line as a link (`chatgroup://`, FUN_100ac4f0). Not seen
live: the reconnect (no connection ended), the left button on a name by hand, Shift or Ctrl with it.

**The unattended pass of 2026-11-11 (the user away; their list: every pending item that can be done, and two notes).**
*The creatures' sizes (the user: "the bounding boxes for mobs seem to be off").* Every creature was a person to the
mouse and to its name tag: a cylinder 0.5 in radius and 1.9 tall to pick it by, and the tag 1.9 + 0.4 above its
feet. The original takes a mesh's height for the tag (`TargetingModule`'s FUN_10025603 asks
`N3Msg_GetMeshHeight`, which is `VisualMesh_t::GetMeshHeight`: twice the greatest radius of the bounding spheres of
the body's triangle meshes, randy31's `BVolume_t`, Ritter's sphere; 1.9 only without a mesh), and picks by the
mouse's line against the dynel's collision. Now each body's size is taken as it stands (`blobshadow::Size`, from
the posed points every network update): the sphere's diameter (`mao_formats::bounds::sphere`, the original's
method) for the tag, and how far the points go up and out for the picking cylinder (not thinner than 0.3, not
wider than it is tall or than 3). The sphere is of the posed body with what it holds, where the original's is of
each mesh at rest: nearly the same for a standing body; a long weapon may lift a player's tag a little.
*The remains (the user: "monsters remains should never be behind geometry").* A dead body stayed where this client
had last walked it, and the server's corpse was matched to it within four metres: live, bodies lay up to 3.2 m from
their corpse and 1.3 m above it, which can be inside a wall or under a floor. The body now lies where the server
puts the corpse (the corpse is what the original draws), and the dead are taken off the path they were walked on.
Seen live (`MAO_COMBAT_LOG=1`): the corpses' places and the bodies' before the move. Not done: a corpse that was
there before the character came (no body is known for it, so nothing is drawn); and some dead entities carry
places that are no numbers of this world (seen in the log: 6e-39 and the like), which is an older fault of how an
entity's place is first read and is not looked into yet. (Later the same pass: a place whose numbers are no
numbers of this world is no longer taken, and `MAO_COMBAT_LOG=1` names the message that carried it; none came in
the minute tried.)
*The Shadowlands' sky (the user: "sky dome in shadowlands is off").* Found off the game, with the account's
expansions given to the scripts as the live client gives them (`MAO_SKY_EXPANSION=<stat 389>` for the headless
picture: the Shadowlands' skies are in the scripts' `Expansion EP01` blocks, which is why the first pictures showed
nothing wrong). The sky there is two domes of `shadowlands_vortex.abiff`, one overhead and one turned over below,
a quarter to a half of the view distance out, and their state blobs turn the fog on (`FOGENABLE`): in the original
they fade into the area's fog, which is what the capture of 2026-10-31 shows in Inferno, an orange haze with the
vortex only overhead. This client drew them unfogged: an opaque blue sheet over the whole sky. A sky draw whose
blob turns the fog on now fades into the world's fog by its distance (`viewer.rs::sky_fog`, `sky_draws`: the
area's fog, the original's `VisualFog_t` range from the near plane to the far one drawn in by the density, else the
atmosphere's haze), its colour times what the fog leaves and the fog's share added over it; a blob's own fog
colour (`FOGCOLOR`, black on the vortex's second layer) is taken. Seen off the game only, with an area fog given by
hand: the horizon in the fog's colour, the vortex above. Not done: fog on the blended sky draws whose fog is not
black (the atmosphere's band); whether the original's fog there goes by the distance or by the depth before the
eye is not known (the distance is taken, as for the world). To be looked at by the user in the Shadowlands.
*The orbital strike, and what else a script's mesh carries.* `/fxscript EP03_Orbital_Attack` ran and showed
nothing: its mesh, `EP03_orbital_attack_trigger.abiff`, is an unseen box whose one connector is named
`eff_universal_71015`. The original's mesh objects make a "universal effect" for each such connector
(DisplaySystem `fxgmo.rs` `light_attractors`: the number and the connector's place go on the environment's
attractor stack; Gamecode's `_EffectHandler_t::RunFunction` 0x100d2423 takes them off, makes each with
`CreateEffect2` and gives it its object's `Position` and `Rotation` every frame until the object's class is
gone). Done here: a started mesh object's connectors are read (`net.rs::universal_effects`), each effect is made
with its sounds, goes with its object and ends with it (`Update::placed`, drawn by `placed_particles`,
`visual_ground_shells` and the shake, as the waterfalls' feet are). Seen live: the strike's rings, its column and
the white of its flash over the pad, gone eight seconds later (`handoff/duel-shots/orbital-strike.png`). The
same makes the effects of the other meshes that carry one (the ICC shuttles' 92001 and 12570). Not done: the
connector's own place and turn on its mesh (the object's place is taken), the object's rotation, and the parts of
an effect that need a body. The fill square (`e_FillSquare`), which the first note named, is not the strike's:
only `Tweak_ProgressiveLava` uses it; still not drawn.
*The frame limit's pacing.* Under a limit the window's thread slept until the frame was due, and the system wakes
a sleeper a millisecond or more late. It now sleeps until two milliseconds before and waits awake through the rest
(`settings::LIMIT_SPIN_MICROS`; behind other windows the sleep alone). At a limit of 60 the slowest hundredth of
the frames is 20.9 ms where it was 27; what is left is the frames that carry the characters' upload.
*A cast's time.* The casting stance was shown for any nano with an attack time. The original works the time out
(Gamecode FUN_1005141d): the nano's attack time less the caster's nano initiative (stat 149: half of it up to 1200,
a sixth of the rest and 600) less the aggression setting (stat 51), not under the nano's own least (stat 523), and
runs a cast of no time at once, without the stance. Ported for one's own casts (`net.rs::cast_seconds`); the
others', whose skill the client does not know, go by the nano's time as before. Not tried live with a character
of high initiative. Not followed: the nano's flag 0x80000 (no time whatever the stats).
*Looked at and left.* Turning the picked character with the mouse in the character list, noted as not done with
the hall: the original does not do it there (`CharacterViewer_c::SetRot` is called by the creation's module alone),
so it is no gap. Left for the user: when the white of a death begins and how long it takes; why the CYT skin's
wing was missing on their screen; the Browse dialog and the restart after a change of the game's folder, which
need a hand on the mouse; the hall behind the creation screens with its seven figures, which is a piece of work of
its own.

**`/fxscript`, the item grids' padding, the CYT skin's wing (2026-11-11, asked for by the user after playing).**
*`/fxscript <script> [parameters] #`.* The original's command (GUI.dll FUN_100b70be, registered by FUN_100bb3e0 for
every player, not only game masters): with a script's name at least, the whole typed line is sent as the effect
scripts' signal (`FXSSignals_t::SignalFXSScript(line, 0)`), which Gamecode's slot (0x100b2dfc) runs as it runs a
script's own `[SEND]`: `<word> <script> <parameters>`, the command itself standing for the word, the script read
from `cd_image/twk/<script>.txt` with the parameters put where the script says `!parameters`, and the first object
it makes started as an environment effect when it has an `FXID`. The engine and the slot were there
(`fxs::script_signal`); the command now reaches them (`chat_commands::Parsed::FxScript`, `net::fxs_command`,
`fxs::command`). A script is only run when something follows its name, which the scripts' own calls fill with a
lone `#` (a comment): a name typed alone is given that, this client's help. The chat window says what came of it:
running, with the effect's number, or nothing started, with the script reader's complaints (the original writes
those to its log). Tried live: `/fxscript EP03_Orbital_Attack` and `/fxscript Alien_Event_Flyby_01` run and start
their effects (the log's `MAO_FXS=1` lines: seven mesh objects, kind 4, for the flyby; kinds 61, 56 and 7 for the
attack), an unknown name starts nothing. What a script shows depends on which effect kinds are drawn: the
environment light (1), mesh objects (4), generic visuals (5), sun rays (42) and single clouds (56) are; the fill
square (61, the attack's flash) and the others are not yet, so the command is now the way to find and try them.
The scripts' names are the files of `cd_image/twk` without `.txt`: `Alien_Event_*`, `EP03_*`, `Unicorn_Event_*`,
the `Tweak_*` of the skies, the atmospheres (`Tweak_Shadowlands_Atmosphere_SubSea` and the like) and the liquids.
*The grids' padding.* The item grids (inventory, bags, bank) put their first column six pixels in from the left,
as the original does, and whatever the window's width holds beyond whole columns was left at the right: a cell's
worth of empty edge on one side. The columns now stand in the middle of the width (`GridMode::left`), and the wear
window's three columns in the middle of its page, whose four tabs are wider than they are. This client's own.
The wear window took two tries. The first set the grid in a row beside a spacer, which gave the page a row's height:
the grid shrank to a scroll bar (the user's picture, the same day; it had not been looked at after the change).
The page had been taking whatever room the window had, which is also why there was empty height under the grid.
Now the grid is given its own size, three columns of five cells (`multilist::grid_pixels`: the layout tree's
smallest and largest sizes are a scroll view's, next to nothing and without end), in the middle of a page as wide as
the four tabs unnarrowed (`chrome::tabs_width`). Seen: the tabs' names whole, the grid with the same edge left and
right and none to spare below.
*The CYT skin.* What lies behind the menu buttons at the bottom left in the AOU mod's CYT skin is the skin's left
wing (`GFX_GUI_CONTROLCENTER_WING_LEFT`, 440 by 220 there; in the Slim skins a strip 6000 wide and 25 tall, which
is the bar across the whole bottom of the screen in the mod's own screenshot), shown while the control center's
wings are on. Drawn here with the skin's defaults it is there; one fault was found on the way: CYT docks the NCU
and credits strip at the top of the screen, and the health and nano bars, which stand on the strip's top where it
lies over the wing, went to the top of the screen with it. They keep the wing's own floor now. Why the wing was
missing on the user's screen is not known (the wings folded away, or a choice of the AOU menu): asked.

**Found by the user playing, 2026-11-11: the camera shook while running; the login's window stayed behind the game's.**
*The shake.* Two causes, both older than the day and both worse the faster the frames come and the faster the
character runs (the user's is level 220, and the frame rate had doubled that day).
The first and larger: the character's place between two network updates is its place at the last one plus its speed
times the time since. That time was counted from when the window's thread read the update, and the speed from when
the network thread built it, but the place is the network thread's last step's, some milliseconds older than either
(the characters are posed in between, and the update waits for a frame to be read). So with every update, 24 times
a second, the character and the camera's look target jumped a few centimetres on or back. Now the update says when
the character was there (`Update::own_time`, the step's time) and the speed is taken over the steps' times. Traced
(`MAO_CAMERA_TRACE=1` prints every frame's eye, look target and feet), a test character running at 5 m/s: its
jitter along its path 0.2 mm where it was 9.9 mm (at the worst 1.1 mm where it was 73 mm), and the view's turning
changing from one frame to the next by 12 degrees a second at the worst hundredth where it was 75.
The second: the original moves the camera's look target a fifth of the way to the head's attractor every frame
(`LookTarget`), and its snapped camera a tenth, whatever the frame took. It drew a hundred frames a second at most;
at two hundred that smoothing all but goes and the look target follows the head's bob as the character runs. The
shares are now taken per sixtieth of a second (`camera::ORIGINAL_FPS`, `keep_for`), the rate the turn keys were
already scaled by: the same smoothing at any frame rate (a test holds it). `MAO_CAMERA_PER_FRAME=1` gives the old
way, to compare. Which frame rate the original's players mostly saw is a guess; sixty is this client's choice.
*The window.* When a character is entered with, the game starts in a process of its own and the login's process
waits for it. The login's window was let go of after its event loop had ended, and winit takes a window down by a
message to that loop: so it was never taken down, and since the login became a window of the game's size (the hall,
the same day) it stood frozen behind the game. It is now hidden and let go of inside the loop, the moment a
character is chosen. Tried without a hand (`MAO_LAUNCHER_ENTER=<name>` enters with a character as soon as the list
is there): while the game runs, the login's window is not visible.
*Not found:* the user also said the Shadowlands' sky dome was off. Nothing drawn off the game showed a difference
between the morning's build and the day's end's (the playfields 4310 and 4542, with and without compressed
textures: the same picture), and no test character is in the Shadowlands; what exactly is wrong there is still to
be asked.

**The geometry packed into a few large buffers (2026-11-11, `Renderer::upload`, `reupload`, `draw_runs`).**
What was left in Arete Landing was the handing over of its 1,800 draw calls, 3.1 ms a frame: each call set a
vertex buffer and an index buffer of the part's own before it drew. Now a playfield's geometry is packed as it is
uploaded: the vertices of part after part into one large buffer and their indices into another (a new pair begun at
64 MB), and a part is where its vertices start, where its indices start and how many there are; the indices stay 16
bits, counted from the part's first vertex. The characters' parts, which already shared a vertex buffer, share an
index buffer too, both written once with each upload (the index buffers kept from upload to upload by a number
made of their contents are gone again: one write of everything is cheaper than keeping count). Drawing sets a
buffer only where it changes, which in a frame is a handful of times, so a call is the draw alone, and a texture
where the next part wears another.
Measured, 2560x720: Arete's pad 166 frames a second (5.9 ms) for 110 (9.1 ms), its hand-over 1.1 ms for 3.1 ms,
and the slowest hundredth of its frames 10.5 ms for 24 ms; Aegean about 215 (4.7 ms) for 185 (5.4 ms). The guess
before the work was one millisecond in Arete; the two buffer commands cost more than the draw itself. Since the
morning: Arete 90 to 166, Aegean 78 to 215.
Seen right afterwards: Arete's pad with its characters, the shuttle room, Aegean, the login hall. A playfield
takes no longer to upload (the geometry's share went from 115 to 98 ms in Arete: two large buffers for thousands).

**Two processor costs of a busy place cut (2026-11-11, `nametag::update_sight`, `Renderer::reupload`).**
Arete Landing's pad ran at 90 frames a second where Aegean ran at 143, and the profile (`MAO_PROFILE=1`, with new
stages for the interface: `ui-sight`, `ui-layout`, `ui-tessellate`, `ui-hud`) showed why.
The name tags: a tag behind a wall is hidden, as the original's tag sprite in the world is, and that was found by a
line from the eye to every tag through the playfield's surfaces, every frame: 2.0 ms with Arete's sixty characters.
Now only the tags that would be drawn are looked at (on the screen and at least half a pixel tall), and each of
those every fourth frame, a quarter of them a frame, the others keeping what was found (by their place in the list
and their name: the list comes anew with every network update). 0.05 ms. A tag may show or hide three frames late.
The characters' upload: their meshes are posed and uploaded again with every network update, 24 times a second,
and each of the 512 parts had its vertices and its indices written into buffers of its own: a thousand writes of
some 3 microseconds each whatever their size, 4.3 ms a time, a hitch in every fourth frame. Now the vertices of all
the parts go into one buffer the parts share, written once, and so do their indices (at first a part's index buffer
was kept from the last upload when its indices were the same; the packing above made that one write too). 1.5 ms a
time.
Measured after, 2560x720: Arete's pad 110 frames a second (9.1 ms) for 90 (11.1 ms); Aegean 185 (5.4 ms) for 143
(7.0 ms). In Arete the first of the two alone moved nothing: the time went to waiting for the card instead, so that
view is held by the card as much as by the processor (at half the drawing scale it ran at 113 before the second
change).
What is left there, by the profile: handing the frame's 1,800 draw calls to the card 3.1 ms (the playfield's parts
there are mostly different meshes, so few go together as instances; packing the playfield's geometry into a few
large buffers would save the two buffer commands of each call), the characters' upload 1.7 ms a time, taking the
frame's picture from the card 1.2 ms.

**The video settings in the game's own options window (2026-11-11, `ui.rs::options_window`, `viewer.rs`).**
What the launcher's Visual page sets can now be set while playing, in the client's options window (the key for the
options, or "Client settings" on the original's option panel). Its Window part has the monitor a borderless or
full-screen window covers; a Quality part has the four presets, the edges' smoothing, the shadows (off, low, medium,
high), "Compressed textures", the drawing scale as the upscaler's named steps where a free slider was, the
sharpening (only below 100%, and now more to the right, in the launcher too, where it counted the other way), and
the two distances; a Frame rate part has the wait for the monitor's refresh, the limit and the background's. Each
is taken as it is changed: the shadow map is made anew at its size, the surface is set up again for the wait (between
two frames: done in the middle of one, wgpu refuses it, which the first live try showed), a monitor chosen moves the
window there, the frame limit holds from the next frame. Two wait for more: compressed textures hold for the
textures loaded from then on, so wholly after the next zone, and the graphics device is the launcher's to choose.
The settings are written when "Save settings" is pressed, as the window's others are.
Tried live by script (`open@options` opens the window; it is centred, so a row that comes or goes moves the rows
above it up and those below down by half its height): the Low preset drew the scene at 59% without smoothing at
once, the shadows went from off to high, the wait for the refresh went on and off again with the log's "present
mode" lines. Not tried: the monitor list with a second monitor, the limit's lists by hand.

**The textures block-compressed on the card (2026-11-11, `mao_render::block_levels`, `[window] texture_compression`).**
The fourth of the renderer's optimisations, for the video memory: the textures were kept as 32 bits a texel, and
Arete Landing with its new 4096-texel textures held 5.4 GB. Now a texture is encoded into the card's block formats
as it is uploaded, 8 bits a texel, its mip levels with it. The encoder is Intel's (`intel_tex_2`, built in: no
tool is needed on the machine). The formats: BC7 for a texture with no texel under full alpha; for one with alpha,
BC3 on the meshes and BC7 on the sky's and the effects' sheets and the liquids. That split was measured (the
`bcquality` example, forty of the game's large textures with alpha): BC3 keeps the colour to 40 dB and the alpha to
60 dB, BC7 at its quickest setting 45 dB and 54 dB, and BC3 takes a thirty-sixth of the time; with BC7 for every
texture the playfield's thousand textures took 2.9 s to put on the card, where the plain ones take 0.75 s. The
sheets are few and of soft gradients, where BC3's colour steps would show, so they take the fine way; a level of a
megapixel or more is encoded in strips on several cores, so a 4096 by 4096 sheet does not hold the window's thread.
A texture whose sides are not multiples of four, or a device without the formats, keeps the plain upload.
Measured (2560x720, the overlay's "held by the renderer"): Arete Landing 5,440 MB plain, 1,856 MB compressed;
Aegean 2,368 MB and 1,088 MB. The textures' share of the load: Arete 0.75 s plain, 1.2 s compressed (1,010
textures); Aegean 0.19 s and 0.34 s (679). The frame rate is the same. The picture: a test draws a textured square
both ways and holds the difference small; by eye, side by side in Aegean, bark, leaves and ground are not told
apart. A disk cache of the encoded textures, which the plan first called for, turned out not to be needed.
The setting is `texture_compression` (on unless set off) and a box on the launcher's Visual page, "Compressed
textures"; it is read when the game's window opens. What is left of the memory in Arete is the compressed textures
themselves (about 1 GB), the geometry (190 MB) and the render targets; the interface's own pictures (the planet
map among them) are not compressed.
Not looked at closely: the sky's domes and the effects' sprites in blocks against plain, at night and in the
Shadowlands' skies; the character textures as characters come into view (each is encoded on the window's thread, a
few milliseconds for a 512-texel one).

**The same part in many places is one draw call (2026-11-11, `mao_render`: `draw_runs`, `record_of`, `TextureBind`).**
The third of the renderer's optimisations, and the first that the frame rate shows. Measured first (`MAO_PROFILE=1`,
Aegean among the trees, 2560x720): of a frame of 11.1 ms, 3.9 ms went on handing the frame's commands to the card
(wgpu checks and translates them when the encoder is finished), more than on anything else; the card's own work on
the scene was 2 ms. The frame drew 1,565 of the playfield's 37,470 parts and some 340 parts of characters, each
with four commands (its bind group, two buffers, the draw); and the 1,565 were only 357 different pairs of geometry
and texture, on 210 textures (908 were plants: the same trees over and over).
What was changed: a part no longer has a uniform buffer and a bind group of its own. What the shader reads of a part
(`PartUniform`: its matrices, colour, flags, tint, blending) is kept beside it and written each frame, for the parts
drawn and in the order they are drawn, into one buffer the shader reads as an array by the draw's instance index
(`parts[instance]`, group 2; the shadow map moved to group 3). A texture has one bind group, shared by all the parts
that wear it (`Renderer::binds`), and a geometry a number. The opaque parts in view are sorted by the faces they
leave out, their texture and their geometry, and those that follow one another with all three alike go in one call
as its instances (`draw_runs`); a pipeline, a texture or a pair of buffers is set only where it changes. The blended
parts keep their order, far to near, one call each; the shadow map's parts are merged the same way. A character's
part is moved and marked for the motion buffer in its record (`record_of`), where its uniform buffer was written
again; `Scene::move_follow` and the tints only note the change.
Measured after, the same place: 1,609 parts in 449 calls; the hand-over 1.1 ms, `draw` as a whole 2.4 ms where it
was 5.9 ms; the frame 7.0 ms, about 143 frames a second, at 2560x720 and at 5120x1440 alike (before: 12.9 to 15 ms
in the window, 8.5 to 9.5 ms over the whole monitor). The performance overlay and the debug line now give the
calls ("1609 of 37834 parts in 449 draws").
Seen right afterwards: Aegean, Arete Landing and its shuttle room, the character walking (its parts follow it), the
login hall. What is different by design: the opaque parts are drawn in another order than they were uploaded in,
which only shows where two surfaces lie in exactly the same plane (whichever is drawn first wins, as before, but it
may be the other one now). Not checked: character highlights seen side by side with before, the room playfields'
light maps beyond the one room, the creation screens.
What still costs on the processor's side, by the same profile: each frame looks at all of the playfield's parts
(37,000 sphere tests and level-of-detail choices, about 1 ms), the interface 1.1 ms, the characters' meshes posed
and uploaded again 24 times a second (3.3 ms each time, 0.9 ms a frame on average).

**The faces turned away are not drawn (2026-11-11, `mao_render::Culling`, `faces_of`, `Material::two_sided`).**
The second of the renderer's optimisations, and as it turned out a matter of likeness and not of speed. The
original does not draw a face seen from its back: its device's cull mode is `D3DCULL_CCW` (randy31's defaults,
render state 22 set to 3), and a material draws both sides only when its own states set the mode to 1, none
(`RMaterial_t::IsTwoSided`; a character mesh's material by bit 1 of its flags, `cat.rs`). Counted over the game's
meshes (`cullcount` example): 2,812 of 37,516 materials are of two sides, in 1,905 of 11,228 meshes (leaves, fences,
glass), and none sets the other sense. This client drew both sides of everything. Now the material carries
`two_sided`, each part is given the faces it leaves out when it is uploaded (none, or one of the two senses by the
sign of its matrix's determinant: the meshes are drawn through the swap of the upright, a mirror, and the terrain
and the room shells are in the renderer's own frame), and the mesh pipelines come in the three kinds. The liquids,
the particles, the shells, the backdrops (the sky's domes) and the blob shadows draw both sides as before, as the
original's effects do; so does the shadow map, so the shadows are as they were. The client's own markers and the
stand-in floor are of two sides.
Checked with `MAO_CULL=show`, which draws both sides and paints magenta what would be left out: an open place with
terrain, trees, buildings and creatures (Aegean), the arrival pad of Arete Landing, the shuttle room there and the
login hall show no fault of the game's geometry. What does come out magenta is what the original leaves out too:
the rim of a far wall seen from behind, and the player's own body from inside when a wall pushes the camera into
it. Measured with `MAO_CULL=off` against it, four runs each way at 2560x720 and at 5120x1440 in the same place:
no difference in the frame rate or in the graphics card's time for the scene (2.0 ms and 4.0 ms either way). The
frame there is held by the processor's side (about 1,600 parts drawn of 37,800 looked at), not by the pixels.
Not checked: mission dungeons, city interiors, shops, vehicles, the Shadowlands. `MAO_CULL=off` brings the old
drawing back if a face is missing somewhere.

**The textures' mip bias under a drawing scale (2026-11-11, `mao_render::mip_bias`, `mesh.wgsl`).**
The first of the renderer's optimisations was to be mip maps, and they turned out to be there already: every
texture is uploaded with a full chain (`texture::mip_chain`, a box filter, made where the texture is decoded) and
read through a trilinear sampler with 16 times anisotropic filtering, since the commit that brought the sky and the
fog. The list that said otherwise was wrong (it counted the `mip_level_count: 1` of the render targets). What was
missing is the bias that goes with the drawing scale: a scene drawn at half the window's size picks each texture's
level for the small picture, a level blurrier than the window's pixels call for, and the upscaler cannot bring that
back. The meshes' texture is now read with a bias of the scale's logarithm to base 2 (0 at Native, -0.38, -0.58,
-0.76, -1 at Performance: the figures FSR 1 asks for), in the frame's globals (`detail.x`). Seen live at 50%, with
and without (`MAO_NO_MIP_BIAS=1` turns it off, to compare): a vent's slats and panel lines some way off are resolved
where they were smeared. The gain is small and only on textures seen smaller than they are; the game's textures are
mostly seen larger. Not changed: the liquids' and the particles' textures, the chain's filter (a box in the stored
values, as Direct3D's own made them for the original; an average in linear light would be a little brighter far
off), the cut-out textures' cover (a plant's leaves thin out with distance, in the original too).

**The launcher's Visual page, second pass (2026-11-11, `launchwindow.rs`, `settings.rs`, `mao_render::Shadows`).**
Asked what belongs on the page, the user chose to have it reworked first. New settings, each honoured by the game's
window: a **frame limit** (`fps_limit`: none, 60, 100, 120, 144, 240 a second; 100 is where the original stopped) and
a lower one **while the window is not in front** (`background_fps`, 30 unless changed): `viewer.rs` asks for a frame
only when one is due and waits until then (`settings::frame_limit`, `next_frame_due`), where it drew as fast as the
present mode allowed; the **sun's shadows** off, low, medium or high (`shadows`): the shadow map's side is 1024, 2048
or 4096 texels (`Renderer::set_shadows` makes the map anew; the shader reads the map's size, which was written into
it as 4096); the **monitor** a borderless or full-screen window covers (`monitor`: the window's own, or the system's
n-th; `apply_mode`). The page itself: four **presets** (Low, Medium, High, Ultra) set the edges' smoothing, the
shadows, the drawing scale and the two distances, and the one whose values the settings have is marked; the drawing
scale is a list of the upscaler's own steps (Native, Ultra quality 77%, Quality 67%, Balanced 59%, Performance 50%)
in the slider's place, and the sharpening shows only below 100%; the character distance is there (it was only in
the game's options); the size's fields are greyed in the borderless and full-screen modes, which take the monitor's
size; the smoothing's choices have the names players know (Off, MSAA 4x, TAA). Seen: the page in both window modes;
live with a limit of 60 and low shadows, the performance overlay read 60 frames a second at 16.6 ms with the wait
for the monitor off. The same settings came to the game's own options window later that day (see above). Not done: a steadier frame pacing (the
limit waits on the system's timer: the overlay's slowest hundredth was 27 ms).

**The hall behind the login and the character list, as the original has it (2026-11-11, `hall.rs`, `launcher.rs`).**
The hall drawn behind the login earlier the same day was the wrong view: the first place of `CharCreateCamera.dat`
is the character creation's corridor. The original's login has a world of its own, GUI.dll's `LoginWorld_c`
(0x100160c5): five of the creation's meshes (the hall with a profession's figure on each pedestal among them) and
one fixed camera, inside the hall, looking down it and twelve degrees downwards (constants in the DLL, now
`hall::LOGIN_CAMERA` and `LOGIN_ROTATION`). The character list's character stands five units in front of that camera
and 1.9 below it (`CharSelectWindow_c` 0x1000efb1 sets its `CharacterViewer_c` there), facing it, in its idle
stance. Both are done: the login and the character list share the hall, and the character picked in the list stands
in it (the hall scene's overlay) where it was shown alone on a dark blue before. Checked against the original's own
pictures, the first frames of the capture of 2026-10-30: drawn with the original's rule for the angle
(`MAO_HALL_ORIGINAL=1`) the pedestals, the figures and the character fall where they do there. The wide-window
choice stands: the view keeps the height a 4:3 window has, so at 32:9 the figures and the character are whole and
smaller than in the original's picture, which cuts them at the knees and the head. (Asked with both pictures, the user chose the whole figures for the character list too.) Tools: `MAO_HALL_CAMERA=x,y,z,
qx,qy,qz,qw,fov` looks from another place, the `meshboxes` example prints where each part of a named mesh lies.
(Turning the character with the mouse is not in the original, and the picked figure's animation is done since 2026-11-18: see "The character list's figure".) Not done: the hall behind the creation screens (they have their own cameras
in `CharCreateCamera.dat`, read by `hall::places` and not used yet) with their seven breed figures, the figures'
own animations.

**The interface size's slider (2026-11-11, `ui.rs::size_from_slider`).** The user found the interface at 200% and the
options' slider giving only its two ends. The size was taken while the slider was dragged, so with every step the
slider itself grew or shrank under the pointer and the value ran to one end; a size of 200% kept that way is what
looked like the default (the default is 100%). The size is now taken when the slider is let go (a typed value at
once). The launcher's slider for the same setting shows per cent in steps of five like the options' one. Not seen
live: a test script could not be made to open the client's own options panel twice the same way; the rule itself
has a test. The script's `options` step presses the options key.

**The game's folder is a setting (2026-11-11, the user's wish: "since we require the original game install").** The
settings' GUI page has a Game row at the head of its folders: the folder's path, Browse (the system's folder
picker, the `rfd` crate), Open, and under it what a check of the folder says (the game's version when the folder has
`cd_image/data/db/ResourceDatabase.dat` and `version.id`, else what is missing). It is kept as `dir` under `[game]`
in `mao.ini`. The launcher now starts without a folder on its command line (`mao-client` alone, or `mao-client
launch`): it takes the settings' folder, else the first of the usual places that has the game (`C:\Funcom\Anarchy
Online`, the program folders' `Funcom\Anarchy Online`, Steam's), which it then keeps; a folder named on the command
line is used for that run and kept only when none was. With no game anywhere the launcher opens on that settings
page with PLAY switched off. Choosing another folder and pressing OK starts the launcher again, since its pictures,
its page, its version and the interface's skin all came from the old one. The other commands still take the folder
as their first argument. Also here: the page is opened half a second after the window's first frame (after the
third, once in about a dozen starts the window's own drawing did not show under it).
Since 2026-11-21 (a tester: "Didn't see a place to point to the original client folder from the launcher. Seems to
have found automatically") the folder is a group of its own, "Anarchy Online folder", first on the GUI page, with a
line saying what it is, that it is found by itself in the usual places and that Browse picks another; it was the
first row of the Folders group at the foot of the page, out of sight under the Skin, Launcher and Font groups.

Not done: the figures on the hall's pedestals (the original stands characters there, `CCCharacter_t`), the hall
behind the character list (the picked character is still shown alone), patching (a version that differs is only
told), the licence dialog, the error pages, which background the original picks.

**Sitting, the recharger, the return from death and the casting stance (2026-11-11; `net.rs`, `whiteout.rs`,
`viewer.rs`).** Four things the user reported from playing the test character, each seen live afterwards.

- *A seated character stays seated.* Any movement key stood it up (the client's own rule). The user's word for the
  original: only the sit key does, and the turn keys spin it where it sits. That agrees with Gamecode's table of
  movement transitions (FUN_1006cb58): the sit mode (8) is left only by `LeaveSit` (0x25), which `N3Msg_SitToggle`
  asks for; the turns (cases 9 to 0xe) are taken in any mode; and FUN_1006f9eb gives the sit mode's vehicle the
  floor of the walking speeds. Now forward, back, the sidesteps and the jump do nothing while seated
  (`seated_keys`), the turn keys turn the character and tell the server as when standing, and another character's
  turn messages no longer stand it up in the client's picture. What a jump does seated in the original was not
  read. Live: seated through all the keys, only turn messages sent, up again on the sit key.
- *The health and nano recharger.* It is an item used from the inventory by a right click, and the server wants the
  character seated and something selected (itself will do: F1), else it answers that a target must be selected.
  The client's part was all there; the kit failed for the user because the first turn or step stood the character
  up. Live after the fix: sat, F1, the kit used, the heal's lines.
- *Coming back from death.* The camera stayed where the character died and found its own way to the arrival pad.
  The server's move is a teleport and the playfield's message again, which in the original is a playfield entered
  and so a new camera (`SetMainDynel` → `CreateCamera`); the viewer makes one now whenever the server sets the
  character down elsewhere in its playfield (`Update::new_camera`). And the world fades to white while the
  character lies dead (`whiteout.rs`), as the user describes the original ("the screen fades to white then you
  teleport to spawn"). **A guess in its times**: the original's code for the white was looked for and not found
  (GUI.dll's `DeadMessage` and `TeleportStartedMessage` have none; Gamecode's `VisualFader_t`s are the weather's
  and the playfield's black one), so the white rises over the minute the server leaves the body lying and clears
  in a second and a half after the arrival; only the world goes white, the windows stay. Live: nearly white 55 s
  after the death, the pad with the camera behind the character after it.
- *The casting stance only for a nano that takes time.* Every cast the server tells played the casting animation,
  so a recharger's use and the auras' repeats did (the user named the rechargers and the keepers' auras). The
  original stands a character in the stance only while its list of casts under way has one (FUN_1007c8af), and a
  cast whose time is nothing runs at once and never waits there (FUN_1005141d). The client now plays the animation
  only for a nano with an attack time (stat 294; the recharger's regeneration nano has 0, an ordinary program 100).
  Not ported: the caster's skill taking a short time down to nothing. Live: the kit used seated, no stance.

The test script's keys reach the character now (`viewer.rs`): `forward/1.5`, `turn_left/0.5`, `strafe_right/1`,
`jump/0.3` hold the movement keys that long, and `sit_stand`, `select_self`, `attack`, `use` and the four target
keys press once. Until now only the camera's keys did anything from a script.

**What a new character's first steps found (2026-11-10, `net.rs`, `gui/controls.rs`).** Three fixes from playing a
fresh character through its first minutes (the list is in `LIVE_TEST_LIST.md`). An item move is still shown at once,
but the server's answer is followed now: it takes a move with `ContainerAddItem` naming the old place (`(0x68,
place)`) and the new, and refuses one with `CharacterAction` 0x92 naming the old place, on which the item goes back
(`World::moves_asked`). A zoning ends following and walking: the followed target's number could stand for something
else, or nothing, in the new playfield. And a link's words are stretched over the spaces between them, so that a
click anywhere in an NPC's answer chooses it (each word was a clickable piece of its own). The test steps grew:
`useitem@<part of a name>` and `wear@<part of a name>;<place>` find the item by its name, since the places shift
as the server hands the starter items out; `MAO_AUTOPLAY` has `who` and `what` (the characters and the usable
things about, with their distances), `target=<part of a name>` and `npcclose`.

**Wearing only what may be worn, and an NPC left waiting (2026-11-10, `net.rs`).** Before a move to a wear place is
sent, the client now asks what the original asks itself (Gamecode `N3Msg_IsItemPossibleToWear` 0x10026c57): the item
must be a weapon, armour, implant or spirit by its class (stat 76), its placement mask (stat 298, FUN_10080659) must
have the place, its "can" flags must have "wear" (FUN_10081d5f), all without a word when not; and its requirements
to wear it (action 6) must be met, else the game's "unable to wear" text (`World::wear_refusal`). Not ported from
that function: locked equipment, the implants' own rule, the social armour's and the belt's deck places; the
server's refusal still takes a move back. The other change is this client's own: an NPC that was being talked to
when a client went without closing the chat answers no new opening, so an NPC that stays silent for two seconds
after being asked is told the chat is closed and asked once more.

**Scripted clicks, and what the first scripted live run found (2026-11-10, `ui.rs::Ui::script`, `viewer.rs`).**
`MAO_CAMERA_SCRIPT` steps with an `@` act on the interface as the mouse and keys would, so the windows can be tried
live without anyone at the keyboard: `open@<window>`, `close@<window>` (`close@` alone: all), `click@<x>;<y>`,
`rclick@...`, `dclick@...`, `move@...` (the place in the screenshot's pixels), `type@<chat line>` then `key@Enter`,
`text@<letters>`, `key@<Enter|Escape|Tab|Up|Down|Backspace>`, `drag@<x>;<y>;<x>;<y>` (press, move, let go) and `usebag@<n>` (uses the inventory's n-th bag). A press and its release each get a frame. Example:
`14:open@chat,15:rclick@3600;1150,16.2:click@3700;1388,18:screenshot`. The run found four things, all fixed: the
Action window's pictures were loaded only on its key press (blank tiles when opened by `/open`, the control center
or kept open); the control center's NCU figure did not sum the running nanos as the NCU window does; the original's
chat config was misread for a window that takes new channels by itself (FUN_1009e379 sets every group to the
window's autosubscribe flag and the listed ones to the opposite, so such a window lists what it does not show: the
main tab had been showing only the fight); and an announced channel became a group only with its first line. What
passed and what was not reached is in `LIVE_TEST_LIST.md`.

**Stats window as the original's (2026-11-10, `stats.rs`).** Read from `StatView_c` (FUN_1008057d): the name and the
level on a line, a white word and its value in 0xbbbbff; the bars, each the skin's `GFX_GUI_HOR_BAR_EMPTY` filled with
a coloured one to the value's share and labelled "value / max" (health red, nano blue; experience yellow, alien
experience green and the duel, solo and team scores purple when turned on); then nine rows, the stat's name from
text.mdb 2003 with a colon at the left and the value at the right. The five optional bars are `StatViewConfig`'s
flags, all off in the player's own files; the right button over the window opens a menu that ticks them (kept in
`tabs.ini`). The experience bar counts from the level's start (FUN_1008038f), shadowknowledge from level 200. The
window had these rows already; the bars were the client's plain ones and the colours its theme's. The picture names
of the original's numeric gfx ids come from AFCM.dll's table (scratch tool `gfxid.py`: 0xda blue, 0xdb empty, 0xdc
green, 0xdd red, 0xde yellow, 0xdf purple).

**A character's info page as the original's (2026-11-10, `charinfo.rs`).** `/inspect` showed a page of the client's
own (name, level, profession, health). It is now laid out as GUI.dll's FUN_1003482c: the headline with the first name,
the name in quotes and the last name, then Breed, Gender, the title level, Profession, Level (with the shadow level
beside it from 201), invaders killed and killed by invaders, the defender rank, the alignment (the side's name from
text.mdb 2005), the organization and the rank in it, the duel wins and losses and the solo and team kills, and for
another character the "Inspect Equipment" link; the labels are text.mdb 506's by name. The names, title level and
records come from the info packet, read by hand now (`n3::character_info`: the schema's reading of this packet is
misaligned, its conditions on the type are not carried out); breed, gender and side from the character's entity,
the organization's name from its org packet. The layout of an organization member's packet (types 65, 67, 71) is the
schema's and has not been in a capture; the types with towers stop after the rank. Not shown: the profession title's
and the defender rank's names, the side experience bonus, the city, the PvP ranks, and the own character's tokens,
veteran points and specializations. Not tried live.

The markup reader now decodes the character entities Utils.dll's reader knows (`&nbsp;`, `&quot;`, `&gt;`, `&lt;`,
`&amp;`): the pages the original builds use them (`&nbsp;` between the level and the shadow level, in the mission's
time left).

**Skill costs, caps, colours and bars (2026-11-10, `skillcost.rs`, `skillwindow.rs`).** The Skills window lacked
what a point costs and how far a skill goes, because the tables were thought not to be in the data. They are records of
the resource database that Gamecode reads at start: a breed's ability costs (`0xf4312`) and ability starts and caps
(`0xf425b`), a profession's cost level for 77 skills (`0xf430e`) and the levels its seven titles start at (`0xf4308`),
and for each of five cost classes the cap at each title level and its growth a level (`0xf4311`). `skillcost::Tables`
reads them and carries out Gamecode's three functions: a point costs the cost level times the value raised from, over
ten (an ability: the breed's number times the value); a skill's cap is the least of the title level's cap, the cap of
the title before plus the growth for each level into this one, and twice the abilities' weighted mean, with more past
level 200; an ability grows three a level to the breed's cap. In the window (as `SkillWindow`'s rows and details): a
skill's name is in its cost level's colour (five steps from green to blue), a page's row ends in a small bar of the
trained value against the cap, the details give the cost of the next point, the cost of the points added and the cap,
and the IP at the top are those left after the points added. The increase button stops at the cap and at the IP left
(the original's `Skill_LackIP` text); with Shift or Ctrl it raises as far as both allow. Points added to an ability
count at once towards the caps of the skills it feeds, as the original's. Seen live on the level 27 test character:
two points of Body Development from 103 cost 247 and 249, the next 252, the cap 107. `MAO_SKILL=<group>,<stat>,<points>`
opens the window on a group's page with a skill selected (previews and live).

**The rest of the Skills window (2026-11-10).** *Suggested IP distribution* (FUN_100fb8de, FUN_100fb667): the game
ships its priorities in `cd_image/data/ipdist.xml`, for each profession and stat a priority over a level range (the
file covers levels 1 to 20; `ipdist.rs` reads it as `RecommendIPUse` does, the names matched against text.mdb's). The
skills of priority 1 get a point each in turn until each is at its cap, then those of 2, then 3; a skill of priority 0
gets none, and it all ends when the IP do not pay for the next point. It runs twice over, as the original calls it
(abilities raised in the first run lift the caps of their skills), and the info area then lists what was raised and by
how much, in two columns. *Resets*: "Reset this skill" (a reset point, stat 199, shown beside it; free for a disabled
skill; none for an ability) and "Reset all skills (n)" each ask first with the original's text, and Yes sends
`N3Msg_ResetSkill` (`n3::reset_skill`: a character action 0x9a with the skill's stat, 0 for all). That message is
built from the decompile: no capture has one and it has **not been sent live**, a reset not being something to try on
a character. "Reset all" counts stats 348 and 691 and is dead when the server has told of neither. *Buttons*: the
increase and decrease act at the press and, held for half a second, every 50 ms after (the original's two timers);
let go with Shift or Ctrl, the decrease takes every added point back. *Value*: white when it is what training and the
abilities give, green when more, red when less (FUN_100fe550). Seen live without pressing anything (the test
character's buffed skills green, 11 reset points); the suggestion on a made-up level 12 character in the preview
(`MAO_SKILL_SUGGEST=1` with `MAO_SKILL`; `MAO_TRACE_SKILLS=1` prints what it chose).

## 4l. Action window (`K`) and the map (`O`)

**Actions.** The catalogue is what the running original client reports (`docs/GAME_MECHANICS.md`): 3 weapon attacks, 12 character actions, 35 perks, 63 social ids with 9 dances, each tile with the original's `action_*` icon (perks get a `perk_*` family by name, a guess). Click a tile or drag it onto a hotbar slot (the hotbar file now stores `special` and `social` entries). Doing one sends Stand, Sneak, Search, Reload and Start Combat; the rest (the weapon attacks, Brawl, Dimach, perks, emotes) say their network message is not captured yet. Preview: `MAO_UI_PREVIEW=actions`.

**Map.** Diablo-style fog of war: the network thread classifies the cells around the character as it walks (floor, or wall from the collision data and steep ground) and sends only what changed; walls are bright outlines on a faint floor. A corner map (210 px, 150 units across, turning with the camera) is always shown; `O` opens the translucent overlay (drag to pan, wheel to zoom, Space re-centres). Dots: players blue, friendly NPCs green, monsters red, target yellow, Whom-Pah booths cyan rings, the character a gold arrow. Preview: `MAO_UI_PREVIEW=map` (`MAO_MAP_OVERLAY=1` for the overlay). Not saved between sessions yet.

## Next
- Decode the key-binding providers (verify the assumed defaults).
- Decode the zone-line records so every booth's destination is known up front.
- Chat window, target and hotbars on the same `egui` layer.
- The AOU skin archive.

**Stats are computed.** The Stats window shows the original's values: the stored stats the server sends plus the modifiers of what is worn (`derived.rs`, from `docs/STAT_ENGINE.md`). Worn places are the weapon/utility slots, clothes and implants; each item's effects are read from its template and interpolated between its low and high template by quality; effects with a level requirement (`item::modifiers_at`: operator 1 = below, 2 = at least) are kept only when the character's level qualifies. Armour classes, attack rating and the bonuses to max health and nano therefore appear (preview: `MAO_UI_PREVIEW=stats MAO_CAPTURE=<log> MAO_LEVEL=<level>`). The perks the world-entry message lists (`FullCharacter.Perks`) are included too: each perk id's record (type `0xf4264`, instance = id) names an item template whose effects are summed (`derived::perk_modifiers`). Not included yet: research ranks and the perk lines the server has not been seen to send (see `docs/STAT_ENGINE.md`), active nano buffs, and the attribute trickle-down into skills.

**Vitals.** A small movable panel (top left, `vitals.rs`) is always on screen once the character's stats are known: health (red) and nano energy (blue), then the experience bar (gold: XP between the last level's experience, stat `LastXP`, and `NextXP`) and the alien experience bar (purple: `AlienXP` against the server's `AlienNextXP`, with the alien level underneath). Max health and max nano are the computed values of the Stats window. Current nano, alien experience and `NextXP` appear only when the server sends them (they show a dash otherwise); the alien bar counts from zero because the previous alien level's threshold is not known to this client. Preview: `MAO_UI_PREVIEW=stats`.

Live check (Brasscasings, level 25, `MAO_STATS=1 mao-client net <game> enter <name> --live` prints the shown stats): the experience stats arrive (XP 241166, LastXP 228150, NextXP 249650: 60% of the level) as do current health and nano; the alien stats arrive as 0 / next 1500 with alien level 0 (no alien level label). Current health 1085 and nano 365 first exceeded the computed maxima (734, 360) because the belt and its NCU units were left out; now the maxima equal the server's current values (see `docs/STAT_ENGINE.md`). A bar whose current value exceeds its computed maximum still counts the current value as the maximum.

**Skills.** `J` opens the Skills window (`skills.rs`): the six attributes and every skill in the original's groups (Body & Defense, Melee and Ranged Weapons, Special Attacks, Initiative, Nano Skills, Trade & Repair, Medical, Misc), each with the value the original shows: stored + bonuses of what is worn, the perks and the active nanos + the attribute trickle-down (`derived::shown`, checked against the live original: Conjecture's strength 1175, body development 1870, nano resist 2129 ...). A "Find" box narrows the list by words; the groups collapse. A skill with no value yet shows a dash. Preview: `MAO_UI_PREVIEW=skills MAO_COMPARE=docs/live_compare_conjecture.txt`. The vitals' experience bar now shows a dash instead of an overflowing figure for a character past the next-level figure (level cap).

Live check of the Skills window (Brasscasings, level 25; `MAO_STATS=1 ... enter <name> --live` prints every group's rows): all 9 groups fill, no dash anywhere (the 69 trickle skills get a value even when the server sent none). Attributes 145 / 134 / 131 / 102 / 110 / 93, Body Development 147, Run Speed 220, Assault Rifle 209, Burst 195, Sensory Improvement 203, the untrained melee and trade skills 29-39. There is no original-client value for this character to compare with (the bridge reads Conjecture only), so the check is that the numbers are present and plausible for a level 25; the formula itself is verified on Conjecture.

**Skills hover.** Pointing at a row of the Skills window lists what its value is made of (`derived::breakdown`): Trained (the stored value, 5 for an untrained skill), then Equipment, Perks, Nano buffs and Attributes (the trickle-down), each only when it is not zero, and the Total. Live (Brasscasings): Strength 145 = 81 trained + 52 equipment + 12 nano buffs; Martial Arts 35 = 5 + 30 from the attributes; Sensory Improvement 203 = 99 + 57 + 20 + 27. The same split feeds the preview (`MAO_HOVER=x,y` puts the pointer there so the tooltip shows in the picture).

**Hover on the Stats window and the vitals.** The rows of the Stats window (attack rating, the armour classes), its health and nano bars, and the same two bars of the vitals panel show the same breakdown as the Skills rows when the pointer rests on them (the bars break down the maximum: Trained + Equipment + Perks + Nano buffs). A stat nothing trains (the armour classes) has no "Trained" line, only its bonuses and the total. Live (Brasscasings): max health 1085 = trained 626 plus equipment and nano bonuses, max nano 365 likewise.

**Live check of the computed stats (new client, four characters of the account, `MAO_STATS=1 ... enter <name> --live`).** The computed maximum health equals the server's current health for all four (Brasscasings 1085, Arctangent 1432, Beetlez 247, Bzzzzbzzzz 170: each was at full health), and the maximum nano energy equals the current nano for the three that were full (365, 784, 190; Bzzzzbzzzz was at 75-78 of 142). Finding it exposed a bug: the hand-written item reader (`inventory::parse`) stopped at the first entry whose flags it did not know, so Arctangent (and its gear window!) had **no items at all**. The world-entry handler now takes the item list from the schema reader (`InventorySlots`, 54 entries for Arctangent) whenever it finds more than the hand reader.

**Gear window of Arctangent (level 40), after the item-list fix.** `MAO_UI_PREVIEW=gear MAO_ENTRIES=docs/live_entries_arctangent.txt` (a file of `slot low high quality` lines, what `MAO_STATS` prints as `worn`) renders the inventory of a live character: the weapon tab holds the jetpack (place 1), the light bar (3), the Nervejolter (6), the belt platform (7), the Fear-forged Blade (8), three "Dark Dreams" NCU units and the NCU memory unit, and 25 more items fill the backpack; the cloth tab has the Exarch robe, the clan shoulder pads and sleeves, rings and the boots, the implant tab 13 Shiny implants. All 54 items resolve to names and icons (none shows as "Item #...").

The other wear tabs of Arctangent (`MAO_GEAR_TAB=Cloth|Impl|Social` picks the tab for a preview): **Cloth** holds 14 items in the right cells (neck: clan merits, back: the robe, shoulders, vest on the chest, sleeves and gloves, rings on the finger cells, trousers, boots; head empty); **Impl** has the 13 Shiny implants in the order of the original's cells (eyes, head, ears, right arm, chest, left arm, right wrist, waist, left wrist, right hand, legs, left hand, feet); **Social** has the head item (place 50), the back item (51) and a sleeve (57) in their cells, the rest empty.

**Weapon tab against the original's placements.** The placement numbers of the original's wear window (the `EquipSlot` list the AOSharp project reverse-engineered from the client) agree with our table for every cell except two: **Hud 2 is place 15 (0x0F)**, not unknown (a Hud 1 = 1, Hud 3 = 2 pair with it; both of the level-220 characters wear a HUD item there: a Nano Controller Unit, a Dreadloch booster), and the cell between the right and left hand is the **Belt** (place 7), not "Deck". Both are fixed: the weapon tab of the level-220 character now shows all fifteen cells (a character that wears a Hud 2 item no longer loses it from the window), and the stat model counts places 1-15. The cloth, implant and social cells (17-31, 33-45, 49-63) matched the list item for item.

**Item tooltips.** Pointing at an item of the inventory window shows: its name, the wear place (for the equipment cells) and quality level, the stack size, the description, and **"When worn"**: what the item adds to the stats at its quality and the character's level (`derived::effects`, the same reader as the Stats window; the first ten, green for a bonus and red for a penalty, then "... and N more"), with the stat names of the Skills window and a few hand-named ones (max health and nano, the armour classes, the damage modifiers). The Dreadloch Endurance Booster in the Hud 2 place reads +250 max health, +20 strength, +50 additional melee damage; the Strong Armor Sleeves +230 melee, +250 projectile, +230 energy armour and so on. Preview: `MAO_UI_PREVIEW=gear MAO_ENTRIES=<file> MAO_HOVER=x,y` (add `MAO_GEAR_TAB=Cloth` for another tab).

**Item requirements in the tooltips.** After the description the tooltip lists what the item needs, per action ("To wear" for armour, clothes and implants, "To wield" for weapons, the belt and NCU units, "To use"), each condition green when the character meets it (by its shown stats, so a skill counts with its bonuses) and red when not; the Kur'Ush Mallet (QL 300) reads 1h Blunt 2251, Fast Attack 1126, Brawl 1351, Dimach 451, Title Level 6 for Conjecture (all green) and the same in red for the level-86 character. The criteria come from the item template (`item::actions`: after the marker `22, 36, 2018` an action number and a counted list of `(stat, value, operator)` in postfix order, AND = 4, OR = 3; operators 0 equal, 1 less, 2 greater, 22 bit flag, 93 has-perk), at the item's quality (the values of the low and high template are interpolated like the effects). **The comparisons are strict**: the template stores "level > 214" for a requirement of level 215, so the tooltip shows value + 1 (and the level-gated effects of `item::modifiers_at` follow the same rule: "above 200" starts at level 201). The title level (stat 37, not sent by the server) is worked out from the level (`derived::title_level`: 1 below 15, 2 below 50, 3 below 100, 4 below 150, 5 below 190, 6 below 205, else 7; the original reports 3 for level 86 and 7 for level 220). Preview: `MAO_COMPARE=<stat file>` gives the character's stats for the colours.

The operator names are the original's: the AOSharp `UseCriteriaOperator` list, and the spell target kinds of the effect tails are `SpellModifierTarget` (1 self, 2 user, 3 target), which explains why nano buffs use tails 1 to 3.

**Live check of the requirement lines** (`MAO_STATS=1 ... --live` prints one line per worn item and action: the number of conditions, the unmet ones and the ones that cannot be judged; `stats-check` does the same for the original's characters). Every condition of the worn items could be judged (none "unjudged": the stats they name are all in the shown stats, the title level included). Results: **the level-86 character (original) meets all 62 conditions of what it wears**; the new client's characters wear items they do not meet (Arctangent, level 40: the Nervejolter needs 1h Edged 301, the belt platform Computer Literacy 181, implants Treatment 242-361; Brasscasings: the Dark Dreams NCU units need Computer Literacy 145 against 124, the Carbonum chest Stamina 165, Agility 135), and the two level-220 original characters have 56 and 39 of about 165 unmet (mostly Treatment and the attributes of their implants and symbiants), which means these characters were dressed while buffed (or by other means) and carry on wearing what they could not equip now; the game does not unequip them. So an unmet line (red) in a tooltip is a fact about the character's present stats, not an error of the item list.

**NCU window** (`N`, `ncu.rs`). The nano programs running on the character, from the character's own `SimpleCharFullUpdate` (`Message::active_nano_times`: each entry's nano id and the time it has left in milliseconds, which the window counts down from the moment the message arrived; the nano's whole duration is its template stat 8): icon (stat 79), name, a bar and the time left; pointing at one shows its description and what it changes (the same effect reader as the Stats window: Mongo's Kraken: +20000 max health, +54 strength and stamina). The bar on top is the NCU memory in use against the maximum (stat 181, which the client computes: 721 for Conjecture). **The memory in use is not sent by the server** (stat 180): the original works it out as the sum of the running nanos' quality stat (template stat 54), which came to exactly 283 for Conjecture's nine nanos and 52 for the second level-220 character (the free nanos carry 999 there and take none). Live: Brasscasings runs four nanos (the composite expertise and attribute boosts, about 10 minutes left each), Arctangent one. Preview: `MAO_UI_PREVIEW=ncu MAO_NANOS=id:ms,... MAO_COMPARE=<stat file>`.

## Machines in the world (shop booths)

The vending machines the server announces (`VendingMachineFullUpdate`, identity type `0xc75b`) are drawn as real models: the message's `Stats` carry `Mesh` (stat 12, a mesh instance of type `0xf6951`; the capture's shop stalls are `shop_clan_critters`, `shop_clan_misc` and `tir_shop`) and its `Rotation` quaternion says which way the booth faces, placed like the other entities (`overlay_model`, one copy of each model's parts and an instance per machine; cleared on zoning). They are not in the playfield's static objects (no statel stands at their positions), so without this the instances showed empty floor. Terminals (`c73d`, `c74e`..), chests (`c749`) and the mail terminal carry no mesh stat in the captures, so they are not drawn yet (their models, if any, are in the playfield statels or come from an item template this client does not look up). `net <game> props-preview <capture.log> <playfield> <out.png>` (with `MAO_INSTANCE=<instance playfield id>` for instanced playfields) draws the machines of a capture; `MAO_NEAR=x,y,z,r` lists the statels near a point when a playfield is loaded.

**Clicking in the world.** A left click (a press that barely moves the mouse, so camera drags don't count) on a machine, usable object or character selects it: the nearest one within 48 px of the cursor wins (the network thread sends the ids and positions of what can be picked with each update, the viewer projects them). A second click on the same one within half a second uses it (a booth opens its shop window, as `E` does). Ctrl+Tab and `E` still work. Not tried live: the click handling was only built and compiled, the use message itself is the same one `E` sends.

## Rooms: the shell of shop instances and dungeons

An instanced interior (a shop) and every dungeon (289 of the 630 playfields) is made of **rooms** (`mao_formats::rooms`, from `RDBPlayfield_t::ReadBlob` and `n3Room_t`'s reader in `N3.dll`; the layout is in the module docs). The playfield definition object (`0xf4241`) lists them: a rectangle of cells of a shared `GNDA` tile-map (`0xf4249`, its instance is in the definition; playfield 1180's is `0x474`, 150x150 cells of 2 units), a centre (`y` is the floor's height), a turn in quarter turns, links to the neighbouring rooms and a packed light map. The tile-map's `DCGA` says which cells exist, `DHGA` is the height of each in steps of 0.2 and `XTHA` its tiles. The shell is a surface that follows the cells' heights (each corner of a cell takes the average of the cells around it that are within a slope, 6 steps = 1.2 units, so the ramps of 5 steps a cell are smooth and a character that climbs only 0.5 at a time can walk them; a bigger difference or a missing neighbour is a vertical wall); verified against the shop booths the server places (a platform of height 30 is 6 units up: the booths on it stand at floor + 6, and the ones in the sunken part at floor + 4).

The statels of an instanced interior are **per room**: the file's zones are the rooms in order, and a zone's positions are relative to its room's centre (x, z; the height is the world's), turned with the room (`place_statel`). Playfield 1180's lamps, pillars and tables sit in the halls this way. `mao-client <game> rooms-check` lists the room playfields and whether their shells build (289 rooms-based, 341 outdoor). `net <game> props-preview <capture> <playfield> <png>` draws one (with `MAO_INSTANCE=<instance id>` for the booths of an instance; `1180` with `2152918` is a shop).

**Textures.** The `XTHA` chunk starts with a palette: a `u16` count (40 for playfield 1180's map) and that many words, each the instance of a dungeon texture (type `0xf6959`, with lower qualities in `0xf6967` and `0xf6968`; entry 0 is a placeholder). The image holds three palette indices per cell (red, green, blue). The shell uses the red one for a cell's top and the green one for the walls around it (a guess that matches the look of the halls: the first three entries are floors, plaster and brick; blue is not used yet, it looks like a decal layer). One part per texture.

**Doors and rooms.** A room's links are its doorways (`rooms::Door`: the room it opens into, or a way out of the playfield; its cell and the side of the cell; `door_place` puts it 0.999 into the cell from that side, `GetDoorPosRot`). Rooms stand side by side with their doorways facing: a test over every room playfield (289, 3,668 doorways) finds the door back from the other room within reach for all but eight two-cell-wide ones, which also settles the quarter-turn sense for every room (it was checked on one shop's booths before). The doors the server places are tied to the doorways they stand in (`interior.rs`, after `Door_t::LinkDoorToRooms`); the character may cross from a room to another only through an open door, or through a room between whose doors are all open (`PlayfieldAnarchy_t::IsDynelRoomTransitionAllowed`), and a step that would cross otherwise is undone. Only the room the character is in and the rooms behind open doors are drawn (`n3RoomMonitor_t`: a room shows as it is entered or as a door to it opens, and goes two seconds after the door closes; the room left lingers the same two seconds here). A generated building's copied rooms get their doors linked afresh by where they stand (`RoomPlayfield::connect_doors`, the generator's FUN_100c92b3: facing doors within a tenth of a unit along the floor and three tenths in height). The room a point is in follows `IsPosInside`: over one of the room's cells, between half a unit under the cell's lowest corner and half a unit over its roof (the cell type's height from the tile-map's `THSA` table, scaled by the cell's code). The camera attractors a room record carries (`rooms::Attractor`: a point, its rotation, a guide point and radius; a radius under 1 disables one) are the camera's vantage points: every tenth of a second the viewer takes the room's enabled attractor of least cost that sees the character (`PointCameraAttractor_t`'s cost: the squared distance, a thousand per unit the character is beyond the guide's radius, a hundred when it looks the way the present point does, 10000 when within half a unit or under a tenth of a unit high) and the camera vehicle arrives at it, looking at the character; none, and it steers as usual. Not ported: the original's two-step door search over the collider for the camera's "visible" test and the manual next/previous camera view keys. The texture layers are per tile (six in a shop's map), so every laid tile is textured; the old `CHGA` tile-map branch is not ported and no room playfield uses one (the doors test checks every tile-map is `GNDA`). The water surfaces and the light maps are done (below).

## Doors and built-in machines (the dynel table)

A playfield's object `0xf425a` (`mao_formats::dynels`) lists what is built into it: doors (`0xc748`), terminals (`0xc73d`: the bank terminals), chests and vending machines, each with its position, rotation (`w x y z`) and the item template whose mesh stat (12) is the model; the record layout is in the module docs. Playfield 1180's table has 7 doors (template `0xa24f` "Door", mesh `door_tir_standard1`), 4 bank terminals and 56 vending machine places (the server announces the ones in use, so those are not drawn from the table). `static_props` adds all the others to the world's props, drawn like the announced machines; they are the same in every instance of a playfield and have world positions (a room's doors stand in the gaps between rooms). **Using a door.** Each `DoorStatusUpdate` (`c748:<id>`) announces a door: outdoors its id is `0xc0 | number << 16 | playfield`, in a shop instance the seven ids run on from the lowest (`0x10c1a5a5` to `0x10c1a5ab`, in the table's order, so the door's number is its place in the run: a guess that fits the capture, where the doors that closed are the ones on the walked route). The announced door becomes a usable object at its table position: Ctrl+Tab targets it, `E` or a double click sends the ordinary use message `(0xc748, id)` and the door's model is left out for 5 seconds (it "opens"). **Not confirmed**: the use message of a door was never captured, nor what the server answers. `DoorStatusUpdate`'s payload is `(int, int, byte, short)` on the wire and `Gamecode.dll` builds it from the door: a byte that is the open flag (`Door_t` calls `DoorOpened` or `DoorClosed` on the room monitor and plays the matching sound) and a stat, but every one in the captures reads the same (a 2 in the first int, the rest zero, which is the closed state), so the open and closed states from the server were not decoded. **Decoded since** (the `DoorStatusUpdateIIR_t`'s reader FUN_100a0330 and apply FUN_100a015a, checked against live bytes from courtyard 3132, `00 00000002 00 00 00000000 00 000003f1`): after a byte, the version (2), the locked flag, the **open flag** (open calls `DoorOpened`, closed `DoorClosed`), stat 195, a flag byte and an empty list. The door now follows the server: open stays open until a closed word; a closed word does not cut short the few seconds our own use opens it for (in case the server never answers a use). Not seen yet: an update with the open flag set (every door seen was closed). Doors are solid where the zone surfaces make them so; zoning by a door needs the character within 0.5 m of its place.

## Friends list (`L`)

The chat server keeps the friends list. At login it sends each friend as a name (`20`) followed by a **buddy packet** (`40`: the id, a word that is 1 when the friend is online, and a string, the type the client gave it: one byte, 1 for a friend, empty in the captured ones). `mao_net::chat` decodes it (`Packet::Buddy`) and builds the two client packets from the public chat protocol: `40` adds (the id and the string `"\x01"`) and `41` removes (the id). Neither client packet was in a capture, so adding and removing are **not tested live** (the decoding of the server's packet is, against the captured ones). The window (`friends.rs`, key `L`, also `/befriend <name>`) shows who is online first with a green dot, a **Tell** button (it starts `/tell name ` in the chat input), **x** to take a friend off and a field to add one by name; a name whose id is not known is looked up first (packet `21`), as a tell is. A friend's login or logout arrives as another buddy packet and updates the dot. Preview: `MAO_UI_PREVIEW=friends MAO_FRIENDS="name:1,name:0" mao-client <game> map 540 --image out.png`.

**The window as the original's (2026-11-10, `friends.rs`).** Read from `FriendListView_c` (FUN_100aa315) and its
controller. A character on the list is of a kind: a friend online, a friend offline, or one of the **recent messages**,
which the original makes of anyone a tell goes to or comes from who is no friend (FUN_100a6487): it adds them as a
*temporary* buddy, packet `40` with the kind byte 0 where a friend's is 1, and the server tells of them with the same
byte (FUN_100a6581: a kind of exactly one zero byte is a recent message). The captures show both: the original client
sending `40 <id> 00` after tells, and one character's list coming back as 32 of kind 1 and 15 of kind 0. The server
keeps the temporary ones between sessions (one character has 188). So `mao_net::chat` now keeps the kind as sent
(`buddy_is_temporary`), `buddy_add_packet(id, temporary)` builds either, the client adds a temporary buddy on a tell in
or out as the original does, and the window puts those under "Recent Messages". Seen live: the test character's four
names are all recent messages (the server's kind 0), where the window before showed them as friends. The chat line "X
is now online" is for friends only now (the client's own notice).

The left button on a row starts a tell in the chat line at the first press (the original opens that character's tell
window, which this client has not). The right button opens the original's menu (FUN_100a99cb) drawn as its
`PopupMenu_c`: "Befriend" (a recent message only: a buddy add of kind 1), "Invite" (chat packet `50`, to the player's
private chat group, whose lines the chat window shows), "Delete" and "Ignore" (each asking
first with the original's question, `chrome::question`) or "Unignore", and "Mail" while the mail window is up. The
"Chat Windows" folder lists this client's one chat window; a press shows or hides it. Which folders are open is kept
(`FriendsWindowConfig`'s four flags: a line of `tabs.ini`). Preview: `MAO_UI_PREVIEW=friends MAO_FRIENDS="a:1,b:0,c:r"`
(`r` a recent message) and `MAO_FRIENDS_MENU=1` for the menu and a question. `MAO_TRACE_CHAT=1` prints each buddy
packet's kind.

**Message boxes and menus (2026-11-10, `chrome.rs`).** `chrome::dialog` is the original's `DialogBox_c`: it comes up at
the pointer, its text (markup, so the names in yellow) 15 from the sides and wrapped about twice as wide as tall, the
buttons in a row at the right, or the one button in the middle; `chrome::question` is the yes/no one with text.mdb's
`MsgBox_Yes` / `MsgBox_No`. Every yes/no and OK box of the client uses it now (removing a friend, ignoring, deleting a
mail or a mission, the trade's second confirmation, a team invitation, turning the tips off, the perk window's boxes);
their names are `chrome::DIALOGS`, none keeps a place and Escape closes each with `esc_dialogs`. `chrome::menu` shows a
right-click menu as the original's `PopupMenu_c` (`gui::popup`): the friends and missions windows use it. The built-in
look now reads the game's `TextColors.xml` too, so named colours in the game's own texts (`yellow`, `CCInfoHeader`)
are drawn in it: before, only the themes had them.

`mao-client ldb-text <game> <category> <name or key>...` prints the game's texts by the names (or the numbers) the
original asks for them with.

The inventory's Shift+right-click menu (Info, Use, Delete: the client's own entries), the NCU window's cancel entry
and the knowledge board's two (the original's `Look@Item` and `DeleteReference` texts) are drawn as the original's
popup too, and deleting an item asks in a message box (`delete-item`) where the menu's entry had to be clicked twice.

## Nano programs window (`Ctrl+N`)

The programs the character has are the world-entry message's `UploadedNanoIds` (40 for the captured character); `nanoprograms.rs` looks each up in the nano table (type `0xfde85`, the same records the NCU window reads) for its name, icon, description, NCU cost (the quality stat) and stat changes (the tooltip, as in the NCU window), sorted by name with a find field. **Casting**: the cast message is `5e477770` action `0x13` (`n3::cast_nano`: four zero bytes, the action, four zero bytes, the target identity `(0xc350, id)` or zeros for none, the nano identity `(0xcf1b, id)` and two zero bytes), matched byte for byte against two captured casts (one at a character, one at oneself). A click casts the program at the current target (not a usable object) or at oneself; a program can also be dragged onto a hotbar (`hotbar::Action::Nano`, saved as `nano <id> <cooldown> <icon> <name>`) and fired from there. Not tried live: the server's answer (the nano effect `4d450114` that fills the NCU window, or a refusal such as range or NCU memory) is not shown, only the line "Casting <name>". Preview: `MAO_UI_PREVIEW=nanos MAO_PROGRAMS=<ids> mao-client <game> map 540 --image out.png`.

## Perks window (`P`)

The "open perk window" action now has a window (`perks.rs`). The perks the character owns are the world-entry message's `Perks` (the same ids the stat engine reads effects from); each id leads through its record (type `0xf4264`, the item template at byte 12) to a template with the perk's name and description. A perk's ranks are consecutive ids with the same name, so the list shows one row per perk with the rank reached (Bio Shielding ids 170-179 are rank 10) and, on hover, the description and what the top rank changes in the stats. The perk templates carry no icon, so the rows show an empty frame. Searchable. Preview: `MAO_UI_PREVIEW=perks MAO_PERKS=<ids> mao-client <game> map 540 --image out.png` (the 143 ids of `docs/owned_perks.json` give 15 perks). What is not here: perks not yet owned (the original lists the whole tree with the research points to spend), and using a perk (that is `UsePerk`, `5e477770` action `0xb3` with the perk's hash: a five-letter string in the captured casts, not yet tied to the perk ids).

## Moving and wearing items (drag in the inventory window)

Dragging an item onto another place of the inventory window moves it: onto a wear cell to wear or wield it, onto a backpack place to take it off or rearrange. The message is `ClientMoveItemToInventory` (`5469373f`, `n3::move_item`): a zero byte, the source identity `(0x68, place)` and the destination place, checked byte for byte against two captured moves (place `0x59` to `0x14`, a cloth slot, and `0x53` to `1`, the first hud slot). The client shows the change at once (an item dropped on an occupied place swaps with it) because the server's answer to a move was not identified in the captures (the `InventoryUpdate`s that follow are bag contents); **a refused move (a wrong slot, an unmet requirement) is therefore not taken back on screen** until the inventory is read again at the next login. The worn items' stat bonuses are recomputed after the move. Dragging to a hotbar still works (the drag also carries the hotbar item). Not tried live.

## Loot window (corpses and bags)

Using a corpse (`0xc76a`, announced like the other usable objects) or a bag makes the server send an `InventoryUpdate` (`4e536976`): the container's items (placement, item ids, quality, count), its identity (`InventoryIdentity`, 51050 = `0xc76a` for the captured corpse) and a **handle** (0x70 in the capture). The window (`loot.rs`) is GUI.dll's `InventoryView_c` for a container that is not one's own (FUN_100ccefe): an icon grid three to a row (its `item_position_map` places items `place % 3, place / 3`, 21 places), titled with the container's name (a corpse's "Remains of <name>"; else Corpse, Bag or Container), each icon taken by a double click or its menu's **Take**, and **Take all**; an empty corpse or chest closes a second later (its `EventTimer_c`, 1 000 000 µs). The server despawns a dead body just before it announces the corpse (`CorpseFullUpdate`, `4f474e05`, at the body's place), so the client keeps the recent deaths' places and links the corpse to the body: using the body (E, double click) uses the corpse, as clicking a body does in the original. Checked live with Sadlee (a Leet's and a Viviparous Lizard's corpses, one item each). Taking sends `ClientMoveItemToInventory` (`n3::take_from_container`): the source `(0x6b, handle << 16 | place)` and the destination `0x6f` (any free place), checked byte for byte against the captured loot (handle `0x70`, places 1, 0, 3, 2). The server confirms each with a `ContainerAddItem` (`47537a24`: source `(0x6b, handle << 16 | place)`, target the character, slot `0x6f`); that is when the item leaves the window and is added to the first free backpack place of the inventory window (a guess: the server's own choice of place was not found in the messages). The corpse's `Despawn` closes the window. The window only shows a container the character opened with `E` or a double click; closing it does not send anything (the original's close message was not identified). Preview from a capture: `MAO_UI_PREVIEW=loot MAO_CAPTURE=<log> MAO_LOOT=<n> mao-client <game> map 540 --image out.png` (the first container of `capture-225436.log` is a corpse with an eye implant).

**Putting items in (bags and other containers, not corpses).** Dragging an item of the inventory window onto an open container's window puts it in: `ClientContainerAddItem` (`n3::container_add_item`: the container's identity and the source `(0x68, place)`), the message of the captured bag fills. The server confirms with a `ContainerAddItem` whose source is `(0x68, place)` and whose target is the container (slot 0, any), and only then does the item leave the inventory window and appear in the container's list (at the next place after the others: a guess, the server's own place was not found). Drops on a corpse are ignored. The drag is shared between the windows through `gear::DRAGGED` (cleared when the mouse button is up). Not tried live.

**One's own backpacks in windows of their own (2026-11-10, `loot.rs`, `net.rs`).** Until now every container the
character opened took over the one shared container window. The original gives a bag of one's own inventory its own
window (`InventoryView_c` FUN_100ccefe, the `N3Msg_IsItemMine` branch), as many at once as bags are opened. Now:
- An `InventoryUpdate` for a container of type `0xc749` whose identity is that of a bag in the inventory opens (or
  refreshes) that bag's window; corpses, chests and the bank still share the other window. The captures confirm that
  a bag item in the inventory carries the container's identity.
- A backpack's window is named `Container_<type>x<instance>` as the original's, so each keeps its own place
  (`layout.ini`); it shows the bag's 21 places three to a row, is titled with the bag's name, does not close when
  empty and has no "Take all".
- Items move as before: a double click, a right click or a drag onto the inventory takes one out
  (`take_from_container` with that bag's handle); an inventory item dropped on the window goes in
  (`container_add_item` with the bag's identity). The server's `ContainerAddItem` moves the item between the lists,
  by the bag's identity going in and by its handle coming out. Several bags can be open and each is updated by itself.
- The right button beside the items opens the backpack's menu (FUN_100cbace) with "Rename": the "Enter backpack
  name." box (FUN_100cb852) with OK, "Reset to default" and Cancel. The names are kept in `backpacks.ini` beside
  `mao.ini` (the original keeps each in the bag's own file under `Containers/`).
- Closing a window only hides it; using the bag again shows it (the server gives it a new handle). A bag that leaves
  the inventory loses its window.
Tested on built messages (an item into a bag and out again, the shared window untouched) and seen in a preview
(`MAO_UI_PREVIEW=loot MAO_CAPTURE=<log> MAO_LOOT=<n> MAO_LOOT_OWN=1`); **not tried live**. Not done: the menu's List
Mode and Auto Arrange for containers, items placed by hand in a bag, moving an item straight from one bag to
another, closing by Escape (`esc_backpacks`), and telling the server a bag was closed (the original's message was
not identified).

## Game text and the fight in the chat window

Three server messages now put lines in the chat window: `ChatText` (`5f4b442a`, the game's own lines such as "Tarasque is no longer immortal."; two zero bytes, the length, the text) goes to the System tab; `FormatFeedback` (`206b4b73`, "You successfully perform a Quick Bash attack.", "You successfully perform Nano Feast.") to a new **Combat** tab (the message is a compact markup of tags and strings, a string being the letter `s`, its length with the closing zero counted, the characters and a zero; only lines that are one plain string are rebuilt, the ones made of names and numbers are not); and `HealthDamage` (`3710256c`: the target's health, an amount, a stat and the target) becomes "You take 49 points of energy damage." / "<name> takes N points of cold damage." for a negative amount (the stat is the armour class that met it, 0 meaning none) and "is healed for N points" for a positive one, also in the Combat tab. All three are checked against captured messages (`mao_net::n3::chat_text` and `feedback_text`). Who dealt the damage is not in the message, so the lines name only the target.

## Weapon special attacks

The Action window's and the hotbar's weapon specials now send `CharSecSpecAttack` (`51492120`, `n3::special_attack`): a zero byte, the target's identity and the **skill's stat**. The captures show five: 142 Brawl, 144 Dimach, 147 Fast Attack, 148 Burst and 150 Fling Shot (the same stats the Skills window lists), and `special_attack` reproduces two of them byte for byte. Brawl, Dimach, Burst and Fling Shot are wired to the matching actions; **Full Auto** uses stat 167, which is the skill's stat in the public tables but was not in a capture, and Fast Attack is not in the Action window's list. The target is the current one (Tab selects); without one the hotbar says so. What the server answers (the `SpecialAttackWeapon` message that starts the recharge, and the "You successfully perform ..." line, now in the Combat tab) is not used to set the hotbar cooldown yet: the cooldown is the entry's own.

## Using perks

A perk action of the Action window or a hotbar slot now sends `CharacterAction` action `0xb3` (`UsePerk`, `n3::use_perk`). Earlier notes could not tie its parameters to anything: the first is the **perk's id** (`0x2991`, `0x2a0a`, `0x27bc` in the captured uses, the fifth column of `actions_data::ACTIONS`) and the second is its **four-letter hash as ASCII** (`QISH`, `WRTK`, `BICC`: the fourth column), so the "five-letter string" was a misreading of the id's low byte. The target identity is the character itself, as in all the captured uses; the perk acts on the current target the server knows from the target messages Tab sends. Three captured uses are reproduced byte for byte. The table lists the 35 perks of the character the Action data was read from; perks of other characters are in the Perks window but have no row here, so they cannot be fired yet (the table would have to be built from the perk records, which carry the hash). The server's answer and the recharge are not used.

## Chat log

Every line the chat window receives or the player types (tells, channels, game text, combat) is also appended to `chat/<date>.txt` beside `mao.ini` (one file a day, UTC): `[09:15:02] tell: Mothra tells you: hi`, with the line's kind (`system`, `tell`, `channel`, `you`, `combat`). `MAO_NO_CHATLOG=1` turns it off. It is only written by the live client, not by the offline previews.

## Screenshots (`F12`)

`F12` (the `screenshot` binding) saves a picture of the world as drawn, with the heads-up display (target, labels, messages, hotbars) but without the open windows, to `screenshots/<date>-<time>.png` beside `mao.ini` (UTC) and says where in the message log. It draws the scene again off-screen at the window's size with the live camera (`Renderer::render_to_image_with`, the path the headless previews use) and paints the HUD over it with `paint_headless`. **Not run in a live window** (only built; the pieces are the ones the headless images already exercise).

## Window positions are remembered

Where the player leaves a window is kept in `layout.ini` beside `mao.ini` (`Nano programs 220 90`, one line per window: Actions, Chat, Friends, Inventory, the container window, Mail, Missions, Nano programs, NCU, Perks, Shop, Skills, Stats and the health panel) and used as the window's starting place next time (`layout.rs`; `crate::layout::pos(name, default)` wraps each window's `default_pos`). The live interface looks at the windows' places every frame and writes the file when something moved, at most every two seconds. `MAO_NO_LAYOUT=1` ignores it; the offline previews (`MAO_UI_PREVIEW`) ignore it too. Window sizes are kept too since 2026-11-10 (`sizes.ini`, below; which windows are open are in `open.ini`). The file format and the round trip are tested. A window with no line takes the original client's own frame for the character when its preference files are on the machine (see "Window places from the original's own files").

## The launcher (`mao-client play <game>`)

A window with the login form (account, password, server, `cm.d1.funcom.com:7503` by default) and, after a successful login, the account's characters (name, level, profession, area) with a **Play** button each (disabled for a banned or uncreated one). The login runs on a thread (`session::login`, the same exchange `net chars` uses) and its connection is closed before the game starts. **Play** closes the launcher and starts `mao-client net <game> enter <name> --live` as a new process (its own window and event loop) with `MAO_USER`, `MAO_PASSWORD` and `MAO_SERVER` in that process's environment only. Only the account name is saved (`account.ini` beside `mao.ini`, to fill the field next time); the password is never written. The window opens and draws the form (checked); the login itself and the hand-over to the game were **not run** (no account in this session).

**Checking the turn's direction.** `mao-client <game> rooms-verify` takes the machines, terminals and chests built into every room playfield (the dynel table) and counts how many stand over a floor cell of the nearest room as placed, and how many would if the turn went the other way. Of 1294 things in unturned rooms 1198 do (the rest belong to a neighbouring room); with one quarter turn 58 of 74 against 43 the other way, with three 96 of 111 against 85, and with two the senses are the same by symmetry. The placement used by the shell and the statels is the better one in both turned cases, but the margin is modest (a floor is a large target): the direction is likely right, not proven.

## More chat commands: /ignore, /camp, /quit

Three more of the original's commands now do something. `/ignore <name>` hides that character's tells and channel lines (the window drops them as they arrive; the same command again shows them; `/ignore` alone lists who is ignored); the list is kept in `ignore.ini` beside `mao.ini`, one lower-case name a line, and channel lines now carry who said them. `/camp` sits the character down for the logout (the `X` key's action; stand up or move to cancel). `/quit` leaves the client. The rest of the original's list still answers "not carried out yet".

The launcher's character rows read `level 220 male Solitus Enforcer`: the sex and breed use the stat values (breed 1 Solitus, 2 Opifex, 3 Nanomage, 4 Atrox; sex 2 male, 3 female), the profession the usual numbering (9 Enforcer, 15 Shade, ...). The captured character lists back the tables: of their names, Imnotshade is profession 15, Redmag3 12, Diabete5 10, Braticrat 8, Noncents 7 and Gigabolts 3, which are a Shade, a Meta-Physicist, a Doctor, a Bureaucrat, a Trader and an Engineer. The list's "area unknown" is not shown.

## Name plates

The names of the characters and creatures within 40 units float over their heads, coloured by what they are as the map's dots are (players blue, friendly NPCs green, hostile creatures red, the target gold), the nearest forty at most. The network thread builds them from the entities it knows (name from the entity info, position plus 2.3 over the head) with every update; `hud_layer` projects each one and draws it with a dark shadow. Preview: `MAO_PLATES=1 MAO_UI_PREVIEW=1 mao-client <game> map 540 --image out.png`. Health bars and levels are not shown (the server's health updates for others are damage events without a maximum).

## Floating combat numbers

Each `HealthDamage` also floats its amount over the one hit for 1.6 seconds, rising and fading: damage to others in yellow, damage you take in red, healing in green (the target's entity, or your own character's place, 2 units up). The numbers come from the same message as the Combat tab's lines. Preview: `MAO_PLATES=1 MAO_UI_PREVIEW=1 ...` draws three samples.

## Help window (`Ctrl+H`)

Lists the keys as they are bound now (so a rebinding shows), the mouse controls and the chat commands the client carries out, with a find field that filters all three (`help.rs`). The window's place is remembered like the others'.

The launcher also remembers the character played last (the second line of `account.ini`) and lists it first, marked "(played last)".

## Chat input history

The command list (the client's own, CHT-035) also takes the mouse (2026-11-21, the user's ask): a press on a row takes that command as Tab does (on the press, since the bar loses the keys, and the list with them, when the button is let go), the row under the pointer is lit, and the list scrolls (`egui::ScrollArea`, the wheel or its bar) over every match, at most 15 rows high, its foot 4 pixels above the bar (it was as tall as the screen above the bar allowed, showed only the rows round the highlighted one, and stood off the bar where it was shorter). Up and Down still move the highlight, and the list follows it. Seen in a preview (`MAO_UI_PREVIEW=chat MAO_CHAT_INPUT=/`; in Git Bash with `MSYS_NO_PATHCONV=1`, or the slash becomes a folder); not clicked live.

With the input line focused and no command list open, Up and Down walk the lines sent before (the last hundred; a line sent twice in a row is kept once), Down past the newest empties the line.

## Auto-run (`Numpad *`)

The `auto_run` key keeps the character walking forward until it is pressed again or a forward or back key is used (the key is rebindable in the options window). Not tried live.

## Item menu (right-click in the inventory window)

Right-clicking an item opens a small menu: **Use** (the use message the hotbar sends for an inventory place) and **Delete**, which asks a second click ("Really delete?") before anything is sent. Deleting is `CharacterAction` action `0x70` (`DeleteItem`, `n3::delete_item`): the target `(0x68, place)` and the item's id twice as the two parameters, reproduced byte for byte from the one captured delete. The item leaves the window at once (the server's answer was not identified, so a refused delete is not taken back). Not tried live. Splitting a stack (action `0x34`, captured once) is not wired: its parameters (0 and 1 in the capture) could mean several things.

## Interface size

The options window has an **Interface size** slider (60% to 200%, `ui_scale` under `[window]` in `mao.ini`): it sets egui's zoom factor, which multiplies the window's own scale, so every window, the hotbars and the labels grow together (for a large or high-density screen). Not tried in a live window.

The inventory window has a **Find** box: items whose name does not contain the text are dimmed, so an item is easy to spot in a full backpack.

## Mail terminals from the dynel table

"Check mail" used four mail terminals seen in captures. The playfield's dynel table (`mao-client <game> dynels <playfield> c773` lists a kind) holds **385 mail terminals in 203 playfields**, each with its serial (`0xc0NN` over the playfield: `0xc003021c`), and the ones the captures knew are there at the same places (`0xc003021c` at (446, 454) against the captured (445.4, 456.1)). The client now looks the nearest one up there (within 25 units) and keeps the captured list as the fallback. `dynel-kinds` counts what the tables hold: doors 5084, terminals (`0xc73d`) 3108, mail 385, vending machines 886, and `0xdac6` 2242, `0xdac1` 407, `0xc418` 269, `0xc790` 210 that nothing draws or uses yet.

## View distance (`/viewdist`, options)

How far the live view sees: the options window's **View distance** slider (200 to 4000, `view_distance` in `mao.ini`, 1800 by default) or `/viewdist <n>`. It is where the fog ends; the haze starts a seventh of the way out (250 for 1800), and the scene's fog is set from it every frame, so it takes effect at once. Not tried in a live window.

## Character distance (`/chardist`, options)

Characters and creatures farther than this from you are not drawn (their name plates and the map dots still follow what is known): the options window's **Character distance** slider (0 to 400, 0 meaning all; `char_distance` in `mao.ini`) or `/chardist <n>`. Machines, doors and the other built-in things are always drawn. It is meant for crowded places, where drawing every character costs the most. Not tried live.

## Other characters' social actions (dances, bows, ...)

When the server announces a social action (`SocialActionCmd`, `3b290771`: its last word is the action's number, the same numbers the Action window's social list uses: 6 is the backflip, 62 another; the words before it are a counter and flags), the character it comes from plays that animation once (`<male|female|athrox>_social-<name>.ani`, found in the database by name: 75 male, 74 female and 75 Atrox ones). After the animation ends the character is drawn as usual again. Our own social actions still are **not sent** (the message of the client's side was not in any capture), so they do not animate our own character either.

## Cancelling a running nano (NCU window, right-click)

Right-clicking a running nano in the NCU window offers **Cancel this nano**. The message is `CharacterAction` action `0x41` (`RemoveFriendlyNano`, `n3::cancel_nano`): the layout of a cast with no target, the nano identity `(0xcf1b, id)` in the two parameters, reproduced byte for byte from the captured cancels (so the "learn / remove" doubt about action `0x41` in the earlier notes is settled). The nano leaves the window at once and its stat bonuses are taken off; the server's own update of the list is not waited for. Not tried live.

## The target's health (info requests)

Choosing a target (Tab, a click) now also sends `CharacterAction` action `0x69` (`InfoRequest`, `n3::info_request`: the target `(0xc350, id)` and zero parameters, byte for byte as in the captures where the original sends one for every target). The server answers with an `InfoPacket` (`4d38242e`) from that character or creature: after the flag byte and the type (64-71 a character, 80 a monster) come five bytes (a flag, the profession, the **level** (200 for the captured character, meaningless for a monster), the title level, the look's profession), the side's experience (a short) and the **health and its maximum**; two captured packets are decoded in the tests (a monster at 92 of 92, a level-200 character at 54304). The target frame shows `HP 54304/54304`, and the damage events keep the current value moving afterwards. The monsters' packets carry no name or level in this layout.

A character whose info packet came (one that was targeted) shows its level in its name plate: `Mothra (200)`.

## The server's cooldowns for weapon specials

The hotbar's weapon specials now recharge as the server says, which the earlier note listed as not used. `CharacterAction` messages to the character carry the timing: action `0xaa` (`SpecialUsed`) names the skill's stat and the recharge in seconds (in the captures Brawl 142 is 15 s, Fast Attack 147 9 or 15 s, Burst 148 20 s, Fling Shot 150 8 s, Dimach 144 1800 s), action `0xa4` (`SpecialAvailable`) says the stat is ready again. `Hotbars::set_special_cooldown` sets the slots' `ready_at` for every entry of that special (`hotbar::special_stat` maps the action names to the stats), so the clock and the queued presses follow the server. Perks have the same pair of messages (`0xcf` with a perk number and the seconds, `0xce` when ready: 25 s, 55 s, 120 s ... in the captures) but their numbers (172, 201, 641 ...) are not the ids the perk actions here carry, so perks keep their own cooldown for now.

## The NCU window follows nanos cast after login

The NCU window only knew the nanos running at login. The server also sends `SetNanoDuration` (`CharacterAction` action `0x62`: the nano `(0xcf1b, id)` as the target, the character it is on and the duration in milliseconds, which equals the nano template's own duration stat: 5000, 2200 and 600 in three captured ones). A nano that lands on the character now joins the list (or has its time reset), its stat bonuses count at once, and a nano whose time has run out leaves it. The other nanos' remaining times are brought up to date when one is added. Tested with a unit test of the list; not seen live.

## Auto-loot

The options window's **Take everything from a corpse as soon as it opens** (`auto_loot` in `mao.ini`, off by default): when a corpse's contents arrive the client sends the take message for every item at once (the loot window still opens and empties as the server confirms each). Only corpses (`0xc76a`) are emptied, never bags or bank terminals. Not tried live.

## /follow

`/follow` walks after the current target (a character or creature that is about; it uses the same walking-to-a-place logic as the Whom-Pah Go button and its detours round obstacles), stopping within 3.5 units of it and starting again when it moves away. Typing it again, any movement key of your own, or the target leaving stops it. This is the client doing the walking; the server is told nothing of it. Not tried live.

## /inspect

`/inspect` asks for the current target's info packet and tells what comes in the chat window: `Mothra: level 200 Enforcer, health 54304 of 54304.` for a character, `<name>: health 92 of 92.` for a creature (the packet has no level for those). Not tried live.

## More commands: /open, /close, /items, /waypoint, /selectself, /setoption

* `/open <window>` shows a window by name (`inventory`, `stats`, `skills`, `ncu`, `friends`, `programs`, `perks`, `help`, `actions`, `missions`, `mail`, `map`, `chat`, `whompah`, `loot`); `/items` is `/open inventory`; `/close <window>` hides one and `/close` alone hides them all (`Ui::set_window`).
* `/waypoint <x> <z>` walks to a place with the same logic as the Whom-Pah Go button (any movement key stops it).
* `/selectself` targets the character itself (the select and target messages with its own id).
* `/setoption name value` sets `vsync`, `render_scale`, `sharpness`, `ui_scale`, `view_distance`, `char_distance` or `auto_loot` (values are clamped to the options window's ranges) and saves; `/setoption` or `/option` alone lists them (`Settings::set_option`, tested).

## Interiors against the dark

The sky is a scene setting now (`Scene::sky`: the horizon colour, which is also the fog's, and the zenith). The live view uses the dark one (`mao_render::DARK_SKY`) in a playfield with no ground (the shops and dungeons built from rooms) so a room is not seen against a blue daylight sky with a pale fog, and the daylight one elsewhere. The offline `map` images keep the daylight sky. Not seen in a live window.

## Nano casts in the Combat tab

~~A `CastNanoSpell` that involves the character becomes a line in the Combat tab.~~ Replaced by the original's behaviour: the server's `CastNanoSpell` has no line (its handler, FUN_1007284f → FUN_10051c98, only starts the cast), and casting a nano yourself tells "Executing Nano Program: Heal Me." in the Combat tab (N3Msg_CastNanoSpell's feedback 0x47), in place of the on-screen "Casting ..." note.

Missed attacks at or by the character (`MissedAttackInfo`: the attacker and the defender) add `You miss Rollerrat.` / `Rollerrat misses you.` to the Combat tab. (`AttackInfo` carries the damage dealt, the target and the kind of hit, but `HealthDamage` already gives the lines and numbers, so it is not repeated; `Feedback` carries only text ids of a text table this client does not have.)

**Superseded: the fight lines are now the original's** (`feedback.rs`). Gamecode formats every fight line from `cd_image/text/text.mdb` (`mao_formats::ldb`: "MMDB", sorted `{category, offset}` tables, texts asked for by category 110 and the ElfHash of a name such as `Feedback_HitWithSpecial`, filled in order as `LDBformat::Feed` does). The client reads that file (with the English texts kept as a fallback) and posts what the original posts:
- `AttackInfo` (FUN_1009b6b7): "You hit Rollerrat for 1913 points of projectile damage." / "Rollerrat hit you for ..." / "Player Bob hit you for ..." / "Bob hit Rollerrat for ...". The damage type is the attacking weapon's stat 436, projectile when it has none (FUN_1009b525). A critical (hit type 4) or glancing (2) hit adds " Critical hit!" / " Glancing hit.".
- `MissedAttackInfo` (FUN_1006b399): "You tried to hit Rollerrat, but missed!" / "Rollerrat tried to hit you, but missed!", or "... with <skill> ..." when the message names a stat.
- `HealthDamage` (FUN_100a0611): "You hit Tower Shark with nanobots for 2833 points of melee damage.", "You were attacked with nanobots from Rollerrat for 1581 points of cold damage.", "Rollerrat was attacked with nanobots for ...", "You were healed for 116 points." (only your own heals have a line), and the fall-damage lines.

- `Absorb` (FUN_1009ef78): "You absorbed 1581 points of cold damage."; another character's absorb is "Someone absorbed ..." (the handler gives no name).
- `ShieldAttack` / `ReflectAttack` (FUN_100a134d / FUN_100a11c2: the one hurt is the identity, the shield's owner `Target`, and its health goes down by the amount): "Your damage shield hit Rollerrat for 173 points of damage.", "Bob's reflect shield hit Rollerrat ...", and "Someone's damage shield hit you ..." (the handler drops the owner when it is you who is hurt).
- Another character's health in a `Stat` message is held, and a drop tells "Rollerrat was attacked with nanobots for 81 points of unknown damage." (FUN_100a2006's 0x1b: no source, no type).
- Your own `Stat` changes (FUN_100a2006, by the change from the value held): "You received 1200 xp." / "You lost 5 xp.", "You gained N new Alien Experience Points.", "You gained/lost N points of Shadowknowledge.", and health changes not already told by a `HealthDamage` (which now sets the stat, as the original does).

**Fixed with it: `HealthDamage`'s direction.** The one hurt is the message's own identity, whose health is `TargetHp`; the `Target` field is the source. The client had it the other way round, so the floating numbers and the health bars followed the wrong character (a monster's health was given to you). Checked on the captured messages.

**Facing the target in a fight.** A fighting character that stands turns to face whom it fights, as the original's character state machine does: every 0.67 s `CharFight_t` (FUN_1007bdd3) compares the facing with the line to the target, and when it is more than 0.26 rad (about 15°) off, `CharTurn_t` (FUN_1007c8af) turns it at 7.2 rad a second until it faces the target. Moving (or steering with the keys or the mouse) holds the look off. It applies to your own character and to the others (between the server's rotations for them). Not tried live.

- **Inspect answers**: an `Inspect` message (`5a585f65`: identity, count, 32-byte records of slot, flags, identity, low id, high id, quality) is decoded (`n3::inspect_items`, tested on captured bytes) and told in the chat window as "Name wears N items:" with each slot and item name. Not tried live: whether `/inspect`'s info request is what makes the server send it is unknown from the capture.

- **Open windows are remembered**: the hand-opened windows (inventory, stats, skills, NCU, friends, programs, perks, help, actions, missions, map) are listed in `open.ini` beside `layout.ini` whenever the set changes, and come back open on the next start. `MAO_NO_LAYOUT` and the previews ignore it. Not tried live (the window set is only checked by building).

- **Absorb / damage shield lines**: `Absorb` (`264e5f61`: amount, damage-type stat; the captured 1581 of stat 95) tells "You absorb N points of cold damage." and `ShieldAttack` (`25192476`: amount, target) tells "X's damage shield hits you for N points." in the Combat tab. The shield wording is a guess at which side the message identity is (the source, from the capture's ids). Not tried live.

- **Organization names on nameplates**: an `OrgInfoPacket` (`2e2a4a6b`: org id, name; captured "Corrupted", "The Asylum") adds `<Org>` after the name and level on that character's nameplate. Not tried live.

- **Reload line**: a `Reload` (`26515e61`) of your weapon now tells the original's "Reloading..." and, once the weapon's reload time (its stat 210, in hundredths of a second) is over, "Reloading completed" (FUN_100697bd, FUN_10068fd2 → FUN_1006884a; told through FUN_10059047 in the plain feedback channel, here the System tab). Not tried live; no reload is sent by the client yet.


- **Stop fighting**: a `StopFight` (`4a41203e`) tells nothing (the original's handler, FUN_1007a2be, only holsters the weapon); the client's own "You stop fighting." line is gone.

- **Interface size keys**: Ctrl+= and Ctrl+- make the interface a tenth larger or smaller (0.6 to 2.0; the same `ui_scale` as the options slider, saved at once). Rebindable in the options.

- **Friends coming and going**: when a friend already on the list goes online or offline (a buddy packet that flips the state), the chat window tells "X is now online." / "X has gone offline."; the first listing at login is silent. Not tried live.

- **Worn equipment on characters**: the `AppearanceUpdate` (`41624f0d`) lists a character's clothes textures (place 0-5 = hands, body, feet, arms, legs, head: they replace its cloth list) and its **meshes**: a position, a static mesh (type 0xf6951), an optional replacement texture and a layer. The position counts the body's attractors (`Attractor01_head`, 02 right hand, 03 left hand, 04 right shoulder, 05 left shoulder, 06 back), so hair and helmets, weapons in the hands, shoulder pads and back pieces are held there (`net::wear`, using `CatMesh::attach` as the head is). Checked in a picture: `MAO_WEAR=0:4e39,1:409f7,2:409f7,3:2cf7:4a0b1,4:2cf7:4a0b1,5:2cff:4a0b1 mao-client <game> player 1 2 male_idle-stand_01_01.ani --image out.png` (the meshes of a captured character: a pistol in each hand, shoulder pads, a back piece). `mao-client <game> attractors <breed> <sex>` lists a body's attachment points; `net <game> gear-preview <log> <connection> <playfield> <png>` replays one connection of a capture. Not tried live: weapon draw/sheathe (whether a drawn weapon is in the list only while wielded is unknown), and head-slot items replacing the head.

- **Polymorph**: a polymorph nano program (nano strain 223: "Calia's Form: Wolf", "Sparrow Flight", ...) carries a `ChangeMesh` spell (`0xcf44`) whose argument is an NPC template (type `0xfde97`, the same records creatures are drawn from; `item::polymorph_template`, tested on a synthetic record). While one of the character's own running nanos is such a program, the character is drawn as that creature (`World::morphs`, set in `refresh_mods` from the running nano list). `mao-client <game> creature <template> <png>` draws any creature template (the wolf, `0x768c`, is checked); `spell-scan <game> <spell hex> [n]` and `nano-line <game> <stat> <value> [n]` list nano programs by spell type or stat. Not done: other characters' polymorphs (a cast on them has no known duration; the capture holds none), and the polymorphed character's own movement speed/animation names (it walks with the creature's idle/walk). Not tried live.

- **Nano effects (researched, not built)**: a nano program's template names its visuals in stats: `CastEffectType` 428, `GatherEffectType` 366, `HitEffectType` 361, `ImpactEffectType` 414, `TracerEffectType` 419, `EffectType` 413, `EffectRed/Green/Blue` 460-462, `CastSelfAbstractAnim` 378 / `CastTargetAbstractAnim` 377 / `GatherAbstractAnim` 376 (character animations), `CastSound` 270 / `GatherSound` 269 / `HitSound` 272 / `TravelSound` 271, and its spells carry `GfxEffect` (`0xcf26`), `AnimEffect` (`0xcf0b`) and `ScreenEffect` (`0xcf57`). The effect ids (43010, 46133, 49999 as the default, 17300...) are **not** resolvable as resource-database objects: the same numbers as instances are unrelated meshes (a tree, a door), and no table in the game's DLLs holds them (checked by byte search). They key the original's effect system (`FXS.dll`: clouds, lightning, clusters), whose definitions are in code or data not yet found, so effects cannot be drawn faithfully yet. What could be done without them: the character animations (`Cast*AbstractAnim`) and a generic coloured glow from `EffectRed/Green/Blue`. Tools: `mao-client <game> instance-name <hex>...` names instances; `STATS=1 mao-client <game> nano-line 75 <line> [n]` prints those stats.

- **Cast animations**: a `CastNanoSpell` (`25314d6d`) makes its caster play the humanoid casting animation: `<male|female|athrox>_spell-self_01_01.ani` when it casts at itself, `spell-dir` at someone else (the same machinery as social actions; checked in a picture: the arms-out pose of `male_spell-dir`). The nano templates carry no animation stat (no `CastSelfAbstractAnim` in any of them), so the choice is only self or directed; the library also has `spell-gen`, `spell-sys`, `spell-self_02` and `unarmed-spell1-3` that the original may pick by school or weapon. Creatures cast without an animation here. Not tried live. (This corrects the research note above: the animation stats are not in the templates.)

- **Nano glows**: every `CastNanoSpell` draws three widening, fading rings at its target for about a second (`ui::Glow`, drawn with the nameplates and floating numbers, over the target's chest). The colour is the program's `EffectRed/Green/Blue` stats (460-462) when it has them, else one of six by its school (stat 405); none of the polymorph/buff templates checked has the colour stats, so by school is the usual case. A stand-in for the original's effects (see the research note above), not a copy of them. Preview: `MAO_UI_PREVIEW=1 MAO_PLATES=1 mao-client <game> map 540 --image out.png` adds three glows next to the sample names (small at that camera distance). Not tried live.

- **Other characters' polymorphs**: a `Buff` (`39343c68`: the character in the identity, the nano program as the instance of its `Buff` field) names a program the character runs; the server repeats these about every twenty seconds (the capture shows the same buff at 20 s steps). A buff whose program is a polymorph (nano strain 223) turns that character into its creature for 30 s after it was last seen. The captures hold such buffs (nanos `0x41999`, `0x46f18`, `0x42aba`), so the message path is real, but I have not seen the result drawn live, and whether the server sends a message when a buff ends is unknown (the 30 s timeout covers it).

- **Character screen**: after the login the launcher lists the characters on the left (click one to pick it, double-click to play it; the one played last starts picked), shows the picked character in the window (its breed's and sex's body and head, standing, turning slowly over a dark blue sky: `net::character_model`, the same models as in the world) with the name and description over it, and a large **Play** button at the bottom (Back at the lower left). The list carries no appearance (only breed, sex, head and size numbers), so the figure wears the base clothes, not the character's own; `Character::head/height/width` are not used yet. Pictures without a login: `MAO_LAUNCHER_DEMO=1 [MAO_LAUNCHER_PICK=Mothra] MAO_LAUNCHER_SHOT=out.png mao-client <game> play` (four characters copied from a capture's list; saves the picture and quits). The README had the arguments the wrong way round: it is `mao-client <game> play`.

- **Remembered looks**: the character list has no gear, so (as in the original, which shows gear only after a first login) the live client writes down what the character looks like (`look.rs`: its breed and sex, clothes textures, texture overrides and worn meshes) to `looks.ini` beside `mao.ini` whenever it changes (one line per character: `name|breed|sex|part=texture,...|material=texture,...|position:mesh:texture,...`, tested round trip). The character screen dresses the picked figure from it; a character never played here is shown in the base clothes. The picture `MAO_LAUNCHER_DEMO=1 MAO_LAUNCHER_PICK=Diabete5 ...` uses the look of that character from a capture's appearance update (kept in memory, not written to `looks.ini`). The cache is only written by the live client, which I could not run (no login); the writing path is the one built and checked here, the reading path is checked in the picture.

- **Real heads**: an appearance update's mesh list carries the character's own head at position 0 (capture of the original client: Fallacy `head_opifexfemale37.abiff`, Diabete5 `head_nanofemale33.abiff`, layer 4). The client drew the breed's default head (`head_*01`) and the real one on top; now a position-0 mesh whose file name is a player head (`Characters::head_meshes`, from the instance index) replaces the default head. The head of an Opifex or any other breed therefore follows what the server sends, and the character screen shows the saved one. The launcher window is 1000x640 (the list no longer covers the figure).

- **Creatures with long names were not drawn**: the creature template (the NPC record a monster is drawn from) is found in the entity message by its pattern (`template, 0, level, 0, 0x1f`) a little after the flags, and the search stopped 40 bytes in; a name of 12 or more characters ("Chimeran Canine", "Granite Spider") pushed the template past that, so those monsters were drawn as plain markers. Found with the original client on the user's Opifex in Upper Scheol (playfield 4880): 14 of the 49 entities had a template, now 48 (the 49th is the character). The search reaches 72 bytes (regression test with a long name). Upper Scheol itself is a cave whose floors are placed objects (the terrain grid is flat at 0 there), and it draws.

- **Faster zoning and first view** (`MAO_PROFILE=1` prints the timings; `mao-client net <game> area <playfield>` loads one as the live client does): a playfield used to be decoded texture by texture on the thread that owns the window (Old Athen: 1.1 s frozen, then 0.8 s of loading before it). Now (a) `Textures::prefetch` decodes every texture of a scene on all cores and builds its mip levels there (`Texture::with_mips`, the texture's `mips` field), (b) the collision's opacity check reads those decoded copies instead of decoding them again, (c) the renderer's upload makes the GPU textures and each draw's uniform buffer and bind group on several threads (the device takes calls from any thread). Measured on this machine, Old Athen: load 0.9 s -> 0.24 s and the main-thread upload 1.15 s -> 0.18 s; Upper Scheol: load 0.5 s (the 897x1025 ground's collision is most of it) and upload 0.57 s -> 0.2 s. The pictures are the same (differences only in the wind animation's phase). Not measured live (no login): the server's own latency, and the first zone after a cold start (the files are read from disk then).

- **Polymorph while standing**: some polymorph creatures have no standing animation (key 120: the "Pronouncement of Greatness"/L33t creature has none, the Sparrow Flight bird's 120 pointed at a nonexistent animation), so the creature could not be built and the character fell back to its own body whenever it stood still (it showed only while walking). `Template::animation_candidates` lists an action's animations best first (idle: 120, 127, the walk), the client takes the first the database really holds, and a standing creature that only has the walk holds its first frame. `mao-client <game> morph-check` lists every polymorph program's creature with its animations. Checked in pictures of both creatures.

- **Zoning no longer logs in again**: on a redirect (a zone line, a whompah) the client connected after logging in afresh, which the server refuses until it has let go of the old session: over a hundred failed attempts and a minute or more of waiting per zone. The original connects to the redirect's address straight away and sends the character with the *session key of the first login* (the captures show the same eight bytes for every zoning of a session, and the new connection opens about 60 ms after the old one ends). `net::hop_direct` does that; the log-in-again route remains as the fallback. Confirmed by the user in the live client ("much better"). `net <game> area <n>` / `MAO_PROFILE=1` time the loading; a sweep of all 630 playfields loads without a failure.

- **Polymorph animation speed**: a polymorphed character's creature animation plays at `MORPH_ANIM_RATE` (0.75) of real time, as the user found it a little fast at 1; the creature templates hold no animation-speed stat (checked the wolf's: mesh, radius, mass, flags, features and the animation list only), so the rate is a constant to tune. A running polymorph (speed over 5.5) now plays the creature's run animation instead of its walk. `mao-client <game> npc-words <template>` prints a creature record's words.

- **Buff auras**: a character under running nano programs shows a soft, pulsing column of light round its body, a ring at its feet and sparks climbing it, one colour per program (up to four; the program's effect colour stats or one by school, as the cast glows). The character's own programs come from its running list (the NCU), the other characters' from their `Buff` messages (announced about every twenty seconds; one seen in the last 30 s counts). Drawn over the screen like the nameplates and sized by the figure's height on screen, so near characters get big effects. A stand-in for the original's effect meshes (see the research note: their ids resolve to nothing in the data files). Preview: `MAO_UI_PREVIEW=1 MAO_PLATES=1 mao-client <game> map 540 --image out.png` (with `MAO_CAMERA=449,309,10.5,6,0.9,0.2` for a close view). Not seen live.

- **Where the nano effects are (found)**: `Setupf/gfxtweak.bin` in the game folder (333 KB, 83,283 little-endian words; the client's `ReloadGfxTemplates` / `_GfxTemplateStorage_t` load it) holds the effect templates. The word 0 is 2687 (the template count?), and records follow with a key word then a kind (`2001`, `2004`, `2007`, `1010`, `1005`, `1001`...: 880 of kind 2004, 486 of 2007) and floats/ints. Every effect id a nano's template names is a key in it: the hit effect 43010 (word 13149), the cast effect 46133, the gather effect 49999 (the default, many times), the tracer 17300, and the `GfxEffect` spell's argument 13600 (0x3520). So the mapping the original uses is this table (id -> a tree of effect elements: meshes, particles, lights, with sizes, colours and times); its record layouts are not decoded. Spell effects reach it through `_EffectHandler_t::CreateGfxControl(id, target...)` (Gamecode.dll). The stand-in 2D auras looked bad on a character and are off by default (`MAO_AURAS=1` turns them back on); the cast rings stay.

- **Nano effects decode, step 1 (the file and the trees)**: `mao_formats::gfx` reads `Setupf/gfxtweak.bin` completely: 2687 records, each `id, kind, n, payload`, to the last word of the file (test against the game; `tools/gfx_scan.py` is the same reader for research). `mao-client <game> gfx <effect id>...` prints a template and everything under it, with floats and `>id` references. Which classes exist: `CreateGfxControl` makes a control for the kinds 1005, 1013, 1018, 1019, 1021, 1022, 1024-1027, 2002, 2004, 2005, 2007, 2010, 2013, 3011, 3015, 3017, 3019, 3020, 3026, 3028 and 4000 (the sprite, particle, spiral, nano and shield visuals of DisplaySystem.dll: `GfxVisualSprite2Type0`, `GfxVisualSpiral`, `GfxVisualNano2`...), the others (1001-1004, 1006-1012) are the parts those refer to. How the nano stats land (over all 1328 nanos that name effects): the **running buff** (`EffectType`) is kind 1001 in 486 (a colour and two kind-1002 helix emitters: a blue `0.2 0.2 0.99` or red `0.99 0.2 0.2`... aura of two counter-rotating spirals of sparks, spin 3.14 and -12.57 rad/s, with a radius path of 21 pairs) and 3004 in 114; the **cast** is kind 1010 in 1320 (colours and six parts); the **hit** kind 2004 (586), 2007 (300), 1018 (189), 2001 (123); the **tracer** 2004 (691), 2007 (319), 2002 (180), 2005 (99); the **gather** is the null effect 49999 (kind 2007, ten zeros) nearly always. What is left is the meaning of each class's payload and drawing it (billboard particles in the renderer): that is the next step; the buff helix (kind 1001) is the one the user asked for first.

- **Nano effects, step 2: the particle pass and the buff spirals** (replaces the screen-space auras, which are gone): the renderer has a particle pass (`mao_render::Particle`, `Scene::particles`; `particle.wgsl`): camera-facing soft sprites added to the scene's colours, depth-tested against it and not writing depth, so they pass in front of and behind the figure; the camera's right and up are in the globals for it. `mao_formats::gfx::Gfx::aura(id)` reads a kind-1001 template (the running buff) into an `Aura` (the colour, the end colour and the kind-1002 helices with spin, start angle, spark size, count and colour); `effects::aura_particles` turns it into sparks for a time: two spirals turning opposite ways round the character (the data's spin of about 2 turns a second), climbing from the feet, each spark with a short fading trail. The live client resolves each running nano's `EffectType` (stat 413) through the table (own programs from the running list, others' from their repeated buffs) and the viewer draws the sparks at the character's gliding position every frame. Checked in pictures: effects 1000 (blue, 0.2 0.2 0.99) and 1001 (red, 0.99 0.2 0.2) draw as a double helix in their colours: `MAO_GFX=<game>/Setupf/gfxtweak.bin MAO_AURA=1001 MAO_AURA_AT=0,0,0 MAO_AURA_TIME=1.3 mao-client <game> player 1 2 male_idle-stand_01_01.ani --image out.png`. Chosen rather than decoded: the radius (0.42 m: the radius path's numbers are 0.06-0.15), the height (1.9 m), a spark's life (1.6 s), the trail, and the sprite size's scale; the other running-effect classes (3004 in 114 programs, the rest) are not drawn yet, nor the cast, hit and tracer effects.

- **Nano effects, step 3: cast, tracer and hit** (all drawn by the particle pass; the decode is partial and checked in pictures only):
  * the **cast** (`CastEffectType`, kind 1010 in 1320 programs) is its tree of particle emitters (kinds 1003, 1005, 1006, 1007, 1008 share one field layout: words 16-19 and 20-23 are the start and end colour as alpha, red, green, blue; 12-13 the start and end size; 26-28 the azimuth and elevation ranges of the directions; 29-30 the speeds; 31 the sprite count; 32 the pull; 35 a particle's life; 10 the rate when it is not zero). `Gfx::emitters` reads them and tints them with the cast's own colour (words 11-13 of the 1010: blue `0.2 0.2 1`, cyan `0.5 0.995 0.995`...); `effects::emitter_particles` makes the particles for an age with no stored state (a hash of the particle's number picks its direction), and a cloud of big sprites has its strength cut by how much it covers so it does not add up to white. Played over the caster's hands for a second at every cast the server announces. `MAO_CAST=<effect id>` previews it;
  * the **tracer** (`TracerEffectType`, kinds 2004, 2007, 2002, 2005) and the **hit** (`HitEffectType`, kinds 2004, 2007, 1018, 2001...): the classes 2004, 2005, 2006 and 1018 are built on a kind-1001 spiral (the buff's), tinted with their own colour (2004's words 19-21: the tracer 17300 is gold, 13600 purple). `Gfx::aura` takes these too; `effects::Shot` flies a short tight spiral from the caster to the target in 0.45 s and then stands a taller one at the target for 1.4 s, fading out. `MAO_AURA=<id>` previews any of them;
  * **since drawn** (step 4 and the paragraphs after it): the second running-effect class (3004/3003), the hit effects of kind 2007 (bundles), kinds 2001/2002, and the effects of items and spell functions (`GfxEffect`'s argument). Chosen rather than decoded: the radius, height and life of the spirals, the hand height of the cast, the flight time and the hit's size. The sprite sheet each emitter's material number names (word 9: 8, 15, 10, 31...) is not read: every particle is a soft round dot.

- **Nano effects, step 4: the rest of the classes and the real sprites**:
  * every effect id a nano names now reaches a look (`mao_formats::gfx::Visual`) except a few rare classes: a **bundle** (kind 2007: its first words are up to four children; 300 hits, 319 tracers, 63 buffs) and a **sequencer** (kind 3004, `_GfxControlSequencer_t`: word 1 entries of child, start and end from word 2) show every child that has a look, a sequencer's from each entry's start (`Visual::Delayed`; an entry's end, which cuts the child's duration to `end − start` in 56 of 522 entries, and flag 0x400's looping round are not followed); kind **3003** (the running effect of 114 programs, 52 of them one cyan ground glow, `0x8800ffff`: the packed colour is word 10, the sprite size word 11) is a soft glow under the feet that breathes; kind **2001** (the hit of 123 programs) a short bright flash with a ring of sparks; kind **2002** (the tracer of 250) a glowing ball in flight. Since drawn (the paragraphs below): kinds 1025, 1022 and 1019 (the tracers), 3001 (the deformer), 3006 (the sky flash) and 3009 (the crazy cones); since drawn too: 2011 (`_GfxControlHighlight_t`, the highlights of 9 programs such as Dimensional Shift and the Monster Cloak) and 3017 (`_GfxControlGlobalSmoke_t`, the fall effect's mist; no program reaches it);
  * **the sprite sheets**: the material number of an emitter (word 9) is a row of `_MaterialManager_t::GetMaterial`'s 102-case switch (Gamecode.dll): a texture by file name with its columns, rows and first and last frame (`mao_formats::gfx::MATERIALS`: 8 = `light_halo2.png`, 15 = `s_bullet.png`, 31 = `x_smoke.png` 8x8, 10 = `muzzleflash.png` 8x8, 3 = `explo04_01_v01.png`...). The particle pass cuts particles from them (`Particle::sprite`/`rect`, `Renderer::add_sprite`; the client loads each sheet once from the database, `Textures::sheet`), plays a sheet's frames over a particle's life, and a sheet that cannot be loaded draws as a soft dot. The blend is now additive with the sheet's colour times its alpha;
  * **creature animation speed**: `Setupf/animcalibration.txt` lists, for a creature's mesh and a movement animation (instance numbers), the factor the original plays it with (the wolf walks at 0.7 and runs at 0.5, the sparrow 1.2, the greatness creature 1.5; tab separated lines `mesh animation factor`). Read at start (`effects::calibration`) and applied to every creature's animation, which replaces the guessed 0.75 for polymorphs.

- **Polymorph sidestep**: a creature's template names its sidestep animations by key: 134 is `sidestep-left` and 135 `sidestep-right` (the wulf's and the cute creature's: keys 134/135 of the wolf are the wulf's `sidestep-left`/`-right`; the L33t creature's roll it onto its side, checked in a picture). A polymorphed character that sidesteps on its own (strafe key, no forward or back) plays them (`Action::StrafeLeft`/`StrafeRight`, `Template::animation_candidates`: 134/135 then the walk). Only the player's own sidestep is known to the client: another character's direction is not read from the server yet. `MAO_ACTION=left|right|walk|run mao-client <game> creature <template> <png>` shows an animation.

- **The original's keys and a clean screen** (from the user's copy of an install's per-user folder, `70dad3e6/Anarchy Online/Prefs`, and the game's shipped `cd_image/gui/Default/CharPrefs.xml`): the key bindings of a character are `Prefs.xml`'s `KeyBindings`, a list of `Input` (a key of the game's own enum plus modifier bits: Shift `0x20000`, Ctrl `0x40000`, Alt `0x80000`, a release `0x100000`) and `Provider` (the action: a hash of its name, `h = 0; for each byte c: h ^= (h << 6) + (h >> 2) + c + 0x9e3779b9`, found in `GUI.dll` at `0x10019281`). The key enum is the table at `GUI.dll` file offset `0x2613f8` (14 ESC, 15 TAB, 23 SPACE, 24 ENTER, 39-50 F1-F12, 70-80 the number pad, 82-107 A-Z, 108-117 1-0, 118 slash...). All 97 provider hashes resolve to names found in the DLLs (`MOVEMENT_FORWARD_V3`, `ACTION_BRAWL`, `WINDOW_INVENTORY`, `SHORTUCT_BAR_ACTIVE_n`...), so the whole default keymap is known; it is now this client's default (`settings::ACTIONS`): movement W/S/A/D, Z and C to sidestep, the arrows as the "global" movement, Space jump, Numpad0 auto-run, F1 select self, F8 camera view, F9 coordinates to chat, F10 options, F12 screenshot, Q attack, B/M/K/L/Comma/V/R/H the weapon and character actions, I/U and Ctrl+1..0 the windows, 1-0 the shortcut slots, Alt+1-0 the shortcut bars, the camera on the number pad (2/8 rotate, 4/6 rotate, +/- zoom, 5 reset, 7 save), chat on Enter, Slash, Shift+R (reply) and PageUp/PageDown. A settings file written for the earlier keys has its `[keys]` replaced by these defaults (`KEYMAP_VERSION`).
  The location, coordinates and frame rate over the game, and the shortcut bar's hint, are developer aids the original does not have: they show only with `debug_overlay = on` in `mao.ini` (or `MAO_DEBUG_HUD=1`). F9 does what the original's does and says the position in the chat window. Escape closes the dialogs (options, containers, shops, perks, ...) as the original's `esc_*` preferences do and leaves the inventory and the nano window; the options are F10.

**Control center.** Drawn from the original skin's own pictures (`cd_image/gui/Default/Graphics.uvga`, indexed by `Graphics.uvgi`: a text list of `NAME offset length`, each entry a whole PNG, pure green = transparent; `tools/uvga_extract.py <skin folder> <out>` unpacks one, so no skin tool is needed, and the AOU skins use the same format). Layout from `Views/ControlCenter.xml`: the left wing in the bottom-left corner with the health and nano bars standing in it, the right wing in the bottom-right with the experience bar (and the alien experience bar, tinted, once the character has an alien level), and the menu strips under them with `ActionMenu/LeftMenu.xml` and `RightMenu.xml`'s icons (Actions, Character, Nano, Inventory; NCU, Map, Missions, Friends). A click toggles the window; the icon is lit in the theme's accent while it is open; hover a bar for its numbers (the health and nano bars keep the breakdown). Without the game's files the plain strip is used. The compass (`GFX_GUI_COMPASS_SCALE` sliding behind `GFX_GUI_COMPASS_FRAME`, top centre; the bearing comes from the camera's yaw, north being the map's +z) is drawn too; the corner map stays as it is, since this client keeps it. The two target buttons (`TARGET_BUTTON` with the self/other icon) stand beside the strips: left targets yourself (F1), right the next enemy (Tab); their real function in the original is a guess. The shortcut bar's default place is above them. The Actions, Character and Map icons open a list over the icon (from `CommandMenu.xml`, `CharMenu.xml`, `MapMenu.xml`, `CameraMenu.xml`, `Windows.xml`): Actions has Emotes, Camera (1st person pulls the camera to the head, 3rd person puts it back), Stats, Control panel and Help (help window, options); Character has Skills and Perks; Map has the map. A press elsewhere closes it. Entries for things this client lacks (pets, tradeskill kit, faction, research, planet map, the Team menu, the 3rd person trail/lock/rubber modes) are left out. Not done yet: the Wear and Shop icons, the AOU skins' own art, and the original's default window positions and chat look.

**Logging out.** `/camp` (like X) sits the character and the server logs it out after about 30 s by closing the connection. The game window now notices the connection thread ending, closes with exit code 42 (`launcher::LOGGED_OUT`), and the launcher that started it starts again as a new process with `MAO_RELOGIN=1` (account, password and server in the environment), logs in at once and shows the character screen. Not tried against the live server yet.

**Plan for the original look.** The ordered handoff plan for making the GUI look, feel and operate like the original (skin directory, window chrome, default positions, the rest of the control center, chat) is `docs/ORIGINAL_UI_PLAN.md`.

## Skins from a folder (`skin.rs`)

The skin is now a stack of folders searched in order: the `gui_dir` setting (`[interface] gui_dir =`, the options window's "Skin folder" field, or `MAO_GUI_DIR`), the folder of the `theme` setting, then the game's own `cd_image/gui/Default`. Each folder's `Graphics.uvgi` is read (a text index of `NAME offset length` lines; the game's has no title line, the AO-Skintool ones do) and a sprite is decoded from `Graphics.uvga` the first time it is asked for, from the first folder that has it (pure green is the transparent key). So an AOU skin, which ships 287 of the 789 sprites, shows its own wings, compass and borders and takes the bars and icons it lacks from the game. The control center draws from it (`crate::skin::skin(ctx)`), and the target buttons are placed by the real strip width and painted after the wings (the AOU wings are 440 and 320 wide against the game's 189). `TextColors.xml` of the theme's folder (or the game's) now colours the chat lines by the original's names (`ct_cmd_feedback`, `ct_itell`, `ctch_misc`, `ct_otell`, `CCMeHitOtherColor`; the built-in theme keeps the old colours). `MAO_THEME=<theme>` forces a theme for previews. Checked offscreen with the game's skin and `MAO_GUI_DIR=.../Gui/AOU_CYT`; not yet in the live game. Not done: the bitmap fonts (`GFX_FONT_CHAT9` etc. are 256-wide RGB PNG strips; the glyph table is not understood yet), the mouse pointers, and the AOU skins' own bar art (`GFX_GUI_ACTIONVIEW_*BAR*`, curved 43x161) and bar positions (`CC*BarConfig` in Prefs.xml): with an AOU skin the game's straight bars stand at the game's offsets inside the AOU wing.

## Windows in the original's chrome (`chrome.rs`)

Every window of the interface (17: inventory, stats, skills, NCU, nano programs, perks, friends, missions, mail and message, shop, loot, help, actions, Whom-Pah, options, chat) is made by `chrome::window(ctx, palette, name, title, &mut open, |w| w.<size/place>, |ui| ...)` instead of `egui::Window` directly. With the skin's pictures the window has no egui title bar: it is drawn with the original's frame pieces (`GFX_GUI_WINDOW3_BORDER_*`: thin top and sides, rounded bottom corners; greyscale masks tinted with the theme's border colour, as the original tints them from `GUIColors.xml`), a translucent panel fill, the title inside the frame at the top left, and the original's pin and close buttons (`GFX_GUI_WINDOW_PIN`, `GFX_GUI_WINDOW_CLOSE_X`, with their hover and pressed pictures) at the top right. Dragging anywhere on the window moves it (egui 0.37's own fallback for a window without a title bar); the frame is drawn under the content from the window's rectangle of the previous frame. The pin was read as keeping a window open on Escape until 2026-11-18; it is the window's fade switch, as the original's ("Windows fade, the pin, and the \"?\" button"). Without the pictures the plain egui window is used as before. The windows' ids are unchanged, so `layout.ini` positions carry over. Checked offscreen (stats, chat, inventory, options; both themes); dragging and the pin not yet tried live. Not done: the launcher's windows, the original's titles where ours differ (Mission, Wear), tabs/buttons in the original's art, the question-mark button.

**Scroll bars.** Every scrolling list of the windows (the 17 `egui::ScrollArea`s) goes through `chrome::scroll(ui, palette, area, |ui| ...)`, which hides egui's bar and draws the one the original's scroll views make (`ScrollView_c::_UpdateScrollBars`, GUI.dll 0x10142b68: a `ScrollBar_c` with `GFX_GUI_SIMPLE_SB_UP`, `_DOWN` and `_SLIDER_VERT`, no track, a point right of the content, as wide as its arrows), tinted with the border colour. It behaves as `ScrollBar_c` does: the knob's length is the visible share of the length between the arrows, at least 8 points (`_CalcKnobSizeAndPos` 0x10141ab9, `chrome::knob`); an arrow scrolls 5 points and repeats after half a second every 0.08 s while held (`MouseDown` 0x101411fe, `SlotRepeatTimer` 0x10141d21); a click above or below the knob scrolls 20 points (the steps are 0.05 and 0.2 of the 0..100 range the scroll view makes the bar with); the knob drags (`MouseMove` 0x101416a0). The wheel stays egui's. The bar shows only when the content does not fit. Checked offscreen (Skills, both skins); not yet tried live. The sprite ids are `DynamicID_t` numbers: the AFCM drop-in's name table (`dropin/afcm/src/table.rs`) in order, so 0x177 `GFX_GUI_SIMPLE_SB_DOWN`, 0x17b `_SLIDER_VERT`, 0x17c `_UP` (the default `ScrollBar_c` art, 0x38–0x3b `GFX_GUI_BUTTON_ARROW*` and 0x174/0x175 `GFX_GUI_SCROLLBAR_EMPTY/FILLED`, is for bars made on their own).

**Tabs.** The tabbed windows (Actions, the inventory's wear tabs, chat) use `chrome::tab_view(ui, palette, names, &mut selected, |ui, open| ...)`, the original's `TabView` (GUI.dll 0x10146767): its strip (`TopBorderView_c`: `GFX_GUI_TAB_BORDER_TL`, `_TOP`, `_TR` along the strip's foot) holds the tabs, each a `BorderView_c` three-slice (`Tab`, FUN_1014694d): `GFX_GUI_TAB_INACTIVE_*` at 0.6 of the alpha with black text, the open one `GFX_GUI_TAB_ACTIVE_*` drawn in front with white text (FUN_101462d8); a tab is its text's width plus a point plus its 5 and 16 point margins (FUN_101463a4), the first 2 points in and each overlapping the last by 6 (FUN_10147438, `chrome::tab_places`; narrowed in proportion when they do not fit, which is near `View::SpaceOut` but not checked against it, and the text is cut at the tab's right margin). The body has `GFX_GUI_TAB_BORDER_LEFT`, `_RIGHT`, `_BL`, `_BOTTOM`, `_BR` round it and `GFX_GUI_TAB_BACKGROUND` inside (`SetRenderFlags` 0x101464c4, `LayoutBorders` 0x10145dad); all tinted with the border colour. A tab opens on the press. The text is in the original's font 5, Verdana 13 (`FontSystem_t`'s table at GUI.dll 0x10272df0: fonts 0 to 3 are the bitmap fonts, 4 to 11 Verdana at 12 to 24 points), which `font::verdana` reads from the system's fonts folder into egui (egui's own font until then). Checked offscreen (Actions, chat, inventory); not yet tried live. The inventory's wear column is narrower than its four tabs, so they are narrowed and their names cut. The sprite ids come from the AFCM name table in order, one entry of which has lower-case letters (`GFX_GUI_StatEnum_eND_BORDER`, 0x18a): count it, or the ids after it come out one short.

**Buttons.** The windows' buttons (friends, loot, mail, missions, NCU, shop, Whom-Pah, options) are `chrome::button(ui, palette, text, enabled)`, the original's `Button_c` (GUI.dll 0x10129021): at rest a `BorderView_c` of `GFX_GUI_BORDER01_*` (corners at their size, sides stretched, the background inside; `skin::nine_slice`) in the theme's `Default` colour with the label in Verdana 13 in the same colour, centred (`Initialize` 0x10128e46, `SetLabel` 0x10128ba3, `LayoutLabel` 0x10127d88); pressed, `GFX_GUI_BORDER04_*` and the label in `Selected`; under the pointer, the `GFX_GUI_BORDER03_*` frame in `Hover` laid over it; disabled, all of it times 0x909090 (`StateChanged` 0x101287ea). Its size is the label plus the side pieces' widths and 10 across, and the label plus the top and bottom pieces' heights and 4 down (`CalculatePreferredSize` 0x10128396, `chrome::button_size`), so about 42 points tall: the windows' button rows are taller than egui's were. The colour codes are `GUIColors.xml`'s names (`LoadGUIColors` 0x1006c13d: 1 `Default`, 2 `Selected`, 3 `Hover`, 4 `Text`, 5 `TextSelected`, 6 `TextHover`); the palette now keeps `Default` and `Selected` as they are (`Palette::default`, `selected`). Left as egui's: the buttons in right-click menus (not `Button_c`s in the original), the launcher's, the control center's. Checked offscreen (options); not yet tried live.

**Dark themes.** The controls' colour code 1 (`Default`) is `GUIColors.xml`'s `Default` itself on a light theme (the game's own), but on a dark one (the AOU Translucent's `Default` is 0x051a2a, near black) it is the palette's border colour, `Selected`, as `Palette::from_gui_colors` already borders the windows there (`chrome::code1`); and an inactive tab's black text is the theme's `Text` there. Otherwise the buttons, check boxes, insets and inactive tabs came out as dark boxes on the dark panels. A choice of the client's, not checked against the original running that skin.

**Check boxes.** The options window's check boxes are `chrome::checkbox(ui, palette, &mut on, text)`, the original's `CheckBox_c` (`Initialize`, GUI.dll 0x10129eb6): a toggle `Button_c` whose only picture is `GFX_GUI_CHECKBOX_UNCHECKED` (11 by 11) in `Default` when off and `GFX_GUI_CHECKBOX_CHECKED` in `Selected` when on, nothing over it under the pointer, and its label 10 points to the right in Verdana 13 in `Default`. Checked offscreen.

**Text fields.** The windows' text fields (find boxes, friends, mail, the skin folder) are `chrome::text_edit(ui, palette, egui::TextEdit::...)`, the original's `TextInputView_c` (`Initialize`, GUI.dll 0x10148aff): the text inside a `BorderView_c` with the class's own pieces (`GFX_GUI_INSET_*`, which `BorderView_c::BorderView_c` 0x10126da4 sets: thin sides, 7-point corners, no background), 3 points in from the sides and 1 from the top and bottom. The pieces are in `Default`: the field sets no colour and takes its window's (taken to be `Default`, not checked). The chat input and the launcher's fields are left as they are (the chat's is the control center's input bar in the original). Checked offscreen (options).

**Target frame.** With the skin, the target at the top of the screen is the original's frame (`GFX_GUI_CC_TARGET_FRAME_*` corners under the compass): the name with the level when known, and the health bar from `GFX_GUI_TARGET_HB_*` (end caps, groove, the sliding fill clipped to the health left; red for an enemy, green for a friend, blue for a player, by the same rule as the name plates), with `hp / max` and the distance written on it. The network thread sends it as `Update::target_bar` (`ui::TargetBar`) beside the old text; without the skin the plain box stays. Checked offscreen only.

**Window places from the original's own files** (2026-11-09, `aoprefs.rs`). The shipped `CharPrefs.xml` has no frames (the original's starting places are code defaults), but the player's own layout is on disk under `%LOCALAPPDATA%/Funcom/Anarchy Online/<hash>/Anarchy Online/Prefs/`: the display size in the top `Prefs.xml` (`DisplayWidth`/`DisplayHeight`), and per character under `derived/Char<id>/` the `WindowFrame` of each `<X>Config` archive in its `Prefs.xml` (skills, options, perks, NPC chat, research, pets, LFT, market…), the `DockAreas/DockArea<n>.xml` files — a `DockTabbedWindow` each with its frame and `docked_view_identities` (`inventory_window`, `mission_window`, `info_window`, backpacks) — and the chat windows' `Chat/Windows/Window<n>/Config.xml`. The wear, NCU, nano program, team and faction windows have no frame: their `DockableViewDockName` is `RollupArea`, the column down the right edge (below). A window with no place in `layout.ini` starts at the original's frame for the character (its own folder, else the player's most recently written character's, since that is the layout they are used to), scaled from the original's display to this window's size in points, as position and size; `MAO_AO_PREFS=<Prefs folder>` names the folder, `MAO_TRACE_LAYOUT=1` prints the frames. The lookup waits for the screen's real size (egui's first frames report 5000- and 10000-point squares) and for the character (set at `net::live`'s start); a window drawn before either is moved to its frame once both are known (`layout::relocate`). Seen live: Humility, who has no folder of her own, takes the account's newest character's layout — skills top left, missions at the left edge, chat bottom centre, inventory bottom right, the rollup column at the right — the original's own picture. `AutoCloseWindows` is `false` in the shipped prefs, so the original does not close the other windows when a menu opens one by default; not done here either.

**The rollup area** (`rollup.rs`; GUI.dll's `RollupController_c` in the control center's `RollupControllerDock`, its `RollupPage_c`s): the column of pages down the right edge of the screen that the original docks the wear, NCU, nano programs, team and faction windows into by default. A window on the list is drawn by `chrome::window` as a page — a header strip with the title and the skin's `GFX_GUI_ROLLUP_COLLAPSE/EXPAND` button (a page rolls up to its header), the undock button (`GFX_GUI_WINDOW_MAXIMIZE`) and the close button, then the window's own body in the original's frame pieces, sized as a free window sizes it (an egui window without a title bar at a fixed place) — so each window's code is the same docked or free. The pages stack from the top in the list's order, each as tall as its body; a free dockable window (wear, NCU, programs, team, faction, friends, pets, stats, missions) has a third title-bar button that docks it at the end of the column. The list and each page's rolled state live in `rollup.ini` beside `mao.ini`; without the file the original's five are docked. The nano programs window lays its school buttons two a row and its grid four columns when docked (the original's `SchoolSelector_c` rows as wide as the window, FUN_100d6051). The corner map keeps left of the column. Not done: dragging a page to reorder it or out of the column (the undock button stands in), the original's fade (`cc_rollup_controller_fade_level`). Seen live: wear, NCU and programs stacked at the right edge with the original's chrome.

**The corner map** is round, draggable and zoomable as before, and now sizeable: a handle at its lower right (`GFX_GUI_WINDOW_CORNER_DRAG`) or Ctrl and the wheel change its diameter (120 to 600 points), kept as `Size` in `automap.ini`. It stays the client's own circular map, as the user chose, in place of the original's square PF Map window.

**Chat tabs in the original's art.** The chat window's tabs (All, Tells, Channels, System, Combat) are drawn with the skin's `GFX_GUI_TAB_ACTIVE_*` / `GFX_GUI_TAB_INACTIVE_*` pieces (`Skin::three_slice`: left cap, stretched middle, right cap; greyscale masks tinted with the theme's border colour, the inactive ones dimmed, the hovered one in the hover colour; the text in the theme's text colours). The input line stays egui's: the skin's `CONTROLCENTER_INPUT_BAR` is the control center's angular bar and looked foreign in the window. Checked offscreen in both themes.

**The original's bitmap fonts (not used yet).** The glyph tables are `cd_image/textures/fonts/Font{Chat9,GameShell12,Tooltip9}.fnt` and `NPE_cc_font.fnt` (5108 bytes each): two `u32` words (256, 256: the strip's nominal size), then one `(x0, y0, x1, y1)` entry of four `u32` per character code from 0, 318 entries (the first 32 unused), pixel rectangles in the strip `GFX_FONT_<NAME>` of the skin atlas (256 wide; `CHAT9` 256x55, `GAMESHELL12` 256x98, RGB with pure green behind white glyphs); three trailing words (line metrics, not understood). Checked: `FontGameShell12` entry 65 = (70, 14, 81, 26) is the `A` of the strip's second row, entry 33 = (97, 0, 99, 12) its `!`. egui draws only vector fonts, so using them means a glyph painter (a mesh of quads per line) for the chat and tooltips: left for the font step of `docs/ORIGINAL_UI_PLAN.md`.

## The original's bitmap fonts in use (`font.rs`)

The chat lines are now drawn in the original's own chat font (`GFX_FONT_CHAT9` + `FontChat9.fnt`, see above): `font::BitmapFont` reads the table and the strip (white glyphs with their brightness as alpha, so the line's colour tints them; sampled nearest-neighbour), lays a line out as one mesh of textured quads (a glyph's offset inside its strip row is kept, so short glyphs sit where the original puts them), and wraps at spaces by summing glyph widths (`wrap`, tested). `[interface] original_font = on` (default) / the options checkbox "The original's chat font" turns it off for egui's vector font. The pictures scale with the interface size like everything else. Checked offscreen: the sample chat lines read exactly as the original's.

The interface font too: window titles (`chrome.rs`), the target frame's name and the on-screen log use `GFX_FONT_GAMESHELL12` / `FontGameShell12.fnt` (`font::shell`, `font::text`) and the chat font, when the original's fonts are on. Checked offscreen.

**Shortcut bar in the original's slot art.** (The smart hotbars'. Since 2026-11-18 the original's own shortcut bars are the
default and these a setting: "The original's shortcut bars".) With the skin the ten slots are 38 px `GFX_GUI_MULTILISTVIEW_SLOT_32_CLOSED` frames (the `OPEN` one for an empty slot) and the bar is stepped through with the control center's big arrows (`CONTROLCENTER_BIGARROW_*`, hover and pressed pictures) around the bar's number; Alt+1-6 still pick a bar directly. Everything else of the smart hotbar (queued presses, cooldown sweeps, drag-and-drop, the menu) is unchanged; without the skin the old 46 px look stays.

**Camera modes.** The Camera sub-menu offers the original's third-person modes besides 1st/3rd person: locked (the camera stays behind the character at all times), rubber (swings round behind it at 6/s while moving, the old behaviour) and trailing (1.5/s). `[interface] camera_mode = lock | rubber | trail` (default rubber); the mouse still steers in every mode. Not tried live.

**The original's mouse pointers** (`pointer.rs`). With the skin, the window's pointer is the skin's `GFX_GUI_POINTER_*` picture of the pointer egui asks for each frame: the arrow, the hand over links and buttons, the I-beam in text, the drag arrows at a window's edges and corners, the hourglass. Each is made into a winit custom cursor once (hot spots: the arrow's tip, the hand's finger, the centres of the drag arrows) and set whenever egui's wanted pointer changes, after egui-winit set the system's. The pointers while steering the camera (hidden) are unchanged. Not checked yet: a picture cannot show it; look at the arrow in the live client.

**Rollup.** The original's rollup button (`GFX_GUI_ROLLUP_COLLAPSE/EXPAND_STATE1-3`, at the left end of the right menu strip) folds the wings and their bars away and back, leaving the menu strips, the target buttons and the compass; not remembered between sessions yet. Checked offscreen.

## Login and character selection as the original's

From the user's screenshots and `Views/LoginWindow.xml`, `CharacterSelectionWindow.xml`, `CharacterSelectionItem.xml`. **Login:** a small centred dialog in the chrome: `Username:` with the list of accounts used before (`account.ini` keeps them after the account and the last character; `parse_account`/`render_account`, tested) and a Remove button, `Password:`, Login and Quit (the Server row shows only when the server is not Rubi-Ka). **Characters:** a 276 px panel at the right in the skin's tab border (`GFX_GUI_TAB_BORDER_*`, 90% opaque): the picked character's name in red (`0xEE4444`, the original's pressed colour) over `Level / Gender / Breed / Profession / Location` rows, the others in the border colour (white under the pointer) with `Level N breed profession`; a disabled New Character button and the count under the list; the picked character turns slowly in the middle; a bar across the bottom with Back at the left and a disabled Delete at the right, Play centred above it. Names in the GameShell font, rows in the chat font. The default theme is now `original` (the game's own `GUIColors.xml`; the gold `built-in` look stays selectable) and the user's `mao.ini` was switched from the never-chosen `built-in` to it. Previews: `MAO_LAUNCHER_DEMO=1 MAO_LAUNCHER_SHOT=out.png` (characters), `MAO_LAUNCHER_DEMO=form` (the dialog). Not done: the original's 3D hall behind the screens (playfield 896 "Character Creation Lab" is not it: an outdoor lab; the hall's resource is still to be found), making and deleting characters, the `x/18 slots available` figure (the account's slot count is not in the character list).

## Running buffs pulse like the original's

A recording of the original client (a level-25 Martial Artist standing still, 34 s) showed what a running buff's effect really is: nothing most of the time, then a short pulse. A yellow ring of sparks forms over the head in about 0.3 s, spins there for a second and fades (seen at 0, 10, 15, 21 and 30 s), and a cyan ring forms at the ankles and climbs to the knees over a second and a half (at 5 and 27 s). The continuous double spiral this client drew was wrong. Decoded from the templates for it (`gfx.rs`): a kind-1002 helix's word 8 is the pulse's life (1.5 s in all 20), word 3 the ring's place on the body as a fraction below the top (0, -0.05, -0.1 the head, -0.13, -0.25 the chest, -0.5 the waist), and the kind-1001 aura's word 20 (a float: 4 in 36 auras, 1 in 20, 2 in one) its repeat interval. `effects::running_particles` now draws a running effect as pulses: every `2 x interval` seconds (the doubling is a calibration against the recording's 5-10 s gaps; the raw 4 s was too often) a ring of the helix's `count` sparks with short trails at `1.9 m x (1 + word 3)`, lit one after another in the first quarter of the pulse, spinning at the helix's rate, climbing 0.2 m and fading in the last third; the buffs of one character are staggered by 3.1 s each. The kind-3003 glow (the common cyan running effect) is a 16-spark ring rising from the ankles to the knees every 8 s (no interval is decoded for it). Sparks are drawn small (half the template's size) with alpha 0.5 so the additive pass does not whiten them. Hits and tracers keep their continuous spirals for their short lives. Tested (`a_running_effect_pulses_and_rests`); previews: `MAO_GFX=<game>/Setupf/gfxtweak.bin MAO_AURA=1010 MAO_AURA_AT=0,0,-0.6 MAO_AURA_TIME=0.9 mao-client <game> player 1 2 male_idle-stand_01_01.ani --image out.png` (the yellow halo; `MAO_AURA=12350` a glow). Not checked live yet; the kind-0 path (21 pairs) and words 5, 24, 25 of the helix are still unread.

## Running buffs: the original's own rules, from Gamecode.dll

The pulse model above was a guess from one recording; the user rightly asked for the original's data. It is in the decompiled Gamecode.dll (`AnarchyReimagined/exports_gamecode`): `ReloadGfxTemplates` (100ce664) reads `gfxtweak.bin` as `CMSBlock`s, and `_EffectHandler_t::CreateGfxControl` (100d0656) builds a class per kind: **1001 → `_GfxControlBPHFSM_t`** (ctor 100d4226, fields 100d3f12, Process 100d4066), **1002 → `_GfxControlBuffPlaceHolder_t`** (ctor 100d5d13, fields 100d54ca, 100d5834), 1003 → the trail (100d67a4), 1005 → the sparkle emitter (100dd9c0), **3003 → `_GfxControlShield_t`** (100ee24e, 100edf78). What they do:

- **1001, the schedule** (`pulses.rs`): each buff has a countdown (starts at 0); when it reaches 0 the buff looks at a gate shared by every buff of the character (the FSMs register by the character's identity in a map, 100d4623/100d455e): if the character showed a pulse in the last **5.0 s** (`_DAT_10161238`, a double) the buff waits `rand() / 16384` = 0-2 s and tries again; otherwise it plays both placeholders (`CreateEffect2` of words 10 and 11 with the start colour words 12-15 and stop colour 16-19), sets its countdown to **word 20** (4 s for most, 1 or 2 s for some) and, when word 21 is not -1, counts a play off. So one buff pulses at once and then every ~5-7 s; several take turns, never closer than 5 s. (My earlier `2 x interval` guess is gone.)
- **1002, the ring**: a point that rides a circle: radius **word 21** (0.15 m), start angle word 22, **word 23 rad/s** (±12.6 = 2 turns a second), for **word 8 = 1.5 s**, placed **word 3** metres from attach point **word 7** (1006) of the character with the rotation words 4-6 (flags word 0). For a `SimpleChar` the constructor reads the stats breed (4), sex (59), fatness (47) and scale (360, /100 → the ring's scale) and takes `(offset, radius)` from the kind-0 table **word 26** at `breed*12-12 + (female ? 6 : 0) + fatness*2` (`Helix::fit`; a normal Solitus male gets (-0.18, 0.08)). The point carries the sparkle emitter word 10 (kind 1005: 20 sparks of 0.01-0.02 m living 0.5 s) and leaves the trail word 11 (kind 1003: material 8 light_halo2, 60 a second, 0.05 m, living 1.5 s): the trail is the ring one sees, which is why it forms over the first lap. `effects::pulse_particles` draws exactly that (attach point 1006 is placed 2.1 m above the feet, a guess that puts the ring just over the head as the recording shows; the bone itself is not resolved).
- **3003 is a shield, not a glow**: `_GfxControlShield_t` draws the character's own CAT mesh again (`GfxVisualShield`) with the material of word 9 and parameters words 18-25, with colour/priority flags in word 0 — the cyan band climbing the legs in the recording is that material animating over the body. It is still stood in for by a ring climbing the legs every 20 s; drawing it properly means a second pass of the character mesh with an additive band shader, left to do.

Plumbing: `Update::auras` now carries the character's id, its body (breed, sex, fatness) and each nano's id with its visual; the viewer's `pulses::Pulses` runs the schedule per character and buff (tests: one buff every 5-7 s, two buffs taking turns never within 5 s, limited plays, characters independent). The preview `MAO_AURA=1010 MAO_AURA_TIME=1.0 MAO_AURA_AT=0.5,0,-0.55` shows one pulse beside the figure. Not checked live yet.

**Where a ring sits: the attach points.** The pulses rang the right way but in the wrong places because every effect was hung on one guessed point. The placeholder's word 7 names a body node, and the original's tables (Gamecode.dll `FUN_10105e6d`: `.rdata` 0x102c63a8 and 0x102c63f8) give them: 1000 `Bip01 Pelvis_ac`, 1001-1004 `Spine..Spine3`, 1005 `Neck`, 1006 `Head`, 1007/1008 left/right `UpperArm`, 1009/1010 `Forearm`, 1011/1012 `Thigh`, 1013/1014 `Calf`, 1015/1016 `Foot`, 1017/1018 `Hand`; 2000 `Attractor02_righthand`, 2001 `Attractor03_lefthand`, 2002 `Attractor01_head`, 2003 `06_back`, 2004/2005 left/right shoulder, 2006-2023 the special/attack/flare/smoke attractors; 3000 a weapon's own frame. The templates use head, neck, spine, upper arms, forearms, thighs, calves and both hand attractors. `Helix::attach` carries the id and `effects::attach_point` places the ring from a table of where each node sits on a standing body (height, side, and the bone's direction, along which the template's offset is applied: a head ring's -0.18 climbs to the crown, a calf ring's -0.25 sits at mid-shin), with the character's heading for the sides. The posed skeleton's bones (the net thread has them) would place them exactly, animation included: left to do; the hands' left/right may be mirrored.

**Limb rings show on the right side only.** The user saw the original ring one leg only, at the thigh, where the data has a helix for each thigh. The skeleton has both bones (`attractors` now lists them: `Bip01 L Thigh_ac`, `Bip01 R Thigh_ac`, ...), so the cause is the rig: the Biped's left-side bones are mirrored, and the placeholder's local offset (-0.22 m) that runs down the right thigh runs up the left one into the pelvis, where the ring is inside the body and unseen. The anchor table now sends the left limbs' offsets the other way, so their rings vanish into the body as the original's do and the visible ring is the right limb's at mid-thigh / mid-calf / the arm. Inferred from the rig and one observation; a left-handed check (an arm buff) would confirm it.

## Doors: each has its own mesh in the dynel table

The user saw doors missing in Andromeda's ICC headquarters (the client was in playfield 655, not 4582 "ICC Shuttleport", which is a dock island). The buildings, towers and signs are statels and were all there; the doors were wrong: `static_props` took the door's model from its item template (`0xa25d` "Door" → `door_omni_med_blue2`, a small white single door), but a dynel record carries **its own stats after the template**: the words past offset 60 are big-endian, `0x66, 0x67, 0x3f1`, a count, then blocks; a block headed `0x0f 0x18 0x7e2` holds `(stat, value)` pairs of the object itself — **12 = its mesh** (the pharmacy door `0x48ae6`, the ICC HQ entrance `0x3eff3`, Old Athen's doors `0xa685`/`0xa686`: the dark slatted double doors of the original) and 21 — followed by a counted name ("Basic Armor Shop", "ICC Assembly Hall": the door's destination); a block headed `2 0x10 0x7e2` is an action with a spell (`0xcf5a`, the teleport through the door). `dynels::tail_stats` reads them (tested on the pharmacy door's record) and `static_props` prefers the record's mesh. The live `DoorFullUpdate` (kind `365a5071`, 171 bytes) carries no mesh stat, only stats 0, 23 (`StaticInstance`), 701-703, 412, 252, 192-195, 259, 50 (= `0x3f1`), so the table is the only source. New dev tools: `template <game> <hex id>...` (an item template's stats), `dynels <game> <pf> <kind>` now prints the record tail. **Open: the closed pose.** With the record's position and quaternion the ICC armor-shop door stands beside its opening turned 90° (0°, ±90°, 180° of extra yaw — `MAO_PROP_YAW=<deg>` on the offline view — all leave it outside the opening), so the position is the hinge jamb and the original computes the closed orientation (the room-link door code, `n3Room_t::GetDoorPosRot`, is the indoor case; the outdoor dynel's rule is not found yet). Check Old Athen's doors, which stand in plain walls, to see whether the offset is ICC-only.

**The door's pose, so far.** The ICC door mesh (`0x48ae6`) is a sliding double door: two leaves 2.55 m wide each, with a `door` animation of 2.67 s whose keys `mao-db dump-mesh` now prints as floats (a translation key is x y z then the time; a rotation key the quaternion then the time). Its saved `anim_matrix` equals the first keys (the leaf at x = -1.271; the open keys slide it to -2.98), so the loader's `anim_matrix * TRS` composition already gives the closed door and a `rest_pose` from the keys (left in `model.rs`, unused) is not needed; using it for every animated node tilted whole buildings, so it stays off. The leaves stand 3.81 m from the mesh origin along its facing, which is why the door's dynel point sits a few metres from the opening: with the facing wrong the door swings 3.8 m to the side of its opening, which is what is seen. The remaining question is the facing convention of a dynel's quaternion for these meshes; the teleport works, nothing animates (the user's observation), and no extra yaw of 0, ±90 or 180 degrees put the leaf across the opening in the offline view.

**Doors in their doorways: the root frame's transform is a pivot, not a placement.** The offset was in the mesh loader, not the table: a door mesh's root `RRefFrame_t` carries `local_pos` (0, 0.96, 3.81), the leaf frame carries the negation, and the leaf's animation keys (and saved `anim_matrix`) add the 3.81 back, so applying the root's transform hung the leaves 3.8 m from the door's point along its facing (the point is the doorway; with the facing read as it is, the leaves swung to the side of the opening). `model::walk` now leaves the root frame's own transform out (`MAO_ROOT_LOCAL=1` applies it, for comparison): Andromeda's shop doors and Old Athen's stand in their doorways, the wide view of ICC is pixel-identical (the statels' roots are identities), the characters are untouched (a different loader). The earlier guesses (an extra turn for doors, the animation's first keys) are gone. Checked offline at three places; not yet live.

**Correction: only the root frame's position is a pivot.** Leaving the whole root transform out (the previous paragraph) turned signs and heads upside down: their roots carry a rotation the original applies. The rule that satisfies the door, the signs and the heads: a mesh **placed** in the world (`model::build_placed`: statels, doors, machines) keeps its root frame's rotation and scale but not its position; a mesh **attached** to a body (`model::build`: heads, worn gear, through `load_model_attached`) keeps the whole root transform, since its position is the neck's. Checked offline: the ICC door in its doorway, the Transportation sign upright, the head on the neck. The door records' stat block header is `0x0f 0x18 <any>` (0x7e2 on the shop doors, 0xbd3 on the ICC HQ entrance). A door record without a mesh of its own (the Unicorn lander, the Whom-Pah to Jobe, the Condemned Subway entrance: their visuals are statels) now draws nothing instead of the template's white door, which was the wrong door the user saw at Phasefront.

## Texture alpha that is light, not a mask

The user saw Old Athen's shops with their walls missing and asked about the doors. The shop building (`hq_omnimall`, 17 parts in one frame) decoded fine (`MAO_PART_BOUNDS=1` prints each part's extent, index range and material as read: a 10 x 15 x 46 box) but four of its materials ("shadowtechdetail", "lightedge", "towerside", the window strip) use textures whose alpha channel is mostly under 128, and the renderer cut every texel under 0.5 alpha, leaving streaks. Those materials carry texture stage states in their `RDeltaState` (`tstm_count` says how many belong to each stage; `tst_type`/`tst_value` are Direct3D 7 stage states): stage 0 `COLOROP ADD (7), COLORARG1 TEXTURE|ALPHAREPLICATE (0x22), COLORARG2 DIFFUSE (0), ALPHAOP DISABLE (1)`, stage 1 `MODULATE (4), TEXTURE (2), CURRENT (1)`, so the colour is `tex.rgb * (diffuse + tex.a)`: the alpha is baked light added to the lighting, and with `ALPHAOP` disabled the pixel keeps the vertex's alpha of 1, so no test or blend cuts it. `model::material` sets `Material::alpha_lights` (and `ignore_alpha`) on that layout and the shader adds the alpha to the lighting term before the colour (flags.y = 2). `mao-db survey-states` tallies the states over all 11228 meshes: 33834 of 37516 material states have none; the alpha-light layout appears 3573 times, mostly with an explicit `ALPHABLENDENABLE (27) = 0`; alpha test is switched on explicitly (`15 = 1, 24 = 128, 25 = 5`: ref 128, GREATEREQUAL) in 1506, blending in 3351 (effects, windows). Not changed: a material with no states still cuts under 0.5 alpha, which is what the trees (no states at all) need. `mao-db names <text>` lists the meshes of the instance index by file name.

Old Athen's shop doors (mesh `0xda4f`: two sliding leaves and a rising top panel, one `RTriMesh_t` each with the closed pose in `anim_matrix`) stand centred at the back of the shop's recessed entrance with the record's rotation as it is; conjugating it (`MAO_DYNEL_CONJ=1` on the offline view) shows their back and breaks the ICC doors, so the sense is right. The tall column in front of the entrance, right of centre, is the building's own `pillar1` part (`MAO_PART_SKIP=<material name>` leaves parts out of a view, which is how it was found), as in the original.

**Dynel meshes: none of the root transform.** The user's Old Athen shop door still stood broken (one leaf across the opening). `RRefFrame_t::UpdateWorldMatrix` (the randy31 port, `refframe.rs`) is `anim_matrix * (rotation * scale, position) * parent`, the same composition as `model::local_matrix`, so the frame math was not the problem: it is what the root frame holds. A door mesh's root carries an exporter transform (Old Athen's `0xda4f`: position (-0.12, 3.62, 0.41) and a quarter turn; the ICC doors a position only) and each animated child carries that root's inverse in its local transform and its own pose in `anim_matrix`, so the child's place is `anim_matrix` in mesh space whatever the root holds. The game sets a dynel's root frame to the object's place, replacing the exporter's root entirely; a statel's mesh keeps its root (the signs' rotation). So three rules now: `build` (attached: the whole root), `build_placed` (statels: rotation and scale, not position), `build_dynel` (doors, machines, terminals: none; `load_model_dynel`, used by `add_static_props` and the live props). Checked offline: the Old Athen door closed in its recess, the ICC door in its opening. The client logs every playfield message and any server move of more than 3 m of the own character (`the server moved us`), for the warps the user saw while trying the doors.

## Doors and zoning, from the original's code

Read from the Gamecode.dll decompile and the DisplaySystem port after the user asked why the door work looked like guessing (it was: the earlier paragraphs' rules were fitted to renders).

**Placement.** `VisualMesh_t::SetMesh` ends in `attach` (`dropin/displaysystem/src/asyncmesh.rs`, FUN_1006d7b9): the loaded mesh's frame is added under the visual's own frame (+8, the one the game positions) **at the origin, unrotated** (`set_position(frame, ORIGIN)`, `set_rotation(frame, NO_ROTATION)`), so the exporter's root position and rotation are discarded and only its scale and the children's transforms apply: `model::build_dynel`. `RRefFrame_t::UpdateWorldMatrix` (randy31's `refframe.rs`) is `anim_matrix * (rotation * scale, position) * parent`, which `model::local_matrix` matches.

**Zoning is client-initiated.** A collision callback (FUN_100a944a) fires when the client character touches a `Door_t`; with the gate FUN_10058ae4 (not dead, not already in it, stat 0xc2 set) it calls `n3EngineClientAnarchy_t::StartTeleportTry` (0x10019002), which sends `N3Msg_MovementChanged(0x16)` — a `CharDCMoveIIR_t`, the ordinary position message with action 0x16, position and rotation — and makes a `TeleportTrier_t` that gives up after `_DAT_10158500` = 30 s with `Feedback_AreaChangeNotInitiated`; `StartTeleportConfirmed` (the playfield message) ends it. The same try starts from `n3Zone_t::IsPosInTeleportal` (FUN_10058b35: a zone's teleportal, the Whom-Pahs and zone borders). So the server zones on a 0x16 report from inside the door; our client had made doors passable and never sent one. Now, within 2 m of a table door's position (`door_slots`), the client sends one `movement::FULL` (0x16) and waits; a new try needs the door left and touched again.

**Movement action 0x16 is the full report, not a jump.** `CheckMotionUpdate` (0x10018f20) sends it when `_DAT_101584fc` = 5.0 s pass in motion without another movement message, or after `_DAT_101584f8` = 0.25 s when the heading turned more than `_DAT_101584f0` = 0.17 rad (the constants read from Gamecode.dll's data with pefile). The captures agree: 389 of them in `capture-225436.log` against 375 of type 15, which appear as the character lands from a height. `movement::FULL` = 22 is now the periodic report (`JUMP` is the same value); 15 (`UPDATE`) is sent on landing only. 0x1e is `N3Msg_SitToggle` (the capture's 30 while standing).

## Walking and running, from the original's code

The user reported the walk and run animations broken. Read from Gamecode.dll (the function addresses are in the code comments):

- **Speeds** (`CharVehicle_t`, FUN_1006f9eb with FUN_1006f2fc): running forward is `RunSpeed / 275 + 5` units a second, within 1.5 and 13; running backwards `RunSpeed / 392.86 + 3` within 1.05 and 9.1; walking 1.5; `RunSpeed` (stat 156) falls off below 15% health. `movement_speeds` replaces the flat 7.5 "from the capture" (that was a RunSpeed-690 character): a low character now moves at the speed the server expects, which is the likeliest source of the warps the user saw (the server pulling a too-fast client back).
- **Animation rate** (FUN_1007009f): `calibration(mesh, animation) * 100 / scale(stat 360) * speed / reference`, held at 1.3 when over 1.3 and faster than 4 units a second; the references are the vehicle's +0x170: 5.0 running, 3.0 backwards, 1.5 walking. `Setupf/animcalibration.txt` lists the player bodies too (solitus male walk 0.90, run 1.15; opifex male run 1.30; nanomage female walk 0.88, run 0.90), which the client applied only to creatures before. The movement clock (`Motion::clock`) now advances at that rate; the invented `START_RATE` 2.2 and `STOP_MS` 280 are gone, the first step and the stop play at the rate like the cycle.
- **Which animation** (FUN_1006c370 on the abstract-animation table FUN_100c0720): walk 100, run 101, walk-back 136, run-back 222, walk-left 134, walk-right 135 (a sidestep plays walk-left/right in every mode), turn-left/right 196/197 standing, idle-stand 120, idle-swim 195. The client's `Action` gained `WalkBack` and `RunBack`, and the players' sidestep plays `walk-left`/`walk-right` instead of the forward walk. Backing is the back key for the own character and, for the others, a move against their facing (`World::backing`). Walk against run is the mode in the original (`WALK_MODE` 0x18 / `RUN_MODE` 0x19 of the transition table); for the others the client still tells them by speed, now at 2.2 (walking 1.5, running from 3).
- **Movement action codes** (the transition table FUN_1006cb58): 0xf is the jump's start and 0x10 its end, not 15 "update" and 22 "jump" as the capture had been read: the client now sends 15 as the jump starts, 16 on landing, 22 as the periodic report. (Other players saw this client jump on every report before.)

## The clock and the areas' environments, from the original's code

Read from Gamecode.dll and GameData.dll (addresses in the code comments):

- **The clock** (`GameTime_t`, 0x1000afdb): a day is 27 hours of 60 minutes; dawn runs from hour 7 for two hours, dusk from 21 for two; the clock advances 15 game seconds a real second (`RunFunction` 0x1000b27e). `GetDayIntensity` (0x1000af23) is 0 at night, climbs through dawn by the hour and minute, is 1 by day and falls through dusk. The server's `GameTime` message (0x5f52412e: seconds into the day, the period, the day count, the server's time; the handler 0x10039800 → `Update` 0x1000b590) sets it. `gametime.rs` is this. The server sends the message at login and at each zoning (17 bytes: a header byte, then the four words, `n3::Message::game_time`); until 2026-11-09 the client read the words from the first byte, so every login landed at midnight (the float came out as a denormal) and the live scenes were drawn in the night's light — the day's light at Borealis's fall was the tell. Before a message comes the real clock, at the game's pace, stands in (the original starts at midnight and waits).
- **Where the light and fog come from.** Each playfield's `0xf425d` object lists its areas as `AreaData_t` records (GameData `area_read`, version 4) and each carries a 26-byte `EnvironmentData_t`: bytes 0–7 are the weights of the eight weathers, 19–21 the ambient light, 22–24 the fog's colour, 25 its density (`mao_formats::areas`; `mao-db <game> areas <playfield hex>` lists them). `Environment_t` (FUN_100be0bb) gives each area a reach — the distance to the nearest other area's centre — and every frame (FUN_100beacd, FUN_100be9bb, FUN_100bf96e) weights the areas the character is within by `1 − (d / reach)²`, scales the weights down when they sum past 1, and lerps them one after another into the playfield's own table (empty here: there is no playfield-level `EnvironmentData_t` in the database). An area with no weather weight at all is left out (FUN_100bd555 gives it no weight) — which is every area of Old Athen and the Athen shires; Mort, the swamps and Tir County have them, with fog. The result goes to `AddAmbientLight` (a per-channel max with the sky's light) and, when the fog colour is not black, to `AddFogLightIntensity(min(day, 1 − cloud × 0.8), at least 0.5)` and `AddFog(colour, density)`.
- **How the fog draws** (`VisualFog_t::process`, displaysystem `fog.rs`, then `Randy_t::SetFogParameters`, randy31 `randy.rs`): a linear Direct3D fog from the near clip plane to `far − (far − near − 5) × density`, in the fog's colour times the fog light; the density itself is not used by the renderer. `Scene::area_fog` does that: the mesh and water shaders fog linearly to that end in that colour when an area's fog is on, and the sky's lower part takes the colour.
- **The scripts' light and the client's own (environment):** outdoors the effect scripts light the world, as DisplaySystem's `EnvironmentLightObject_t` (`fxvisual.rs` `environment_frame`) has it: the environment light's `AmbientLightCurrent` and `GroundLightCurrent` with `GroundLightDirection` (`Tweak_Rubi-Ka_SunLight`: curves of `GAME.DayTimeFactor`, the ground light along `GAME.Sun1Rotation`) are the scene's ambient and sun (`Scene::lighting`, `Scene::sun`), lit one-sided as a Direct3D light is (`mesh.wgsl`'s `scripted()` path, `max(n · l, 0)`): at night the ground light shines up from under the horizon and the world is left to the ambient (0.18 grey at 1 am: `shots/sky_night.png`; dusk's blue light at 21:30: `shots/sky_dusk.png`), which is the original's night. The shadow map keeps its sun a little over the horizon; the lighting takes the true one (`lit_sun`). The atmosphere's `AddFogR/G/B` is the haze colour. Without the scripts (an interior, a playfield without sky scripts) the client's own light stands in, scaled by the day's intensity and keeping a fifth at night; the terrain water's reflected sky is still the client's own, scaled the same (`DAY_SKY`, `day()`); under the scripts the sky behind their domes is their haze colour flat (seen live where the star dome left a gap at the zenith, blue by the client's gradient before); an interior ignores the clock. The weather roll (fog, mist, clouds, rain) is not ported yet: the areas are blended as in clear weather. For a picture offline, `viewer::headless` takes `MAO_DAYLIGHT=<0..1>` and `MAO_AREA_FOG=r,g,b,density`.

## The camera, from the original's code

N3.dll's `n3Camera_t` and `CameraVehicle_t` with Vehicle.dll's `Vehicle_t` (decompiled with Ghidra for this; the addresses are in `camera.rs`). The camera is a steered vehicle, not an orbit:

- **Modes** (`PreferredCameraMode`, the `3rdPersonCamera` switch): first person; the free camera (1, the default), which keeps its direction and is pulled along at the preferred distance, so it trails behind the character as it walks and stays put as it turns; the rubber camera (2), which steers to the spot behind the character; the locked camera (3), which is held there (easing out at a tenth a frame when a blocked view opens). The settings' `trail` / `rubber` / `lock` are 1 / 2 / 3; F8 toggles first person. (2026-11-21, the user: the camera should be locked by default, and went odd in a shop where locked could not be set: the preference's own default is 3, the locked camera (GUI.dll's `IndependentPrefs_t` table; the earlier note here read FUN_10021859's "a stored 0 becomes 1" as the default), now the settings' default and the camera for any unknown name; and a room's camera attractors steer the free camera only (`CameraVehicle_t::CalcSteering` 0x1001e797 asks +0x1a0, `CameraVehicleFixedThird_t::CalcSteering` 0x1001f752 never does), where the client drew every third-person camera to them.)
- **The vehicle:** mass 20; each frame the speed is the character's run speed × 6 (at most 16), the maximum force `mass × speed / 0.3`, the brake distance `0.3 × speed`; `SteeringArrive` wants `distance / brake × speed` (at most the speed) and pushes with `(wanted − velocity) × mass × 4`; the position integrates in steps of at most 0.4 s. Behind the character the preferred spot is (0, 1.5, −4.5) from the look target (the `PreferredCamPos*` / `PreferredCamDist` settings), and when the look target cannot see it a bisection finds the nearest seen point of the way.
- **Turning and zoom:** the rotate keys turn the camera round the target by 0.02 rad a frame (scaled to the original's 60 frames); the mouse look is the same turn (the client's 0.005 rad a pixel); the zoom keys and the wheel move the camera along the view at `ZoomSpeed` (10 units a second here; a wheel notch is a quarter second), the distance kept within 0.8 and 25; zooming in past 0.8 goes to first person and zooming out of first person comes back. Reset (Numpad5) puts the camera behind the character at its horizontal distance.
- **The view** is 90° across (`SetViewPlaneWindow(1.5708, aspect)`), the vertical field following the window's aspect — wider than the client's 0.9 rad vertical was.
- The free camera's obstacle sensors, its line of sight (`n3Playfield_t::LineOfSight`: the zones' surfaces and the tile-map ground tried both ways, a dungeon's rooms and their doors; the props' meshes where a playfield has no surfaces, `Collision::line`), its lift within 0.4 of the surface's closest point and its own collision (`Vehicle_t::EnsureSurfaceAlignment` after every step of its vehicle, a body without gravity) and the rooms' camera attractors are ported (`camera::Surface`, the viewer's `CameraSurface`). The look target is the body's camera attractor as `n3Camera_t` smooths it (FUN_10020bdb; `camera::LookTarget`): `Attractor31_camera`, which no player body has, so the head's (`AttractorMesh::GetName(0)`), its x and z held as first found and its y live, a fifth of the way a frame in third person, at once in first person, or held until it drifts over 0.25 and then chased a hundredth a frame with `UseNoBobCamera`; never under 0.3 above the feet. Not ported: the two-shot framing. See `ORIGINAL_AUDIT.md`. Tried live (2026-11-09, `MAO_CAMERA_SCRIPT=<seconds>:<action>[/<seconds held>],…` presses the camera keys by themselves, `screenshot` and `log` among them, for a test without a keyboard): Beetlez in Old Athens's plaza orbits at the preferred distance looking at the head (1.54 up), pitched under the character the camera is held on the ground a tenth above it, zooms out along the ground and up overhead; Poisonouss, drawn at its Scale stat beside a structure, had the camera pulled in to two units by the blocked line of sight with the target at its scaled head; a sweep past the whompah's pillars never reached them, so the sweep along a wall is unseen live. Two of the day's live sessions ended with the server closing the connection with no message before: once right after a door teleport try at (2611, 1482) in Greater Tir County, once after ten minutes with the character standing still; the cause is unknown. Captures of the original at those doors the same day (`captures/door*`, kept out of git): the border doors between Tir County and Greater Tir County at (2449, 1461), (2612, 1458) and (2880, 1330) zone inside the connection with the same 0x16 report our client sends, crossed eleven times without trouble; the Tir city door at (2207, 937) hands off by system message 0x3c to another game server, to which the original logs in straight away with the character and its session key (what `hop_direct` does — seen working live the same evening: Humility's door out of Borealis at (680, 73, 538) sent us to another server and she arrived in Stret West Bank at (1283, 0, 2893) by the Whom-Pahs and played on); the kind-14 operator frames go unanswered by the original too; and under trace our client crossed the border doors three times and stood by for ninety seconds without a close, so the drop is not reproduced.

## The map, from the original's data

The original's planet map (`cd_image/textures/PlanetMap/normal/`: an index of zoom levels, 128×128 PNG tiles stored column by column in `PlanetMapGfxNormal.bin`, and `MapCoordinates.xml` placing each playfield) is read by `mao_formats::planetmap`. A world position lands on it as GUI.dll's `PlanetMapView_c` places it (FUN_1004bedd): the playfield's `x`/`z` plus the position, less the map's origin (31022, 24880), scaled by the level's `MapRect` over the map's extent (18751 × 24115) and the playfield's scales, north up. For any playfield in that file the corner map and the Ctrl+6 overlay now show this picture (about 21 world units to a pixel, so the overlay zooms between half and eight times that and the corner map spans 600–8000 units); interiors and instances keep the client's walked map. The `PlanetMap` window of the original (its own zoom levels, centre on quest) and the Shadowlands set are not done.

## The weather, from the original's code

`mao_formats::weather` is `Environment_t`'s weather object: an R250 generator (the 250-word table from Gamecode's `.rdata`, the seed added to one word), seeds from the area's environment bytes and the day count, a 19440 s schedule in three thirds of states 120–1200 s long, each state's own 16 floats and 8 words, and FUN_100bcec6's roll: clear, fog, mist, clouds or one of four precipitations by the area's weights (bytes 0–7, doubled plus 100 when above 1), faded in over a quarter to all of the state's length, the wind, darkness, thunder and lightning values after. `environment.rs` turns the blended table into one fog as `VisualFog_t` gathers them — the weather fog (grey 0.7 × [1]), the mist (blue-grey by day, grey when dark, × 0.7/0.8/0.9, weight 0.8 × [0]), the four precipitations' fogs (orange, dark brown, dark red, green at × 0.8, weight 0.7) and the area's own fog lit by `min(day, 1 − darkness × 0.8)` — and dims the sky by `1 − darkness / 2.333` (the original's `VisualEnvFX_t` +0x1134). (Corrected 2026-11-18, see "The weather that falls": the dimming and the fog's light go by the clouds, entry 2; "mist" is the rain's or snow's own fog; the rain, the storms and the lightning are drawn, and the cloud layer always was.) Because the seeding barely changes between days, an area's weather pattern repeats daily; Mort's areas, for instance, are under some dust or fog most of the time by their weights.

## The water and the playfield's own environment, from the original's code

Gamecode's `RDBPlayfieldAnarchy_t::ReadBlob` (FUN_10124692) reads on where N3's playfield reader stops (the header, then the zones — a camera attractor count and the attractors per zone — or the rooms): a count of water polygons, each a flags word (`& 0x1f` the liquid type, the low bit a falling liquid, `>> 5` over 10 a value), its points and index triangles (a polygon with an even type and more than three points is fanned round its centre), then two words, a size and the playfield's own 26-byte `EnvironmentData_t`. `mao_formats::playfield::anarchy` reads it; the water goes through `mao_formats::liquid` to the renderer, and the environment is the base table the areas' environments blend into (Old Athen's: a warm ambient (75, 50, 25), grey fog at 0.25, and weather weights of its own) — the earlier note that there was no playfield-level environment was wrong.

**The liquid's shape** (`VisualWater_t::Create`, DisplaySystem FUN_1003a4a4 → `mao_formats::liquid`): a still polygon
(the type's low bit clear) is cut into four by its edges' middles while twice its area is over 10000; a falling one under
a tenth of a unit high is treated as still, otherwise its heights are made 0..1, snapped to 1 or −0.01, and with two
corners level it is laid as ten strips from the top to the bottom with each height bent back as `1 − (1 − y)³` (the sheet
of a fall curls over its lip). A triangle is kept only when its middle is on the playfield's 60 by 60 tiles of 100, its
corners differ and make a face, which is turned up. Each gets its type's colour (water 0xcc00186a, lava 0xffff0000, toxic
0xf727fc05, … the alpha is the translucency) and its layer: water (types 0, 1, 4, 5, 8–13), lava (2, 3), toxic (6, 7),
the star water (0xe, 0xf), the wide water (0x12, 0x13); 0x10 and 0x11 are not drawn.

**The liquid's passes** (`VisualLiquid_t`, FUN_1003f3db and `fxliquid.rs`'s `water_pass` / `toxic_pass` / `lava_pass` /
`wide_pass` → `liquid.wgsl`, after the meshes without depth writes, lit by the scripts' `WaterLightCurrent` (its
brightest channel as grey, `set_light(brightest)`) along `WaterLightDirection`, the glint by the light's colour times
`SpecularLightIntensity`): the water's `water5.png` over 40-unit tiles drifting with a flow phase (down by the frame's time
over a hundred, wrapped), lit with the vertex colour as diffuse, the scene's ambient and a white specular of 0.505 at power 10,
blended by its alpha (the material's ambient stays Randy's default white: `RViewPort_t::SetMaterial` sends only the alpha,
the emissive, the specular and the power to Direct3D, so the water material's black ambient and the wide water's (0.2, 0.3,
0.4) tint, both set on the Randy material's +0x44, never arrive; until 2026-11-09 the client took them as the ambient, which
left Borealis's pool dark blue where the original's is lilac-grey); then the same texture rippling down the slopes (the normal's horizontal part, a step of 0.003 a
frame) blended `1 − α` over `α`. The toxic liquid's `toxicwater.png` over 12-unit tiles unlit in the vertex colour, then
stepping down the slopes by their signs (0.03 a frame). The lava's water texture over 8-unit tiles in red, opaque, then
`env_lava2.png` over 75 and 250 units drifting, its alpha the product of both (the original projects it in camera space,
which comes to the same world tiling). The wide water: the water texture over 160-unit tiles, lit the same without the
specular. Two things the original adds are left out on purpose: the frame block's wave offsets,
which the scripts' `GameWaveCurve0..15` set to 0 everywhere so they never move anything, and the lava's second (rippling)
pass, whose alpha of 1 under the inverse blend contributes nothing. The textures come by name from the resource database
(`Textures::sheet`); a missing one leaves the pass white.

**The refraction pass** (`VisualLiquid_t::RenderRefraction`, FUN_1003d13a and FUN_1003b9c9; `Renderer::refraction`, pass 7 of
`liquid.wgsl`, `blit.wgsl`). The original's frame with water goes through its render targets (`DisplaySystem` FUN_100793b8):
the sky, ground, meshes and the liquid's own passes are drawn to target 1, `VisualOffscreen2Refraction_t` (render list 9)
copies that picture into target 2, and then the water's triangles are drawn again with target 2 as their texture: each
vertex's texture coordinates are its own screen place, the water layer's mirrored top to bottom and shifted by the camera's
lean (1.414 times the camera axis's up component), with a vertex alpha of `(1 − |5 (t − base)|)² (1 − |tilt|)²` about the
mirror line (`t` the height up the screen, `base` half minus half the lean; nothing under 0.2) — so a level camera sees what
stands just above the horizon mirrored in a band of the water just below it, and a camera looking down sees nothing of it.
The wide layer's copy is straight and opaque, which with the scripts' wave offsets at 0 redraws the picture as it is, so it
is left out. The client does the same with a screen copy: with water in the scene the pass up to the liquid passes resolves
into (multisampling) or draws into (without) a copy texture, a second pass carries on in the multisampled target or after
blitting the copy back, and the water layer is drawn once more sampling the copy, clamped and unfiltered, blended by that
band. `MAO_NO_REFRACTION=1` turns it off; `shots/refr_sea.png` against `shots/refr_sea_off.png` shows the band on the
Coast of Peace's sea. Not followed: the frame block's byte 0x188, which skips the pass when set (what sets it was not found).

**Under water** (Gamecode FUN_100b77cb and FUN_100b79f7, run each frame by the playfield's state; `viewer::under_water`,
`under_water_fog`, `environment::add_fog`, `Scene::under_water`). Each frame the original tests the eye a tenth lower against
the playfield's liquid (`WaterCollisionInfo_t`'s previous test data) and, with the surface above it, calls
`VisualWater_t::CameraIsUnderWater` and adds a fog by the liquid's type (`VisualFog_t::AddFog`: the water's a dark blue-green
`(0.1, 0.3, 0.25)` at 0.99, lava's red at 0.99, type 4's dark green and the toxic's bright green at 1, type 8's brown at 0.99,
type 10's the water's at 0.98; none under the star water); the liquid's frame (DisplaySystem FUN_100406d3) then sits 0.2
higher for that frame. The client does the same from `liquid::current()` at the camera's eye: the fog is gathered after the
environment's by `AddFog`'s rules and lit by the environment's fog light, and the surfaces are raised in the liquid vertex
shader (`LiquidUniform::screen.w`). `shots/under_sea.png` is the Coast of Peace's sea from under it
(`MAO_CAMERA=2600,400,4,10,1.5708,0.05`). Seen live (2026-11-09): Humility swimming in a pond of Greater Tir County at
(2825, 1648), the camera pitched down to a hundredth under the surface, the view in the water's teal fog with the swimmer
drawn through it (`shots/live_under_pond.png`); the 0.2 raise is not told apart by eye, and no waterfall was near.

**The fall effect** (`VisualWater_t::Create`'s effect 2, DisplaySystem FUN_1003b1cd; Gamecode FUN_100afc4c;
`liquid::fall_effects`, `gfx::GlobalSmoke`, `effects::global_smoke_particles`, `placed_particles`). A falling liquid triangle
(the type's low bit) over a tenth high with one high corner, but lava, starts the environment's effect 2 at one low corner with
the other as its ground; the environment's frame hands it to Gamecode's effect handler, which walks from the one towards the
other a unit at a time (`floor(length) + 1` steps) and makes gfx 0x30d6 at every step the playfield's surface does not veto
(`VetoPosition`, off the map) whose closest surface point (`CalculateClosestPoint`: the ground under the step, held 1.2 under a
liquid's surface — `vehicle::Ground::closest`) lies below it: along a foot standing in or over water. 0x30d6 is kind 3017, `_GfxControlGlobalSmoke_t` (FUN_100e0d8a; FUN_100e060f the words, FUN_100dfc38
the run): sixty-four sprites of a material on a `GfxVisualSol`, added to the scene, each born in a free slot (`word 30 +
(rand() & word 31)` a frame) and living word 26 seconds while its width, height and colour run between the template's pairs;
the mode sets the start, and 0x30d6's mode 3 spreads them up to 1.5 along the world's x (flag 2: the world's axes; the
original's `rand() / 21845 − 1.5` is one-sided) and moves them along −z at 1.35 to 1.42 a second for 1.4 s, 0.7 to 3 wide,
white at alpha 0.15 to 0.05 of material 49 — a mist along the foot — hidden past 80 units from the camera. The client finds
the feet with the playfield (`Area::falls`, `Update::falls`) and draws the effect at each from the frame the playfield came,
while the environment effects are shown; the births are modelled at sixty a second, with a hash for the random numbers, salted
by the emitter's place (every step of a fall's foot drew the same cloud a unit apart, a string of beads where the original has
a band). Not
drawn: the quads' spin, mode 5's blend under alpha, mode 1's wandering emitter (taken as mode 0). Across the playfields
the effect lands at 25 places on each coast, 437 in Adonis City, 337 in Central Elysium, 178 in 4 Holes, 135 in Belial
Forest and Clondyke, 57 on Borealis's fall (`liquid::tests::fall_effect_survey`, which stands in a liquid's surface within
half a unit of a step for the surface's closest point); `shots/fall_coast.png` is the Coast of Peace's fall at (574, 1836) with
`MAO_FALLS=5`, `shots/fall_mist.png` the effect alone on a standing player (`MAO_AURA=12502`). Seen live (2026-11-09, `shots/fall_live.png`): Humility at the foot of Borealis's fall at (476, 513), where the playfield's surfaces put the effect at 18 steps (the survey, with the liquid alone, at 57) and the mist runs along the pool's edge as the original's does (`shots/fall_day.png` the same at the game's 18:32 once the clock was read right: the sheet a streaked grey curtain, the pool dark blue where the original's is lilac-grey; `shots/fall_dawn.png` at 07:55 with the water lit by the scene's ambient: the pool (67, 66, 68) against the original's (69, 71, 79), the mist a band (193, 196, 200) against (154, 161, 169), the sheet (96, 90, 86) against the original's opaque dark grey (44, 45, 46) — ours shows the sand cliff through its 20% and takes a glint; the original's hour is not known, a low sun fits its pool). What the original's states say (Randy's `set_defaults`, FUN_10041ede): stage 0 modulates both colour and alpha by the vertex (so the water is 80% opaque, as drawn), `COLORVERTEX` on with the diffuse from the vertex colour and the ambient from a colour the water vertices lack, so the material's white; before the surface's closest point was asked, the foot a hair under the pool's surface (9.9 against 10.0) got nothing. Not seen: Old Athens's sea lies beyond the city wall, which the walk to it cannot pass; Andromeda's lake sits forty units under the platform Poisonouss stands on; Upper Scheol's nearest fall is six hundred units away behind a mountain.

**Sixteen-bit tile-maps** (`terrain::build`): the original's ground height is the stored value times the tile-map's
second float (`_GfxControl_t::GetGroundHeight` over FUN_100459f8, which gives a 16-bit map's value as it is and an 8-bit
map's top byte), so on the maps whose patches carry 16-bit heights (the three coasts at 1/512, Montroyal 1/320, Playa del
Desierto 1/1280, the Space Apartment and Sunrise Station 1/640) the float is per unit of the 16-bit value and `height_scale`
(per eight-bit step) is 256 times it. Until this was read the coasts came out flat at 0 under their sea at 8; now their
ground runs 0–128 with the sea over the seabed, the hollow pond at 14.6 and the hill lake at 62 (`shots/coast_*.png`).

## The rooms, from the original's code

A shop or a dungeon in the original is a `GNDA` tile-map (one per such playfield) and N3.dll's rooms cut from it; the walls are not in the map's images but built from them (`mao_formats::dungeon`; `mao-db <game> dungeon <playfield hex>` prints what each cell gets):

- **The floor** is the map's height surface: the sample `(i, j)` of `DHGA` is the corner `((i + 1) × scale, (j + 1) × scale)`, so a cell rests on the four samples before and at it (FUN_1001769d), and the whole floor sits one unit above the room's position (`n3Room_t` +0x38 is −1, taken off in `GetFloorHeight`). The cells are laid from the room's centre by half the width rounded down to odd (`CreateDungeonRoom`), then the room is turned by its quarter turns. The floor was a unit too low before, and half a cell off in rooms of odd width.
- **The walls** are pieces (type 0xf695d: a few textured tiles each, spanning a cell). `n3Tilemap_t`'s constructor works out, for every cell, which of its four sides face an empty cell and whether a door (`IRHA`) opens in one, turns those bits to their smallest form (which fixes the piece's turn), and looks the pattern up in the cell type's table (`RDBTilemap_t`'s `0x23c` tables, read with the tile types): a one-wall piece, a corner, a closed cell, a doorway. A cell with no empty neighbour gets nothing. Each piece is turned by its quarter turns plus a half turn, bent onto the floor's bilinear height with a cubic smoothing from the samples around (and its own height scaled by the `HCDA` codes), and its tile `k` is drawn with the cell's texture layer `k` of `XTHA` (six layers in the shops: the floor, the ceiling, the wall halves). The pieces are 8 units tall and carry a ceiling over the wall cells.
- **Water surfaces** in a room record are read (fanned or indexed like the playfield's) and placed as the room's statels are.
- **The light map.** The original welds each tile's vertices onto the earlier ones they coincide with (FUN_10001645: within a thousandth of a unit, normals close, texture coordinates a whole number of tiles apart — the tile's coordinates are shifted by that so neighbours share the vertex; the candidates are the three cells of the row before and everything laid since the cell before began), then colours the room's vertex stream from its zlib-packed light map, a 565 colour per vertex in that order, as the emissive term (`VisualRoom_t`'s state sets D3DRENDERSTATE_EMISSIVEMATERIALSOURCE to the vertex colour; white when the map is short). `dungeon::weld` / `light_colours` and `rooms::room_parts` do the same; `model::Vertex::colour` carries it to the shader (`mesh.wgsl` adds it to the lighting). The port of the weld is exact: `ao-difftest`'s `n3_weld` replays N3.dll's own function over the tiles the client lays (a dump the `rooms` tests write with `MAO_WELD_DUMP`) and gets identical vertices and indices. The maps in the files, though, hold more colours than the rooms have vertices (playfield 1180: 416 for 226): they were baked from other geometry, so the original client colours its walls from the map's start regardless, and so does this one. The floor is not in the stream (the original draws it from the height map) and keeps the scene's light. The sky still shows over a room; the original drew interiors against black.

## The characters' shadows, from the original's code

The original casts no shadows from buildings or props; each character (every CAT mesh) throws `GfxVisualSimpleShadow_c`'s blob (`blobshadow.rs`, from displaysystem's `gfxshadow.rs` and Gamecode's FUN_100ed21e / FUN_100ed439 / FUN_100ed0c0): a black disc at alpha 0x5f (0x2f in a dungeon), faded to 0.9, whose extent is the mesh's spread across the light (every fourth vertex above the ground point), projected along the light onto the ground plane, with a solid rim at `0.7 + 0.2 × h × (light · normal)` of the extent and a transparent rim at `1 − 0.4 × (light · normal)`, its points laid on the heightmap 0.02 up, its segments from the size and the distance to the camera so it is gone some 50 units out, and nothing when the light grazes the ground (`light · normal ≥ −0.1`). The characters and the machines no longer appear in the sun's shadow map; the playfield's statels still do (the client's own choice, the original had no such shadows).

## The characters' animations, from the original's code

A character's animations are now kept as `VisualCATMesh_t` keeps them (`mao_formats::animstack`, from displaysystem's `catloader.rs` / `catrun.rs` and randy31's `anim.rs`): a list by priority where each animation claims body layers (every player animation tags its bone tracks 1 or 2, the root both), plays the tracks on the layers not claimed above it, fades in over what was left of the cycle at its priority (200–300 ms) and, if it runs a counted number of laps, fades out again before its last; a looping one that has faded in drops the earlier animations of its priority, a counted one goes when its laps are done. The pose comes from the chain of blends from the top: on a track an animation plays it is faded over the chain by its weight, on a track it does not the chain's pose stands. `Characters::drive` (`net.rs`) tells the stack what the original's code tells the mesh: the movement at priority 0 claiming layer 2 (FUN_1006c370), its range from the `loopstart`/`loopend` notes for the start, the cycle and the stop (FUN_10010929), its speed scaled every frame (`SetSpeedScale`), a social action once at priority 0 claiming nothing (FUN_10010a46) — so a wave plays over walking legs instead of replacing the walk. Attacks (priority −2 on the layer the table leaves) are not driven yet.

## The statels' level of detail, from the original's code

The original's `LODManager` (displaysystem `lodmanager.rs`) watches every statel mesh: one more than 50 of its radii from the camera is hidden; beyond `distance² − radius² > 6000` its low-detail twin (type 0xf696a, the same instance; 668 of the 11228 meshes have one) is drawn in its place, and the full mesh comes back below 4000. `add_props` loads the twin beside each statel mesh (`Model::lod_groups`) and the renderer decides per instance every frame (`Lod`), for the shadow map too; `MAO_NO_LOD=1` turns it off to compare. The original looks at 20 meshes a frame, so its switches lag a little; the client's do not.

## The hit and tracer stars, from the original's code

Most hit and tracer effects are `_GfxControlStars_t` templates (kind 2004: 3534 hits and 2760 tracers name one). The two most used of its 29 modes are now played as the original plays them (`mao_formats::gfx::Stars`, `effects::stars_particles`): 128 billboard sprites of the template's material, each a random colour between the template's two colours and a random late frame of the sheet, growing and shrinking along a parabola over the template's life; mode 6 bears them on a circle round the attach point, rising and swirling, mode 19 scatters them in a sphere at the hit. The other modes keep the spiral the client drew before. The Stars control's decompile is mangled, so these follow its shape rather than its exact numbers.

The sparks of a hit (`_GfxControlSparks_t`, kind 1018, under 686 hit effects) are played as the original throws them (`mao_formats::gfx::Sparks`, `effects::sparks_particles`): a small burst from the attach point, each spark on a direction from the template's azimuth and elevation ranges at a speed from its speed range, pulled down by its gravity, running its colour, size and sheet frame from start to end over a random life from its range.

A hit of the `_GfxControlDeformer_t` kind (3001, mode 1, under 160 hit effects) bends the target's own mesh as the original's vertex callback does (`mao_formats::gfx::Deform`, `effects::deform_parts`): every skinned vertex is pushed along its normal by the template's amplitude times a squared sine running with the template's frequency, the strength ramped in over a tenth of a second and out over the last one to three seconds of the hit. The network side notes the hit against the target (`World::deforms`) and the character's posing applies it until the duration ends.

The tracer of 124 programs is `_GfxControlTracer4_t` (kind 1024): three cords wound round the line from the caster to the target, each twenty links of the template's width and colour, the winding's radius under a half-sine envelope along the line, spinning at 12.56 radians a second. The client draws each link as a sprite (`mao_formats::gfx::Cords`, `effects::cords_particles`) for the flight; the original's ribbon between the links and its locator riding along the line are not done.

The volume grids (`GfxControlVolGrid_t`, kind 3038: 31 hits, 27 casts, 24 tracers and 17 running effects name one of its 13 templates) are played as the original builds them (`mao_formats::gfx::VolGrid`, `effects::volgrid_particles`, from Gamecode's FUN_10116292 and displaysystem's `gfxvolgrid.rs`): a box whose height, width and depth follow the template's curves over its duration, filled with stacks of textured slices (20 across, 20 deep and 1 to 30 up) coloured by two colour curves, the middle colour below and the top colour above, the standing slices' bottoms half the width out and their tops half the depth as the original's own code has it, each slice faded by how squarely it faces the camera. The renderer's particles can now be quads laid in the world (`Particle::quad`) as well as billboards; the original's fan gradient within a slice is stood in for by two flat quads, and the client adds its slices to the scene instead of alpha-blending them.

The smoke streams (`_GfxControlSmoke_t`, kind 1009: 63 running effects name one of its 17 templates) are played as the original's run makes and moves them (`mao_formats::gfx::Smoke`, `effects::smoke_particles`, from Gamecode's FUN_100f04bd and displaysystem's `GfxVisualSprite2Type0`): the template's rate of sprites a second, each born on a disc about the emitter at a random height, drifting along the emitter's −z by the template's drift over its life, which the emitter's turn (a quarter turn about x in most templates) points up; its size, colour and sheet frame run from the template's start values to its end values over a life spread by up to 30%; push kind 5 rises on top and kind 7 flies off at random. The emitter's offset and turn and the attach point place it on the body. Not done: the push of the other kinds by the effect handler's force (the client keeps none) and the sprites' spin.

The single-quad effects (`_GfxControlSprite_t`, kind 1012: 27 running effects name one of its 16 templates, the tower beams and the halos among them) are played as the original runs them (`mao_formats::gfx::Sprite`, `effects::sprite_particles`, from Gamecode's FUN_100f6d2b and displaysystem's `gfxsprites.rs`): one quad of the template's material at the attach point plus its offset, cycling over the template's period for its count of cycles or its duration, its width, height, colour and sheet frame running between the template's start and end values as its flags ask, with the sinusoidal swell of the pulsing ones (the "random texture spot of the shimmering ones" this said was a misreading, taken out 2026-11-21: see "Lamp halos that were boxes"). A `GfxVisualSprite2` template is a billboard; a `GfxVisualSprite3` one stands on its height and turns about it to face the camera, which the renderer's particles now do when given an up without a right. Not done: a standing quad fixed in the body's own frame, the separate height of a billboard, and alpha blending.

The thrown billboard particles (`GfxControlBParticle2_t`, kind 3028: 21 hit effects name one of its 66 templates, debris and embers mostly) are played as the original bears and moves them (`mao_formats::gfx::BParticles`, `effects::bparticles`, from Gamecode's FUN_1010bf63 and FUN_1010b499): the template's count of sprites born at once within its scatter of the hit point, each with its own velocity, life and size from the template's ranges, pulled by its gravity, damped and grown at its rates, its sheet frame run under its frame mode and its colour and alpha read from the template's colour curve at the fraction of its life; a stream template reborns its dead particles while it still emits. Not done: the particles' spin, the ground bounce, the fades by height above the ground and by daylight, and the turn of the body they fly from.

Stars mode 22 (37 hit templates) now runs as the original's `case 0x16` does: each frame up to the template's count of stars are born on one of the body's six edges, the lines between the calves and thighs, the pelvis and head, the forearms and upper arms and the two upper arms, the template's word 28 out from the edge, and swing back through it on an undamped spring, dying as they cross it or when their life is spent. The edges are measured between the client's attach-point table, so they run across the body's x rather than its heading.

Stars mode 5 (23 hit templates) runs as the original's `case 5`: the stars of mode 6 born 1.2 below the point, flying outward along the circle's radius and pulled back toward the axis every frame, so the burst breathes out and in instead of swirling up.

Stars mode 20 (19 hit templates) runs as the original's `case 0x14`: fifteen stars a frame born in a sphere about the hit point, standing at one size for a quarter of a second, a dense sparkle at the point of impact.

Stars mode 2 (18 hit templates) runs as the original's `case 2`: all 128 stars at once on a sphere that grows from 0.6 to 1.3 over a second and a half while the stars shrink to nothing and run back through the sheet's frames, each in one of eight palette colours mixed with the template's second colour.

Stars mode 13 (17 hit templates) runs as the original's `case 0xd`: a ray of stars curling out from the chest, its direction turned a little each step about a drifting axis, the newest stars at the point and the oldest at the tip, the ray growing and shrinking back over the hit.

Stars mode 15 (14 hit templates) throws its stars off the target's own surface, as the original's vertex callback and `case 0xf` do: thirty of the target's posed vertices are sampled, and each star leaves one along its normal, slowing as it goes and shrinking over a third of a second. The client takes the vertices from the target's blob-shadow points, which now carry normals.

Stars mode 1 (10 hit templates, at the hands) runs as the original's `case 1`: two stars a frame appear a metre out from the hand and are drawn in onto it on a damped spring over 1.2 s, growing from nothing as they come, a gathering of light into the hand.

Stars mode 9 (6 hit templates) runs as the original's `case 9`: drops of light born two metres above the target over a small disc, falling at the template's speed, each a large glow at the top that shrinks to a small spark as it drops past the shoulders.

## The wind, from the original's code

The original's environment keeps a wind (`wind.rs`, from Gamecode's FUN_100bff68, FUN_100bfc80 and `_EffectHandler_t::ComputeWind`): a base wind as strong as the weather's wind value (held to 33), and two gusts kicked at random nearly every frame, larger the stronger the wind, decaying by a fiftieth a frame; the wind is the base plus the 3D gust, and the effect handler keeps a smoothed copy of it. The viewer steps it from the weather each frame and the effects it pushes now drift with it: the sparks of the 14 Sparks templates with a push (blown downwind more the higher they fly) and the smoke of the push kinds (by the smoothed wind). The wind and rain sounds the same functions drive are not done.

Stars modes 16, 17 and 18 (by the programs that name them, the most seen hit stars: 603, 4008 and 947 uses) run as the original's `case 0x10`, `0x11` and `0x12`. Mode 17's three templates each draw a spoke of stars half a metre long that turns about the hit point, their phases a third of a turn apart, so a hit shows a spinning three-armed pinwheel; 16 and 18 are short sparkles standing in a small sphere at the point. Counting the Stars templates by how many programs name them (`mao-client <game> gfx-usage 2004`) showed these three modes, five templates in all, outweighing the many-templated ones.

## The buff rings, from the original's code

A running buff's ring is the cord ribbon its placeholder lays (`effects::ring`, from Gamecode's `_GfxControlBuffPlaceHolder_t` and `_GfxControlCord_t`): the placeholder's point turns at the template's 12.6 radians a second and, every 0.45 radians, the cord gets a new link there; each link lives half a second, fading and drifting a little outward, and the ribbon of light_halo2 runs through the newest 32 of them, so the ring is drawn as a short comet that circles the limb and widens as it fades, for a second and a half each pulse. The original steps only forward-spinning placeholders, so of each left/right pair only the right limb's ring ever shows, as seen in the original. The placeholders' flares emit nothing, so the rings have no sparkles. The client lays the rings level; the original tilts them with the bone.

## The cast effect, from the original's code

A nano's cast effect (`CastEffectType`, 5237 programs) is `_GfxControlSpell1_t` (`effects::cast_particles`, from Gamecode's FUN_100f331d and its phases): while the cast lasts, a flare at each hand throws sixteen streaks a second in the cast's colours, each growing back from the hand along a random direction and fading over half a second to a second; a cast of four seconds or more would also draw a line of light between the hands that closes onto the middle. The effect ends with the cast; the client takes the cast to last as long as the casting animation it plays (1.2 s at someone, 1.95 s at oneself). The projectile and impact phases the class also has are switched off in every template a program uses.

The tracer of 611 programs (`_GfxControlPlasma_t`, kind 2002) is drawn as the original draws it (`effects::plasma_particles`, from Gamecode's FUN_100ec2e6 and displaysystem's `GfxVisualPlasma`): not a ball but a band of light three quarters of a metre long along the world's x axis from the flying point, a fifth of a metre wide, wobbling a little on four sine waves, in the template's colour. The original wobbles it across the view; the client wobbles it up and down.

## The shields, from the original's code

A shield effect (`_GfxControlShield_t`, kind 3003: 933 programs) is the character's own mesh drawn again a little bigger (`effects::shield_shell`, from Gamecode's FUN_100edcf4 and displaysystem's `GfxVisualShield`): every vertex pushed out along its normal by the template's distance (a quarter of a metre for the common ones), textured with the template's material scrolling over it, in its colour, fading in by a sine over the first second, and gone at its duration (3 s). The renderer has a new pass for it, translucent textured shells over the scene. A running shield plays once when it starts; a shield hit plays on the target when the spell lands.

The Suns effects (`_GfxControlSuns_t`, kind 2005: 587 programs) play their two used modes as the original's run places them (`effects::suns_particles`): the common tracer (mode 4) is a crackle of electric sprites standing at the flying point, three born a frame, each living 0.3 s and running back through its sheet; mode 0 stacks 32 concentric halos two units above the character, growing and shrinking over the effect's duration, in eight fixed colours.

The hit of 286 programs (`_GfxControlSpiral_t`, kind 2001) is a coil, not a flash (`effects::coil_particles`, from Gamecode's FUN_100f4cb5 and displaysystem's `GfxVisualSpiral`): two bands of thin smoke, half a turn apart, winding once round the target from the feet to nearly two metres up, 0.6 out, turning as they go; over two seconds they wind out to full length, and over the next two they are eaten away from the bottom.

Electra (`_GfxControlElectra_t`, kind 2006: 180 programs, mostly hits) is a crackle of lightning quads round the target (`effects::electra_particles`, from Gamecode's FUN_100d9de5): about fifty-six a second, each a randomly turned flat quad of the Electricify sheet some 1.4 m across, born on a tall sphere about the body and living half a second through its frames, in the template's colours, for six seconds.

The second shield (`GfxControlShield2_t`, kind 3034) is drawn over the target's own posed mesh like the first (`effects::shield2_shell`, from Gamecode's FUN_10111110 and displaysystem's `GfxVisualShield2`): the most used is a red shimmer of thin smoke added over the body, fading in over the first eight seconds of twenty and out over the last eight, its texture scrolling and its alpha a moving pattern of sines over the skin. As in the original it is skipped on meshes over 1000 vertices when the template says so.

Effect meshes (`GfxControlEffectMesh_t`, kind 3025) show one of the original's 28 named meshes, moved, spun, grown and faded as its run (Gamecode's FUN_1010ce9c) does (`effects::effect_mesh_shells`): Boom's hit is a glowing EMP ring turned to face the camera, growing from two to about forty units over two and a half seconds and fading after its first tenth. The most used ones, the Phasefront hoverboards' spinning fans and the hoverbikes' glowing circles, hang on the vehicle body's own attach points (`Attractor07_special` and kin): an effect mesh at attach point 2000–2023 is held at that attractor's frame on the character's posed body (`effects::Locator::attractor`, FUN_10105e6d with the name table at 0x102c63f8; the head's attractor when the body lacks it, nothing without a body), the frame times the character's place as FUN_10106591 composes them, and with flag 0x200 grown by the body's scale. The board's fan sits in the board's fan housing (checked in a picture from below). The tools `mao-client <game> gfx-usage <kind> <template>` (the programs naming a template) and `mao-client <game> names <text>` (the name index) helped find them.

## Hoverboards and hoverbikes, from the original's code

A Phasefront hoverboard or hoverbike program (270431 "Phasefront Phantom - Beige", 270327 "Phasefront Ghost", 270836 "Phasefront Spectre", 284003 "Phasefront Chimera" and kin) carries a `ChangeMesh` spell (`0xcf44`) for each body, like a polymorph's but keyed on the body: the hoverboards' criteria name stat 12 `Mesh` (the thin, plain or fat CAT mesh of a breed and sex, robed or not, ORed, or stat 359 `MonsterData` already the board), the hoverbike's stat 4 `Breed` AND stat 59 `Sex`. The argument is an NPC template whose mesh is the rider and vehicle as one CAT body (`Hoverboard_b_male.cir`, `Hoverbike_a_atrox_t9.cir`) with its own animations (`Hoverboard_a_male_run_001.ani`, `_idle_stand_`, `_turn_left_`, `_runbackwards_`, ...). In the original the server applies the spell by setting `MonsterData`, and the client's stat handler (FUN_1005a3b1, case 0x167) calls FUN_10052686, which loads the template (FUN_10052271) and sets its stats on the character (FUN_100525f3: its `Mesh` among them), so the character is drawn with the template's body and animations.

The client works the template out from the program, as for polymorphs (`item::vehicle_template` evaluates the spells' criteria in reverse Polish order against the body; `item::is_vehicle_program`): the player's own from its running nano list (`refresh_mods`), the others' from their `Buff` messages (30 s after the last one, `World::riding`). `Characters::vehicle` picks the template for the character's breed and sex (its plain body), and `Characters::parts` then draws `vehicle_parts`: the template's CAT mesh posed by the template's animations as a creature's are (the run for walking and running, the turns for the sidesteps, the run backwards, the idle stand; the movement rate as a player's, FUN_1007009f), with the character's own head, worn meshes and clothes on it (the board's body has no head of its own). Each posed body's attractor frames go with its blob-shadow caster to the viewer, where the program's effect meshes at `Attractor07_special` and kin are placed (see the effect meshes above).

`mao-client <game> ride <breed> <sex> <program> <out.png>` draws a character on its vehicle (`MAO_AURA=<stat 413 effect> MAO_GFX=<gfxtweak.bin>` adds the program's effect meshes at the body's attractors, `MAO_ACTION=walk|run|left|right` another animation). Checked in pictures: a Solitus male on Phantom's board with its fan in the housing, an Atrox on Spectre's bike.

A bundle (kind 2007, `_GfxControlMeta_t`, FUN_100e5f7a) starts each of its ten children as an effect of its own (`_EffectHandler_t::CreateEffect2`), so it is now drawn whole (`Visual::Meta`: every child with a look, each while its own life lasts; the pulse the longest child's, once only when every child plays once) where it used to be drawn as its first child. This reaches every program whose running, hit or tracer effect is a bundle, not only the vehicles. An Atrox's hoverbike circles are its own meshes (FUN_1010d457: flag 0x10000 and the target's breed 4 pick `hoverbike_*_effect_circle*_atrox.abiff`); the viewer now passes the character's breed. Spectre's circle (72346) is drawn, in the round port on the bike's underside (checked in a picture from below). Its mesh carries an offset of its own (the anim matrix and FAF key: 0.37, 0.12, −2.91 for the Atrox, 0.26, 0.09, −2.10 for the Solitus), and the original composes it as the port does, read from FUN_1010ce9c's disassembly: the position the locator's (FUN_10106306) plus the run's offset; the rotation the locator's matrix (the turn, here E2 by π, times the attractor's frame times the dynel's) without its translation, over the body's scale, made a quaternion and joined with the spin (0x1010d1de–0x1010d260); the mesh's frame at the origin under it with its anim matrix first (`UpdateWorldMatrix`: anim × TRS × parent; the drop-in `asyncmesh::attach` zeroes only the position and rotation). With the half turn and `Attractor07_special`'s frame (turned half round x), the offset takes the disc from the rear exhaust to the bike's centre line, 0.02–0.04 below its hull under the rider. (The bike's nose runs forward to +5 m; the rider sits at the back.) Tools: `ride` prints the special attractors' frames, placements and bones; `MAO_PROBE="x,y,z;..."` prints the posed body's nearest vertex and extent at points (Z up); `MAO_AURA` prints where each effect-mesh shell lands. Not done: the thin, fat and robed bodies (the client draws every character with its breed's plain body, so it takes that body's board), the server's own `MonsterData` (359) for the character is not read (the program is the source), jumps on a board (the template has no jump animation; the stand plays), the trails (3039) and particles (3028) at the attractors, and the vehicle flags (0x2000 / 0x4000: shown while the vehicle moves forward). Not tried live.

Shock waves (`_GfxControlShockWave_t`, kind 3000), the hit of Message from Sol, the costumes and Hack Grid Data Stream, are drawn as the original's run lays them (`effects::shockwave_shells`): rings spreading over the ground one after another (nine, a quarter second apart, for the commonest), coloured and fading as they grow, and a pillar of nested cones of light forty units tall that flashes with each new ring.

Trails (`GfxControlTrail2_t`, kind 3039) are the hoverboards' and hoverbikes' exhausts: four crossed glowing ribbons from the board's special attractors back through the last third of a second of its frames, blown backwards a few units a second, about 0.4 wide and pulsing in width, in the template's colour, and dark while the board runs into its own exhaust (`effects::TrailState`, from Gamecode's FUN_10115605 and displaysystem's `GfxVisualTrail2`).
In the `ride` preview (`MAO_AURA=72307`, with `MAO_RIDE_SPEED` for a ride along +y) Phantom's exhaust starts 2.5 units along `Attractor08_special`'s x axis (the template's locator offset), behind the board's back end, and is blown along that attractor's z: whether it lies so in the original is not yet checked against the original client.

VulcanRocks (`_GfxControlVulcanRocks_t`, kind 1029), the hit of Cataclysm, Eruption and Fiery Breath, throws up to 23 small rocks (`rock01.abiff`, about a fifth of a unit) almost straight up at 18 units a second from the target; they fall, bounce off the ground at half speed up to four times and come to rest, and the effect ends once all have rested, after about eight seconds (`effects::vulcan_rocks`, from Gamecode's FUN_10103bc1).

BParticle (`GfxControlBParticle_t`, kind 3024), the hit of Pulse of Frost, Cast Aside and Relativistic Decay among others, is a burst of large billboard pops (`effects::bparticle_particles`, from displaysystem's `GfxVisualBParticle` kind 8): Pulse of Frost's is ten pale blue frost flakes up to ten units across within a unit of the target, each popping within a few hundredths of a second and shrinking away over half a second.

Tracer5 (`_GfxControlTracer5_t`, kind 1025), the tracer of the flechettes and Molecule Lance, is a thin glowing streak running from the caster to the target at fifty units a second (at least a fifth of a second), its bright head half a unit long (`effects::tracer5_shell`, from Gamecode's FUN_1010065a and displaysystem's `GfxVisualCord4`).

Toggles (`GfxControlToggle_t`, kind 3036) start their child effect as a hoverbike moves off (`effects::resolve_toggles`, from Gamecode's FUN_10112952): the Spectre bikes give a puff from both exhausts each time they start forward, and a stream of exhaust while they keep moving, gone when they stop. The viewer tells moving off from each character's movement along its facing.

Tracer1 (`_GfxControlTracer1_t`, kind 1019), the tracer of Ice Flechette, Glacial Advance and Frost Slivers, flies a little model of glowing streaks, about two and a half units long, along the line from the caster to the target (`effects::tracer1_shells`, from Gamecode's FUN_100fe545 and displaysystem's `GfxVisualFlareType0`).

Tracer3 (`_GfxControlTracer3_t`, kind 1022) drags a nano emitter (`_GfxControlNano0_t`, kind 1007, now played wherever an effect names one) from the caster to the target: the commonest leaves a trail of soft blue sprites, three hundred a second, each half a unit wide, drifting and fading over half a second, gone as the tracer arrives (`effects::tracer3_particles`, `effects::nano_particles`).

TParticle (`GfxControlTParticle_t`, kind 3020) is a pool of streaks, each two crossed strips from its head to its trailing tail (`effects::tparticle_shells`, from displaysystem's `GfxVisualTParticle`): bursts of white sparks thrown up and falling, their heads brightest at half the effect's second, and still rays from a point out to scattered tails.

Sky flashes (`_GfxControlSkyFlash_t`, kind 3006), the hit of Gift of Life and Holy Book Blessings, are a beam of nested cones of light sixty units tall coming down on the target and fading over a few seconds (`effects::skyflash_shells`, from Gamecode's FUN_100ef141).

Crazy cones (`_GfxControlCrazyCone_t`, kind 3009), the hits of the Destruction perk line, are open cones about the target
whose radii, heights, colours and spin follow the template's tracks (`mao_formats::gfx::CrazyCone`, `effects::crazycone_shells`,
from Gamecode's FUN_100d6a6f, FUN_100d73b2 and FUN_100d6dcc): each track moves the values at its speeds and the speeds at its
accelerations for its duration, each further cone is a step wider, the cones share a turn, and a track's pulses blink the
colours. Not drawn: the texture's offsets the tracks scroll.

**Highlights** (`_GfxControlHighlight_t`, kind 2011; `mao_formats::gfx::Highlight`, `effects::visual_highlights`, from
Gamecode's FUN_100e2dbf, FUN_100e2efc and FUN_101086fa) tint the target's own body: each frame the original sets the mesh's
root frame transparency and emissive colour (and its attractor meshes') to a colour between the template's start and end
colours (words 3–10, alpha first) at a fraction of the run: mode 0 a ramp over the duration (word 2), modes 1 and 3 a bump
`1 − (2t − 1)²`, mode 2 a ramp over the fade held while the program runs and run back down when it is told to stop. The
renderer carries a tint per part (`PartUniform::tint`, `Scene::set_tints`): the emissive colour is added to the lighting as a
room's light map is, and a part under full alpha is drawn blended after the opaque parts (at alpha 0 not at all). The net
thread marks which overlay parts are which character's (`Update::character_parts`); the viewer tints them from the running
programs' visuals each frame, the player's own body through the follow parts, and keeps a sustained highlight fading out
after its program has gone. In the data: 11506, the running effect of Dimensional Shift, Monster Cloak Nano and Umbral
Fastness (mode 2: two seconds to alpha 0.2 with a 0.2 grey glow), 61112 and 61110 (alpha 0 for 10 or 3 s: the body gone in
the Destruction perks' hits and the scanner's sequence 61090, which then fades it back in over 61111's 4.5 s), and 11500–11508
under bundles no program names. Not drawn: mode 3 (the worn items alone, with their specular: 11507/11508, unnamed), flag
0x400's render priority, and the hide word (set in no template). Previews: `shots/hl_cloak.png` (11506 at 1 s),
`shots/hl_fadein.png` (61111 at 1.5 s), `shots/hl_gone.png` (61110: nothing drawn), `shots/hl_bump.png` (11502 at 0.5 s).

Spiral2 (`GfxControlSpiral2_t`, kind 3033) winds ribbons of light round the target, climbing as they turn and spinning about it (`effects::spiral2_shells`, from Gamecode's FUN_10111c40 and displaysystem's `GfxVisualSpiral2`).

TParticle2 (`GfxControlTParticle2_t`, kind 3031) throws the same particles as BParticle2 but draws each as a streak along its flight, bright at its head and fading to its tail (`effects::tparticle2_shells`).

The Rage buffs (Berserk, Incandescent and Fell Rage) burn (`_GfxControlFire_t`, kind 1004, `effects::fire_particles`): sixteen flames a second off a disc just behind the character, streaming back from it and shrinking over a second, pushed by the wind.

Ground shakes (`GfxControlGroundShake_t`, kind 3032) shake the camera: its eye jitters each frame by up to the effect's curve (a fifth of a unit for the commonest), less the further the camera is from the effect, to nothing at thirty units (`effects::shake`, from Gamecode's FUN_1010f12a).

Ground grids (`GfxControlGroundGrid_t`, kind 3030) lay a sheet of the effect's texture over the ground round the target, fading out from its middle and rippling, fading in and out over the effect (`effects::groundgrid_shells`).

## Attack swings

A character who attacks now swings its weapon, as the original's `AttackInfo` handler does (Gamecode `AttackInfoIIR_t`, FUN_1009f257 → FUN_1006ae3c → FUN_1006a782; a miss, `MissedAttackInfo`, goes the same way through FUN_1006b399). The network thread keeps the weapons the server says each character holds (`WeaponItemFullUpdate`: the owner, the place in `Unknown2`'s low byte, the template in stat 702) and each attacker's last swing (the place the message names); the player's own weapons also come from its inventory. The swing is one of the weapon's attack actions, picked in turn (the original's `rand()`): `WeaponItem_t` fills them when it is wielded (FUN_1009e84b → FUN_1009d964) by the template's AnimSet stat (353): 0 small arms (`smallarms-shot`, `-shotl` in the left hand), 1 a one-handed blade (slash, stab, slash), 2 a two-handed blade (chop, downcut, slash, stab), 3 a rifle, 6 a bow, 7 `spell-sys`, 8 a bazooka; any other set keeps the template's own animation set (its chunk 0x12 or 0x0e, class 0xb: the martial arts items' and the monsters' innate weapons' punches and kicks; the template reader is FUN_1002b458, chunk 0x14 being the sound set). The action's animation is `<prefix>_<name>_01_01.ani`, else `<prefix>_<name>.ani` (FUN_10010cc2; the 197 names are the original's table, FUN_100c0720, now `appearance::ACTION_NAMES`), falling back along FUN_10010b3c's chain (a rifle's shot to the small arms'); a weapon with no attack action at all waves (social 0x3e, FUN_10010dee's fallback). It goes on the character's animation stack at priority −2 claiming the action's layer (`~mask & 3`), played once, sped up by the animation's first note over the weapon's attack time (stat 294 × 10 ms) held to 1..2 (FUN_1006a782's `SetSpeedScale`). Checked: a captured fight's 202 attacks, 170 of them by a character whose weapon the capture named; every captured weapon resolves to an animation the files have (test `captured_weapons_swing_animations_the_files_have`: the Ofab Peregrine shoots, the Ofab Shark fires a rifle, crowbars and bats slash and stab, the Martial Arts Item punches and kicks). Not yet seen live. The character's own animation set (type 0xfde97, e.g. 26092 "solitus male") is a character's `MonsterData` (stat 359: FUN_1005c3ed / FUN_1005a3b1 → FUN_10052686 → FUN_1004e275), so players have none and use the file names, while NPCs with one are drawn from it (the template path); it is now read exactly as FUN_1004de5d reads it (the stats, then the animations with every variant of an action, counted 1009 × (n + 1)) and each character picks its variant (FUN_10045976 picks at random each time; the client picks per character). Not followed: the crawling variants (the owner's stat 430 at 14), and the target's height (FUN_1006a782's `param_3`).

**Creatures** swing too, by the same path: FUN_1006a782 needs the weapon the server names in the slot (the capture's NPCs have theirs), then a body with weapon attachment points (`Attractor03_lefthand`, `HasWeaponAttractors`) takes the weapon's actions and one without them `unarmed-rswing` (0x40a); the animation is the creature's own set's for the action (FUN_1003cac9 → FUN_10045976: one of its entries for the action, in turn), along the fallbacks, played once at priority −2 on the action's layer and sped up as a player's (test `a_creature_set_swings_and_dies`: the grid default set has 0x40a and a death).

**Special attacks** swing too (`SpecialAttackInfo`, FUN_100a1f10 → FUN_1006af0e → FUN_1006a782 with the special flag): the weapon in the message's `EquipSlot` swings the class FUN_1003c7a6 gives the character's queued special (`item::special_class`: Burst 0x16, Full Auto 0x17, Aimed Shot 0x15, Fling Shot 0x1c, Fast Attack 0x19, Sneak Attack 0x1d, Backstab 0x89), from the same per-AnimSet table (`item::weapon_actions`: small arms burst `smallarms-burst`, full auto `smallarms-auto`, a rifle's `rifle-burst` / `rifle-auto`, a blade's fast attack the slash and sneak attack the stab), else the plain attack. Only the player's own character has a queued special; the others swing the plain attack. Brawl and Dimach take the special's own item (FUN_10068c19), which the client does not know, so they swing the weapon's plain attack, as the original does without the item; a Brawl names place 0 in the capture, where the original finds no weapon and swings nothing.

**Weapons drawn in a fight.** A fighting character standing still with a weapon in place 6, else 8, else 0 (the server's `WeaponItemFullUpdate`s, or the player's own inventory) loops its weapon's idle instead of `idle-stand`. The weapon holder (`WeaponHolder_t`, FUN_1006ab3f) keeps a fight state at +0x44: 1 at rest, 2 once a fight starts (FUN_1006a1b1), 1 again when it stops (FUN_100690c8, which has the controller holster the weapon); wielding a weapon in a fight (FUN_1006b2dd / FUN_1006ada0 → FUN_1006ac49) or starting one has the animation controller draw it (its vtable's +4, FUN_1003cce2), which looks at those places in that order and loops class 0x10 of the weapon's map: `idle-smallarms` for small arms, `idle-blade` for one- and two-handed blades, `idle-rifle` (falling back to `idle-smallarms`), `idle-bow`, `idle-bazooka`; the martial arts items keep the unarmed stance their template's set gives. The client keeps who is fighting from the `Attack` message (from the attacker, naming its target), the full update's `FightingTarget` (flag bit 5, `HasFightingTarget`: the entity decoder used to give such a character up altogether; it now reads the identity, though no capture has one yet) and `StopFight`. Drawing the weapon (the fight starting) first plays its start once (class 0x1a: `smallarms-start`, `blade-start`, `rifle-start` ...), and putting it away (the fight over: FUN_100690c8 → the controller's holster, its vtable's +8, FUN_1003ce27) its stop (0x1b); the idle set with either waits for it (`SetAnimationDelay`, FUN_1003c6b2: the start's or stop's length less 300 ms). Out of a fight the weapon still sets its holder's movement when it is wielded (FUN_1009cda2, which also hangs its mesh on the hand): the idle becomes the weapon's class 0x29 when it has one (a rifle's `idle-2h`, a bow's `idle-bow`, a bazooka's `idle-bazooka`; the rest keep `idle-stand`), and a rifle (AnimSet 3) walks and runs two-handed (`walk-2h` 0x421, `run-2h` 0x422; `item::wielded_movement`). Wielding a weapon out of a fight plays its class 0x27 once before that idle (a rifle's `2h-start`), or, for a weapon without the idle, its class 0x28 before `idle-stand`; putting a weapon away out of a fight plays its 0x28 (`2h-stop`) — FUN_1009cda2 through the controller's vtable +0xc; most weapons have neither, so nothing plays. Not yet: a weapon without a start playing `idle-stand` once in its place, and the controller's busy count (+0x3c) that holds the draw back.

**Deaths.** A character or creature that dies now plays its death and lies in its last pose until it is despawned (about ten seconds in the capture), instead of standing on in its idle. The server's `CharacterAction` 99 (`Death`, from the one dying) names the death's action in `Parameter2` (500 `die-knees`, 503 `die-shot` ...); the original's `CharDie_t` (FUN_1007b803) puts the weapon away (FUN_100690c8), plays the sex's death sound and has the animation controller play the action (its +0x14, FUN_1003cc42 → FUN_1003cac9). A creature takes the animation its template's set gives the action, a player or NPC the original's file names (`male_die-knees_01_01.ani` ...), each along FUN_10010b3c's fallbacks (`die-knees` and the rest to `die-pain`, 6000); it is set on the stack once at the movement's place, direction 3, so it plays through and holds the end. The death ends with the `Despawn` or a new full update. Checked by a test on a captured death and despawn; not yet seen live. The death sound plays too (FUN_1007b803: an animation set's 0x1e, else `SM_Sandy_Game_FemaleDies` for sex 3 or
`SM_Sandy_Game_MaleDies`). Not followed: the controller's own priority and loop arguments for it (+0x18's, not decoded: the client holds the pose by the direction instead).

**No muzzle flash on held guns.** The `muzzleflash` attractor effect (a mesh connector `eff_muzzleflash` → effect template 2000, kind 1006, `_GfxControlFlare1_t`: Gamecode FUN_100def53 bursts it on its own timer, FUN_10106591 showing it while the mesh is shown) is a statel's (turrets); the held guns' meshes carry only `Attractor01_weaponfire`, and a gun's template names no effect, its stat 420 choosing the fire sound (FUN_1009d1f1). Nothing was found that flashes a held gun.

Tools: `mao-client <game> record <type hex> <instance> <out>` writes one resource's bytes; `anim-sets` lists every item template's animation set; `char-anim-sets` every character animation set (type 0xfde97) with its words.

## Sound, from the original's code

The client now plays sound through rodio, as SandyInterface.dll does over Miles (the `sandyinterface` drop-in). `audio.rs` is
`SandyInterface_t` and `SandyInterfaceModule_t`; `mixer.rs` is `SoundEngineInterface_t` with the Miles sample handles under it,
mixed into one 44.1 kHz stereo rodio source (the driver `SE_Initialise` opens). The sources come from the source files under
`cd_image/sound/SourceFiles` (`mao_formats::sound::parse_bank`: version 11, records of 0xe4 bytes, the sample's file name with
its bytes rotated, the children by ID): `SM_Sandy_Gui.sbf` (85 sources) when the module starts, `SM_Sandy_Game_Dummy.sbf` (6810)
when a playfield is entered (`PlayfieldInit` → `ActivateGameZone`). A source is named by the hash of its name
(`mao_formats::sound::sound_id`, FUN_10008213: `SM_Sandy_Game_Feet_Man_Walk` is 0xc743f93d). Playing one works out its volume
by distance from the listener between its minimum and maximum distance, then its random volume range; it plays its variant for
the sound material (`MapGameMaterialToSoundMaterial`), or its children (all, one at random, in turn, or on timers); the loudest
play of each source in a frame is carried out at the frame's end. Samples are read when first played (WAV in PCM, IMA ADPCM or
MP3, and Ogg Vorbis), within the original's 15 MB budget that frees the least recently used. The engine keeps the original's 13
channels and its priorities, and a pitch is the rate the sample played at times a percentage. The listener is the controlled
character (FUN_100b1f3c → `SetListenerRotPos`). `MAO_NO_SOUND=1` keeps the client silent. To try a source:
`mao-client sound <game> <name> [material] [seconds]` plays it and prints the sample files that played.

**Steps and swishes.** The characters' animations now play their notes' sounds, as Gamecode does. A note's name gives its
event (FUN_10075c84: `left`, `right` and `step` 0x26, `land` 0x85, `swim` 0x38, `swish_punch` 0x73 to `swish_huge` 0x76,
`idle` 0xf, `attack_start_1..9` 0x77..; `step` is tested before `stepfast`, so a fast step is a step); a note fires when the
animation's time passes it (FUN_1003c248 over FUN_1003c0b7: from the first note at or after the last run's start to the last
before its end, wrapping round a loop, within the range played). FUN_100452d3 then plays at the character: a player its
controller's own sounds (`SM_Sandy_Game_Feet_Man_Walk`, `..._Swim`, `..._Land` at twice the volume held to 1,
`SM_Sandy_Swish_punch` ...), an NPC with an animation set its set's sound for the event (the sets' sounds section, FUN_1007dc1e:
985 of the 1469 sets have some), a variant picked at random. Steps play on the material under the character: 5 (concrete)
indoors or in the air (more than 0.1 above the height of the tile's corner), else the tile's material from `RDBTileMaterial_t`
(0xf4258:1). The game's own table is version 3, which N3.dll's reader skips (only versions 1 and 2 are read, 0x1001c582), so in
the original, and here, every ground tile is material 0, which `MapGameMaterialToSoundMaterial` makes 3. In a liquid (`liquid.rs`,
the character under the surface: the vehicle's +0x10c depth and +0x110 type) a step is on material 10 for liquid type 2, 11 for
type 6, else 8 deeper than 0.35 and 9 shallower, and a landing makes no sound. A landing (without an animation set's own sound) starts the
environment's effect 0x2c where it lands, 0.2 lower and kept above the ground: the lightning's splash, which shows only where
that is in a liquid. Not followed: a fly-mode step at a quarter of the volume.

**Shots and hits.** An attack's swing fires its `attack` note (0xb; `attack_effect_1..4` 0x8e..0x91), and the attack's sounds
play as FUN_10068e42 → FUN_1009bd92 plays them, with the weapon of the place the swing used (the server's
`WeaponItemFullUpdate`, or the client's own inventory) and what the server's `AttackInfo` put on it (FUN_1006ae3c: the target,
the hit type and the damage; a `MissedAttackInfo` no hit): the weapon's sound for the event from its template's sound set (chunk
0x14, the item's +0x88 map; every captured weapon has a shot sound under 0xb), or its 0x2c when a player's weapon has no Energy
left (stat 26, the rounds: `WeaponItemFullUpdate`'s, then each attack's `AmmoCount`), at the attacker (FUN_1009d2b2). For a hit
(type 2 glancing, 3 normal, 4 critical; the capture's 202 attacks are all hits) FUN_1009b9f3 plays the weapon's sound for the
event again (else its 0xb), and with damage the impacts at the one hit, 0.4 s later: a creature with an animation set its set's
impact (0x1f) and the weapon's (0x1f) on the set's material (its stat 0x29), a player `SM_Sandy_Game_MaleGetsHit` or
`..._FemaleGetsHit` by sex on material 7; the variant moves by the damage against the maximum health (a tenth or more: mode 2,
a twentieth: mode 1, less: mode 0, which pick the material's variant +15, itself, or +30). Not followed: a special attack's own
fields (FUN_1006af0e), hits on items (FUN_1009b4b0), the "Sneak Attack!" texts.

**Music.** The interactive music plays as SIMPlayer.dll plays it (`music.rs`, from the `simplayer` drop-in): the project
`sound/music/env/anarchy.sws` (serialize.dll's object archive: `mao_formats::music`) holds 91 layers (directories such as
`plains\day`, `battle\Neutral`), 1257 short samples and their transitions (where in a sample to leave, the fades, the time).
A thread steps the player every 10 ms: a playing sample takes its next transition (FUN_10008d2e: to the asked layer when one goes
there, by weight; forced, one made to an entry sample of that layer; else one within its layer), the next sample is prepared
200 ms ahead and started at the fade out, the two cross-faded by the fades' tables (50 ms steps). A layer's pause track holds the
music to stretches (the desert by day plays 188 beats, then rests 940). What plays: the startup music (`mountain\night`, as
GUI.dll's `CharacterLoggedInMessage` starts it) at login, then once a second the music of the district the character is in
(FUN_100b72bf: the playfield's districts, `mao_formats::districts`, by N3's zone grid; the district's layer for the weather, past
0.4 of the strongest precipitation the 7th, of the weather's entry 0 the 6th, of entry 1 the 5th, else for the day's period:
dawn, day, dusk or night); in a fight, the battle layers (`ProcessCombatMusic` over the character's reports, FUN_10059c7d: its
health in percent, its level). The sound preferences (volumes, mutes, the battle music mode) come from the GUI's
`LoginPrefs.xml`. `mao-client music <game> <layer> [seconds]` plays a layer and prints the samples. The server can override a district's music for a while (a
`ScriptIIR_t` of type 3: the music's layer name, the district's name, a start and a length; type 1 ends the last), which then
plays in its place. With Notum Wars (the character's stat 389 bit 0) a district of land control (one with a land control
level range) plays the land control music (`Landcontrol_neutral`, the layer -911) instead. Not followed: indoor rooms' zones.

**Environment sounds.** The world sounds as `Environment_t`'s frame (FUN_100becbe) plays it, every frame through
`PlayEnvSound` (`ambience.rs`): the district's background for the time of day, `SM_Sandy_Env_Background<Night|Dawn|Day|Dusk>_<district>`
(FUN_100bd930's windows: night to 7:00, dawn 7–9, day 9–21, dusk 21–23, each fading in and out over 300 game seconds, late
night counted as 1:00; 103 districts have all four; each background is a loop with timed calls of its own); the rain at the
weather table's entry 0 squared times entry 8 over 3.5; and the four winds (FUN_100bff68: the plain wind, the sand storm, the red
and the green fallout winds) at `(0.5 + 0.5 × gust) × strength / 33` times their shares of the weather (`max(1 − entry 2,
entry 4)`, entries 3, 5 and 6), the gust being the wind object's (`wind.rs`). The quake and the thunder are below.

**Nano and effect sounds.** A nano cast plays the program's own sounds as `CharCastNano_t` does (5255 of the 10964 programs
have some, all sources of the game's file): its gather sound (stat 269) when the cast is made, at 0.6 and held a moment, then
each frame while it gathers (your own until the server starts the cast); when it starts (FUN_1004f434) the cast sound (270) and
the travel sound (271) at the caster, the travel sound on while the cast lasts (the nano's cast time, stat 294); then the hit
sound (272) at the target. Effects carry sound controls too (`_GfxControlAudio_t`, kind 4000: twelve templates, the buff and
debuff pings, the land control towers' sounds and an explosion): each plays once at the effect's place when the effect is made,
as SandyInterface's `PlaySoundCommand` (with its range, delay, deferral and pitch).

**Quakes and jingles.** The environment's quake plays as FUN_100becbe runs it (`ambience::Quake`): while the weather's entry 12
is up, a count grows each frame and past 100 starts a quake of up to 2 s (FUN_100bd902), which plays `SM_Sandy_Env_Quake` at
its share of the full strength and shakes the eye by up to about 8 cm on each axis, running down with time; then the sound
stops. Your own new level plays `SM_Sandy_Game_Level_Complete` (`NewLevel`, FUN_10075f55), a new shadow level its Neutral, Clan
or Omni version by your side (`ShadowLevel`, FUN_10077ae7), and a finished mission (`CharacterAction` 59, FUN_1005d61f) the
mission-complete jingle by your side.

**Hazards.** Mines, traps and central controllers sound as the original's `Mine_t`, `TrapItem_t` and `CentralController_t`
do (`hazards.rs`): their places come from their full updates (each a simple item's update first); when the server says one was
sprung (`ActionIIR_t` 0x6d) `SM_Sandy_Game_Explo_Big` plays at it and, if it caught you, the Combat tab says so; a disarm
(0x6c) says you succeeded; a mine's malfunction (0x8c) plays its malfunction sound. A central controller hums its idle sound
while idle and plays its shutdown sound once when it shuts down. The blast's effect is below ("A sprung mine's or trap's
blast"). Not done: a mine's bleep as you walk into it.

**Voice lines.** A group (channel) message whose blob carries a voice (record 1: breed, sex, mood and line) plays it as the
original's chat does (`voices.rs`): when your character's stat 389 has bit 0, the group allows it (organisation groups by
`VoiceSndFxHearGuildOn`, the team by `VoiceSndFxHearTeamOn`, as the player has them now: `prefs.rs`, the control center's
values, which the option panel sets; `LoginPrefs.xml`'s only before it has loaded), and the sender was not heard in the last
3 s, the first of `<sender>_<line>_01`, `<breed>_<sex>_<mood>_<line>_01`, `<sex>_<mood>_<line>_01`, `<sex>_<line>_01` and
`<line>_01` found in `cd_image/sound/sfx/player/` plays at the voice volume (e.g. `atrox_male_cool_heal_01.wav`). Not done:
voices in vicinity speech, and sending one.

**A mine's bleep.** Walking into an armed mine bleeps it as the original does (`hazards::Hazards::bleeps`): N3 checks only
your character's body against the other dynels, sphere against sphere, each sphere the mesh's packed sphere word (radius from
its top byte, height from its second, through DisplaySystem's curve) times the body's scale (stat 360); the mine reads its
collisions on the frames its count of every other second is a multiple of its stat 437, when it wants them (flags bit
0x80000), and bleeps when its stat 36 is set, it is armed (its full update's flags bit 1) and has not malfunctioned, once each
time the game clock's second turns, higher the closer you are: `(3 − distance) × 10 + 100`.

**Vicinity speech.** Chat packet 34 (`u32 speaker, text, blob`, the ppj parser's `S2C_VIS_MESSAGE_FMT`) is vicinity speech:
it shows as `Name: text` and its voice line (the blob's records after its first byte) plays when `VoiceSndFxHearVicinityOn`
is set; packet 35 (`name, text, blob`, `S2C_VIS_ANON_MESSAGE`) is the anonymous kind the system lines come as, read the same
way.

**Thunder.** Storms thunder as the original's rain does (`ambience::Lightning`): two schedules of strikes, the same for every
client (an R250 generator seeded 0x85c19 for storms, gaps of 0.1 to 20 s over 600 s, and 0x1e23a for fallout, 0.1 to 2 s over
300 s, used while the weather's entries 5 and 6 add up to more than 0.2), run on the game clock (its seconds into the day over
15). While the weather's entry 9 is above 0 and its entry 2 at least 0.4, each strike falls somewhere in a cell six zones
across around you and plays `SM_Sandy_Env_NearLightning`, quieter the farther it is (`1 − d / cell × 0.5`) and later by
`d / 100` seconds, like sound travelling. The bolt and the camera's jolt are below ("Lightning's light").

**A sprung mine's or trap's blast.** When one is sprung its effect (0x834, template 2100) bursts at it: a flare of 64 streaks
made at once (its word 31, which `_GfxControlFlare_t` makes first, FUN_100dd071 taking it off the count it has made, before the
rate of word 10 — none here — adds more), flying out in every direction for 1 to 2 s. Flares started this way stand alone in
the world (`Update::flares`) and go when their streaks have.

**Lightning's light.** Each strike heard is also seen (`lightning.rs`), as DisplaySystem's environment effects draw it: a cloud
light 30 above you at the strike (effect 8, `VisualCloudlight_t`), ten square flashes across the sky (the first 55 either way,
the rest scattered by an R250 generator seeded with the strike's word), yellow or blue-white at their centres and clear at their
corners, fading over a second (`1 − (1 − age)³`, each flickering); and bolts (effect 9, `VisualLightning_t`), one from the
strike to the ground and one at each scattered flash four times in ten, 5 to 14 jittered segments narrowing from 2, living up
to half a second and shaking more each frame. Both are added to the scene with the environment's `dustcloud.png` and
`newsun_frame01.png`. A strike within 25 jolts the eye: the quake is kicked to `1.5 × (1 − d / 25)` of its full strength, its
rumble and shake with it (FUN_100bdfe3, the environment's +0x78).

**Speaking a voice line.** `/voice <sound name>` (heal, help, inc, no, run, yes) speaks one as the original's FUN_100b88b1 does
(`voices::command`): it needs Notum Wars (stat 389 bit 0, else "This function requires Notum Wars."), picks your breed's and
sex's voice in your `VoiceSndFxType` mood (the option panel's "Your voice", as FUN_1008c472 reads the value each time;
`CharPrefs.xml`'s simple by default), refuses one you don't have ("You do not have that
voice effect installed.": a name with both a `.wav` and a `.txt` in `sfx/player`), and sends the first line of its `.txt` (any
`[T]`/`[M]` tags taken off: "Medic!") with the voice record as the message's blob. The original sends to the chat window's
selected group; this client has no group selection, so it goes to your organization's channel. A channel flagged 0x40 takes no
voices: the send is refused with the original's words, and a voice line received in one is dropped.

**Speaking in the vicinity.** Plain text and `/say`, `/shout` (`/s`) and `/whisper` (`/w`) are said to the people around as the
original says them: GUI.dll packs the text's length, the text, its kind (0 say, 1 whisper, 2 shout) and any voice record
(FUN_10089669), and Interfaces.dll sends it on the game connection as a `TextMessage_t` (frame kind 5) of object 3, 4 or 2,
with the current target's identity and the data's length before it (`Client_t::SendVicinityMessage` and its kin). The line
comes back from the chat server as vicinity speech (packet 34), so nothing is echoed here. `/voice` lines are said this way too:
the original sends them to the chat window's selected group, Vicinity unless another is picked.

**A bolt's rocks.** Each frame a bolt lives, its tip throws a rock (effect 0x2d, run straight away by the environment's frame):
one of `rock01` to `rock07.abiff` at random, set off at 3.5 to 6.5 a second in a random upward direction, spinning at 2 to 6
radians a second about the up direction crossed with its way, falling at 9.8 (Gamecode's `EnvFXGravityObject_t`). On the
ground it bounces, up to seven times: its speed divided by `4 × normal.y + 4` and kicked a quarter of its fall along the
ground's normal, its spin slowed by a third; then it is gone. The tip's other effect, a water splash, only shows over a liquid,
which this client does not know yet.

**Emotes.** `/me <text>` is vicinity speech of kind 3 (FUN_1009d247). `/emote <name>` and `/<name>` (the 70 of GUI.dll's
table: `/wave`, `/bow`, `/disco`, `/sleep`, …) do a social action as the original does (`N3Msg_DoSocialAction`): a
`SocialActionCmd` from your character to the server (the command's byte, a word 0, N3's command number counting from 1, the
action's `AbstractAnimID`), which the server shows to everyone; `/lounge` and `/sleep` want you sitting ("You must sit in order
to use /lounge or /sleep."). The hotbar's and the actions window's social actions are sent the same way now. An unknown name
says the original's "Error: No emote named '…'".

**Liquids and the lightning's splash.** The playfield's liquid is known now as the original tests it (`liquid.rs`): the still
water triangles of its definition object (cut small, on the 60 by 60 tiles), a point in one when below its top, inside its
triangle, and its depth reaching past the point. A bolt over water ends on the surface; its tip, under it, throws no rock but
splashes (gfx 0x859b, `_GfxControlSplash_t`): four cones of material 35 on the surface, each starting 0.025 s after the last
and living 0.75 s, swelling up from a 0.8 ring and fading from a faint white. A rock that falls into water splashes there too,
and in the water its speed and spin are halved each frame.

**Effect scripts (FXS).** The game's effect scripts run now (`fxs/`, FXS.dll ported): the text files of `cd_image/twk` read
into objects of typed variables (strings, numbers, vectors, quaternions, matrices, sounds, arrays of them) whose values are
formulas over other objects' variables (`This`, `Parent`, `Spawner`, `GAME`...), each object's `Heartbeat` worked out every
frame and the object ended when its `Use` reaches `e_Kill`. When a playfield becomes current its tweaks load
(`Tweak_Playfield_<n>`, else the outdoor or indoor default: the `GAME` interface, the sun, moons, clouds, horizon...), and
the `GAME` object gets the frame's time, the day's time, the server's clock and the place each frame. The server's script
events (`ScriptIIR_t` types 0, 2 and 4, such as the alien flybys) run their scripts: an event's heartbeat spawns its
children (`[SEND]`, the crafts), and their sounds play. What they show is drawn (see the sky below); `MAO_FXS=1` lists the
effects started and the scripts' errors.

**The sky.** Outdoors the sky is the effect scripts' (`Tweak_Playfield_<n>` or the outdoor default): behind the world, in the
scripts' order, the star dome, the three moons (lit by the space lights), the atmosphere's gradient (its colours by the time of
day, alpha by its intensity row: stars show at night), the battle stations, the two suns' ray fans (Gamecode's solar model:
they rise about 7:15 and set about 20:30 and dim below the horizon), the thick clouds (by the weather's cloud intensity) and the
horizon's dish under the horizon line. The scripts' ground light (`GAME.Sun1Rotation`) is the scene's sun, so shadows follow
the sun over the day (held a little above the horizon at night). `MAO_SKY=<hour>` with `map <playfield> --image` renders it
headless (`MAO_SKY_ONLY=<sorting,…>` for single layers).

**Which scripts a playfield gets** (`mao_formats::playfield::sky_scripts`, from `n3EngineClientAnarchy_t::PlayfieldInit`
0x10016e94 and Gamecode FUN_100b1e95): the playfield's own `EnvironmentData_t` (the 26 bytes at the end of its definition
object, `Anarchy::environment`) carries a sky kind in its byte 0x12. On a ground tile-map kinds 6–11 become the environment
types 7–12 and the rest 0; in an interior kind 1 is type 3, 2 is 2, 3 is 4 and the rest 1. Types 1–4 load the playfield's
own `Tweak_Playfield_<n>` (or the indoor default) plus an extra: the Rubi-Ka sky objects without the atmosphere and clouds
("OrbitSpace", type 2), or with them ("RockSkyline" for type 3, "SandSkyline" for 4: both the same stars, moons, atmosphere,
sun, single clouds and horizon); types 10–12 load the Omni, Neutral and Clan training grounds' scripts (950, 954, 952)
outdoors; anything else the playfield's own outdoors. In the data every outdoor playfield comes out as type 0 (the cities
carry kind 1 or 2, Borealis 3, the training grounds 6–8, the Pandemoniums 14, none 9–11), and the interiors with a kind are
the Shadowlands temples, the Arrival Hall, the Teleport Tower, the Shadow Crypt and the Sealed Antechamber (kind 1: the
Rubi-Ka sky over them) and Smuggler's Rearranged (kind 2: the orbit's stars and moons). `MAO_SKY_ENV=<0..4>` forces one in
a headless render. The environment type also steers the original's indoor sounds and effects (FUN_100b2478's switch), not
followed.

**Single clouds and the scripts' light.** Thirty cloud puffs (`single_cloud01`–`04`) hang round the camera at the script's
altitude, sinking toward the horizon with distance, drifting back as you move and wrapping across a 3200-wide field; they fade
in a storm and darken at night and under thick cloud. The world is lit with the scripts' colours: their ambient light (dim at
night, brighter by day) and the ground light's colour (warm at dawn and dusk, blue at night) in place of the client's fixed sky
and sun colours, and distance hazes into the atmosphere's horizon colour. The game database may hold more segment files than
its main file's header counts (an install updated by a texture pack): the reader opens the further ones that belong, which
brings back their textures.

**Cloud drift and texture scrolling.** The single clouds drift with the high-altitude wind (the weather's wind and its gusts,
scaled as the original's environment scales them) as well as against your movement. The scripts' texture transforms are
followed, but the thick clouds' and horizon's scrolling never comes to anything, in the original as here: their matrices name
`This.ScrollMatrix`, which a state blob's transform cannot reach, so they stay still and untiled.

**The resource database (this install).** The texture pack installed on 2026-08-01 left the database inconsistent: the main
`.dat`'s header (restored from a backup of 2026-07-31) counts 3 of the 14 segment files, and 2006 of the index's 460193 entries
pointed at no record or at another key's. `mao-formats` examples find and fix it: `dbbroken` groups the broken entries,
`dbscan` scans every segment for their records (`docs/db-repair/dbscan.txt`: one intact copy each for 1836 keys), `dbverify`
decodes them, `dbmissing` names the 170 found nowhere (`docs/db-repair/dbmissing.txt`: their positions in segments 0, 4 and 8,
the part the restored main file overwrote), and `dbrepair <game> [--write]` rewrites only the leaf
entries' position words, after checking each new position holds that key's record and no other entry's, keeping the old index as
`ResourceDatabase.idx.before_repair`. Done on 2026-10-28: 170 broken entries are left. The main `.dat`'s header was not touched.

**GUI.dll's view system (`gui/`).** The skin's `Views/*.xml` windows are being moved onto a port of GUI.dll's own view
system (`docs/GUI_DLL_PARITY.md`): the view tree and its horizontal/vertical layouts with `SpaceOut`, spacers and width groups,
the XML loader (labels like `#Back` or `#10010:0` from the text database), the font table, and the first classes (border
views, buttons, check boxes, pictures, plain text). Such a window sits in the client's window frame; `MAO_UI_PREVIEW=view
MAO_VIEW=<file>.xml` with `map <pf> --image` draws one offscreen (all 27 files load and lay out; Skills, Tip and Info look
right, their text through the port of `HTMLParser_c`/`TextRenderer_c`: colours by `#rrggbb` or `TextColors.xml` name, fonts
by face, centring and right alignment, `indent=wrapped` blocks, underlines, clickable links in 0x2299ff; `MAO_VIEW_TEXT=<markup>`
puts text into a view named BrowserView). Not used by the game's windows yet.

**The option panel** (`gui/options.rs`, the control center's Options menu entry,
`optionpanel_window`): the skin's `OptionPanel/Root.xml` as GUI.dll's
`OptionPanelModule_c` shows it (FUN_100c3d62): a folder list of its pages on the left
(each page's `label`, looked up in category 700, split at `/`: Audio, Video, GUI,
Controls ...), the page picked on the right. The game's own `Root.xml` is empty; the
AOU skins fill it. The controls are ports of `OptionControl_c` (FUN_100c29cd):
`OptionCheckBox`, `OptionSlider` (FUN_100c1178: label, `value_fmt` value, `Slider_c`),
`OptionRadioButtonGroup` with `RadioButton`s; `view_enable_expression` greys a control
out. Each shows one setting: a distributed value (`opt_type="variant"`, ranges from the
prefs files' `min`/`max`) or an `IndependentPrefs_t` preference (`int`/`float`, with
GUI.dll's defaults and ranges from `Preferences_t::SetDefaultCharPrefs` /
`SetDefaultLoginPrefs`). What the player changes is saved to `prefs.xml` beside
`mao.ini` and loaded at the next start. The sound settings drive the sound module as
GUI.dll's do (FUN_100c573c: `MasterVolume`, `FXVolume`, `MusicVolume`,
`VoiceSndFxVolume` and the on/off switches mute). The client reads the rest through
`prefs.rs` (a copy the control center keeps): the Effects page's switches decide which
effects are made, as Gamecode's `_EffectHandler_t::isEffectInCategoryActive` does
(the nano programs' visuals on characters `BuffsFX`, casts `NanoEffectFX`, shots
`TracersFX`, strikes `EnvironmentFX`, flares and landings `OthersFX`), `Shadows` the
ground shadows, `MouseLookInverted` the mouse look. Not used yet: view distance, fog
mode, water, clouds, wildlife, names, character fading. The client's own settings window
(window mode, keys) is still there for what the original has no option for.
`MAO_UI_PREVIEW=optionpanel` (with `MAO_OPTION_PAGE=<n>`) shows it.

**Chat markup.** A chat line the server sends with markup (`<font color=...>`, `<a href="itemref://...">`) keeps it and is
drawn with GUI.dll's text view (`gui::controls::markup`: `HTMLParser_c`/`TextRenderer_c` from `gui/text.rs`, the chat bitmap
font); clicking a link opens it in the info view (an item's page for `itemref://`). Lines without tags are drawn as before.
`MAO_UI_PREVIEW=chat` shows one.

**The info view (`gui/infoview.rs`).** GUI.dll's `InfoView_c` window (`Views/InfoView.xml` of the skin) opens from the
inventory's right-click **Info**: an item page built as GUI.dll's FUN_1003882b builds it (`gui/iteminfo.rs`) — the name in
its rarity's colour, NODROP/UNIQUE, rarity, quality level, the requirements of each action in Gamecode's words
(`gui/criteria.rs`, a port of `N3Msg_GetUseByString`/`CriteriaAndOrNot`/`ConvertCriteria`: "Wear: / On Self: / Strength
from 826 and ..."), the locations, the modifiers (`gui/spells.rs`, `N3Msg_ConvertSpellList`: "On User: / Modify Radiation AC 819", read with a
spell reader in `mao_formats::spell` from GameData's spell formats), charges and description, all labels from the text
database. Back/Forward walk its history; links in its text open their page (`text://`, `file://`, `chatcmd://` run as chat
commands). Weapons get FUN_1003732c's page (ammunition, damage, speed, initiative, range, specials, attack and defence
skills from the template's skill lists, `item::skill_lists`). The nano programs window's right-click **Info** opens a nano
program's page (FUN_1003683d, `itemid://53019/<id>`): nano cost with the reader's cost modifier (red when their nano is
short), NCU, school, stacking order, nanolines, duration, range, speed, cooldowns, attack/defence skills, modifiers, on
death/on cancel, requirements, description. Weapons show their damage per second with the reader's attack and recharge times (`N3Msg_GetDpsString`, Gamecode
FUN_10067c74/FUN_10067d92: initiative skill and stat 51). Not yet: the nano radius, shop prices, spell types other than
the common ones.

## Fighting live: what was wrong and the test switches

Tried live with Sadlee (2026-10-29). Tab, Q and E had stopped reaching the game: egui takes the keyboard while any widget
has focus, and its own Tab focuses the first button, so after one click every key went to egui. Now the keys go to egui only
while a text field has focus (`Ui::event`), and the chat input lets go after a line is sent (Enter still starts typing, as
the original's key table binds it; `/` starts a command with the slash typed). The server's plain `Feedback` lines
(`50544d19`, text.mdb category 110: "You are too far away; please move closer!", "The target is outside special attack
range!") go to the chat's Combat tab. `/follow` on the creature being fought closes to 2 m (at the old 3.5 m the server only
answered "too far"). The world connection used socket read time-outs; on Windows a read that times out leaves the socket in
an undefined state, and the client lost the connection mid-fight (error 997, or a frame read from the middle of the stream).
`mao_net::socket` reads non-blocking and polls instead.

* `MAO_COMBAT_LOG=1`: the fight's server messages, the attack key's decision (with both places and the distance), Tab's pick,
  corpses linked to bodies, containers opened.
* `MAO_AUTOPLAY=15:tab,16:attack,17:follow,30:stop,31:use`: keys pressed by themselves that many seconds after entering the
  world (`tab`, `attack`, `use`, `follow` the target, `stop` following), for testing a fight without a keyboard.
* `MAO_TRACE=1`: also each movement message sent, and while fighting where one stands against the one fought.
* `net <game> attack <character> <seconds>` with `MAO_ATTACK_RANGE=<m>`: the headless fight stops that far from the creature.

## Creatures walking (`FollowTarget`, `SetPos`)

The server moves creatures mostly by `FollowTarget` (`260f3671`, Gamecode's `FollowTargetIIR_c`, up to 30 places): type 1 a
path of waypoints, type 2 a target to follow (live, always identity 0: stop where it is). The client ignored it, so a
wandering or charging creature stood still until its next full update: Tab's distances, /follow and the place of the body a
corpse is matched to were all off by metres. Now (`World::paths`, `walk_paths`) each path's first place is taken as where the
server has the creature (it is moved there), and it walks the rest at its speed; `SetPos` (`195e496e`) puts it down. Live,
wandering creatures send a two-place path every 1-3 s and walk a steady 1.5 m/s (the default); the speed of each creature is
learnt from two paths in a row (how far along the first the second starts, over the time between them: `path_speed`), so a
running one keeps up. Tested (`creatures_walk_their_paths`) and checked live: the body's place now matches its corpse's to
half a metre. Dead ones stop. Kills and loots tried live (2026-10-29, Sadlee): two items taken from a Reet of Paradise.

**Bodies stay down.** The server despawns a dead creature ten seconds or so after it dies and announces its corpse where it
fell; the client used to clear the death on the despawn, so the body stood up again. Now a dead body's despawn keeps it lying
there (dead, its death animation held) as its corpse's picture until the corpse itself despawns, and then both go; any other
despawned character leaves the world (it used to stay drawn where it was last seen). Tested on the captured death and live.

**Corpse names.** The server names a corpse in its `CorpseFullUpdate` ("Remains of Leet": a big-endian length and that many
bytes ending in a zero, after the corpse's stats; `corpse_name`, tested on a live corpse). The corpse and the body it stands
for are shown by that name (target bar, tags, the loot window's title, "Using ..."); "Remains of " and the creature's name is
the fallback when no name is found.

## Logging out (`/camp`)

As the original (GUI.dll `FlowControlModule_t::StartQuitToLoginMessage` → Gamecode `N3Msg_StartCamping` 0x1001c9c6, then
`CampStartedMessage` 0x1002a86c): `/camp` is refused while anyone is fighting the character ("You can't log out
while in a fight."); one's own attack stops; the character sits (`SwitchToSit`, movement 30: without it the server ignores
the logout) and sends the logout action (CharacterAction 0x78); the chat says "Timed logout started. This will take 30
seconds." (text 200 `LogoutStartedXseconds` fed 30) and the original's timer bar, labelled "Logout", runs out over 40 s in
the middle of the screen (`TimerBar_c`: a 128 by 16 bar in white; drawn plainly, the original's bar pictures 0x1a8/0x1a9
are not read). Standing up (X, or any movement) sends `LeaveSit` (37) and `N3Msg_StopCamping`'s 0x79; the server answers
0x7a and the chat says "Timed logout aborted." (`CancelCampMessage`). When the server closes the session the game closes as
`ActivateGameClosing` does: the window positions are written at once (`layout::flush`; the open windows are already kept
in `open.ini` as they change) and the launcher shows the character screen. Sitting characters, ours and others', are drawn
in the sitting-down animation held (action 0xd5 `ground-start`; the `idle-ground` loop is not played). Tried live with
Sadlee: logged out after the countdown; cancelled by X with the server's 0x7a.

**X and `/sit` only sit.** X (and `/sit`, a command of this client's own: the original's list has none) sits the character
on the ground (`SwitchToSit`; a fight of one's own stops first) or stands it up (`LeaveSit`); it no longer logs out. Standing
up still gives up a `/camp` under way, and `/camp` while seated logs out without sitting again (the original sits only a
character not sitting). Tried live: seated 50 s and still in the game.

## Frame-rate work (2026-10-29)

`MAO_PROFILE=1` now also prints, every five seconds, the frames per second and each stage's average and worst time: the
frame's `update` (the network's news, the characters' upload), `acquire`, `scene`, `draw` (with the renderer's own
`render-setup`, `render-shadow`, `render-scene`, `render-post`, `render-submit`), `ui`, `paint`, `present`, and the network
thread's `net-iteration` / `net-receive-*` (`prof.rs`). `MAO_DRAW_GROUPS=1` prints the draws and shadow casters each frame.

Measured live (Sadlee, 2560 by 1440, no vsync): 55 fps, with `queue.submit` taking 7 ms a frame. Halving the render scale
changed nothing; leaving out the shadow pass gave 134 fps. The sun's shadow map drew 1 212 parts every frame (the
playfield within 112 m all round, half of them plants) against 363 in the camera's view. Since only the playfield throws
those shadows (the characters have the original's blob shadows), the map is now kept while it still holds and drawn again
only when the player is 8 m from where it was drawn, the sun has turned (cosine under 0.99999), shadows were switched, or
the playfield changed (`ShadowCache`; `MAO_SHADOW_EVERY_FRAME=1` draws it every frame as before). Now about 165 fps
standing and 140 walking (one 9 ms frame per 8 m). Checked by `a_kept_shadow_map_draws_the_same_shadows`: a pillar's
shadow drawn afresh and from a map kept 5 m back are the same picture.

The network thread handled one message per pass of its loop (a pass is about 13 ms: the wait for the next message and the
frame's own work), so at most about 75 messages a second; messages that have already come are now handled in a row (up to
200) before the frame's work, unless a zoning is due (`Connection::try_receive`).

The characters (every network update uploads them again, about 24 times a second) now go into the last update's GPU buffers
and bind groups where a part has the same sizes and texture (`Renderer::reupload_overlay` / `reupload_follow`): the
overlay's upload went from 3.6 to 1.7 ms and the player's from 1.2 to 0.4 ms, the worst frame from 7 to 4.5 ms.

**Soak test (20 minutes, live, Sadlee, `MAO_AUTOPLAY` repeating Tab / attack / follow / use / loot every 40 s).** No
disconnects or errors; 26 kills, 26 corpses opened (4 with items, all taken); memory 302 MB at the start, 330 MB from the
sixth minute on and flat to the end; 55 to 175 fps by the view (the slow windows were while builds ran beside it). Tab in
that field also picks the guards and guides (another side than a neutral's, as `N3Msg_GetCloseTarget` counts it); attacking
them does nothing, and Q while "fighting" one stops instead of attacking the next: the original's toggle.

## Stopping and zoning (2026-10-29)

**The stop shuffle.** The original never plays a movement animation's start (before `loopstart`) or its tail (after
`loopend`): `ForwardStartTransitionAction_t` (Gamecode FUN_1006ef8d) sets the walk's or run's loop, segment 2, at once, and
the stop (`ForwardStopTransitionAction_t` FUN_1006f0ac → FUN_1006c5ae → FUN_1006c370) sets the idle's loop; the mesh's
blending eases each change. The client played the first step, the loop, then jumped from wherever the cycle was to the tail
(`motion`, now the original's). Also one's own character counted as moving for 90 ms after its last step (walking in place
for that long; now `World::own_moving`, from the keys), and the window glided it on between updates (every ~40 ms) for up to
0.12 s and stepped it back when the stop came (up to ~30 cm at a run): a stop now sends the window an update at once.

**The camera on zoning.** The original makes a new camera whenever the character enters a playfield
(`n3EngineClient_t::SetMainDynel` → `CreateCamera`, N3.dll 0x10007842): at the character's place plus (0, 2, −1) turned by
its rotation (2 up, 1 behind), at rest, and it steers out to the preferred place from there. The client kept the old camera,
which flew from the old playfield's place to the new one. Now `SteeringCamera::create` does the original's placement on
every new playfield (the first one too). (Not ported: `SetMainDynel` also makes playfield 0x380 use the free camera.)

## Loading screen

`loading.rs`, from GUI.dll's `ServerLogin3DModule_t` (`SetupScreen` 0x10017331, `FrameProcess` 0x1001713c): a black sheet over
the whole screen, `cd_image/gui/Default/gfx/ai_loading_login.png` scaled from 1024 by 768 to the display (kept in
proportion here) in the middle, text.mdb's `AO_Loading` ("..Anarchy Online is loading..") in 0xdddddd 50 pixels above the
bottom. Entering the game the picture and text fade in over 4 s on the black; when the playfield is drawn all of it fades
out over 7 s (the original's timings). The client's own additions: the picture is tinted blue (each channel multiplied by
`TINT` = 0.35, 0.6, 1.45), and a zoning shows the screen too (at once, fading out over 1.5 s when the new playfield is
drawn). Not ported: `welcome_to_rubika.jpg`, which the original shows instead after a character was made. Preview:
`MAO_UI_PREVIEW=loading mao-client <game> map 566 --image out.png`.

## Character creation (step 1: the protocol, 2026-10-29)

From Interfaces.dll's `Client_t` and MessageProtocol.dll's `CharacterData_t` (`mao_net::create`): making a character is
system message 0x0f to the login server, the `CharacterData_t` as `WriteStream` writes it (version 4, id 0, an empty
playfield proxy, "not created"; then the `CharacterInfo_c`, version 5: the name, breed, sex, profession, level 0, "area
unknown", not banned, head, height, width) and an int 0; asking for a name is 0x55 with the breed, sex and profession. The
server answers 0x11 with the new character's id, 0x10 with a refusal code, 0x56 with a suggested name (a `u16` length).
GUI.dll's rules are in `charcreate.rs`: the screens' seven breed choices and fourteen professions as the game's numbers,
the heads (mesh instances of `head_<breed><sex>NN.abiff`, type 0xf6951, with the African and Asian Solitus sets; the
original's counts without the expansion flag), height 90/100/110 and build 0/1/2, the name's rules (4 to 12 characters,
letters, digits only at the end after the fourth; a capital then small letters) and the refusal codes' texts (category
600: 8 CantCreateMoreChars, 30 NicknameTaken, 31 NicknameInvalid ...).

Tried live: `mao-client net <game> suggest 4 4` got "Nath"; `mao-client net <game> create Maotestone 4 4 0 1 1` made an
Opifex male soldier (id 2063341223), listed at level 1. Next: the launcher's New Character screens.

**Step 2: the launcher's New Character screens.** The character screen's New Character button opens four steps in a
window at the left, with the chosen breed standing in the 3D preview: **Breed** (the seven choices; text.mdb's
`BreedHelp`, `ChooseBreed`, `<Breed><Sex>Selected`, `<Breed>Description`), **Profession** (the fourteen in the original's
order; `<Profession>Selected` and `Description<Profession>`, the Meta-Physicist's being GUI.dll's
`DescriptionMetaPhysicists`, their `<font>` tags and entities taken out), **Appearance** (`SelectAppearance`; the head by
number among the breed's heads, height short/normal/tall, build small/medium/large), **Name** (the name, Suggest Name,
Create). Create checks the name as the original does, logs in on a connection of its own, sends the creation and shows a
refusal's text (category 600); once made, the list is fetched again with the new character selected (the original logs it
straight in). Not ported: the original's 3D creation scenes and camera, the head shown on the preview, the intro.
Preview: `MAO_LAUNCHER_DEMO=1 MAO_LAUNCHER_CREATE=<0-3> MAO_LAUNCHER_SHOT=out.png mao-client <game> play`.

**Step 3: after Create.** As the original (`Client_t::ProcessMessage` 0x11 → `LoginCharacter`), a character just made is
played at once: the list is fetched again and the game starts with it. Its loading screen is GUI.dll's
`welcome_to_rubika.jpg` two times in three (`SetLoadingScreen(rand() % 3)` at the creation, types 1 and 2; the game is
told with `MAO_NEW_CHARACTER=1`), tinted blue like the other. The preview wears the chosen head (the look's gear at
attachment 0), the build's body (`<breed>_<sex>_thin.cir` / `_fat.cir`, as `CCCharacter_t::ChangeMesh` names them) and
the height. Tried live: Maotestone's first login reached playfield 6553 (the new players' area) and stayed connected.

## Moving as the original's vehicle (2026-10-29)

The character is the original's steered vehicle now (Gamecode `CharVehicle_t`, Vehicle.dll `Vehicle_t::Run`):

- **Speeding up and stopping.** The steering (FUN_10071537, on the vehicle at 0x10161c48) pushes along the facing with the
  maximum force, forwards or (times −1) back; FUN_1006f9eb sets that force to `2 × mass × the top speed` (mass 10), so the
  character reaches full speed in half a second; the step adds `force / mass × dt` to the velocity and keeps it under the top
  speed. The stop transition (FUN_1006f0ac) and a change of direction (`SetDirection`) call `Vehicle_t::Halt`: the stop is
  immediate. Turning while running bends the path: the velocity swings round to the new facing at that rate
  (`vehicle_step`). Sidestepping is still added as before.
- **Turning by key** (FUN_1006ca4f setting the vehicle's +0x368, the rotation steering FUN_10071795): `TurnSpeed` (stat 267)
  / 11000 radians a second, halved while moving; with the stat at 0, 3.5 standing and 1.5 moving (`turn_rate`; the client
  turned at an invented 2.0).
- **Walk and run** (`MOVEMENT_TOGGLEWALK`, default key Backspace as the shipped `CharPrefs.xml` has it (Numpad /
  until 2026-11-18), rebindable as `toggle_walk`): walking is 1.5 a second
  both ways (mode 2); the switch is sent as the movement codes 0x18 (walk) and 0x19 (run); one's own character plays the
  walk or the run by the mode, as FUN_1006c370.
- `/follow` now starts again only a metre past its distance (it started and stopped every step at the edge once starting
  took half a second).

Tried live (Sadlee): walking measured 1.47 a second, no corrections from the server. Still the client's own: the report
timing (every 0.4 s while moving), the jump's numbers, collision, swimming.

**Reports as the original** (`CheckMotionUpdate`, Gamecode 0x10018f20): a full report (0x16) after 5 s in motion without
another movement message, or after 0.25 s when the facing turned more than 0.17 rad since the last message other than by the
turn keys (mouse steering, a walk to a place); not on the heels of a start or stop sent the same moment. The client used to
send one every 0.4 s in motion (`MAO_OLD_REPORTS=1` brings that back). **The server remembers the movement mode** (walk or
run) between sessions and checks the reports against its speed: after a walk-mode test it pulled a running character back
to where it had logged in. The client now tells the server its mode (0x19 run, or 0x18) on entering every playfield. Tried
live: a 30 m run with turns, reports at 5 s and 10 s, every place accepted.

**Collision from the original's surfaces** (`mao_formats::surface`). Each zone of a playfield has a collision surface,
database type 0xf424d with the zone's id as the instance (the playfield in the high half), which N3.dll's
`n3Zone_t::LoadSurface` (0x1001a947) reads as an `n3SurfaceResource_t` (`ReadBlob` 0x10016179). Every one in the game is
version 5 (`ReadVersion5` 0x10015fc1): behind the 12-byte head a range-coded KD tree (FUN_1002f1b0, the static library
city.dll and Collision.dll also link, ported from the city.dll drop-in's `read_v5` and `ao_kdtree::coder`): precision
0.01, the volume and node counts, the quantized box, each volume's mesh, the marker 0x5a, the nodes, then an optional
teleportal. The client keeps only the volumes' triangles (y up, turned to the viewer's z up) and builds the walls and
floors from them in place of the props' visible meshes, which stay the fallback for a playfield without surfaces (a
generated dungeon) or with `MAO_VISUAL_COLLISION=1`. All 237,877 surfaces decode (42 million triangles; `mao-db <game>
surfaces all`, or a playfield's number in hex). They give closed wall rings where the visible meshes had gaps, and leave
out foliage and decoration without the opacity guess. A blocked step now also slides along the walls in the way (the
step less its part into them, `Collision::wall_normal`) before a single axis is tried; the original's own response is in
N3's position callbacks, not yet read. `MAO_COLLISION_PNG=out.png mao-client net <game> area <playfield>` draws the
blocked places from above (`MAO_COMPARE_COLLISION=1` logs both sources' sizes). Tried live (Sadlee, playfield 952):
walked into a wall and around a rock, every report echoed back by the server unchanged.

**Swimming** (Vehicle.dll `Vehicle_t::EnsureSurfaceAlignment` 0x1000d1aa, liquid mode 0, and Gamecode's `CharVehicle_t`):
the liquid at the character (`liquid.rs`, `GetLiquidSurfaceHeight`); with the feet under its surface plus 0.1, water more
than 1.19 deep (surface to ground) is swimming, the feet held 0.01 under the surface; shallower is wading, and walking out
or wading ends it. The change of the vehicle's in-liquid flag makes the character ask for the swim mode (movement 0x1a,
`SwitchToSwimModeTransitionAction_t`; FUN_10044aac) or to leave it (0x23, back to the walk or run), which the server
hears. Swimming's speed is mode 4 of FUN_1006f9eb: `RunSpeed / 440 + 3` within 1.5 and 8, both ways; no jumping in the
water; the animations `swim` (0x85, played at the reference speed 3) and `idle-swim` (0xc3). Other characters swim on
their own 0x1a until their 0x23. Casting, attacking and social actions are refused while swimming (the existing
`Caster.swimming` checks, now fed). An arrival in deep water takes no ground offset from its height (it used to lift
the ground to the surface and walk on the water). `MAO_COLLISION_PNG` draws swimmable water blue (wading light blue).
Tried live (Sadlee, the backyard pond of playfield 952): in at the edge (surface 13.02, feet 13.01), about 3 m/s
in the water, out again with the 0x23, and a login in the pond starting swimming; the server echoed the 0x1a and the
surface heights without a correction.

**Ways out** (`n3Zone_t::IsPosInTeleportal`): a zone's surface may end with a teleportal, `PortalArea_t` (FUN_1001b473:
an outline of up to 999 corners, coded unsigned and scaled 0.01, then the localizer whose low half is the destination
playfield); 8,638 in the game, all decoding to the stream's end mark 0x79 (after the KD tree's nodes, now walked too).
Standing inside one's outline (FUN_1001b21a, crossings on the ground plane) starts the teleport try, as a door does.
**Doors** now start it only within 0.5 m of their place: live, a report 1.1 m (and 2 m) from the backyard door was
ignored and one 0.24 m from it zoned Sadlee to playfield 3132.

**The original's vehicle** (`vehicle.rs`). Where the playfield has collision surfaces the character now moves as
Vehicle.dll moves it. `Vehicle_t::Run` (FUN_1000e3d3) takes steps of at most 0.4 s: gravity −20 in the air, the
vertical speed held within 50, the move by the steering's velocity. After each, `Vehicle_t::EnsureSurfaceAlignment`
(0x1000d1aa) settles the body:
- a sphere of radius 0.4 centred 0.4 over the feet is swept from the last place along the move (FUN_1000b2e5, ten tries at
  most): rays of the radius to each side centre it between near walls, three rays (the centre's and half a radius to
  each side) find the nearest face ahead, the sphere stops a radius short of it and slides along it (a wall met going up
  is laid flat and the radius grown; a slide may not climb a vertical face);
- the ground under the feet is the playfield's closest point (`n3TilemapSurface_t::CalculateClosestPoint` 0x10018f0c: the
  tile-map's ground, or a zone's floor found within 0.3 under the ground's height over it, raised to 1.2 under a
  liquid's surface) and three rays down round the feet (0.04 out at 0°, 135°, 225°) from over the highest point reached;
- within `0.48 + 1.1547 × the step` over the highest of these the body stands, snapped 0.01 over it, unless the rays'
  face is steeper than 60° (normal y under 0.5); else it falls (and lands when it stands again: the jump's stop, 16, is
  sent only after a jump, as `LandNow`'s callback FUN_1006f442 does);
- then the liquid (swimming, below) and `VetoPosition` (0x10018a7c: held 0.1 inside the map, between 0.01 and 1999.9 high).
The playfield's side: the zones' surfaces (`mao_formats::zones`, the KD records' segment test FUN_10012014, front faces
only) and the tile-map's ground, now planar per tile triangle with N3's split (`Terrain::surface`, FUN_10017800 over
FUN_1001769d), stepped tile by tile for segments (`GetLineIntersection` 0x10018b72); its heights are the server's exactly
(the backyard 14.0 against the server's 14.01, 3132 25.0 against 25.01: the 0.01 is the vehicle's), so the ground offset
is gone on this path. **The jump** is the original's (CharVehicle_t FUN_1006ff32, FUN_10058994): `(Strength + Agility) /
200 + 1` metres high (at least 0.5, the sum held to 800 unless stat 215), less what a ceiling 100 up leaves (two body
scales and 0.1 clear), at `√(2 × h × 20)` up; it replaced the invented 7 m/s and gravity 10. The client's old steps (the
props' triangles, the per-axis slide) stay for playfields without surfaces and noclip. Tests: walls, sliding, steps (0.3
climbed, 1.0 not), ramps (30° walked, 70° not), a jump; `mao-client net <game> simwalk <pf> <x> <z> <heading> <s>` runs
it offline (Old Athens: up the hill, over a low bevelled object, held by the wall; into the lake swimming at 32.09 under
32.10, out at the shore), `net <game> ground <pf> <x> <z> [y]` shows the ground queries. Live (Sadlee, 3132): walking at
the server's heights, a jump (15 then 16 0.7 s later), sliding along the building's front; every report echoed unchanged.
**Interiors** (`n3RoomSurface_t`): a room playfield's surfaces hold its walls and furniture but not its floors, which
`CalculateClosestPoint` (0x10013ee6) takes from the rooms' tile-map heights; the vehicle now gets the rooms' floors as
triangles from the same four samples a cell's floor is drawn on (`rooms::floor_triangles`), split as the original splits
them (bit 0x4000 of the cell word before). Simulated in the shop 1180: on the floor at 6.01, held by a wall, along it and
up a ramp to 12.01. Not ported: the room surface's own transforms (the shop's surfaces are already in world places),
`VetoPosition`'s room transitions, room playfields without surfaces (missions keep the client's old steps), the
vehicle's turn (orientation, FUN_1000c616).

**Text fields** (`gui/textinput.rs`, `gui/edit.rs`). The skins' `TextInputView` elements (mail, trade, market deposit,
looking for team) are GUI.dll's `TextInputView_c` now: the XML constructor (0x1014928a) reads `label` and
`label_position` (left, right, above, below, inside; above otherwise) and `Initialize` (0x10148aff) puts an inset border
(`BorderView_c(rect, "", 0, 4)`: the `GFX_GUI_INSET_*` pieces) round an editor made from the same element (`_editor`, its
feature flags plus 0xd: typing, mouse, selection) 3 in from the sides and 1 from top and bottom, with the label beside it 5
away, over or under it in a row with a spacer, or drawn inside at 0.7 opacity while the field is empty and unfocused
(`UpdateLabelView`). Editing follows `TextRenderer_c::KeyDown` (0x10164224) and `TextInput` (0x10163496): Enter is the
field's Enter (a new line with Shift or `TVF_ALT_ENTER_MODE` in a multi-line field), Backspace (Ctrl: back to the word's
start), Delete, Home/End (Ctrl: the text's ends), Left/Right (Shift selects, Ctrl by words), Up/Down a line in a
multi-line field, paste (Ctrl+V) and copy (Ctrl+C, not from a password); `TVF_NUMERIC` keeps only digits; the cursor is a
pixel-wide line blinking every half second (`_EnableCursor`, `_ExtrendCursorFlash`); a click places it and a drag selects;
a single line scrolls to keep it in view. The view tree has keyboard focus: a click on a field takes it, Tab moves to the
next field, Escape or a click elsewhere drops it, and while a field has it the game's keys go to the field
(`gui::host::keyboard_taken`). The field's text is plain characters (the original escapes `<>&` in what is typed, which
amounts to the same). Home/End go to the ends of the line between new lines, not of the wrapped line; the selection
colour is the client's. `MAO_TYPE=<text>` types into the preview after `MAO_CLICK`'s clicks.

**Scroll views in the XML views** (`gui/scroll.rs`). `ScrollView` and `ScrollViewChild` elements (the research
windows, the skills window's lists, the character selection) are GUI.dll's `ScrollView_c` and `ScrollViewChild_c` now:
the bar modes (`never`, `always`, `auto`, `auto_reserve`), `scroll_client`, the client sized to at least its frame and its
minimum and at most its maximum and moved by the scroll (`LayoutChilds` 0x10143dd4), its minimum without the scrolled axes
(0x10143ca5), the scroll view's preferred size with the bars' room (0x10142a4b), the bars as wide as the skin's arrows
(`GetDefaultWidth`) a pixel right of and below the client (the vertical of `GFX_GUI_SIMPLE_SB_UP`, `_DOWN`, `_SLIDER_VERT`,
the horizontal of `_LEFT`, `_RIGHT`, `_SLIDER_HOR`, as `_UpdateScrollBars` makes them), drawn and handled as `chrome::scroll`
does (arrows 5, page 20, knob drag), the wheel 10 a notch (`WheelMoved`, step `_DAT_101a98ec`), and with snapping
(`SetSnapToScrollWheelStep`, which the multi-list view turns on with a row's step) the notches add up and the scroll lands
on a multiple of the step. What lies outside a scroll child is neither drawn nor clicked (the host clips by every clipping
ancestor).

**Drop-down lists and the pop-up menu** (`gui/dropdown.rs`, `gui/popup.rs`). The skins' `DropdownMenu` elements (looking
for team, the Notum Wars signup terminal) are GUI.dll's `DropdownMenu_c`: a three-piece box (`GFX_GUI_DROPDOWNMENU_LEFT`,
`_MIDDLE`, `_RIGHT`, in colour code 0x1000000) with the chosen entry's text 6 in from the sides and 3 from top and bottom,
as wide as the widest entry (70 with none), and the arrow beside it (`_CLOSED`, `_OPEN` while the list is open); a press
opens the list 5 right of the box's left edge and a pixel under it, choosing an entry selects it (`SelectByIndex`, the
window hears `Signal::Selected`); the first entry added is selected. The list is `PopupMenu_c` (`Go` 0x1013c13a): the
inset frame over `GFX_GUI_WINDOW_BACKGROUND`, the entries 3 apart and 3 in, white (0xa0a0a0 disabled), the one under the
pointer on a 0x2020c0 band, a click outside or Escape closing it, Up/Down/Enter choosing from the keys, kept inside the
screen and at most its height less 100. Views draw such things over everything through a new `overlay` step of the
host. `MAO_DROPDOWN_ITEMS=a;b;c` fills a preview's drop-downs.

**Radio button groups** (`gui/radio.rs`). The skins' `RadioButtonGroup` elements (the mail's delivery mode, research)
are GUI.dll's `RadioButtonGroup_c`: a radio button turned on turns the group's others off (its buttons at any depth,
`_ClearButtons`) and the window hears the button's value (`RadioButtonActivated` 0x10140453, `ValueChanged`:
`Signal::Value`); the group's value is the first button on (`GetValue` 0x101402f5) and setting it turns on the buttons of
that value (`SetValue` 0x101405cb); a button on stays on when clicked again.

**Tool tips** (`gui/tooltip.rs`). What the pointer rests on for half a second shows its tip as GUI.dll does
(`WindowController_c::UpdateToolTip` 0x10157adc starts a 500 ms timer when the tip under the pointer changes; `Render`
opens a `ToolTip_c` at the pointer holding an `InfoContainer_c`, FUN_101494d9): the title in font 5 (in `Default` when
alone), under it the body word-wrapped two wide to one high (`SetAspectRatio(2)`), framed in colour code 0x3000000
(`Hover`, 2 wide with a body, 1 without). It serves the XML views' `tooltip` and `tooltip_body` (text database names
with `#`) and the control center's menu entries (they had egui's hover text). The frame's pieces are drawn as lines round
a dark ground, the title ruled off from the body; the original's exact surface sizes are not followed. **Wrapped lines**
of every view's text now start at their first word's text, as `TextRenderer_c::_RenderLine` draws a line from its start
in the buffer: the space before a word that moved to the next line is neither drawn nor counted there (it was drawn as a
one-space indent).

**View selectors** (`gui/selector.rs`). The skins' `ViewSelector` elements (16: the skills window's pages and groups, the
mail window's To/From and Send/Take rows, the city terminal, the mech selection) are GUI.dll's `ViewSelector_c`: one of
its views shown at a time over its whole bounds (`SetValue` 0x10150236, `FrameSized` 0x1015000e), the first shown at load
(`InsertView` 0x1015050e), its preferred size the largest of its views' with `use_max_size` (on by default), else the
shown one's (`CalculatePreferredSize` 0x10150074). They used to draw every page at once (the skills window's intro over its
skill page). `selector::set_value` / `show_named` switch the page.

**Item slots** (`gui/itemslot.rs`). `ItemSlotView` (the market deposit, mail, trade, the tradeskill kit) is GUI.dll's
`ItemSlotView_c`: `Initialize` (FUN_10042af9, from the XML constructor FUN_10042d63 with `iconview_flags`) draws
`GFX_GUI_MULTILISTVIEW_SLOT_48_CLOSED` (0x11d) over the view, the slot as big as that picture, and centres an
`ItemIconView_c` (FUN_1003f67b, 48 by 48) in it, the left and top halves of the room rounded down. An `ItemIconView`
element alone is the icon without the slot. The window sets what a slot shows (`itemslot::set_item`: an icon texture,
and the item's name as the slot's tip); a click on a slot is its `Clicked` signal. The market deposit window now uses these
slots and the XML's real `Cash` field (`TextInputView`, `TVF_NUMERIC`) instead of drawing its own boxes and an egui text
edit over them. Not ported: the icon flags' meaning, the open slot picture (0x11e) and dragging from a slot.

**Scrolling text** (`gui/scrolling.rs`). `ScrollingTextView` (the trade and shop windows' `PartnerName`) is GUI.dll's
`ScrollingTextView_c` (FUN_10008da8): one line in the normal font, centred upright, and centred across when it fits.
A text wider than its view starts at the left, waits `initial_scroll_delay` (1.5 s), then moves 0.4 px every
`scroll_delay` (20 ms) until its end shows, waits `turn_scroll_delay` (0.7 s) and comes back, and so on (FUN_1000871e,
FUN_1000886a, the timer at 0x10008853). `scrolling::set_text` gives it its text, optionally a colour and flashing
(FUN_10008a02: shown `flash_on_time` 0.6 s, hidden `flash_off_time` 0.3 s, FUN_100089b4). The shop window's partner name
is now this view instead of text drawn over it.

**Text buttons** (`gui/textbutton.rs`). `TextButton` (the character selection's name buttons) is GUI.dll's
`TextButton_c` (XML constructor 0x1000507a, `StateChanged` 0x10004ca2): a button that is only its text, a `TextView_c`
made from the same element (its `font`, e.g. `HUGE`). It shows `raised_text` in `raised_color`, under the pointer
`hover_text` in `hover_color`, and while a toggle button is on `pressed_text` in `pressed_color` (each defaults to `text`
and `color`; no colour means white). Disabled, the colour's channels are two thirds (`ButtonBase_c::GetDisabledColor`
0x10129421). Its preferred size is its text's. The launcher's character screen still draws its own list; the XML item
now shows the button as the original does.

**Combo boxes** (`gui/combobox.rs`). `ComboBox` (the login window's user names) is GUI.dll's `ComboBox_c`
(0x10002454, `Initialize` 0x10002216): a `TextInputView` whose inset border becomes a row of the editor and an arrow
(`GFX_GUI_COMBOBOX_CLOSED`, `_OPEN` while the list is open, 5 from the border's right). A press on the arrow opens a
`PopupMenu` of the items at the editor's left, 3 under its bottom (`SlotMenuOpening`, `SlotMenuOpened`); a second press
closes it. Choosing an item puts its label in the field, and a changed text is the editor's `Edited` signal
(`SlotMenuItemSelected` 0x100020f2). Items are a label and an id (`combobox::append_item`, `clear`). The preview's
`MAO_DROPDOWN_ITEMS` fills combo boxes too. The launcher's login dialog still draws its own list.

**Sliders** (`gui/controls.rs` `Slider`). The `Slider` elements (the research and tradeskill windows) and the options'
sliders are GUI.dll's `Slider_c` (0x10144647, from XML 0x1014485a: `min_value`, `max_value`, `default_value`). The track
is `GFX_GUI_CONTROLCENTER_AGGDEF_SLIDER_BACKGROUND` stretched over the slider, and the knob is
`GFX_GUI_CONTROLCENTER_AGGDEF_SLIDER` in `Default`, centred upright and placed by the value (`_Layout` 0x101442fe). Only
the knob is grabbed: a press on the track does nothing, dragging the knob sets the value (held to the range), and
letting go is the change. It is as tall as the knob and any width. It used to be drawn with option-slider pictures the
skins don't have, and a press anywhere on it moved the value. `gfxid_slider`/`gfxid_background` (picture numbers) are
not read; no skin gives them.

**Power bars** (`gui/powerbar.rs`). The XML `PowerBar` (the limbo countdown, progress dialogs, the raid window's health
and nano) is GUI.dll's `PowerbarView_c` (0x1013f846, `Initialize` 0x1013edfc). The `bg_gfx` background is stretched over
the bar, between optional `left_gfx`/`right_gfx` end caps. On it the `full_gfx` picture is drawn at its own size, centred
in the background's size and cut from the bar's start to the value: the far edge is floor(size × value), in the
`direction` `right` (default), `left`, `up` or `down` (`Recalculate` 0x1013e946). The value starts at 1 and is held to
0–1 (`powerbar::set_value`, `SetValue` 0x1013f283). `gfx="small_blue"` picks the small blue pair. The labels are not
drawn; no skin gives them.

**The mini toolbar** (`gui/controlcenter.rs` `MiniToolbar`). The control center's `CCMiniToolbar` is GUI.dll's
`MiniToolbar_c` (FUN_10072ced). It holds ten toggle buttons (`ToolbarButton_c`, FUN_1007316d) side by side for
`wear_window`, `specialaction_window`, `knowledge_window`, `mission_window`, `team_view`, `map_window`,
`friends_window`, `nano_window`, `stat_window` and `ncu_window`. Each has its tip (Wear, Controls, Knowledge, ...) and its
`GFX_GUI_SB_*_OFF` picture, lit (`GFX_GUI_SB_*`) while its value is on or under the pointer. A click flips the value,
which for the client's windows opens or closes them. It shows when `cc_mini_toolbar`, `cc_rollup_panel` and
`cc_section1` are all on (`cc_mini_toolbar` is off by default). `MAO_DVALUES=name=value;...` sets values for a test.

## Talking to NPCs (2026-10-29)

Using an NPC (E, or a double-click) sends `KnubotOpenChatWindow` (`3b132d64`) to it, as aosharp opens a conversation
against the live servers (the original has no `N3Msg` for it; the plain use message is still sent for everything
else). The server's `Knubot` messages fill GUI.dll's `NPCChatWindow_c` (`npcchat.rs`, FUN_100596c2): the conversation
(a text view of the original's HTML, each line coloured by its kind as FUN_10058ee7 does: `CCNPCChatText`,
`CCNPCOOCText`, `CCNPCChatQuestion`, `CCNPCChatSystem`, `CCNPCChatEmote`, `CCNPCChatDescription`), the separator
(`GFX_GUI_NPCCHAT_SEPERATOR`), the answers (FUN_100595ae: a bullet and a link to the answer's number) and the button bar
(FUN_10059eee): Desc (`KnubotNPCDescription`), Info (the NPC's `charid://` page), Give and Shop (greyed: the NPC trade
is greyed: the server's shop for an NPC is not known). A clicked answer is `KnubotAnswer` with its number from 0; closing
the window `KnubotCloseChatWindow`. Give (`N3Msg_NPCChatStartTrade`: `KnubotStartTrade` with no slots) asks the NPC for
its trade; the server's `KnubotStartTrade` (slots, text) opens the trade bar (`NPCChatTradeBar_c`, FUN_10057d8b) over the
conversation: the text, "GIVE ITEM(S)" slots an inventory item is dragged onto (`KnubotTrade` action 0 with `(0x68,
slot)`; a click takes it out, action 1), "GIVE CREDITS", V and X (`KnubotFinishTrade`: declined or not, the credits).
`KnubotRejectedItems` is told in the conversation. Preview: `MAO_UI_PREVIEW=npcchat MAO_NPC_TRADE=1`.
All are sent from the character with an `i16` 2 and the NPC, as aosharp sends them. Preview: `MAO_UI_PREVIEW=npcchat`;
`net <game> talk <char> [npc name]` opens a chat headless and prints the server's answers. Not tried live (no NPC was
near the test character).

## The mission journal (Ctrl+4) (2026-10-29)

Ctrl+4 (`mission_window`) is the journal, GUI.dll's `MissionView_c` (`missionlog.rs`), not the booth's window
(`MissionSelectionView_c`, which a booth's use opens as `missionselection`). The server's `QuestFullUpdate`
(`465a4061`) is the whole list (its last byte belongs to the message, not to each `Quest`: the schema had it inside
`Quest`, which shifted every mission after the first by a byte), and `Quest` action 1 drops one. Its menu
(FUN_100d3123) is Look at Mission, Upload to map, Update Team Members (a team mission's only, greyed) and Delete Mission
(asked first, then `Quest` action 1). Uploading (FUN_10067d5f) takes the
first action's playfield and place as the waypoint and says "Waypoint uploaded to map and compass." and either "The
waypoint is shown as a white dot..." or "Mission is not in this area..." (the texts are text.mdb's category 10000);
the waypoint is a white dot on the planet map and a yellow mark on the compass in its playfield. Preview:
`MAO_UI_PREVIEW=missionlog MAO_CAPTURE=capture-225436.log`.

**The window as the original's (2026-11-10).** It was a grid of icons; the original's is a list. `MissionView_c` holds
one `MissionListView_c` (FUN_100d3df7), a `MultiListView_c` in its list mode with three columns named from text.mdb's
category 10000: "Icon" (16 wide), "Name" (200) and "Tab_Remaning" (55), the view 167 by 112 until the player sizes it
(FUN_100d0177). A row (`InventoryListViewItem_c`, FUN_100d2c3b, 15 high) is the mission's icon in a square as tall as
the row (`multilist.rs` now draws a row's icon in the icon column, FUN_1003edf2), its name, and the time it has left as
hours and minutes ("%02d:%02d", FUN_1003d7d0's column 15; 00:00 with none), set again every second. The headers sort
and size the columns as in the mail list. The selected row is not the clicked one but the active mission
(FUN_100d3366): the one last uploaded to the map, until it is gone. A click shows the mission in the info view as
`itemid://56003/<instance>` (FUN_100d2cfa), and the page is the original's (FUN_100360d5, `MissionLog::page`): the
name, "Description:" and the text, the time left in days, hours and minutes, a team mission's rules, then the reward:
the credits, the experience, and the reward items as icons that link to their own pages, or that there is none (the
labels and formats are text.mdb's category 506 by name: `RealtimeLeft`, `DayHourMinute`, `TeamMissionRules`, `Reward`,
`Cash`, `NumCredits`, `Experience`, `NumPoints`, `ItemReward`, `NoReward`).

The time left is `N3Msg_QuestRemainingTime` (Interfaces.dll 0x1000a8ac): of the mission's actions whose deadline has
not passed, the last one's deadline less the server's time (`N3Msg_GetServerSynceSystemTime`, the clock the `GameTime`
message sets: `gametime::server_now`). The deadline is `QuestAction_t` +0x5c, read by Gamecode FUN_100acba0 as the
word after the action's sixth identity: the schema had those four bytes as a text ("UnknownHash1"), now `Deadline`.
The 2026-11-08 capture's terminal mission carries 1794356426 there, 48 hours after it was taken. In the same pass the
`Quest`'s `Unknown4` became `Flags` (`Quest_t` +0xa4: bit 8 a team mission, `N3Msg_IsTeamMission`), `Unknown6`
`Credits` and `Unknown8` `Xp` (FUN_100870a8; the capture's values are the terminal's offer's). These four names are
set by hand in `protocol.json`, not by `tools/gen_protocol_schema.py`.

The C `printf` the text database's formats go through (`gui/spells.rs`) now pads to a width ("%2d", "%02d"): the time
left's format needs it.

Seen live (Humility's one mission in the list with its icon) and as a preview; the page was rendered from the game's
texts in the info view's preview. Not done: the columns' widths, order and sort kept between sessions
(`MissionViewConfig`'s `listview_config`) and the active mission with them (`active_mission`, whose place the original
uploads again at the start); "Update Team Members" (`GiveQuestToMembers`, never captured); the experience row for a
reader of level 200 to 220, whom the original pays in shadow knowledge (`GetSKForXP`); a mission with a deadline has
not been seen live, only in the capture. The window opens 168 points wide like the original's, which hides the time
column until it is widened; the width is kept from then on (`sizes.ini`).

**Window sizes kept (2026-11-10, `layout.rs`, `chrome.rs`).** The original's `WindowFrame` is a rectangle, place and
size together; this client kept only the place. A window the player can size (an egui window left resizable, asked of
its `Resize`) now keeps its size in `sizes.ini` beside `layout.ini`, one line a window as there (`name width height`,
the whole window's in points), and starts at it the next time: over the original client's own frame and the window's
own default. What is kept is the room the window gives its content plus its frame, noted by `chrome::window` each
frame (`layout::note_size`), not the rectangle the window was drawn in. The first cut kept the rectangle and two
windows drifted at every start: the perk window lost 6 points each time (its lists stop 6 short of their room), and
the planet map gained 29 (its button rows were guessed too short on the first frame, and a window never shrinks back;
the guess is now too tall instead, so the window is short for one frame). With the room kept, ten windows open came
back at the same sizes over three starts with nothing written in the second and third. The dialogs and the windows
docked in the rollup column keep no size; `MAO_TRACE_LAYOUT=1` prints each size as it is noted. Deleting `sizes.ini`
gives every window its starting size back. Not done: the list views' column widths and sort (`listview_config`).

## The planet map (P, Ctrl+6) (2026-10-29)

GUI.dll's `PlanetMapView_c` (`planetwindow.rs`): the map's levels drawn tile by tile (the 128 px PNG tiles are decoded
on a thread and kept as textures while used, `planetmaps::Tiles`), `-`/`+` and the wheel step through the index's
levels keeping the point under the pointer, a drag moves the map, Character and Mission centre on the character or the
waypoint (the buttons sit under the map in the original's two rows — Zoom in / Zoom out, Character / Mission — and act
the frame after they are pressed); the window starts small at the top left as the original's does (a twelfth and a half
of the screen's width square, at 1.8% in and 5.3% down, the user's shots of a 5120 by 1440 display) and open on the
first start (`open.ini` missing), the list of maps shown only where this planet has more than one; the character is an arrow along its heading (placed as FUN_1004bedd places it), the waypoint a white dot,
and the pointer's place is shown in the playfield's coordinates. The overlay of the walked map moved to O.

**The maps.** As the original's `PlanetMapIndexFile` and `ShadowlandMapIndexFile` (FUN_1004cb15 lists the index files
by their `Type`), the options `planet_map` and `shadowlands_map` in `mao.ini` name an index file under a maps' folder:
the client's own `PlanetMap` folder beside `mao.ini` first, then the game's `cd_image/textures/PlanetMap`. The defaults
are the aftermarket **Saavick's Map of Rubi-Ka 3.3** (`SaavicksMap/saavicksmap.txt`) and **SLmap 2.22**
(`SLmap/SLmap.txt`), copied into the client's folder; every map found (the game's default, Dovves, Shadowlands, and
Saavick's 2x and SLmap Wide) is offered in the map window's drop-down for its planet. A playfield is shown on the
Rubi-Ka map when it has a place there, else on the Shadowlands one. The corner map uses the chosen map too, at its
biggest level of 2048 pixels a side at most. `mao_formats::planetmap::discover` finds the maps; the `planetmaps`
example lists them (`cargo run -p mao-formats --example planetmaps -- <folders> [--image <index> <level> <out.png>]`).
Preview: `MAO_UI_PREVIEW=planetmap MAO_WAYPOINT=566,444,340`.

**The maps were not in the user's folder until 2026-11-15.** The copies of 2026-10-29 went to a private copy of
`%APPDATA%` that only this assistant's tools see, so the game the user played showed the game's own maps: its older
Saavick's (the same index path) and its Shadowlands map, never SLmap. Found when the user downloaded SLmap again
("I downloaded two additional maps for the client please work on integrating them"). Saavick's, SLmap and the Atlas
below are now copied by a process outside that view, and the folder was listed from outside to see them there.

**The Atlas of Rubi-Ka, made a map from its source pictures (2026-11-15).** The user's download is the Atlas'
source (v1.3.1): four Photoshop files, no index, no tiles, no coordinates. What was made of it:

- `mao_formats::psd::flattened` reads a `.psd`'s flattened picture (8-bit RGB, raw or PackBits rows);
  `planetmap::pack` cuts a picture into a level's 128-pixel PNG tiles, column by column;
  `PlanetMap::index_text` writes the index. The `planetmapbuild` example makes a map's folder of pictures:
  `cargo run --release -p mao-formats --example planetmapbuild -- <maps folder> AoRK "Atlas of Rubi-Ka v1.3.1" Rubika
  assets/planetmaps/AoRK.xml AoRK_map1.psd 4 1 360 459 AoRK_map2.psd 54 3 944 1148 AoRK_map3.psd 8 0 3568 4582
  AoRK_map4.psd 8 0 3568 4582`. The pictures and the map made of them are not in the repository; the numbers are.
- The levels: the Whom-Pah overview (384 by 512), the overview with its legends (1152 by 1280), the detail map and
  the grey one of the outdoor encounters (3584 by 4608 each).
- **The rectangles and the coordinates are measured, not the Atlas' own** (its released index and coordinates file
  were not at hand). Saavick's map is drawn over the same ground pictures, so patches of Saavick's big level were
  matched in the Atlas' detail picture (normalised cross-correlation): the Atlas lies 212 pixels left and 12 down of
  Saavick's, and most playfields are drawn a little bigger or smaller (up to 2 %). For each outdoor playfield a
  line was put through the offsets of the patches inside its ground (east and north apart; the ground's size from
  the tile-map, the `pfsizes` example), and the playfield's `x`, `xscale`, `z`, `zscale` follow from Saavick's and
  that line: `assets/planetmaps/AoRK.xml` (54 playfields, numbers only). The two overview levels are the detail
  picture shrunk by 4 and by 10 exactly and moved by (52, -3) and (3, -1) pixels (matched the same way), which gives
  their rectangles; a rectangle's top is counted as `PlanetMap::pixel` counts it.
- **Guesses and gaps.** The towns drawn another way on the two maps had no match and keep Saavick's place moved by
  the common offset (Old Athen, West Athen, Rome's three, Omni-1 Trade, the Omni arena); so do the U-turn
  playfields, which the Atlas does not draw. The fit leaves about a pixel of the detail level (5 m) in most
  playfields, more where a playfield's picture is not one stretch (not measured further). The Atlas' own drawings
  of the alien sectors and the Unicorn outpost (in the western desert) have no coordinates yet, and it has no
  dungeons: there the window shows the map without the arrow. The outlines of every playfield were drawn over the
  three levels and looked at; `MAO_UI_PREVIEW=planetmap` with the Atlas chosen shows the arrow in Old Athen.
  **Not tried live.**

## Everyone is a blue triangle in the Grid (2026-10-29)

In a playfield whose first area is named "Grid" (`PlayfieldAnarchy_t::IsGrid` 0x101222d7: the Grid, 152, and the Fixer
Grid, 4107) the characters' stat getter (FUN_10059399) sets `Features` (224) bit 0x800, and FUN_100524b2 then draws the
character from FUN_10058088's NPC template 0x1863e (99902) instead of its own: a blue triangle
(`mao-client <game> creature 1863e out.png`). Here every character and NPC in such a playfield is drawn with that
template, as a polymorph is (`net.rs`, `World::grid`). Not yet seen live.

## Names behind walls (2026-10-29)

The original's name tags (`TargetingModule_t::HandleNametags`, GUI.dll 0x100268ee) are `VisualSprite_t`s placed in the
world at the indicator's position (FUN_100258b3, while `N3Msg_IsVisible`), so a wall between the camera and a tag
hides it as it hides anything. Here the tags are drawn over the picture, so each frame a tag whose line from the eye
crosses the playfield's surfaces (the zones' collision surfaces) is left out (`nametag::Tag::hidden`).

## The sky stays outside (2026-10-29)

Zoning from outdoors into a shop or other interior kept the outdoor playfield's effect objects (the sky's domes and
clouds) alive, and the next playfield's `start_automatic` started them again, so the outdoor sky was drawn round the
shop. The original deletes the whole FXS instance when a playfield shuts down (Gamecode FUN_100b2972:
`FXS_t::DeleteInstance`, `VisualEnvFX_t::DeleteInstance`) before the new one loads its tweaks; the client now does the
same (`fxs::reset` on a playfield change; test `a_new_playfield_starts_from_its_own_tweaks` with the game's scripts:
Old Athens then a shop).

## Shop names (2026-10-30)

A vending machine's `VendingMachineFullUpdate` (`7f544905`) carries two templates: `StaticInstance` (stat 23), the
machine every shop shares ("Specialist Commerce", whose stats the prices are read from), and `NameTemplate` (stat
446), the item whose name is the shop's ("Basic ICC Armor", "Miiir Headwear"). The original's SimpleItem_t name
(FUN_100809a0, vtable +0x34) is the dynel's own name if it has one, else the `NameTemplate` item's, else "NameMissing";
the shop window's partner name (GUI.dll FUN_100e121e) is that name (or its parent's). The client names the machine's
target and its shop window the same way. `cargo run -p mao-formats --example itemnamed -- <game> <text>` lists the
item templates whose name or description has the text (`"vending machine"` lists the shops).

## The market window's page (2026-10-30)

Closing a web window (the market, the item shop) puts its WebView2 page away rather than dropping it (`browser::Page::hide`):
the keyboard focus goes back to the game window first (`focus_parent`: a page that kept the focus while it went away left
every key press to a window that rang the error bell), and the page stays alive, so WebView2's browser and the site's session
cookie with it. Dropping the last page ended both, and the market then refused the game's ticket a second time ("403 Invalid
session"). Opening the window again for the same request shows the kept page at its home page; another request replaces it,
the old page let go only once the new one is up.

## Moving meshes: doors, Whom-Pahs, machines (2026-10-30)

A mesh's frames can carry `FAFAnim_t` keyframe animations (`mao_formats::model::Keys`, after randy31's `keyanim.rs`:
translation keys x y z t, rotation keys x y z w t, visibility keys, texture keys u-scale v-scale u-offset v-offset t blend);
`model::build_*_animated` gives each part its `Animated` (the frames from the root with their keys), whose `world_at(t)` is the
original's `SetAnimationTime` + `UpdateWorldMatrix` (`anim * TRS * parent`) and `texture_at(t)` the texture matrix
(`u * a + c`, `v * b + d`) visual.rs's `rasterize` sets from the part's own frame.

* **Doors.** As `Door_t` (FUN_1008563d / FUN_10085684 play the mesh's animation forward from 0 or back from its end,
  FUN_10088970 steps it each frame), a door with an animation swings open over it and back when it closes, instead of
  vanishing; its shut pose is the animation's start (the pose it is drawn in). The original opens a door on a character's touch
  (FUN_1007fccc, unless FUN_1007fad8 keeps them out: a lock, an owned building); here the Whom-Pahs' doors (template "Dummy
  Whom-Pah") open as the character comes within 4 m and close 5 s after, the other doors on a use as before. (Since
  2026-11-18 every door by the touch itself: "A door opens at the character's touch".)
* **Outer textures.** A dynel's `OverrideTexture` (stat 336) is put on its mesh's materials whose names have neither "face" nor
  "under" (`n3VisualDynel_t::SetOverrideTexture` → `VisualMesh_t::ChangeOuterTexture`, FUN_10070f1f): from the playfield's
  dynel records (a Whom-Pah door's destination sign; a record without its own takes its item template's, as with the mesh:
  the ICC shops' machines are table records of their shop's template, "ICC Tech Supplies", on the shared "Specialist
  Commerce" mesh) and from the vending machines' announcement, else their `NameTemplate` item's.
* **Statels.** Their parts whose animation moves anything (not the plants, which sway in the shader) are kept out of the static
  scene and drawn each frame within 250 m at the clock's time: the Whom-Pah towers' scrolling glow (`whompa_fx`), propellers,
  holograms.

## The bank (2026-10-30)

A bank terminal's use is answered by the server's `Bank` message (`343c287f`, `BankSlots`): Gamecode's `BankIIR_t` makes the
container `0xdead` (`BankEntry_t`, FUN_10125b1b) and GUI.dll shows it as an `InventoryView_c` named `bank_window`
(FUN_100ccefe). The client shows it in the container window (`loot.rs`, titled "Bank", eight to a row, no "Take all", not
closed when empty). Taking an item (a double click or its menu) is `N3Msg_MoveItemToInventory`: `ClientMoveItemToInventory`
with the item's place, `(0x69, place)` (`BankByRef`, as aosharp's clientless `Bank` names them), to `0x6f`, any free place;
an inventory item dropped on the window is `N3Msg_ContainerAddItem` into `(0xdead, character)`.

**Tried live** (Sadlee in Unicorn Outpost, 4364, at its terminal `(0xc73d, 0xc006110c)`, 1.25 m off): the use is echoed with
a 1 in its second word and the `Bank` message follows (`01 000003f1`: an empty bank). When the server does not have the
character where the client does, the use is echoed with a 2 and the `Feedback` "You are too far away; please move closer!"
(category 110) comes instead, and no bank: that was the failure. The server refuses a move it does not accept (a walk off a
ledge, through a rail) by sending back the last place it took as a full report (`0x16`), every 3.4 s, while it echoes the
refused reports too; the client now goes back there (`own_report`: a report more than 5 m from us that is none of the places
we told it in the last 4 s). Taking and depositing are still untried (the test bank is empty).

**Instances.** In an instanced playfield (ICC's shops, 1180..1187) the server gives the built-in objects its own ids, not the
dynel table's `0xc0NNpppp`: a use of the table's id is echoed with a 2 and nothing else (Humility at ICC's bank: "Using ..."
and no window). The playfield message (`5f4b1a39`) carries them, after the proxy: a `0xc77d` word and three more, the number
of runs, then runs of five words (kind, 1, the first record's index in the dynel table, how many, the first id; the others
count on). ICC 1186: terminals `(0xc73d, 54, 2, 0x58c197d5)` and `(0xc73d, 59, 1, 0x58c197d7)`, vending machines from
`0x1324f4e2`, doors from `0x10c1c178` (the ids the server announces them by). `Message::instance_runs` reads them; a use of
a table object goes to its instance id, the table id's `NN` (its number among the records of its kind) counted through the
runs of that kind (`instance_id`). **Tried live**: Humility's use of ICC's bank went to `0x58c197d6` and the `Bank` message
came (an empty bank). Terminals the server announces itself (`SimpleItemFullUpdate` on a `0xc73d` with a place) are kept
with their names (`terminals`).

The live client's `MAO_AUTOPLAY` takes `usenear=<name>`: target the nearest table dynel with the name and use it.

`net <game> walk <who> <x> <z> [secs]` takes `MAO_WALK_USE=<name>` (use the nearest table dynel of that name on arrival and
print the server's answers) and `MAO_WALK_ZONES=1` (heights from the zone surfaces, as the live client's); `net <game> ground
<pf> <x> <z> <y>` with `MAO_GRID=<x1>,<z1>,<step>` prints a height grid to plan a route by.

**The help assistant, "Ask the guides" (2026-11-12, the user: "Start looking at scraping ao-universe and other related
sites for integrated via a small LLM help window"; branch `agent/help-assistant`).** This client's own; the whole of
it, the sites looked at and what each allows, is in `docs/HELP_ASSISTANT.md`. A window (`helpwindow.rs`; `/open
assistant`, or "Ask the guides" in the Help menu) with a question field: the answer is the best pieces of the help
pages, numbered, each with its page's title, where it is from and a few lines (a click shows the whole piece; "open
the page" a site's page in the system's browser). The pages are the game's own help files, read from the installed
game (`cd_image/text/help`, 110 pages), and the guides of sites the player turns on under "Sources and the model":
today AO-Universe's alone, off by default, its terms shown beside its checkbox, read only at the press of "Read its
guides now", a page every two seconds by the site's robots.txt, into `help/` beside the settings. Nothing fetched is
in the repository or in a package. The ranking is by words (BM25, `mao_help::index`), all in memory. With `endpoint`
and `model` under `[help]` in `mao.ini` (an OpenAI-protocol chat endpoint such as a model server on the same machine,
or Anthropic's with `api = anthropic`; a key only by the name of the environment variable that holds it) the top
pieces are also sent to that model and its answer stands above them, marked as the model's. The core is the crate
`mao-help` (text out of HTML, robots.txt, the crawler, the index, the model's request), with a command-line program
of the same name (`mao-help ask`, `fetch`, `sites`, `stats`). The pages are fetched with the system's `curl`. The
shared files have only hooks: `ui.rs` (`window_open`, `set_window`, the two draw calls), `settings.rs` (the `[help]`
section passed to `helpwindow::configure` / `settings_text`), `vitals.rs` (the menu entry). Seen in a picture
(`MAO_UI_PREVIEW=assistant MAO_HELP_QUESTION=...`, `MAO_HELP_DIR` for the pages' folder) and tried from the command
line, the model's part against a stand-in endpoint; **not tried in the live game, with a real model, or on Linux
and macOS.**

**The mouse on items, and the items' pictures (2026-11-12, the user's list after a play test; `gear.rs`,
`invwindows.rs`, `loot.rs`, `ui.rs`).** What changed, and what of it is the original's:

- **The items' pictures stand on their slots.** Every item stood on a tan square: the icons went through the skin
  tint that paints a character texture's pure green, and the icons' green is their key. It is now nothing
  (`icon_key` in `main.rs`), for every window that shows an icon.
- **A left click takes an item up**, and it stays on the pointer until the next click puts it down: on a wear cell
  it is worn there, on the inventory it goes back, on an open bag it goes in, on a hotbar slot it is set there, on
  the world it is used on what is there, anywhere else it just goes back. A press that moves does the same in one
  stroke, as before. Escape puts it back. This is the original's "new mouse" (`UseNewMouseFunc`): its item lists
  raise the pick-up signal as the left button goes down with neither Shift nor Ctrl held (GUI.dll FUN_10041aa4 ->
  FUN_1004258f), which the inventory answers with `View::BeginDrag` (FUN_100cc6e8). Here the item is taken up as
  the button comes back up, not as it goes down. **There is no use by double click any more** (the first click of
  two has the item in hand); Ctrl+double click still puts an item into an open trade. (Since 2026-11-18 it is
  Ctrl + right click, the original's: "A shop's rows act at one click".)
- **A right click uses an item** (as before: button 2 is the list's use signal, FUN_1004266a). **A bag's use opens
  its window, and the next closes it**, worn or carried: the client closes and shows the window itself and sends
  the server nothing for the second and third use, since it knows what is in a bag it has opened. Where the original
  closes a backpack on its second use was not found (its `InventoryGUIModule_c::CloseContainer` is called when a
  bag is taken up, FUN_100cc6e8): the behaviour is the user's word for the original's.
- **A right click with an item in hand uses that item on the one clicked** (a tradeskill combine, `UseItemOnItem`:
  the item in hand is the source, so the order matters), and the item goes back to its place. Both must be in the
  backpack. Before, letting an item go on another did this; now that only puts it down.
- **Shift+click shows an item's info**, in the inventory, the wear window, a bag, the bank, a corpse.
- **In a bag or the bank a click takes the item up** as in the inventory (the next click over the inventory window
  takes it out); **in a corpse or chest a click takes the item** (the original's `MoveItemToInventory` for a
  container that is not one's own, FUN_100cc6e8). A right click takes an item out of any of them, as before.
- **Shift+T opens and closes the tradeskill window** (`tradeskill_window` in the `[keys]` section; the key is the
  user's word for the original's, the action's name is this client's).

`MAO_ITEM_TRACE=1` prints what the mouse does with the items, and where each of the inventory's items is in the
window's pixels, for a test by script. Tried live with one (Maotesttwo): an item taken up and put down by a second
click and by Escape, used by a right click, used on another (the server said "No combination found!" of two pieces
of junk), a bag opened, closed and shown again by three right clicks. **Not tried:** a worn bag, Shift+click (the
script's clicks hold no keys), Shift+T, an item in hand set on a hotbar slot or put into a bag or a shop by a
click, and everything in a bag, the bank and a corpse.

**The occluder's bodies are not drawn (2026-11-12, the user's screenshots of Jobe: pink and purple see-through
boxes round the buildings; `mao_formats::model`).** Three meshes of the database, `[OCC]8x8`, `[OCC]16x16` and
`[OCC]32x32`, are boxes a level designer stood where a wall or a building hides what is behind it: 774 statels in
six playfields (Jobe Research 83, Nascense Frontier 107, South of Elysium 62 and its test twin 62, the ICC
Shuttleport 51, Adonis City 409). Their one triangle mesh is named `[OCC]` and its material is a half see-through
magenta with no texture, which is what was on the screen. The original never draws them: randy31's
`RTriMesh_t::Process` (0x10049554) tests the first five characters of the mesh data's name for `[OCC]` the first
time it runs, keeps the answer in the mesh, and from then on returns before the mesh is drawn or its frame's
children are processed, unless the debugger's mode has bit 2 (the "OcclusionBodies" render debug). The mesh
reader does the same (`model::is_occluder_name`, in `walk`): a mesh with that name gives no parts, in every window
that draws a mesh. `MAO_OCCLUDERS=show` draws them, as the debug mode does. The original also hands these bodies to
its occluder (`HMOccluder_t`), which leaves out what stands behind them; this client has no occlusion culling, so
nothing is left out here. Seen in offline pictures of Jobe Research before and after (6165 parts, then 6082);
`cargo run -p mao-formats --example occluders -- <game>` lists the meshes and the playfields. Not seen live.

**The modellers' helpers named "invis" are not drawn (2026-11-12, found by a sweep after the occluders;
`mao_formats::model`).** 95 of the 11,228 static meshes hold a triangle mesh whose name has the word "invis" in it
(`invis`, `invis07`, `invis_object`, `invis_colis_stairs`, `invisible_helmet` ...), placed by 5,273 statels: the
blocks that stop a walker (`bsp_horizontal_2x2x05`, 931 statels: the two under every Market Search terminal of Rome
and Athens, in ACG and temple dungeons; `invisible_bsp_*`, `bsp_sl_*`, the Omni mine buildings' `_bsp` twins), the
anchors of the fires, smokes and halos (`fx_campfire`, `fx_bonfire`, `fx_smoke*`, `fx_halo_*`, `halo_*_medium`, the
Shadowlands' `eff_*` meshes), the triggers of the alien invasion's effects, the collision spheres, and the head
`head_solitusmale_invisible`. All but eight have no texture and most have the exporter's default material, which
this client drew in the file's colour: light-blue slabs round the terminals, white floors, grey pyramids, red
crosses. The original hides them: DisplaySystem's `VisualMesh_t` set-up (FUN_1006d49a, `asyncmesh.rs`
`make_attractors` in the drop-in) goes over every triangle mesh at or under the mesh's root and calls
`RRefFrame_t::SetVisible(false, false)` on each whose name `strstr` finds "invis" in: that frame alone (what hangs
under it is still drawn), the word anywhere in the name, in lower case only. The mesh reader does the same
(`model::is_hidden_name`, in `walk`); `MAO_HIDDEN_MESHES=show` draws them. Seen in offline pictures of Rome Blue's
terminals before and after (8,444 parts, then 8,424). The survey that found them:
`MAO_OCCLUDERS=show MAO_HIDDEN_MESHES=show cargo run -p mao-formats --example untextured -- <game>` (every mesh with
a bracket-tagged, an "invis" or an untextured part, by how many statels place it); `--example placed -- <game>
<mesh>` says where a mesh stands, `--example meshobjects` prints a mesh's objects and fields.

What the sweep found and left alone: no other bracket tag than `[OCC]` and no "invis" in another case; 1,086 other
parts without a texture in 643 meshes (a bridge's underside, a chair's frame, a helmet's face that takes the
wearer's texture): the original has no rule against them that was found, so they are drawn in their material's
colour as before. **Found on the way, not done:** the same set-up function turns a mesh's connectors named `eff_*`
into effects (`eff_fire0..9` 2200-2209, `eff_smoke*` 2300-2302, the halos 7000-7006, `eff_torch` 12124,
`eff_geyshir`, `eff_waterdrip`, `eff_billboard`, with a colour and a radius from hex digits in the name; another
name is the red halo). This client reads a mesh's connectors only for the meshes an effect script places, and of
those only `eff_universal_<id>`: no statel got its effects, so a campfire was a ring of stones without its fire (done later the same night: see the statels' own effects, below).

**The mouse on items, tried further by script (2026-11-12, night; Maotesttwo).** The script has clicks with keys
held now: `sclick@x;y` and `srclick@x;y` with Shift, `cclick@x;y` and `cdclick@x;y` (a double click) with Ctrl
(`ui.rs` `script`: the keys go down as the pointer comes to rest and up a frame after the click, as
`ModifiersChanged` events, since the windows read the keys from the frame). `MAO_ITEM_TRACE=1` also tells where a
container's list and its items are, when an item in one is taken up, let go or asked about, and where the hotbar's
first three slots are and what one takes. Five runs:

- Shift+click on an inventory item: its info asked for (`itemref://291082/291082/1`).
- An item taken up by a click and a second click on an open bag's window: it is in the bag (the bag's list then
  shows it at its first place, the inventory closes the gap).
- An item in hand and a click on a hotbar slot: the slot takes it, and the item goes back to its place.
- In the bag: a click takes the item up, a click on the inventory window takes it out; Shift+click asks for its
  info.
- The bag's window closed and shown again by right clicks, as before.

Shift+T is tested as a key of the table (`settings.rs`: the default key map gives the tradeskill window for it and
the target's info for T alone, also for a settings file written before the key was there), not by a press in the
game. **Still not tried:** a worn bag (no test character has one), an item sold by a click on a shop's list, the
bank, a corpse. One thing seen: an info window opens over the bag's first places with the default layout, so a
click meant for the bag's item after a Shift+click lands on the info window (the test's own business; the test
character's Scrap of Notum was left in its bag by it).

**The statels' own effects: a campfire burns (2026-11-12, night; `mao_formats::attractors`, `model::connectors_placed`,
`main.rs` `statel_effects`, `net.rs`, `viewer.rs`).** The other half of the function that hides the "invis"
helpers (DisplaySystem FUN_1006d49a) turns each of a mesh's connectors named `eff_...` into an effect: the name
after the prefix picks the template from a table of 31 (`eff_fire0..9` 2200-2209, `eff_smoke`, `eff_smoke1`,
`eff_smoke2` 2300-2302, the halos 7000-7006, `eff_torch` 12124, `eff_geyshir`, `eff_waterdrip`, `eff_billboard`,
`eff_muzzleflash`, `eff_explosion`, `eff_lightsabre`), `eff_universal_<id>` is that number, a name the table lacks
is the red halo 7001, and after a prefix entry up to five hex bytes give a colour (four or more) and a radius (one
or five: the last, over 255, times 4). `attractors::effect_of` is that table and reading; a connector is the
`RRefFrameConnector` a frame's `conn` points at, placed by the frame (`connectors_placed`). For every statel of the
playfield the client now takes its mesh's effects through the statel's placement and draws each at its place as
the waterfalls' effect is drawn (`placed_particles`), under the environment effects' setting. Newland Desert has
18 (8 fires, 5 smokes, halos), Andromeda 138, Arete 46, Jobe Research 181.

What is this client's own, or left out:
- **The effects whose name gives them a colour or a radius are not drawn** (180 of Jobe Research's 181 halos, 24
  of Andromeda's 138): how the original puts the four values and the radius on the effect was not read
  (`AttractorEffectData_t`'s words go to Gamecode's effect handler), and a halo in its template's colour would be a
  guess. Those lamps stay without a halo, as before.
- Only the effects within 150 units of the eye are drawn, and each place runs a little apart in time from its
  neighbours so that the fires do not flicker as one: both limits are this client's. When the original starts and
  stops them was not read.
- A generated building's statels (a mission's rooms) are not followed. The rendering effects (`eff_hologram`,
  `eff_space`, `eff_clear`) are not made. A template the effects' file lacks (12200, 12570, 96003 in the playfields
  looked at) draws nothing.

Seen in an offline picture of a Newland Desert campfire at 3 and at 40 seconds (it burns on), and a live run in
Arete came up with its 46 effects and no complaint; the fires themselves were not looked at live.
`mao-client <game> statel-effects <playfield>` lists a playfield's effects by template;
`MAO_STATEL_EFFECTS=<seconds> MAO_GFX=<game>/Setupf/gfxtweak.bin` puts them into an offline `map` picture.

**The market's search: the name field, and the refused session (2026-11-13, the user's report: "market name
search field is unusable", three dots where the field belongs; `browser.rs`).** Two things, both from the page
being made for the original's browser (Awesomium, a WebKit of 2012):

- **The text fields' widths.** The search page's name field is `<input type="text" size=51>` in a table cell of
  55% of a 720 pixel page, and the cells have `overflow: hidden; text-overflow: ellipsis`. Today's engine makes a
  field `size` times the width of a "0" (7.4 pixels for the page's 13 px Arial): 390 pixels, measured, in a cell
  with 340 for it, and it then draws the cell's three dots in the field's place. The old engine counted `size`
  times the font's average character width plus the difference between its widest character and the average
  (WebKit's `RenderTextControl`), which is about 280 pixels and fits; the quality fields (`size=1`) came out wide
  enough for a number there and 11 pixels here. The market's pages now get a script (`INPUT_WIDTHS`) that gives
  every sized text field without a width of its own the old width, also the fields the page makes later. The two
  figures (5 and 27 pixels) are worked out from Arial's tables, not measured in the original; the item store and
  the other pages are left as they were.
- **"403 Invalid session".** The web view kept its cookies between runs of the client, and the market refuses a
  cookie of an earlier game session (seen three times in a row on the test character, and gone with an empty
  profile). The original clears the cookies each time it sets its browser up (`awe_webcore_clear_cookies` after
  `awe_webcore_initialize` in GUI.dll's `Reinitialize` 0x1010bbd7); the web view is now opened in its private mode,
  which keeps nothing.

Seen in pictures of a test window (PrintWindow shows the page too): the dots before, the field after, and the
page loading with the old cookies still on disk. **Not tried:** typing a name and running a search. For tests: the
script step `market@` opens the market's page away from a terminal; with `MAO_BROWSER_TRACE` the page now also
tells each text field's box against its table cell; scratch `live2.py` and `win_shot_live.py` run the free test
character with its settings in a scratch folder, beside the user's own game.

**A screenshot's notes (2026-11-13; `shotnotes.rs`).** A picture of a character drawn wrong (the user's "male
watcher is unequipped", in Jobe, where no test character stands) says nothing of what the server sent for it. The
network side now keeps each character's last full update as a line (its template, scale, textures, clothes and
worn meshes) with the update's bytes, up to 1,500 characters, and a screenshot taken in the game writes
`<the picture's name>.txt` beside the picture: the place, the character's and the camera's positions, the selected
target's look with its bytes, then the other known characters' lines. The file stays in the screenshots' folder;
nothing is sent anywhere. `MAO_LOOKS=<part of a name>` (or `*`) prints the same lines to the log as the updates
come. The text is tested (`shotnotes::tests`); a screenshot with notes was not taken in a live run (the screenshot
step is not used while the user is at the machine).

**The Male Watcher without its gear is open:** its update is needed first (a screenshot with it selected, taken
with a build that has the notes).

**The user's list of 2026-11-13 (a play session on Laxative): what was done and what waits.**

- **The garbled loot line** ("~&!!!...Laxative...NanoCrystal (...)...Remains of ...~" in the chat). The chat
  server's lines can hold packed texts (`~&`, a text.mdb text's category and number, its arguments, a closing `~`),
  and only the game server's lines were unpacked. GUI.dll's `HandleGroupMessage` 0x1008692a,
  `HandleVicinityMessage` 0x10086c53 and `HandlePrivateMessage` 0x10087e59 each pass the text through LDB.dll's
  `RemoteFormat::ParseString` before it is laid out; the client's four kinds of chat lines (a notice, vicinity, a
  tell, a group's) now do too (`net.rs` `unpacked`). Not seen live: nobody looted beside the test character.
- **An item's menu by a held right button** (the user: "long right click on item should give a menu with look at
  item, delete and create reference"). The right button held on an inventory item for 0.4 s opens the menu, and
  letting go then is not a use; Shift+right click opens the same menu as before. Its entries: Look At (the item's
  info), Use, Create Reference (the item's link, `<a href="itemref://low/high/quality">name</a>`, added to the
  chat's line), Delete (asks first). The original's own item menu was not found in the decompile: the entries and
  the hold are the user's word for it, the 0.4 s is this client's. The link stands in the chat's line as its
  markup until the line is sent. Tried live by script (`rhold@x;y`): the menu opened and the item was not used;
  the entries were not clicked.
- **The game's log.** A game started from the login now writes its own lines to `game-log.txt` beside the settings
  (the run before is kept as `game-log.previous.txt`), and `[debug] log_messages = on` in `mao.ini` adds the
  server's messages as they come (`MAO_TRACE`). Nothing of the account is in it. This is for the reports below,
  which cannot be found out from a picture.

**Open from that list, each waiting for a log of it happening** (`log_messages = on`, do the thing, tell the time):
- "sale didn't register right" (a shop's window: what was done and what the window showed is not known yet);
- "shop window should close when out of range": Gamecode has no distance check of its own for an open shop that
  was found (`Feedback_TargetOutsideRangeForTrade` is said only when a trade starts, `N3Msg_TradeStart`
  0x10019148); the window is closed by the trade's end (FUN_10066be9, FUN_10066ade), so the server's message at
  walking away is what has to be seen;
- "equipping a yalmaha or ganimedes does nothing";
- "camera is still jittery when running or jumping" (`MAO_CAMERA_TRACE=1` is the tool; no cause known);
- the Male Watcher without its gear (a screenshot with it selected: `shotnotes`).

**From the first session with a log and notes (2026-11-13, morning; the user on Cotangent).**

- **An NPC wears the meshes its update lists** (the user: the Male Watchers "should be wearing helmets"). The notes
  of a screenshot in Old Athen gave the Watcher's update: template 0x661b, five clothes, and the worn meshes
  `(head's place, opifex_male_helmet_techarmour, texture 0x7be0)` and `(right hand, hellspinner_shock_cannon)`.
  Players' worn meshes were attached (`wear`), a creature's with a template were not: `creature_parts` drew the
  template's own head and nothing else. It now takes the update's list: a mesh in the head's place stands for the
  template's head, the others go on their attachment points. Offline picture of the template with that list
  (`MAO_CREATURE_GEAR=80:4e6d:7be0,1:25a1d:0 mao-client <game> creature 661b out.png`): the helmet on, the cannon
  in the hand. Every NPC with listed meshes changes with this (Arete's dock workers carry their tools); how a
  two-handed gun is held at rest was not compared with the original. A listed *head* in the head's place stands for
  the template's head too (2026-11-20): the check knew only a helmet, so Pamela Borroel in Arete Landing wore her
  template's dark-haired head and her listed blonde one at once, the dark hair showing through as a black wedge.
  Gamecode's FUN_100598bd puts one head in that place (it takes the one before off and sets stat 64), and
  `vehicle_parts` already had the rule. Offline: `MAO_CREATURE_CLOTHES=<part>:<texture>,...` (decimal, as a
  screenshot's notes list them) gives the preview what the update wears.
- **What was sold leaves the backpack when the trade completes.** The log of a sale: `Trade` `AddItem` naming the
  character, container 0x68 and the place, once for each item; `Complete`; then the shop opened anew. The server
  says nothing more of the items, and the client kept them until the next full inventory (a zoning). The places
  put into an open shop's trade are now remembered (taken out again by `RemoveItem`, forgotten at a decline or a
  new opening) and removed from the backpack at `Complete`. Not tried live.
- **A shop's window closes when its place is left.** The log shows the server leaving the trade open when the
  character walks off and answering "You are too far away" to what is tried from afar. Where the original ends it
  was not found (Gamecode's range test FUN_1005a1e7 is called when a trade starts; no caller for an open one was
  found). The client now declines the trade and closes the window once the character is more than 8 units from
  where it stood when the shop opened: the rule and the distance are this client's. Not tried live. (Taken out
  on 2026-11-18: the original has no such rule, "A shop stays open when the character walks away".)
- The screenshot's notes match the target by its name (the target's line has its health and distance after it).
  The message log is now read by the game itself (`[debug] log_messages`), and the log says "message log: on/off":
  the first run with the setting had none of the server's messages in it, and why the login's process did not pass
  it on is not known.

**Still open from that session:** a right click on an item that can be worn should wear it (now: use); a Yalmaha
or Ganimedes equipped does not put the character into flight (the user: the Ganimedes shows on the back; for a
Yalmaha the player shrinks, flies and is replaced by the vehicle); "weird slabs" in the tradeskill part of the clan
shop (`ord_smarket_clan_advanced`); the camera's jitter.

**The camera's default is the original's free camera (2026-11-13; the user: "not a fan of the WoW style camera
movement").** The camera was already the original's (`camera.rs`: N3.dll's steered `CameraVehicle_t`s), but a new
settings file chose the rubber camera, whose preferred place turns with the character, so the camera swings in
behind it at every turn. The original's own choice when nothing is preferred is mode 1, the free camera
(FUN_10021859): it keeps its direction and is pulled along at its distance. A new settings file now has it
(`camera_mode = trail`), and `/setoption camera_mode free | rubber | lock` changes it in the game at once and for
the games to come. A file written before keeps its `rubber` until the command is used. What a fresh install of the
original writes into its own preferences was not looked at. `/setoption log_messages on | off` does the same for
the message log: a settings file is the user's own process's to write (what this assistant writes under
`%APPDATA%` lands in a private copy the game never reads, which is why the log stayed off twice). Both commands
tried live on the test character: the settings file had the two lines and the log its "server n3" lines.
The user's own build is made with the `hide-console` feature since that day (no console window, as the packages).

**A worn item can put its wearer into flight (2026-11-13; the user: "equipped ganimedes showed on my back but never
put me in flight mode").** The Ganimedes Personal Jetpack's wear event (event 14 of item 290215) has the spell
0xcf4c with the flying bit (stat 73, bit 2), the same one a hoverboard's program has (`item::grants_flight`), beside
a run speed modifier and eleven 0xcf2d spells that pick the back mesh by breed, sex and build. The client took
flight from the running programs only; the game loop now also takes it from the items in the worn places
(`worn_flight` in `net.rs`). Not tried live: no test character has such an item, and whether the worn list is
brought up to date at the moment of equipping (not only at a zoning) was not seen. A Yalmaha is not done: by the
user's word the player shrinks, flies and is replaced by the vehicle, which wants its wear event read the same way.

**Flight by the original's code (2026-11-13, not tried live).** The note above named the wrong spell. What switches
a character to fly mode (7) is spell **0xcf92** (Gamecode FUN_100a8107: movement action 0x1d applied, 0x27 when its
list ends); the 0xcf4c with flag 2 only takes a state flag of stat 224 away. `item::grants_flight` and
`item::worn_grants_flight` now test for 0xcf92, so the hoverbikes' programs (Spectre, Banshee, Wraith and their kin,
30 of the vehicle programs), the jetpacks and the Yalmahas fly, and the hoverboards' programs (Phantom, Ghost,
Chimera, Loungemaster) **no longer do**: they have no 0xcf92, and a board rides over the ground
(`cargo run -p mao-formats --example vehicleprograms`). The server's spell list is read too
(`spelllist::Outcome::Fly`, kept per character in `World::flying`: the only way another character is known to fly);
the client's own reading of the running programs and of the worn items stays beside it, since no capture shows the
list for a worn item, at login or after a zoning. In flight:
- the body plays the fly action, 0xb3 `hover-norm` (`appearance::Action::Hover`), standing and moving alike
  (FUN_1005d0b1, FUN_1006e4f8, FUN_1006c5ae), at its own rate; a board's or bike's rider keeps the vehicle body's
  own animations here (the original asks the body's template for 0xb3 and falls to its walk, 100);
- the speed is `7 + RunSpeed / 275` within 1.5 and 15, backwards and sideways too (`fly_speed`, FUN_1006f9eb's
  mode 7), and a sidestep while going forward is no faster;
- the right button's vertical movement pitches by N3.dll's `VehicleForwardUpdate` rule (`viewer::fly_pitch`): a turn
  is kept while the cosines of the old and the new elevation multiply to more than 0.001, so the pitch stops near
  88° instead of at 1.4 rad, and, as in the original, a slow drag back from a pitch a fast one took past 88.2° is
  dropped until a larger step comes. The mouse's 0.005 rad a pixel is unchanged (`MouseTurnSensitivity` is not a
  setting of this client). The body is still drawn level and the camera does not tilt while the button pitches, so a
  drag shows nothing until the character moves forward.
- **A worn Yalmaha** (a wear list with a `ChangeMesh` without criteria and a 0xcf76 of stat 360: `item::craft`; the
  four Yalmaha programs have the same list) makes one's own character the craft: its NPC template drawn in the
  character's place by the polymorph's road (the original's is the same, stat 359 from FUN_100a7aa2) at the scale
  the list sets, 20 % (`World::crafts`), playing the template's walk while it flies. "Yalmaha XL" is template 99470,
  `sleek_vehicle_cat.cir`, 11.4 units long and so 2.3 m at 20 %; whether that is its size in the original was not
  checked against a picture. (Another character's Yalmaha is known since 2026-11-18: see "Other players' crafts and flight".)
Not done: the entering effect (2000, colour 0xfff0e0ff), the ways flight ends besides the list (character action
0xac, the kind-5 areas, state flag 0x2000000), the mode's refusal of sit, sneak and crawl, the criteria operators 70
and 112, the pitched body.

**The inventory against the original's code (2026-11-13; the user: "Analyze the original code for inventory and
provide something close to that").** The reading is `docs/INVENTORY_ORIGINAL.md` (GUI.dll's item lists, the
inventory, wear and container views, Gamecode's use and move functions; each statement with its function, and a
table of sixteen differences). It corrects two notes above: the inventory's lists take the **old** mouse path
whatever `UseNewMouseFunc` says (the flag at +0x319 is set for the nano, shortcut and action lists only), where an
item is taken up as the left button comes **up** on its cell; and FUN_100cae1b is the right click's handler, whose
Ctrl is the trade's add. Done from it so far:

- **A right click wears or takes off what cannot be used.** The list always asks for a use; Gamecode's
  `N3Msg_UseItem` (0x10028883) looks at the item's `Can` flags (stat 30): without 8, "use", an own item is not used
  but moved: one in the backpack to its wear page (stat 76: 1 weapons, 2 clothes sixteen on, 3 and 5 implants
  thirty-two on) at its place (stat 88), one that is worn into the backpack's first free place; one with neither
  goes nowhere and nothing is sent. `net.rs` does the same where a use is carried out (another place the item's
  flags, stat 298, allow is taken when its own is taken and that one free: this client's). Tried live: the test
  character's helmet went on (place 18) by a use and off again by the next; a piece of junk sent nothing.
- **An item's menu is the original's** (FUN_100caf07): the right button held 0.3 s (`MouseButtonHoldDelay`), with
  Look At, Create Reference and Delete; no entry to use it, and Shift+right click is a plain use.
- **A bag taken up is closed** (FUN_100cc6e8 → `CloseContainer`).
- **Items stand where they are put** (FUN_100cc855; `places.rs`, `invwindows.rs`, `loot.rs`). The item in hand let
  go on an empty cell of the inventory moves there: the client's own matter, nothing is sent (a cell is not the
  item's place on the server). On another item the two join when they are stacks of one kind, else they change
  cells and the other is in hand. The cells are kept in `places.ini` beside `mao.ini` (`<type> <instance> <place>
  <cell>` a line, zeros for the inventory: one map for every character), an item without one takes the first free
  cell, and the cell of an item that is gone is forgotten. A cell is counted along the rows (the grid takes the
  columns its window is wide), where the original keeps a column and a row. One's own backpacks the same (move and
  swap; no join or split inside a bag yet, and an inventory item put into a bag takes the first free cell); the bank,
  corpses and chests are arranged as before.
- **Stacks.** Join (`N3Msg_JoinItems` 0x1001bb2a, both stackable, `Can` 0x200, one template and quality, no more
  than 50000 together): `CharacterAction` 0x35, the target the stack let go on and the parameters the one in hand
  (`n3::join_items`); the server's answer, the same action, is what makes them one here (FUN_1004a12e). Split:
  Ctrl+click on a stack opens a box for the count (the original counts by the mouse's way sideways while the
  button is down; this box is the client's own), OK has that many in hand, and an empty cell of the inventory makes
  them a stack there: the client splits its own list at once (FUN_1004a2bf, the new stack at the first free place)
  and sends `CharacterAction` 0x34 with the parameters `(0, count)` (`n3::split_item`); anywhere else, the right
  button or Escape gives the split up. Both messages are from the code alone (no capture has one): **not tried
  live**. Offline: `MAO_UI_PREVIEW=gear MAO_ENTRIES=<file> MAO_ITEM_TRACE=1 MAO_CLICK="x,y;x,y"` clicks an item and
  then a cell (the trace tells the items' and the empty cells' pixels and what was done); a fifth number on a line
  of the entries file is a stack's count, and `MAO_SPLIT=<place>` opens the split box.

Left from the table, in the order a player would notice: an item in hand with a plain right click is `UseItemOnItem` and only with Shift the tradeskill
combine (the client sends the combine always); dropping on the ground and using on a character; closing a bag's
window or using an open bag sends the use again (the client only hides the window); Ctrl+right click adds to any
trade; an unfit wear place leaves the item in hand; a corpse has no right click and no "Take all"; the windows'
own menus (List Mode, Auto Arrange, Labels On Top, Unlock All Items); the tooltip and the count's look. Also seen:
`wear_refusal` tests the place's bit without the page (a drag onto a place of the wrong page is not refused here).

**Placing by hand and the stacks, tried live (2026-11-13; Maotesttwo, by script with `MAO_ITEM_TRACE`).** An item
in hand clicked onto an empty cell stayed there and `places.ini` had its cell; clicked onto another item the two
changed cells and the other was in hand; Ctrl+click on a stack of arrows, the count box answered with Enter and a
click on an empty cell split one arrow off, and the next login had the two stacks (the server took the split,
action 0x34, as it was built from the decompile); the small stack clicked onto the big one joined them (0x35), one
stack in the list at once and at the next login. Not tried: the same in a bag's window, a join of stacks of
different quality, the count box used by hand.

**A swap by right click goes one move at a time (2026-11-13; the user: swapping from the Ganimedes to a garden key,
"the key gets unequipped right after").** The log of it: the jetpack's taking off was answered by the server, the
key's wearing, sent in the next turn, never was, and the key came back to the backpack. The two moves of a swap now
go one after the other's answer and a moment (0.6 s; 3 s at most without an answer), a wear place still taken when
its turn comes ends the swap, and a taking off the server took is now marked as answered (only moves out of the
backpack were). Tried live on the test character's two pairs of shorts: both moves answered, 0.64 s apart, and
kept at the next login. Why the server left the key's wearing unanswered is not known (the same pair of moves the
other way round, key off and jetpack on, was taken); the original sends both at once.

**The character's own item list is no container (2026-11-15; the user: "when retrieving my daily special another
window like inventory showed up but wasn't that").** Their log: after the claim the server names the reward into the
overflow (`35505644`), moves it in (`47537a24`) and then sends an `InventoryUpdate` (`4e536976`) of 2621 bytes, 81
items, whose `InventoryIdentity` is the character and whose handle is 0x68; the client took every such message for a
container that was opened and showed the whole inventory in the loot window. Gamecode's handler (`InventoryUpdateIIR_t`,
FUN_100a0957) opens a container only when the identity is a `Chest_t`; for a `SimpleChar_t` it hands the list to
FUN_100479e7, which makes it the character's inventory (handle 0x68) or its overflow (handle 0x6e; when that list will
not go in, `Feedback_InventoryOverflowed`). `net.rs` now does the first: the list replaces the character's items and
no window opens. The two such messages in the captures (16:55 #781, events #76) are 0x68 too. **Not done:** the
overflow's list (0x6e) is only counted in the log; this client has no overflow window. Not tried live: a claim shows
on a paid account only.

**A taken mail shows nothing left (2026-11-15; the user: "when removing an item from mail the icon stays the mail
after showing in my inventory").** Their log: the mail came with flags 0x7c and an item, the take was answered with two
flag changes, 0x7d and 0x7f. The client kept the flags but went on drawing the item. GUI.dll: the mail's fill
(FUN_10107b87) shows the item and the money only while bit 1 of the flags, "taken", is clear, and enables Take All
while there is something to take and Delete only when there is not; the flags' change for the open mail with that
bit set (FUN_10106eaa) empties the item's place, writes "0" into Cash and COD, turns Take All and Return To Sender
off and Delete on. `mailmessage.rs` does both now (`left_in`). Different from before: Delete in an open mail is off
while its things are untaken, as in the original (the inbox's own Delete is as it was).

**The overflow (2026-11-15; the user: "How are we handling inventory overflow?").** It was not handled: an item the
server named into it was kept aside until the server moved it in, and nothing else. The reading is
`docs/OVERFLOW_ORIGINAL.md`. The original has no overflow window of its own: the character has a second item list
(identity type 0x6e), and its window is the ordinary container window. Done from it in `net.rs` / `loot.rs`:

- the list (`World::overflow`): a `TemplateAction` into it adds the item at its first free place, with no window and
  no line; the server's `ContainerAddItem` from `(0x6e, place)` brings that item into the inventory's first free
  place (the message's target and place are not looked at; with no room nothing happens); an `InventoryUpdate` that
  names the character with the handle 0x6e replaces the list, and when the window is not up opens it and says
  `Feedback_InventoryOverflowed`; zoning empties it and closes the window;
- the window (titled "Overflow"): a click or a right click on an item asks the server to move it in
  (`ClientMoveItemToInventory` with `(0x6e, place)` and 0x6f) and nothing moves until the server answers; with a
  full inventory nothing is sent and the game's `Feedback_InventoryFull` is said; nothing can be dropped into it; it
  stays up when empty;
- nothing is put on while the overflow's first place holds something (`Feedback_WearingDisabledWhenItemsInOverflow`).

Not done: the refusal of a unique item one already has (left to the server); the window's title from the game's own
text; list mode. This client's own: the container window's "Take all" stands in the overflow's too. Not tried live:
it needs a full inventory and a given item.

**A statel turned and tilted (2026-11-15; the user: "the ramps for the shuttles platform are messed up", and the
"weird slabs" in the clan shops' tradeskill part).** A statel's rotation is packed as a yaw, a pitch and a roll in
degrees. N3.dll (FUN_10026e66, FUN_1002777a) makes the yaw's quaternion and multiplies the pitch's and the roll's
onto it with FUN_10003dc3, whose `a *= b` is Hamilton's `b * a`: the product is roll * pitch * yaw, the turn about Y
first and the pitch and the roll after it about the world's axes. `mao_formats::statels` had yaw * pitch * roll: the
same for a statel turned about one axis only, which is nearly all of them, and wrong for the rest. Between ICC's
landing platforms two bridge pieces (`icc_bridge_slant01`, a piece that rises 26 degrees along its length) lie
mirrored, one with a pitch of 26 and its twin turned half round with a pitch of -26: both lie flat as a bridge in
the right order; in the wrong one the twin stood up at 52 degrees. The clan shops' `sa_metalplate` plates (playfields
1180 to 1182), turned on edge and tilted, are the dark triangular walls beside the ramps in the user's pictures of
the original; they lay about as slabs. Offline renders of both places match now. The collision is built from the
same rotations and follows.

**A texture's alpha without a blend or a test (2026-11-15; the user: "city controller doesn't look right").** The
controller (`ai_buildings_city_controller`, mesh 251929, a built-in object of a city's playfield, type 0xc418) has a
band of dark machinery under its head whose texture carries an alpha channel of black and white noise; the band was
drawn full of holes. The client cut every unblended textured part out at half alpha. The original does not: its
device has the alpha test off (randy31 `randy.rs`, state 15 = 0) and a material read from a mesh keeps the states it
was stored with (`FAFMaterial_t`'s reader runs no alpha mode), so a material that sets neither blending (27) nor an
alpha test (15 with 24) is drawn whole. `mao-render` does that now (`blend_of`: the cut's reference is the
material's own or none; plants keep their half). 42 of the 11,228 meshes have such a material
(`cargo run -p mao-formats --example opaquealpha -- <game>`): the three city controllers, the cloak towers and the
swimming pools of the cities, Omni-Trans signs, the QO screens and servers, a yellow cab, earmuffs.

New tools: `--example pfdynels -- <game> <playfield>` (a playfield's built-in objects with their templates' names
and meshes), `--example rawget -- <game> <kind hex> <instance hex> <out>` (one record's bytes; a texture as its
picture file).

**A player city's buildings (2026-11-15; the user: "org city buildings don't show").** The client kept the server's
list of a city's houses (`PlayfieldAllCities`, 2007 bytes in the user's city) and drew nothing. The reading of the
original is `docs/CITIES_ORIGINAL.md` (city.dll, Gamecode's city holder; each statement with its function). In short:
the list is a count and 25 bytes a house, its place (the least corner of the tiles it stands on), its rotation in
degrees and its template, a database record of type 0xf4268 (83 of them) that holds the tiles, the meshes with their
places and rotations, where the doors go and a collision surface. The original makes no object of the game for a
house: a plain visual mesh for each of the template's meshes, as a statel's, and the surface added to the playfield's.

Done: `mao_net::landcontrol::houses` (the list), `mao_formats::house` (the template, `rotate_local`, the surface through
`surface::parse_bare`), `crate::houses::add` (the meshes into the playfield's scene with their far twins, the surfaces
into the collision and the walked surfaces). The list comes right after the playfield's own message, so the houses
go into the scene before it is sent; a list that comes later has the playfield read again and the scene sent anew
with the camera kept (`Update::keep_camera`). Zoning empties the list.

Checked: all 83 templates read, their meshes are in the database and their surfaces read and lie within their turned
footprints (`the_house_templates_read`); the turn of a place against a run of the original's own arithmetic; and an
offline picture of five made-up houses in playfield 6010 (`MAO_HOUSES=<hex of the list>` with the `map` view), which
stand upright on the plots. **Not checked against the original's picture or a real list:** the user's log kept only
the first 28 bytes. The client now writes a whole list to the log ("city: the houses' list ...") and, for each house,
where its doors should be by the same arithmetic ("city: house N ... doors [...]"), to hold against the doors the
server announces at the next visit.

Not done: a house built or taken down while the character is in the city (`AOTransportSignal` 0x62741e15, codes 2
and 9, with the building effect 0x30e8): no such message has been seen, and whether it has a lead byte is a guess.
The far twin where a mesh has none in the database (15 of 25 looked at) is simply the full mesh.

**Taking an item with a full inventory (2026-11-15; the user: "it allowed me to loot even though I had no space left
in inventory and never spawned or went to overflow").** Their log: the take was sent, the server moved the item to
the character (`ContainerAddItem` from the corpse's handle, no place named), and the client, whose list of the
backpack had thirty items, dropped it without a word. Two things of the original's were missing. `N3Msg_MoveItemToInventory`
(Gamecode 0x10027bac) asks nothing with a full inventory: it says `Feedback_InventoryFull` and sends nothing, for a
corpse, a chest, the bank or the overflow alike; the take does that now. And the item the server moves in is taken
as FUN_1004ac4b takes it (`World::take_in`): one that stacks (its `Can` flags have 0x200 and not 0x1000000) joins the
stack of the same item in the backpack, the two no more than 50000 together; else the first free place; with none it
is not taken in. **Not explained:** the server had room for the item where the client's list had none, so the
client's list held one entry too many from the login on (nothing changed it before the loot). When that happens
again the log names the item and lists the backpack as the client has it ("items: no free place for item ..."), to
find the entry; a relog shows the server's list.

**Thrown forward into the water (2026-11-15; the user: "the server placed me in the water then slingshotted me back").**
Their log shows it was the client: walking a hillside in playfield 4310 it reported itself 74 m further on in under a
second, over a lake, fell in, and twenty seconds later the server, which had never taken that place, pulled it back.
A hunt (`MAO_HUNT=<half-width>,<speed>` with `net <game> caught <playfield> x y z`: bodies run from a grid of places
every way at several frame times) found such steps on that hillside, 40 to 80 m in one. The cause was one comparison
in the port of the vehicle's sweep (`vehicle.rs` `advance`, Vehicle.dll FUN_1000b2e5): the original ends the sweep when
a step is as long as the distance left or longer ("below or equal"); the port had "below", so a step of exactly the
distance left was taken whole, nothing was left to count down, and every further pass ran against the budget alone,
which on the ground is 10000: up to ten metres a pass. With the original's comparison the hunt finds none in 6.8
million runs where it found dozens. Not done: the server's own `SetPos` for the client's character is still not
taken (it is for the others).

**Jobe's teleporters (2026-11-15; the user: "there should be a teleporter to jobe platform").** They are doors of the
playfield's table with the template "Teleporting Ring", whose mesh (44891, a ring with two glow rings and a light,
animated over 1.4 s) hangs under each sign. `static_props` gave an outdoor door without a mesh of its own none at
all, a rule of this client's from the time a shuttle's door looked out of place; it is gone, and a door has its
record's mesh or its template's everywhere. The ring moves as a door does, when the character comes to it. Seen
with it: the plain "Door" template's small white leaf stands in the hatch of the shuttles at ICC's landing
platforms. New tool: `--example pfdynels` showed the record.

**The server's `SetPos` (2026-11-15).** Read while looking at the throw above: `SetPosIIR_c` (reader FUN_1007733b,
handler FUN_100773a3) is a lead byte, the place, a byte, an int and a byte, and the handler puts the dynel there with
the rotation it has, the client's own character like any other (with `Feedback_CrowdLimiting` said when the int is
set). The client read the place from the first byte on, a byte early, so a creature the server set down went to a
place of numbers like 6e-39 (the "no place of this world" of an earlier note; 310 such messages in the captures),
and it did not move its own character at all. Both are as the original now (`set_pos_puts_a_character_down`, from
the 16:55 capture's bytes).

**The special actions against the original (2026-11-15; the user: "can we [check] against original code for all the
specials and add any missing ones?").** The reading is `docs/SPECIALS_ORIGINAL.md`. **None is missing:** the 26
actions of `actions::TEMPLATES` are FUN_10042b17's, the seven a weapon gives (`WEAPON_CAN`) are FUN_1009e84b's, and
Backstab, Reload, Brawl and Dimach and the perk actions come and go by the original's rules. What differed was what
happens when one is used. Done:

- **Refusals before anything is sent** (`World::special_refusal`; `N3Msg_PerformSpecialAction` FUN_100427d6 and
  `N3Msg_SecondarySpecialAttack` 0x100281fc): an action that is not in the character's list says
  `Feedback_ActionIsNotAvailable` (a hotbar slot or a key for Burst with no burst weapon on was sent all the same); a
  special whose skill the server has locked gets the game's wait line (`UnableToPerformActionSkill`, with the time
  left) and is not sent; an attack at oneself, at the dead or while swimming gets the game's line; with no target
  nothing is said (the client's own "needs a target" lines are gone). While fighting a special goes at the one
  fought, not at the selection.
- **The server's lock** (`CharacterAction` 0xaa and 0xa4) is kept by the skill (`World::special_locks`); 0xa4 says
  `Feedback_SkillAvailable` with the skill's name, and the server's own wait line (0x84) is shown.
- **The recharge on the icons**: the hotbar found a special's skill by its name and had no Aimed Shot, Sneak Attack,
  bow's special attack or Backstab; it is by the action's number now (`hotbar::special_stat`), and the Actions
  window's tiles count down too (`actions::lock_left`), which only the perks' did.
- **Search** sends `CharacterAction` 0x42 (FUN_1003fc96) with the game's line; it sent 0x66, which no capture had
  shown. Its lock (skill 0x88) gives the wait line.
- **Sneak** leaves sneaking when the character sneaks already (movement action 0x24) and is refused in a vehicle.
- **Keys** can be bound to Sneak Attack, Fast Attack, Aimed Shot and the bow's special attack (`action_sneak_attack`
  and so on; without a key at first: the original's own keys for them were not read).

Left from the report's list: the six tiles that should change with the character's state (Sit or Stand, Walk or
Run, Sneak or Stop Sneaking, Start or End Combat, Suspended Animation or Disrupt, Crawl or its stop) and the tiles
that still do nothing (Use, Pick-Up, Walk, Suspended Animation, Crawl); the held weapon's "can" bit for a special
(the original refuses Brawl and Dimach with a weapon that lacks their bits: left to the server here until that is
seen live), the special's range and the line of sight; the queue of specials not yet swung; `CharacterAction` 0xb2,
0x14 and 0xfc; the icon's time text; the special attack at an attackable item (the message names a character).
Not tried live against a target.

**The last two (CMB-023, 2026-11-19).** The icon's time text: the original's icon view (GUI FUN_1003ddd9, for the item,
nano and action views: kinds 1, 4 and 6) asks the progress and, with `IconTimerText`, writes the time left over the icon
(FUN_1003f8c2: "%2ds" up to 60 s, else "%2dm"), centred in yellow (FUN_1003f9ea's `<center><font color=yellow>`). The
shortcut bars had it; the list views' icons now have it too (`multilist::Item::timer`, `actions::timer_text`): the
Actions window, the nano programs and the inventory grids. A special attack at an item: FUN_10069a9f judges a
`SimpleItem_t` by its own flags (the dynel's +0x4c, stat 0, read by FUN_10003d5b): without 0x10000000 it says
`Feedback_YouCantAttackThisItem`; with it the item's requirements for action 11 decide (FUN_100816f6, told by the
evaluator); none of the character checks (oneself, swimming, the dead) is asked. The client keeps the flags and template
of the items standing in the world (`World::item_dynels`, from `SimpleItemFullUpdate`), and `CharSecSpecAttack` now
names the target by its own identity (FUN_10072d55) instead of always as a character. Tests:
`a_special_at_an_item_asks_the_items_flags`, `a_locked_icon_writes_its_time_left`.

**The teleport target window (2026-11-15; the user: "in the grid the notum controller and organisation city should
open a window to let you select either org city or any towers field your org has control of").** Their log had the
server's message at the Grid's organisation city exit, of a kind the client did not know: 0x0639474d, which is
`GridDestinationSelectIIR_t` through `n3InfoItemRemote_t::MapToKey` (N3.dll 0x10009826: the class name's bytes XORed
into a word; `mao_net::grid::map_to_key`). It carries a list (a playfield, an identity, a name and two words nothing
reads, per place) and a token of the server's (`mao_net::grid`; Gamecode FUN_101295f0, FUN_1012905a, FUN_1013d67a).
The handler (FUN_10129529) keeps both and turns on GUI.dll's `TeleportTarget_c` (FUN_10100022), which is
`teleport.rs` now: `Views/TeleportTarget.xml` in a window titled "Vehicle" (the original's own title for it), 500 by
420 in the screen's middle, a list of "Location" (the place's name) and "Playfield" (the playfield's name, "unknown
pf" without one), "Go!" and "Close". A click selects a row; "Go!" with a row selected answers with
`GridSelectedIIR_t` (0x3a322a4a: the token, the row's number, its playfield and identity) and closes the window; and
every close, after a "Go!" too, answers row -1 with playfield 0 and identity (0, 0) (FUN_100ffedf, the window's close
request). So a "Go!" sends two answers; the original does. Not drawn: the window's one tab, "Target List". **Not
tried live:** the list in the user's log was kept in part only (28 of 54 bytes: one place, playfield 6010, a name
that starts "City Grid"), so the tests' lists are made up past that; the client writes the whole message to the log
now ("grid: the destinations' list ..."). `MAO_UI_PREVIEW=teleport` shows the window with two made-up rows.

**Inside an organisation's buildings (2026-11-15; the user: "the inside of organsition buildings are not mapped
out").** Their log: through a silo's door the server zoned them to a playfield the client numbered 3637258 and drew
nothing for ("collision 0 triangles"; their picture is the dark with the characters in it). The playfield message's
proxy was of a type the client had not met, 0xc79d, and its second word (0x37800a) is the instance's number, not a
playfield of the game's data. Gamecode's `GetPlayfieldFactory` (0x1001886c) gives that type its own factory,
`AnarchyBuildingPlayfieldFactoryClient_t`, whose making of the playfield (FUN_1011f23e) first reads the object after
the proxy, `OwnedBuildingGeneratorData_t`: the object's identity and a word, a version, then the playfield whose
rooms the inside is (`ReadPlayfield` gets `(0xf4241, it)`), then the playfield the building stands in, a word, a
place and a list of fixtures. `Message::owned_building` reads it and `arrival` draws that playfield: 4350 "Notum Silo
Inside" and 4349 "Superior Neutral org hq" are the insides of the two the user entered, by their house templates'
`inside` word, which is that number (a guess of `docs/CITIES_ORIGINAL.md` no more). **Not tried live**, and the log
had kept 28 of the message's 138 bytes: the playfield's number is taken at byte 58 by the original's order of reads,
which no captured message confirms yet; the client writes the whole message to the log now ("an owned building's
inside ..."). Not read: the fixtures (what an organisation has put in the building).

**A city's controller under the mouse (2026-11-15; the user: "the city controller should be targettable and
usable").** It was both by the code, as every object of a playfield's table is, but the mouse picked such an object
by a cylinder of a terminal's size on its place (0.8 by 2.2), and the controller is 19 units wide and 27 high: only
its foot could be clicked. A table object is picked by its mesh's size now when that is larger
(`table_pick_sizes`; `--example meshsize` measures a mesh). Its use goes to the server as any object's
(`n3::use_object` on the identity the playfield message's runs give it). What the server answers and the window it
opens are the next piece (see below, when it is there).

**The city's houses checked (2026-11-15).** The user's log has their city's whole list (nine houses) and the
server's `DoorFullUpdate`s of five of its doors: the doors the client computes from the houses' places, turns and
templates stand where the server's do to the centimetre (`houses::tests`, `landcontrol::tests::a_real_citys_houses`).
The placing and facing of city buildings is no longer a guess.

**The city controller's window (2026-11-15; the user: "This is the only way to start alien raids, and manage org
city buildings").** The reading is `docs/CITY_CONTROLLER_ORIGINAL.md`. The controller is a `CityTerminal_t`, used as
any terminal; the server answers with `AOTransportSignal` (0x62741e15) code 5, which opens a session and GUI.dll's
`CityTerminalWindow_c` (`cityterminal.rs` on `Views/CityTerminal.xml`; `mao_net::city` the messages, every number
checked against its class name's hash). While the session lasts the server's codes 6, 10, 13, 14, 15 and 16 fill it
(lease mode, upkeep, seconds to the next bill, the cloaking device, the charge, the owner); code 7 ends it, and so does
leaving the playfield. Done for GUI type 2:
the upkeep tab (the game's help text, owner, the next bill's long date, the upkeep, the charge bar, "dump city
advantages", which is the organisation command 0x1f and whose answer `CityAdvantages` 0x365e555b is written to the
chat as numbered item links); the cloaking device's tab (a toggle button, down while the device is on, enabled with
lease mode 3 while the server lets it be switched; **a press sends the request at once, as the original's does: it
asks nothing first, and the cloak off is what lets the aliens come**); the one-tab window of lease mode 1; and the
close request (the window stays till the server's code 7). The original has no "start raid" control, no guest list
and no button that pays the upkeep: the cloak's switch is the raid's. The houses' signals (2 built, 8 demolishing
begun, 9 gone) change the city's houses in the scene now. **Not done:** the city's map (`BuildingMapView_c`), so the
build and demolish tabs show their texts and their buttons with nothing to press them on (building is a building
item dragged from the inventory onto the map; demolishing a house chosen on it); GUI type 1, the dialog that buys a
city's land; the Escape option `esc_cityterminal`. Shown before the server tells it: no bill date (the original
would write its last stamp's date). **Not tried live**: no capture has any of these messages; every signal is written
to the log whole ("city: signal ..."). `MAO_UI_PREVIEW=cityterminal` (with `MAO_CITY_TAB=<n>`) shows the window on a
made-up session.

**The city's map: building and demolishing (2026-11-15; the user: "Build the map now").** The reading is
`docs/CITY_MAP_ORIGINAL.md`; the keeping is `citymap.rs`, the drawing and the mouse `cityterminal.rs`. GUI.dll's
`BuildingMapView_c` is the playfield map's ground renderer with an overlay of tiles: the ground from above (the
picture the playfield map already has, `AutoMap::ground_texture`) at 2 pixels a world unit, +z up, over the city's
zones and 50 units round them (842 by 1082 pixels in playfield 6010) in a scroll view. New formats:
`mao_formats::citydata` (the playfield's city record, type 0xf4267: the zones a house may stand on, ten tiles by ten
each with a mask, and the land's price and upkeep; 74 zones with 2569 set tiles in 6010) and the house template's
footprint mask and guards (`house.rs`; one headquarters is an L, every other template a full rectangle). On the map:
the zones' tiles, the houses (yellow, blue-green once their demolishing began, the selected one bright), the houses
about to be built (green where they may stand, red where not) and a dot at each door (blue) and guard (red). A click
selects the house under it (not one being demolished); a drag elsewhere scrolls. In build mode a building item of the
inventory (stat 620 names its house template) let go on the map waits there with its least corner on the tile under
the pointer, unless it is no building item, of another side, or past its type's limit (one headquarters, market,
grid, guard house, uplink, cloaking device and tax office a city, one of each unique house; no market at all in an
instanced city), which the original's error boxes say; it is dragged, turned by "Rotate", taken off by "Delete".
"Construct" is enabled when something waits, there is a headquarters, no two houses share a tile and every waiting
one lies in the zones (`CanBuildLayout`); it sends `ClientRequestBuild` for each, the headquarters first (template,
tile, turn, and the item as `(0x68, place)`). "Demolish" sends `ClientRequestDemolish` with the selected standing
house's tile. **Both go at once, with no question, as the original's do; neither can be taken back.** The map only
shows, without the buttons, unless the lease mode is 3 and the opening's third word 0 or 1. Checked against the
user's real city: all nine houses lie in the zones by the original's test and none share a tile. **Not tried live.**
Not done: the land-buying dialog (GUI type 1); the inventory's lock on an item that waits on the map (such an item is
not taken a second time instead); the pointer's place as a tip. A guess kept from the reading: the building item's
identity in the request is its inventory place.

**The org headquarters' inside (2026-11-15; the user: "the floors are too high hiding the walls and fish tanks, the
bar has several objects out of position and the two teleporters should be taking me to other rooms").** Five things,
found from the user's two screenshots and the log of the visit (which now has the whole messages):

- *A placed mesh's root frame.* The lobby's mesh (`ai_buildings_inside_lobby`, 252279) is stored about its middle and
  its root frame's saved matrix lifts it 4.99; the client dropped that lift, so the tunnel stood half under the floor
  and the room's own floor cut it at mid height. The original: a statel's `VisualMesh_t` has a frame of its own that
  takes the place (N3 FUN_1002435c: `SetAnimMatrix` when the statel has one, `SetPosition`, `SetRotation`,
  `SetScale`), and the loaded mesh's root goes under it "at the origin, unrotated" (DisplaySystem FUN_1006d7b9,
  `asyncmesh.rs` `attach`: `SetPosition(0)`, `SetRotation(none)`); its scale and its saved matrix stay. So a placed
  mesh's root gives its matrix whole and its scale, and nothing else, a statel's as a dynel's (`model::walk`,
  `Root::Placed`). Until now a statel's root kept "its rotation and scale, not its position", and the matrix's shift
  went with the position. `--example rootframes` counts them: of the 7,245 meshes the statels use, 1,205 have a
  shifting matrix on the root (98 by 5 units or more), none a rotation or a scale of its own. The same rule put the
  bar's bottles 1.1 m beside their places (`bottle-brown`, 18492, 536 statels), the lounge's curtains across the
  floor, and the commonest grass tuft (206509, 20,018 statels) 2.9 m from its spot. Checked: the lobby's floor comes
  out at the statel's height 8.40, and the server stands the character on it at 8.435.
- *The rooms' textures.* A tile value's texture number with the top bit set is a mesh texture, type `0xf6954`, of the
  number without the bit (`VisualRoom_t::AddMaterial`, displaysystem `rdb.rs`); the headquarters' and the notum silo's
  rooms name all of theirs so. Read as room textures (`0xf6959`) none was found and the floors and walls were blank
  white (`rooms::texture_identity`).
- *The elevators.* They are no doors and no machines to use: the server moves the character (`N3Teleport`) from the
  hall to the tactical room or the laboratory when it walks into one. The client took the move and then undid it: the
  two rooms share no doorway, and the step after was refused as a crossing without a door. A place the server gives
  is no step (`RoomState::placed`, from `n3TeleportIIR_t::Activate`'s `SetRelPosRot`); the log had "the server moved
  us to [262.3, 1.8, 196.1]" four times while the reports kept coming from the elevator.
- *The built-in objects' ids.* An owned building's playfield message has the runs of ids an instanced shop's has,
  after its own data (the count at byte 106): `Message::instance_runs` reads them, so the headquarters' machines (the
  grid terminal, the two entrances) are used by the server's ids. Its doors take the number of the table record their
  run names (`World::door_number`): the four are the records 4, 7, 9 and 10 with the numbers 3, 2, 0 and 1, and
  counted along the ids the server's word on one door went to another.
- *Logging in inside.* The character list's playfield is the building's own number, which has nothing to draw; the
  login's playfield message names the template. It was thrown away as "the playfield already drawn"; one that names
  another playfield is kept and zoned into ("playfield 3637254: collision 0 triangles" in the log).

Tests from the real messages: `an_owned_buildings_inside_is_its_templates_playfield` (the silo's 138 bytes and the
headquarters' 258: the template is the word at byte 58, as read), `a_real_citys_signals_read` (the controller's
opening, upkeep, bill, cloak, charge and close, as the made-up ones had them),
`an_owned_buildings_doors_take_their_table_records_numbers`, `a_teleport_between_rooms_without_a_doorway_is_kept`,
`a_placed_mesh_keeps_its_roots_matrix`, `an_owned_buildings_rooms_take_mesh_textures`. Seen in offline pictures
only (`map 4349`); **not tried live.** Not done: a statel's own matrix (`unread_rotation` in `statels.rs`, the
original's `SetAnimMatrix`); the elevators' sound and effect.

**Leaving a Yalmaha, and the speeds (2026-11-15; the user: "leaving yalmaha does not restore size ... Not sure I saw
runspeed increase either").** The screenshot has the character on foot at about a fifth of its size. Two things, both
from the original's reading of a stat:

- *The size.* A Yalmaha's wear list has, beside the craft's template, a temporary set (spell 0xcf76) of `Scale` (360)
  to 20. The original runs the list on the client (FUN_100a95c7): beginning, the value the stat had is kept, unless one
  is kept already, and the stat is set (FUN_10064d77); the list ending, the kept value is put back (FUN_1006420f). The
  client drew the craft at the list's scale and never touched the stat, and the server sends none for it at taking
  the craft off (the log of the flight has only the two item messages there). Its full list at a zoning has the 20,
  though (the character's own update in the screenshot's notes names template 99470 at scale 20): so after a zoning on
  the craft the stat was 20 for good. Now `World::temp_stat` keeps and gives back as the original does, for the worn
  items' wear lists (`item::worn_sets`, begun and ended as the worn list changes: the server sends no spell list for
  what is worn) and for a spell list of the server's (`spelllist::Outcome::TempStat`). The kept values last the
  session, so a zoning does not lose them.
- *The speeds.* The original reads `RunSpeed` and the greatest health for the movement's speeds in mode 3
  (FUN_1006f2fc; the stat container's FUN_10059399 → FUN_10064d46): the stored value, the modifier map's part and the
  attributes' trickle, the number the skill window shows. The client used the stored value alone, so a Yalmaha's 300
  did nothing, and neither did any other worn `RunSpeed`, a program's, or the trickle: the flight's log has 7.44 units
  a second, which is the trained 121 and nothing else. `moving_stats` reads them as the original (`derived::shown_one`),
  for running, swimming and flying. The body's `Scale` is read the same way (the callers of `SetBodyScale` ask mode 3).
  **Every character now runs faster than before**, by what it wears and its attributes give.

Tests: `leaving_a_yalmaha_gives_the_size_back` (the user's Yalmaha, worn, a zoning's list in between, taken off; the
300 in the flight's speed), `the_temporary_set`, `one_shown_value_is_the_tables`. **Not tried live.** Not known: what
the original has kept for a character that logs in on its craft (the server's stored value is then the 20; here the
character would stay small after leaving it until the server says otherwise). The log now has a line when the server
tells one's own `MonsterData` or `Scale` ("own stat 360 from the server: ..."). (Another player's craft from its update: done 2026-11-18; what is kept for one that logs in on its craft is in the update too, the byte with the flags' bit 23.) Not done: `MonsterData`'s own handling in the temporary set.

**A click steps along the mouse's line; the mission request's first byte (2026-11-15; the user: "using the teleporter
behind windcaller Karrec was hard to target and use", "missions aren't working in ICC").**

- *The click.* The teleporter is the ring of a shuttle's door in Tir County (playfield 647, the table's "Shuttle Door"
  at 1814, 2644): a flat ring on the ground with NPCs standing before it. The log has nine clicks from the ramp above
  it, every one answered with the info of one of three characters, and none on the ring: a click took the first thing
  the line from the eye met, and that was whoever stood in front. The original keeps everything the line meets, nearest
  first (`n3Camera_t` +0x244), and GUI.dll's click (FUN_1002cfac) asks `GetNextTarget` with the selected target: the
  one after it on the line, round to the first again, or the first when the selected one is not on the line. So a
  second click in the same place takes what stands behind. A right click uses what `GetObjectUnderColLine` gives: the
  selected target when the line meets it, else the first. Ported so (`viewer.rs`: `on_line`, `next_on_line`,
  `under_line`; the selected target's id now goes to the window, `Hud::selected`). (All of this paragraph's own rules went on 2026-11-18: "What the mouse points at".) This client's own and kept: a second
  left click within half a second is a double click and uses what the first took, without stepping on. The original's
  other two left clicks are not ported: with Shift it opens the info page, with Ctrl or Alt on a character it switches
  the fight to it. Every click now writes a line to the log ("click (left): on the line [...], selected ..., took
  ..."). Not known: whether the original's list has one's own character (here it is on the line only where nothing
  else is), and the shapes it meets (here cylinders).
- *The missions.* The request (`QuestAlternative`) began with a byte of 1, from the AOSharp definitions it was written
  from; the original's, captured on 2026-11-08 at the very terminal the user stood at (ICC's "Basic Individual Mission
  Terminal"), begins with 0. The user's log has the terminal's use echoed seven times and no offers once. The request
  is now the original's byte for byte (`n3::quest_alternative`, test `the_mission_request_is_the_originals`), and each
  one writes a line to the log ("missions: asked ..."). The one error box in that log, "You need to be in a team to
  use a team mission booth", is the original's own rule: the nearer of the two terminals there is the team's.

Tests: `a_second_click_takes_what_stands_behind`, `the_mission_request_is_the_originals`. **Not tried live.**

**The mission window's sliders (2026-11-15, later; the user: "missions are still not working").** The log of the try
has the request with the first byte put right, at ICC seven times and at Old Athen once, and no answer to any. The
request's seven slider bytes were the fault: the client's window had the difficulty from 0 to 7 and the six others
from 0 to 255 with 128 for the middle, a guess from one capture. The original's:

- The window (GUI.dll FUN_100d10fc) has the difficulty from 0 to 100, at 50 when nothing is kept for it, and the six
  others from -100 to 100, at 0.
- `N3Msg_GenerateMissions` (Gamecode 0x10016276) makes the request's difficulty `floor(slider * 11/101 + 1)`, 1 to 11,
  and puts the six others in as signed bytes (`DimensionValues_t::SetValue`, Gamedata.dll 0x100021eb, takes -100 to
  100 only).
- The message's reading (`operator>>` 0x100022fd, and `ACGQuestIIR_t` FUN_100c9f49: the same code the server has)
  throws the message away when a difficulty is outside 1 to 11 or one of the six outside -100 to 100. A byte of 128 is
  -128: every request of this client's was thrown away, unless a player had moved all six sliders.

Both captures fit: `06` and six zeros is the window untouched (2026-11-08), `02 14 a1 1e b8 1c 00` a low difficulty
with 20, -95, 30, -72, 28, 0 (2026-10-21). Ported so: `mission::SLIDERS`, `SLIDERS_AT_FIRST`, `request_bytes`. And the
Request Missions button sends the request alone, as the original's (FUN_100d09cb): it used the terminal a second time
just before, which the original does not (the terminal is still used first when the window was opened another way,
this client's own). Test: `the_sliders_make_the_requests_bytes`. **Not tried live** (a request costs credits, and the
test character's playfield has no terminal). Not done: the original keeps the sliders between sessions
(`MissionSelectionViewConfig`), and shows the six others only with its "advanced options" button down.

**A clicked offer marks the planet map (2026-11-15; the user: "missions should send a marker (waypoint) to the planet
map when I click on them to see the location").** The original's terminal window does it (`MissionSelectionView_c`):

- The list's selection changing (GUI.dll FUN_100d0704): with an offer selected whose place is known
  (`N3Msg_GetQuestWorldPos`), the window sends the signal the journal's "Upload to map" sends, with the offer, its
  place and its playfield. The receiver (FUN_10067d5f) takes away the waypoint it has, keeps the new one with the
  mission it is of, and tells the two lines ("MissionWptUploaded", then how it shows here). With no offer selected the
  window sends the other signal, and the waypoint goes if it is that offer's (FUN_10067956); the window going sends
  it too (FUN_100d04fe).
- A new mission (`InventoryGUIModule_c::SlotGotNewMission`, 0x100c742e) uploads its place by itself: the original's
  Accept closes the window, which takes the offer's waypoint away, and the accepted mission's comes with the mission.

Ported: `MissionWindow::mark` (asked every frame, the window open or not), `missionlog::upload_lines` and
`waypoint_goes`, `Hud::waypoint_of` (the mission or offer a waypoint is of), `MissionLog::news`. The dot is the planet
map's white one and the compass's yellow mark, as for the journal's upload; the planet map's Mission button centres on
it. Tests: `a_selected_offer_marks_the_map`, `a_new_mission_is_uploaded`. **Not tried live.** A guess: what makes the
original's new-mission signal was not read; here a mission is new when the server's list has one that the list before
did not have (the session's first list is not news). Not done: the offer's right-click menu (its page, and "Upload to
map" again), the window closing at Accept, the tip the original shows at a new mission (`OnReceiveMission`).

**Accept closes the terminal's window (2026-11-15; the user: "accpting a mission doesn't clear the mission window").**
The original's Accept button (GUI.dll FUN_100d0c61) sends `N3Msg_SelectMission` and then deletes its window (its first
virtual function, the one `SlotCloseMissionSelection` calls with 1); the next use of a terminal makes a new window
with no offers. With fewer than two free inventory places it shows its error box instead and the window stays. Here
the window stayed with its offers and a status line. Now the network thread tells the window when the acceptance is
sent (`Update::mission_accepted`) and `MissionWindow::accepted` closes it and drops the offers; the refusal leaves it
as before. The accepted offer's waypoint goes with the window and the mission's own comes with the mission (the
paragraph above). Test: `accepting_closes_the_window`. **Not tried live.**

**The missions stack in the journal (2026-11-15; the user: "missions should be able to stack in the missions window
not replace another").** The journal took every `QuestFullUpdate` for the whole list. It is not: the user's log has one
mission accepted in Old Athen and one in ICC, and the second message holds the second mission alone. The original's
handling (`QuestFullUpdateIIR_t`, Gamecode FUN_100acfde) adds each of the message's missions to the character's list
(FUN_10056cdd) and never empties it. The message's last byte (the schema's `Unknown28`) says the missions are new: 1
at an acceptance, 0 when the server tells the missions again after a zoning (both in the log). With it the original
says "Feedback_YouGotANewMission" and sends the signal `SlotGotNewMission` answers, which uploads the place.

Ported: `missionlog::Arrived` (the missions of a message with that byte, or a zoning) and `MissionLog::arrived`; the
network thread sends them in their order. A zoning empties the list, as the original's character is made anew there,
and the server's telling fills it again (seen twice in the log, right after the zoning). This client's own: a mission
that is in the list already takes its old row instead of a second one. The new-mission upload now goes by the byte
(the guess of the paragraph before last is gone), and the feedback line is told. Tests:
`a_second_mission_stands_beside_the_first`, `a_new_mission_is_uploaded`. **Not tried live.** Not known: whether the
server tells several missions in one message or in one each after a zoning (either works here); the original's other
condition for the new-mission line (a flag of the character, +0x140) is not looked at.

## The turn messages and the word of one's own death (2026-11-15)

The user: "dying takes too long to respawn. on a related note can fix zoning when walk by a teleporter but not in".
Both were in their log of that evening (the message log was on): zoned to Omni-1 Trade from ICC while jumping past
the Whom-Pahs, killed there three seconds on, and set down in West Athen 59.8 s after the death.

**The teleporter.** The server's teleport message (`43197d22`) begins with the place it has the character at. For the
death it is the character's own last place; for the Whom-Pah it is (3177.6, 870.6), half a metre from the northern
booth, while this client had the character seven metres off at (3172.3, 874.8) and had told the server so. So the
server keeps its own copy of the character and moves it itself, and the two had parted. Why: the turn messages. The
original's transition table (Gamecode FUN_1006cb58) has case 9 `TurnRightStart` (10 the mouse's), 11 the right turn's
stop, 0xc `TurnLeftStart` (0xd the mouse's), 0xe the left turn's stop; the captures agree (the heading grows from a
9 to its 11 and falls from a 12 to its 14, and they come in pairs: 876/885 and 852/852 in the long Arete capture).
This client sent a left turn as 9, a right one as 11 and every stop as 12, from a guess of the first captures. So
every turn by the keys ended with "left turn begun" to the server, which turned its copy of the character on and on
while it ran: a circle of a few metres beside the real path, until the next message. The copy walked into the booth.
(It is likely what others saw of this client's characters too, and it may be behind other oddities of the server's
answers near things; none was looked at again.)

Now `movement::TURN_RIGHT_START` 9, `TURN_RIGHT_STOP` 11, `TURN_LEFT_START` 12, `TURN_LEFT_STOP` 14, and
`net::turn_message` picks by the keys before and now (test `the_turn_messages_are_the_originals`). The client's
`HEADING` (a new heading while standing, 14) is the left turn's stop and stays. **The landing's message is gone:**
the client sent 16 (`JumpStop`) as the character landed; the original sends no 16 in any capture (some 870 jumps)
and nothing else at a landing. Not done: the mouse's turns (10 and 13; the client sends the full report 22 as the
heading turns, which the original also does). **Not tried live.**

**The death.** The original's `CharDie_t` runs each frame (Gamecode FUN_1007bad5): it waits for the death's animation
to play out (FUN_1003c6e7 of its handle); for the client's own character it then makes an effect (3000) and counts
three seconds (`_DAT_1015e6dc` = 3.0); then it sends `CharacterAction` 0x98 (the protocol's `Die`: no target, no
parameters) and is done. This client never sent it, and the server moved the dead character after a minute, which
looks like its own time-out. Now `net::OwnDeath` does the same (`Characters::death_played` asks the death's animation
on the character's stack; a death never seen to end counts as played out after 6 s, this client's own) and
`n3::DIE` is sent; the log says "the death has played out: telling the server". The white of the death
(`whiteout.rs`) begins when the death has played out and rises over the three seconds instead of the minute; that
the original's effect 3000 is that white is a guess (the effect was not read). Test:
`the_own_death_is_told_three_seconds_after_it_played_out`. **Not tried live**: no capture has the original's own
death, so that the server answers the 0x98 at once is read from the code only.

## An item of several actions says what it asks (2026-11-15)

The user: "not able to equip ganimedes on Arctangent". Their log has the try five times over: the item (290215, the
Ganimedes Personal Jetpack) used from the backpack, the client moving it to its place, and the server's answer each
time, a `FormatFeedback` that unpacks to the game's own line: the character's Vehicle Navigation, Airborne has to be at
least 81. So the server refuses, and by the item's own condition: the template's wield action holds `(139, 80, 2)`,
that skill above 80.

What was wrong here is that the client did not know it. `mao_formats::item::actions` took `22, 36, 2018` for a marker
before each action; the third word is the number of actions, `(count + 1) * 1009`, and 2018 is a chunk of one. An item
with two or more actions had none read: 1,903 templates have two, 8 three and 1 four (the `itemactions` example's
`survey`), the jetpack among them (wield, 8, and an action 136). Their item windows showed no "To wield" or "To wear"
lines, so nothing but the server's line in the chat said why such an item would not go on. Now the whole chunk is read
(every action of it has to be whole, else the words are taken for something else), and the window's lines turn the
list's "not" (operator 42) over the condition before it instead of printing it as a condition of its own. Tests:
`a_chunk_of_several_actions_is_read`, the jetpack in `real_item_criteria`, the "not" in `gear`'s requirement test.
**Not looked at in the window** (no picture was made). Not done: the client still takes off for a moment when a flying
item is used and the server then refuses it (the log's "flight: on" and "off" a second apart).

## The name over one's own character holds still at a run (2026-11-15)

The user, with a recording: "name is hard to read when moving". Frame by frame (thirty a second) the name over the
running character stood up to fifteen pixels left or right and eight up or down of where it had stood a frame
before, while the character itself ran steady in the middle of the picture. Two causes:

- **Sideways.** The window lets the own character and the camera glide on between the network thread's updates, at the
  speed it was walking (`move_follow`); the name tag stood where the last update had the character and jumped on with
  the next. Now a tag knows it is the own character's (`nametag::Tag::own`) and the window puts it where it draws the
  character, each frame.
- **Up and down.** The tag stands over the body's height plus 0.4. The original measures that height once, of the mesh
  at rest (`N3Msg_GetMeshHeight` in GUI.dll FUN_10025603, when the tag is made); here it was the height of the body as
  posed that frame (`blobshadow::Size`, a sphere round its points), which changes with every stride. Now
  `nametag::steady_height` keeps the first height seen of a character and takes another only when it is a fifth off or
  more (another mesh or scale). This client's own rule, standing in for the mesh at rest, which is not measured here:
  a character first seen in the middle of a stride keeps that stride's height, a few hundredths off. Test:
  `a_tags_height_is_kept_steady`.

The second holds for everyone's tag. **Not looked at in a running game** (the recording was the user's; no new one was
made).

## No take-off before the server has taken the flying item (2026-11-15)

The user: "fix the takeoff flicker when the server refuses a flying item". An item's move is shown in the list at once
and taken back when the server refuses it (`World::moves_asked`); the client's own reading of flight (something worn
whose wear event has the flight spell) went by the list as shown. So the jetpack the server would not have lifted the
character for the second until the refusal came (the log's "flight: on" and "off", five times). The original flies
when the server's spell list says so, which can only come after the move is taken.

Now `net::worn_flight` goes by where each item was before the moves still asked: a flying item put on lifts the
character when the server has taken the move, one taken off sets it down when the server has taken that, and in a
swap each of the two counts where it was until the answer. Test: `a_worn_item_flies_when_the_server_has_agreed`.
**Not tried live.** With it a take-off waits for the server's answer, a tenth of a second or so. A move the server
never answers (seen once with a key swapped in right after a taking off) leaves the item counted where it was.
Not done: the original asks itself whether the item can be worn before it asks the server
(`N3Msg_IsItemPossibleToWear`, the item's own check at its +0x5c); the client's `wear_refusal` judges the wear action
(6) only, and which action the original judges for an item of the weapons' page (the jetpack's is wield, 8) was not
read, so the jetpack is still asked of the server and refused there, with the server's line.

## The Jobe Whom-Pah says it needs Shadowlands (2026-11-15)

The user: "when a free player tries to enter the jobe whompah give an error that they need the Shadowlands expansion".
Their log has four tries at Old Athen's fourth booth and nothing from the server for any of them: it neither zones
such a character nor says why. **This is the client's own addition**, not the original's: what the original shows a
free account in that booth was not seen (no capture), and its door code was not found to ask anything of the
expansions.

- **Which booth leads to Jobe** is in the game's data after all: every booth has a sign over it (a statel of
  `sign_whompa_default` at the booth's place) with a texture named for where it leads (`new_whompa_sign_tir`,
  `..._newland`, `..._omnitrade`, `..._wailing_wastes` ...), and Jobe's is `new_whompa_sign_jobe` (older stations:
  `teleportersigns_omni-whompa_jobe`). `whompah::jobe_booths` finds the statels with such a texture;
  `Area::jobe_booths`. In the game's files: Old Athen (449.0, 317.7), ICC (3183.1, 866.0), which the learned
  `whompah.ini` agrees with (the booth's four doors share one template, "Dummy Whom-Pah noselect", and say nothing).
- **Whether the account has Shadowlands**: stat 389, bit 1 (the original's text for a condition on that bit is
  `Feedback_YouMustHaveExpansionpack1`, Gamecode FUN_1000556a; bit 0 is the Notum Wars booster, 3 Alien Invasion,
  5 Lost Eden, 7 Legacy of the Xan). A stat the server did not send counts as 0 once the character's stats are in.
- **The error**: as the teleport try starts at such a booth (`whompah::needs_shadowlands`), an error box with the
  game's own text of that name (text.mdb, category 110). The try is sent all the same, so an account wrongly judged
  still zones. The log says what the stat was: "the booth to Jobe: the account's expansions (stat 389) are ...".

Tests: `the_jobe_booth_asks_for_shadowlands`, `the_games_jobe_booths` (`MAO_GAME`). **Not tried live**: that the
server sends stat 389 to a free account's character with bit 1 clear (and to a paying one with it set) is not
seen; the log's line at the next try shows it. Not done: the signs would give every booth's destination without
learning it (the labels over the booths still come from `whompah.ini`).

## No tags over the Whom-Pah booths (2026-11-15)

The user: "now that whompah signs are showing you correctly can we remove the tag thing". The client drew a tag over
every booth ("Whom-Pah > Tir", or plain "Whom-Pah" where the destination was not learned yet, and over an interior's
way out), on top of everything in the world; it was this client's own, from before the booths' signs had their
textures. The signs say where each booth leads, so the tags are no longer drawn (`ui.rs`, the labels' loop). The
booths' labels are still made: they fill the Whom-Pah window (its Go button walks to a booth) and the corner map's
dots. Looked at: Old Athen with the interface (`MAO_UI_PREVIEW=1`), no tags over the station.

## The organization's question boxes (2026-11-16)

The user: "do all of the Readable from the decompile now". First of that list: `/org leave` and `/org disband`,
which the original asks about in boxes before it sends anything. GUI.dll's `OrganizationGUIModule_c` has six such
boxes, all ported (`orgboxes.rs`; the table of who raises each, its title, its text and what its yes sends is in
the module's head):

- `/org leave`: a warning box; yes sends the organization command 0x10 with nobody. The box's line is this
  client's own words: the original's is written into GUI.dll, not into the game's texts.
- `/org disband`: the game's `Org_ConfirmDisband`; yes sends command 6 with the selected target.
- From the server (`OrgServer` types 5, 7, 8 and 9, `mao_net::n3::org_ask`): an invitation (yes: command 0xf with
  the organization), tax owed on leaving (yes 0x1d, **anything else 0x1e**), the tax changed (yes: 0x10 with the
  selected target), a promotion to confirm (yes: command 10 with the member and the byte 1; only when the member
  is in sight, since the box names them).

The titles "OrgDisband", "OrgLeave" and "OrgJoin" look like names but are what the original's code hands its
windows as titles. Tests: `the_boxes_answers_are_the_originals`, the `org_ask` lines in `mao-net`'s text command
test, `org_commands_come_to_their_messages`. **Nothing of this was sent to a server or seen come from one**: the
four server messages' layouts are from the reader's code (Gamecode FUN_10126d95) alone. Not done: the timer the
original looks at before accepting an invitation (it says the time left instead of asking the server); the leave
and promotion boxes come up at the pointer, not in the middle.

## The chat commands `/cc`, `/rp`, `/name`, `/gfx`, `/bank close` (2026-11-16)

Second and third of the user's "do all of the Readable from the decompile now". Each handler was read down to
what it sends before anything was written; **none was sent to a server**.

- **`/cc <words>`** (GUI.dll FUN_100ba6f4): the words after it, each with its `%` arguments put in, go to the chat
  server in packet 120 (FUN_1016caf7; packed `sI`: the list of texts and the chat window's number, 0 here).
  `mao_net::chat::command_packet`. The original's `Befriend` script file is `/cc addbuddy %1`, so that script now
  runs. Without a word the original says its syntax is wrong; here the usage line.
- **`/cc info <name>`** (exactly those three words): not a command packet. The name is looked up and packet 110
  goes with the character's id and a map of two entries, `commane` (so spelt in the program) to `ccinfo` and
  `destination` to `chatserver` (`info_request_packet`). The original says it is asking, in a line of the
  program's; here a line in this client's words. **What the server answers is not known**: nothing reads an answer
  yet, so it would show in the log as an unread packet.
- **`/rp`** (FUN_100b27c2, Gamecode `N3Msg_EventFeedback` 0x1001cc3d): the character's stat 673 with bit 0x200
  turned over, sent as `CharacterAction` 0xa6 with the new value as the second identity's number
  (`n3::ROLE_PLAY`). The server is expected to set the stat; what it answers is not seen.
- **`/name <request>`** (FUN_100baba6, FUN_10089940): a tell to the character called `name-request` whose text is
  **all that follows the command** and whose blob is empty; the game's
  `ChatCmdFeedback_Name_SendingNameChangeRequest` is said. **Tried live on 2026-11-16** (see "`/name` live"
  below). **Not to be tried on a character that is to keep its name.**
- **`/gfx enable|disable <number>`** (FUN_100b3863, `N3Msg_EnableGFX`): an earlier note here had the words the
  other way round. Nothing is sent: the effect template of that number is put in, or taken out of, the storage's
  set of switched-off templates, and a switched-off template is not found by the lookup every effect starts with
  (Gamecode FUN_10107068; here `mao_formats::gfx::Gfx::get` and `set_enabled`). A first word that is neither,
  written just so, does nothing, as in the original.
- **`/bank close`**: shuts the container window while it shows the bank. `/bank open` is a game master's and does
  nothing.
- **`/bug` is knowingly not carried out.** The original asks in a box and hands the text to
  `BugReport_t::SendUserMadeBugReport`: its makers' own bug database, about their own client. A report from this
  client would be a false one, so the command says so and sends nothing.

Tests: `the_sent_packets_are_framed` (the three packets' bytes), the command lines in `chat_commands` and
`chatwindow`, `a_switched_off_template_is_not_found`.

## Steering with the mouse sends the original's messages (2026-11-16)

Fourth of the user's "do all of the Readable from the decompile now". Until now the right button's steering sent
only the periodic full report as the heading turned. The original (`N3Msg_MouseMovement`, Gamecode 0x100196b3;
`N3Msg_EndMouseMovement` 0x10019aa0; `N3Msg_MovementChanged` 0x10018bc4):

- A mouse movement to the right while the character turns left or not at all sends movement 10
  (`MouseTurnRightStart`); one to the left while it turns right or not at all sends 13. Any other movement only
  turns the character where it is (the movement 0x2b, which is not sent: it marks a report as due). So a steering
  sends 13, 10, 13, 10 as the mouse wavers, which is what the capture of 2026-11-09 has.
- Letting the button go sends the stop of the side the turn was on: 11, or 14.
- While the mouse steers, the turn keys step sideways: 12 becomes 7, 9 becomes 5, and their stops 8 and 6.

Here: `net::mouse_turn` (the side, and the message when it changes or ends), the turn keys counted as strafe keys
while `keys.face` is set, `movement::MOUSE_TURN_RIGHT_START` and `MOUSE_TURN_LEFT_START`. Test:
`the_mouse_turns_are_the_originals`. **Not tried live.** Differences kept small and known: the original counts a
movement straight up or down as one to the right; when the steering begins during a key turn it first stops that
turn and starts the strafe (here the same messages come from the keys being counted as strafe keys); at its end it
sends both strafe stops where a converted key was held (here the one that was running). Not ported: the limit on
the mouse's turn the original applies while stat 224's bits 2 or 4 are set and stat 267 is there.

## The original's own wear check, read to its end (2026-11-16)

**Corrected on 2026-11-18** (see "An item is judged before anything is taken off" at the end of this file): the
first point below is wrong for an item in the inventory. Such an item is judged by another function, and one of
the weapons' page by its wield requirements; the jetpack is refused by the original itself.

Fifth of the user's "do all of the Readable from the decompile now". `N3Msg_IsItemPossibleToWear` (Gamecode
0x10026c57) and what it calls were read whole. What that changes:

- **A correction to the note of 2026-11-15** ("the original asks itself whether the item can be worn ... the
  jetpack is still asked of the server"): there is nothing more to do for the jetpack. The check judges the
  item's **wear** requirements (action 6) and nothing else, for a weapon too (`WeaponItem_t`'s check FUN_10087a42
  is the same test; an item with no action 6 passes, FUN_100816f6). The wield requirements (action 8,
  FUN_1009c967) are not asked there. So the original asks the server about the jetpack as this client does, and
  is refused there.
- **Ported: the equipment's lock.** The first thing the check asks (FUN_100266c9), and the take-off check with it
  (`N3Msg_IsItemPossibleToUnWear` 0x100268df): while the value `LockEquipment` is on, nothing goes onto a wear
  place or off one, and the game's `Feedback_EquipmentLocked` is said. `net::equipment_locked`; the value is the
  original's (false in the game's `CharPrefs.xml`), set here with `/setoption LockEquipment true`. Test:
  `the_locked_equipment_stays_as_it_is`. Not here: the original's second lock, single tabs locked from the wear
  window (`EquipmentLocks`); this client's wear window has no such locks.
- **Read and knowingly left to the server**, because each needs something of the character the client does not
  keep, and a wrong "no" would stop the player where the original lets them: an implant needs bit 0x10 of stat
  224 (set at a surgery clinic by a spell the client does not run; else `Feedback_AccessingImplants`); a weapon
  on the places 9 to 14 needs that many deck places (stat 45, the belt's; else
  `Feedback_ThisItemRequiresComputerDeck`); social armour against other armour (two rules, by a flag of the
  character's and by the worn pieces' stat 702); an item that is not one's own. They are listed in
  `World::wear_refusal`'s head. The server refuses each of these moves and the client takes the move back.

## Every Whom-Pah booth named from its sign (2026-11-16)

Sixth of the user's "do all of the Readable from the decompile now" (this one is read from the game's data, not
from the program). A booth's destination was only known once the character had been through it (`whompah.ini`);
any other booth read plain "Whom-Pah" in the Whom-Pah window's list and on the map.

Every booth has a sign statel at its own place whose texture is named for where it leads
(`new_whompa_sign_tir.png`, `..._bliss.png`, `new_whompa_signs_omnitrade.png` ...): 77 booths in 27 playfields, 31
names. The names do not say playfields; the table `whompah::SIGNS` gives each its playfield, and it is not a guess:
in the game's data the network is the same both ways (where a sign leads, a sign leads back), which with the
playfields' own names fixes every entry, and the one booth checked against the server (Old Athen's, to Tir) reads
as it was found. Three playfields have two stations: Broken Shores and The Reck (one of each side), and The
Longest Road, whose clan station is Bliss. Signs that name a town are labelled with it: "Whom-Pah > Bliss (The
Longest Road)", "20K (Pleasant Meadows)", "2HO (Stret East Bank)", "Hope (Mort)", "Wine (Belial Forest)", "ICC
(Andromeda)", "Rome (Rome Park)".

`whompah::signs`, `sign_destination`, `sign_at`; `net::booth_labels` takes what the server showed first, then the
sign. When a booth is taken and its sign said another playfield, the log says so ("has a sign to ... and led to
..."). The Jobe booths of yesterday are the signs that lead to playfield 4530 (a third one turned up: Rome's).
Tests: `a_signs_name_is_its_destination`, `the_signs_lead_both_ways` (`MAO_GAME`: every placed sign known, the
network both ways, 77 signs). Tool: `cargo run -p mao-formats --example whompasigns -- <game>`. **Not looked at in
the window** (no picture taken; the list's text is the label's).

## Mail: cash on delivery and Return To Sender (2026-11-16)

First of the window gaps in the user's "do all of the Readable from the decompile now". From GUI.dll's message
window (FUN_10107fc8, FUN_10107b87, FUN_1010733d, FUN_10106692, the return handler at 0x101063d4) and Gamecode's
`N3Msg_ReturnMail` (0x10023a8c):

- **A mail's credits are signed.** A negative amount is cash on delivery: what the receiver pays to take the
  mail's things. The client read the number unsigned and had no field for it.
- **Sending.** The Credits field's number goes if it is 1 or more, else the C.O.D. field's negated. A negative
  number in either field: the game's `Feedback_MailNoNegative`. A text over 2000 letters (4000 bytes) or a subject
  over 64: `Feedback_TextSubjectTooLong`. The name goes with a capital and small letters. With money or a C.O.D.
  the window asks first (the game's `ConfirmSendMail` and `ConfirmSendMailCOD`, naming the amount, the item or
  "no item", and the name).
- **Receiving.** Money sent along shows in Credits and a C.O.D. in its own field, the other "0". Take All on a
  C.O.D. mail asks first (`ConfirmTakeMailItems` with the amount). The delivery choice shows how the mail came:
  bit 5 of its flags is "express" (the client had taken that bit for "has credits").
- **Return To Sender** is on for a mail not yet taken, not barred from it (bit 4 of its flags), that has an item or
  something to pay; it sends the mail message's action 7 with the mail's number (`mail::return_to_sender`) and
  the list is asked for again.

The two boxes' titles ("ConfirmSendMail", "MailTakeItems") are what the code hands its windows. Tests:
`cash_on_delivery_and_the_way_back`, `a_mail_is_returned_and_its_credits_are_signed`, `mail_matches_the_capture`.
**Nothing of this was sent to a server**: no capture has a C.O.D. mail or a returned one, and sending needs a
paying account. Kept from before and this client's own: a mail without a recipient or a subject is not sent.

## The shops that only sell: the `ShopBuy` view (2026-11-16)

Second of the window gaps. The trade view has three kinds (GUI.dll FUN_100e1074): with a player, with a shop, and
`ShopBuy`, which the client never used. Which one a trade gets is decided as it starts (Gamecode FUN_1006692a,
raising `InventoryGUIModule_c::SlotStartTrade`): a shop whose type (stat 156; 61 when it has none, FUN_10099ae4)
is 61 is a `ShopTrade`, **any other shop is a `ShopBuy`**: the stock and the bought
items, no "Sold items" list, and nothing of one's own goes in (the view's flag at +0x1a8). (This section first
called 61 "the Computer Literacy kind". **The type is the stat the shop is paid in, and 61 is credits**: see "The
test character's run" below.)

Here: `shop::Rates::buy_only`, `ShopState::buy_only`, `ShopWindow::wanted_view`; an item dropped on such a
shop's lists or sent to it from the inventory's menu is not put in. Tests: `a_shop_of_another_type_only_sells`,
`the_skin_has_the_shop_view` (the game's `ShopBuy` view has no place for one's own items). Looked at: the preview
with the captured stock, both ways (`MAO_UI_PREVIEW=shop MAO_CAPTURE=<log>`, and `MAO_SHOP_BUY_ONLY=1` for the
buy-only view): Shop and Bought Items over the buttons, no Sold Items. **Not seen at such a shop live.** Which
shops they are is surveyed since (`shopkinds`): the vendors paid in points. Not done: the lists' kept columns
(`shop_listview_config`), and the money shown at such a shop (the cost is drawn as a plain number, with nothing
saying it is points and not credits).

## The option window's tabs and its "Fixed keys" (2026-11-16)

Third of the window gaps: `OptionPanel/HotKeys.xml` was not used. The original's option window (GUI.dll
FUN_100c4315) has three tabs: "Preferences" (the pages of `Root.xml`, which was all the client had), "Fixed keys"
when `HotKeys.xml` holds anything (`HotKeyPanel_c` FUN_100bf5a2, filled by FUN_100beee3: a tree of `HotKeyGroup`s
holding `HotKey`s, each a label and a key command's id with the key the command is on), and "Key bindings" (its
editor of every key, FUN_100bcfb3).

Here (`gui/options.rs`): the window has the tabs; "Fixed keys" reads the skin's file (`fixed_keys`) and shows
each command with **this client's** key for it (`Keymap::original_key`: the four target keys and the eight
camera keys; the chat's history, reply and coordinates commands have no counterpart here and show no key). The
game's own file has three groups of 4, 4 and 8 commands. "Key bindings" is not the original's editor: it says
that this client's keys are set in its own settings window and has the button to it. Tests:
`a_hot_keys_file_is_groups_of_key_commands`, `the_games_fixed_keys` (`MAO_GAME`). Looked at:
`MAO_UI_PREVIEW=optionpanel MAO_OPTION_TAB=1`. Not done: the original's key editor and its "enter a key" box
(FUN_100bf286); the fixed keys are shown and not changed here, which is what "fixed" means in the file, but
whether the original lets its fixed list be edited was not read.

## `/name` live, and how the original cuts a command's line (2026-11-16)

`/name` was sent for the first time, on a character of the user's that they named for it. What came of it:

- **The first try was refused, and that was this client's fault.** `/name Buzzleton` got the tell "Please check
  your request, it was not handled properly by this system" from `Name-Request` (the chat server knows the
  character: id 0x2001; it also shows as a friend come online for a moment). `/name help` then gave the service's
  own language: `nick <name>`, `first <name>`, `last <name>`, `<first|last> remove`, `policy`, `status`,
  `delete all`, `delete <id>`. A request is two words, and the client sent the first word only.
- **Why.** The original does not cut a line into all its words. Each command is registered with a number of
  pieces (GUI.dll FUN_100bb3e0, FUN_100b2ac9, FUN_100b4001 hand it to FUN_100a3c6a, which keeps it at +8 of the
  command's record); the runner FUN_100a3d83 gives that number to `String::Tokenize` (Utils.dll 0x1000e2a6), which
  cuts at blanks until one piece fewer than the number is had and makes **all the rest the last piece**. `/name`
  has 2, so its second piece is everything after the command; the handler's "second word" was read as one word.
  The numbers are in `chat_commands::PIECES`. The same mistake sat in `/invite`, `/kick`, `/leave` and `/ignore`
  (all 2): what follows them is the name, whole. A quoted piece keeps its quotes; that is not followed here (only
  the group commands keep quoted names).
- **With the whole request sent:** `/name policy` (nick changes only below level 20 or for a name against the
  policy; first and last names at any level), `/name status` ("no name requests in the queue"), `/name nick
  Buzzleton` ("entered into the queue. It may take up to 24 hours ... changed while you are offline") and
  `/name status` again ("Nick name to be changed to buzzleton", with the request's number). `/name delete all`
  takes a request back; that was not sent.
- The service's help writes its placeholders in angle brackets, and they do not show ("/name  " for "/name
  <type> <request>"). That is the original's doing too: `HTMLParser_c::_ParseTag` (0x1015cced) takes a tag of a
  name it does not know for one of the kind it gives `section` (5) and says the tag was read, so it is not text.
  Only an empty tag and an `a` without `href` are left as text.
- `MAO_TRACE_CHAT=1` now also prints each tell as it came (sender, text with its markup, blob).

Tests: `lines_become_commands` and the `/name` lines in `chat_commands` (the whole request; `PIECES` names known
commands only).

## The test character's run of 2026-11-16: the new commands live, and what it found

The free account's test character (Arete Landing) was taken through what was built from the decompile earlier
that day, unattended, with the log and pictures of the client's own window (`win_shot_live.py`, PrintWindow).

**What works as built:**

- `/rp`: the server sets stat 673 as asked (543 with the bit, 31 without; told back twice each time).
- **Steering with the mouse**: a turn to the right sends 10, a swing to the other side 13, the letting go 14
  (the left's stop), nothing while the turn goes on the same way. Sent from a new script step, `face@<radians>`
  and `face@off`, which steer as the held right button's mouse does. Not seen from a second client.
- **The equipment's lock**: `/setoption LockEquipment true`, then a helmet to the head: the game's line about
  locked equipment and nothing sent; with the lock off the helmet goes on.
- **`/org leave`** on a character in no organization: the warning box at the pointer, Yes sends command 0x10 and
  the server says the character is in none. (Leaving a real organization was not tried.)
- `/cc addbuddy <name>` and `/cc rembuddy <name>` add and remove a friend; `/cc help` lists the chat server's
  commands for a player: `help [command]`, `addbuddy <player>`, `rembuddy <player>`, `rembuddy ?`. **`/cc info
  <name>` is not among them and gets no answer at all** (packet 110 goes and nothing comes back): it is
  probably not a player's command.

**What was wrong, and is mended:**

- **The chat server's own lines were not read.** It answers `/cc` with packet 37, `S2C_SYS_MESSAGE_LOCAL_FMT`
  (GUI.dll's reader FUN_1016d6f4, "IIID"): a character, a chat window, the number of a text in the game's
  category 20000 and its fill-ins, each a letter and a value (`I` a number, `S` a text, `l` another text's
  number), fed into the text in order (`LDBformat`). Packet 36, `S2C_SYS_MESSAGE`, is a line as it is. Both
  become a `SystemMessage_t`, and `ChatGUIModule_c::HandleSystemMessage` (0x1008633c) puts one that is about a
  character and for no window among that character's tells, with no name in front, and any other out as a system
  line. Here: `chat::Packet::System` and `SystemText` with `chat::Fill`, `net::chat_system_text`. The window a
  line is for is not followed. Tests: `the_chat_servers_numbered_lines`, the packets in `the_captured_packets`.
- **A friend taken off by the server stayed on the list.** Packet 41, `S2C_REM_BUDDY` (the character's id), was
  not read; the reader takes the character out of its list, and so does the friends list now
  (`Packet::BuddyRemoved`). Seen: the friend under Offline Friends after the add, gone after the remove.
- **The shop "type" is the stat the shop is paid in.** The survey the `ShopBuy` section lacked (`cargo run -p
  mao-formats --example shopkinds -- <game>`): of the 730 item templates with shop prices, 705 have type 61 or
  none, 1 has 68, 20 have 669, 3 have 672 and 1 has 695, and the others are the vendors paid in points (the
  veterans', the Ofab ones, the Phasefront ones, the ICC reward terminal). Gamecode agrees: what a character can
  afford is its stat of the shop's type against the cost (FUN_1009a69a), and Computer Literacy moves the prices
  only where the type is 61 (FUN_10099e9b, FUN_1009a12d). 61 is the character's credits. The code said "the
  Computer Literacy kind"; it is `CREDITS_SHOP` now, and `a_shop_of_another_type_only_sells` checks two of the
  vendors and a plain booth against the game's data. No behaviour changed.

Not a fault, noted so it is not chased again: the strip of plain lines at the bottom of the screen in the test
layout is the client's own message strip, which writes each chat line as plain text, so a several-line answer
reads as one paragraph there; in the chat window the same line breaks where the server broke it
(`MAO_UI_PREVIEW=chat MAO_CHAT_LINE='one\ntwo <b>'`, new, shows a server line as it comes).

Left as they were found: the test character wears the helmet now; the friend was removed again; the lock is off
(it was set in the test's own settings folder).

## Character creation as the original's four scenes (2026-11-16, `ccscene.rs`, `charcreate.rs`)

The New Character window of 2026-10-29 is gone. In its place is GUI.dll's `CharCreateModule_t` (0x1011cbcf): the
station's world, a camera that flies between the scenes, and the scenes' own views. What the original does, and
where it is here:

- **The world** (`hall::creation`): the module's six meshes (its table at 0x102724e4: the station, the two of
  the profession hall, the intro's `charactercreation_seq01`, the nano effect, the adventurer). The hall's four
  show only from the profession scene on. The seven breed figures (`SlotMeshReady`: each breed's body at the middle
  build and height, table 0x10272490) stand on the main mesh's connectors `breed_0` to `breed_6`, the chosen
  character on `character_0` (breed scene), `character_1` (appearance) or `character_3` (name), turned as
  `CCCharacter_t::MoveToConnector` turns it (`hall::on_connector`; `mao_formats::model::connectors` is new). What
  comes and goes is the station scene's overlay, rebuilt when it changes (`Launcher::creation_camera`).
- **The camera** (`ccscene::CameraFile`, `Mover`): `CharCreateCamera.dat` has 53 cameras and 23 transitions; the
  mover is the module's camera control (step FUN_1011698d): eased time, one speed along the path, most of the old
  camera kept each frame, Escape to the end. The renderer's camera can roll now (`Camera::up`): the way down
  through the station does.
- **The states** (`ccscene::Director`, the module's `FrameProcess`): the intro in space (transition `5 1`, the
  opening music, the two lines of text after 4 s), the way down (`1 1 1`), then breed, appearance, profession and
  name **in that order** (the old window had profession before appearance), and after the server has made the
  character the walk to the shuttle (`4 3`) before the game starts. Each scene's camera orders are in
  `Director::go`, read from the scenes' `SlotNextPressed` / `SlotPrevPressed`, `MoveCamereToHead`,
  `MoveCameraToFullView` and `ProfessionChanged`.
- **The views** (`Creating::show`): every button, label and text area at the place its constructor gives it
  (`charcreate::place`, shares of the window), with the skin's `GFX_GUI_CC_*` pictures (the numbers the code passes
  are AFCM.dll's table of names: `gfxids.py` in the notes), the purple 0x6666aa boxes, and the texts of category
  600 by the names the DLL uses. The breed is picked by clicking a figure (the line from the eye through the mouse
  against each figure's box); the mouse over a button or a figure changes the text and plays the mouse-over sound;
  the mouse over the head's area takes the camera to the face. A scene's views show only while the scene runs, as
  `StopScene` and `StartScene` have it. The exit button asks first (`AskExitMessage`, the box "ExitCC").
- **Sounds**: the scenes' voice lines, each breed's and profession's line, the select, mouse-over and error
  sounds, the ambience, by the original's names; the launcher starts the sound module when the creation begins.
  Two names the name scene asks for (`SM_Sandy_CC_Nick`, `SM_Sandy_CC_Name`) are in no sound file: silent in the
  original too.

**This port's own choices, to be told apart from the original:** a wide window sees the same height as a 4:3 one
(the hall's rule, the user's choice of 2026-11-11), where the original cuts the figures at the knees; the fonts
are egui's Verdana. The buttons are the original's: their width goes by the window's width over 1024 and their
height by its height over 768, so a wide window pulls them wide (first built unstretched; asked, the user chose
the original's on 2026-11-16).

**Seen:** all of it in a demo window with no login (pictures at 1280 by 900 and at 2560 by 720 beside the
capture's own shots of 2026-10-30, shots 36 to 108): the intro, the way down, the four scenes, the face's camera,
the profession's cameras, the walk to the shuttle. `MAO_LAUNCHER_DEMO=1 MAO_LAUNCHER_CREATE=intro` (or `0` to `3`
for a scene) with `MAO_CC_SCRIPT=<seconds>:<step>,...` runs it by itself (steps in `charcreate.rs`).
**Not tried:** a creation sent to the server through the new scenes (the messages are the old ones), the mouse
(the script chooses directly), the sounds by ear (the runs were silent).

**Not done:** the meshes' own animations (the profession figures and the intro's ships stand still); the breed
figure's selection glow; picking by the figure's mesh (its box is used); the head's place from its attractor (the
figure's top less 0.12 is used); the half second a scene waits before the next begins; the two later lines after
a creation (`DockingCompleted`, `ContinuingDNA`: when they show was not read) and `TextWindowFade_t`'s own timing
(a second's fade here); the ambience is started once (the original asks for it every frame); the expansion's
extra heads are not offered; the choices are kept while the launcher runs,
not written down as the original's preferences are; the profession text area's height (156 units of the lowest
resolution in the original, a fifth of the window here); the camera's debug display.

**The Keeper and the Shade need the Shadowlands (2026-11-16, the user's report: a free account could pick them).**
The login's character list ends with the account's character slots and its expansion flags
(`CharSelectWindow_c::SlotCharListReceived` 0x1000ebd5 keeps the flags as the value `ExpansionFlags`);
`mao_net::session::login_account` gives them now, and the launcher's log has a line of them. As
`ProfessionScene_t` does, the two buttons are switched off when bit 2 is not set, and Next with one of them
chosen (a choice kept from another account) plays the error's sound and shows `CharCreation_SLOnlyProfession`
(`charcreate::shadowlands_professions`). Read from the two test accounts: the free one has flags 0x1, the paid one
0xab. Not ported: the original's other way in, the client's `nSLProfsEnabled` (where it is set was not read).

## Inspecting another character's equipment (2026-11-16, the user's report: "I wasn't able to inspect other players")

The Inspect window was there and the server's answer was read, but **nothing ever asked the server**. The
original's chain: the info page of another character ends with the link `chatcmd:///inspect <the character's
number>` (GUI.dll FUN_1003482c; ours carried the name), `/inspect` with an argument reads it with `strtoul` and
calls `N3Msg_Inspect` on the identity (50000, number) (FUN_100b280d), and Gamecode sends a `CharacterAction` with
action 0x105 on that character (`n3EngineClientAnarchy_t::N3Msg_Inspect` 0x1001ddd4). Here:
`mao_net::n3::inspect_request`, `chat_commands::Parsed::InspectEquipment`, `charinfo::ask_equipment`; the link
carries the number. The answer's window was titled with one's own name: whom an `Inspect` message is about is the
identity its data begins with, not the message's own (which is the receiver's).

`/inspect` with no argument does nothing in the original (it wants the number); here it stays this client's own
"tell me about my target", which opens the info page with the link.

**Tried live** (the test character, a player standing by): `/inspect <number>` brought the Inspect window with
that player's deck and hud items and the player's name in the title. Not tried: the click on the link itself
(the same command), a creature.

## A new character's first arrival: the inventory was one place off (2026-11-16, the user's report)

The user: on a character just made, equipping and the starter nanos did not work until a relog. Reproduced with
a new test character and a full trace (`MAO_TRACE_FULL=1`, new: every byte of a traced message).

On a first arrival the character's own list is empty and the server hands the starting items over one by one: a
`TemplateAction` (`35505644`) naming the item into the overflow, then a `ContainerAddItem` (`47537a24`) that
brings it into the inventory's first free place. **One of them, the survival pack, is a backpack and comes
another way**: a message of its own (`465a5d73`, on the pack's container identity, type 0xc749: the owner, the
playfield, and the item's stats), then the same `ContainerAddItem`. The client did nothing with that message, so
the pack took the fifth place on the server and none here, and every item after it was one place off: "use the
nano crystals" asked the server to use the pack (it opened the pack, which was seen in the trace), "wear the
staff" asked it to wear the crystals. A relog mended it because the character's list then has the places.

Now `backpack_given` reads the announcement (the owner must be the character, and the two bytes after the
playfield's words must be 0x71 0x6f, which the captures have for a pack on its way in and not for one already
in the list) and the pack waits in the overflow like the other items, so the `ContainerAddItem` places it.
Test `a_new_characters_pack_takes_its_place` replays the original's capture of 2026-10-30. **Checked live** only
as far as this: the fault was reproduced on a new character (the pack opened at the "crystals'" place), and on
its relog the server's list has the pack fifth and the crystals sixth, as the fix now numbers them; the starter
nano then cast and the belt went on. **A first arrival with the fix was not run** (it needs another new
character).

Along the way, about the same starting kit: the belt needs Computer Literacy 20, which the starter nano
(Composite Utility Expertise) gives; the kits need First Aid and Treatment and a seated character. Those are the
server's rules and its lines are in the combat channel.

## The character list's slots line (2026-11-16, the user's report)

Under the list the original writes the text `AvailableCharSlots` (category 10000) with the slots left and the
slots in all (`CharSelectWindow_c::SlotCharListReceived` 0x1000ebd5; a character takes a slot when the number
after it in the list has bit 1), and New Character is off when none is left. The client wrote "N characters".
The slots come with the login's character list (`mao_net::session::login_account`, `loginviews::set_slots`,
`slots_line`). Seen in the demo window with `MAO_LAUNCHER_SLOTS=8`; not seen after a real login.

## The inventory out of step after a quest's hand-in (2026-11-16, the user's reports from a new character)

Three faults, read from the user's log of the session and the original's capture of 2026-10-30:

- **A thing given to an NPC stayed.** The trade with an NPC is `KnubotTrade` naming an inventory place, then
  `KnubotFinishTrade`; the server's only word after it is `KnubotRejectedItems`, the items it gives back (none, as
  a rule). Nothing says "the item is gone": the original takes it out itself. The client kept it, and worse
  followed: the server's place was free again and the client's was not, so the next item to come took another
  place here than there, and the next NPC was given the wrong thing (the user's "Alex Gibbs doesn't accept the
  Bio Analyzing Computer": three trades, each answered as if nothing had been given). Now the given places are
  kept (`World::npc_given`) and taken out when the answer comes, save what is given back.
- **Stacks came without their number.** `TemplateAction`'s word after the quality is how many (a new character's
  recharger 50, its stims 25); the client took every item named so for one.
- **A use did not count down.** `TemplateAction` with action 3 at an inventory place is that many of the stack
  used (the last one goes with the `DeleteItem` that follows).

Test `stacks_and_things_given_away`. **Not tried live** (it needs a quest's hand-in on a new character).

Looked at and not a fault here: Shift and a click on an inventory item opens its info page (tried by script on
the test character, `sclick@`).

**Reported the same day and not done:** helmets that leave a blue spot (Flint Novak's); the Wounded Dockworker
standing where he should lie (his announcement has a state byte, 8 where the standing NPCs have 1 or 3, that the
client does not read: the place to start); the junk shop not showing; "some of the bounding boxes seem off"
(which ones was not said).

## A helmet's face (2026-11-16, the user's report: "helmets ... leave a blue spot")

Flint Novak's mining helmet (`opifex_male_helmet_mining_01`, and the Atrox one the dockworkers wear) is two
parts: `helmet`, and `face`, whose material has no texture and a plain blue colour and whose texture
coordinates are the head texture's. DisplaySystem fills it in (FUN_100739a2, when a mesh comes to the head's
place): a mesh there with a part named `head` gives its first material's texture as the character's face
texture (FUN_1007388b), and every mesh at the head's place with a part named `face` gets that texture put on
it (FUN_1006d3e2). The head is in the update's mesh list beside the helmet (flags 4 beside the helmet's 2), and
`shown_meshes` threw it away.

Now `shown_meshes` keeps the head a helmet stands in for as `n3::FACE_DONOR` (not drawn), and `net::wear` gives
the helmet's `face` parts that head's `head` material. One departure, said in the code: the original puts only
the texture on the face's own material; here the head's whole material is taken, because the face's own colour
is the modeller's blue. Seen offline for both helmets (`MAO_CREATURE_GEAR=80:<helmet>:0,81:<head>:0` with the
`creature` command); not seen live.

## A shop the server alone puts there: the Junk Shop (2026-11-16, the user's report: "junk shop isn't visible")

Arete Landing's Junk Shop (Alex Gibbs's, for the tradeskill quest) is not one of the playfield's own machines: its
dynel table has the ICC machines and no Junk Shop. The server announces it (`VendingMachineFullUpdate`, at
3527.4, 5.2, 858.1, beside the ICC Ammunition machine) with its template (`StaticInstance` 297281) and neither a
`Mesh` stat nor a `NameTemplate`. The client drew a machine only when its announcement had a model or the
playfield had its own copy, and named it only by its name template: this one was there to be clicked, unseen,
as "Booth d835".

Now a machine with no model in its announcement and no copy in the playfield gets its template's `Mesh` stat
(for the Junk Shop mesh 6546, `clan_fillobject_automat`), and a machine with no name template is named as its
template is. Test `a_shop_the_server_alone_puts_there_has_its_templates_model` (the original's captured
announcement, `MAO_GAME`); the model was looked at by itself offline. **Not seen in the world**: where it
stands and which way it faces are the announcement's, as for the other machines.

New: `cargo run -p mao-formats --example templatestat -- <game> <stat> <template>...`.

## Creatures announced on the ground: the Wounded Dockworker lies (2026-11-16, the user's report)

A character's announcement (`SimpleCharFullUpdate`) has, after the template, the scale, a word and a byte, a
counted block of 28 bytes that Gamecode's reader copies out whole (FUN_1007916d, FUN_10067b62). Its byte 12 is the
movement mode the character is in. Over the four captures' 4140 creature announcements: 3 for most (running), 2
for some (walking), 1 for the NPCs that never move (Rex Larsson, terminals, turrets), 4 once, and **8, sitting on
the ground**, for Arete Landing's Wounded Dockworkers and Wounded Clan Protesters and a handful of seated NPCs.
The client did not read it, so they stood.

Why the wounded *lie*: their template (0x48448) names its own animations for the ground's actions: sitting down
(0xd5) is `die-pain`, the ground's idle (0xd7) is `idle-sleep-ground`. The original's shot 474 of 2026-10-30 has
one flat on the deck.

Now `EntityInfo::move_mode` reads the byte (`npc_move_mode`), and a creature announced with 8 is `grounded`: it
holds the ground's idle, 0xd7 (0xd5's end for a body that has no 0xd7), without sitting down first, until it is
announced otherwise or stands up. Seen offline (`MAO_HELD=d7` with the `creature` command, new, holds an action);
not seen live. Not done: players announced sitting (their update's fields before the block differ and were not
read); the idle is held at its end where the original loops it.

New: `cargo run -p mao-formats --example creatureanims -- <game> <npc template>` lists a creature's actions and
animation files.

## The launcher hardened (2026-11-17, the user's report of a flash and someone's of a crash)

(2026-11-21, the user: "a white box logging in or to the character selection screen".) The login's window (`launcher.rs`, the login and the character list) was still made visible at once: `launch_probe` saw it on the screen at 52 ms and its first frame at about 600 ms, the system's white in between. It is now made hidden (`with_visible`), its first frames drawn from `about_to_wait` (a hidden window is asked for none), and shown once a frame is presented (`Gfx::on_screen`), or after two seconds; the pictures' small window stays as it was. Probed again: first seen at 447 ms with the hall drawn.

Nothing more was known of either, so the launcher's window was started in a scratch settings folder and its own
picture taken every 40 ms (`PrintWindow`, nothing else of the screen), and every place it could end was read.

What flashed, and what is done about it (`launchwindow.rs`):

- **The page was a white sheet for half a second** on the dark window, half a second after the window came: a web
  view put up at once is white until its page has drawn. Made hidden and shown when loaded, it was still white for a
  tenth of a second in three starts of four. Now it is made beside the window's inside, where a child window is cut
  off, and moved into its place 300 ms after it says it has loaded (or after eight seconds, as far as it has come).
  It is moved away, not hidden, while the settings are open. Ten starts of ten showed the sky and then the whole page.
- **The window stood empty for up to a second** while the graphics started. It is made hidden and shows with its
  first frame in it (drawn from the wait, since the system asks a hidden window for no frame); after two seconds
  without one it shows as it is.
- **A black frame now and then** (two in seven seconds in the pictures): the frame was a clearing pass and the
  interface's pass, submitted one after the other. It is one pass now (`Ui::paint_over` with a clear), and none was
  seen in twelve seconds. The frame after the page's opening, for which the surface is set up anew, is drawn at once.
- The window is taken off the screen before it is taken down (the page going was white for a moment).

What could end it without a word, and what is done (`crash.rs`, `main.rs`, `launcher.rs`, `mao_render::Gpu`):

- **The launcher's own window left no crash log at all**: only the login (`play`) and the game did. It is a run of
  its own now (`launch-window`, marker `run-launch-window.txt`).
- **The packaged program has no console**, so a panic or an error ended the launcher or the login with nothing on
  the screen: to a player, PLAY pressed and nothing came. Both are told in a box of the system's now (`crash::tell`:
  what happened and where `crash.log` is), once a run, for a panic of the main thread and for the error a run of the
  launcher or the login ends with. The box is the packaged build's (`hide-console`); `MAO_ERROR_BOX=1` asks for it in
  any build and `=0` forbids it. A game that ends badly is not told by the login that waited for it.
- **A window, a surface or a graphics device that could not be had was a panic** (`expect`) in the launcher and in
  the login. They end the run with their reason now. The launcher asks for its device leniently (`Gpu::lenient`): the
  settings' or the system's choice, then the same with only the limits the device has, then the system's software
  device; it is where another device is chosen, so it should open on anything. The login and the game ask as before.
- **The pictures and the page** (the game's background, the buttons out of `Anarchy.exe`, the emblem, the web view)
  run guarded (`crash::guarded`): a failure there is logged and leaves the plain window with its settings and PLAY.
- A dimension download whose thread went without an answer is a failed one (it left "Getting the dimensions..." up);
  a window of no size lays nothing out; a size of nothing (a minimized window on Windows) does not set the surface up.

To see them: `MAO_GPU_FAIL=<1-3>` (the first n devices fail), `MAO_LAUNCH_PANIC=frame` or `=pictures`, with
`MAO_ERROR_BOX=1` for the box. Seen in a test window on this machine only (Windows, an AMD card and the software
device). Not seen: the box on macOS, the hidden start on Linux and macOS, another machine's fault, whatever it was.
Not done: the login's window still shows before its first frame; minimizing and restoring showed no fault.

## The dead that did not become remains, and creatures that ran at a walk (2026-11-17, the user's report)

The report: "some of the mobs didn't turn into remains of <mob>", and the attack animation "seemed off". One
screenshot was in the folder (Arete Landing, 21:24: dead dockers lying under their own names beside one "Remains
of"), and its notes listed dockers twice and three times at the same spawn places. The game's log and the captures
gave the rest.

**What the server does at a death** (the captures, 288 deaths): the death's action (`CharacterAction` 99), then the
corpse's announcement 0.1 to 0.9 s later, then the dead character's `Despawn` about ten seconds after the death. A
sixth of the deaths have no corpse at all.

**What the client did**: a corpse was given to the nearest dead body within 4 m of it, and a dead body never left
on its own despawn (it was to stay as its corpse's body). So a creature that died further than 4 m from where the
corpse came kept its name, was not its remains, could not be looted through its body, and lay there for good; so
did every one that died without a corpse. In the user's log two of their own kills were 8.6 m and 7.2 m off.

**Now** (`net.rs`, `mao_net::n3::corpse_of`):

- A corpse names the one that died: its stats 415 and 416 are that character's type and number (all 371 captured
  corpses have them). That body is the corpse's, wherever this client had it, and is put where the corpse is. The
  4 m rule stays for a corpse that names nobody.
- A dead body whose corpse lies there stays as that corpse's body when its own despawn comes, as before; a dead
  body with no corpse leaves with its despawn.
- A corpse of one the client has not (the dead of before one's arrival) is named by its own name, not "Object".

`MAO_CAPTURE=<dump.txt>` with the test `a_captures_dead_become_their_remains` runs a capture's server messages
through the world: 223 of 223, 29 of 29 and 5 of 5 corpses that named one the world had are its body, and none that
died is left after its despawn.

**Why the bodies were so far off: creatures ran at a walk.** The server sends a creature along a path or after a
target with `FollowTarget`, whose byte after the kind (the schema's `Unknown1`) is 0x18 for a walk and 0x19 for a
run (17,000 captured: walked paths are gone along at 1.5 a second, median 1.45; run ones at up to 13). The client
went at 1.5 whatever the byte, until two paths in a row had shown a speed. A creature's own `RunSpeed` is in its
announcement: after the 28-byte block and the head's mesh (flag 0x80) comes a byte, or two with flag 0x2000
(`EntityInfo::run_speed`). The fastest tenth of each kind's runs agree with the characters' own rule,
`RunSpeed / 275 + 5` within 13 (FUN_1006f9eb): 5.0 for Arete's creatures of RunSpeed 13 to 50, 12.8 for those of
2,300. A creature sent running now goes at that; one whose RunSpeed is not known at what its paths showed, or 5.

A creature that fought from eight metres off, hitting the air, is likely what looked wrong in the attacks; the
report did not say more, and nothing else of the attack animations was changed.

Not done: a corpse whose body the client has not got (one walked away from and come back to, or made before the
arrival) is not drawn, only named; the original draws a corpse from its own data. The run speed's fall below 15%
health is not applied to creatures. Not seen live.

## Creatures' attacks: the swing without a weapon, the fight stance, the run (2026-11-17, the user: "the creatures'")

Asked which attack animation looked off, the user said the creatures'. What the client did: a creature's attack
(`AttackInfo`) swung "the weapon the server said the creature holds in the place it names", and with no such
weapon nothing. The captures' dockers, automatons and fleas attack from places 0 and 1 and are never told to hold
a weapon item (`WeaponItemFullUpdate`: none for them in 93 attacks), so they stood still while they hit. They also
stood in their plain `idle-stand` through a fight, and walked when the server sent them running.

The original (Gamecode):

- `FUN_1006b14c` makes an item of each of a creature's natural weapons (a list the character holds, at the
  creature's level as the quality) and gives it the stance's actions: class 0x1a the start, action 0x406; class
  0x10 the idle, 0x407; class 0x1b the stop, 0x408. For a creature it is put into the next free weapon place.
- `FUN_1003cce2` loops the drawn weapon's idle (class 0x10) after its start (0x1a) for as long as it fights: for a
  creature its `idle-unarmed` after its `unarmed-start`.
- `FUN_1006a782` swings: a body without the weapons' attachment points `unarmed-rswing` (0x40a), whatever is in
  the place; one with them the weapon's class 0xb action (`FUN_1006a014`). The animation is the creature's own for
  the action (`FUN_1003cac9`): the android's `attack-push`, the floating picker's `attack-grab`.

Now (`net.rs`, `creature_parts`): an attack swings whether or not a weapon item is known for the place; a creature
in a fight (`World::fighting`) stands in its 0x407 where its template has one, plays its 0x406 when the fight
begins and its 0x408 when it ends, the standing animation waiting for them as a player's does; a creature sent
running (`path_runs`) plays its run.

Natural weapons (CMB-016, 2026-11-19): a creature's full update with the flags' bit 0x100 lists its natural weapons
after its meshes (Gamecode FUN_100798aa: template pair and a four-letter code, `EntityInfo::natural_weapons`), of which
FUN_1006b14c makes items kept by code. An `AttackInfo`'s last word names the code that swung (FUN_1006ae3c). A body WITH
the weapons' attachment points whose attack names one of its natural weapons plays its own animation for action 0xb, the
attack (FUN_1006a014: FUN_10045976(0xb)). Still a guess, said plainly: such a body whose attack names no code and holds
no item swings `unarmed-rswing`; the original swings only what its place holds, and where that comes from was not found.

Seen offline: the docker standing, in its stance and after its push (`creature 44f1` with `MAO_HELD=407`, `40a`),
and the test `a_creature_with_no_weapon_swings_and_takes_its_stance`. Not seen live.

## Back from the dead with no health, no heals and no logging off (2026-11-17, the user's Maowild)

The report: after a death the character came back with zero health and could neither be healed nor logged off.
The log has the death, the word to the server eight seconds on, the corpse, the teleport and the playfield message
that set the character down at the reclaim terminal 150 m away, and then the character walking: nothing wrong in
what the server sent ("You were healed for 4 points" every half minute).

The cause is the client's: at a death the dead character became a body for a corpse to be given to, the client's
own character like any other, so its own corpse took it for its body. A corpse that goes takes its body out of the
world. The server takes a corpse away when the character is set down far from it, and the client's own character
went with it: not in the world any more, so no health to show, nothing for a heal to be used on, nothing to sit
down and log off. The next login mended it.

The arete-20261030-1655 capture (the original client) has the same sequence, a death of the capture's own
character, and run through the world (`MAO_CAPTURE`, the test `a_captures_dead_become_their_remains`) it failed
on exactly this: "the own character was taken out of the world by 36510078 of (c76a,103e803), the own corpse".

Now the client's own character is never a corpse's body: not put among the bodies at its death, not taken by the
corpse that names it, not taken out with a corpse. Its corpse is an object of its own, named by the corpse's name.
And when the playfield message sets the character down, what the death left on it goes: it is not among the dead,
and the health kept for its target line (nothing, since the death) is forgotten, so the server's is shown.

The test asserts both from here on: the own character is no corpse's body, and nothing but a teleport takes it out
of the world. Not seen live (it needs a death).

**That was not all of it** (the same evening: "that still doesn't work", with a picture of the character's name
standing ten metres from the character). The log of the second death shows it: after the character was set down
the client sent 32 moves and the server answered none, where it answers every one. The original says it is ready
(`570c2039`) after every playfield message: 52 of the captures' 52, among them the one after its own death, which
is in the playfield it died in. This client said so only when the playfield message named another playfield. So
after a death in Arete Landing, whose reclaim terminal is in Arete Landing, the server went on waiting: it took no
move (the name stood where the server had the character), told none of the stats (no health), and let the
character neither be healed nor log off. Now the ready goes out after a playfield message for the same playfield
too, and the walk or run mode is told again as on any arrival. Not seen live.

How it was missed the first time: the replay runs the server's messages through the world and cannot see what the
client fails to send. A fault after an arrival wants the log's count of moves sent and moves answered first.

## Oneself as the target in a fight, and what is locked shown as locked (2026-11-17, the user's two asks)

**"I'm not able to target self during combat to use stims."** The log of the fight has seven clicks in a row, each
`on the line [the docker], took the docker`: the character itself was never on the line. A click's line holds what
the line from the eye through the pointer meets, and this client's own rule left the character out of it wherever
anything else was met ("the camera looks over its shoulder"). In a fight the one fought stands behind the character
on every such line, so the character could not be clicked at all, and a stim went at the docker: "Item must be
applied on a friendly target." Now the character is last on the line, not off it (since 2026-11-18 on it by its distance, as the original has it). The stepping does the rest
(`GetNextTarget`: a click takes the one after the selected target): with the one fought selected, a click on the
character takes the character, and the next takes the one fought again. A click at something past the character
still takes that first. The select-self key and `/selectself` are as before; the fight goes on with the one fought
whatever is selected.

**"Anything that is locked should be greyed out and announce when available."** What the game locks, and where
the client knows it from:

| what | the lock | known from |
|---|---|---|
| a special attack | its skill, for the server's seconds | `CharacterAction` 0xaa (used) and 0xa4 (available) |
| a stim, a kit, any item | the skills its use locks | the item's spells of function 53033: value 0 the skill, 39 the seconds; the starter's Health and Nano Stim locks First Aid (123) for 40 |
| a nano program | every program while one is cast and for its recharge after | the client's own cast as the server starts it: the cast's time and the program's RechargeDelay (210) |
| a nano program, too | the skills its own run locks | its spells of function 53033 |

One list holds the skill locks (`actions::LOCKS`, all skills: it was read for the special attacks only), and three
questions ask it: `lock_left` (a special), `item_lock_left`, `nano_lock_left`. Shown: the Actions window's tiles
faded as before; **the inventory's items** and **the nano programs** now faded the same way (the list view's own
fading of an icon, `ItemIconView_c` +0x174: the share of the time still to go is grey); **a hotbar's slot** grey
with the seconds left, for a special, an item or a program, whatever the slot's own wait says.

Told when it is over: a skill by the game's own line, as before ("First Aid skill available.", "Fling Shot skill
available."), which covers the specials and the items that go by a skill; the nano programs by a line of this
client's, "Nano programs available.", after a wait of two seconds or more (a quick program's would be said after
every cast).

Seen offline: the inventory with First Aid locked beside the same with nothing locked
(`MAO_UI_PREVIEW=gear MAO_ENTRIES=<file> MAO_LOCKS=123:40`; `MAO_NANO_LOCK=<seconds>` for the programs). Not seen
live. Not done: an item with a wait of its own that locks no skill; a program is not shown locked for want of
nano points; and the two lines a locked special's key tells at times ("Action is not available." before the wait
line) were not looked into.

## Buying from a shopkeeper who talks: the Shop button (2026-11-18, the user's Antonio Stacklund)

The report: "could not shop with Antonio Stacklund who sells special items. Please make sure any shopkeeper can
shop." His window came up with his talk and the four buttons, and no way led to his shop.

In the original's capture of 2026-10-30 every shopkeeper of Arete Landing is the same: the chat is opened
(`KnubotOpenChatWindow`), and a second or two later the client sends a plain use of the NPC (`GenericCmd`, action
3, the NPC as its target), to which the server answers at once with the shop's list (`58362220` of a `(0xc75b,
..)`) and the trade's opening: Antonio Stacklund, Barry the Food Vendor, Marco Spida, eleven times in all, and
later presses with the chat still up open the shop again. That use is the window's **Shop** button: its handler
(GUI.dll 0x10058b34, a label the decompile does not list as a function) is `N3Msg_UseItem` with the NPC and
`false`.

The client had the button switched off ("not known here"). Now it sends the use, and the shop's window opens as
for any vending machine: the server's list is read whoever sends it.

With it: the shop an NPC keeps is announced as the NPC's (`VendingMachineFullUpdate` with the NPC's identity where
a machine's place would be, and its template, 0x3ca30 for Antonio's, which gives the prices). The client read a
place of nothing from it and set a thing to be targeted down at the playfield's corner; an announcement without a
place now sets nothing down and keeps the template.

Not seen live. A shopkeeper that does not talk at all (none in the captures) is still used with a double click,
which opens the chat first: the original's own choice between talking and using was not read.

## An item is judged before anything is taken off (2026-11-18, the user's mission belt)

The report: "wrong belt shown from mission and trying to equip it unequipped my existing. we should be checking
requirements before starting equipment swaps that could cause problems later."

What the log has. The mission "Yell at Patrick for Killing you" names its reward as the templates 36782 and 36777
at quality 30, and that is what the server handed over: a Belt Component Platform between the Ti-200X (quality 20,
Computer Literacy above 30 to wield) and the 300X (quality 40, above 120), so at 30 it asks for more than 75. The
client showed that item, by the name of its lower template as everywhere. Nothing was found wrong in what was
shown; if the game shows another belt for this mission, that is still to be said. The right click on it then went:
the worn belt off to the backpack (the server did it), the new belt to the belt's place (the server refused:
"Your Computer Literacy is required to be at least 76!", and the client took the move back), and the old belt lay
in the backpack.

What the original does (Gamecode `N3Msg_UseItem` 0x10028883): each of its branches first asks
`N3Msg_IsItemPossibleToWear` (0x10026c57), and a no ends the use before the place is emptied (FUN_10047732 /
FUN_1004b3ee come after). For an item in an inventory that function goes to FUN_100498ed, which asks the
inventory's own item object (`DummyItemBase_t`, vtable 0x10163a64) by the item's class (stat 76):

- class 1, the weapons' page (weapons, the belt and its units, utilities, HUDs): vtable +0x64, FUN_10081ed7, the
  **wield** requirements (action 8). No such action, or none in it: it passes. Else the criteria evaluator judges
  them with its feedback flag set, and its own line is the only one said.
- the other classes: vtable +0x5c, FUN_10081d5f, the wear requirements (action 6), the evaluator's lines and then
  `Feedback_UnableToWear`.

The note of 2026-11-16 had read the branch for an item lying in the world (`WeaponItem_t`, action 6 only) and
concluded that wield requirements are the server's to refuse. That was wrong, and it is why the belt's
requirement was not asked here. 12,282 of the 19,549 templates of class 1 have a wield action (the new
`itemactions <game> worn` counts them).

Ported:

- `World::wear_refusal` judges the wield action for class 1 and the wear action for the rest.
- The evaluator's lines (`requirements::evaluate_told`, `Told`; `feedback::requirement`): a comparison that fails
  leaves a line, told when the whole list failed and the character is one's own. Below and above
  (`Feedback_IsNotLessThan`, `Feedback_IsRequiredToBeAtLeast`: whose, the stat's name, the number) and the
  equalities of breed, side, sex and profession. The other operators' lines are not ported.
- **The value judged is the one the windows show.** The evaluator reads a stat in mode 0, which for a character
  (FUN_10059399) is the shown value: what is stored, what the worn items, perks and nanos add, and for a skill
  what the attributes add. The client judged the stored value alone, in the wear check, the perks' and the
  spells' criteria alike (`derived::judged_one`; a few stats are read stored whatever the mode, as there).
- The check is asked at the right click, before any move is queued (`net.rs`, the use of an item that is worn
  instead). The log line is `items: the use of place N (item X) is refused before any move`.

The client's own, at the user's wish: when the place is taken, the character is judged **as it will be without
the item that comes off** (`derived::modifiers_without`, `Stripped`). The original judges it as it stands, takes
the old item off, and the server may then refuse the new one for want of what the old one gave: the same state
as in the report. `DEVIATIONS.md` has it as `INV-084`.

A risk to know: the shown values are this client's own sum (worn items, perks, nanos, organization, trickle). A
source it does not know makes a value too low, and the check then refuses what the server would take. The skill
window shows the same numbers, so a refusal can be checked against it.

Tests: `a_failed_list_tells_what_failed`, `a_belt_is_judged_by_its_wield_requirement_before_a_swap` (`MAO_GAME`).
Not seen live. Not ported from the two checks: the refusal while fighting (asked only with a page number the use
does not pass) and the wear bit of the "can" flags.

## The "missing" parts brought in overnight (2026-11-18)

The user: "bring in all the \"missing\" parts of the deviation report that should [be] easy and not
affect too much while I sleep." Each part below was read anew in the original before it was ported (the catalog's
"Original" lines are the notes' words); the catalog entry is marked in place. None of it was seen in the live game.

### Lists keep their column widths and their sort (SET-036, WIN-130, WIN-119, NAN-039, NAN-040 in part)

The original keeps each list's columns and sort in its window's config as `listview_config`
(`MultiListView_c::StoreColumnInfo` 0x10133655: `col_id`, `col_flags`, `col_width` in the columns' order;
`ItemListViewBase_c`'s FUN_100411b8 adds `list_sort_column` / `list_sort_order` and the grid's own pair), written
when the window goes and read when it is made (`LoadColumnInfo` 0x10136a6d: from the store come the width, the
hidden bit and the order; a column the store does not name keeps what it was built with).

Here: `layout::ListConfig` (the sort and the widths) in a new file `lists.ini` beside `layout.ini`, a line a list
under the original's config name, written with the windows' places soon after a change (one set for all
characters, like `tabs.ini`). `multilist::Keeper` holds it for a window: the NCU window (`NCUWindowConfig`), the two
target NCU windows, the nano programs (`NanoViewConfig`, one for this client's one list where the original has one
a school) and the mission journal (`MissionViewConfig`). The kept sort is the list mode's; a grid stays in its own
order (the NCU's by the time left, the programs' by the chosen "Order"), as the original keeps the two apart.

Left out: hidden columns (the header's right-button menu), swapping columns by dragging a label, the mail's and
the shops' lists, the journal's active mission, and reading the original's own kept widths as a first value.
One of the notes' open questions is answered by the way: the original does not count kept widths twice
(`ORIGINAL_CODE_QUESTIONS.md` 4): the loop over the copy sees only the columns the store did not name.
Tests: `the_lists_file_round_trips`, `a_list_keeps_its_widths_and_its_own_sort`.

### Locks: a locked action's click, the rewards' claims, and two server messages (WIN-115, WIN-140, NET-011)

Read anew in Gamecode's `CharacterAction` handler FUN_1005d61f (the message's two parameters are the words 17 and
21 of its data, the target the words 9 and 13) and in `N3Msg_PerformSpecialAction`'s FUN_100427d6.

- **A fault found by the reading and mended: perk locks were read from the wrong words.** `PerkUnavailable` (0xcf)
  carries the lock's number and its seconds in the two parameters (the captures: `...000000c9 00000019`), and
  `PerkAvailable` (0xce) the number in the second. The client read one word early, so every lock went under the
  number 0 with the lock's number as its seconds: no perk action was ever shown locked.
- **A click on a locked action says its wait line** (WIN-115). The original's click runs
  `N3Msg_PerformSpecialAction` whatever the action's state; a locked one says the wait line of what locks it (text
  category 1000: `UnableToPerformActionSkill` with the skill's name for a special attack,
  `UnableToPerformActionPerk` with the perk's name for a perk, each with hours, minutes and seconds) and nothing is
  sent. The client's action window dropped such a click without a word; now it goes on, and the network thread
  says the line (`special_refusal` for the attacks, `World::perk_lock_left` for a perk). The server's own wait line
  (0x84) uses the perk's text for a perk too, and `PerkAvailable` says the game's "ready again" line with the
  perk's name (`Feedback_PerkAvailable`). The name is the perk action's own template's; the original takes it from
  a perk table by the lock's number (FUN_10053bb2), which was not read.
  "Action is not available." is another case: an action not in the list, or one whose lock has run out here while
  the server has not yet freed it.
- **The daily rewards' claims wait for their lock** (WIN-140; `N3Msg_RequestClaims` 0x1001db10,
  `N3Msg_RequestClaimsDBCheck` 0x1001dbd8): the server locks stat 0x8c for 60 s at each claim and 120 s at each
  check; while it runs the client says the wait line that names nothing (`UnableToPerformAction`) and sends
  nothing (`World::claims_refusal`). The claims now name the character itself as their target, as the five in
  the original's captures do (`n3::claims` left it zero).
- **A lock's length** (found by the way): `SpecialUsed` (0xaa) does nothing for a skill that is locked already, and
  the seconds are changed by the character's lock modifier, stat 382, for every skill but the special attacks' own
  (FUN_10063de2; `actions::lock_seconds`). A capture's 60 s lock was freed after 48.3 s on a character with -20.
- **0x14** makes a skill's lock longer (or starts it); **0xfc** takes every perk lock away and clears the time of
  the last perk reset (stat 577); **0xb2** marks an action usable, a mark this client does not keep apart from the
  locks (NET-011). None of the three is in a capture.

Read and left: `CharacterAction` 0x6e (INV-072) is not "the end of an action's progress": it closes the open reclaim
container (`0xdeae`; 0x6a the bank's), which this client does not have. The skills window's refusal of a
"temporarily changed" skill (WIN-142) was read (`N3Msg_GetSkillMax` gives 0 for a stat a 0xcf76 spell holds, with
`Feedback_CantChangeTemporarilyChangedSkill`) and not ported: rare, and the window is not handed the needed set.
Tests: `a_special_is_refused_as_the_original_refuses_it` (the locks' messages), `a_lock_runs_by_the_lock_modifier`,
`the_claims_are_as_the_original_sends_them`.

### Missions, the level-up and the factions window (NPC-030, NPC-035 / WIN-133, OTH-012, WIN-125)

- **No missions** (GUI.dll FUN_100d0e71, the mission window's slot for every change of the offers): the list is
  emptied first, then for no offers a line is said in the chat (the system text's signal). The client kept the old
  offers in the window and put its words in the window's status line; now the offers go and the line is a chat
  line. The original's sentence is a literal of GUI.dll's, in no text file the client reads, so the wording stays
  this client's own: say so if the original's words are wanted.
- **The mission page's experience for a reader of level 200 to 220** (FUN_100360d5 at 0x1003645d): where shadow
  knowledge can be had (`CanReceiveSK`, Gamecode 0x1001827d: bit 2 of the first flag word of the playfield's
  record, set for the Shadowlands' playfields) the row is the game's `IT_QuestSKReward` with `GetSKForXP` of the
  experience (0x10018255: by the standing with the side's faction, a thousandth of the experience at 50000);
  elsewhere there is no row, which is what the client showed everywhere. `missionlog::Reader`, `sk_for_xp`,
  `mao_formats::playfield::flag_words` (checked against real records). Not confirmable without such a character.
- **The level-up's lines** (Gamecode FUN_10075f55): the experience that made the level is told first (the
  message's last word; no other message tells of it, the capture's `Stat` after it names the same total), and a
  new title is told with `Feedback_CongratulationsReachedLevel` and the title's name (text category 1060). The
  title part is read from the code alone: the schema calls its word `IpResetPointsGained`, the code uses it as the
  new title level, and no capture has a level-up across a title. It is only said for a level 1 to 7 whose text is
  found. The notes' "reset points line" does not exist. Not done: the hints (`got_ip`, `got_perk`, `got_tech`),
  which are the original's notice buttons for the skill, perk and research windows.
- **The factions window** starts with the titles, as the original's does (the preference `IsFactionTitleShown`
  has the default 1; the client's note said "unset"), and keeps the choice (`tabs.ini`).

Read and left for now: the mission request window's kept sliders and its "advanced options" button (WIN-135: the
config's eight values and the button that hides six sliders are read; it changes the window's layout, which wants
a picture). Tests: `a_level_up_tells_the_experience_first`, `the_page_lists_the_time_and_the_pay`,
`real_playfields_flag_words`.

### Flight: what a flyer cannot do, getting up to fly, the flight item's two questions (MOV-022, MOV-023, MOV-025, SND-015, MOV-021 in part)

- **A flyer cannot sit, sneak, crawl or change its pace** (MOV-022). When the client's own character goes into fly
  mode the original marks eight of its actions unusable (`SwitchToFlyModeTransitionAction_t`, Gamecode FUN_1006e4f8:
  stand, sit, walk, run, sneak, leave sneak, crawl, leave crawl) and takes the marks away on landing. A key or a
  tile for one of them says `Feedback_ActionIsNotAvailable` and does nothing (`N3Msg_PerformSpecialAction`'s
  FUN_100427d6). Here: the sit key, the walk/run key and the Sit/Stand, Walk/Run, Sneak and Crawl tiles
  (`actions::fly_refuses`). `/camp` in the air says `Feedback_YouMustBeSitting` and starts no logout
  (`N3Msg_StartCamping` asks whether the character may sit). The catalog's "FUN_1006c6f4 case 7" is the second,
  silent layer under this one (the mode's own rule), which lets the walk/run switch through.
- **Getting up to fly** (MOV-023). The fly spell (0xcf92, FUN_100a8107) first has one who sits, sleeps or lounges
  get up, beginning or ending, and tells the server nothing. Here for every character a spell list switches
  (`World::spell_outcomes`) and for one's own at each switch of its fly mode. It goes in with the point above:
  a seated flyer could otherwise no longer stand.
- **A flight item's two questions** (MOV-025), in the criteria evaluator FUN_100065d8: operator 70 (0x10007bdd)
  asks whether the dynel its value's picker finds is a character in fly mode (the Ganimedes jetpack: "the user is
  not flying"); operator 112 (0x1000754c) whether the character's playfield allows flying, which it does unless
  the third flag word of its record has bit 0x80000 (`mao_formats::playfield::flag_words`), and says text category
  100's `FlyingMustMeAllowedOnThePlayfield` when it is asked to tell. Both gave "unknown" and the jetpack's whole
  list went unjudged, to the server. **Arete Landing, the ICC shuttleport, Sunrise Station and Jobe Research
  have the bit**: a jetpack is refused there by the client now, with that line, as the original refuses it.
- **Sneaking steps are quiet** (SND-015, which was misnamed): the step sounds' quarter volume is for movement mode
  6, which is sneaking, not flight (FUN_100452d3; a landing at half). A flyer makes no step at all (the hover
  animations have no notes), which was so already.
- **CharacterAction 0xac** (MOV-021, the small part): a character in fly mode leaves it (FUN_10045020). It ends a
  flight a spell list of the server's began. No capture has one. Not done: one's own flight from a worn item
  ending by it (the client works that out anew each pass), the other ways out, and the entering effect 2000.

Read and left: the stun's "select a target" line (CMB-035) cannot be reached by any path found (a target that is
neither a character nor a world item is an inventory entry, and those spells carry no caster): nothing to port.
Noticed and not changed: the original stands up from sitting with `CharacterAction` 0x57, not the movement 0x25
this client's X key sends.
Tests: `a_flyer_cannot_sit_sneak_crawl_or_change_its_pace`, `a_flight_item_asks_whether_one_flies_and_may_fly`,
`real_playfields_flag_words`.

### The belt's deck: a unit needs a place, and the belt stays while units are in it (INV-074 / INV-075 in part)

- **A unit needs a deck place** (Gamecode FUN_10047ece, the last thing the inventory item's wear check FUN_100498ed
  asks, and FUN_10046f26): an item of the weapons' page going to one of the places 9 to 14 passes when the place
  less 9 is below the character's deck places, stat 45 as the windows show it; else
  `Feedback_ThisItemRequiresComputerDeck`. The server stores no such stat: the worn belt's effect gives it (2 for
  the starter's belt), so the client's own sum is the number. `World::wear_refusal`.
- **The belt and its units** (found by the reader in the move's own check, FUN_1004bd28 at 0x1004c354 and
  0x1004cc08, and read again here): the belt does not come off place 7, and nothing goes onto place 7, while one
  of the places 9 to 14 holds something: `Feedback_DeckMustBeEmptyBeforeUnequipped`. So a belt is not swapped with
  units in it, by a right click or by a drag (`World::unwear_refusal`, `deck_holds_something`).
- **A rounding fault mended** (found while the item pages' interpolation was read, INV-065): the original takes a
  value between two templates to the nearest whole number, a half upward, whichever way it runs (FUN_100cb909).
  The client cut a falling value (a penalty that grows with the quality) toward the low template's: -21 where the
  original has -22. `derived::between`, for the worn items' effects and the requirements. The shown stats of
  three checked characters still match (`docs/STAT_ENGINE.md`'s tests).

Still left to the server from that check: an implant's surgery bit, social armour against other armour, an item
that is not one's own. Read and left: the item page's own stats between two templates (INV-065: which 27 stats
are interpolated, the price by a curve, and the high template taken whole at its own level are read; the page
still shows the low template's); a bank item let go on a bag (INV-060: text 10003
`ItemsDraggedFromBankCanOnlyBeDroppedToBank` as a chat line; needs a drag to try); a bag window's close told to
the server (INV-062: the server keeps each bag open or closed and every use flips it, so this client's local
toggle would have to follow the server's open and close actions: a live matter); the city signal's id (ZON-018:
the compare is in city.dll and no capture has such a signal). Nothing to port: a points shop's cost (INV-070) has
the same "Credits:" label in the original.
Test: `a_belt_is_judged_by_its_wield_requirement_before_a_swap` (the deck's part), `a_value_between_two_templates_is_the_nearest_whole`.

### `/start` asks before it leaves the full screen; the mission request window keeps its sliders (CHT-056, WIN-135 in part)

- `/start <address>` (GUI.dll FUN_100ba073, FUN_100b9acf): while the game owns the screen (the display system's
  full-screen byte: the exclusive mode, not a borderless window) a box titled "Warning:" asks with text 10000's
  `ReallyMinimizeGame`, Yes and No, and only Yes opens the address. `chatwindow::EXCLUSIVE` is set where the
  exclusive mode is taken (`viewer::apply_mode`). Not done: the usage line for `/start` with no address.
- The mission request window's seven sliders are kept between sessions under the original's names
  (`MissionSelectionViewConfig`: `difficulty`, `dim_good`, `dim_ctrl`, `dim_secret`, `dim_mystery`, `dim_stealth`,
  `dim_reward`; GUI.dll FUN_100d10fc reads them, FUN_100d04fe writes them), here as numbers in `tabs.ini`, written
  when the mouse lets a slider go. Not done: the "advanced options" button that hides six of them (a change of the
  window's layout).

### An item between two templates is shown as it is at its quality (INV-065)

Gamecode makes the item object for a low template, a high template and a quality in FUN_100cc5a1 (FUN_100cbfc8
works out what lies between): at the low template's level it is the low template; **at the high template's level
it is the high template itself**, name, icon and all; between the two it is the low template with 26 of its stats
taken between both by the rounding of `derived::between` (the list is FUN_100cbbe0's: the value, the deck places,
the ranges, the delays and the like) and its price by a curve (FUN_100cbb68). `derived::item_at` does that for
the item pages (`gui::iteminfo::item_page`) and for the names and icons of the inventory's items
(`gear::from_entries`): a Belt Component Platform of quality 30 between the Ti-200X and the 300X has 3 deck
places and a price of 4375 on its page, and the same item at quality 40 is named the 300X.

Left: the page's modifier and skill lists still read the low template's (the worn effects and the requirements
are worked out between both already, `derived::effects` and `derived::actions`).
Test: `an_item_between_two_templates` (`MAO_GAME`).

### The action tiles that did nothing, and a special's weapon (WIN-112 / CMB-025 in part, CMB-022 in part)

- **Tiles** (`N3Msg_PerformSpecialAction`'s FUN_100427d6, by the action's number): Use is `N3Msg_UseItem` on the
  selection; Start and End Combat both `N3Msg_DefaultAttack`; Sit and Stand both `N3Msg_SitToggle`; Walk and Run
  the pace's switch (`N3Msg_MovementChanged` 0x18 / 0x19); Suspended Animation `N3Msg_StartCamping` and Disrupt
  `N3Msg_StopCamping` (`CharacterAction` 0x79 alone; the server's 0x7a ends the bar). Each of these has a key in
  this client that does the same, so the tile presses the key (E, the attack key, X, the walk/run key, `/camp`),
  and with a camp under way the Suspended Animation tile is its other side. Before, Use, Walk and Suspended
  Animation said "its network message is not captured yet", Stand only stood a sleeper up, and Start Combat could
  start a fight but not stop one.
  Pick-Up and Crawl followed on 2026-11-19 (below); the tiles' pictures follow the state since WIN-113.
- **Pick-Up** (2026-11-19): `N3Msg_GetItem` (Gamecode 0x10027d76) on the selection. With no free place in the
  inventory from 0x40 on it says `Feedback_InventoryFull` and sends nothing; else it sends `ClientGetItemIIR_t`
  (`n3::client_get_item`, number 0x37136c6b, the class name's `MapToKey`), whose writer (FUN_100152a5, the class's
  vtable +0x20) puts only the target's identity. The selection goes as it is, also an empty one. No capture has the
  message: the zero byte before the identity is taken from the other messages of its kind, as for the drop. Not
  ported: the item's own answer first (`SimpleItem_t`'s vtable +0xc4, FUN_10088a73: `Feedback_CantCarryThat`
  when its "can" flags lack carrying, `Feedback_AlreadyGotUniqueItem`, `Feedback_CantTakeFixtureFromBuilding` in
  another's building, and a byte at +0x130 that stops it silently), and the pick-up animation (FUN_100823be).
- **Crawl** (2026-11-19): `N3Msg_CrawlToggle` (0x10027a45): nothing while swimming (the movement's mode 4); while
  crawling (`WaitState`, stat 430, is 0xe) the movement 0x28, `LeaveCrawlMode`; a polymorphed character (stat 359
  not 0) gets `Feedback_YouCantBePolymorphed`; else the movement 0x1b, `SwitchToCrawlMode` (FUN_1006eb8f sets the
  `WaitState` 0xe, FUN_1006eb2c back to 0). The client keeps the own crawl from the movements it sends and drops it
  when the character gets up (`World::own_crawling`); the Crawl tile shows its other half meanwhile. The crawling
  body's animation and pace are not drawn here. Test: `the_crawl_tile_toggles_the_crawl`, `a_pick_up_names_its_target`.
- **A special's weapon** (`N3Msg_SecondarySpecialAttack` 0x100281fc, after the lock's wait line): with a weapon in a
  hand, one of the held weapons has to have the special's bit among its "can" flags (FUN_1009aced: Brawl
  0x2000000, Dimach 0x4000000 beside the weapons' own), else `Feedback_SpecialAttackIsUnavailable` and nothing is
  sent; with bare hands nothing is asked. So Brawl and Dimach, which the server lists whatever is held, are not
  available with only pistols or rifles in the hands (a Solar-Powered Pistol's flags have neither bit, a Dull
  Katana's both). `actions::weapon_lets`, `World::special_refusal`. The special's range and the line of sight are
  checked too since then (CMB-022, below).

Noticed and not changed: the original stands a seated character up with `CharacterAction` 0x57 (ten pairs in the
captures: sit is the movement 0x1e, stand is 0x57), where this client's X key sends the movement 0x25; the server
has taken that so far.
Tests: `a_weapon_lets_its_own_specials_through`, `a_special_needs_a_weapon_that_lets_it` (`MAO_GAME`).

### Doors have their sounds; what the original's "closing time" really is (ZON-017 in part)

- **Sounds.** A door's sound is not a stat ("stat 131, else 100" in the notes): it is the door template's sound
  set (the item's +0x88 map, chunk 0x14, which `mao_formats::item::class_sets` reads). Opening (Gamecode
  FUN_10080319) plays the set's class 0x83, else 0x64; closing (FUN_100803b4) class 0x84, else 0x66; each with
  `PlayGameSound(id, the door's place, 0.0, 1.0, 0, 0.0, 100, 1, 0, (0, 0, 0))`, before the animation. Of the 95
  door templates 28 have the first pair, 36 the second and 25 are silent. Here: `door_sound_pair`,
  `table_door_sounds` for a playfield's own doors and the same for the doors the server announces; a sound is due
  at each change `tick_doors` notes. Not heard (nobody was there to listen). Not done: the touch's own sound
  (class 0x6f, six templates).
- **The closing time, read and left as it is.** The original has no closing timer: `Door_t`'s run (FUN_1007ff44)
  closes an open door in the first frame in which no character's body touches it (unless the server's "stay
  open" byte holds it), and stat 335 is not a time but a count of whole seconds a touching character waits before
  the door lets it through or sends it on (1 on every door). The server's `DoorStatusUpdate`s are snapshots as
  things come into range, not events. This client holds a door open for five seconds from the last time one's own
  character was within four metres, and a door the server showed open for a day: both are its own, kept tonight
  since the doors work as they are and a door that flaps at the edge of its reach would be worse than one that
  closes late. The notes that call stat 335 a closing time are wrong. (Both went on 2026-11-18, and the stat 0 bits
  below are read now: "A door opens at the character's touch".)
- **Read and left**: the locked door's question (FUN_1007fad8: a key item, the door's requirements, a key list, an
  access card, an owned building and its team; its only line is for a full inventory) is not small and wants a
  locked door live. Found beside it and not changed: a door's "open" and "locked" are bits 0x80 and 0x40 of its
  stat 0, not of stat 259 as the client reads them (stat 259's bit 0x40 marks a door that keeps its template's
  flags: one template, an always-open door, which the client takes for locked); and a status message's lock byte
  is ignored.
Test: `a_door_sounds_as_it_opens_and_closes`.

## Five "missing" entries the user picked (2026-11-18)

The user, after the list of the catalog's 25 high "missing" entries: "port NPC-028, INV-048, INV-049, CHT-048 and
SET-031". Each was read anew in the original by a reader, and the claims a port rests on were checked against the
code and the captures before it was written. None of it was seen in the live game.

### Stacks join and split inside a backpack (INV-048)

GUI.dll's FUN_100cc855 is the one drop handler of every item view (the inventory, a backpack, the bank, the wear
pages), so a backpack does what the inventory does:

- **Join.** A stack let go on another item of its own backpack asks `N3Msg_JoinItems(the other, the held)`
  (0x1001bb2a: both stackable, the same low and high template and quality, together no more than 50000); when that
  says no the two change cells and the other is in hand, with `Feedback_CanOnlyStackItemsOfSameLevel` for one kind
  at two levels. The client's own list changes only when the server answers with the same action (FUN_1004a12e).
- **Split.** Ctrl+click on a stack asks how many (this client's box; the original counts by the mouse's sideways
  movement while Ctrl is held, FUN_10041ca4); that many let go on an empty cell of the same backpack are split off
  (`N3Msg_SplitItem` 0x1001bd13: `Feedback_YouCantSplitThisItem`, `Feedback_CantSplitAsInventoryIsFull`), the
  client's list first (FUN_1004a2bf: the new stack at the bag's first free place, counted from 0), then the
  message. Let go on an item, over another window or over the world, the split ends with nothing done.
- **On the wire** an item of a bag is `(0x6b, handle << 16 | place)` (`Chest_t`'s FUN_1007e3e3; the handle is the
  `InventoryUpdate`'s), in both messages; the bag's own identity is not in them. Across two containers nothing is
  joined: a stack dropped from one into another is a plain move, in the original too.

The Arete Landing capture has three joins and a split, all in the inventory (the server answers a join with the
same bytes and a split with nothing): the builders' bytes are theirs. **None is inside a backpack**, so two things
rest on Gamecode alone: that the server answers an in-bag join with the bag identities, and that it puts a split's
new stack at the bag's first free place. If the second is wrong the bag's window and the server disagree until the
bag is opened again. Not done: the bank's stacks (`(0x69, place)`: its window has no placing by hand here).
Tests: `stacks_split_and_join_inside_a_bag`, `a_click_in_a_container_takes_or_takes_up`,
`stacks_join_and_split_as_gamecode_builds_them`.

### Other players are drawn at their own size (NPC-028)

The scale is in every character's full update, a short right after its `MonsterData` word, and the original sets it
as the `Scale` stat (360) of players and NPCs alike (the exec FUN_1007835c at 0x1007857b; `SimpleChar_t`'s SetStat
holds it at 20 or more) and draws the body at a hundredth of it, read in mode 3 (FUN_10044eb0, FUN_1005c3ed,
FUN_10058800). The catalog's `FUN_100892ca` is the items' SetStat, not a character's. The client read the scale for
creatures only, by the shape of the bytes around it; a player's part of the message is now read as the reader
FUN_1007916d reads it (`n3::player_monster_and_scale`): every one of the captures' 1404 player updates reads so.
**Most players are not 100**: 580 are 90, 540 are 100 and 278 are 110 (character creation's three heights, by the
look of it), so most other players become a tenth shorter or taller than they were drawn, their steps a little
quicker or slower with it (`movement_rate` already took the scale), and their name tags move with them (this
client's tags follow the posed body).

A player with a monster body (`MonsterData` not 0: a polymorph, or a Yalmaha, template 99470 at scale 20) keeps 100:
the scale is that body's, and this client does not draw it for others from the update yet (MOV-020, RND-135: the
field is read now, `EntityInfo::monster`, and what is missing is knowing when the body is put away again). Not
followed: a change of the stat after the update. The creatures that run the "increase size by 40" nano (the Arete
bosses: their remains are announced at 640 where they were 600) are probably drawn 40 too small for that reason;
not ported, a picture of the original beside ours would settle it.
Tests: `a_player_is_announced_with_its_scale`, `another_player_has_the_scale_it_was_announced_with`.

### The item timer starts with the server's answer (INV-049: the catalog's reading was wrong)

The catalog said the original holds an item's use back until the first phase of the item timer is over. It does
not. `N3Msg_UseItem` (0x10028883) queues the use command and the engine sends it in that frame, once the command's
own check let it through (`n3Command_t::Activate` in state 0 → FUN_1007ce00). **It is the server that waits**: in
the captures a one-second stim's answer comes 0.38 s after the use and its effect 1.1 s after; a recall beacon's
(five seconds) effect comes 5.04 s after, a three-second chip's 3.07 s. The timer is the client's picture of that
wait: the server's answer in state 1 starts it (FUN_1007ccb3 → FUN_1003bcdc → FUN_1003bb5a: ItemDelay, stat 294,
as the first phase), and when the first phase ends the entry is taken away and the client does its own, local,
part (FUN_1003badb: one off the stack); nothing is sent then, and the second phase never runs. Sending the use
late, as the entry asked, would have made every item wait twice.

What did differ, and is ported: the bar started at the click, a round trip early, and for a use the server refused
too. It now starts with the answer (`World::add`'s arm for `52526858`: lead byte 0, state 1, action 3, 5 or 0x20,
an inventory item). Not ported, noted at `start_item_timer`: the original refuses a new use silently while an item
timer runs; and its `CharacterAction` 0x92 ends the equip timer (kind 6), not this one, where the client's takes
the item timer away at every start of a move. The two are left together: mended alone, the second would leave the
timer of an item without ItemDelay used on something running for ever.
Test: `the_item_timer_starts_with_the_servers_answer`.

### `/raid` is the raid's commands (CHT-048)

`/raid` is a line for Gamecode's text interpreter (GUI.dll hands it over with `N3Msg_TextCommand`, as `/team` and
`/org`); it never opened a window. The interpreter (FUN_1003fe10, the branch at 0x10040725) knows six words:
`create` 1, `list` 2, `listlocal` 3, `move` 4, `lootaccess` 5, `locks` 6. No word, or another: nothing, and
nothing said. `listlocal` says the team's members and sends nothing. The others send `RaidCmdIIR_c` (`4e525e58`;
FUN_100a3a3f packs the number as a word and an identity, three identities for number 5): `create`, `list` and
`locks` with no identity and no check at all; `move <character's number> <team 0-6>`; `lootaccess <corpse's
number> <place> <character's number>`. `raidcommands.rs` has the parsing, with the original's oddity that a missing
word reads as the word before it. The server's `RaidIIR_c` (`3b3b2878`) is read: type 0 says the game's
`Feedback_RaidCreated`, type 1 is the lock list, said in the chat (playfield, number, time left; the words are the
client's own, the original's are in its program).

**None of it was sent to a server**: no capture has a raid message, so the bytes are the code's alone, like the
organization's commands were before they were tried. `create` turns the team into a raid for all its members,
`move` and `lootaccess` act on other players by raw numbers: they are the player's to type. What `list` brings
back is not in the client's code (the server probably answers in chat). The raid window is no longer opened by
`/raid`: it is `/open raid` and the control center's menu entry (the original also has a key for it, Ctrl+Shift+R,
which this client has no action for). The window's two buttons still send nothing (CHT-049), though the message
they need is here now (`N3Msg_CreateRaid` sends number 1 for a team's leader, `N3Msg_RequestRaidLocks` number 6).
Tests: `the_raid_words_are_the_interpreters`, `move_and_lootaccess_read_their_numbers_as_c_does`,
`the_lock_list_is_told_lock_by_lock`, `the_raid_commands_are_packed_as_gamecode_packs_them`,
`the_raids_answers_are_said`.

### A character's keys from the original (SET-031)

The original keeps the rebindable keys per character, as the `KeyBindings` archive of
`<Prefs>/<user name>/Char<id>/Prefs.xml`: binds of an `Input` (the game's key number with Shift 0x20000, Ctrl
0x40000, Alt 0x80000) and a `Provider` (the action, by the hash of its name, FUN_10019281). The shipped default is
the same archive in `gui/Default/CharPrefs.xml`. The loader (FUN_10018e37) takes the character's archive for all
its keys and, when the shipped version is newer, adds the shipped binds of actions the file never names.
`keybindings.rs` reads both and, at the game's start, lays the character's keys over the client's own **for the
actions whose keys differ from the shipped default's** (`Keymap::set_character`): for the session, shown in the
Keys list, asked before the client's other keys, and never written to `mao.ini`. A key changed in this client
stays (it is the later word), and the original's folder is only read.

Two things the reading turned up:

- **No character on this machine has rebound a key in the original**: all 239 character files hold the shipped
  binds (126 of them an older version without Shift+T, which the loader adds). So this changes nothing visible
  until a key is rebound there.
- **This client's defaults are not the shipped file's in five keys**: walk/run is Backspace there (NumpadDivide
  here), sneak attack J, fast attack N, aimed shot O and the bow's special Period have keys there and none here,
  and O is this client's map overlay. (Changed the same day, see the next part.) Reading an untouched file in whole would have moved them
  silently, which is why only changed keys are taken over.

The character's folder is looked for under every install's `Prefs` and every user folder in it (the old lookup
knew `derived` only), the file written last where two have it, and no other character's stands in. Not read: the
fixed keys (camera, Tab, chat: `KEY_...` lines of the account's `Login.cfg`), mouse buttons as keys, and the
original's actions this client has none for (the team and pet selection keys, `WINDOW_TEAM`, `WINDOW_RAID` and
others).
Tests: `keybindings::tests` (six), `a_characters_own_keys_lie_over_the_clients`,
`a_characters_own_file_is_found_by_its_number`.

### The five default keys are the shipped file's ("match the original", the same day)

Asked whether the five differing defaults should follow the shipped `CharPrefs.xml`, the user said "Match the
original". So, since version 3 of the key table (`settings::KEYMAP_VERSION`): walk/run is Backspace (was Numpad /),
sneak attack J, fast attack N, aimed shot O and the bow's special attack Period (they had no key), and the map
overlay, this client's own, is on Ctrl+P (it had O; left without a key at first, then the user: "you can map it to
ctrl+p for now").

A settings file written before is not thrown away for it (another version's keys used to be replaced by the
defaults in whole): a version 2 file is read as it is and brought up to date (`settings::MOVED_DEFAULTS`): each of
the six actions that still has its old default gets the new one, unless another action of the file has that key; a
key the player chose stays. So someone who never touched these keys gets the original's, the overlay moved from O
to Ctrl+P included, and someone who put the overlay on O and F6, or J on the jump, keeps that and the action that wanted
the key stays without one. The file is written as version 3 at the next save.
Test: `the_older_key_table_is_brought_up_to_date`.

## Three more: the raid window, other players' crafts and flight (2026-11-18)

The user, after the list of what was left of the high "missing" entries: "port CHT-049, MOV-020 and RND-135".
Read anew in the original by two readers; the claims the ports rest on were checked against the code and the
captures. None of it was seen in the live game.

### The raid window: its teams and its two buttons (CHT-049)

`RaidWindow_c` (GUI.dll FUN_10103127, refill FUN_10102bbe), as read:

- **The six teams are filled only in a raid** (`N3Msg_IsInRaidTeam`): each from `N3Msg_GetTeamMemberList(place)`,
  a member's name, profession picture and level, the leader of one's own team in 0x11eeaa. In a plain team, and
  without one, all 36 lines are empty. This client showed the own team as Team 1 in a plain team: no longer.
- **Convert To Raid** is enabled in a team that is no raid yet (leader or not); the click is `N3Msg_CreateRaid`,
  which sends the raid command 1 only for the team's leader and says nothing otherwise. **Raid Locks** always sends
  command 6; the answer is said in the chat. Both use today's `n3::raid_command`.
- **Where the teams come from** (`team.rs`): Gamecode's team object has six lists and the own team's place (+0x38,
  -1 outside a raid). The world-entry message carries them (its team flags' bit 1: the own place and six lists),
  `TeamMember`'s third field is the member's team's place, `CharacterAction` 0x20's second parameter the place of
  the one who leaves, and `RaidIIR_c` of type 0 makes the own team place 0. `Team::raid`, `Team::others`,
  `Team::raid_team`.

**Two faults found on the way, both mended:**

- **Zoning in a team emptied the perk list.** The schema read the world-entry message's team flags as a count of
  identities; with a team (flags 1) the lists behind it slipped by one and the perks were left unread. The tail is
  now as `FullCharacterIIR_t`'s reader FUN_10073dca reads it: all 52 captured world entries read to their last byte
  (17 of them sent in a team of two), which they did not before.
- **A team member's profession was never right**: `TeamMember`'s profession is a short and the name's length a
  word (FUN_1007a4ea); read as a word and a short, the profession came out shifted (0x40000 for a Fixer), so the
  raid window's profession pictures could not show.

No capture has a raid. Everything about one (the flags' bit 1, places other than -1, the `RaidIIR_c` answers,
and above all what the server sends when) is the code's alone; the non-raid halves of the same readers match the
captures byte for byte. Not ported: a line's menu for the leader (make leader, kick), dragging a member to another
team (`/raid move` does it by numbers), and the raid bars (`RaidBar.xml`: a small window per member with its health
and nano, made by dragging a member out of the raid window; it needs the drag and other characters' stats).
Tests: `the_teams_show_only_in_a_raid`, `members_go_to_their_teams`, `the_teams_are_set_whole`,
`the_team_follows_the_server` (its raid part), `the_world_entry_message_has_its_team_part`,
`captured_world_entries_read_whole` (`MAO_CAPTURES`).

### Other players' crafts and flight (MOV-020, RND-135)

How the original knows, for a character that is not one's own:

- **At its announcement, from the full update**: `MonsterData` and `Scale` (set as stats 359 and 360 for a player
  too), and the **movement mode**, the thirteenth byte of the counted block behind them, which the character's
  vehicle reads straight into its status (FUN_1006f587 → FUN_10070981): 3 run, 8 sitting, 7 flying. With the
  flags' bit 23 there is also the scale the character had before a list set it (FUN_10064ea9: what `Scale` goes
  back to), which answers the old question of what is kept for one that logs in on its craft. In the captures:
  three players on a Yalmaha XL (template 99470 at 20, flying, 100 or 110 kept), one on another craft at 80, one
  flying on his own body, and several polymorphed (the leet, 17655, at 90).
- **Afterwards the server sends the cause, not the result**: `TemplateAction` 6 or 7 for the character (an item
  put on or taken off), and the client runs the item's wear list on that character itself (FUN_1007aa44 →
  FUN_1004d68c, no test for one's own). One capture has it: a player 41 m away puts a Yalmaha XL on, the message
  comes, and then only his moves, half a metre up and climbing; no stat, no spell list. A character that is
  announced again while present was sent a `Despawn` first.

Ported (`World::add`'s arm for a player's update, `World::others_wear`):

- A player announced with a monster body has that body at its scale (`crafts`, drawn by the road one's own craft
  takes), until announced without, gone (`Despawn`), or the playfield changes. Announced in mode 7 it flies, on a
  craft or on its own body. Announced sitting (mode 8) it sits; until now every announced player stood.
- An item put on in view: when its wear list has a body without criteria (the Yalmahas) and the character has
  none, that body; when it has the fly spell (the Yalmahas, a jetpack), the character stands up and flies. Taken
  off: the body goes if it is still that item's, the flight ends, and one who had been announced on the craft is a
  person again at the kept scale.

What rests on what: the announcement on ten captured players and the code; the mount on one capture and the code
(what the original draws in the seconds after it was not checked in a picture); **the dismount on the code alone**
(the message kind is captured for clothes and weapons, not for a craft). A missed dismount would leave a small
craft walking about until the player's next announcement.

Not ported, and found beside it (the first was mended the same day, see the next part):

- **`Buff` (`39343c68`) is the removal of a program, not a sign that it runs.** Its exec (FUN_100726e5 →
  FUN_10051041) ends the nano, and in the captures it comes when a program's time is up (15.07 s after a 15 s
  program's start), when it is cancelled, and just before a re-cast. This client takes it for "this nano is
  running" (`buffs_seen`, the 30 s of `morphed_until` and `riding_until`, a target's NCU list). The start is
  `CharacterAction` 0x62 on the character, which the client reads for its own only; and there `word(17)` is taken
  for the character the nano is on, where it is the caster (the character is the message's identity). So another
  player's polymorph started in view is still shown by the wrong signal; announced polymorphs are right now.
- A hoverbike program on another player, started or ended in view: its list has criteria, the client does not
  run it, and no capture shows whether the server sends the list to onlookers.
- `docs/FLIGHT_ORIGINAL.md`'s "Who runs the spell" says a worn item's list comes from the server. It does not:
  the client runs every spell of the wear list at `TemplateAction` 6 (corrected there).
Tests: `another_players_craft_and_flight_are_known` (`MAO_GAME` for the mount), `a_player_is_announced_with_its_scale`.

### `Buff` is a program's removal; program times are hundredths of a second (the same day)

Asked whether to mend the `Buff` misreading found above, the user said "Yes, fix it". Two faults, the second
found while mending the first:

- **`Buff` (`39343c68`) takes a program off a character.** The client took it for "this program is running" and
  counted a program as running for thirty seconds after each (`buffs_seen`, `active_buffs`, `buff_first`,
  `morphed_until`, `riding_until`: all gone). The programs on other characters are now kept as they are
  (`World::others_nanos`): those a character's full update lists, those the server starts on it in view
  (`CharacterAction` 0x62 **for that character**, the nano as its target, the caster in the first parameter, the
  time in the second: `program_started`), until its `Buff` (`program_ended`), its time's end, or the character's
  going. A target's NCU window (now with the real times, not an estimate from the first sighting), the auras,
  another's polymorph body and its vehicle are read from there. One's own programs leave the running list at
  their `Buff` too (the server's word; before, only the clock or one's own cancelling took one off), and a program
  **another player casts on one's own character now joins the list**: the client had taken 0x62's first
  parameter, the caster, for the character the nano is on, and read only its own casts. A polymorph's body goes
  with its program when the body is still that program's (FUN_100a7aa2's ending).
- **Every program time is in hundredths of a second**, on the wire (0x62's, the full update's list) and in the
  database (stat 8: 1440000 for a four-hour program). The client took them for milliseconds: the NCU window showed
  a tenth of each time (24:00 for four hours), and **one's own programs were taken off the client's list, with
  what they added to the stats, after a tenth of their time**, until the next world entry listed them again. In
  the captures: a program told 500 is taken off 4.95 s later (dozens of them), one told 1500 after 15.07 s, and a
  four-hour one's time left in the character's later updates goes down by 100 a second. The times are made
  milliseconds where they are read (`n3::active_nano_times`, `program_started`, the NCU's and the program
  window's totals).

Not changed: a hoverbike program of another player is still drawn from the program running on it (this client's
own way; in the original that list has criteria and would come as a spell list, which no capture shows for an
onlooker). The ending run's criteria gate (FUN_10050b90) was not read.
Tests: `a_program_runs_from_its_start_to_its_buff`, `the_active_nanos_are_found` (the times).

## The container item menu, own death, the pet window (2026-11-18)

The user: "port INV-050, DTH-014 and NPC-029". Read anew in the original by three readers; the claims the ports
rest on were checked against the code and the captures. None of it was seen in the live game.

### An item's menu in the container windows (INV-050)

GUI.dll FUN_100caf07 is connected for every item view (`InventoryViewBase_c`'s FUN_100ccde7), so a backpack, the
bank, a corpse, a chest and the overflow have the inventory's menu: the right button **held** on an item for
`MouseButtonHoldDelay` (0.3 s; the list's FUN_10041aa4 starts a timer at the press and FUN_100420fd raises the menu
for the item the button went down on). `loot.rs`:

- **Look at Item** (the info window, as Shift+click), **Create Reference**, **Delete Item**: in every view that is
  not read-only or a wear page, so on a corpse and another's chest too (the catalog's "not on read-only" was short
  of that). Not on the organization's contract here (its items' identities were not read).
- **Delete** asks first (10001's `Warning`, `ReallyDeleteItemX` with the name, Yes / No), then `N3Msg_DeleteItem`
  (0x1001c864): `CharacterAction` 0x70 with the item's own identity, `(0x6b, handle << 16 | place)` in a bag, chest
  or corpse, `(0x69, place)` in the bank, `(0x6e, place)` in the overflow, and the low and high template. Nothing
  is taken out here: the server's answer, the same action with zero parameters, does it (FUN_1004a5e9). A capture
  has exactly this for a backpack. The identity is made when the message goes out, and only while that container
  still has that item at that place: a delete cannot be taken back, a backpack opened again has another handle,
  and the one shared window goes from container to container.
- **Looter**, on a corpse for one in a team: free for all, a random looter, or a member of the team (enabled while
  the client knows that character); in a raid the teams first. `N3Msg_SetLootAccess`: the raid command 5 with the
  corpse, the item's place and whether to roll, and the looter. The submenus replace the menu (the popup here has
  none of its own); their labels are this client's words (the original's are literals in its program).

**A fault mended with it**: the server's own 0x70 for an item inside a bag (a combine's piece used up there) was
ignored, since only the inventory's was read; the piece stayed in the bag's window.

What rests on what: the delete inside a backpack on a capture, both ways; the bank's, the overflow's, a corpse's
and a chest's deletes and every loot access on the code alone. Found and not changed:

- **Create Reference** puts the link into the chat's line here, as the inventory's menu does. In the original it
  puts the item on the reference board (the Knowledge window: `RefMan_t::AddItemRef`), and a chat link is made
  only of what is dropped on the chat's line.
- **A plain right click** takes the item here. That is the original's for the bank, a chest and the overflow; on
  an item of **one's own backpack** the original uses or wears it straight out of the bag (`N3Msg_UseItem` with
  the bag identity), and on a **corpse** it does nothing (list flag 0x400).
Tests: `an_items_menu_follows_its_container`, `a_containers_item_is_deleted_by_the_servers_word`,
`an_item_is_deleted_by_its_own_identity`.

### One's own death: what was missing, and what was never there (DTH-014)

The entry listed five things as not ported. Read:

- **The reason in the other messages: ported.** Gamecode FUN_1005b3d8 (health to nothing, and the reason's line
  for one's own character) is called by five handlers, not one: `AttackInfo` (done before), `SpecialAttackInfo`
  (its last word, for its target), `HealthDamage` (its fourth word), `ShieldAttack` and `ReflectAttack` (their
  last word, which the schema calls a stat), and `CharacterAction` 0x99. In the captures the reasons are there:
  4 on 237 weapon hits and 48 specials, 5 on 8 health damages, 3 on a shield hit. **The capture's own death came
  by a nano program, `HealthDamage` with 5: this client told nothing for it.** `World::died_of`.
- **`CharacterAction` 0x99: ported.** Its case is `FUN_1005b3d8(Parameter2)` and nothing else: "this character
  died of this reason". No capture of the original has one; this client's logs have it a minute after a death
  the client had not answered, just before the server moved the character.
- **The death music mode: there is nothing to port.** `SandyInterface_t::SetDeathMusicMode` is an empty function
  (`ret 4`), as is `SetCombatMusicMode`; the drop-in agrees and is difftested. What sounds at a death is the death
  sample, which the client has.
- **The windows' reaction to the dead signal: the notes were wrong.** GUI.dll's `DeadMessage` and `AliveMessage`
  are not death and life: they are "the client's character object is gone" and "is in play" (every zoning, login,
  logout), and the dead signal has no listener that was found. What the original does at one's own death is stop
  all input (`CharDie_t` → `DisableUserInputMessage`: no keys, no mouse, no typing, the pointer off) until the
  character is in play again, about eight seconds in the capture. **Not ported, on purpose**: this client's
  deaths have at times lasted a minute or stuck until a relog, and a frozen interface would leave no chat and no
  way out. The moving, attacking and using are stopped already.
- **The resurrection (FUN_1003ec1f): the message is read, its shock is not.** It runs only from `ResurrectIIR_t`
  (`445f2a0b`: health and nano), which is in no capture or log; the ordinary return after a death is a teleport
  and a new announcement. The handler sets the health and nano, lets the character stand, and from level 15
  tells `Feedback_ResurrectionShockFillsYourBody`. Not ported: the shock itself (stat 247 at 5000, counted down,
  lowering the skills and refusing vehicles meanwhile, `Feedback_YouFeelOkAgain` at its end).

Found beside it and not changed: the original sends `CharacterAction` 0x98 **with the target** while its fight
target is a dying creature (`CharFight_t`'s frame; three of them after each kill in the captures); this client
sends 0x98 only for its own death. And the original sets a dead character's health to 1, not 0, once its death
has played out.
Test: `a_death_is_told_by_the_message_that_carries_its_reason`.

### The pet window: the programs on a pet, its place, its kept name (NPC-029)

- **The programs under each pet.** `PetView_c` (FUN_1007d817) puts an `NCUListView_c` under the bar row, the
  list the NCU and target windows are: a grid in the smallest icon size (19 by 11), spacing 7, borders 6, sorted
  by the time left, filled from the pet's own list of running programs, which is what `World::nanos_on` gives
  since the same day's `Buff` fix. No programs: no room taken; any: one row (FUN_1007d0fa). A pet that is away has
  none. A click on an icon shows the program's page (the original: Shift+click, or the held button's "look at"
  entry; a pet's programs cannot be cancelled from here). Not done: the flash of an icon in its last ten seconds,
  the icons' tooltip.
- **`PetType` is in the entity message after all** (the notes said it was not): the third of the three creature
  fields the client already stepped over (the capture's Ravening M-60 has 12, the Widowmaker Battle Drone 10).
  With it the window's places are the original's (`PetTypeToLocation`: 10 → 1, 12 → 2, 11 → 3, 15 → 6), and
  "Fixed Pet Buttons" holds pets by that place instead of by their family.
- **Remember Pet Names** (`AutoPetRenaming`, on at first, a third entry of the window's menu): when a pet of
  one's own is renamed (the server's `SetNameIIR_t`, `734e5a7b`, now read: any character's name changes with it)
  the name is kept for the pet's place, and a pet of that kind that comes with another name is renamed
  (`PetCommandIIR_c` with command 15 and the name). **Neither message is in any capture**: both are read from the
  code. Nothing is sent by itself until a name change has come from the server and been understood, so a wrong
  reading of it keeps the whole thing quiet.
- **`/pet rename <name>` carries its name now** (it sent the bare command): the pets named before the word, `all`
  for every pet, and with none named the selected target (FUN_100542ad). The original re-checks every pet's name
  on each pet's going and after zoning; here only when a pet comes.
Tests: `a_pet_has_its_place_and_its_kept_name`, `a_pets_name_is_kept_and_given_back`,
`a_pet_command_carries_its_pets_and_its_text`.

## Popup menus like the original, and options that did nothing (2026-11-18)

The user: "port SET-030 and WIN-111". Read anew in the original by two readers; what a port rests on was checked
against the decompile and the drop-in ports first, and what could not be found there was left. The menus were looked
at in offscreen renders; none of it was seen in the live game.

### The popup menu: separators, the mark at the right, sub-menus, sliders, typing (WIN-111)

`gui/popup.rs` is `PopupMenu_c` whole now, bar what no menu of the original uses (icons, more than one choice):

- **Entries of three kinds** (`popup::Entry`): text, a separator (5 high, a line of 0x606060 at 2 and one of 0xd0d0d0
  at 3; the pointer, the keys and the search pass over it, `IsStatic`), and a slider (`PopupMenuSliderItem_c`: a
  `Slider_c` in the label column, 80 wide at least, no label of its own).
- **The tail column**, as wide as the widest picture plus 10: `GFX_GUI_MENU_CHECK` for a ticked entry (it was drawn
  at the left before), `GFX_GUI_MENU_SUB_ARROW` for an entry with a sub-menu.
- **Sub-menus**: an entry rested on for 300 ms opens its own (200 ms when another is open already), 6 right of the
  parent's entries, to the left when it would leave the screen; the pointer may cross into it; Right opens it by key
  and Left closes it. A choice is told by its place through the menus (`popup::Event::Chosen(path)`).
- **Sliders**: only the knob is grabbed, the value is told while it moves and when it is let go, and the menu stays
  up. Only the chat window's "how solid" menu has them, as in the original.
- **Typing** finds the first entry that starts with what was typed (case put aside, a leading `/` put aside, the
  typed text forgotten after a second). Escape, Delete and Backspace close; Enter chooses; Tab pages; Up and Down
  do not wrap. Any button let go after a press in the menu chooses the highlighted entry; a dead one closes the menu.
- `chrome::menu_of` / `chrome::menu_path` are the windows' way to it; `chrome::menu` (one flat list, the entry's
  number) is unchanged for the menus that need no more.

**The menus changed with it:**

- **The chat window's menu** is one menu with its sub-menus, in the original's order: Visual (the mode's sub-menu,
  the two sliders' sub-menu for a borderless window, the four switches, always behind / on top), a separator, the
  channel to talk to, the channels shown (public and combat, each its list), the auto-subscribe switch, a
  separator, the chat settings, a separator, rename / delete / new window; this client's own "move to a window" entries
  after one more separator. Before, each of those was a new flat menu in the first's place, and "how solid" was two
  lists of ten steps. The sliders run 0 to 1 (the window not active first, then the active one), the window follows
  the knob, and each has its tip (the tip is a guess from the texts' names, `..._InactiveToolTip`: the item's own
  tip code was not read). A group ticked in a channel list still leaves the list up for the next one, which is
  this client's own.
- **The container windows' item menu**: Looter is a real sub-menu (in a raid: the teams, each its members' sub-menu).
- **The nano programs window's menu**: Auto Arrange is a sub-menu with its two separators.

Not done: the same sub-menu in the backpack's own menu (WIN-109) and the NCU window (its grid's order is not
ported). `MAO_UI_PREVIEW=popup` (`MAO_POPUP_PATH=7`, `MAO_POPUP_AT=x,y`) renders a menu with every kind
of entry. Tests: the five in `gui/popup.rs`, `the_windows_menu_holds_its_sub_menus`,
`an_items_menu_follows_its_container`, `the_arrange_menu_marks_the_grids_order`.

### Options that did nothing (SET-030)

The entry's "names" were done before (`ShowAllNames`, the organization's name overhead). Of the rest, what could be
confirmed in the code was ported, and no default look changes by it:

- **`FogMode`** (`VisualFog_t::SetFogMode`, DisplaySystem 0x10058409, difftested in the drop-in): at 0 the frame's
  gathered fog density is a tenth. The fog is never turned off by it (the mode kept is 3 for 0 too), and 1 to 3 draw
  alike. `viewer.rs` `fog_mode`.
- **"All effects"** (`ToggleAllEffects`, GUI.dll FUN_100c00ea): a change sets thirteen preferences to its value:
  the water, the wildlife, the space ships, the two clouds, the shadows, the six effect kinds and the smooth
  animations. `controlcenter::ALL_EFFECTS`.
- **`RealisticClouds` / `SimpleClouds`** (`VisualSingleCloud_t`'s frame, the drop-in's `fxsinglecloud.rs`): with both
  off the single clouds are not drawn. Not done: the original takes the realistic ones as off while the camera is
  under water, builds the simple-only ones with two sides for ten (RND-075), and its volume-dot effect draws only
  with the simple clouds on.
- **`WaitForVertSync`**: this client's own `vsync` setting follows a change of the option (the reverse is not
  shown: the option's box keeps its own value).
- **The seven `VisualFlags` boxes** (`opt_bitnum`, `OptionCheckBox_c` FUN_100c22ba / FUN_100c21b7): **a fault**.
  Each box showed "the whole number is not 0" and wrote 0 or 1 over all seven; now each shows and sets its own bit
  (helmet 2, shoulder pads 0 1 3 4, social tab 5 6). The number is still not sent to the server (the original's
  `CharacterAction` for stat 673 is not ported), so the boxes change nothing others see.
- **Escape** closes the raid window (`esc_raid`, FUN_10103127), the team search (`esc_lft`, FUN_100f0d2e) and the
  city terminal (`esc_cityterminal`, FUN_100e7847) by their options. **A fault with it**: the shared container
  window of a bag that is not one's own read `esc_backpacks`; the original reads `esc_chests` there and
  `esc_backpacks` for one's own backpacks' windows, which Escape did not close at all (FUN_100ccefe). `esc_vehicle`
  is the mech selection's and `esc_towerbuildmenu` the tower menu's: neither window exists here.

**Left, and why:**

- `ViewDistance`, `DisplayCharViewDistance`, `ZoomTo1stPerson`, `MouseWheel` (for the camera): the code that reads
  them is in none of the decompiles at hand (GUI.dll only stores them; `MouseWheel` = 1 there connects the wheel to
  the sidebar's scrolling). The reader's figures for them (a far plane of the value times 1000, characters within 5
  to 80 m, wheel modes) could not be checked, so nothing was built on them. This client's own view and character
  distances (`/setoption view_distance`, `char_distance`) are as before.
- `FadeCharacter*` and `ShowMyCharacter` (the own character fading near the camera, hidden in first person): a real
  port of its own, not started.
- `RealisticWater`, `Wildlife`, `IsSpaceShipsShown`, `SmoothAnimations`: what they switch is not in this client.
- The Direct3D 7 ground and texture options, the advertisements, the hotbar and fade options, and
  `AutoTargetMOB` / `AutoTargetPvP` / `DisableXPGain` (their message was never seen).
Tests: `the_fog_mode_thins_the_fog`, `a_bit_box_shows_and_sets_its_bit`, `escape_closes_by_the_original_options`.

## The launcher closing on Direct3D 12: "Invalid surface" (2026-11-18)

A player of the released 0.1.5 sent the error box and the crash log: the launcher closed about a second after it
opened, with wgpu saying "In Surface::configure: Invalid surface". Reproduced here, mended, and tried again.

### What it was

The launcher opens its news page (a WebView2 child window) half a second after its first frame and sets its surface
up anew right after (`launchwindow.rs`, `Launch::draw`). That was done inside the frame's own block, with the frame
presented but **the view of its picture still alive**. Direct3D 12 does not make a window's pictures anew
(`ResizeBuffers`) while anything still refers to one of them; wgpu reports that as "Invalid surface", and an error
nobody catches is a panic. Vulkan lets it pass, which is why it was never seen here: this machine draws with Vulkan,
by the system's choice and by its settings. A machine whose choice is Direct3D 12 lost the launcher at every start.

- **Reproduced** with the Direct3D 12 device named in a scratch `mao.ini` (`adapter = <name> (Dx12)`) and
  `launch_probe.py`: the same error, 2.4 s after the start, in `Launch::draw` under `window_event`.
- **Mended**: the view is let go before the surface is set up anew. Tried again the same way: the launcher stays
  and its page comes.
- **The login window had the same fault** (`launcher.rs`): the error page of a refused login is opened, and the
  surface set up anew, with the frame's view alive. On Direct3D 12 a wrong password closed the login
  (`MAO_LAUNCHER_DEMO=error` showed it). Mended the same way and tried.
- **Found with it: the game's own process never asked for the device of the settings.** Only the launcher and the
  login called `mao_render::prefer_adapter`; the game, a process of its own, drew with the system's choice whatever
  was chosen on the launcher's Visual page. It asks now, and prints `graphics device: <name> (<backend>)` at its
  start. With that, one test login (Maotesttwo) was run on Direct3D 12 through the login hall into the game: the
  scene and the interface draw, nothing failed in 44 s. The game's other places that set the surface up anew (a
  resize, the vertical sync, a lost surface) do it with no frame in hand.

Not done: catching a failed surface set-up instead of ending (an error scope round it, a few tries, then the
message); nothing was seen to need it once the two faults were out. How to try Direct3D 12 here again: the device's
name with `(Dx12)` as `adapter =` in the scratch settings of `launch_probe.py` (`hardencfg`) or of
`win_shot_live.py` (`inv/cfg`, whatever its own note says).

A player on 0.1.5 can get past it until a new release: a file `launcher.ini` beside `mao.ini`
(`%APPDATA%\\ModernAnarchyOnline`) with the line `page = off` keeps the launcher from opening its page, and with it
from the fault (tried with the installed 0.1.5: it closes with the page on and stays with it off); the login
then closes only at a refused password.

## The weather that falls, the city window, the container windows as lists (2026-11-18)

The user: "port WIN-109, WIN-110 and RND-138". Each was read anew in the original by a reader who had to name the
file of every rule; what a port rests on was then checked in the decompile, the drop-in ports and the game files.
None of it was seen in the live game.

### The weather that falls: rain, snow, the storms' lines and tints (RND-138)

**The entry's cloud layer was a misreading.** `VisualEnvFX_t` +0x114 is not something drawn: it is one float of
the frame block (the weather's cloud cover, entry 2, written by Gamecode FUN_100adb33). The cloud layer is the
effect scripts' `ThickClouds` mesh (`skydome_clouds.abiff`, its alpha `GAME.ThickCloudsIntensity`), which this
client has drawn and fed since the scripts' visuals were ported. What the weather draws and this client did not:

- **The rain or snow is entry 0, and nothing was drawn for it.** The notes had called entry 0 "fog" and drew
  entry 4 as the rain. `Environment_t`'s frame (FUN_100becbe) gives entry 0 to the object whose sound is
  `SM_Sandy_Env_Rain` (FUN_100b9da8): shown while it is above 0, at its own value as the alpha, under no
  preference. Its look goes by entry 8 (FUN_100b98fe; the numbers read from Gamecode.dll): above 4 it is rain,
  (0.4, 0.5, 0.7), falling at 16.2 with 0.4 of the wind at 0.75 of the alpha; at −0.5 and below snow, grey 0.8,
  falling at 2.2 with all of the wind; between them a blend.
- **The four storms (entries 3 to 6) are blown line clouds, not rain.** Their colours are their fogs', their fall
  0.3 (0.6 for the dark one) and their part of the wind 0.9; alpha `entry³ × 0.45` times 0.318, 0.7, 0.718, 0.318
  (FUN_100b9ba5, FUN_100b9b10 and its four callers' arguments). The lines follow the `UseStormParticles`
  preference, which was not read (the client went by the environment effects' switch instead).
- **Each storm tints the screen** in its colour, flickering (`VisualFader_t`): `flicker × 0.55 × entry³`, the
  flicker over 1.07 each frame and, one frame in five, up by 0.17 to at most 0.85. New here.
- **The line cloud is `VisualRain_t`** as the drop-in has it (`rain.rs`, difftested): 350 lines in a box of 15
  hung on the camera, each line's tail where its head was a frame ago. `precipitation.rs` is that now
  (`LineCloud`, `WeatherFx`); the client's own particle box, counts and speeds are gone. A check on the way: the
  original turns the fall and the wind into the cloud's frame by three matrices, which is exactly the inverse of
  the cloud's own rotation (tried with numbers), so the lines fall straight down in the world whatever way the
  camera looks.
- **A fault mended with it: the clouds' entry.** The sky's dimming (`1 − x / 2.333`) and the area fog's light
  (`1 − x × 0.8`) were read from entry 8; the frame reads the table's byte offset 8, its third float, the clouds
  (entry 2). Entry 8 is the rain-or-snow number. Overcast weather now dims the sky and the fog; the old reading
  dimmed them by the cold.

This client's own in it: the lines are thin sheets about a picture element wide where the original draws lines;
the tint's sheet stands at a tenth of the original's distance and size so that nothing near covers it; the wind is
this client's own wind value; the flicker runs per frame as in the original, so it is quicker at a higher frame
rate. Not read: the light handed to the rain's colour (the day's intensity is used, as the rain's fog does).
Not ported: cloud shadows (no reader of `UseCloudShadows` was found; they are switched off in Rubi-Ka's script).
`MAO_WEATHER=<entry>:<value>,…` sets the table by hand, for a picture (`map <playfield> --image`) or a live look.
Tests: the four in `precipitation.rs`.

### The city controller: the dialog that buys the land, the lock, the tip (WIN-110)

(The Escape option of the entry was done the same day with SET-030.)

- **GUI type 1, the land-buying dialog.** It is the terminal's own opening (`AOTransportSignal` code 5) with its
  type 1 (`SlotOpenCityTerminal`, GUI.dll FUN_100e6c24: types 1 and 2 have a window). FUN_100e7339: a window of
  389 by 239 with its own kept place (`CityTerminalBuyDlgConfig`; here the layout name `CityTerminalBuy`) and two
  tabs. "Buy Land Area" is `BuyLandView_c` (FUN_100e6388), made in code, not from `CityTerminal.xml`: the text
  `CityTerminal_BuyLandDialogText` (category 10002) fed the price and then the upkeep, which are the playfield's
  city record's two numbers (the client read them already, `citymap::Data::costs`), and the buttons Purchase and
  Cancel at the bottom right. "City Map" is the house view in its show-only mode with no control view.
  **Purchase is enabled in lease mode 3 only (FUN_100e6208), and its press sends `ClientRequestBuyIIR_c` at once
  (FUN_100e6225 → `CityAI_c::ClientRequestBuy`): the original asks nothing first.** Cancel and the window's
  close button send the session's close request. `Command::Buy` → `mao_net::city::buy`, which was built and
  tested before and never called.
- **A fault that went with it:** a type 1 opening left the client holding a session with no window: nothing could
  send the close request, and a later opening of any controller was ignored while that session was held, until
  the server ended it or the character zoned.
- **The lock.** An item that waits on the map as a house about to be built is locked in the inventory
  (`InventoryItemLocker_c`, set by the map's FUN_100e4cef; taken off by Delete, by Construct at its press, by the
  item's going and by the map view's end: no answer of the server is waited for). Here the lock is not kept: each
  frame the places that wait on the map (`Map::locked`) are the locked ones (`gear::city_locked`), so it cannot
  stick. A locked item shows `GFX_GUI_INV_ITEM_LOCK` in its cell's bottom right corner (FUN_10040bda), is not
  taken up, split or added to a trade (FUN_100cc6e8, FUN_100cae1b), is not used, and an item let go on it gives
  the line `DroppedItemWasLocked` (category 10003, FUN_100cc855). Not done: the same mark on a shortcut bar's
  copy of the item; the picture's own colour (`GUIConfig` colour 0x1000000, not read).
- **The tip.** Over the map, in every mode, the pointer's place in the world is a tip (FUN_100478c7, the map
  view's answer to the tips' question): the title `PointerPosition` (category 10000), the body "%.1fx%.1f" of the
  world's x and z under the pointer, not snapped to the tiles. It comes through the general tip (half a second's
  rest); a button held takes it away. A small departure: this client's tip goes while the pointer moves, where
  the original leaves the old one up until the next rest.

What rests on what: **no capture has a type 1 opening, a purchase or its answer**; which lease mode the server
sends with a type 1 opening is not known, nor what it answers. The window, its texts and its enabling are from the
code and were looked at in offscreen pictures (`MAO_UI_PREVIEW=cityterminal MAO_CITY_TYPE=1`, `MAO_CITY_LEASE`,
`MAO_CITY_TAB`; the lock with `MAO_UI_PREVIEW=gear MAO_CITY_LOCKS=<places>`). **Purchase must never be pressed in
a test: one click, no question, the price shown (hundreds of millions).**
Test: `the_buying_text_and_the_map_tip`.

### The item windows' own menus: List Mode, Auto Arrange, Labels On Top (WIN-109)

**Where the menu comes from.** In the original it is not a click in the item view (the list's mouse handler does
nothing off an item, FUN_10041aa4): it is the frame's menu button, which asks the shown view to fill the menu
through its virtual slot +0xf8 (FUN_100cbace for `InventoryView_c`, FUN_100e20d6 for `WearView_c`; the frame adds
its own Autofade / Close entries after). This client's windows have no such button, so the menu is on the right
button **beside the items**, as the nano programs' and the NCU's are.

**The inventory and every container** (FUN_100cbace; `invwindows.rs`, `loot.rs`):

- **List Mode**, with its mark: the items as rows, columns Icon (16), Name (200), Count (30), Quality (100)
  (FUN_100ce55d through FUN_100ccde7's flags 0xb: no price column in these windows), sorted by the name at
  first; a header press sorts, the widths can be dragged, and both are kept. Count is the stack (nothing counted
  reads 1), Quality the item's.
- **Auto Arrange**, a sub-menu (FUN_10041637, the one the nano programs window has), dead in list mode. For one's
  own windows: "Don't Auto Arrange" (marked while the items stand where they were put), a separator, "Unsorted"
  (packed in their present order), "By Name", "By Count", "By Quality", a separator, "Sort Ascending". For
  another's container (a corpse, a chest, the overflow): only the three "By" entries and the order.
  Choosing "Don't Auto Arrange" leaves every item in the cell the arranging had it in (`SetAutoArrange(false)`),
  from where it can be placed by hand again.
- **Another's container starts arranged by name** (FUN_100ce55d: `SetListSortColumn(1, 0)` and the grid's sort
  (1, 0)): this client showed a corpse's items in their places' order.
- **Rename** stays, third, on one's own backpacks.
- While a view is a list or is arranged it has no cells to put an item in (FUN_100cc855): an item let go on an
  empty spot stays where it was, two items do not change places, stacks still join, and a split's new stack
  takes the first free place.
- Kept per window: the inventory, each own backpack by its identity, the bank, and one for all other containers
  (the original keeps a file each, `Containers/*.xml`; another's container has a file by its handle there, which
  is new each time).

**The wear window** (FUN_100e20d6): **Labels On Top**, with its mark, on all four pages (`SetGridLabelsOnTop`,
FUN_100e1c7b; the labels white over the items, or grey under them), kept.

**Left:**

- **Unlock All Items** (the wear menu's second entry): it empties the locks that the item menu's Lock Item puts
  on worn items (`InventoryItemLocker_c`, the `EquipmentLocks` value), which this client does not have.
- **The bank placed by hand.** The original's bank is an own-style view ("Don't Auto Arrange" offered); here the
  bank's items cannot be placed by hand, so its menu has "Unsorted" (the places' order, as before) and the three
  "By" entries only.
- The Inspect window's Labels On Top (its own kept value in the original); the name's rarity colour in list mode;
  the frame's menu button and its Autofade / Close entries.
- The nano programs window's "Don't Auto Arrange" is live in the original too (FUN_100d4f90 passes the flag);
  here it stays dead, since programs cannot be placed by hand.

A note corrected: the "list features 0x15" this client's comments give for these lists is an argument of
FUN_100ce55d that it never reads; the lists' feature flags are 0 (0x10 for the wear pages).

Looked at in offscreen pictures: a container as grid, as list and with its menu, an own backpack's menu and its
arranging, the inventory in its three states, the wear page with its labels under the items
(`MAO_UI_PREVIEW=loot` with `MAO_LOOT_LIST`, `MAO_LOOT_SORT=<n>`, `MAO_LOOT_MENU`; `MAO_UI_PREVIEW=gear` with
`MAO_INV_LIST`, `MAO_INV_SORT=<n>`, `MAO_WEAR_LABELS=0`). **The mouse paths were not tried**: letting items go in
list mode and in an arranged grid, and switching the arranging off, rest on the code alone.
Tests: `the_arrange_menu_follows_the_container`.

## Lamps that glow, effects in mission rooms (2026-11-18, the first batch with the new division of work)

The user: "lets get started", after setting up the local model and saying which work is whose. I took the high
entries that can be judged in offscreen pictures first: RND-136, RND-137 and DTH-013. Each was read by a reader on
a smaller model with the rule that every claim names its file; the tracing and the checks were mine.
None of it was seen in the live game.

### Statel effects with a colour or a size of their own (RND-136)

How the original puts the name's colour and size on the effect, which the notes had left unread:

- The connector's name gives an `AttractorEffectData_t` its template, up to four colour values and a size
  (DisplaySystem FUN_1006d49a; `mao_formats::attractors` has had that table). A statel's mesh keeps them
  (`VisualMesh_t::GetEffectData`), Gamecode makes an `n3GenericEffect_t` for each (FUN_1010ca78), and when the
  mesh shows, N3.dll's `n3GenericEffect_t::EnableVisibility` (0x1000806d, read by disassembly) hands the data to
  the engine's slot 0x68, which is `_EffectHandler_t::EnableEffect` (Gamecode 0x100ce0dd).
- `EnableEffect` makes the effect (slot 0x58: the template and the connector) and then, when the data's word has
  bit 0, calls `SetEffectColor` (0x10016935) with the four values in the name's order, which calls the control's
  `SetStartColor` **and** `SetStopColor` with the fourth value first: the name's fourth byte is the alpha. With bit
  1 it calls `SetEffectSize` → `SetSize`.
- What a control does with them is its own matter; the base class does nothing. The single quad
  (`_GfxControlSprite_t`): the colour takes the place of both of its colours (FUN_100f5fa3, FUN_100f60e3: the
  template's words 15 to 22), and a size above 0.01 multiplies its two widths and two heights (FUN_100f6102).

`gfx::Visual::with_overrides` does that for the quad, and the effects are grouped by template, colour and size
(`net::group_statel_effects`). **Every effect with a colour or size of its own in the two playfields looked at
is template 7006, a quad** (Jobe Research: 180 of 181; Andromeda: 24 of 138), so the lamps there have their
halos now; before, all of those were left out. Not done: the colour on fire, smoke and sparks (the original's
controls take it; none was seen with one), which draw as their templates have them.
Looked at in an offscreen picture of Jobe Research at dusk: white and red glows at the lamps where there were
none. Tests: `a_halo_takes_its_connectors_colour_and_size`.

### Statel effects in a generated building, and in rooms (RND-137, the first half)

`statel_effects` returned nothing for a generated building (a mission's), and for an ordinary interior of rooms
(a shop) it took the statels as the file has them, inside each room's own frame, so their effects stood at
places that are nowhere in the world. The props themselves were placed by their rooms all along
(`add_props_with`); the effects now go through the same placing (`placed_statels`: a zone's statels by its room,
a generated building's by each room it was built of, `rooms::place_statel`). In the original the effects hang on
the mesh's connectors, so they go wherever the room's copy of the statel goes (the generator's `SetPos` /
`SetRot`, Gamecode FUN_100ca6f8).

The Arete capture's alien ship (template playfield 382): 21 effects, twenty smokes and a lamp's halo, each inside
one of its eight rooms; none before. `statel-effects <playfield>` takes `MAO_GENERATED=<hex>` as `map` does.
Test: in `the_ship_rooms_meet_at_the_servers_doors`.

**The entry's second half is left.** `eff_hologram`, `eff_space` and `eff_clear` are not effects but ways a mesh
is drawn (DisplaySystem's `set_rendering_effect` 1, 2, 3: `visualmesh.rs` in the drop-in): the hologram a
blended draw with two scrolling textures (resources 0x28022 and 0x28023, phases 0.2 and 0.15 a second) in place
of the mesh's own; "space" a draw with other states; "clear" a clearing of the view before the mesh. This
client has no draw of its own per mesh; which statels carry these names was not looked for. The templates
12200, 12570 and 96003 are named by `eff_universal_<n>` connectors and are not in `gfxtweak.bin`; where the
original has them was not found.

### A corpse this client has no body for (DTH-013)

The reader could not find where the original draws such a corpse and took the corpse for an invisible chest. It
is not. Traced here:

- **A corpse has a CAT mesh of its own.** `Corpse_t` (a `Chest_t`, constructor Gamecode FUN_1007eb9c) is made
  with its visibility off. The announcement (`CorpseFullUpdateIIR_t`, reader FUN_1009fa4c) carries the corpse's
  stats, among them `CATMesh` (0x2a), the dead one's own mesh, and the body's scale (0x168); after the name and a
  spell list the dead one's identity again, its clothes and its texture overrides. The message's set-up of the
  new corpse (FUN_1009f967) hands the clothes to the corpse (FUN_1007efee, kept at +0x1e4) and the textures to its
  visual (`SetTextureDataList`), and the corpse's own virtual function FUN_1007ed2c puts them on its CAT mesh
  (`SetCATTexture` for each part, the skin by breed and sex, a head mesh when it has one, `SetBodyScale` of stat
  0x168 over 100).
- **It is shown when the dead character is not.** FUN_1007eaf0: with a mesh, the corpse turns its own visibility
  on unless the dead character exists and is visible. So a creature that dies in view lies there itself and the
  corpse stays hidden; a corpse found on arriving, or one whose creature this client never had, shows by its own
  mesh.
- **How it lies.** The corpse's stats could name a pose (0x1a1, `CorpseAnimKey`), but none of the 371 captured
  corpses has it, and how the original poses the mesh without it was not read. Its own screenshots answer for
  the look: the capture of 2026-10-31 15:05 arrives at six Hiathlin corpses it never saw alive, and they lie on
  the ground.

Here (`mao_net::n3::corpse_look`, `World::remains`, `overlay_model`): a corpse with no body is drawn from its
announcement: its mesh at its scale, turned as the message says, in its clothes and textures, lying at the end of
a death of that mesh's creature (the first creature template that names the mesh gives the animations,
`Characters::template_of_mesh`; `die-knees` and along the original's fallbacks, held at its end from the first
frame). It goes when the server takes the corpse away. 30 of the captures' corpses are of this kind (their
creature was never announced).
Looked at beside the original's screenshot: the same three bodies by the same rock (`net <game> gear-preview`
on the capture cut just after they come). **A guess in it: the pose**; the original's may be another of the
creature's deaths. Not done: the skin by breed and sex and the head mesh for a humanoid's corpse (the creature
templates' own are used). Test: `a_corpse_says_how_it_looks` (a captured announcement).

## Pictures in text, item locks, the frame menu, locked doors, Notum Wars windows (2026-11-18)

The user: "OTH-010, ZON-017, WIN-117, WIN-109". Four readers on a smaller model read the original, each told to
name the file of every rule; what a port rests on was checked here against the decompile, the drop-in ports and
the captures before it was written. None of it was seen in the live game.

### Pictures in the text (WIN-117)

The markup's `<img src=...>` was read into an item of its own, but the words were measured with no picture sizes, so
it never took room and nothing drew it. The original (GUI.dll; `gui/pictures.rs`, `gui/text.rs`, `gui/controls.rs`):

- **The source** (FUN_1015efd3): `rdb://` or `tdb://`, either case, and after it an expression that gives a number
  (`ExpressionParser_c::ParseExpression`, then `Variant::AsInt32`). `rdb://<n>` is the icon `n` of the resource
  database: `GuiResourceManager_t::GetRDBTexture` (Interfaces.dll 0x1000b6ff, disassembled) asks for type 0xf6958
  and names it `e_RDB_Res_Icon`. `tdb://<n>` is the skin's picture number `n` (`GetGuiTexture`), numbered by
  AFCM.dll's table of the pictures' names; `id:<NAME>` is the expression for a name's number, so
  `tdb://id:GFX_GUI_NPCCHAT_BULLET` is that picture. A number of 0 is the skin's picture 0xb9
  (`GFX_GUI_DEFAULT_ICON48`) whichever the prefix. A source with neither prefix, with nothing after it or with no
  number to be had leaves the tag as text. The tag's only other attribute read is `title`.
- **Its room** (`_UpdateWordSizes` 0x1015dab9, `_UpdateWordSize` 0x1015c7a9): the picture's own width and height,
  never scaled; the width is added to the word it stands in and the word is as high as the highest thing in it. A
  line is as high as its highest word (`_ReWrap`).
- **Where it is drawn** (`_RenderLine` 0x101613f0): copied whole at the pen, which is at the line's top
  (`LineDesc_c` +0x10 less the scroll), with pure green as the see-through colour and no tint
  (`SpriteInfo_t::Copy` with 0xff00, as the DisplaySystem drop-in ports it); the pen moves on by its width. A link
  round it is the picture's too (`_UpdateInheritedData` hands the link to what follows it).
- **Text beside it hangs from the line's top too.** `SpriteInfo_t::RenderGlyph` (the drop-in's port) draws a
  glyph's rows down from the pen, and the underline is at the pen plus the glyph's height less 2. This client stood
  the words of a line on its bottom, which only shows where a line holds two heights; it is the top now.

Here: the skin is read by name, and AFCM's table is not carried, so of bare `tdb://` numbers only the four the
original's own markup writes are known (FUN_1003f534: 0xb7, 0xb8, 0xb9, 0xe3); any other draws nothing. An icon the
database has not takes no room (the original logs it; what it shows then was not read).
Seen offscreen: the NPC answers' bullets, which the client's own markup has written all along
(`MAO_UI_PREVIEW=npcchat`; beside the original's screenshot of an NPC's answers: the same orange bullet at the
line's top), and icons, the default icon and a bullet in an info page (`MAO_UI_PREVIEW=infopage` with a `text://`
page). The mission page's rewards are the other place: an item with no icon of its own shows the default icon now
(`<img src=tdb://185>`, FUN_1003f534) where it was left out. No captured chat line, NPC text or info page has an
`<img>`: the server sends none in what was captured.
Tests: `a_source_is_a_prefix_and_a_number`, `what_is_no_picture_stays_text`.

### What was left of the item windows' menus (WIN-109)

**A worn item locked to its place.** The original (GUI.dll FUN_100caf07, FUN_100ca3ae; Gamecode FUN_100266c9):

- A worn item's menu is Look At, Create Reference and then "Lock Item" or "Unlock Item" (text names `LockItem`,
  `UnlockItem`); "Delete" is only in the menu of an item that is not worn. The lock's entry is left out for an item
  whose stat 8 (`TimeExist`) is above 0.
- Choosing it puts the place's identity (0x65 to 0x67 and 0x73 with the place's number) into
  `InventoryItemLocker_c`, the same set that holds the items waiting on the city map, and into the character's
  kept `EquipmentLocks`. The item shows the lock's picture at the bottom right of its cell (FUN_10040bda).
- A locked item is not taken up (FUN_100cc6e8), not used, worn or traded by the mouse (FUN_100cae1b), and an item
  let go on it is refused with `DroppedItemWasLocked`. Gamecode asks the kept list itself: nothing is worn on a
  locked place (`N3Msg_IsItemPossibleToWear` asks for the place the item is to go to) and nothing is taken off one
  (`N3Msg_IsItemPossibleToUnWear`), each with `Feedback_EquipmentLocked`.
- The wear window's "Unlock All Items" (FUN_100e1f9b) takes the lock off all 64 wear places and empties what is
  kept. It is never greyed, and only the character's own window has it.

Here (`gear::equipment_locked`, `set_equipment_lock`, `unlock_all_equipment`, `gear::locked`; `invwindows.rs`;
`net.rs` `equipment_locked`): the same, with a bit for each wear place kept per character among the windows' kept
numbers (`EquipmentLocks<character>` in `tabs.ini`; the original keeps a list in the character's preferences).
Departures: the item's stat 8 is the template's (an item's own stats are not kept here); the lock does not go
with an item that the server moves (the original's set follows such a move, its kept list does not); the inspect
window has no lock entry (the original's code would offer it there, on the viewer's own set by the look of it;
not read to the end, so not done). **Changed with it: "Delete" is no longer in a worn item's menu**, as in the
original.

**The bank placed by hand.** The bank has the original's "own" flag (FUN_100ce55d sets `+0x14c` for it), so its
menu offers "Don't Auto Arrange" and it starts not arranged. The places are the client's own matter there too: each
item view keeps `item_id` / `item_pos` pairs in its own file (FUN_100cdfbb), nothing of it is sent, and an item
let go on a bank cell from elsewhere is sent with `N3Msg_ContainerAddItem` naming no cell, the view keeping the
cell for the item that comes (the first free one when the cell is taken). Here: `LootView::placed`, 102 cells in
`places.ini`, an item moved to an empty cell or changing cells with another by the mouse, and an inventory item
let go on an empty bank cell stands there when it comes (`awaited_cell`: the next item that has no cell yet).
**Not done: joining two stacks inside the bank by the mouse** (how that goes on the wire for the bank is not
known here; an item let go on a stack it could join stays where it was). The bank was ordered as the server sends
it before; whoever had it so finds it "not arranged" in that same order.

**The inspect window's own Labels On Top.** The inspect window is the wear view made for another character; its
menu has the one entry, kept apart (`InspectViewConfig`; here `InspectLabelsOnTop`). Read with it: when the window
is made, a kept "labels under the items" is set on three pages only, before the social page exists (FUN_100e24b8
calls `SetGridLabelsOnTop` three times), while the menu's entry sets all four (FUN_100e1c7b). Ported so for both
windows: after a restart the social page's labels are over its items again until the entry is chosen.

**The frame's menu button.** The original's window frame has a button at the left end of its title
(`WndBorder`, FUN_1015ae85: `GFX_GUI_WINDOW_ICON_I`), which shows the window's menu under itself, five below its
bottom (FUN_1015aa2f): the view's own entries, then "Autofade Window" and "Close Window" (FUN_1015b34f). Here
(`chrome::window`, `chrome::frame_menu`, `chrome::close_entry`): every framed window has the button, the title
moved right by it. The inventory, the wear window, the inspect window and the containers show their menus by it
(the same ones the right button beside the items shows) with "Close Window" after their entries; any other window
shows the one entry. **Not there: "Autofade Window"** (this client's windows do not fade; its pin keeps a window
open at Escape, which is another thing), the button's pressed look, and the button on a window docked at the
screen's edge.

Seen offscreen: the wear window with two locked places (`MAO_EQUIPMENT_LOCKS=6,8`), the bank not arranged with its
menu (`MAO_LOOT_BANK=1 MAO_LOOT_MENU=1`), the frame's menu on the inventory, the inspect window and the mission
window (`MAO_FRAME_MENU=<window's name>` shows the frame's own). **Not tried: every mouse path** (locking by the
menu, a refused move, items moved in the bank, an item put into the bank on a cell).
Tests: `the_locked_equipment_stays_as_it_is`, `the_arrange_menu_follows_the_container`.

### The locked door's question (ZON-017, what was left)

Read again in the original (Gamecode; `Door_t`'s vtable by RTTI: +0xdc the touch FUN_1007fccc, +0xfc "locked",
+0x100 its own access FUN_100857ac, +0xac open):

- **Locked** is bit 0x40 of the door's own flags word, which the door's set-up (FUN_1007fdd4) clears unless stat
  259 has the bit, and which the server's `DoorStatusUpdate` sets and clears: its apply (FUN_100a015a) unlocks the
  door when the message's locked byte is 0 and locks it otherwise (which shuts an open one), then opens or closes
  it by the open byte, then sets the door's stat 195 to the message's key.
- **The touch** (FUN_1007fccc) of a locked door asks FUN_1007fad8 and opens the door for one it lets through,
  telling the server nothing; one it refuses is pushed back from the door (FUN_1007f8ee).
- **The question** (FUN_1007fad8), in its order: a door with an exit (stat 189) lets everybody through; so does
  one that is not locked; one whose stat 259 has 0x10 lets nobody; one whose stat 259 has 0x20 lets through who
  holds an item whose stat 195 is the door's (FUN_1004777e: every item of the inventory, and of the bags in it);
  then the door's own access (FUN_100857ac: the door's requirements when stat 259 has 1, then a list the door
  keeps, of identities and of access cards: an `AccessCard_t` in the backpack whose stat 195 has the key's upper
  28 bits and at least its lowest four, or whose stat 188 is the key, FUN_100472d0 / FUN_1007d041); then
  everybody inside an owned building (`PlayfieldAnarchy_t::IsOwnedBuilding`, which does not look at the
  character; the team's part after it cannot be reached). The one line it says,
  `Feedback_YouNeedFreeEntryInInventory`, is for a door whose list holds a card to give when the backpack is
  full; a refusal is silent otherwise. The catalog's "team's mission" is not in it.

**What the game's data has**: of the 5,082 doors in the playfields' tables 13 are locked (11 in Will To Fight, 2 in
The Crypt of Home), all with flags 0x68 (locked, a key door) and no key of their own: the server gives the key.
All 201 doors the captures' servers announce have flags 0 and no key, and all 306 captured door words are
"unlocked, no key". So no locked door was ever seen being passed. `mao-client <game> doors` lists the tables'.

Here (`net.rs` `door_lock`, `door_may_pass`, `World::tick_doors`, `keys_held`; `main.rs` `DoorStats`,
`door_stats`):
- The server's word locks and unlocks a door now, and its key is kept. Before, a door was locked or not by its
  flags when it was first known, for good: a door the server unlocked stayed shut at the approach.
- A locked door in reach asks the question and opens for one who may pass: a door with an exit, a key door when an
  item's stat 195 is its key, any door inside an owned building (the playfield message that
  `Message::owned_building` reads; taken to be what `IsOwnedBuilding` asks for).
- **Not asked: the door's own access**, its requirements and its list, which are not kept here: a door only they
  would open stays shut until the server opens it. An item's key is its template's stat (an item's own stats are
  not kept). The line about the full backpack hangs on the list and is not said, and nobody is pushed back.
Tests: `a_locked_door_lets_through_who_may_pass`, `a_locked_door_opens_at_the_touch_of_one_who_may_pass`, the door
word's lock in `door_tests`.

### The Notum Wars and battlestation windows (OTH-010): surveyed, one ported

A survey of the six windows the entry names, by a reader; the signup terminal's part was checked here and ported.
**None of their messages is in any capture**, so everything below is the decompile's word alone.

**How four of them are opened.** A used terminal whose template has the spell 0xcf7b names a window kind (its stat
39; Gamecode FUN_100a9cf4, which `client_window` already reads for the market and the mail) and GUI.dll's
FUN_1002e2fe opens it: 3 the signup terminal, 4 the laser target, 5 the airstrike, 7 the vehicles (0 the market, 8
the mail). Kinds 4, 5 and 7 are only opened for a character whose stat 389 has bit 0x20; kind 3 has no such test.

| Window | Class, view | Opened by | Asks and gets | Here |
|---|---|---|---|---|
| Signup terminal | `SignupTerminal_c` FUN_100f499a, `Views/SignupTerminal.xml` | kind 3 | CharacterAction 0xfe (how the character stands), 0xfd (join, the side as the second parameter, after `SetStat` of 0x29c), 0xff (leave); `QueueUpdate` `2c2f061c` with the place | **ported**, `signup.rs` |
| Laser target, airstrike | `LaserTarget_c` FUN_100f2e32 (one class, the mode from the kind), `Views/LaserTarget.xml`: a list "Player Name", "Area", "Level Range" | kinds 4 and 5 | CharacterAction 0x100 (refresh), 0x102 (orbital attack), 0x103 (airstrike) on the chosen row's identity (0xc73d, row); `LaserTagList` `2933154f` | not done: the list's record was not read to the end (which number is which column), and no terminal of these kinds was looked for |
| Vehicles | FUN_100f3e80, `Views/MechSelect.xml`, `esc_vehicle` | kind 7, the control center's vehicle button (Shift+V, `OpenVehicleWindowMessage`) | a button for each finished global research goal whose item is a vehicle; `SpawnMech` `464d000a` with the perk | **ported** the same evening, `vehicles.rs` (below) |
| Tower build menu | `TowerBuildMenuView_c` FUN_1005cb45, made in code, `esc_towerbuildmenu` | the map control's Build and War buttons (FUN_100ec95f, FUN_100ec9f3) | an item dropped in; a check (CharacterAction 0xaf by the reader's reading) and `BuildTower`, whose message is in no decompile at hand; the answer's texts are Gamecode FUN_10129b8f's | not done: the exchange is unknown |
| Battlestation | `BattlestationView_c` FUN_100f8967, made in code from four pictures and the points' places | the server's `ArriveAtBs` `540e3b27`; closed by `BattleOver` `4b062919` | `SendScore` `44483b3a`, the points' places | not done: needs a battle's own state |

**The signup terminal as ported** (`signup.rs`, `mao_net::n3::queue_action` / `queue_place`): the window from its
view file, the drop-down's two sides (texts 8 and 9 of category 2005) with the character's stat 0x29c chosen; the
question to the server as it comes up; the place shown, and "Sign Up" and the drop-down dead while the place is not
0 (FUN_100f47ae); "Sign Up" refused with a line when no side is chosen or the character's stat 0x29e is below 1
(FUN_100f4595), else the side set and the queue joined; "Leave Queue"; "Close". Each queue action is
`FUN_10072a88(own identity, own identity, 0, the action, (0, side), "")`: a CharacterAction whose target is the
character itself. The window's three lines are English in the program and are in this client's words here. Not
read: what a change of the drop-down does by itself. Looked at offscreen (`MAO_UI_PREVIEW=signup`,
`MAO_SIGNUP_PLACE=<n>`). **A guess in all of it: that a server answers these as the decompile reads.**
Tests: `signing_up_needs_a_side_and_a_good_name`, `the_queues_actions_and_its_word`.

### The vehicles window (OTH-010, the second of its windows)

The user, after the survey: "Yes, vehicles". Read here in the original (GUI.dll's mech select, FUN_100f3e80; the
two handlers the export lacks by disassembly; Gamecode's `N3Msg_ActivateMech`):

- **What opens it**: the control center's vehicle button and its key (`OpenVehicleWindowMessage`, Shift+V) turn
  the module on, and a used terminal of window kind 7 does for a character whose stat 389 has bit 0x20. Turned
  on, the module (FUN_1002ef9c) makes the window only when more than 20 seconds have gone since a vehicle was
  ordered (`mech_ordered`), `N3Msg_CanUseMech` holds (outside, and a playfield that allows vehicles; each says its
  line otherwise) and the character's stat 0x213 has not bit 0.
- **The list** (FUN_100f3b73): a button for each finished global research goal
  (`N3Msg_GetCompletedGlobalResearchGoals`) whose item's stat 177 has 0x200, labelled with the goal's name, with
  the item's icon as its picture, alive when the character meets the goal's criteria, all one width. The game's
  data has six such goals: the window on the research preview's data (every captured global goal finished) lists
  Personal Mechanized Vehicle, Anti-Personnel Turret, Anti-Vehicular Battery, Mechanized Scout Vehicle, Mechanized
  Assault Vehicle and PMV Obsidian Edition.
- **A click on one** (LAB_100f3a51): chosen (the button's name is the goal's number), `VehicleInfo` shows
  `N3Msg_GetPerkDescription` of it, "Order Vehicle" comes alive. "Leave vehicle" stands in its place when the
  character's stat 0x296 is not 0 as the window is made.
- **"Order Vehicle"** (LAB_100f3931): `N3Msg_ActivateMech` of the one chosen, and the server's time kept as
  `mech_ordered`. **"Leave vehicle"** (FUN_100f3a36): `N3Msg_ActivateMech(0)` and the window's close.
- **`N3Msg_ActivateMech`** (Gamecode 0x10027250, FUN_10079c50): while a lock on 0x296 runs nothing is sent and the
  wait is said (FUN_10064847, the line that names nothing). A vehicle then needs the character outside
  (`Feedback_MustBeOutside`), not falling (`Feedback_YouCantDoThisWhileFalling`), in a playfield that allows
  vehicles (`Feedback_MechCantBeOrdered`), not in resurrection shock (stat 0xf7,
  `Feedback_NotWhileInResurrectionShock`), and one whose record's flags have 0x100 a battlestation or the
  organisation's own tower area (`Feedback_OnlyInOwnLCArea`). Then `SpawnMech` (`464d000a`: the header's byte and
  the goal's number, FUN_10079d57 and its reader FUN_10079c1d) is sent; 0 leaves the vehicle.

Here (`vehicles.rs`; `mao_net::n3::spawn_mech`; `PerkRecord::more_flags`): the window from its view file with the
list from `research::Model::finished_global`, the description `Model::description`, the criteria
`perks::meets_perk`; Shift+V (`open_vehicle_window`, which was bound to nothing) and terminals of kind 7. Asked
before an order: the lock and the resurrection shock, and before the window is made the 20 seconds and stat
0x213. **Left to the server**: whether the character is outside, falling, where vehicles are allowed, in its own
tower area; this client cannot tell those, so it sends the order and the server answers. Not there: the buttons'
pictures, the control center's own vehicle button. **A guess in all of it: that a server takes `SpawnMech` as the
decompile reads it**; none was ever captured.
Looked at offscreen (`MAO_UI_PREVIEW=vehicles`). Tests: `an_order_waits_for_its_lock_and_for_the_shock_to_pass`,
`the_window_is_not_made_right_after_an_order`, `the_queues_actions_and_its_word` (the message's bytes).

### The character list's figure (LCH-037)

The entry said three things were missing: the picked character turned by the mouse, the hall's meshes looping their
animations, the hall's nano effect running. Read in the original, two of the three are not there:

- **No turning.** `CharSelectWindow_c` (GUI.dll 0x1000efb1) takes no mouse on the figure, its key handler
  (0x1000deb7) has Play, Quit and the list's up and down, and `CharacterViewer_c::SetRot` (0x10005742) is called by
  nothing. (Noted once before as "looked at and left"; the entry's line was not corrected then.)
- **No animation of the hall.** The login's world, `LoginWorld_c` (0x100160c5), adds its five meshes and only shows
  and hides them; nothing steps their time. Looping the meshes and stepping them every frame is the character
  creation's module (`CharCreateModule_t::SlotMeshReady` 0x1011b7e0, `FrameProcess` 0x1011c355), where the "nano
  effect" is one of the meshes, `charactercreation_nanoeffect`, drawn blended. That is `ccscene.rs`'s gap ("the
  meshes' own animations"), not the login hall's.
- **The figure is animated**, and that was missing. The viewer (`CharacterViewer_c` 0x10005a36) makes a
  `CCCharacter_t` of medium build that plays `idle-stand_01_01`, and every frame runs
  `CCCharacter_t::RunFunction` (0x1011ac64): when the animation has ended, `rand() % 0x73` below 0x17 plays that
  entry of a table of 23 social animations (GUI.dll 0x101c3fa0: angry, applause, blowkiss, bow, bulge, curt,
  fishsize, giggle, greet, italian, lookout, rocky, salute, scared, scratch, strong1 to strong3, surprised,
  surrender, thinker, thumbs, wave), anything else the idle again: a fidget about one time in five. Each is played
  once (`PlayAnim` 0x1011ab3e: priority -2, one loop). A new pick starts afresh. The same function sets the figure
  down by its build: 0.02 for the viewer's medium build of breeds 1 and 3.

Here (`net::ListFigure`, `net::fidget`, `launcher.rs`, `hall::viewer_drop`): the hall's character is posed again 24
times a second from its own animation stack; the idle loops under the fidgets (the world's way with a standing
character) and the choice is made each time the idle's length or a fidget's has passed. Seen in my own test
launcher's window (`win_shot.py figure play 7,9,12,16,20 MAO_LAUNCHER_DEMO=1`): the figure differs between the
pictures. Test: `the_list_figure_fidgets_one_time_in_five`.

### The options that were left (SET-030)

The four names an earlier reader gave figures for "in no decompile at hand" are read by N3.dll, which has no
decompile here: found now by their names in the binaries and disassembled (`DLL=N3.dll xdis.py`). `ViewDistance`
and the `FadeCharacter*` four are preferences of the kind `IndependentPrefs_t` keeps; the others are the option
panel's own values.

- **`ZoomTo1stPerson`** (on at first; N3.dll 0x100200ef, 0x10020866, 0x1002118c, 0x10022841): it gates both ways.
  On: zooming in past the nearest distance, 0.8, goes into first person, and zooming out of first person leaves
  it. Off: the camera stops at 0.8, and first person is not left by zooming. Here: `camera::Inputs::hold_third`.
- **`MouseWheel`** (0 at first): the wheel over the world, when no window is under the pointer (GUI.dll
  `WindowController_c::HandleMouseWheel`). 0: the camera connects its zoom to it (N3.dll 0x100215c8). 1: the
  control center's side bar scrolls by it, 40 a step (GUI.dll FUN_10048822). 2: nothing. Here: 0 and 2; **1 is not
  done** (it does nothing, as 2).
- **`ShowMyCharacter`** (off at first; N3.dll 0x10019927 in `n3VisualDynel_t::Run`): the player's own body is not
  drawn in first person unless it is on. Here it was always drawn; now by the option.
- **`FadeCharacter`, `FadeCharacterStartDist`, `FadeCharacterEndDist`, `FadeCharacterEndAlpha`** (1, 1.5, 0.7,
  0.15; DisplaySystem.dll's `VisualCATMesh_t::RefreshAlpha` 0x10073b7f, as the drop-in ports it; only the client's
  own character has the flag for it, N3.dll 0x1001a0d9): the own body fades as the camera nears its head: solid from
  the start distance on, down to one less the "end alpha" at the end distance and nearer. Here: `viewer::own_fade`,
  drawn as the own body's alpha (the renderer draws a tinted body blended, and none at 0).
- **`ViewDistance`** (0.8 of 0 to 1; N3.dll 0x1001fc91): the camera's far plane is the value times 1000, not under
  the near plane's 0.2 and 50. Here the option's slider sets this client's own view distance to that (which the
  launcher's own setting can put further, to 4000).
- **`DisplayCharViewDistance`** (80 of 5 to 80; N3.dll 0x1001f964 and `n3VisualDynel_t::Run` 0x100199c0): other
  characters and creatures further than it from the player's own, in all three directions, are not drawn, on a
  ground playfield; a value outside 5 to 80 counts as 70. Here the option's slider sets this client's own
  character distance to that, and the distance is measured in all three directions now (it was level). **Kept as
  they were**: this client's own default, which draws everybody, and its cap inside rooms too (the original has
  another rule there, not read).

Not seen running: no offscreen picture has a camera near a body. Tests: the camera's zoom test (the switch
off), `the_own_body_fades_near_the_camera`.

### What was left of one's own death (DTH-014)

**The input stop.** Read exactly now (it was left on purpose before, for fear of a death that sticks; the user named
the entry again):

- The death state's constructor (`CharDie_t`, Gamecode FUN_1007b803) stops the character's moving and fighting and,
  for the player's own, tells the interface to take no input (FUN_1003eb6f -> the notifier's kind 9 ->
  `FlowControlModule_t::DisableUserInputMessage`, GUI.dll 0x100286e2): `InputConfig_t`'s flag is set, the AFK timer
  stops and the pointer is switched off.
- With the flag, `InputConfig_t::FrameProcess` (0x1001b253) drops the frame's whole input: keys, mouse buttons,
  the wheel, typed characters, clicks on windows. (Two key chords pass, Alt and Shift+Ctrl+Alt with keys that
  were not identified.)
- Nothing ends it but the character being in play again after the server has moved it (`AliveMessage`) or a
  resurrection (FUN_1003ec1f). **There is no timer.** The "eight seconds" are the original's capture: 7.8 s from
  the killing blow to the client's own `0x98`, with nothing sent in between, then the server's teleport.
- No window is closed and nothing is shown for it; the death's own effect (3000) and the health set to 1 once the
  death has played out were known.

Here (`net::INPUT_STOPPED`, `viewer.rs` `window_event`): while the own character is dead every key, mouse button,
wheel step and pointer move is dropped before the interface or the world sees it, the pointer is hidden, what was
held is let go, and it ends when the character is alive again (a playfield message, or health above the dead
one's: the two places that already ended the death). **If a death ever sticks, so does this**: the window's own
close button still works. Not ported: the AFK timer stopped meanwhile.

**The resurrection shock** (stat 247):
- Set to 5000 when a character of level 15 or more is brought back where it lies, with its line (FUN_1003ec1f ->
  FUN_1006411f). Counted down by the client itself: 27 every two seconds (the character's frame function
  FUN_1005b55d on the game clock's ping), some six minutes, and at 0 the line `Feedback_YouFeelOkAgain`.
- Meanwhile every shown skill (stats 100 to 168) and ability (16 to 21) is less `shock * base / 10000`, cut to a
  whole number, `base` being its value before the modifiers (FUN_10064b51, through which the shown reads go): half
  at the start. And no vehicle is ordered (already in `vehicles.rs`).

Here: `derived::SHOCK`, `shock_penalty` (in `shown` and `shown_one`, so the windows and the requirement checks
have it), `shock_step`, `World::tick_shock`. The resurrection message is in no capture (the ordinary return after
a death is a teleport), so the shock will only ever come from another player's resurrection.
Tests: `the_shock_takes_its_share_of_skills_and_abilities`.

### The last three Notum Wars windows (OTH-010)

Two readers read them to the end; the byte orders and the paths below were checked here in the decompile. **Still
nothing of it is in a capture**: all of it is the decompile's word, and each message that comes for these windows
goes to the log whole (`notum: ...`). The network side is `mao_net::notum` (its seven message numbers are their
class names through `MapToKey`, tested) and `n3::character_action`, a `CharacterAction` with a whole word for the
action and two identities.

**The laser target and the airstrike** (`laser.rs`; GUI.dll `LaserTarget_c`, FUN_100f2e32): opened by a used
terminal of window kind 4 (the orbital attack's) or 5 (the airstrike's) for a character whose stat 389 has bit
0x20. `Views/LaserTarget.xml`, 500 by 420: a list of the tagged players (name, area, level range) from the
server's `LaserTagList` (`2933154f`: a count, then per tag three numbers, the area's and the name's text, four
numbers), asked for as the window comes up, by Refresh and every 30 seconds (`CharacterAction` 0x100). The attack
button sends the orbital attack (0x102) or the airstrike (0x103) on the selected tag `(0xc73d, its number)` and
does nothing with none selected. No terminal of these kinds was found in the game's data, so nothing known opens
it. Not ported: the columns' widths kept.

**The battlestation's view** (`battlestation.rs`; GUI.dll `BattlestationView_c`, FUN_100f8967): the server's
`ArriveAtBs` (`540e3b27`: the points' places) turns it on and closes the signup terminal; `BattleOver`
(`4b062919`: the character's points and how it ended) turns it off. A square a tenth of the screen's width at the
top's middle: the picture `GFX_GUI_BS_MAIN`, a star for each point at its place (x from 292 to 1696, z from 1653
down to 442) in its holder's colour once a `SendScore` (`44483b3a`: the two scores, each point's side) names it,
the green dot at the character's own place, the clan's score at the left in red and Omni-Tek's at the right in
light blue. As it comes up it asks for the points (`CharacterAction` 0x104). The two boxes: `InfromPlayer`
(`3301337a`) asks whether to go to a battle, 30 seconds, Yes answers `AcceptBSInvite` (`166a435e`) with the
invitation's identity; the battle's end tells the outcome and the points, and its button sends `LeaveBattle`
(`3f3a1914`). Not ported: `BattleOver`'s release of two of the character's state flags and its setting of stat
27 to 40000 (not understood), the view's place kept, and whether the end's box leaves the battle by itself after
its 30 seconds (here it does not).

**The tower build menu** (`towerbuild.rs`; GUI.dll `TowerBuildMenuView_c`, FUN_1005cb45): the map control has its
"Build" and "War" buttons now (dead without Notum Wars, as "Info"), each opening the menu empty and closing it
when pressed again. **Using a tower item opens the menu instead of using it**: the interface's `N3Msg_UseItem`
(Interfaces.dll 0x1000a73f, disassembled) does that for an item whose template's stat 388 is not 0 (1 a
controller, else a tower) and for a shield disabler (its use action has a criterion of operator 0x53); here the
inventory's right click does, and an inventory item let go on the menu. The item is checked at once: its
template and quality go to the server (`CharacterAction` 0xaf), unless the lock on Computer Literacy runs (the wait
is said). The answer (`CharacterAction` 0xb0, its second parameter the marks of what failed) fills the
checklist: 13 lines for a controller, 15 for a tower, 6 for a disabler, each with its pass or fail picture and a
link to its help page (the game's own `BuildMenu_*.html` and `WarMenu_*.html`). With nothing failed "Build" (or
"Disable shield") lives, until the character moves; it uses the item the ordinary way and the menu closes.
Not ported: the item's mark in the inventory while it is in the menu, the answer for somebody else's item.
The use of such an item from a hotbar slot or by dropping it on the world still goes the ordinary way.

Looked at offscreen: `MAO_UI_PREVIEW=laser`, `=battlestation`, `=towerbuild` (with `MAO_CAPTURE=<net.log>` for an
item and `MAO_TOWER_FLAGS=<hex>` for the answer). Tests: `mao_net::notum`'s four, `laser`'s, `battlestation`'s
two, `towerbuild`'s three.

## The creation scenes that move, weapon specials, the shop by one click (2026-11-18)

The user: "LCH-034,CMB-015,NPC-014,INV-001". Three readers on a smaller model read the original, each told to
name the file of every rule; what a port rests on was checked here against the decompile and the captures
before it was written. None of it was seen in the live game.

### The special attacks' list is the original's (CMB-015: no longer a guess)

The source of a gun's attacks in the original's window is traced now, and the client's rule is the same one.
The window (GUI.dll `SpecialActionView_c`, FUN_100dc9b1) shows the list the game keeps, and two things write it:

- the server's `SpecialAttackWeapon` (`1d3c0f1c`; Gamecode FUN_10079de3 to FUN_1006a166), which in the captures
  always names the same three (Martial Arts' attack, Dimach, Brawl) and is **added**, never taken away;
- **the client itself, as a weapon is put on** (`WeaponItem_t`, FUN_1009e84b): one attack for each of seven bits
  of the weapon's stat 30 (0x800 Burst, 0x1000 Fling Shot, 0x2000 Full Auto, 0x4000 Aimed Shot, 0x8000 the bow's,
  0x20000 Sneak Attack, 0x40000 Fast Attack), Reload when its ammunition stat 26 is not -1, and Backstab beside
  Sneak Attack for a character the server has allowed it. Taking the weapon off (FUN_1009d39a) removes what the
  other hand's weapon does not give.

That is what `special_numbers` does. Two small things differed and are the original's now:

- the server's list **replaced** the client's copy of it; it is added to (`SPECIAL_ATTACK_WEAPON`'s handler);
- a weapon's rounds were taken from every attack's word; the original takes them only while the stored number is
  above -1 (FUN_1006ae3c, FUN_10068869: `net::rounds_after`), so a weapon without ammunition stays one.

Test: the specials test's `rounds_after` line.

### The character creation's scenes move (LCH-034)

The catalog's entry said more than the original does. Read to the end (GUI.dll `CharCreateModule_t::FrameProcess`
0x1011c355, `SlotMeshReady` 0x1011b7e0, `CCCharacter_t::RunFunction` 0x1011ac64, `BreedScene_t::SlotBreedButton`
0x10113baa, FUN_1011d692):

- **The meshes' animations run** (`hall::Creation::moving`, `moving_at`): every one of the six meshes is set to
  loop and is given each frame's time (at most half a second a frame), in every scene. Two of them have frames
  that move: the main mesh's two far suns and its star field, and the intro mesh's two star layers and the
  shuttle. `mao_formats::model::build_full_animated` gives each part its animation; the parts that move are kept
  out of the scene and drawn on the renderer's second layer at the director's time (`Director::total`), 24 times
  a second. The profession hall's four meshes have no animation that moves anything: **the profession figures do
  not animate in the original either** (they are plain meshes).
- **The chosen character plays its idle and its fidgets**, by the same function as the character list's figure
  (`net::ListFigure::of_build`). The seven breed figures stand still: the original runs that function for the
  chosen character alone.
- **The chosen character's size and place are the original's now** (`hall::creation_scale`,
  `creation_drop`, `on_connector_set_down`): the scene shows a short one at 0.95 and a tall one at 1.05 (the
  floats 0x101c404c and 0x101c4048; the client drew 0.9 and 1.1, the numbers the server is sent), and sets a
  Solitus or a Nanomage down from its connector by its sex and build (0.02, 0.05 or 0.07, a thin woman raised by
  0.01).
- **The heads offered follow the account's Shadowlands flag** (`charcreate::head_sets`, `head_left_out`):
  FUN_1011d692 tries more numbers of eight sets with `ExpansionFlags & 2` (Opifex women 32 to 43, Opifex men 30
  to 43, Solitus women 50 to 75, Solitus men 51 to 71, the Solitus men's Asian set 6 to 9, Nanomage men 30 to
  41, Nanomage women 30 to 43, Atrox 30 to 41) and never offers thirteen numbers of the Solitus women's first set
  nor fourteen of the Solitus men's, whatever the flag. The client offered the smaller sets to everybody and the
  left-out numbers too (where the game has the files). An account whose flags are not known gets the larger sets,
  as it is let to the Shadowlands' professions. **A head's place in the list changed with this** for a Solitus:
  `net <game> create ... <head n>` counts the new list.
- **There is no selection glow to port.** `CCCharacter_t::ShowSelectionGlow` (0x1011ae3c) has one caller, the
  breed button's slot, which turns it on and off again in the same call: it is never drawn. The camera's debug
  display is developer text behind a flag. Neither is in the client.

Tests: `hall`'s `the_creation_figure_is_sized_and_set_down_by_its_choices`, `charcreate`'s head counts in its
name test. Seen: the demo creation in a test window (`MAO_LAUNCHER_DEMO=1 MAO_LAUNCHER_CREATE=...`).

### A shop's rows act at one click (NPC-014)

The trade window's three lists are plain item lists that `TradeView_c` wires itself (GUI.dll FUN_100e07b9 the
stock, FUN_100e0942 the bought items, FUN_100e0b00 the sold ones), with the list flag that turns a click into a
signal instead of a pick-up (FUN_1004141b, checked in its assembly):

| Gesture on a row | Original | Was |
|---|---|---|
| left click | the row's action: a stock item goes in (one of it, `N3Msg_TradeAddItem`), a bought or sold one comes out (`N3Msg_TradeRemoveItem`) | marked the row |
| right click | the same | a stock item's page |
| Shift + left click | the item's page | nothing |
| two clicks | two actions (there is no double click) | the action, once |
| right button held 0.3 s | nothing, and nothing as it is let go | (a right click) |

`shop::row_click` is that table; the window has no marked row any more. The page of a stock row is the
original's `shopitemid://` page less the shop's price (the client's page has none), a bought row's is its stock
item's, a sold row's the inventory item's.

**An inventory item goes into the trade by Ctrl + right click** (FUN_100cae1b: the inventory's use handler adds
the item to the trade instead when a trade's window is up and Ctrl is held), not by Ctrl + double click as the
client had guessed; Ctrl + double click does nothing now. Dropping an item on a list is unchanged.

Not ported: the original's checks before an add that say why not without asking the server (no room in the
trade, cannot afford it: `CannotAffordThisItem` with the shortfall), and its refusal to take a sold item back
with a full inventory. The server answers those here.

Tests: `shop`'s `one_click_on_a_row_acts`, `invwindows`' click table.

### A shop stays open when the character walks away (INV-001)

The client's own rule (the trade declined and the window closed 8 m from where the shop was opened) is gone:
nothing in the original ends a vending machine's trade by distance. `TradeView_c` has no slot on the character's
place and its one timer is the credits field's; a `VendingMachine_t` (and a `PlayerShop_t`) runs the plain
dynel's frame function; and Gamecode's range test FUN_1005a1e7 (5 m between the two surfaces, an open door
between rooms) is asked for a **player's** trade as it starts, a use, a tradeskill combine, the mission
booth's own check every two seconds, the mission terminal's messages (`ACGQuestIIR_t`,
`QuestAlternativeIIR_t`, `CreateQuestIIR_t`), a loop over chests and the windows a spell opens. No capture
shows the server ending a shop either.

So the shop's window stays until Decline, Escape, the server's Complete or Decline, or another trade's start,
as in the original; what is tried from afar the server answers with "too far away" (the client's own earlier
log). Not found in the original: what zoning does to an open trade, and whether the window has a close box.

## The inventory mouse as the original has it (2026-11-18)

The user: "Inventory four" (INV-002, INV-003, INV-004, INV-016). The first three rest on the reading of
2026-11-13 (`docs/INVENTORY_ORIGINAL.md`, each statement marked verified there), looked at again in the
decompile where a port leans on it; the fourth had a reader on a smaller model, whose claims were checked
here. None of it was seen in the live game.

### An item is taken up by a click, not by a drag (INV-002)

The original's item lists take an item up when the left button comes **up on the cell it went down on**
(`ItemListViewBase_c` FUN_1004141b; `MultiListView_c::MouseUp`, GUI.dll 0x10133d04, signals for nothing else).
A press that moves to another cell and is let go there does nothing. The client lifted the item as soon as a
press moved three pixels and put it down where the button came up, a drag in one stroke.

That is gone from the inventory, the wear pages, bags, the bank, corpses and chests: `multilist::click_only`
makes a list tell of clicks only, and a press that wanders inside its cell still clicks, as the original's
does (the grid dropped such a click before). An item is carried the original's way alone now: click to take it
up, click again to put it down. The nano programs' list and the Action window keep their drag (the original's
lists of that kind pick up from a held press).

`loot::container_click` lost its drag case: the item pulled out of a corpse by a drag and taken when let go over
the inventory.

Tests: `gui::multilist`'s `grid_cells` (the click-only list), `invwindows`' and `loot`'s click tables.

### No use by double click: the original has none either (INV-003)

The entry had the client's rule as its own. It is the original's: its item lists have no double-click handler at
all. Two clicks are two clicks: the first takes the item up (FUN_100cc6e8), the second lets it go on its own
cell, which cancels the drag and leaves it where it was (FUN_100cc855: the held item's identity is the cell's).
An item is used by a right click. Nothing was changed for this entry; the Ctrl + double click the client had
beside it went with NPC-014.

### An item in hand and the right button: use it on the other, Shift combines (INV-004)

GUI.dll's drop handlers (FUN_100cc855 for the inventory's, a bag's and the bank's cells, FUN_100e220f for the
wear pages'), read again:

| With an item in hand, right click on | Original | Was |
|---|---|---|
| another item of the inventory | `UseItemOnItem(held, clicked)`: the use message's action 5 | a tradeskill combine |
| the same, **Shift held** | `N3Msg_TradeskillCombine`: `CharacterAction` 0x51 (both must be of the inventory) | (the same combine) |
| a worn item | `UseItemOnItem(held, worn)`, Shift or not | nothing |
| an item in a bag, the bank, a corpse | `UseItemOnItem`; with Shift the drag just ends | nothing |
| the item itself | nothing is sent (Gamecode 0x1002695c, 0x10028707 test it) | nothing |

`invwindows::held_right_click` is that table. **A tradeskill combine is Shift + right click now**; a plain right
click sends the other message, which the server may or may not make a combine of. The hand is empty after
either. An item is named in these messages by its page (`mao_net::inventory::place_identity`: weapons 0x65,
clothes 0x66, implants 0x67, social 0x73, the backpack 0x68, the place counted through all of them, as the
server's own messages name a worn place), a bag's item as `(0x6b, handle and place)`, the bank's as
`(0x69, place)`.

Not ported: the same right click with an item taken out of a **bag or the bank** in hand (that item rides on
the container's window here, which looks at the left button only). On a worn target both of the original's
handlers are connected and may each send the message (not followed to the end); one is sent here.

Tests: `invwindows`' `a_right_click_with_an_item_in_hand`.

### An item let go over the world: a character, an object, the ground (INV-016)

GUI.dll's FUN_100cbcb5 by what is under the pointer (`dropitem.rs`, `viewer.rs`):

- **A character** (a player or an NPC): `N3Msg_UseItemOnCharacter`, the use message's action 0x20
  (`n3::use_item_on_character`; the original's capture of 2026-10-30 has one, and the test holds its shape). The
  client sent action 5, the one for things.
- **Any other object**: `UseItemOnItem`, action 5, as before. The identity type 0x9c52 takes nothing.
- **The ground**: the original asks `N3Msg_IsDropPossible` (Gamecode FUN_10048dbd) and shows a box by the answer.
  Ported in its order (`dropitem::reason`): further than 5 from the character, the "cannot be dropped there"
  box; **an item that is no fixture, the box that says so and offers to delete it instead (Delete, Cancel)**;
  a unique fixture and a no-drop one, their boxes; else the question whether to drop it (not asked with the
  preference `ShowDropItemDialog` off), whose Yes sends the drop.
  A fixture is an item whose `Can` has bit 0 and whose flags have 0x1000 (FUN_10080ad7): 526 of the game's
  120820 item templates, the furniture. **So an ordinary item let go on the ground brings the delete offer,
  as in the original**, where nothing happened here.

The ground under the pointer is found by walking the line from the eye until it goes under the terrain
(`viewer::ground_on_line`). Guesses and gaps, plainly:

- **The drop's message, `DropTemplateIIR_t` (`3a243f41`), was never captured.** Its fields are the writer's
  (FUN_10073244: the item, then x, height, z); the zero byte before them is what the other messages of the kind
  have. It goes only for a fixture, after the question, and is said in the log.
- The distance is taken from the character's place; the original takes it from the character's sphere.
- Inside (no terrain) nothing is met and nothing happens.
- Not ported: the character's state test, the placing of a fixture in a building
  (`CanAddAnotherFixture`), a worn item's own tests (a worn fixture gets the original's later refusal,
  `Feedback_MoveItemToInventoryBeforeDropping`), and the half second for which the original keeps the last
  thing pointed at before the ground counts.
- `docs/INVENTORY_ORIGINAL.md` had Delete and Cancel on the no-drop box; they are on the no-fixture one.

Looked at offscreen: `MAO_DROP_BOX=fixture` and `=drop` put the two boxes up. Tests: `dropitem`'s three (one
counts the game's items), `mao_net::n3`'s two.

## What the mouse points at, and what its clicks do in the world (2026-11-18)

The user: "Targeting three" (TGT-001, TGT-003, TGT-017). Two readers on a smaller model read N3.dll and GUI.dll;
what the port rests on was checked here in the decompile and the disassembly. All three entries are one change
(`picking.rs`). None of it was seen in the live game.

### The line, the list and the shapes (TGT-001, TGT-003)

`picking.rs` is N3.dll's way, in its order:

1. **The line** (`n3Camera_t::SetMousePos` 0x10020571, FUN_1001fe47): from the eye through the mouse, **cut short at
   the first ground or surface it meets**. Outdoors that is the terrain and the playfield's collision surfaces, so
   a building's wall hides what stands behind it (the tile-map surface asks its cell surfaces first); indoors the
   rooms' surfaces. `Zones::line` and the terrain are used for it.
2. **The list** (`n3Playfield_t::RunCameraSelectionCollChecks` 0x1000d8d1, at most ten times a second): everything
   within 200 m of the eye that the line meets, nearest first.
3. **One's own character is on the list like anything else, by its distance** (FUN_10020a3c), and only in the first
   person is it left out. The client's rule of 2026-11-17 (the character last on the line) is gone: a click whose
   line passes through one's own body now takes the body first, and the next click what stands behind.
4. **The shapes** (`n3VisualDynel_t::FineCollisionCheck` 0x10019452). There were no cylinders in the original:
   - a character is met by **the box its posed vertices fill in the mesh's own axes** (the option
     `BoundingBoxTargeting`, on by default; randy31's `RCATMesh` line test): the box turns with the body and follows
     its animation (`Caster::body`, `picking::ray_box`);
   - a built-in object (a terminal, a door, a booth, a city's controller) is met by **the triangles of its mesh that
     face the line**, part after part, the first one crossed (`PickMesh::hit`, `VisualMesh_t::IsLineIntersecting` and
     randy31's `IsRayIntersecting`). A test on the game's data holds a city's controller and its key machine to it.

Not ported, plainly: the option's other setting (the body's triangles); a held weapon as a mesh of its own (here it
makes the body's box larger); a dynel's `IsVisible` (hidden characters, a statel's owner and team rules); door leaves
as they swing (met where they hang closed); and an object the server announces that is not in the playfield's table,
which keeps the old upright post of its kind's size.

### The clicks in the world (TGT-017)

`ActionViewMouseHandler_c` (GUI.dll FUN_1002cfac on a release, FUN_1002ce31 on a press), all on **what is under the
pointer**: the selected target when the line meets it, else the nearest, and what was last there for half a second
after the pointer has left it (`InputConfig_t::SetCurrentReference` 0x1001a191).

| Gesture | Original, and now here | Was |
|---|---|---|
| left click | the next on the line after the selected target | the same |
| second left click soon after | the same again, stepping on | used what the first took |
| **double click** | at the second press `N3Msg_DefaultActionOnDynel` (not on oneself; the option `DoubleclickAction`), and its release steps on as any click | as above |
| **Shift + left click** | the info page of what is under the pointer | nothing |
| **Ctrl + left click on a character** | it becomes the target and the fight goes to it (`N3Msg_SwitchTarget`) | a plain click |
| **right click** | uses what is under the pointer: **the target stays** | also made it the target |
| right button dragged with Shift | the character turns: only Ctrl keeps it to the camera (FUN_1002ccbe) | camera only |
| a release after the mouse travelled | no click from 0.02 of the mouse's counts over 1000 on (about 20 counts) | the same in screen heights |
| either button with the other held | a click all the same | no click |
| with an item in hand | no click | a click |

`N3Msg_SwitchTarget` (`combat::switch_attack`, `N3Msg_DefaultAttack(id, true)` 0x10028147): whoever fights stops; the
one already fought is only stopped; anyone else is checked as for the attack key and attacked, with `AttackIIR_t`'s
last byte 2. **No capture has that byte other than 0**: its place is the writer's (FUN_1007cb09), its effect untried.

Guesses and gaps: what counts as a double click comes from the window system in the original and was not found; here
it is Windows' default, half a second and four pixels. `N3Msg_DefaultActionOnDynel` on anything but a character takes
what its `Can` stat says can be got (bit 0) or used (bit 3), and a thing that can only be got is used here
(`N3Msg_GetItem`'s message is not ported); on a character it is the client's earlier approximation (an NPC's chat, a
player's trade, else a use). Shift + click on a thing with no template in the table shows its name only. The pointer's
shape over what it points at: see "The pointer over the world" (2026-11-21).

Tests: `picking`'s nine, `combat`'s attack key, `blobshadow`'s `a_body_keeps_its_box_in_its_own_axes`, the city's
controller in `main.rs`.

## A door opens at a touch, and what a worn item adds (2026-11-18)

The user: "ZON-001, INV-017". Two readers on a smaller model read the original; what the ports rest on was checked
here in the decompile and counted over the game's own data. None of it was seen in the live game.

### What a worn item adds is its wear list (INV-017)

The client looked through a template's bytes for anything shaped like a modify spell, in any list. Now the wear list is
read as the original runs it (`item::wear_spells`: event 14, `e_SpellList_Equip`; Gamecode FUN_10080fc3, the executor
FUN_100a5f4b):

- **Only the wear list counts.** The scan also took the spells of an item's other lists: about 2600 modify spells of
  the game's items are in the lists of being used, of the vicinity and of personal buffs, and a worn item with one
  showed that bonus for as long as it was worn.
- **A spell's criteria are judged by the evaluator** (`requirements`), on the stats as they show, and no longer on the
  level alone: wear spells that ask for a profession, a breed or a skill now count when they hold. In an item's tip,
  where only the level is known, such a spell is left out as before.
- **The quality is applied to the spell**, its value and its criteria's values between the low and high template's
  (FUN_100cbfc8, FUN_100cbe8b), the pair found by its type and stat, and the spell is judged after.
- `spell.rs` reads what it used to give up on: a type with no format of its own (the default format), 0xcf17's second
  criteria list and 0xcf20's expression. A list with one of these in it used to be dropped whole.

Counted over the game's 120842 templates: the wear lists' only spells that add to a shown stat are 0xcf35 (about
241000) and 0xcf14 (18). **The catalog's "other spell types" are not there**: no wear list has the percent types
0xcff5 or 0xcfc0, nor 0xcfb7. The other stat spells of wear lists change stored stats: 0xcf22 (283), 0xcf76 (155: the
Yalmahas' scale among them) and 0xcf93 (467). Those are not applied here. On the captured character's 19 worn items
the old scan and the wear list agree on every number; the 2062 perks are the same by both. An organization's benefits
keep their amounts in the list of being used and are read as before.

Tests: `what_is_worn_is_the_wear_list`, `a_wear_list_at_a_quality`, `the_spells_that_used_to_end_a_list`.

### A door opens at the character's touch and shuts as it leaves (ZON-001)

The client's 4 m and 3 m and its five seconds are gone. The original (N3's `n3Dynel_t::CheckBodyCollision` 0x1000483b,
`Door_t` FUN_1007fccc and its frame FUN_1007ff44):

- **The touch is two spheres meeting**, the character's body sphere and the door's, each from its mesh's packed word
  (`hazards::body_sphere`, as for a mine) and only for the client's own character (`net::door_touched`). A door's
  sphere is large: over the game's 5084 doors most are about 3 m round, centred about 2 m up.
- **A door that is not locked opens at once**, and nothing is told to the server.
- **It shuts on the first frame nothing touches it.** There is no timer. A door the server said stays open
  (`DoorStatusUpdate`'s third flag, `net::door_stays`) stays; every captured message has that flag at 0, and a door the
  server opens without it shuts again at once when nobody is at it (the client kept such a door open for a day).
- **What locks a door is bit 0x40 of its stat 0**, not of stat 259 as the client read it (`door_locked`). That is 684
  of the game's doors against the 13 that have stat 259's bit: those 684 stay shut until the server unlocks them or the
  character may pass (`door_may_pass`), as in the original. **To watch live: a door that used to open and no longer does.**
- **A shut door does not stop the character between rooms**: `Door_t`'s FUN_1007fc97 lets anyone through a door that is
  not locked (`interior::allowed` asked for an open one). Which rooms are drawn still goes by the open doors.

An open question, left out on purpose: FUN_10089f0a calls the touch only while a count of every other second divides by
the door's stat 437, which no door of the game has. Read as it stands a door would answer for its first two seconds
alone, so something supplies the stat that was not found. Not ported: the wait of a locked door that lets the
character through (stat 335, a second), the push back from a door that does not (dungeons), the touch's own sound
(class 0x6f) and the door's own use, which keeps its five seconds (ZON-004).

Tests: `a_door_is_held_open_by_the_touch`, `doors_are_numbered_from_their_ids`, `a_generated_missions_doors_are_usable`.

## Bare skin and the alpha of character textures (2026-11-18)

The user: "RND-020, RND-021". What the ports rest on was checked here in the decompile and counted over the game's own
data, and compared in offline pictures. None of it was seen in the live game.

### What a character texture's alpha does (RND-020)

Character parts were drawn whole, whatever their texture's alpha. The object's texture list holds one word for each texture,
and the stream reader turns it into an alpha mode: set is mode 5, clear is mode 0 (`operator>>(BinaryStream &, TextureData_t &)`,
GameData 0x1000c0b4; randy31's FUN_10040645 then applies the mode). **The word is not the mode itself**: read as a mode, 1 would
be an alpha test at 0x80, which cuts nearly the whole body out of the creatures that use it (`catmesh::set_texture_alpha`).

- **Clear (3441 of the game's 4248 listed textures):** randy31 turns alpha blending on for a texture with alpha; 156 of them
  have some. **Drawn whole here since 2026-11-19**: the original's own capture shots (`arete-20261030-1655`) show Arete's Garbage
  Fleas and Waste Collectors solid, though their skins (`fleamonster1.png`, `floatingpicker2.png`) are RGBA PNGs whose alpha is
  nearly 0 everywhere. Blended by it they were not seen at all (the user's report). What drops that alpha in the original (the
  bitmap, the surface's format or the device's states) was not found.
- **Set (807):** the alpha of a texture that has any is light, added to the lighting before the colour is multiplied in, and
  nothing is blended. These are the glowing eyes, lamps and chest lights of the creatures; 738 of them have such alpha.
- **A texture without alpha** is drawn whole either way. A material with an opacity under 255/256 is then blended by that
  opacity (`set_opacity`). The opacity reaches the renderer now; every part had 1.0.
- **What a character wears** takes mode 0.

The renderer finds out whether a texture has alpha when it decodes it (`blend_by_texture`). It counts a pixel under 255, where
the original asks the texture's format: two listed textures have an alpha channel that is full everywhere, and the original
would light those fully.

### Bare skin under the clothes (RND-021)

The catalog's "paints a skin colour" was wrong: **the original has no skin colour.** Pure green in a body texture is a key, and
the character's naked texture shows through it (`bareskin.rs`; DisplaySystem FUN_1007457f and FUN_10074393).

- **The key is exact:** 565 colour `0x07e0`, which is red and blue up to 7 and green from 252. The client's old tan took a much
  wider range of greens.
- **The naked texture** is `<part>_<breed><sex>[_<race>]_naked.png` (type 0xf695b, FUN_1006ff7f), for hands, body, feet, arms and
  legs. Only Solitus have a race (`caucation`, `african`, `asian`), and an Atrox is always `male`. All 55 files exist.
- **The race** is the head's: Gamecode's table FUN_100c3239 gives `head_solitus…_african` 2, `_asian` 3 and the others 1
  (`bareskin::race_of_head`). It lists most plain Solitus heads, and for the ones it leaves out the original keeps stat 89; here
  they count as race 1.
- **A body gets its skin only with a head mesh** (FUN_100585bf: stat 64), with the breed and sex the server sends. Remains and
  morphs have neither at hand, so this client reads them from the head's file name (`bareskin::body_of_head`): that is this
  client's own way to the same answer.

Pictures compared offline, before and after: 6 creatures (the alpha) and 6 humanoid NPCs, and all seven breed and sex pairs in
the appearance scene. `MAO_CREATURE_BODY=<breed>,<sex>` gives the `creature` preview a server's word.

Tests: `the_green_is_exact`, `the_naked_texture_shows_where_the_green_is`, `the_naked_textures_names`, `a_pair_has_one_number`,
`a_head_names_its_body`.

## The Action window, container windows, special attack range and the shortcut bars (2026-11-18)

The user: "WIN-022, WIN-023, WIN-113, CMB-022, WIN-001 with the option to switch to the smart hotbar". Four readers on a
smaller model read the original; what the ports rest on was checked here in the decompile and the game files. None of
it was seen in the live game.

### The Action window lists what the character has, and its pairs change (WIN-022, WIN-113)

The window showed the whole catalogue read from a running client: every special attack until the server listed them, and
every catalogue perk while the character had none. Now it lists what the original's list holds (`actions::listed_templates`,
`N3Msg_GetSpecialActionList`), in its order:

- **The character's nine actions**, as Gamecode's FUN_1004326f adds them when the character is made: Use, Pick-Up, Start
  Combat, Sit, Walk, Sneak, Suspended Animation, Crawl and Search.
- **The special attacks** next: the server's list (`SpecialAttackWeapon`), then what the weapons in the hands give, Reload
  with them.
- **Then the perk actions** the server listed (`CharacterAction` 0xb4).
- **Before the server's lists** there are the nine and nothing else.
- **An action listed twice shows once** (GUI.dll FUN_100dc05e finds a tile by its identity).

**Six of the nine come in pairs** (FUN_1004326f): start and end combat, sit and stand, walk and run, sneak and stop sneaking,
suspended animation and disrupt, crawl and stop crawling. Gamecode's FUN_1004300e is given the number just done or entered,
and the entry then shows the other one: the tile always shows what a click does next (`actions::shown_number`).

- **The callers swap a pair when its state changes**: a fight starts or stops, the character sits or stands, changes its pace,
  sneaks, camps or crawls.
- **A fresh character** shows Start Combat, Sit, Walk (it runs), Sneak, Suspended Animation and Crawl.
- **The client follows its own states** (`actions::Toggles`, set by the network thread every frame): fighting, sitting,
  walking, sneaking and the camp under way. Crawling is not kept here, so Crawl stays Crawl.
- **The other half is named and pictured by its template**: End Combat, Stand, Run, Stop Sneaking, Disrupt Suspended
  Animation.
- **The shortcut bars and the smart hotbars follow the swap too**, as the original's bar does (GlobalSignals +0xa8, GUI.dll
  FUN_100d9380). A click does what the tile shows; Sneak and Stop Sneaking are told apart by their numbers now, not the
  tile's name.

Not ported: the tiles' places kept per character (`SpecialActions.xml`).

Tests: `the_window_lists_what_the_character_has_in_the_original_order`, `a_pairs_tile_shows_what_a_click_does_next`. The
pictures `MAO_UI_PREVIEW=actions` and, with every state on, `MAO_ACTION_TOGGLES=1`.

### A nano cast is checked as the original checks it (CMB-039)

`N3Msg_CastNanoSpell` (Gamecode 0x1001b5d4) asks FUN_1004fbe4 before it sends the cast; every refusal tells its line
(category 0x42000018, shown as combat lines) and then `Feedback_UnableToExecuteOnThisTarget`. In its order:

- **The character** (`combat::cast_refusal`): in a vehicle (stat 0x296) `Feedback_NotInVehicle`; state flag 0x800
  `Feedback_ExecutingProgramsForbidden`; without state flag 2 (a fear takes it) `Feedback_ExecutingProgramsUnavailable`;
  moving `Feedback_UnableToExecuteProgramsMustStandStill`; swimming, crawling, falling each their line; sitting, sleeping or
  lounging `Feedback_YouMustBeStandingUpToExecuteNanoprogram`; then less nano than the cost
  `Feedback_YouNeedAtLeastRemainingNano`. (Dead: the client's own silent refusal, kept.)
- **The nano's requirements** for action 3 (FUN_100816f6), told as the evaluator tells them.
- **Whom it is cast on** (`World::nano_target_refusal`): oneself when nothing is selected and the nano is not hostile
  (flag 0x8000), or when it is for oneself only (`ncu::self_only`); else the selection. An item
  `Feedback_YouCantExecuteNanoprogramsOnItems`; no character `Feedback_UnableToExecuteNanoprogramTargetNotFound` (a
  terminal or door selected included); one in a vehicle `Feedback_YouCantUseOnVehicles`; a friendly nano at an NPC that is
  not one's pet `Feedback_MolecularStructurePreventsUse`.
- **Reach and sight**: the range is the nano's stat 287 with one's stat 381 share, at most 40 (FUN_1008658e), against the
  distance less both bodies cut to metres (`Feedback_TargetOutOfRangeForNanoprogram`); then the line of sight as a special
  attack's (`Feedback_TargetNotInLineOfSight`); then FUN_1004ed2f: the gap uncut (`Feedback_TargetIsOutsideRange`); a
  hostile nano asks the attack check and then says `Feedback_YouAreNotAllowedToExecuteNanoprogram`; a friendly one at a
  monster (side 3) `Feedback_YouAreNotAllowedToExecuteFriendlyOnEnemy`.

The stacking (NAN-041, `combat::stacking_refusal`): a nano of the same line (the same strain, stat 75, or one of the lines
in stats 546-550 shared) running on the target with a higher stacking order (stat 551) refuses the cast,
`Feedback_NCUErrorBetterProgramRunning`; a new nano with the flag 0x800000 refuses to replace one of its line it would
replace, `Feedback_NCUErrorNanoprogramCantReplaceOther`. Running nanos: one's own list, another's from its updates.

Not ported: a cast waiting on one of the same stat 0x195 (FUN_1004f3d9) and a program not ready (FUN_1004f0f9); the
caster's +0x80; casts still under way on the target; and the rules by fight mode (team, raid, battlestation, sides:
`Feedback_YouAreNotAllowedToExecuteFriendlyOnOppositeSide`), as the client keeps no fight mode.

### A special attack's range is checked before it is sent (CMB-022)

**Fear holds the specials back (2026-11-19, evening).** 0x100281fc's first check is the character's byte +0x21d: set, it
sends nothing and says nothing. Two spells of the server's spell lists set it on a player's character
(`spelllist::Control`, the object at the character's +0x1f0, FUN_10045a78):

- **Fear, 0xcf81** (FUN_100a86b7 → FUN_10045bdb): state 1. It also takes state flag 2 and gives 0x500 (on any character),
  and tells `Feedback_FearActivated` when the list's +0x28 identity is one's own character.
- **The spell 0xcff0** (FUN_100a8741 → FUN_10045ba5): state 5, on a player's character only.
- **Entering a state** leaves a higher one first (FUN_10045b64), so fear replaces state 5 and not the other way round.
  The list's end (or the spell's stat 0x42 of 1) leaves only the state the character is in (FUN_10045ab3, FUN_10045a8b).
- **A character made by its full update** takes the byte from the update's last byte (FUN_1007916d's +0x32f, which
  FUN_1007835c copies; `EntityInfo::refuses`). It is 0 in every captured update.

Not ported: the two spells also give a player's character an `NPCVehicle_t` (FUN_1005ac62(1)), so that the server moves
it; the client still moves it as before. States 2-4 (0xcf7e, mind control, daze) are only for NPCs.

**The rest the byte holds back (2026-11-19, late evening).** The same byte is asked by:

- **A nano cast** (`N3Msg_CastNanoSpell` 0x1001b5d4, before its own checks): nothing is cast and
  `Feedback_UnableToExecuteOnThisTarget` is told (its category 0x42000018 is shown here as a combat line). The cast tells
  the same line after every refusal of its check FUN_1004fbe4 too, after that check's own line (until 2026-11-19 the
  client told only the check's line; the check itself is the next section).
- **Stopping a fight** (`N3Msg_StopAttack` 0x100280e0, which the attack key, sitting down and /camp call): no `StopFight`
  is sent, so the fight goes on.
- **Starting an attack** (FUN_1006817d, the sender behind the attack key and Ctrl + click): no `Attack` is sent and
  `Feedback_PleaseWaitUntilPreviousAction` is told, as for an attack still on its way (`combat::Own::held`).

Not ported: the nano target checks read the caster's byte too (FUN_1004fbe4, FUN_1004ed2f), which a held character of
one's own never reaches; FUN_10051c98 (a speed, stat 0x126, for a held or NPC character) and FUN_10071519 (a vtable
function whose owner was not found).

`N3Msg_SecondarySpecialAttack` (Gamecode 0x100281fc) checks the reach after the weapon's bit (`World::special_refusal`):

- **Each weapon in a hand with the special's bit must reach the target** (the hand of place 8 first, then 6; FUN_10068c44 and
  FUN_10067f0a). One that does not says `Feedback_TargetIsOutsideSpecialAttackRange`. With no weapon left that may, the
  line `Feedback_SpecialAttackIsUnavailable` follows, so the two can come together, as the code reads (not seen live).
- **Then the special's own item must reach it.** That is the item the server lists the special with (`SpecialAttackWeapon`'s
  low template, kept by its stat: `World::special_items`; FUN_1006b14c makes an item of it at the character's level).
  Brawl's reaches 3 m, Dimach's 10 m and the plain attack's 2 m.
- **The reach** is the item's stat 287 and the character's stat 380 share of it in percent, cut to a whole metre and no more
  than 40 (FUN_1009abd5: `combat::special_range`).
- **The gap** is the distance between the two, centre to centre in three dimensions, less 1 m for the attacker's body and 1 m
  for a character target's. It must be less than the reach, not equal (`combat::special_reaches`).
- **The 1 m is read, not guessed.** It is the float at +0x30 of the object at a character's +0x1ec, which that object's
  constructor sets to 1.0 (FUN_1004496c); no store to it was found in Gamecode's code.

**The line of sight, last (2026-11-19).** After the range, 0x100281fc asks FUN_10058e4f and says
`Feedback_NoLineOfSight` when it fails, sending nothing. FUN_10058e4f takes the attacker's place and the target's, each
raised 1.6 (the float at 0x10158454), and asks `n3Playfield_t::LineOfSight` (N3.dll 0x1000d34c): the rooms
(`RoomCanSeeFromTo`), then the surface's `GetLineIntersection` from the one to the other and back. A target with no dynel
fails it. Its skip, when both dynels have byte +0x21c set outside a dungeon, never happens for one's own special: +0x21c is
the NPC byte (flag 0x10000000 coming on sets it, FUN_10058932, and FUN_1007803b gives that flag to every NPC; it was first
thought never set because that store is written relative to +0xe8), and the attacker is a player. The client asks the same test
the camera uses (`camera::playfield_line_of_sight`, moved out of the viewer's camera surface) from the network loop,
with the playfield it holds there: the zones' surfaces and tile-map ground, the props' meshes where there are no
surfaces, a dungeon's rooms and the rooms shown. A target the client has no place for is refused the same way.

Left:

- **The "queue of specials not yet swung"** the catalog named was not found. The check in that place (FUN_1002a027 on the
  weapon holder's +0x84) is of something the reload sets, and it refuses without a word.
- **Fear and its sibling state** (0x100281fc's first test, the attacker's byte +0x21d): it is set while the character is
  in state 1 or 5 of FUN_10045b64's state machine, which the spells 0xcf81 (fear, `Feedback_FearActivated`) and 0xcff0
  enter through FUN_100a86b7 and FUN_100a8741. A special is then refused without a word. Not ported: states 2 to 4 come
  from other spells and decide whether 1 and 5 may begin, and the client keeps none of them.

Tests: `a_special_reaches_by_its_items_range`, `a_special_needs_a_weapon_that_lets_it`, `a_special_needs_a_line_of_sight`.

### One window per container (WIN-023)

Corpses, chests, the bank, the overflow and the organization's contract shared one window named "loot", so a second corpse
replaced the first. GUI.dll's `InventoryGUIModule_c::SlotContainerOpened` (0x100c7bef) keeps one view per container
identity:

- **The same container opened again keeps its window**; another gets one of its own. There is no limit.
- **The names are the original's** (`InventoryView_c`, FUN_100ccefe): one's own backpack `Container_%ux%u`, the bank
  `bank_window`, and any other (a corpse, a chest, the overflow, the organization's) `TempContainer_%u`.
- **A `TempContainer` window's number** is the lowest that no other container window up has (FUN_100cb7f9), and it is given
  back when the window goes (FUN_100cade8). A window's place is kept by its name, as the original keeps it per number.
- **The client keeps every open container** (`World::loots`), each window by its container in `Ui::bag_windows` beside the
  backpacks. A window shut comes up again when its container is opened anew.
- **Escape** closes a corpse's window by `esc_corpses` and a chest's by `esc_chests`, each by its own name.
- **This client's own choices:** a new `TempContainer` window starts 30 px down and right per number, as the original's
  first place was not found. The bank's window is `bank_window` now, so the place the old "loot" window had is not carried
  over.

Not ported:

- the window's X sending `N3Msg_UseItem(container, false)` (its bytes were not traced);
- the list mode kept per number (`TempContainer_<n>.xml`: here one for all, as before);
- a chest closing when the character walks away (Gamecode FUN_1003f8fc).

The "Take all" button is this client's own: GUI.dll has none.

Test: `a_backpack_has_its_own_window_and_name`. The network tests now keep the containers as a list.

### The original's shortcut bars (WIN-001)

The hotbar was this client's own design. The original's shortcut bars are the default now (`shortcutbar.rs`, `ui::shortcut_layer`).
The smart hotbars stay as a setting: "Smart hotbars" in the client's settings, `smart_hotbar = on`. From GUI.dll's
`ShortcutBarWindow_c` (FUN_100d9e5f) and `ShortcutBarView_c` (FUN_100d9588):

- **Up to ten bars** (`NumHotbars`, 1 to 10, one by default; `hotbars =` in the settings). Each is a row of ten slots over ten
  pages: slot `s` of page `p` is cell `s + 10 p` (FUN_100d8665).
- **One bar is the primary**, chosen by its round toggle (FUN_100d8622).
  - **1 to 0** press the primary bar's slots (`SHORTUCT_BAR_ACTIVE_n`).
  - **Shift+1 to 0** pick its page (`SHORTUCT_BAR_ROW_n`).
  - **Alt+1 to 0** make bar n the primary (`SHORTUCT_BAR_SELECT_n`).
  - These are the game's own defaults in `CharPrefs.xml`, matched here by the providers' hashes.
- **The look:**
  - the toggle (`GFX_GUI_RADIOBUTTON_CHECKED`/`_UNCHECKED`, 11 by 11);
  - the control strip (`GFX_GUI_TOOLBAR_CONTROL_H`, 32 by 38; upright `_V`) with its page buttons
    (`GFX_GUI_TOOLBAR_BUTTON_UP`/`_DOWN`, `_LEFT`/`_RIGHT`, lit when the pointer is on them) and the page number in yellow,
    `(page + 1) % 10`;
  - ten cells 31 wide, 36 apart and 2 in from the edge, on `GFX_GUI_MULTILISTVIEW_SLOT_32_CLOSED`. Ten of them come to the
    402 px the original's saved bars are wide.
- **Moving and turning:** the strip is dragged to move the bar and double-clicked to turn it upright (FUN_100dbc76,
  FUN_100dbe17), neither while `LockHotbars` is set (`lock_hotbars`). The wheel over a bar turns its pages, up to the one
  before (FUN_100dbef9).
- **Putting a shortcut in:** a shortcut dropped on a cell takes it, and the one it held goes into the hand (FUN_100d89bd).
  One dragged out leaves its cell and is gone unless it is let go over a cell (FUN_100d8045). A click does it (FUN_100d833f).
  A right click does nothing, as in the original.
- **A shortcut with time left** shows its icon full for the share gone and faded for the rest, with the time over it in
  yellow: `%2ds` up to a minute, then `%2dm` (`IconTimers`, `IconTimerText`). A macro is `GFX_GUI_ICON_MACRO` with its name
  in yellow.
- **The first run fills bar 1** (FUN_100d9e5f, while `IsFirstTime`): Start Combat, Walk, Sit, a new macro "Follow" that says
  `/follow`, and Suspended Animation in the last slot.
- **The bars are kept in `shortcutbars.ini`** beside the settings.

Guessed or this client's own:

- which page button goes which way (their functions are not exported): the upper one goes back, as the wheel turned up does;
- the bars' first places: stacked up from the bottom left, where the original's saved bars stand.

Not ported:

- **Reading the original's `ShortcutBar_<n>.xml`:** its items are kept by their place in the inventory, not their template.
- **Shift+click's item information.**
- **The red fade of a shortcut that cannot be used** (`GFX_GUI_ICONFADER32`).
- **Stack counts.**
- **A new nano program put on the primary bar** (`ShortcutNewNanos`).
- ~~The bars' close, which hides them all (`shortcutbar_window`)~~: the option panel's hide/show box does it since 2026-11-21.

Tests: `the_keys_go_to_the_primary_bars_shown_page`, `the_bars_round_trip_through_text`, `the_timer_says_seconds_then_minutes`.
The picture is `MAO_UI_PREVIEW=shortcutbar` (`MAO_HOTBARS=2`, `MAO_HOTBAR_UPRIGHT=1`).

## The last high entries: pets, stats, camera, fading, chat tabs, the map (2026-11-18)

The user: "NPC-005, CHT-001, WIN-024, WIN-114, WIN-151, WIN-152 and CAM-001". Four readers on a smaller model read
the original; what the ports rest on was checked here in the decompile, the captures and the game files. None of it
was seen in the live game.

### `/pet` as the original reads it (NPC-005)

`/pet` took its first word and sent the command to every pet. Gamecode's FUN_100542ad reads the line from its end
(`chat_commands::pet_line`):

- **The words.** The line is split into words; a word in double quotes may hold spaces.
- **The command.**
  - If the last word is a command that takes nothing, that is the command: follow, behind, survive, wait, guard, attack,
    terminate, free, report and heal.
  - Otherwise the last word is the argument of the word before it, which must be a command that takes one:
    - `cycle`: a word starting with `a` (value 1) or `w` (value 0);
    - `social`: one of the 70 socials of Gamecode's table, by its place from 1;
    - `rename`, `chat`, `script`: the word itself is sent as the text.
  - A line that fits neither form sends nothing and shows the usage.
- **The pets** are named by the words before the command (`World::pets_named`):
  - none named: every pet (the empty list), except for a rename, which goes to the selected target;
  - `all` first: every pet;
  - otherwise every pet whose name is one of them, without regard to case. Names that match no pet are left out, and
    when none matches nothing is sent.
- **With no pet at all**, the game's line `Feedback_YouHaveNoPet` is shown and nothing is sent (FUN_1003fe10).
- **The message** (`n3::pet_command_full`) now carries the value. The pet window's buttons also set the tower word when
  the pet is a service tower (`N3Msg_IsPetTower`, 0x10016d1e: `PetType` 15); it was always 0.

The capture's eleven typed `/pet attack` lines are the reference for the bytes. A per-pet command was never captured;
its layout comes from the writer (FUN_10076600) and the reader (FUN_100766ec), which agree. `/tower`, which the code
reads as the same with the tower word set, is left as it was: its own command list sends it elsewhere.

Tests: `lines_become_commands`, `a_pet_command_is_packed_as_the_writer_packs_it`, and the pet naming in
`a_pets_name_is_kept_and_given_back`.

### Stats: movement without the trickle, and which places count (WIN-024)

The catalog asked whether the client should show the server's numbers instead of summing them. It cannot: the server
sends the stored stats and the lists (inventory, perks, running programs, organization benefits), and the original's
client builds the modifier map itself, as `derived.rs` does; that matched the live original on three characters. Two
differences from the original's stat container were found and ported:

- **The movement reads mode 3, which has no trickle.** Gamecode's FUN_1006f2fc reads `RunSpeed`, the health and the
  greatest health in mode 3. That is FUN_10064d46: the stored value and the modifier map's part, less the shock, and not
  the attributes' trickle, which only the windows' mode 0 adds (`derived::mode3_one`, used by `net::moving_stats`).
  The client added the trickle since 2026-11-15, so a character with high attributes ran faster than the original lets it.
- **Every place under 0x30 counts** (FUN_10080fc3 runs a worn item's whole wear list there; from 0x30 on, the social tab,
  only the look's spells run). The client counted 1-15, 17-31 and 33-45 (`derived::worn`).
- **Max health is never under 1** with a modifier on it (AddModifier, FUN_10064a0e).

Not ported (found, no player is known to meet them): the percentage spells 0xcff5 (FUN_100a8e44, in no wear list); the
criteria of nano, perk and benefit spells beyond the level; the stats made of two others (0x20d-0x274); mode 4.

Test: `only_the_worn_places_count`.

### The mouse's camera as the original's (CAM-001)

The catalog's "WoW-style camera" was already gone: `viewer.rs` follows GUI.dll's `ActionViewMouseHandler_c` (0x1002d1ae),
and section 4d is corrected. What still differed:

- **How far the mouse turns.** It is `MouseTurnSensitivity` (1 to 20, 10 by default) over 1000 radians a count. GUI.dll's
  FUN_1002ccbe takes the counts over 1000 (InputConfig_t's FrameProcess, 0x1001b253) times the preference, read at every
  motion. The client turned a fixed 0.005, half the default.
- **The wheel's step.** A notch moves the camera `ZoomSpeed` / 10 units, 2 at the default 20 (N3.dll FUN_100200ef). The
  client moved 2.5.
- **The look's button.** The look is the last button pressed (FUN_1002ce31 sets the kind at each press), and it ends when
  either button is let go, the other still held or not (FUN_1002cc90). The client looked on while any button was down.
- **The turn keys step sideways** once a look has moved, whichever button it is (`N3Msg_MovementChanged`'s mouse-look flag,
  set by the first motion: `Input::looking`). The client did so only while the right button turned the character.

Not ported:
- the head's turn handed to the character when a first-person look ends (`N3Msg_EndCameraMouseLook`, 0x100195ff);
- the right drag's up and down going to the camera only by `RMBMouseLook1st` / `RMBMouseLook3rd`;
- the camera's default mode, which is `CAMERA_3RD_LOCK` in the game's `CharPrefs.xml` and `trail` here (the user's).

The middle button's camera reset is this client's own; the original resets with Numpad 5 only.

### Windows fade, the pin, and the "?" button (WIN-114)

- **Windows fade** (`chrome::fade_level`). When the pointer leaves a window (and no mouse button is held), the window
  fades to 0.3 over a second, starting two seconds later. When the pointer comes back it goes solid over 0.2 s.
  - Source: `WindowController_c::HandleMouseMoved` (0x1015867f) and `FadeWindows` (0x1015831b); `Window::FadeTo` and
    `UpdateFadeLevel` move the level linearly.
  - The message boxes do not fade (`FadeWindows` leaves a modal window alone).
  - The chat windows keep their own look (`chrome::keep_solid`): their fading is their own and was not read.
- **The pin is the fade switch, nothing else.**
  - GUI.dll's `WndBorder::GetPinButtonState` (0x10159181) is asked only by `Window::_CanFade` and the saving.
  - A pinned window stays solid. Setting or taking off the pin turns the window solid over 0.2 s.
  - The pins are kept in `pins.ini` (the original's `WindowPinButtonState`).
  - The frame menu gains "Autofade Window", checked while the window fades; choosing it turns the pin.
  - **Escape now closes a pinned window as well.** Until today the client's pin meant "left open by Escape", which the
    original has no rule for.
- **The "?" button** is on the nine windows the original gives a help page (`WndBorder::SetHelpFile`, 0x1015b139;
  `chrome::HELP_PAGES`): perks, pet, LFT, laser target, vehicle select, signup terminal, research, skills and teleport
  target. It opens the page from `cd_image/text/help/` in the info view (`GFX_GUI_WINDOW_QUESTIONMARK`, its tip
  `InfoAboutThisWindow`).

Left: docking windows as tabs of one frame (`DockTabbedWindow`, the `DockAreas` files). That is a window tree and saved
layouts of its own.

### Chat tabs are dragged (CHT-001)

A chat tab is moved as the original moves it (`ChatWindow::carry`).

- **Taking it up.** The right button pressed and let go on the tab takes it up. That is GUI.dll's `Tab` (FUN_10146ef3)
  beginning the drag on the second button (`TabView::SlotBeginDrag`, 0x101459dc). This is read from the decompile and
  has not been seen live.
- **Carrying it.** A picture of the tab follows the pointer, held where it was taken (`DragObject_c::StartDragging`).
- **Putting it down** with the left button:
  - on a tab of any chat window's strip, it goes before that tab;
  - on the strip past the tabs, it goes last (`ChatWindow_c` FUN_10098038, `TabView::SlotItemDropped`);
  - on itself, nothing moves;
  - anywhere else, it becomes a window of its own whose corner is there (FUN_100984c2). The only tab of a window cannot
    leave it that way.
- **What is left behind.** A window left without tabs is gone.
- **Escape** lets go of the tab. That is this client's own choice: how the original drops a drag otherwise was not read.

The tab menu's two entries for moving a tab stay as this client's own.

### The original's playfield map window (WIN-151)

GUI.dll's `PlayfieldMapView_c` (0x100ec21c), the "PF Map" window `map_window`, is the default map now
(`AutoMap::pf_window`). The client's own round corner map and its overlay are a setting ("This client's own round corner
map", `own_map = on`), off by default.

- **Opening it.** The control center's Map menu entry `map_window` opens it now, not the planet map. `WINDOW_MAP` is
  its key; the game's files give it none, so it has none here either.
- **Size and scale.** It starts 179 by 199 and can be sized. It is north up and never turned, with no zoom: the ground at
  one pixel a world unit over black (FUN_100ea040).
- **Following and dragging.** The character is in the middle every frame (FUN_10046700). Dragging moves the view half
  again as far as the mouse (FUN_100ea177); let go, it comes back to the character (FUN_100ea145).
- **What is drawn on it:**
  - the land-control layer and the towers, as on the client's map;
  - the characters within 500 as 8 by 8 squares by side, `MAPSQUARE_CLAN`, `_OMNI`, `_NEUTRAL`, a monster
    `_MONSTER`, a team member `_TEAMMEMBER` (`automap::PfKind`, `N3Msg_GetMapCharacters` 0x1001bfba);
  - the character's own `GFX_GUI_MAP_ARROW`, turned the way it faces.
- **Tip and menu.** The tip gives the pointer's world place. The right button's menu shows the land-control buttons
  under the map (`ShowButtons`).

Not done:
- the floors of a playfield of rooms (FUN_100474b7);
- the vending machines' and reclaim booths' squares;
- the selected target's and team members' facing arrows (the selected target is drawn as `MAPSQUARE_SELECTED`);
- the mission target's square;
- the map withheld on 31 playfields until the character has their map part (stats 471, 472, 585, 586; FUN_100581ec).

In the offline picture the ground is not made, so the window shows its markers on black.

Test: `a_character_is_drawn_by_its_side`. Picture: `MAO_UI_PREVIEW=map MAO_PF_MAP=1`.

## After the play test of 2026-11-18

From the user's play test with the dev build and its game log.

### A mission's item comes into the inventory

The user's play test on Arete Landing: "Plant a Bug" gave no recording device. The log showed the server's
`ContainerAddItem` from the overflow's place 0 into the inventory, with nothing at that place in the client. The Arete
capture of the original (2026-10-30, lines 28144-28145) shows what comes before it: a `SimpleItemFullUpdate` of a
`0xc73d` item owned by the character, with no place in the world, template 0x48379. The client took those messages only for
terminals standing in the world. Now an item owned by the character and standing nowhere waits in the overflow at its
first free place, and the move brings it in. Test: `a_mission_item_given_comes_into_the_inventory`, on the capture's bytes.

### The round map by default again

The user's word after the play test: "I want the square PF Map as an option and the circle PF Map as default". The round
corner map and its overlay are on by default again (`round_map = on`). The original's square playfield map window opens from
the control center's Map menu either way. The setting has a new name: the first one, `own_map`, had already been written as
off by the build that made the square window the default.

## Overnight of 2026-11-19

Unattended work while the user slept: entries of docs/DEVIATIONS.md, none of it tried live.

### Play on an inactive character asks first (LCH-035)

`CharSelectWindow_c::SlotLoginPressed` (0x1000dcf0): an active character (its list flag's bit 1) logs in; an inactive one
opens `Views/CharacterActivateWindow.xml` (`CharActivateWindow_c` 0x1000d688), its text 700 `ActivateCharacterWarning`
fed the name, the slots left and the slots in all, and Yes logs in, Cancel closes; with no slot free (`slots <= active`)
only text 10000 `CharacterSlotsExhausted` comes, with OK, and nothing is sent. Slots not known (-1) ask as with one free
(the client's own rule). Test: `play_asks_for_an_inactive_character`. The limbo window (system message 0x4e, a countdown
before login) is not ported: no capture of the message, and the client closes its connection after the list.

### The launcher's version check and licence (LCH-036)

Anarchy.exe reads the game folder's version (`version.id` without its `_EP1`/`_EP2` mark) and each dimension's with
`%u.%u.%u.%u` (0x412511-0x412535) and marks the dimension (0x4126d6-0x412761): the first number that differs, the game's
lower, needs a patch; the game's higher is an old server, unless the dimension's version has more than two numbers and
the game's just two (then a patch); versions of another count of numbers need a patch (an empty one too: sscanf's EOF).
The launcher's list says so after the name (in the client's own words), and PLAY takes the original's PATCH pictures
(bitmaps 183-185) while the chosen dimension needs a patch. PATCH does what the original's does (0x40c13d): it opens the
game's `AnarchyPatcher.exe` (ShellExecuteEx "open", 0x407d37) in the game's folder with the dimension's version and the
game's expansion mark (`18.8.63_EP1`), and the launcher closes as Cancel closes it (mfc100 ordinal 8137, EndDialog(2)).
The original closes whether the patcher started or not (a failure is only written down, in `Anarchy.err`); since
2026-11-21 the client also tells why in a box once the launcher is gone (see "The launcher's PATCH crashed it" below). The
client's own fallback dimension (no list kept or downloaded) takes the game's version, so it never asks for a patch.
An older server is refused with a line, as the original's click check (0x40b60f) refuses it. The licence
(`anarchy.eula`, dialog 152) is shown until it is accepted; I decline ends the launcher. Where the original remembers the
acceptance was not found, so the client keeps it as `eula = accepted` in `launcher.ini` (its own rule). Tests:
`versions_compare_by_their_numbers`, `the_licence_is_remembered`,
`the_patcher_is_given_the_dimensions_version_and_the_expansion`.

### No guessed water sheet (RND-025)

The original's water is the playfield's own water triangles and nothing else. The client used to lay a flat translucent sheet 1.5 above any ground with a big flat low floor; it now draws water only from the triangles (and from the rooms' water in interiors). Playfields that lost the sheet: Omni-1 Trade 710, Garden_of_unredeemed 4011, Pandemonium Tower Gigantomachy 4900, Uturn Canyon 6550, Uturn Forest 6551, Foundry of Nightmares 8040/8045/8046/8050. `MAO_WATER=level` still lays one for tests.

### Player body table (RND-028)

FUN_10057a2e picks the body and head by breed and sex: Solitus, Opifex and Nanomage male and female, Atrox neuter, each its `<breed>_<sex>.cir` body and `head_<...>01.abiff` head; breed 5 neuter the `unfinished-breed.cir` body with no head; any other pair nothing (the stats Mesh 12 and HeadMesh 64 are only set while still 0). The client follows the same table.

### Statel effects to the view distance (RND-124)

A statel's own effects (a campfire's fire, chimney smoke, a lamp's halo) run with the statel in the original (_EffectHandler_t::EnableEffect 0x100ce0dd); no distance limit of their own was found. The client used to make them only within 150 units of the eye; it now makes them out to the view distance, where the fog has hidden everything. Their start times are still staggered per place, the client's own.

### Effects on the body's bones (RND-139)

An effect's attach point 1000-1018 is a bone of the posed body in the original (FUN_10105e6d, table at 0x102c63a8: pelvis, spine to spine3, neck, head, upper arms, forearms, thighs, calves, feet, hands, each `Bip01 <name>_ac`), the head attractor when the body lacks it. The client now keeps the bones' posed frames with the attractors and places effect meshes and trails on them; before, such effects were not shown. The weapon (3000) and the mesh's attractor (3001) are not given yet.

### Statels of the second flag layout sheared (RND-141)

A statel whose flags have bit 0 is turned about Y, stretched along X and sheared: N3.dll's FUN_1002777a gives it an anim matrix (scale (1, 1/s, 1/s), then y plus p × x with p = (v / 630 / 211) / 100 − 1.25 of the packed v), which the frame applies before its scale s × C, turn and place. The client read only the turn and the stretch; it now applies the shear too (Statel::matrix).

### Animation cross-fades slerp (RND-030)

When one animation fades into another, the original's CATAnimBlend_t (FUN_10050586) turns each bone by a slerp of the two rotations at the blend's weight (FUN_10053d0b) and moves it by a straight mix of the two positions (FUN_1005063c). The client now does the same in its animation stack and in pose_blend; it used a renormalised weighted sum, which turned a little slower mid-fade.

### Attractors by whole name (RND-031)

The original finds a body's attachment points by their whole names (RCATMesh_t::GetAttractor, FUN_10054df1): the head at `Attractor01_head`, and a worn mesh at the name its place gives in AttractorMesh::GetName's table (0x100af7f0: head, right hand, left hand, right shoulder, left shoulder, back, hip, thighs, crus, arms, forearms, fifteen in all). The client now does the same; it used to take the first attractor whose name merely contained the text.

### City map dots are the pictures' corner (RND-035)

The original's marker atlas (U:0x10046d0b) keeps only the top-left 4 x 4 texels of each GUI picture, so the city map's door dots (GFX_GUI_MAPDOT_BLUE) and guard dots (GFX_GUI_MAPDOT_RED) are those texels. The client now draws the same corner of each picture at 4 x 4 pixels, where it drew the whole picture shrunk.

### Toggle effects by playfield (RND-091)

A toggle effect (kind 3036, GfxControlToggle_t, FUN_10112952) starts its child only on the right playfields (FUN_101125ed): the playfield's first flag word must meet the toggle's mask, and then the playfield must be in its list, or with flag 0x800 not in it. The client now tests the current playfield the same way; it used to pass only toggles with 0x800, as if no playfield were ever listed. The restart under 0x400 is not done.

### Interiors against black (RND-009)

A playfield with no ground (shops, dungeons built from rooms) is seen against black in the original, whose device clears to colour 0. The client's dark backdrop (`mao_render::DARK_SKY`), which its fog fades into, was a near-black blue; it is black now.

### A room playfield's statels all kept (RND-016)

N3's ReadGlobalData makes every global statel of a playfield, wherever it stands. The client used to drop the statels of a room playfield that stood away from all its rooms; it keeps them now.

### Far statels hidden past 50 radii (RND-005)

DisplaySystem's LODManager::Process (globals at 0x100ae1a4) hides a statel mesh when its squared distance over its squared radius passes 2500, and swaps it for its low-detail mesh 6000 squared units beyond its radius (back under 4000); the client's renderer uses the same three numbers (LOD_CULL, LOD_FAR, LOD_NEAR). The original looks at 20 meshes a frame in turn and measures from the mesh's position and bounding radius; the client looks at all of them every frame, from the world box's centre and half diagonal.

### A new character goes straight into the game (LCH-009)

The original logs a character in as soon as the server says it was made (Interfaces.dll `Client_t::ProcessMessage` 0x10002a9e, message 0x11 at 0x10002ee3: the new id into `s_nCharID`, then `LoginCharacter`). The launcher now does the same: once the creation's walk to the shuttle has ended, the game starts for the new character (the welcome loading screen), without logging in again to fetch the character list first.

### The fixed keys' defaults are the original's table (SET-026)

The original's fixed keys come from a key table built into GUI.dll (the text at 0x101aab28, read by `ParseFile` 0x1001b058): the camera on the numeric pad, Tab and its modifiers for targets, Enter / Slash / Shift+R / PgUp / PgDn for the chat, F9 for the coordinates, Shift+P and Shift+V for the perk and vehicle windows. The client's defaults are that table's; the chat input also starts on Shift+Enter and a command on the numeric pad's divide, as there (Alt+Enter, the table's third chat key, stays the client's window-mode switch).

### Entering a character: the progress window until the ticket (LCH-038)

The original's Play (GUI.dll `LoginModule_c::SlotSelectCharacter` 0x10011be1, `Show(4)` 0x1001175a) puts the login and the character list away and shows its progress window ("Please wait...", `DimensionNetworkTimeout`, 30 s) until the server has handed out the character's ticket; only then does the loading screen go up. Running out of time (`SlotSelectCharacterTimeout` 0x10011523) goes back to the login form. The launcher now does the same: the progress window stands over the hall while the game's process logs the character in; that process writes `MAO_TICKET_UP` when the ticket has come, and the loading screen goes up from that moment in the launcher's window and carries on in the game's. Cancelled or timed out, the game's process is ended and the login form is back.

### Deleting a character asks for its name exactly (LCH-015)

The original's deletion confirmation (GUI.dll `CharDeleteWindow_c::SlotOkayButtonActivated` 0x1000db27) goes on only when the typed text is the character's name exactly, with its capitals and nothing around it; otherwise it shows `MatchError`. The client compares the same way now (it ignored case and spaces before).

### A creation refusal the original does not know says nothing (LCH-021)

The original's name scene (GUI.dll `NameScene_t::SetState` 0x1011fc0f) has a text for the refusal codes 1-10, 19-21, 28, 30 and 31 only; for any other code it shows nothing and lets the buttons be used again. The client does the same now instead of showing "ServerProblem".

### A new character's windows start at their own places (SET-003)

The original reads a character's window places from that character's own preference files (`ControlCenterModule_c::LoadUserConfig` 0x1006c3b1); a character it has never played has none, the shipped `CharPrefs.xml` holding no frames, so its windows open at their views' own places. The client no longer borrows the most recently played character's layout for such a character; a character the original has played still starts from its own frames.

### The Keeper and the Shade: the expansion flag or the list's switch (LCH-040)

The character list ends with three numbers: the slots, the expansion flags and a switch that Interfaces.dll (`Client_t::ProcessMessage` 0x10002a9e, at 0x1000319b) keeps as `s_nSLProfsEnabled`. The original's profession scene (GUI.dll `ProfessionScene_t` 0x101216d1, `SlotNextPressed` 0x10121035) lets the Keeper and the Shade be chosen when the account has the Shadowlands (`ExpansionFlags & 2`) or that switch is on. The client reads the switch too and uses the same rule; the head counts still follow the flag alone.

### The loading screen's picture in its own colours (LCH-003)

The original's loading screen (GUI.dll `ServerLogin3DModule_t::SetupScreen` 0x10017331) shows `ai_loading_login.png`, or `welcome_to_rubika.jpg` after a creation, as it is, scaled to the display. The client had tinted it blue; it now shows the pictures in their own colours. The screen over a zoning is still the client's own.

### A new character's name is made as the original makes it (LCH-018)

The original's Finish (GUI.dll `NameScene_t::StartServerCreation` 0x1012015d) says `MustEnterName` for an empty field; otherwise it makes the name itself (FUN_10124093): the first typed character a capital, the rest small, and everything that is not a letter or digit left out (so "sad-lee" becomes "Sadlee"), and only then checks its length and digits. The client now does the same, where it used to refuse such a name as not alphanumeric.

### The profession text area's height (LCH-027)

The original's profession scene (GUI.dll `ProfessionScene_t` 0x101216d1) gives its text area a height of 156 pixels at 480 lines, growing by 0.4 of the display's growth beyond that (193 at 768, 234 at 1080), not a share of the window. The client uses the same rule now instead of a fifth of the window.

### A character refused after Play (LCH-041)

When the server refuses the chosen character instead of handing out its ticket (system messages 0x10 and 0x21), the original shows that error's page over the login form, as for a refused login (GUI.dll `LoginModule_c::SlotLoginReply` 0x10012179 → `ShowError`). The launcher now does the same: the game's process passes the refusal's message and code back through `MAO_TICKET_UP`, and the launcher's progress window gives way to the login form with the error's page above it.

### The error page stays until it is closed (LCH-005)

The original keeps a failed login's page in its Browser window until that window is closed (`SlotBrowserQuitPressed` 0x10011c33), and shows it over the login form again each time the form comes back (`Show(0)` 0x1001175a). The client now does the same: a login that works no longer puts the page away, so it is there again after Back from the character list.

### The game clock before the server's word (OTH-001)

The original's game clock starts at midnight of day 0 (Gamecode `GameTime_t::GameTime_t` 0x1000afdb) and runs from there until the server's `GameTime` message sets it. The client does the same now, where it used to start from the real clock at the game's pace.

### The creation's choices are kept between runs (LCH-013)

The original keeps the breed, profession, head, height, build and name chosen in a character creation among its preferences (`CCSelected...`, read back by `CharCreateModule_t::InitialiseMessage` 0x1011bdaa) and starts afresh only after a character has been made (`WasCharacterCreated`). The client keeps them in `creation.ini` beside its settings: a creation left with Back starts with the same choices the next time the launcher runs, and making a character resets them.

### /ignore as the original's (SET-028)

The original's `/ignore` (GUI.dll FUN_100bafeb) takes one name, `list`, or nothing for the selected target; more words are refused, the player's own character cannot be ignored, and the toggle is said with the game's own texts ("Ignoring character ..." / "Unignoring character ...", text.mdb 10001). The client now follows those rules and texts; `/ignore` alone ignores the target instead of printing the list, which `/ignore list` shows. The list is still kept by name in the client's ignore.ini.

### Back to the character list after a camp (LCH-045)

The original's `LoginModule_c::SlotInitialize` 0x100123ee, called on the way back from the game, logs in again at once with the account and password it kept (`SlotLoginAccount`) instead of showing the login form; the client's launcher restart with `MAO_RELOGIN` does the same, so this is the original's behaviour, not an addition.

### Others walk or run by their mode (MOV-005)

The original picks another character's walk or run animations by its movement mode (FUN_1006c370 over FUN_10070a2f), not by how fast it goes. The client keeps that mode per character (`World::walk_modes`): from the announcement (mode 2 walk, 3 run), from the walk/run mode switches (0x18/0x19) the server passes on and from a server path's walk/run byte. Only a character whose mode was never told is still judged by its speed (over 2.2 a second is a run).

### The server's echo of one's own movement (MOV-017)

The server sends back each movement message of one's own character. The original ignores those for the client's own character (FUN_1006c20f clears a message from the server when its dynel is the client control dynel; CharDCMoveIIR_t's activation FUN_1006bd94 then does nothing). The client did the same except for a full report more than 5 m away that was none of its own in the last 4 s, which put the character back there; now such a report is only logged. Put-backs by SetPos and N3Teleport are followed as before.

### The attack key's refusals (CMB-003)

The attack key and Ctrl+click follow N3Msg_DefaultAttack (Gamecode 0x10028147): a fight is stopped first; swimming then says "can't attack in this state"; otherwise the selection is checked as FUN_10069a9f does and a refusal says its own line followed by "unable to attack target". Of FUN_10069a9f's checks the client follows: oneself, the dead, own state flag 0x400 or flying ("you're unable to attack"), and own flag 0x80000000 with stat 215, which allows any character. Not yet followed: the target's own state flags, surrender, suppression districts, the side and team checks and attackable items.

### A hit and the health message after it (CMB-004)

The server sends a weapon or special hit and then the victim's new health. The original lowers the victim's health by the hit when it tells it (FUN_1009b6b7, FUN_1006af0e), so the health message finds nothing more to tell; a drop beyond the hits is told as "attacked with nanobots for N points of unknown damage" (FUN_100a2006, event 0x1b). The client does the same now; it used to forget hits after 2 seconds instead.

### NCU in use (NAN-004)

The original adds a running nano's NCU (its stat 54) to stat 180 only for nanos flagged 0x10000 and none of 0x4000, 0x8000 (hostile) and 0x100 (Gamecode FUN_100517f3). The NCU window's title and the strip sum the running nanos by that rule (`ncu::ncu_cost`), so heals and hostile nanos running on the character take no NCU.

### Which swing an attack plays (CMB-006)

Of a weapon's attack actions the original picks one by rand() at each swing (FUN_10045976: rand() % count). The client draws the CRT's rand() (seeded 1) for each swing (`World::swing_rand`); it used to go through them in order.

### Self-only nanos in the NCU list (NAN-045)

The NCU window's type column follows FUN_1003f4cf: hostile 2, team 4, self only 3, else friendly 1. A nano is self-only as FUN_100a49b1 decides it: all its spells aim at target 1 or 2, or its stat 30 has 0x800000 (`ncu::self_only`).

### Waiting for the new playfield (ZON-016)

After the server's teleport the playfield is stopped (StopPlayfield) and the character stands still until the playfield message comes, with no timer, as in the original. The client used to let it move again after 30 s.

### How long a cast's effect shows (NAN-002)

The original's cast effect (the program's CastEffectType, a _GfxControlSpell1_t) is deleted by the casting state when the cast ends. The client shows it for the cast's own time: one's own by the cast time with initiative and the aggression setting taken off, another's by the nano's attack time; an instant cast shows none. It used to last the casting animation (1.2 s or 1.95 s).

### /tell alone (CHT-007)

`/tell` with no name does nothing in the original (FUN_100ba8d5 wants two pieces). The client used to open a tell window for the target; it now does nothing too.

### /inspect takes a number (CHT-005)

`/inspect` alone does nothing in the original (FUN_100b280d); with a word it reads it as a number by strtoul and asks what that character wears, a name reading as 0. The client's own /inspect of the target is gone.

### Tells of other kinds are not shown (CHT-025)

HandlePrivateMessage (GUI.dll 0x10087e59) shows a private message only when its blob begins with the kind 0 (a tell), 2 or 6 (an away reply); a message with no blob or of another kind is not shown. The client now drops the same ones; it showed every private message as a tell. Kind 2's script files are not written.

### No team leader before the server names one (CHT-043)

The team window colours the leader the server named (interface message 0x17, TeamViewModule_c +0xe0). The client used to colour the first member until then; now nobody is the leader before the server says who is.

### Team level until the server tells it (CHT-044)

The team window's header shows GetTeamLevel (GUI.dll 0x1007a5d3), the level interface message 0x10 last gave, 0 before. The client showed the highest member's level until then; it now shows 0, as the original.

### Every mail in the inbox list (CHT-046)

The inbox list reads every entry the server's count gives; the client used to stop at 64.

### /cc alone (CHT-004)

FUN_100ba6f4 answers `/cc` with no command by a fixed line that the syntax is invalid. The client now says that (in its own words) instead of its usage line.

### Bold and italic interface fonts (WIN-027)

The original's font table (GUI.dll 0x10272df0) marks entries 4 (CC17, Verdana 20), 6 (BOLD, Verdana 13) and 9 (HUGE, Verdana 24) bold and 11 (ITALIC, Verdana 16) italic; `CreateTTFont` (0x1012ed48) makes them with weight 700 and the italic flag. The client draws those entries with the system's Verdana Bold / Italic faces (verdanab.ttf, verdanai.ttf), falling back to regular Verdana when the face is missing. Text size still follows egui's points, not the original's screen-height scale.

### Inventory and wear are two windows (WIN-028)

The original makes the inventory (`SlotInventoryActivated` 0x100c766b) and the wear window (`SlotWearViewActivated` 0x100c76f1) on two separate distributed values, `inventory_window` and `wear_window`. The client now does the same: I toggles the inventory, Ctrl+1 (the original's WINDOW_WEAR) and the control center's Wear entry toggle the wear window, and either can be open without the other.

### Mission selection window texts (WIN-029)

The original's mission window (GUI.dll FUN_100d10fc) labels its seven sliders' ends with text.mdb 10000's QuestSel_* texts and its buttons with RequestMissions, AcceptMission and MsgBox_Cancel. The client reads the same texts; the window's layout is still the client's own.

### Every window of the game's menus opens (WIN-031)

The game's control-center menu scripts (ActionMenu/*.xml, ControlMenu_c FUN_10071d7d) name each window by its distributed value. The client now ties all of them to its windows, including the Team menu's Team entry (`team_view`), the Vehicle entry (`mechselect_window`) and Chat Config (`chat_group_window`) in the Help/Windows menu.

### Character page: title and defender rank names (WIN-116)

The original's character info page (GUI.dll FUN_1003482c) writes the profession title as its name from text.mdb 1060 (profession * 100 + title level, Gamecode `GetTitleStr` 0x10016185) followed by "(TitleLevel n)", always, and the defender rank as its name from text.mdb 1169 (alien level * 100, `N3Msg_GetAlienLevelString` 0x10016d9b). The client's page now does the same instead of showing bare numbers.

### Links in text views fire on the press (WIN-121)

The original's text renderer (`TextRenderer_c::MouseDown` 0x10163bcd) acts on a link as soon as the left button goes down on it, in a view that takes the mouse (TVF_ACCEPT_MOUSE_INPUT); elsewhere in the text the press begins a selection. The client now opens links on the press too; selecting text in views that do not edit is not done yet.

### Colour codes follow GUIColors.xml on every skin (WIN-002)

The original's `LoadGUIColors` (GUI.dll 0x1006c13d) turns GUIColors.xml's six colours, Default, Selected, Hover, Text, TextSelected and TextHover, into colour codes 1 to 6, whatever the skin; tabs show white text when open and black otherwise (FUN_101462d8). The client now draws code 1 in the skin's Default and code 6 in its TextHover on dark skins too (it used the border colour and Hover there), and inactive tab text is black on every skin.

### City controller's bill date (WIN-050)

The original writes the next bill's date with `strftime("%#x")`; the game never sets a C locale, so it is always the English long date, without a leading zero on the day ("Thursday, November 5, 2026"). The client writes the same.

### Organization boxes' places (WIN-061)

The original raises the organization's leave box (`LeaveOrg` 0x10052c90) and promotion box (`OpenPromotionDialog` 0x10052e53) in the screen's middle (`Window::MoveToCenter` after `Go`) and the disband, invitation and tax boxes at the pointer, where every `DialogBox_c` comes up. The client now does the same.

### Web pages' AcceptLanguage (WIN-062)

The original hands the in-game pages `AnarchyOnline.AcceptLanguage` as the user's locale, language and country ("en-US", `BrowserModule_c::GetLocaleName` 0x1010b794). The client now gives the system's language and country too (WebView2's own on Windows, `LANG` elsewhere) instead of a fixed "en".

### Stat names are the game's (WIN-070)

Gamecode names a stat by text.mdb 2003 and the stat's number (`ConvertCriteria` 0x1001e2cf, `N3Msg_GetSkillToUseString` 0x1001a272). The client's tooltips and requirement lines now take the same text; its own English table remains only for when the game's texts are missing.

### Control center experience bars (WIN-090)

The original's control center bars (`CharacterBar_c`, GUI.dll FUN_1006716e and FUN_100672bb) count experience from the last level's figure to the next's below level 200 and shadowknowledge from level 200 (from zero unless stat 576's bit 0 is set), alien experience from zero, each capped at a full bar. The client's wing and docked bars now do the same (they showed no bar past the figure and ignored shadowknowledge).

### Planet map window's first place (WIN-100)

The original makes the planet map window with the frame Rect(0, 0, 300, 400) (`PlanetMapView_c`, GUI.dll FUN_1004d92d): 300 by 400 at the screen's top left until the player moves it. The client's window now opens there too; a kept place still wins.

### Bars show their maximum as it is (WIN-105)

The original's bars (`PowerbarView_c`, GUI.dll FUN_1007fed3) fill to value over maximum, no more than full, and label themselves with the two numbers as they are. The client no longer raises a bar's maximum to its current value when the computed maximum is lower; the bar is full and its text shows both numbers.

### The inbox keeps its columns (WIN-131)

The original's mail window keeps its list's column widths and sort in `MailWindowConfig` (read by FUN_10109002, written by FUN_1010911b). The client now keeps them too, with the other windows' lists.

### Score bars fill toward the next rank (WIN-141)

The original's Stats window fills the duel, solo and team score bars over the next PvP rank's score (`N3Msg_GetPVPScoreForRank(rank + 1)`, GUI.dll FUN_1008049c; Gamecode FUN_1003ea89: (2500^(1/9))^(rank-1) x 20 x 100 in float steps, rank 1 at 2000 and rank 10 at 4,999,900). The client now gives those bars the same maximum; they showed only the number before.

### Item cells kept per character (INV-019)

The original keeps where each item stands in the inventory and in each backpack in files of the character's own folder (GUI.dll FUN_100ccefe / FUN_100cac66, `item_position_map`). The client's `places.ini` now keeps a line per item with the character first (`<character> <type> <instance> <place> <cell>`), so two characters no longer share one layout; older lines without a character are kept as nobody's. A cell is still counted along the rows of the window, not as the original's column and row.

### An item let go on an empty wear cell (INV-020)

GUI.dll FUN_100e220f: an item let go on a cell of a wear page that is no wear place (the implant page's corners, for one) goes to the item's own place on that page, its stat 0x58, and the wear check (`N3Msg_IsItemPossibleToWear`) is asked as for any wear place. The client now does the same (`invwindows::wear_drop_place`); before, such a drop did nothing. The item is still not kept in hand after a refusal, and there is no "no" pointer.

### Which containers open sorted by name (INV-021)

ItemContainerView_c FUN_100ce55d: a corpse, a chest, another's container, the organization's contract and a trade's list open arranged by name; one's own backpacks and the bank are placed by hand, and the reclaim terminal's list keeps the server's order. The client does the same (`loot::first_sort`).

### More spell lines on item pages (INV-023)

Gamecode `N3Msg_ConvertSpellList` (0x100213ec) writes a line for each spell type it knows, mostly with the program's own English formats. The client now also writes the stat set ("Set <stat> <value>"), the skill lock ("<skill> skill locked for <n> seconds", with the reader's own time in brackets when its skill lock modifier is not 0) and the reduced lock ("Reduce <line> by <n> seconds"), and writes nothing for a temporary set of the stats the original skips. The area nano casts, perk locks and a few rarer types are still not written.

### Shutting the bank tells the server (INV-051)

The original's container window, when shut, uses its container again (GUI.dll FUN_100ca891); for the bank that is Gamecode's `N3Msg_UseItem(0xdead)`, which sends `CharacterAction` 0x6a with nothing in it. The client now sends it when the player shuts the bank's window (`loot::bank_shut`, `net::close_bank`). Not sent: the close when walking away from the terminal.

### Item pages: vicinity and personal modifiers, skills to use (INV-054)

GUI.dll's general item page (FUN_1003882b) shows, after the modifiers, the effects on friends and foes nearby and the personal ones (spell lists 24, 25, 26), then the skills the item is used with, then its charges; the armour and implant pages (FUN_1003826b, FUN_10038586) show the skills to use after the modifiers. The client's pages now follow that order. The shop price, the nano radius, the insurance cost, a building's upkeep and fixtures are still not shown.

### A trade needs the partner within reach (INV-055)

Gamecode's `N3Msg_TradeStart` (0x10019148) starts a trade only when the two characters' bodies are within 5 m of each other (FUN_1005a1e7: centre to centre less both radii), else it says the target is out of range. The client now does the same before it sends the trade's Open. The dungeon's room-and-door test is not ported.

### Requirements "On Self" judged in the wear and use checks (INV-056)

Gamecode's requirement evaluator changes whose stats it reads by the apply-on operators, through FUN_10001fba: "On Self" is the character itself, "On User", "On Target" and "On Secondary Item" other objects, and anything else nothing. The client's checks before wearing or using an item now know "On Self" (before, such a list could not be judged and was let through) and fail a list with "On Fight Target", as the original's picker has no case for it. The user, target and secondary item are still not known.

### Requirements comparing two stats (INV-057)

Gamecode's requirement evaluator (FUN_100065d8) has four operators that compare one of the character's stats with another (equal, below, above, unequal: 0x82-0x85), judged only for the client's own character. The client now judges them; before, such a requirement was "unknown" (no colour in the tip, a perk counted as not met).

### Requirement lines name the breed and sex (INV-058)

Gamecode's `ConvertCriteria` writes a breed or sex requirement by its name ("Solitus", "Female"), with "not" before an unequal one. The client did "Breed exactly 1"; it now writes the name, taken from the game's text database. Special criteria (items, perks, places) are still written as plain stat lines.

### No right click loot on corpses (INV-007)

In the original a corpse's item list ignores a plain right click (GUI.dll FUN_100ce55d sets its feature flag 0x400, which FUN_1004141b tests): items are taken by a click or the item's menu. The client now does the same; a chest, the bank and the overflow still take an item on a right click.

### Item tips (INV-005)

The original's item tip (GUI.dll FUN_10042193) has the item's name in its rarity colour as its title, and under it the item's own info page as a tip shows it (FUN_10038f6b with its "tip" argument: no header and no description, and on the general and implant pages no requirements and no skills to use), while the `ItemDescriptionTooltips` preference is on, which it is by default. The client's tips are now that, drawn as markup; the client's own tip (slot, QL, stack, requirement lines in green and red, "When worn" effects) is gone.

### Destroying an item (INV-025)

The original's `N3Msg_DeleteItem` (0x1001c864) asks the server and the item goes when the server answers; it refuses to destroy a worn item or a bag with something in it, with the game's lines. The client now does the same; it no longer takes the item out before the answer or says "Item deleted".

### The tradeskill result's name (INV-035)

The tradeskill window's "Result will be QL n ..." line names the item the chosen quality makes (GUI.dll FUN_100df105: a dummy item of the two templates at that quality, then its name), so a result whose higher qualities have another name reads so as the slider moves. The client now does the same.

### Wear places take their own page's items (INV-036)

The original's wear check (Gamecode FUN_100498ed, FUN_10080659) lets an item onto a wear place only when the place's page takes the item's class (weapons, clothes, implants and spirits, and on the social page clothes and weapons, a weapon only on the social hands) and the place is one of the item's placements. The client now does the same before it sends a move.

### A dropped item's cell waits for it (INV-046)

The original keeps the cell an item was let go on until the item comes (GUI.dll FUN_100ce25d, the view's +0x150). The client forgot it after five seconds; now it waits, so on a slow server the item still lands where it was let go.

### Items put into a backpack land where they were let go (INV-063)

As in the original (GUI.dll FUN_100cc855 and FUN_100ce25d), an inventory item let go on an empty cell of an open backpack now appears in that cell when the server has moved it, as it already did for the bank.

### Wearing by right click goes to the item's own place (INV-015)

The original's `N3Msg_UseItem` wears an item at its own place (stat 88) and takes off what is there first. The client no longer picks another free place the item could also go to (a second ring finger, the other wrist); the item goes to its own place, swapping with what is worn there.

### An unknown camera mode is the free camera (CAM-002)

When no camera mode is preferred, the original uses its free camera (mode 1, GUI.dll FUN_10021859). The client now does the same for a `camera_mode` name it does not know, where it fell back to the rubber camera.

### The music's random choices differ from run to run (SND-005)

The original's music player seeds its random numbers with the tick count when it starts (SIMPlayer.dll, `srand(GetTickCount())`), so which piece and layer play differ every time. The client had seeded them with 1, so every run chose the same; it now seeds them from the clock.

### The inbox size counts the list's rows (WIN-009)

The original's mail window shows its size as the rows in its list over 30 (GUI.dll FUN_10109489). The client now writes the same, where it showed the server's count.

### The district's level range at the map's corner (WIN-011)

The original's land-control view shows the character's district and its level range at 5, 5 of the map (GUI.dll FUN_100eb04b). The playfield map window now shows it there too when the land-control view is on; the client's own overlay map keeps it under its button.

### Mail without a recipient or subject (WIN-013)

The original's Send (GUI.dll FUN_1010733d) checks only the lengths and the money; a mail with no recipient or subject goes to the server, which answers it. The client no longer stops such a mail with its own line.

### The shortcut bars start where the original had them (WIN-017)

The original keeps each shortcut bar's frame in the character's `Containers/ShortcutBar_<n>.xml`. When the original's bars are on (`smart_hotbar = off`) and a bar has no place of its own here yet, it now starts where that file puts it, scaled to this window; otherwise the bars stack up from the bottom left as before.

### A text button is as big as its text (WIN-019)

The original's text buttons (`TextButton_c::CalculatePreferredSize` 0x10004e3f) are as big as the text they show now: raised, pressed or under the pointer. The client used the widest of the three; it now uses the shown one.

### Tip patterns as the original matches them (WIN-020)

The original's tips (GUI.dll FUN_100539be) match a pattern `*x` against the start of an item's or playfield's name, as `x*`; only `*x*` looks inside the name. The client had taken `*x` as "ends with"; it matches as the original now, so those eleven tips fire where the original's do.

### The attack key's checks (TGT-004)

The original's attack key calls `N3Msg_DefaultAttack(selection, false)` (Gamecode 0x10028147; a Ctrl + click is the same with `true`, `N3Msg_SwitchTarget`): a fighting character is stopped first; swimming then says "Feedback_CantAttackInThisState" and nothing else; a fight that went on ends there for the key. Else FUN_10069a9f checks the target, saying its own line, and the key adds "Feedback_UnableToAttackTarget" after any refusal. The check's order: oneself, swimming, the own stat 224's bit 0x400 or flying, a dead target, then a target that fights oneself may always be attacked, and a member of the own team or raid is refused with the local action 0x76 of parameter 5 (FUN_100692b4), which the action handler's table says as "Feedback_NotAllowedToAttackTeamMembers". The client now does the same (`combat::default_attack`, `combat::can_attack`). Not checked, the client not knowing them: the target's own flags (stat 224 bits 0x20000000 and 0x80000000), the surrender stats, an item's attackable flag, the suppression-gas line.

### Friends for the targeting keys (TGT-005)

`N3Msg_GetCloseTarget` (Gamecode 0x1001c49a) counts a character as a friend when its side (stat 33, or 668 on battle stations) is one's own, and takes that side as one's own for a player in the own team (FUN_10065d65) or in any team of the own raid (FUN_10065dab), and for a creature whose master (stat 196) is oneself or such a member. The client now does the same for players and for its own pets (`combat::friend`); the pets of the other team members are not known to it.

### The white at one's own death (DTH-007)

When one's own death has played out, `CharDie_t` (Gamecode FUN_1007bad5) makes effect 3000, which `gfxtweak.bin` has as a `_GfxControlWhiteOut_t` (kind 1015, FUN_101052b2): a white sprite over the world whose alpha is its age over the template's time (FUN_1010509a); template 3000 gives no time, so the world is wholly white at once, and stays so (life -1) until the revival deletes the effect (FUN_1003ec1f → FUN_1003e61c), when it is gone at once. The client does the same (`whiteout.rs`): no rise and no fading out, the windows and the chat drawn over it as before.

### A creature's animation for an action (NPC-007)

The original takes a creature's animation for an action from its own set (FUN_1003cac9 → FUN_1003ca14 → FUN_10045976), going down FUN_10010b3c's fallbacks when the set has none (sneaking and flying fall to the walk; the walk, the run, the sidesteps, the backward moves and the standing have no fallback). Finding none, FUN_1006c370 starts nothing and the animation already playing goes on. The client does the same (`Template::animation_candidates`, `Characters::creature_last`): a creature with no run keeps whatever it played as it runs, one with no standing keeps its last movement going when it stops. Only a creature that has played nothing yet still starts in the client's own order (`Template::start_candidates`: the walk for the run, a standing on 127 or the walk's first frame).

### A creature's announcement (NPC-018)

The client reads a creature's full update in the order of the original's reader (Gamecode FUN_1007916d): the family and two more fields, the level, the health and its maximum, then the template (`MonsterData`), the body's scale, the movement block (the velocity and the status bytes, FUN_10067b62), the head's mesh and the run speed, each with the width its flag bit gives. It used to find the template by looking for the bytes that follow it (the scale, a zero and 0x1f), which drew an unusual creature as a box.

### A creature's movement mode (NPC-016)

The movement block in a creature's announcement is the original's vehicle state (Gamecode FUN_1006f587): three floats of velocity, then the status bytes (FUN_10070981), the first of them the movement mode (FUN_10070a2f): 8 is sitting on the ground, which the client draws lying or sitting as before; the run speed follows the block and the head's mesh. Now read field by field, no longer by measured offsets.

### A creature's running speed (NPC-017)

A running creature goes at the characters' speed rule (Gamecode FUN_1006f9eb, mode 3: `RunSpeed / 275 + 5`, within 1.5 and 13), with its run falling off below 15% of its health (FUN_1006f2fc), as a player's does. The client now applies the fall too, with the health it knows of the creature.

### A creature's walking speed (NPC-019)

A creature walking a path goes at 1.5 a second: the original's vehicle sets every character's walking speed (mode 2) to that constant (Gamecode FUN_1006f9eb, `_DAT_1015e7ac`), which the client uses, where it had the same number from measuring wandering creatures.

### Fight lines of one's pet (CHT-016)

The original queues a fight line of one's own pet as its own kind: a hit between others where the pet is hit or hits (Gamecode FUN_1009b6b7 0x21, a special FUN_1006af0e 0x33), and damage to the pet (FUN_100a0611 0x1c, even when oneself is the source), which the formatter FUN_10012c3e puts in the chat groups "Your pet hit by other" and "Your pet hit by nano". The client does the same for the pets the server gave it, so these lines follow those groups in the chat windows.

### Camera keys follow the turn and zoom settings (CAM-006)

N3.dll's camera turns by the rotate keys' 0.02 rad a frame times `MouseTurnSensitivity` (FUN_10022345 → FUN_1002118c) and moves along the view by the zoom keys' time times `ZoomSpeed`; the defaults (LoginPrefs.xml) are 10 and 20. The client now reads both settings for the keys, as it already did for the mouse and the wheel.

### District music indoors (SND-002)

The original finds the district of the character's zone (`PlayfieldDistrictInfo_t::GetDistrictData(GetZoneInstanceID())`), and in an interior the zones are its rooms. The client now does the same with the room the character stands in, so an interior whose rooms belong to different districts plays each district's music and shows its land control info; before, every interior took its first zone's.

### Talking to an NPC (NPC-001)

The client asks an NPC to talk once, as the original does. The client's own retry (an NPC silent for two seconds was told its chat was closed and asked again, which opened the window two seconds late) is gone.

### /help (CHT-002)

`/help` shows the game's own help page of commands (`helpcommands.html`) in the info window, and `/help <topic>` the page of that topic (org, pet, emote, macro, chat, team, raid and the rest of GUI.dll's list, FUN_100a4d15); an unknown topic says so in the chat. The client's own list of the commands it carries out is in its help assistant.

### City map buttons (TGT-006)

The original's city map (`BuildingMapView_c`) does not look at which mouse button was pressed: any button selects a house, drags a waiting one or scrolls. The client now does the same.

### Own death line (DTH-004)

Each message that tells one's own death with a reason says the game's death line (Gamecode FUN_1005b3d8), with no gate of the client's own between them.

### The mission journal's list (NPC-002)

The server's missions are added to the journal as the original adds them (Gamecode FUN_10056cdd: pushed onto the list without looking), so a mission told again is listed again; a zoning empties the list, as the original's new character has none.

### No "Take all" in container windows (INV-080)

The client's own "Take all" button is gone from corpse, chest and overflow windows. As in the original (GUI.dll `InventoryView_c` FUN_100ccefe, `ItemContainerView_c` FUN_100ce55d, which build only the item list), items are taken one click at a time; the `auto_loot` option still empties a corpse as it opens.

### Mail window without a status line (WIN-166)

The client's own status line under the inbox ("The inbox holds N messages.", "Sending ...", "Mail sent.", the declined-mail and no-terminal notes) is gone. As the original's `MailWindow_c` (GUI.dll FUN_10109a11), the window shows the list and the "n/30" inbox size (FUN_10109489); a delete refused for an untaken attachment is still told in chat (`Feedback_MailDeleteAttachments`, FUN_101096f1).

### Inspect window: title and item menu (WIN-168)

The Inspect window is titled "Inspect" for everyone inspected, as the original's (GUI.dll FUN_100e24b8); the client's "Inspect: name" is gone. A right press on an item opens its menu at once, as the original's lists without click listeners do (FUN_10041c04, FUN_100caf07): Look At shows the item's page, Create Reference puts its link into the chat's line. The original's lock entry for a worn item is not in this menu.

### No "Trade complete." status in the shop (WIN-173)

The client's own shop status text ("Trade complete.", "No open trade to buy in/sell in.") is gone; it was never shown. As the original's `TradeView_c`, the trade window says nothing of its own when a trade ends.

### Skills window: no own confirmations (WIN-175)

The skills window no longer writes "Changes sent." after Accept or "Reset asked for." after a reset; the original's skill view says nothing then. Its refusals stay: the disabled skill's line (GUI.dll FUN_100fe968), the IP line from text.mdb (`Skill_LackIP`, FUN_100fe550) and the cap's line (the original's `N3Msg_SkillMaxFeedback`, FUN_100fdf13; still the client's words here).

### An empty account reaches the character screen (LCH-043)

The client's own "This account has no characters." on the login form is gone. As the original (GUI.dll `LoginModule_c::SlotLoginReply` 0x10012179, reply 0xe shows the character screen with no count checked), an account with no characters gets the character screen, where New Character makes the first.

### No expansion message at the Jobe booth (ZON-024)

The client's own box "You must have the expansion pack..." at a Jobe booth is gone. As the original, a character without Shadowlands who steps into the booth gets no message (the server answers the try with nothing); the original says `Feedback_YouMustHaveExpansionpack1` only as a criterion's failure line (Gamecode FUN_1000556a).

### Walk/run mode not announced on arrival (MOV-033)

The client no longer tells the server its walk or run mode on entering each playfield; as the original, the mode goes out only when the player switches it (the walk/run key or tile). This replaces the note that the client tells the mode (0x19 run, or 0x18) on entering every playfield.

### No backpack count heading (INV-082)

The "Backpack 12/30" count described earlier is not in the client: the inventory window shows no such heading, as the original.

### Nano landing glow removed (NAN-056)

The client's own landing glow (three coloured rings widening round the target's chest for about a second, coloured by the program's effect colour stats or, without them, one of six colours by school) is gone. As in the original, a landing nano shows only its own effect from gfxtweak.bin (the program's HitEffectType, drawn with the cast). Area nanos (`AreaFormula`) draw nothing of their own yet: the original runs the nano's landing on each listed character (Gamecode FUN_1007212d -> FUN_1005141d), not ported.

### No distance haze without an area fog (RND-185)

The original's only fog is the area's (`VisualFog_t`: linear from the near plane to the far plane drawn in by the density, in the area's colour). The client used to fade far scenery into the sky's horizon colour wherever no area fog applied; that haze is gone, and such areas are drawn clear to the view distance, as the original draws them. Areas with a fog are unchanged. The effect scripts' own fogged sky draws (a blob that turns fog on) still fade over the view distance.

### Plants no longer sway or bend (RND-187)

The client's own plant motion (a sway in the wind, stronger towards the top, and a bend away from the player within 3 units, both in the shader) is gone. As in the original, which has no plant flag, a tree, bush or grass moves only by its mesh's own animation, which the client now plays for plant meshes as for any other. Plants are still told by their mesh's file name, for their cut-out texture only (RND-002).

### Generic particle emitter draws its template's own numbers (RND-191)

The generic emitter (kinds 1003 and 1005-1008, used by the `MAO_CAST` preview) used to dim dense clouds by their coverage, scatter particles round their origin and clamp life, size and count. It now draws the template's own life, rate, sizes and colours, as the original's emitters run by their template.

### Hit effects keep their own fading (RND-192)

A nano's hit effect used to have its particles eased in over the first 0.1 s and out over the last 0.4 s of the client's landing window. That fade of the client's own is gone: the particles show as their template fades them, as the original's hit effects do.

### No floating combat numbers (CMB-038)

The client's own numbers floating up over the fight (damage dealt, taken and healing, coloured, for 1.6 s) are gone. As in the original, a fight is told only in chat (the Combat lines, Gamecode FUN_100a0611 for each HealthDamage); the damage meter is unchanged.

### Using things says nothing (CHT-070)

The client's own "Using <name>" line (on every use of the target and every perk action) and its "Nothing targeted to use" notice were removed. As in the original (N3Msg_UseItem; a perk's FUN_100427d6), a use is sent without any line, and the use key with nothing targeted does nothing.

### /fxscript runs the line as typed (CHT-072)

The client's own help for `/fxscript` was removed: a bare script name is no longer given a parameters mark, and no chat line tells what the script started or what its reader complained of. As in the original (GUI.dll FUN_100b70be), the typed line goes to the scripts' engine as it is, a script runs only when a parameters part follows its name (`/fxscript <name> #`), and the reader's complaints go to the log only.

### Deposit to Market says nothing (CHT-075)

The client's own "Sent N items and X credits to the market." line was removed. As in the original (GUI.dll FUN_10009c40, N3Msg_SendMarketItem), Send only sends the items and credits; the market window shows what was sent.

### Inspecting opens the info page only (CHT-076)

The client's own chat line with the inspected character's name, level, profession and health was removed. As in the original, /inspect, the target-info key and Shift+click open the character's info page (FUN_1003482c) and say nothing in chat.

### Commands confirm nothing (CHT-078)

The client's own confirmation lines for /macro, /waypoint, /befriend, /viewdist, /chardist, /char&viewdist and /bug were removed. As in the original (FUN_100b8c82, FUN_100b9966, FUN_100b69a7, FUN_100b68dc, FUN_100b6a54, FUN_100b6019), these commands do their work silently; /reply with nobody to answer and /ignore keep the lines the original has.

### Camera mode changes silently (CAM-013)

The client's own "Camera: third person ..." line was removed; choosing a camera mode in the interface sets it without a word, as the original's menu does.

### No "Nano programs available." line (NAN-058)

The client's own line telling that the nano programs are free again after a long cast and recharge was removed; as in the original, nothing is said when the programs can be cast again.

### NPC chat: no "given back" line; answers go after a click (NPC-040)

The client's own "N item(s) given back." line in the NPC chat window was removed: the original's window (GUI.dll) has no handler for the items an NPC gives back, which only return to the inventory. A clicked answer is still told at once as the player's line, which is the original's own echo (FUN_10059504 → FUN_10058ee7 kind 2); as there, the answer list is now emptied until the NPC gives new answers.

### No /sit command (MOV-029)

The client's own `/sit` command was removed: the original has none, and typing it now does what any unknown command does (it is taken for a script's name). The X key sits down and stands up as the original's `N3Msg_SitToggle` does, and `/camp` logs out.

### No message box of the client's own (WIN-154)

The client's own box of transient messages at the left bottom of the screen was removed. As in the original, messages are feedback lines in the chat window: the game's texts where the original writes them (logout, actions not available, /follow's "Following ..." and "Cannot follow selected target.", a failed crossing's "Area change not initiated ...", /assist, /duel, mission waypoints), and nothing where the original says nothing (a screenshot, a missing macro's press, an unknown window name, a cancelled nano, the walk-to and Whom-Pah notices, zoning). The client's smart hotbar, an option of its own, tells its waits in chat.

### Windows docked as tabs of one frame, and the chat windows' fading (WIN-114)

The original keeps each dockable view (inventory, bank, backpacks and other containers, mission log, mission terminal, info, tradeskill kit, knowledge, actions, and the rollup's views when out of the column) in a dock area (GUI.dll `DockingController_c` FUN_1003b01d/FUN_1003aad8, `DockArea_c::AddView` FUN_10039d56), shown as a `DockTabbedWindow` (`DockWindow_c` FUN_1003c78b): a window with no title and a tab per open view. A view opened the first time gets an area of its own (`DockArea<n>`, FUN_1003aca6), one opened again goes back to its place in its old area; the tab opened is shown, a closed tab gives way to the one at its place, else the last (FUN_1003c384, `TabView` FUN_10147fbc); the window goes with its last tab and its close button closes every view in it (FUN_1003c2f9, FUN_1003c94d). A tab is taken up by the right button let go on it and put down by the left: on a tab, before it; on the strip, last; elsewhere into a window of its own whose corner is there, when its area lists more than one view (FUN_1003c428, FUN_1003c5a9, FUN_1003b263). The client does the same (`docks.rs`, drawn by `chrome::window`): the area's window keeps its place, size, pin and fading under the area's name, and the areas live in `docks.ini`; without it they start as the original's own `DockAreas/*.xml` for the character (`aoprefs::dock_file`, which reads every identity of an area). Escape drops a carried tab (the client's own). Chat windows: a framed one fades as every window does (the original's chat window has no fading of its own in its normal mode, FUN_100972da); a borderless one fades as a whole, text and background, to its "active" level over 0.2 s while it is typed in and to its "inactive" level over a second otherwise (FUN_100972da, FUN_1009767c), and never by the pointer. Not seen live.

### Thin, fat and robed bodies in the world; a board's jump (NPC-006)

The original draws every character with the body Gamecode's FUN_100585bf names: `<breed>_<sex>`, `_thin` or `_fat` by the Fatness stat (47) the full update's packed word and Stat messages set, `_robe` by the robe flag of the full update and the appearance update (FUN_100583fa, FUN_1005853e). The client now does the same for everyone (until now only character creation showed the thin and fat bodies); the body also picks the vehicle program's board (a robed body has its own) and the animation's calibration. A character on a vehicle body jumps with that body's own animations: a hoverboard's tricks in the air (0x9c standing, 0x9d moving, FUN_1006dcdb) and its run on landing (0xbb/0xb9/0xba, FUN_1006dd6a); a hoverbike has none.

### The limbo window (LCH-035)

When the login is answered with system message 0x4e (a character of the account still in the world), the original shows `LimboWindow_c` in the character list's place (Interfaces.dll `Client_t::ProcessMessage` 0x10003537 → GUI.dll `LoginModule_c::SlotLoginLimbo` 0x10011c56, `Show(2)`): `Views/LimboWindow.xml` with the two limbo texts, the character's name, level, gender, breed and profession, a countdown bar over the time-out (at least 10 seconds) and Login. Login enters that character (`SlotSelectCharacter`: the 0x16 and the progress window); the time running out or closing the window goes back to the login form (`SlotLimboTimeout` → `Show(0)`). The client now does the same instead of showing an error page; Login starts the game, whose own login enters that character. Built from the code: no capture has the message.

### Rendering effects, veins and trails (RND-137)

DisplaySystem's VisualMesh_t set-up (FUN_1006d49a) gives a mesh a rendering effect when one of its connectors is named eff_hologram (1), eff_space (2) or eff_clear (3), the last such connector counting (`mao_formats::attractors::rendering_effect`; every part gets `Material::effect`). Of the game's meshes only the asteroid 0x403b7 (playfield 109) has one, eff_space. It is drawn as FUN_1006c005 draws the space effect: with SpaceDeltaState alone (FUN_1006c61d: lit, writing depth, no fog), so its materials' own states are not applied and it is drawn opaque, whole, one-sided and without the fog. No mesh names the hologram or the clear, and the hologram's textures 0x28022/0x28023 are not in the database, so those two have no draw here. The templates the eff_universal connectors name that drew nothing (12200, 12570, 96003) are in gfxtweak.bin; their classes are now drawn. The vein (kind 3014, `_GfxControlVein_t` / GfxVisualVein: Gamecode FUN_10102e1a and FUN_10102c87, DisplaySystem FUN_1002def9 / FUN_1002d79f) is strands climbing on a B-spline cross-section, its heights growing by 1.3 a level, coloured by two gradients, drawn twice, with the fire 12205 at its foot: the Shadowlands' veins (12200) and the Whom-Pah pillars' beams (96003). The trail (kind 3019, `_GfxControlTrail_t` / GfxVisualTrail: Gamecode FUN_10102566 and FUN_10102331, DisplaySystem FUN_1002b486) keeps a sample every twentieth of a second and draws ribbons back along them at 2.5 a step, fading with each sample's life: one ribbon facing the view (12570, the ships' engines) or four from the samples' orientations (71000, 71003). Statel effects and the script meshes' universal effects lay these shells; a script mesh's effect carries its object's turn.

### Flight ends as the original ends it (MOV-021)

Besides the end of the spell list that began it, the original takes a character out of fly mode (FUN_10045020) on the character action 0xac, when a Notum Wars site of tower class 5 is among the four sites nearest it within 40 m (FUN_100450c0 on each player's game tick, LandControl_t's tower tree, FUN_1012a2e6), and when its state flag 0x2000000 comes on (FUN_10044aac). The client now does all three. The spells 0xcf4b and 0xcf4c (FUN_100a7bd4 / FUN_100a7c79), which add or remove state flags (stat 224) from their stat 0x49, are run into the character's state flags. One's own flight from a worn item or a program, once ended this way, stays ended until the item or program no longer gives flight. Not drawn yet: the entering burst, effect 2000 (_GfxControlFlare1_t), which the client has no drawing for.

### The attack key reads the target's own stats (CMB-003, TGT-004)

FUN_10069a9f, the check behind the attack key and Ctrl + click, also looks at the target. Its state flag 0x20000000 refuses with "Feedback_CantbeAttacked" (Arete's newcomers carry it). Its flag 0x80000000 with its stat 215 lets anyone attack it. A dead target that still has health is refused without a word. An NPC surrendering (stats 449/451) is refused with the surrender lines; "You accepted surrender" is said when it surrendered to oneself. An item standing in the world is judged by its flag 0x10000000 and its requirements for action 11, whose lines come first. The client now holds other characters' stats as the server's Stat messages set them, and does all of this. Not done: the stat 673 test at the start (its map keys are unidentified), and the team check's +0x1f0 exception.

**Fight modes (2026-11-19).** Each district of a playfield has a fight mode, 0 to 4 (`DistrictData_t` +0x54; text.mdb's
`SuppressionField100`, 75, 25, 5 and 0), and a character's mode is its district's, found by the zone it stands in (FUN_1003e3e2;
2 where the playfield has no district info). The client reads it from the districts' records (`District::fight_mode`) and:

- **tells the district's line** when one's own character comes into a district of another mode, on zoning in too
  (FUN_1003f6e6 → FUN_1003f5b2 → FUN_1003f4a3). The original sends it as AFCM module 0x19's message 4; which window shows
  that was not traced, so it is a system line here, a guess;
- **warns before an attack** at a character in a district of a lower mode than one's own:
  `Feedback_TargetIsInADistrictWithHigherSuppression` (FUN_10069a9f, 0x10069e67); the attack still goes out;
- **holds friendly nanos to one's own side** when either character is in mode 2 (FUN_1004ed2f): a team or raid mate, else
  one of one's own side (in a team: the target's side against one's stat 0xd5), else
  `Feedback_YouAreNotAllowedToExecuteFriendlyOnOppositeSide`. On a battlestation (the playfields 4380 to 4388,
  `n3Playfield_t::IsBattleStation`) the side does not count: the target's stat 0x29c, its team in the battle, must be one's own.

Not ported: the playfield's `FightModeHandler_t` changes to a district's mode (FUN_1011fddd's list: the server's, not kept),
and a dynel's own mode kept at +0xf4 while its +0x138 has bit 16 or 2.

### Tab: others' pets and characters not yet in play (TGT-005)

N3Msg_GetCloseTarget (0x1001c49a) counts a creature as a friend when its master (stat 196) is oneself or a member of the own team or raid. It passes over any character not in play (stat 194). That flag comes from the announcement's flags bit 1 (FUN_1007803b) and is set later by the CharInPlay message (FUN_10072b96). The client now does both, taking other pets' masters from the server's Stat messages. Not yet: a master known only from the announcement, and the battlestation's own side stat.

### Creatures after a target stop 4 m short (NPC-017)

When the server sets a creature after a target (FollowTarget), the original's vehicle aims for the target's place less 4 m along the way and stands once within 4 m, measured in three dimensions (FUN_100706ce, DAT_10161ad0 / DAT_10161ad8). The client now does the same; it stopped 1.5 m short before. A running creature's speed is the characters' rule (FUN_1006f9eb) over its RunSpeed as the server last told it, by its announcement or a later Stat. The speed the client used to learn from two paths in a row is gone.

### NCU in use (NAN-004)

The original keeps the NCU in use (stat 180) as a running total: a starting nano adds its stat 54 (Gamecode FUN_100517f3, 0x100518a7) and an ending one takes it away again (FUN_10051041, 0x1005131b), both only for nanos whose flags have 0x10000 and none of 0x4000, 0x8000 and 0x100. The client sums the same nanos' stat 54 over the running list, which is that total; a cost of 999 counts as it is, as there.

### Nano programs placed by hand (NAN-039)

The original's nano grids are not arranged to start with (GUI.dll FUN_100423fc clears the list's +0x318): each program stands in the cell kept for it (NanoView_c's item_position_map, FUN_100d4d44 / FUN_100d5328), a new one in the first free cell of its school's grid in the order of the program list (FUN_100d48a9). The client does the same, keeping the cells in places.ini under the nano identity and the school. A program dragged onto another cell of its grid moves there; onto another program the two change places and the other is then in hand (FUN_100d4514, MultiListView_c::MoveItem). "Don't Auto Arrange" in the Auto Arrange sub-menu (FUN_10041637(1)) is live; each school's grid keeps its own arranging. The radius column shows "---" as the original does: N3Msg_GetFormularRadius reads a field Gamecode only ever sets to 0.

### NCU window's Auto Arrange (NAN-040)

The NCU window's menu (GUI.dll FUN_100d758a) has "List Mode" and "Auto Arrange", whose sub-menu (FUN_10041637(0)) offers "By <column>" for each column but the icon's and "Sort Ascending", dead in list mode. The client shows the same; the grid's sort chosen is kept (the original's grid_sort_column/order), the time left to start with. The radius column's "---" is the original's (N3Msg_GetFormularRadius reads a field that is always 0).

### /version (CHT-002)

The original has no /version of its own in GUI.dll: the line goes to Gamecode's text commands (N3Msg_TextCommand, FUN_1003fe10 at 0x1003fec0), which ask the server with CharacterAction 0x9d and say nothing themselves. The client does the same; the client's own version line is gone.

### Fonts that follow the screen's height (WIN-027)

FontSystem_t (GUI.dll 0x1012eef1) sizes two of its fonts by the screen: CC17 is 20 times the display height over 768, TT_MIN12 is 14 times one plus 0.4 of how far the height over 480 is above one, each cut to a whole number; the others keep the table's sizes. The client does the same with the window's height in points (CC17 28 and TT_MIN12 21 at 1080).

### Character info page: more of the original's lines (WIN-116)

GUI.dll FUN_1003482c's page also has the side's tokens for one's own character (stat 62, an omni's 75), the side experience bonus from the info packet ("%.02f%%"), an omni's organization as "Detachment", and for one's own character the veteran points, victory points, ICC commendations, Freelancers Inc. tokens and specializations, then the duel, solo and team ranks by name and number (N3Msg_GetPVPRank's thresholds) before their kills. The client now writes all of these. Not yet: the organization's grid destinations, the city, the account's items and the item page's shop price.

### Mission journal: active mission and Update Team Members (WIN-119)

The original's journal keeps its active mission (MissionViewConfig's active_mission, GUI.dll FUN_100d3644 / FUN_100d2b0e) and, whenever its list is filled (FUN_100d347d), uploads that mission's place again if it was not since the window was made or since the mission was last missing (after a zoning). The client does the same, keeping the active mission per character. "Update Team Members" on a team mission's row sends GiveQuestToMembersIIR_t with the mission (N3Msg_UpdateNearbyTeamMembers, Gamecode 0x10017e63).

### Selecting text in views that do not edit (WIN-121)

The original's text views that take the mouse (TVF_ACCEPT_MOUSE_INPUT) begin a selection where the left button goes down off a link (TextRenderer_c::MouseDown 0x10163bcd: GetBufferLocation, BeginSelection) and grow it while the button is held. The client now does this in views that do not edit too, selecting the text as drawn and shading it; the right-click menu that copies it (PopulateRightClickMenu) is not there yet.

### Requirements "On Target" read the selection (INV-056)

The requirement evaluator's "On Target" (operator 18) moves to whom the holder targets: Gamecode FUN_10001fba case 3 reads the character's +0x1d0 object's +0x5c identity, set only by N3Msg_SelectedTarget (0x10017790) for the client's own character. The client's wear, wield, use and special-attack checks now take the selected target there (its below/above criteria pass, as for anyone not the own character; nothing selected fails the list); another character's target is nobody. "On User" and "On Secondary Item" are still unknown.

### Requirements read worn items, nanos, perk locks and free places (INV-057)

Gamecode's criteria evaluator FUN_100065d8 answers many operators from the world. The client now answers those the game's items and nanos use and it can know: the server's time (5, 6), the character's identity (9), an item of a template worn or wielded and its "not" (0x1f-0x22), a nano program known or not (0x23, 0x24), a nano or a strain running or not (0x5b, 0x65, 0x5c, 0x66), a perk lock running or not (0x5e, 0x61), free backpack places (0x6a), two pickers finding the same dynel (0x32), and 0x2c, which always fails on the client. One's own worn list, programs, running nanos, perk locks and backpack are read; others' running nanos as heard of. The lines the original says for these failures are not told.

### Requirement lines in the original's words (INV-058)

Gamecode's `ConvertCriteria` (0x1001e2cf) writes many requirements as lines of their own: "must have <item> equipped", "must have nanoprogram", "affected by <nano or strain>", "must have perk <name> <rank>", "must have N free inventory slots", team, raid, organization, pet kinds, flying, indoors, Shadowlands and others. The client now writes these (texts read from text.mdb by name) instead of a plain stat line, and, as `N3Msg_CriteriaListToString` does, moves an "On Target"/"On Self" heading criterion down past the and/or/not before it. Mission-flag, weapon-flag-list, team-side, expansion-list and random-roll criteria are still written as plain stat lines.

### A zoning keeps the last picture (LCH-003)

The original shows no loading screen at a zoning: `FlowControlModule_t::TeleportStartedMessage` (GUI.dll 0x10029c42) says "ChangingArea" and turns the 3D view off (`DisplaySystem_t` +0x44), so neither the world nor the interface is drawn and the last picture stays until the new playfield is in. The client now does the same: the line is said, and no frame is drawn from the zoning until the new playfield's scene is uploaded (`loading::freeze`, `frozen`, `done`); the loading picture with its 1.5 s fade, the client's own, is gone from zonings (it stays for the login, as the original's `ServerLogin3DModule_t`).

### AutoCloseWindows and the menus' activate actions (SET-003)

A window entry of the control center's menus runs its `on_activate` actions when its window comes on and `on_deactivate` when it goes off, as GUI.dll's `CCMenuEntry_c` value callback (FUN_10065dd0) does. The game's menus use this for `AutoCloseWindows` (false in CharPrefs.xml): with it on, opening one of the character's windows from the menu closes the others.

### Item cells keep their column and row (INV-019)

The original keeps each item's cell in a container as a column and a row (`item_pos`, an IPoint: GUI.dll FUN_100ccefe reads it, FUN_100cac66 writes it; `MultiListView_c::AddItem` 0x10135c61, `MoveItem` 0x10136fd5), and a grid that is not auto-arranged is as wide as its view or as the cells used reach (`RecalcUsedSlotExtents` 0x10134009, `RecalcVirtualSize` 0x10134944); a new item takes the first free cell along the rows of the view's columns, no fewer than three (`GetFirstFreePos` 0x10133940). The client now does the same: `places.ini` keeps a column and a row per item (per character), an item stays in its column when the inventory, bank or bag window is made narrower (the grid scrolls sideways) or wider, and older one-number lines are read in the columns each window opens with.

### Wear cells refuse an item in hand at once (INV-020)

The original's wear pages ask `N3Msg_IsItemPossibleToWear` as an item in hand moves over them and when it is let go (GUI.dll FUN_100e1ded, FUN_100e220f): the pointer becomes the "no" pointer over a cell that will not take the item (a split in hand, beside the cells, a cell that is no wear place, a place the check refuses; an item worn nowhere everywhere but over another item), and an item let go where the check refuses stays in hand while the refusal's lines are told. The client now does the same: the game loop judges the held item at every wear place (`World::refused_wear_places`) and the wear window reads that verdict for the pointer and the drop.

### Spell lines of every kind on the item pages (INV-023)

Gamecode `N3Msg_ConvertSpellList` (0x100213ec) writes a line for each spell it knows; the client now writes the casts with a link to the nano's page (single, team and area casts), nano resists, an item's special ability, perk locks (perk name and rank), faction sets and changes, health and nano hits, anchors, offensive and defensive procs, quest and mail actions, cleared locks, organization cash, the end of a fight and sneaking, and Root and Taunt read the values the original reads. A nano program's damage and heal lines add the reader's own figure in brackets when its healing or damage percentage is not 100, as the original does for nano spells. Not written: a hit on a map (the map names are a table in the original program).

### Walking away from the bank or a corpse (INV-051)

The original closes an open bank when the character has walked more than 5 m from where it stood when the bank opened, and tells the server (Gamecode FUN_1003f8fc, FUN_1005a367: `CharacterAction` 0x6a); an open corpse or chest that is not one's own closes when it is out of the 5 m reach (FUN_1005a1e7), told with `CharacterAction` 0x8e naming it. A team (or raid) mate's full update is named to the server with `CharacterAction` 0x80 (FUN_1006610b). The client now does all three; the corpse's own size is the client's default body sphere (a guess), and the reclaim terminal's case is not done.

### More sections on item pages (INV-054)

The original's item pages (GUI.dll FUN_1003882b and the other builders) also show the shop's price on a page opened from a shop's stock (FUN_10031fda), a fixture's note (`IsFixture`, FUN_10032092), the insurance cost (item stats 362 and 61 with the reader's level), a building's upkeep and size from its house template (FUN_10032d30), a tower's or controller's type in Roman numerals (FUN_1003210c, FUN_10031ae0), and an NF crystal's formula with the nano's page. The client's pages now have all of these; a shop's stock item opens the client's own `shopitemid://low/high/ql/price` page. Not shown: a nano's radius (its source in the original was not found).

### A trade across dungeon rooms (INV-055)

The original allows a trade only within 5 m between the two bodies, and in a dungeon, with the two in different rooms, only through an open door between those rooms (Gamecode FUN_1005a1e7, N3's `FindDoorIDFromRoomLink` / `IsDoorOpenBetweenRooms`); else it says the target is out of range for a trade. The client now checks the rooms too.

### Statel zones read as the original reads them (RND-140)

N3.dll reads a room playfield's statel zones (n3StatelController_t::EnableRoomStatels 0x10025ae8, loader FUN_100274b1 -> FUN_10027f16) in the room layout: the statels' packed 565 colours, two lists of lights, then the statels and the light objects (FUN_10026e66); an outdoor playfield's (FUN_1002739d -> FUN_10027cb2) in the tile layout: the global statels the zone holds, then the statels and light objects. The client now chooses the layout from the playfield's definition (rooms or not) instead of trying both, reads every part of a zone, and hands each room statel its colours as FUN_100271b6 does (two-word counts from data version 10). Room zones of playfields 1186, 1187, 4221 and 9070 that the old guess read in the wrong layout now give the right props. The colours and lights are read but not drawn yet (the renderer instances a mesh's parts for all its statels and has no local lights).

### Cast effects: no cords and no projectile, as in the original (NAN-038)

The original's cast effect (`_GfxControlSpell1_t`) also has hand cords and a projectile and impact phase, but none of them ever shows: the cords are made at a point and their flag word (5 in every cast) leaves their locator moving instead of laying links (Gamecode FUN_100d6420, FUN_101067af, FUN_100d69d8), and phases 3–4 run only in templates 9000 and 9002 (FUN_100f3f30, word 33), which no nano program or item names. The client draws the hand flares and the beam only, as the original does; a game-data test checks the data.

### Effects at the posed body's attach points (NAN-028)

The original moves each effect's locator every frame to its attach point on the posed body (Gamecode FUN_10106591 / FUN_10105e6d: attach 0 the body's own frame, 1000–1018 the bone of that name, 2000–2023 the attractor of that name, else the head), and the cast effect's hand parts follow the caster's left and right hand attractors (2001, 2000; FUN_100f331d, FUN_100f3f59, FUN_100f2bab). The client does the same wherever the character's pose is known (`effects::body_place`, `cast_particles_on`, `visual_particles_on`, `pulse_particles_on`, `Shot::particles_on`): cast flares and the beam come from the hands as they move, and the sparks, stars, smoke and lightning of hits and running effects stand at their bone or attractor; the standing-body figures are used only when no pose is known. The tracer's ends (1.1 m and 1.2 m above the feet) are still the client's: the original's tracer points arrive through the server's effect triggers.

### Friends list: unread tells, folder colours, chat windows (CHT-050)

The original keeps each character's tells in a FriendListNode_c with a count of unread ones (+0x34, FUN_100a7394). The count rises with a tell from or to the character while their tell window is shut and no chat window that is up shows the tells' group. Opening the tell window reads them (FUN_100a7ca7). The client counts the same way (`ChatWindow::unread`). In the friends list (FUN_100aa315) a row flashes while its character has unread tells or an invitation waiting, and is selected while its tell window is open. A folder's label turns red (0xff6666/0xc04c4c) when a row with unread tells is put into it (FUN_100a87fe). It is set again as a row is taken out (FUN_100a86f6, on every change of a row), and that pass whitens every folder before the row's own: the client replays these events (`friends::recolour`), quirks included. A left press on a character opens or closes the tell window. "Chat Windows" lists every chat window by name (FUN_100a9fff): the left button shows or hides it, the right button opens the window's menu (FUN_100a8a69). The menu's click-through, deactivate-on-send, default-window and log-window entries are drawn but disabled. A channel's name in a chat line is a `chatgroup://` link (FUN_1009bc20). A plain press makes the window talk to that channel when the window shows it; otherwise it types `/group "name" ` (FUN_100ac4f0).

### Private chat group invitation (CHT-051)

An invitation (FUN_100a7986) marks the inviter's row in the friends list, which flashes; the inviter becomes a recent message (FUN_100a6487). With `ChatPGInviteAction` 1 the question comes at once; with 2 it comes when the marked row is pressed (FUN_100a76c9, which takes the mark off); 0 declines with a line, and an ignored inviter is declined without a word. The question (`InviteDialog_c`, FUN_100ae840) shows the inviter's name, a list of the chat windows by name where several can be chosen (those already showing the group start chosen), and Yes/No. Yes puts the group into the chosen windows and out of all the others, then joins (FUN_100a74cb).

### Shop lists are item-container lists (WIN-035)

The original's trade view (GUI.dll FUN_100e121e) puts an ItemContainerView_c (FUN_100ce55d) in each dock (FUN_100e07b9, FUN_100e0942, FUN_100e0b00): a grid of item cells by default, sorted by name, with Icon/Name/Count/Price/Quality columns in list mode (no Price in one's own list or in a player trade), the stock's icons tagged with their price in yellow (FUN_100406a4), the bought and sold lists one row of cells high (SetViewCellCounts). The client builds the docks the same way on gui::multilist; the window's menu (FUN_100e03c7: List Mode, Auto Arrange) switches the stock list (a player trade: the partner's) and its order, kept per ShopViewConfig/TradeViewConfig. The window starts at the view's own 119 by 452 and can be resized.

### NPC give-trade bar (WIN-030)

The original's NPCChatTradeBar_c (GUI.dll FUN_10057d8b) sits in its own small window with a "Trade" tab, docked to the right of the NPC chat window (FUN_10059236, Window::DockWindow(2)): "GIVE ITEM(S)" over an item grid of the NPC's slots, "GIVE CREDITS" over the credits field, and V / X. The NPC's request replaces the answer list. The client now builds the bar the same way: drops go in while there is room, a click takes an item out, Shift-click shows its page, V gives the items and at most the credits one has (the bar stays up), X or Give again declines and closes the bar.

### Key bindings tab (WIN-033)

The option window's "Key bindings" tab is GUI.dll's `BindView_c` (FUN_100bcfb3, `gui/bindview.rs`): a Category / Function / Key list of every rebindable action this client has, one row per key ("Inactive" without one), the keys in aqua with the original's key names, sortable by any column; "Reset All" puts those actions' default keys back, "Add" binds one more key, "Change" replaces the row's key, "Clear" unbinds it. Add and Change open the original's "Bind Key" box, which takes the next keys pressed (with Ctrl, Alt, Shift) until OK or Cancel. A key bound here stays on any other action it is on (the original's bind). The keys are this client's own and are kept in `mao.ini`. "Fixed keys" stays read-only, as the original's is with the game's and the skins' files (their group is `mode="fixed"`), and writes its keys as the original does (aqua, "No Key").

### The original's key editor (SET-032)

See "Key bindings tab": the editor of GUI.dll's `BindView_c` over this client's own keys. Not listed: the original's providers this client has no action for (pets, team members, per-bar shortcut keys, the team/raid/research/knowledge/LFT/browser/item shop/daily login windows, next/previous camera view, copy to clipboard, switch target, create reference) and the second (`GLOBAL`) movement set, the same actions here; mouse buttons cannot be bound.

### Rollup area (WIN-034)

The column down the right edge is the original's `RollupController_c` (`rollup.rs`): 192 pixels wide, its pages touching one under the other and scrolled by holding the left button on a page and moving. Each page has the original's header on black (the "i" menu button, the title, the roll and close buttons), its window 3 rows under it as tall as the page's own height, and a dark 5-pixel grip that sizes the page when dragged; a double click on the header rolls the page up or down. A right press on a page picks it up; the next click puts it down: on the column before the page there (or last), on a dock window's tabs into that window, anywhere else as a window of its own there. A dock window's tab carried (right button) to the column becomes a page there. The menu button shows the window's own menu, or "Close". The black behind the column follows the option panel's rollup fade level. Heights and rolled state are kept in `rollup.ini`, at first taken from the character's own `RollupArea.xml` of the original. The title-bar dock button remains only on Faction, Pets and Stats, which are not dock windows here.

### A seated character's movement keys (MOV-014)

The original lets every movement pass a gate first (`N3Msg_MovementChanged` 0x10018bc4 asking `CharMovementStatus_t`'s slot 3, FUN_1006c6f4): in the sit mode forward, back, the sidesteps and the jump are refused, the turns and getting up are taken. The client does the same (`seated_keys`), and like GUI.dll's movement slots (0x100289f5 and its twins, AFCM 0xe3, `MovementWhileSittingMessage` 0x1002888f) raises the tip event `OnMovementWhileSitting` for each forward, back or sidestep key while sitting; the game ships no tip for it.

### The set-stat spell's build and profession (CMB-032)

The original's set-stat spell (0xcf22, Gamecode FUN_100a4f8a) leaves eight stats alone except on a character, where the `Fatness` (47) is set by FUN_1003e464, which also has the body remade, and the `Profession` (60) by FUN_10056552, which then sets stats 10 and 0x25 to 1. The client does the same for its own character; its body takes the new build at once.

### Nanos cast at once by their flag (NAN-020)

The original's cast time (Gamecode FUN_1005141d) is nothing, whatever the nano's times and the caster's skill, for a nano whose flags (stat 0) hold 0x80000 (the test at 0x10051591 through the nano's vtable +0x14, FUN_10003d5b). The client follows it (`timers::instant_cast`): such a nano shows no cast bar and no casting stance, one's own or anyone's. Others' cast times are still the nano's own time, since their initiative and aggression are not sent.

### The attack-check spell (NAN-047)

The original's spell 0xcf7d (Gamecode FUN_100a83d8) checks whether the caster may attack whom the spell lands on: on anything that is not a character the caster is told it needs a target, on an NPC it may not attack (FUN_10069a9f, without that function's own lines) that it is not allowed to attack it; items and players say nothing. The client does the same for its own character as the caster.

### A spell's colours on the effect it makes (RND-043)

A spell of type 0xcfd4 (Gamecode FUN_100a9159) makes its effect and hands its start and stop colours (stats 0x99-0xa0 over 255) to the effect's controls. The sparks, smoke, fire, nano streams and quads take them as their two colours; the stars, suns, global smoke, electra and plasma as the ends of their random colour range; a bundle passes them on to its children with green and blue swapped (FUN_100e5e4a). Its scale and repetitions reach no control that uses them. The client now draws such effects with the spell's colours (`Gfx::visual_coloured`); spirals, trails and flares keep their own.

### Lightning's effect pools (RND-054)

The original's environment effects come from pools made once per kind (DisplaySystem `VisualEnvFX_t::ActivateFX`, counts set by FUN_1006228f): two cloud lights, ten bolts and twenty rocks of each of the seven kinds; an effect is refused while its pool is empty and goes back when done. A refused strike makes nothing; a refused rock still flies and splashes unseen (Gamecode FUN_100afc4c / FUN_100b103b). The client keeps the same limits.

### Skin pictures drawn unfiltered (RND-076)

The original draws its GUI sprites through DisplaySystem's `RenderSprite_t` with point sampling (FUN_10079c06), switching filtering on only for the loading screen's picture (GUI.dll `SetupScreen`, 0x10017331). The client now samples the skin's sprites at the nearest texel too.

### Skinning by the first bone (RND-077)

randy31's CAT render (FUN_100543bc, drop-in `catrender.rs`) places a vertex weighted above 0.99 by its first bone alone and blends the two bones otherwise, and always turns the normal by the first bone. The client does the same; a vertex on its first bone no longer needs its second to be known. A bone past the pose's matrices leaves the vertex at rest (the original reads past its array).

### The camera's near and far planes (CAM-007)

N3's game camera (`n3EngineClient_t::CreateCamera`, 0x10007842) has a 90° view, its front plane at 0.2 and its back plane at the view distance: the `ViewDistance` option × 1000, never nearer than 50.2 (FUN_1001fc91, which also gives both planes to the fog). The client's game view now draws between the same planes, so nothing beyond the view distance is drawn; other views keep planes fitted to their scenes.

### Room heights from the tile-map (RND-115)

Whether a point is inside a room (n3Room_t::IsPosInside, N3 0x10011664) goes up to the cell type's height from the tile-map's HSTA table (RDBTilemap_t +0x8244, read by DisplaySystem FUN_10078391 as one short in tenths per tile type after the chunk's head; the files' size word counts only the shorts), scaled by 1 + 0.25 × the cell's code. The client read the chunk under the wrong name and dropped it when its size word was below 12, so every room counted as 8 high; it now reads it as the original does (heights from 2.8 to 100).

### Scripted dot stars (RND-158)

FXS's BlueprintRender draws a script's generated mesh as a point list (Visualization 0, e_Render_Dots) or a triangle strip (1). The client drew only the strip; it now draws the points too, one pixel each, as a sky backdrop: the Rubi-Ka outdoor sky's dot stars (Tweak_Rubi-Ka_DotStars.txt, 500 dots added over the sky, with the first expansion).

### Sun rays flicker (RND-178)

The environment's frame (Gamecode FUN_100b05b4) keeps a phase moving 50 degrees a second and makes sixteen slow waves from it, each at its own offset, as whole numbers of 255; the sun-ray fan (VisualSunRays_t, DisplaySystem FUN_1005a3a9) dims each ray by its wave (ray i by wave i & 7) under the horizon's brightness, the centre left at the brightness. The client now does the same, so the rays shimmer slowly instead of standing steady.

### The Connection page's time-outs (LCH-008)

The original launcher (Anarchy.exe) keeps its Connection page's two time-outs in the game's own preferences, GeneralNetworkTimeout and DimensionNetworkTimeout (written at 0x4050cb), so the game's login and entering bars run over the launcher's values. On OK it takes each field as a plain number (atoi, 0x40ecdc) without a check; at its next start it raises one below 5 to 5 (0x404bad, 0x404cc1); there is no upper limit. The client keeps the two in launcher.ini the same way: the login's bar and now also the entering bar use the page's values when saved, else the game's own (90 and 30), with 5 the least and no cap.

### Character creation's lines and the walk to the shuttle (LCH-024)

GUI.dll's TextWindowFade_t fades a line in over its first number of seconds, holds it for the second and fades it out over the first again. CharCreateModule_t makes the intro's two lines (5, 4: fourteen seconds), then, once the way down's picture has come back (state 0x44e's four-second PlainSpriteFade_t), "Docking completed" and "Continuing DNA" (StaticDoneTimerCallback 3; 6, 4), unless the camera already rests in a scene. After a character is made (state 0x4b1) the walk to the shuttle starts and a second later "Boarding shuttle" shows (1, 1); as it ends the picture goes to black in two seconds and the creation ends, the walk cut short. The camera's transition times (1.9, 3.4, 3.7, 0.3 s) are the original's. The client now does all of this.

### Statels' moving parts run on the clock (RND-177)

DisplaySystem's LODManager::Process gives every playfield statel mesh (0xf6951) the frame's time while its `animate` flag is set, and the flag is never cleared (it starts at 1; Gamecode only adds and removes meshes). So a playfield statel's moving parts always play on the clock in the original, as in the client; only the things the server announces follow AnimPlay/AnimPos.

### A window's inner border (WIN-084)

The original gives every normal window two borders (`WndBorder::SetBorderGfx`, GUI.dll 0x1015ab0e): its frame, `GFX_GUI_WINDOW3_BORDER_*` round `GFX_GUI_WINDOW_BACKGROUND` at GUIConfig_c's layer-1 alpha, and inside it a "top_level_border" of the `GFX_GUI_TAB_BORDER_*` pieces round `GFX_GUI_TAB_BACKGROUND`, in the border colour and fully solid, round the client (3 in at the sides and bottom, 24 or the frame icons + 10 down, grown by a point: `UpdateBorderSizes` 0x1015a279, `DoSetFrame` 0x10159b69). The client now draws that inner border and ground over its panel fill (`chrome::frame_shape`, `top_level_border`): with the game's skin the window's body is an even dark teal with a thin light line round it, as in the original's shots. The title row and the frame ring keep the client's own fill; what darkens the original's title row was not read.

### Where chat windows first come up (WIN-086)

Before the player has placed them (the saved place, `Window::LoadWndConfig`, still wins), the original puts its default chat window across the foot of the screen, 224 points in from each side and from 200 above the bottom to 3 above it (FUN_100971a3 through `ChatWindow_c`'s FUN_1009829a, for the window FUN_100950df makes as "Default Window"); any other chat window has a 400 by 200 client and a tell window (`TellWindow_c`, FUN_100b243e) a 389 by 219 one, both in the middle of the screen (`MoveToCenter`). The client does the same (`chatwindow::first_place`); tell windows no longer stack 26 points apart over the chat window. The original has a single tell window; the client keeps one per character.

### Action pictures (WIN-098)

Every tile of the Actions window and every action shortcut is pictured by its action's item template's icon, as the original's are (Gamecode FUN_10042b17 gives each special action the template that names and pictures it). Reload now shows its own picture rather than Aimed Shot's, and the catalogue's perks their own rather than a family picked by words in their names. A shortcut saved with an older picture is drawn with its action's own.

### Which planet map the window shows (WIN-101)

The original's planet map window (`PlanetMapView_c`, FUN_1004d92d) shows one map: the Shadowlands one (`ShadowlandMapIndexFile`) when bit 0 of the character's ExpansionPlayfield (stat 531) is set, else Rubi-Ka's (`PlanetMapIndexFile`). The game's own client sets that stat on entering a playfield, 1 for a playfield whose record marks it a Shadowlands one (Gamecode FUN_1005997f, `IsShadowlandPF` 0x10121e59). The client does the same (`planetmaps::expansion_playfield`, `window_kind`): in a Shadowlands dungeon the window now shows the Shadowlands map; before, a playfield on neither map showed Rubi-Ka's.

### Selected text (WIN-103)

The original's text views draw selected text on a light grey ground: `TextRenderer_c::_RenderString` (GUI.dll 0x101602bb) clears each selected glyph's cell to 0xc0c0c0 and draws the glyph over it in its own colour. The client's interface text views (reading and editing) now use that grey instead of a translucent blue. The client's band still runs to the view's edge on the middle lines of a selection, where the original shades only the glyphs.

### Where a tower would stand (WIN-118)

While the tower build menu holds an item, the original's playfield map (with its land-control button on) marks where the tower would go: the map's circle in light grey (0xdcdcdc) three units in front of the character (`PFMapRenderer_c` FUN_100ea243; the flag is the map data's +0x75, set by the menu's item setter FUN_1005c605 through GlobalSignals +0x160 and cleared when the menu or its item goes). The client draws the same circle on its maps while the menu (Build or War) is open with an item in it (`TowerWindow::placing`, `automap::placement`).

### The mission booth's offer menu and the new-mission tip (WIN-134)

A right click on an offer in the mission booth's window opens the original's menu (`MissionSelectionView_c`, GUI.dll FUN_100d0f79): "Look at Mission" shows the offer's page in the info view (FUN_100d042b: its text, pay and reward items, as the journal's page of a mission), and "Upload to map", when the offer names a place, puts that place on the map and the compass again (FUN_100d08be). A new mission is reported to the tips of the day as "OnReceiveMission" (`SlotGotNewMission` 0x100c742e), as the original does before it uploads the mission's place.

### The notification window's place (WIN-136)

The original's notification window takes the frame its saved `NotificationWindowConfig` keeps (FUN_1000ba68, `Window::LoadWndConfig`), else the whole width and the top third of the screen (FUN_1000b9df). The client now reads that frame from the original's layout for the character (`aoprefs.rs`), scaled to the screen, as it does for the other windows. Links in the lines (clickable by default in the original, FUN_1000bbaa) are not ported yet.

### Labels on XML power bars (WIN-137)

A `PowerBar` element's `label` is written centred over the bar, and `left_label` with `right_label` at its two ends, in font 5 (the original's `PowerbarView_c::SetLabel` 0x1013f37c and `SetLabels` 0x1013f593: text views between stretching spacers). No skin of the game's gives these; a custom skin's XML windows now show them.

### The tradeskill window's build text from the server (WIN-148)

The server's `FormatFeedback` lines (`FormatFeedbackIIR_t`) carry a kind in their last word: kind 1 is not a chat line in the original but the tradeskill window's build text (Gamecode FUN_1005aff0 emits it on GlobalSignals +0x1d8, `TradeSkillView_c` FUN_100de45e sets `BuildFeedback`): the "you need at least ..." lines of a combine. The client now puts them there (`tradeskill::Event::Feedback`) instead of the chat; the other kinds stay in the chat. The window's other global slot (+0x1d4) does nothing in the original.

### The rollup column's wheel and scroll (WIN-034)

With the options' mouse wheel set to scroll the side bar (`MouseWheel` 1), the wheel over the world scrolls the rollup column 40 points a notch, as the original's `RollupController_c` does (FUN_10048822 connects its `WheelMoved`, FUN_100487aa, to the global wheel); 0 still zooms the camera and 2 does nothing. The column's scroll is kept between sessions, as the controller keeps its `scroll_offset` (FUN_10048e92, FUN_100489f9): a line of rollup.ini.

### The wheel moves only vertical bars (WIN-078)

The original's `ScrollView_c::WheelMoved` (GUI.dll 0x10142820) scrolls only along the vertical: a notch is the view's wheel step (`SetScrollWheelStep`), or with snapping the notches add up and the scroll goes to a whole multiple of the step; a view that cannot scroll (no vertical bar, no step, or its content fits) passes the wheel to its parent. A horizontal bar is never moved by the wheel in the original either, so the client's vertical-only wheel (`gui/scroll.rs`) is the original's, not a simplification.

### Where a click in a text view chooses a link (WIN-012)

The original takes the clicked point to a character of the text (`TextRenderer_c::GetBufferLocation` 0x101622f6, 0x101617f0: the line at the point's height, then the first character whose right edge is past the point) and uses that character's link (`GetHyperLink` 0x10162f14, `FindTextAttrib` 0x1016132a). The client now does the same with each line's spaces and words: a space inside a link is on it, a click left of a line's text is on its first word's link, past a wrapped line's end on the next line's first word's link, and every area is the line's whole height (pictures too). Before, the client stretched a link's words over the gaps between them, with no area beside the line.

### Nano programs' school buttons in rows as wide as the window (WIN-016)

The original's `SchoolSelector_c` (FUN_100d6051, on resize FUN_100d7244) lays its school buttons left to right, 6 pixels apart, each at its preferred size less 8 by 2 (FUN_100d6df0); a button that would come within 3 pixels of the width starts a new row 5 pixels below, and each row is spread over the width. The program grid takes as many columns as its width holds. The client did two buttons a row and four columns docked, one row and six columns free; it now lays them as the original at any width (`nanoprograms::flow`). The selector asks only for its widest button's width, so the free window can still be narrowed.

2026-11-21 (the user's screenshot: the buttons stood one under another): the window's vertical layout held each child to its largest size, and the selector, a view with no layout of its own, had its least size as its largest, so it was only as wide as its widest button and `flow` gave a row each, centred. The selector now may be as wide as the window (`max_extend`), as the original's is, and the buttons flow across it (seen in a preview: two rows of three). Its frame's menu button (`WndBorder::SlotIconMenuOpening` 0x1015b34f fills the frame's popup as it opens; `NanoView_c`'s FUN_100d4f90, which fills a popup it is handed and has no direct caller, is the window's menu) now shows the window's menu, As a list, Show the hidden ones and Order with the sorting, then Close Window, as the inventory's does; it showed only the frame's own two entries. Not clicked live.

### Scroll bars drawn as their pictures are (WIN-018)

The original's scroll bars are made in code (`ScrollView_c::_UpdateScrollBars` 0x10142b68, `ScrollBar_c::ScrollBar_c` 0x10141f27) and never given a colour: a view starts white (FUN_1014fc2a), and so do the bar's picture surfaces (`ViewSurface_c::_Initialize` 0x1015208d) and its knob's `BitmapView_c`. The client tinted the XML views' scroll bars with the theme's accent colour; it now draws their pictures untinted.

### Windows move by the strip under the title (WIN-021)

In the original a window is moved only by pressing in the strip under its top border (`WndBorder::HitTest` 0x101596b7, item 1; the rect from `WndBorder::Layout` 0x1015a4ba: the top border's width, from one pixel under it down to 18). The client's skinned windows were dragged from anywhere on their body (egui's fallback for a window without a title bar); now egui's drag-from-anywhere is held in place and the strip moves the window. The pin, close and frame menu buttons over it still work.

### Bitmap fonts at the original's line height and advances (WIN-040)

The original reads each `.fnt` as 255 glyph rectangles and a width table (`FontSystem_t::FontSystem_t` 0x1012eef1, each entry plus one); a font's height is the rectangle of 'a' (`FontInfo_t` +0xff0), used as each word's height with no gap (`TextRenderer_c::_UpdateWordSizes` 0x1015dab9), and a character advances by its table entry (`FontInfo_t::GetGlyphWidth` 0x1012eada, -1 above 0xff). The client drew the bitmap fonts with the glyph height plus 4 and advanced by each rectangle's width; it now uses the original's height (9 for the chat and tooltip fonts, 12 for the shell font) and the width table, and draws glyphs from the line's top.

### Faction bands from the game database (WIN-053)

Gamecode reads the faction bands at start from the database's 0xf4265 records (FUN_100c6914, FUN_100c24b8: low, high and two texts each) into a map by instance, which `N3Msg_GetFactionRange` (FUN_100c246b) walks; a band's place picks its title from Gamecode's own table. The client's faction window now reads the same records from the game's database instead of twelve bands written into its code.

### Timer bars with the skin's pictures (WIN-058)

The original's `TimerBar_c` (FUN_100519d6, the logout bar and the game's attack, nano and other timers) is a `PowerBar_t` (0x10020b9f) of the skin pictures 0x1a8 and 0x1a9 (`GFX_GUI_TIMERBAR_EMPTY` and `_FULL` by AFCM's table, 113 by 10), tinted with the bar's colour: the full picture's left part to the level (`Recalculate` 0x100208f8), the empty picture's pixels after it, and the label over the bar in black (a `TextLine_t` at (0, 1)). The client drew plain 128 by 16 rectangles and put "Logout" above its bar in white; it now draws the pictures (`ui::timer_bar`) and the label over the bar for all of them.

### A long popup menu has its scroll bar (WIN-064)

The original holds a popup menu in a scroll view with the vertical bar on auto (`PopupMenu_c::Go` 0x1013c13a, `ScrollView_c::_UpdateScrollBars` 0x10142b68): a menu taller than the screen less 100 shows the `GFX_GUI_SIMPLE_SB` bar at its right and is that much wider (`CalculatePreferredSize` 0x10142a4b). The client's popup scrolled only by the wheel; it now has the bar: arrows step 5 and repeat when held, a press beside the knob steps 20, the knob drags, and pressing the bar chooses no entry.

### Selected list rows in the original's solid colour (WIN-069)

The original fills a selected list row with a colour-only surface of 0x88aadd (FUN_100f1f68), drawn solid: `ViewSurface_c::_Render` (0x10152beb) uses the view's alpha and a `ViewArea_c`'s own alpha starts at 1.0 (0x10151d76). The client blended the colour at 0.45; it now fills the row solid.

### Crowded tab strips shared out as the original (WIN-071)

The original's tab strip (`TopBorderView_c`, FUN_10147438) gives each tab a least width (its label's first letter, FUN_101463a4) and its own (the whole label), and shares the strip between them with `View::SpaceOut` (0x1014a44a): each its least plus an equal share of what is over, a tab reaching its own width held there and the rest shared again, the remainder going to those still under; widths are rounded and kept between least and own. The client scaled all tabs by the same factor; it now shares the room as the original, so short labels keep their width while long ones give way, and no tab goes below one letter's width.

### The pet window's options on its frame's menu (NPC-004)

The original's pet window (`PetWindow_c`, FUN_1007e9d7) puts its options on the frame's icon menu, built each time it opens (FUN_1007e75e): "Show buttons", "Fixed Pet Buttons" and "Remember Pet Names", each ticked as it stands. The client had its own menu button with check boxes inside the window; the options are now on the frame's icon menu as in the original, followed by "Close Window".

### A corpse's name read from its announcement's fields (NPC-022)

The original reads a corpse's announcement (`CorpseFullUpdateIIR_t`, Gamecode FUN_1009fa4c through FUN_100a1661) field by field: the dynel's identity, its place and turn, a word, an identity, two bytes, the stats, then the name as a length and its bytes. The client found the name by scanning the message for the first string that looked like one; it now reads it at its place.

### NPC templates read only as the original reads them (NPC-024)

Gamecode reads a creature's template (type 0xfde97) section by section (FUN_1004de5d). The client had a second, guessing reader for records that did not read so; a pass over all 1470 of the game's templates found none, so the guess is gone and the pass is kept as a test.

### A machine with no name is "Name missing" (NPC-027)

Gamecode names an item (FUN_100809a0) by its own name, else by its NameTemplate's (stat 446) template's name, else with "NameMissing". The client fell back to "Booth" and the machine's number; it now uses the original's word.

### Friends coming and going (CHT-010)

The original writes nothing in the chat when a friend logs on or off: the chat library's S2C_ADD_BUDDY handler (GUI.dll FUN_1016d6f4) updates its buddy list, and FriendListController_c (FUN_100a6581, FUN_100a72c5) and FriendListView_c (FUN_100a8a4c) move the friend between the online and offline rows of the friend list. The client now does the same; its own "is now online" line is gone.

### Lines typed while the chat is down (CHT-011)

The original's chat requests wait in ChatGUIModule_c's queue, which its tick (GUI.dll FUN_1008a2c1) works through only while the chat socket is open, and the chat library holds a message until it has logged in (FUN_1016c84a → FUN_1017265d). The client now keeps what was typed while the chat server was away and sends it after reconnecting, instead of dropping it.

### Plain copies of chat lines (CHT-021)

The chat window draws a line's markup with the port of HTMLParser_c; the plain copy the client keeps beside it (for tips, logs and filters) now reads the entities as the markup reader does: one pass over Utils.dll's table (nbsp, quot, gt, lt, amp) and the numbered characters (String::GetUCChar FUN_1000db51).

### Filling the game's texts (CHT-027)

LDBformat::Feed prints each `%` piece of a game text with snprintf and the piece as the format, and Dump joins the pieces with the leading spaces dropped and each run of spaces made one (the ldb drop-in's format.rs). The client now does the same: widths, flags, precision, hex, characters, floats (msvcr100's three-digit exponent) and times (" 1 day 2 hours ..."), a number in a `%s` as `int_value<n>`, and the spaces collapsed. Only a text fed into a number's place still prints the text, where the original would print its address.

### A speaker whose name is not known yet (CHT-028)

The chat library's id-to-name map gives an empty name for a character it has not been told about (GUI.dll FUN_10014b93), and the line writer FUN_1009bc20 then writes neither name nor ": ". The client now shows such a vicinity line as its text alone and a group line as the group and the text, instead of the speaker's id in hex.

### /viewdist, /chardist and /char&viewdist (CHT-031)

GUI.dll FUN_100b69a7 sets the `ViewDistance` preference to the number times 0.01, FUN_100b68dc the `DisplayCharViewDistance` value, and FUN_100b6a54 both; nothing is said and nothing is limited, and with too few pieces an error line is written (FUN_1009bad0). The client now does the same: `/viewdist 60` is the option window's 60 % (600 m), `/chardist 40` 40 m (anything outside 5 to 80 is 70, as N3.dll maps it), and the option window shows the new values.

### Fading chat lines (CHT-034)

GUI.dll's ChatTextView_c fades a line by its frame timer (FUN_100926f6): `ChatTextFadeDelay` and `ChatTextFadeTime` in whole milliseconds (FUN_1008f8e7), the alpha the time left over the fade time once the delay is past, and the line taken away after both. The fading follows the window's "fade messages" setting alone, typed in or not. The client now does the same; before, it did not fade a window that was under the pointer or typed in.

### Script words (CHT-041)

GUI.dll's script loader (FUN_100b1517) cuts the command line with Utils.dll's `String::Tokenize`: blanks separate words, and a word that begins with a double quote runs to the closing quote and keeps its quotes. The client now cuts a script's command line the same way, so `/myscript "two words" x` gives `%1` = `"two words"` and `%2` = `x`.

### Sent lines and how many lines a chat window shows (CHT-042)

The original keeps the last 200 lines sent for the arrow keys (FUN_1008f5b5: a line sent again leaves its old place and goes last), and each chat view shows at most its last 200 lines (ChatTextView_c, FUN_10091d8c). The client now does both; its one shared list of 600 lines behind the windows stays.

### Disbanding one's own private group (CHT-060)

The original's menu on a private group's row in the chat settings (GUI.dll FUN_100a1102) offers "Disband" for the character's own group and "Leave" for another's; disbanding sends the chat server packet 54 and takes the group out of the windows (FUN_100852e8). The client now has both entries; before, it offered only "Leave", for any group.

### Ignored players in vicinity and groups (CHT-067)

The original's `HandleVicinityMessage` and `HandleGroupMessage` drop a message from an ignored character, text and voice, unless the message's records carry a flag with bit 0, which lets it through. The client now does the same; before, an ignored player's voice line still played.

### How many pieces each command's line is cut into (CHT-068)

Every command GUI.dll registers for a player now has its piece count in the client's table (FUN_100bb3e0, FUN_100b2ac9, FUN_100b4001, FUN_100b348c). `/org` and `/team` are not registered there: their lines go whole to Gamecode's text commands (N3Msg_TextCommand → FUN_1003fe10), which read the words at blanks, as the client does.

### The key editor's last parts (SET-032)

A key bound to several of the original's actions now does them all, as GUI.dll's input (FUN_10018376) does, and only when none takes it does a fixed key act. OK in the Bind Key box with no key pressed binds the original's "no key", a row of its own kept in mao.ini as `Inactive`. The original's team, raid, LFT, research, knowledge, daily login and item shop window keys are actions with the shipped keys (Ctrl+5, Ctrl+Shift+R, Shift+F, Shift+O, Ctrl+3). Mouse buttons as keys and the team, pet, camera-view, reference, switch-target, copy and per-slot shortcut keys are not done yet.

## An item in hand let go over the world by either button (2026-11-20)

The user: "right click when holding an item still doesn't work". GUI.dll's `WindowController_c::HandleMouseUp`
(0x10157f13), with a drag object in hand and no window under the pointer, tells the drop (its signal +0xc, FUN_100cbcb5:
use on a character, on an object, or the ground's boxes) on the release of any button, then cancels the drag. The client
let go only on the left button's release, so a right click over the world did nothing. Now either button does
(`gear::letting_go_on_world`), and the hand is empty after. Over a window nothing changed: a right click on another item
is still `UseItemOnItem` (the lists' own handler, FUN_100cae1b).

## Combat text over characters (2026-11-20, CMB-038 corrected)

The removal of the client's floating numbers (CMB-038, 2026-11-19) rested on a wrong reading: the original does float
combat text. Gamecode's fight-line formatter (FUN_10012c3e) ends each event that has an amount by making it (`"%d"`) over
the event's character as effect 0x2f5a (FUN_10011171), a `_GfxControlFont_t` text effect, coloured by the event from the
table at 0x10157b08. The client now does the same (`World::combat_text`, `ui::CombatText`):

- **Where and how:** centred 1.5 above the character's place, 0.12 high (12 glyph pixels at word 28's 0.01), rising 0.4 a
  second (word 29), gone after 1.3 s (word 26); over one's own character only with the third-person camera.
- **Colours, by event** (`feedback::text_colour`): your hits light grey; hits on you red; others' hits olive (orange when
  your pet is in it); nano damage to you white; your nano damage pale blue; healing light blue; experience,
  alien experience and shadowknowledge yellow.
- **A guess:** the glyphs are the effect's material's, placed by a table Gamecode fills at run time; the client's own font
  stands in for them.

Not ported: the first-person message, and the special attacks' names ("Burst!" and the like) over the attacker (CMB-033).

## The long tail of low-weight entries (2026-11-20)

Six readers went through the catalog's open low-weight entries in parallel, each in its own worktree, and ported what the original's code settles without the live game. None of it was seen in a live game.

### A frame's saved animation matrix (RND-143)

The original's frame reader keeps a frame's saved `anim_matrix` even when the frame has an animation (it goes into the animation's holder, randy31 `refframe.rs`) and reads no keys until the animation is played. The client's mesh reader does the same; it no longer carries an unused reader of the animation's first keys.

### A body's torso sphere (RND-149)

A character mesh's packed word, once thought a colour, is the torso sphere: the original reads its top byte and then its low three through a curve (`0.05 b + 0.000006 b³`, FUN_1006aab4) and the body keeps them as the torso's radius and centre (`VisualCATMesh_t::GetTorsoSphereRadi` 0x10072c22, `GetTorsoSpherePos`). The usual body's sphere is 0.25075 across the radius; big creatures' run to 5. The client decodes it (`CatMesh::torso_sphere`) and carries it, through the body's scale, among the body's frames for the effects that size themselves by it: a sky flash whose lower radius is negative is that many torso radii (FUN_100ef80f), where the client took 0.5.

### Effect meshes at the feet of a scaled body (RND-152)

With flag 0x200 an effect mesh grows by the body's scale (FUN_1010ce9c: `GetBodyScale`, whatever the locator). The client did so only on attractors; a mesh held at the feet of a bigger or smaller body now grows or shrinks with it too.

### Fire effects' gate (RND-157)

The original's fire effect makes its flames only in the frames where `rand()` masked by word 31 is zero, the ones held back all at once at the next frame let through (FUN_100dbed5). The client does the same with its frame-numbered stand-in for `rand()`: rage flames with a mask come in small bursts rather than a steady stream. The attractor frames are not followed yet.

### Spirals sized by the body (RND-174)

The Spiral2 effect's radius, rise and turn are scaled by the body's torso: its torso sphere's radius over the usual body's 0.25075, times the body's scale (FUN_1011230d, used by FUN_10111fdb and FUN_10111c40). The client carries each body's torso sphere among its frames and sizes the spirals by it; a usual character at its normal size is unchanged.

### Sneaking backwards plays the sneak reversed (MOV-010)

The original (Gamecode FUN_1006c370) plays the sneak action with its reverse flag when a sneaking character moves against its facing; the walking and running modes have back animations of their own. The client now sets the sneak cycle in the animation stack's backward direction while sneaking backwards (`movement_backwards`), and forwards again when the character turns to move ahead.

### The ground's line test walks the tiles (MOV-011)

The original's tile-map ground test (N3.dll `n3TilemapSurface_t::GetLineIntersection` 0x10018b72, `Intersect_Tile` 0x1001891d) walks the tiles a segment crosses in order and takes the first tile hit inside the segment; the client now does the same walk (`vehicle::tiles_crossed`) instead of sampling the segment every half tile, and prefers the ground to a zone hit at the same distance, as the original does.

### The ground's idle loops (NPC-012)

In the original a character in the sitting-on-the-ground mode 8 plays the ground's idle 0xd7 looping (Gamecode FUN_1006c5ae through FUN_1003c6f3), and the standing idle 0x78 when its body has none. The client now loops it too (the Wounded Dockworkers' `idle-sleep-ground`, players announced sitting) instead of holding its last frame, and a creature without a ground idle stands in its idle instead of holding the end of sitting down.

### A bank item let go on another container is refused (INV-060)

In the original (GUI.dll FUN_100cc855) an item taken out of the bank can only go to the inventory: let go on a bag's, chest's or corpse's window it stays in the bank and the game says ItemsDraggedFromBankCanOnlyBeDroppedToBank (text 10003). The client now says that line in the same case (the wear cells not yet).

### More requirement lines (INV-071)

The original's criteria evaluator (Gamecode FUN_100065d8) says a line for a failed comparison when asked to; the client now also says the original's lines for the equal and unequal comparisons on teams, fighting, sitting, combat mode, controllers, the Shadowlands, the place behind or in front of the target, weapons that cannot be wielded, and a side or profession refused.

### Tower updates of other kinds (ZON-023)

The original (Gamecode FUN_1012b350) treats a tower update whose kind is neither 1 (destroyed) nor 2 (planted) as a plant of an empty tower (the message's own, zeroed by TowerProxyBase_t's constructor FUN_1012b9d9). The client now adds that empty entry at the front of the playfield's list too, instead of ignoring the update.

### The knowledge board grows (WIN-057)

The original's knowledge board is a grid MultiListView_c: a drop on a cell already holding another item goes to the grid's first free cell (FUN_100cf299, GetFirstFreePos), past the last row when every cell is taken, so the board grows. The client's board does the same now: it is never full, shows as many rows as its items reach (scrolling past the window's five) and keeps the grown rows in knowledge.ini. Its six columns stay, the window not being resizable.

### Renaming a chat window (WIN-066)

The original asks a chat window's new name in a text input box (FUN_10099d7a: DialogBox_c with a TextInputDialogForm_c, 0x1014851a): titled "Enter window name", the prompt over a field 150 wide holding the window's name, OK and Cancel; Enter is OK. A name another window already has brings the box back saying so ("WindowWithNameExists", FUN_1009983b), with the typed name kept; the window's own name or an empty one changes nothing (FUN_10094950). The client's rename box is now that box (`chrome::text_dialog`, usable for other text questions), with the same answers; the name is taken as typed, not trimmed.

### Missions of one's own character (WIN-096)

The original adds a mission update's missions to the list of the character the message names (Gamecode FUN_100acfde) and says "You got a new mission" for each new one only when that character is the player's own (FUN_10056cdd: the +0x140 flag SetMainDynel 0x10019fdf sets). The client's journal now takes the update only when it names the player's character, and the line comes once per new mission.

### The planet map's opening zoom (WIN-099)

The original's planet map view starts at its first zoom level, or at the level kept in its PlanetMapViewConfig (`ZoomLev`), held to the map's levels (FUN_1004d92d, FUN_1004c2e2). The client's planet map window now does the same: it opens at the level last zoomed to (kept in layout.ini), else the first, never past the map's last level; it no longer jumps to the second level for a new map.

### Research requirements as the original writes them (WIN-139)

The research window's description of a goal is Gamecode's N3Msg_GetPerkDescription (0x10029d14): the description, the modifiers, then under "Requirements" the criteria of the template's wear action followed by those of its use action as N3Msg_GetCriteriaString writes them, with no "Wear:"/"Use:" headings, and nothing when the template's MoreFlags has bit 4. The client now builds it so; before it showed the use-by string with each action's heading.

### Skills held for a while cannot be raised (WIN-142)

Gamecode's skill maximum is 0 for a skill a temporary list (spell 0xcf76, e.g. riding a Yalmaha) holds set, with the line "Feedback_CantChangeTemporarilyChangedSkill" (FUN_100656fb, FUN_100642cd). The client's skills window now refuses to add points to such a skill, saying that line, and the IP suggestion passes it over.

### The camera on slopes (CAM-009)

The original's camera never gets pulled towards the character to clear the ground: its vehicle settles against the playfield's surfaces (Vehicle_t::EnsureSurfaceAlignment). The client used to shrink the camera's distance until the eye was 0.6 above the ground; now that its camera vehicle settles against the playfield's surfaces and tile-map ground as the original's does, that pull-in is kept only for playfields without surfaces.

### Dragging a menu's chat command makes a macro (CHT-013)

The original's menu entries can carry a `<RunChatScript value=... macro_name=...>` action (`ActionRunChatScript_c`, GUI.dll FUN_100656a4). Clicking the entry runs the line (FUN_10065798 with 0, GlobalSignals +0x180); beginning a drag on it runs its `on_begin_drag` actions (CCMenuEntry_c's list +0x1d0), and there the action makes a macro (FUN_10065798 with 1, the `/macro` signal +0x1a0) named by `macro_name`, else by the line without its slash. The client now does the same: dragging, for example, the Social menu's Bow makes a macro "bow" of `/bow`, saved and carried on the pointer to a hotbar slot like a `/macro` one.

### /voice in the window's chat group (CHT-018)

The original's `/voice` (GUI.dll FUN_100b88b1) speaks in the chat window's selected group (FUN_1009a9bd), not always the vicinity: a channel gets the line with the voice record in the message's blob (FUN_10085a42, FUN_1008a2c1), unless the channel's flags have 0x40, when it is refused. The client now does the same with the window's output group (`/ch`): the vicinity when there is none, else the channel, refused there with a line of its own words.

### Social actions refused while swimming (CHT-024)

The original's `N3Msg_DoSocialAction` (Gamecode 0x10026b4f) refuses every social action while the character swims (its movement mode 4, `Feedback_CantDoSocialActionsWhileSwimming`), before the lounge/sleep sitting rule. The client now does the same (`actions::social_refusal`); the third refusal, lounge or sleep with a vehicle equipped, reads an object of the character's (+0x2c8) the client does not know and is not done.

### Shift on a name or group link (CHT-039)

The chat's link handlers test the qualifiers' low two bits (FUN_100ac4c4 for a name, FUN_100ac4f0 for a group), which `WindowController_c::GetQualifiers` (GUI.dll 0x10156df6) sets for the left and right Shift keys; Ctrl and Alt are other bits. So Shift-click on a name types the start of a tell, and Shift-click on a group types `/group` and its name; Ctrl-click now acts as a plain click, as in the original.

### Chat lines with control characters (CHT-045)

The client used to drop any chat line whose text held a control character, a guard from its first chat parser. Packets it cannot read now make no line at all, so the guard is gone: every line shows, as the original's chat view takes whatever its parser (GUI.dll FUN_1016d6f4) read.

### Going away on a battlestation (CHT-058)

The original's `SetAFK` (GUI.dll 0x10087669) does more than keep the message: when a character that was not away goes away while on a battlestation (`N3Msg_IsBattleStation`, playfields 4380 to 4388), it leaves the battle (`N3Msg_LeaveBattle`). The client now does the same, for `/afk` and for going away by itself; the leave is the one the battle's end box sends.

### A chat window's "Deactivate On Send" (CHT-059)

The original's chat window menu (GUI.dll FUN_100a8a69) has "Deactivate On Send" (`deactivate_on_send`, on by default, FUN_1009e379): after a line is sent the window stops taking typing (FUN_100ab37b → FUN_10099084) only while it is on. The client now keeps the option for each window (from the original's Config.xml or its own chatwindows.ini) and the friends list's menu of a chat window toggles it; off, the input bar stays typed in after Enter. "Enable Clickthrough" and "Default Window" are still drawn without being choosable.

### The chat frames' places from the original's files (CHT-064)

A chat frame's place is kept in the `Config.xml` of the window whose folder the frame is named after (`window_name`, written by GUI.dll FUN_1009aecc); the windows sitting in another's frame keep none. The client now reads every frame's place so: the startup window's frame is its "Chat" window, each other frame `Chat:<frame>`, so a player's second chat window starts where the original left it, not only the first.

### Sound preferences passed on as they change (SND-009)

GUI.dll's `SoundOptionsMonitor_c` (FUN_100c573c) passes nine preferences to the sound module the moment they change: SoundOnOff, MasterVolume, SoundFXOn, FXVolume, MusicOn, MusicVolume, BattlemusicMode, VoiceSndFxOn, VoiceSndFxVolume (slots 0x100c5242 to 0x100c5329). The client's control center does the same for changes from its menus and the option panel; the battle music mode, which it used to read only at start, now follows too.

`SoundOnOff` ("Sound on/off") is `SlotPrefSoundOnOffChanged` (0x100c5329): on, `SandyInterfaceModule_t::InitializeSandy`;
off, `DeinitializeSandy` (the music player deleted, every sample stopped and released, nothing played until it is
started again; the original also closes the Miles driver), each with a `GlobalSignals_c` +0x194 signal. The client
mapped it to `SetMasterMute`, which GUI.dll never calls and which silenced the effects only; it now starts and shuts the
interface (`audio::Module::initialize_sandy`/`deinitialize_sandy`). Its mixer stays open, so `LoadSimProject` makes no
music player while the interface is shut (the original's would be made on the closed driver and be silent); the music
returns with the next district's `SetupENVSounds`. At start (`audio::start`) the nine preferences are passed on as
`SlotInitialize` (0x100c5541) passes them, from the game's defaults under the player's `prefs.xml`
(`controlcenter::apply_all_sound`); before, only `LoginPrefs.xml` was read there, so the first music and the character
creation's sounds ignored the player's settings until the control center loaded. What each control does to what reaches
the mixer is test `audio::tests::each_sound_control_reaches_the_mixer`: FXVolume and SoundFXOn the effects (a playing loop
at once), MusicVolume and MusicOn the music player only, MasterVolume the effects' total and the driver's level (which
the music's streams are mixed under too: effects get the master twice, as in the original, where `SetMasterVolume` folds it
into the effects' total and the frame sets it as Miles's digital master level), VoiceSndFxVolume and VoiceSndFxOn the
voice lines (the player effect total given with the play and again as it plays, as the original), BattlemusicMode the
combat music, SoundOnOff everything.

The option panel's other voice settings are read where they are used, as GUI.dll reads them (`GetDValue` each time):
`VoiceSndFxHearVicinityOn` (`HandleVicinityMessage` 0x10086c53), `VoiceSndFxHearGuildOn` and `VoiceSndFxHearTeamOn`
(`HandleGroupMessage` 0x1008692a), `VoiceSndFxType` (FUN_1008c472). Until 2026-11-21 `voices.rs` read them from the game's
default `LoginPrefs.xml`/`CharPrefs.xml` only, so those boxes did nothing; it now reads the player's values (`prefs.rs`), the
files only before the control center has loaded (test `voices::tests::the_players_voice_settings_are_read_as_they_are_now`).
`VoiceSndFxHearFriendsOn` ("Known friends") is read by nothing in GUI.dll or Gamecode: original behaviour, the box does
nothing there either.

Until 2026-11-21 that held only for the control center's own menus (the tester: "sound controls in settings were not working"). The option panel is drawn after the control center in a frame, and `ControlCenter::show` began each frame by putting the client's open windows into their values and then clearing the list of changed values, so it threw away the option panel's changes of the frame before with the windows' (and those of `/dvalue`, `/setoption`, the target NCU windows' close box and the "all effects" switch's thirteen, all set between two of its frames). No option of the panel reached the sound module, `prefs.rs` or `out.changed`, and `prefs.xml` was never written (the tester's settings folder has none). `show` now takes the changes waiting since the last frame first, keeps a window asked for among them, clears only the windows' own syncing, and passes the waiting changes on with the menus' (`ControlCenter::pass_changes`, test `a_value_changed_between_frames_is_passed_on_and_saved`). The option panel's "Number of shortcut bars" and "Lock shortcut bars" (`shortcutbar::follow_options`, read from `out.changed`) depended on this too.

### More windows start where the original left them (SET-023)

The original keeps each window's place in its own `*Config` archive of the character's `Prefs.xml` (`WindowFrame`). Besides the twenty mapped before, the client now takes the original's frame for the inspect, laser target, mail, mech select, nano programs, playfield map, planet map, signup terminal and teleport target windows and the compass (`aoprefs::NAMES`), checked against the original's files on the user's machine; a place the client kept itself still comes first.

### A door's mesh: its record's, else its template's (RND-008)

A door of the playfield's dynel table draws its record's own mesh (stat 12 in the record's tail) and, without one, its item template's (main.rs `static_props`); the earlier rule of this client that a door without a mesh of its own drew nothing is gone (the sentence about it under "Doors in their doorways" is out of date).

### The hit ripple's shape (RND-103)

`_GfxControlDeformer_t`'s vertex callback (Gamecode FUN_100d776f) ripples a hit body in mode 1 with a phase of the vertex's place and of t = frequency × time: `((x + 13t) × 13 + (y − 11t) × 17 + 19tz) × 11 + t`, each vertex pushed along its normal by `amplitude × sin²(phase) × strength`; mode 3 swells the whole body by `sin(time) × 0.05 × strength`. The client does both now (`effects::deform_parts`); the earlier ripple up the body with invented factors is gone. Modes 0, 2 and 4 are not done.

### Shield shells turn with the character (RND-071)

The shield shells (`GfxVisualShield` FUN_1001c94f, `GfxVisualShield2` FUN_1001d569) take their texture mapping, fade point and shimmer in the model frame, Y up with the feet the origin, turned with the character. The client's shells did that in the world's frame; they now turn it by the wearer's heading (`effects::visual_shells`). A shield landing as a shot's hit still uses the unturned frame.

### Stars mode 2 flickers (RND-084)

`_GfxControlStars_t` mode 2 (Gamecode FUN_100f8491, `case 2`) draws eight colours every frame, each between a palette entry (0x102c5ee8) and the template's second colour, and star k takes the (k mod 8)th: the burst's stars flicker. The client does the same at 30 frames a second (`effects::stars_particles`); before, each star kept one colour.

### Stars mode 3 circle the point (RND-086)

`_GfxControlStars_t` mode 3 (Gamecode FUN_100f8491, `case 3`) bears two stars a frame at random points about the point (level reach the radius, word 29) moving level across the radius; each frame a star's velocity is pulled a tenth of the way toward the point and the star moves a tenth of its velocity, so the stars circle the point and bob about its height, each drawn anew in a colour between the template's two every frame, its sheet frame running from 0 to 15 over its life. The client steps this at 30 frames a second (`effects::stars_particles`).

### Toggled effects that repeat (RND-091)

A toggle effect (kind 3036, `GfxControlToggle_t`, Gamecode FUN_10112952) with flag 0x400 makes its child again each time the child ends, for as long as the toggle runs; without it the toggle ends with its child. The client restarts such a child every one of its lives (`effects::resolve_toggles`); the vehicle-start toggles (0x2000) already restarted at each start.

### Thick clouds do not darken the daylight (RND-102)

The weather's thick clouds set `VisualEnvFX_t` +0x1134 to `1 − clouds / 2.333`, which only becomes the effect scripts' `CloudTranspariency` (DisplaySystem FUN_1005cdcf); the environment light looks the variable up but never uses it, and only Outerspace's sunlight script reads it. The client used it to dim the sky's light under thick clouds; it no longer does (the scripts still get it).

### The suns turn (RND-032)

`_GfxControlSuns_t` in mode 0 (Gamecode FUN_100fc425) shows eight halos two units above the point, sun k (k from 0) `size + k × step` wide under the envelope `1 − (2u − 1)⁴`, each turning in the view at `(k − 4) × time × 0.5` radians (`GfxVisualSol`, DisplaySystem FUN_10020836): the inner ones one way, the outer ones the other, the fifth still. The client drew 32 unturned halos; it now draws the eight, turning (the renderer's billboards can spin: `mao_render::Particle::spin`).

### A hit as a creature dies names the creature (CMB-009)

In the original a corpse is a dynel of its own and the dying creature keeps its name until its despawn, so a hit told in between names the creature (Gamecode FUN_10012c3e). The client shows the creature's body as its remains from the corpse's announcement on, but the fight lines now name the creature until its despawn (`World::died_as`), not "Remains of ...".

### /petduel's target (CMB-018)

GUI.dll's /petduel handler (FUN_100b8d78) challenges whenever the selection is a character (type 50000), oneself and NPCs included, and says its "target a player first" line otherwise; the client now does the same (it refused oneself and sent a challenge for an object). The no-target lines of /duel and /petduel are the client's own words, as the original's is a literal of GUI.dll, not a text.mdb entry.

### Alien experience at an alien level-up (CMB-026)

Gamecode FUN_100a2006 adds the held stat 178 (the experience the alien level took) to the alien experience line's amount when the same Stat message tells an alien level (169) above the held one, so the line tells what was gained (the client, which tells only a gain, had said nothing at a level-up since the value drops). The client now does the same (`net::alien_level_extra`), and tells the lines in the message's stat order: health, alien experience, experience, shadowknowledge.

### A crawling character's weapon actions (CMB-029)

Gamecode FUN_1009d964 fills a wielded weapon's animations with crawling variants while the holder crawls (WaitState 14): draw, idle and holster 0xe7/0xe9/0xe8 (a rifle 0xcd/0xce/0xcf), and small arms and rifles attack with 0xeb (0xea left hand) and 0xd1. The client now uses them for its own character while it crawls (`item::weapon_actions_as`); others' crawling is not known to the client.

### A stun stops a creature's walk (CMB-034)

The original's stun (Gamecode FUN_100a879c) also calls the character's movement vehicle (FUN_1005a02c(1)); for a creature (`NPCVehicle_t`, FUN_100713de) that is a full stop with its path and followed target dropped. The client now ends the walk the server set a stunned creature on (`World::paths`) at the stun's beginning; one's own character was already held still by its state flags.

### A stun on no character (CMB-035)

Gamecode FUN_100a879c tells the client's own caster `Feedback_YouNeedToSelectATargetForProgram` when a stun spell runs on something that is neither a character nor an item; the client now does too (`spelllist::Outcome::StunNoTarget`).

### A weapon drawn with no start (CMB-036)

Gamecode FUN_1003cce2 plays `idle-stand` (0x78) once when a drawn weapon's map has no start (class 0x1a); the client now does the same instead of nothing. The controller's busy count that holds a draw back is not ported.

### The control center's target buttons (TGT-011)

GUI.dll's two `CCTargetControl_c` (FUN_10074d26) each have a target button (slot 0x10073651): the one with the self icon selects oneself (`TargetingModule_t::SelectSelf`), the one with the other icon starts the fight (`N3Msg_PerformSpecialAction(0xb)`). The client's right button now attacks, as the attack key does, instead of the guessed next hostile.

### Every corpse by its own name (TGT-016)

`CorpseFullUpdateIIR_t`'s reader (Gamecode FUN_1009fa4c → FUN_100a1661) always reads the corpse's name; the client now shows it for a corpse that names no dead too, where it showed "Object <id>".

### A corpse's dead read at its place (DTH-009)

The corpse's announcement names its dead in its stats 415 and 416; the client now reads them from the stat list where `CorpseFullUpdateIIR_t`'s reader has it (`n3::corpse_stats`, FUN_1009fa4c → FUN_100a1661, the list FUN_1002e7d9), not by searching the bytes. Both ways agree on every captured corpse.

### Every running program's effect (NAN-007)

The original draws the effect (EffectType, stat 413) of every program running on a character; the client drew the first six only. Now all are drawn, on one's own and others' bodies (others' bodies by their announced fatness).

### Spell lists setting another's stat (NAN-022)

Gamecode FUN_100a4f8a (spell 0xcf22) sets a stat on any character, pets and NPCs included; the client now keeps it for others too (`World::others_stats`), and a Fatness set so is the build the body is drawn with.

## Remainders of part-done entries (2026-11-20, day)

Five porters took what was left of seven part-done entries in parallel, each in its own worktree, and ported what the original's code settles without the live game. None of it was seen in a live game.

### PvP grace and disgrace periods, and the server's fight-mode changes (TGT-004)

Crossing into a district of another fight mode (FUN_1003f4a3), the original gives one's character a grace period on entering a fighting district from a suppressed one (FUN_1003ed59: 60 s where the district has a land control range, else 15 s; none while attacking, `Feedback_NoPvPGracePeriod`) and a disgrace period of 60 s on leaving it for a suppressed one (FUN_1003ef7f); during either its fight mode is the one it came with (FUN_1003e43a, the dynel's +0x138 bits 2 and 16 and +0xf4). Each ends on its timer with `Feedback_PvPGracePeriodEnded` / `Feedback_PvPDisgracePeriodEnded`; an attack at a character ends the grace at once (FUN_1006a1b1). The server's `FightModeUpdate` changes districts' modes through the playfield's `FightModeHandler_t` (FUN_101250c3, FUN_1011fddd). The client now does both for its own character (`combat::Periods`, `destinations::FightModeHandler`); other players' periods are not followed.

### Dead: the away timer and the two keys that pass (DTH-014)

The original's input stop at one's own death (`DisableUserInputMessage`, GUI.dll 0x100286e2) also stops the
180-second timer of going away by itself, and its end (`InputConfig_t::EnableUserInput` 0x1001a127) starts it anew.
Every press still reaches `InputConfig_t::AddUserInput` (0x1001b115) before the frame drops it, so input while dead
restarts that timer and ends being away. And `InputConfig_t::FrameProcess` (0x1001b253) lets two chords through the
stop: the inputs 0x8002a and 0xe002f, Alt+F4 and Shift+Ctrl+Alt+F9 (pressed, not released). The client now does the
same: `afk::Away::input_off`, `afk::INPUT_WHILE_STOPPED`, `viewer::passes_input_stop`.

### A new level's notice buttons (OTH-012)

At a new level (`NewLevel`, Gamecode FUN_10075f55) the original sets the values `got_ip` (always), `got_perk` (every
tenth level, with Shadowlands: stat 389 & 2) and `got_tech` (level 5, with Lost Eden: & 0x20); a new shadow level
(FUN_10077ae7) sets `got_ip` and `got_perk`. GUI.dll's watchers (FUN_1002e9d4, FUN_1002eb2d, FUN_1002ec86) show each as
a `NotifyButton` (FUN_1005370d): the skin's `GFX_GUI_NEW_IP`, `GFX_GUI_NEW_PERK` or `GFX_GUI_NEW_TECH`, its corner 60
pixels left of the screen's right edge and 273, 333 or 393 above its bottom, unless the preference `ShowIPPerkbutton`
is off (and `got_tech` not while the research window is open). A click opens the skill, perk or research window
(FUN_100536a6), and that window's opening takes the button away (FUN_100fef4c, FUN_10061a07). The client does the same
(`notices.rs`, `Ui::notice`).

### Combat numbers over oneself in first person (CMB-038)

In first person the original does not make the floating number over one's own character; Gamecode (FUN_10012c3e, FUN_100044c2) sends GUI.dll's RenderTextModule_t a damage-text message (module 0x19, message 0x3b) instead, which DamageTextMessage (0x1004b4f2) turns into a flat screen number 50 pixels from the left (plus a random -20..20), rising 70 pixels over 2.3 s. Its height is taken from a value GUI.dll never sets (DAT_102761c0, always 0), so the number always stands above the screen's top edge and is never seen. The client draws nothing over its own character in first person, which looks the same.

### The flash on switching to fly mode (MOV-021)

The original's switch to fly mode (FUN_1006e4f8) makes effect 2000 at the lifted body in the colour 0xfff0e0ff, for any character. It is a kind-1006 flare (`_GfxControlFlare1_t`: words FUN_100ddf10, run FUN_100def53, burst FUN_100de65e, streak FUN_100de366, SetColor FUN_100dde25): after 0.02 s one burst of thirteen short streaks (0.08 s), four rounds of three fanning up and out a quarter turn apart plus one straight up, jittered, fading from the given colour to nothing. The client now draws kind 1006 (`gfx::Flare1`, `effects::flare1_particles`) and makes the flash for its own character at the switch and for another character as it is first seen in fly mode. The spells 0xcf4b / 0xcf4c begun with stat 0x19 above 0 also queue their undoing for that many seconds later (FUN_1002dfc8, FUN_1002dc9e, FUN_10059e90's events 0x80 / 0x40), which the client now does (`Outcome::FlagsLater`).

### Trade and corpse reach inside dungeons by room (INV-055)

The original decides whether a trade partner or an open corpse is in reach inside a dungeon by the rooms the two bodies' sphere centres are in (Gamecode FUN_1005a1e7: `PosToRoom` with the character's room as the hint, `FindDoorIDFromRoomLink`, `IsDoorOpenBetweenRooms`); a point in no room is in reach. The client now uses the same centres and one shared rule (`interior::RoomState::rooms_reach`) for both the trade start and the walk-away check of corpses; before, it looked the rooms up at the feet.

## The performance overlay's own hitch (2026-11-20)

The user's screenshots all read the same: frames of 7.5 to 10 ms, a "1% low" of 18 to 23 ms, the card busy 35 to 41% of
the frame, and spikes in the graph about a second apart. The overlay (`perf.rs`) sampled the processor and memory on the
frame's own thread once a second, and timed alone that sample is about 10 ms (sysinfo 0.40's refresh of the one process:
8.5 to 11.7 ms; the processor and memory 0.2 ms each), the first one 0.64 s. At about 110 frames a second the slowest 1%
is one frame a second, so the "1% low" was that sample. It now runs on a thread of its own (`perf-sample`), started by
the overlay's first frame and ending three seconds after its last; the overlay reads the last sample. Test
`the_sample_is_taken_off_the_frame`. Not measured live: what the frame times are without it, and whether other hitches
are left.

For finding the rest: `MAO_PROFILE=1` now also prints, every five seconds, the three slowest frames of those seconds stage
by stage (`slow: 33.6 ms | draw 3.1 | ... | outside 26.2`, `outside` being the time between frames no stage counts: the
event loop and the system), and `MAO_QUIT_AFTER=<seconds>` closes the window by itself, for a timed run. The offline
`map` view redraws only when something changes (3 frames in 66 s), so it cannot show stutter; that needs a live run.

### Measured live, and the characters' textures prepared ahead (2026-11-20)

Live runs on Maowild (the user's go-ahead), a minute each standing, turning and walking in Arete Landing, with
`MAO_PROFILE=1`, `MAO_QUIT_AFTER=60` and a scratch settings file copied from the user's (2560x1400 windowed, no frame
limit, no limit behind other windows). With the overlay's old sampling the slowest frames of every five seconds were 24
to 29 ms with 10 ms more in `ui` than in its layout; with the new one they were the steady 20 ms, so the fix holds live.

What a frame costs there now: the interface's layout about 7 ms (the most), the update about 2.5 ms, the characters'
upload about 2.4 ms every few frames, the scene's draw about 2.5 ms. And a hitch now and then: the characters' upload
(`reupload_overlay`) took 0.1 to 0.2 s when characters came into sight, their textures being decoded (and their mip
levels made) and then encoded in blocks (BC7/BC3) on the window's thread. The playfield's textures were decoded ahead
(`Textures::prefetch`), the characters' never. Now:
- `Textures::prefetch_new` decodes, on every core, the textures of the characters' model that the renderer was never
  handed (`Textures::handed`; the renderer keeps every texture it was given), on the network thread before each update
  takes the model to the window (`net-decode-ahead` in the profile: 29 to 864 ms there, off the frame).
- `mao_render::pack` makes the blocks there too (`Texture::packed`), the meshes' quick way; `upload_blocks` uses them
  when blocks are wanted and the fine alpha is not, else encodes as before. Nothing is packed while uploads are not made
  in blocks (the setting or the device).
Measured on the same route after: the characters' uploads at most 22.8 ms in play (from 102 and 202 ms), the frames
about 80 a second (about 67 before; not the same characters in sight, so not a clean comparison). One 0.34 s upload is
left at entry, when every character around comes at once, as the world is first drawn. Test
`a_texture_in_blocks_is_a_quarter_the_size` (the packed blocks are the ones the upload made). Not measured: zoning live
(the offline `map` upload of ICC Shuttleport is the same 1.7 s either way, so something else is most of that path).

### The interface's 6 ms: the chat's input bar searching the disk (2026-11-20)

The layout timed by groups of windows (`ui-w-*` in `MAO_PROFILE`), live on Maowild with a copy of the user's own window
files (a scratch folder; theirs untouched): of 6.3 ms, the chat window 5.7 and every other window together under 1. In
the chat window, its 2 lines took 0.01 ms; its input bar 5.5. The bar asks `scripts::names()` every frame for the
command list, and that looked up the scripts' folders (`scripts::dirs`: `aoprefs::prefs_dirs` lists the original's
`Prefs` folders under `%LOCALAPPDATA%/Funcom`, and for a character the original never played here
`newest_prefs_root` reads the date of every character's `Prefs.xml`) before it looked at its five-second cache. Now
the cache is keyed by the character playing and looked at first, and the names are read again on a thread of their own
(`script-names`) every five seconds and when another character plays; the bar gets the last reading (none until the
first is in, a few milliseconds after the first frame). Test `the_script_names_are_read_from_the_folders`.
Measured after, same place and windows: the layout 0.6 ms (from 6.3), 160 to 175 frames a second (from about 80 in
the same test window, 2560x1400), the slowest frame of every five seconds 10 to 12 ms. One run of the four had the
window not drawn for about 38 s near its end while the network thread went on (the next, longer one did not); not
explained, perhaps the window minimized.

## A new character's first hour, played by script (2026-11-20)

The user asked for a run with a new character of about an hour. Maohourly (paid account, id 2065585413, Solitus female
Enforcer, made with `net <game> create Maohourly 1 11 0 1 1`) played fourteen scripted segments in Arete Landing, each
logging in, playing, taking pictures of the client's own window (PrintWindow) and closing itself (`MAO_QUIT_AFTER`; the
runner never stops the client): the first arrival and its kit, Rex Larsson's robots and cargo box, Marcus Stone's gas
fires, the hand-over of the suppressant, the wounded dockworker and the stim's return, Flint Novak in the junkyard,
the Bio Analyzing Computer's robots, a death there and the return to the pad. Level 2 came during it (the level's
notice button showed). Everything the run did worked as the game expects but for what follows.

### The skills window: 8 ms of every frame

With the skills window up the frame rate fell from about 180 to 73 (`ui-w-skills` in the profile, 7.8 to 8.8 ms). Two
causes. The window set each of its 75 skills' values, colours, bars and buttons every frame by name through
`Tree::find`, a search of the whole view tree from the top, over a thousand of them a frame; it now looks them up in
an index of its named views made once the view is loaded (`skillwindow::index`, test: the index gives what
`Tree::find` gives for every name). And the tree's preferred sizes, worked out from every text in it, were worked out
afresh twice a frame (the window's default size and `host::show`'s minimum), 5.5 ms; the original keeps each view's
preferred size until told it changed. `Tree::keep_preferred` now keeps them from one call to the next until the tree is
to be laid out again (`relayout`) or gains views; only the skills window asks for it, as every change it makes sets
`relayout`. Measured live after: 0.54 ms, about 160 frames a second with it open.

### Test script steps added for the run

`MAO_CAMERA_SCRIPT` took three new interface steps, so that a script can do what the quests ask: `useon@<item words or
place>;<kind>;<id>` (an inventory item used on a thing in the world, the identity as `MAO_AUTOPLAY`'s `what` prints it:
`useon@Suppressant;0xc73d;0x5975d86b`; it sends what a right click with the item in hand sends once the click has found
its target), `give@<item>` (the item put in the open trade with an NPC, as a drop on its bar does) and `giveaccept@`
(the bar's V). `MAO_PROFILE`'s `ui-w-skills-team-extras` is now four: `ui-w-target-ncu`, `ui-w-skills`,
`ui-w-team-pets`, `ui-w-faction-extras`.

### The NPC trade bar kept the given place

After Marcus Stone took the suppressant (V, `KnubotFinishTrade`), his reward came to the same inventory place, and the
bar, which keeps the places put in, showed the reward as if it were being given; V again would have given it. GUI.dll
leaves the bar up after V (FUN_10057cc6) and has no handler for `KnubotRejectedItems`; its bar holds the items
themselves, so one the server takes leaves it. Now the bar keeps the item each place held when it was put in
(`Trade::put`) and drops a place whose item has gone (`npcchat::drop_gone`, test `a_given_item_leaves_the_bar`).
Not seen live after the change.

### Seen in the run and left for a decision or a later look

- The skills window's list showed Abilities and Body & Defense with an Abilities-sized gap between them and the other
  groups below sight, where the original's (capture arete-20261030-1655) stacks all eleven group buttons. Fixed the
  same day: GUI.dll's FUN_100fcbac hides each group's list and gives it `SetFlags(0x100)`, so a folded list takes no
  room (`gui::view::FLAG_COLLAPSE`); the client hid it without the flag. Seen in the preview
  (`MAO_UI_PREVIEW=skills`, and `MAO_SKILL=1,152,0` with Body & Defense open).
- The missions window that opened so small in the run was docked in a small dock area by the run's scratch settings
  (`docks.ini`: `DockArea4 MissionView`, written by an earlier test). Opened undocked from settings with no entry for
  it, it comes up at the original's 167 by 112 view grown to its columns: the mission's whole name, the time column and
  room for about five rows. Not a fault; nothing changed.
- The client's own walk to a `/waypoint` (MOV-028, kept by decision) that cannot reach its point (on a walkway above
  it) steps back and forth along the rail for as long as it is left, with no end; the original has no such walk.
- Marcus Stone, while he fights the cleaning robots, opens no chat for a use: the game's own rule (seen before), not a
  fault.

## The launcher's PATCH "crashed" it (2026-11-21, a tester's report)

"Launcher could not patch, just crashed the launcher." PATCH (PLAY while the chosen dimension's version is newer than
the game folder's, LCH-036) started the game's `AnarchyPatcher.exe` with `std::process::Command::spawn`, which is
Windows' `CreateProcess`. The patcher's manifest says `requestedExecutionLevel level="requireAdministrator"` (read
from the game's own file), and `CreateProcess` from an ordinary (medium integrity) process refuses such a program with
`ERROR_ELEVATION_REQUIRED` (740) without starting it. The launcher wrote the error to its output, which the packaged
program (no console) has nowhere, and closed as PATCH always closes it: to the player, the launcher vanished. The
original's Anarchy.exe starts the patcher with `ShellExecuteEx` "open" (0x407d37), which asks Windows for the
administrator's rights (the consent box) and then starts it.

Now (`launchwindow.rs`, `start_patcher`): the patcher is started through Windows' command shell, `cmd /d /c start ""
/D "<game folder>" "<folder>\AnarchyPatcher.exe" <version><expansion>` (no console window, no pipes), since `start`
hands the program to `ShellExecuteEx` and so gets the same consent box; the workspace forbids unsafe code, so the
call cannot be made directly. It runs on a thread of its own (the consent box waits for an answer) while the status
line says "Starting the patcher (Windows asks for permission)..." and PLAY is switched off. When the shell says it
started, the launcher closes as the original's does. When it could not be (no `AnarchyPatcher.exe` in the folder, the
consent refused: the shell's code 1223, any other code, or a folder name with `%` or `"` or a version from the
dimension list with anything but letters, digits, dots, underscores and dashes, which are not put on a command line),
the launcher closes all the same, as the original's does (the user's answer to LCH-052 on 2026-11-21: the launcher
may be among what a patch replaces, so it is not left running), and then a box of the system's ("Modern Anarchy
Online: Patch", rfd, on the main thread after the window is gone) gives the reason, the two versions, and that Funcom's
own launcher (Anarchy.exe in the game's folder) can patch too; it is also written to `crash.log`. The box is the
client's own (the original only wrote a line in `Anarchy.err`). (A first version that day kept the launcher open with
the message in it.) When the patch is done the patcher restarts Anarchy.exe, the original's launcher (its own strings:
"Restarting launcher"), not this one; the message says to start Modern Anarchy Online again.

`MAO_LAUNCH_DEMO=patch-failed` closes the launcher at once and shows the box without a patch. Tests:
`launchwindow::tests::the_patcher_is_started_through_the_shell` (the shell's line, the refused characters, a folder
without the patcher, the message's words). Not tried: the consent box itself and a real patch (it would change the
installed game; there is no dimension of a newer version to patch to here).

## The launcher's settings too dark to read (2026-11-21, a tester's report)

"Font color of settings is too dark to read compared to background." The settings (and the licence) are egui windows
in the interface theme's colours (`ui.rs::theme`): a panel 215/255 opaque, for the game's world behind it, and the
theme's `Text` colour, which for the original skin and the AOU skins is 0x99ccaa, a grey green. Over the launcher's
picture the panel let the art through (bright clouds and the dimension list showed under the lower groups), and what
is switched off (the window's size while the mode is borderless, the usual case) was egui's half-faded grey. The
original's settings were the system's own dialogs (Anarchy.exe dialog templates 153, 161 and 107: black on the
system's grey); these pages are the client's own (LCH-002), so the fix is the obvious one, in the theme's colours
(`launchwindow.rs`, `readable`, set once on the launcher's context): the window's fill opaque, the theme's text
colour lightened 55% of the way to white (0x99ccaa becomes 209,232,217), and a switched-off control keeps 75% of its
colour where egui's keeps 50%. Only the launcher's windows change; the game's own windows keep the theme as it is.
Seen in pictures of the Visual and GUI pages with the original theme and with aou_slim_cni (PrintWindow of a test
launcher started with a scratch settings folder). Test: `launchwindow::tests::the_launcher_windows_are_easy_to_read`.

## A windowed size larger than the screen (2026-11-21, a tester's report)

"Resolution when windowed will make a window larger than your screen. Seems like when I touched the corner it resized
but moved." The settings' width and height (the launcher's Custom fields, its Predefined list, which offers the
monitor's own size, and the options window) went straight to `with_inner_size`: a framed window of the monitor's own
size or more, plus its frame, ran off the screen, and Windows cut it down to its largest size (the screen and the
frame) at the first drag of a corner, which moved it.

The original (DisplaySystem.dll, `D3DDisplaySystem_t` slot 3 Init, FUN_10002c67, windowed): the client size is
grown by its frame (`AdjustWindowRectEx`); where the framed window is wider or taller than the monitor's work area
(`GetMonitorInfo`'s `rcWork`) it is made the work area's width or height; it goes to its kept place
(`DisplayXPos`/`DisplayYPos`) when it fits the virtual screen there, else it is centred (on the work area when it fits
it); `MoveWindow`, then `GetClientRect` gives the size the device is made at.

Now (`viewer.rs`, `fit_window` and `keep_on_screen`, called from `apply_mode`'s windowed branch, so for the login's
window, the game's, F11/Alt+Enter back to a window, and after the options window sets a size): the frame is the window's
outer size less its inner size; the framed window is made no larger than the monitor's work area, and it stays where
it is when it fits there, else it is centred in that area. The settings keep the size asked for. The work area is the
system's, as the original reads it (`work_area`: `GetMonitorInfoW` on winit's monitor handle, `rcWork`), so a taskbar on
any side, of any size or hidden is allowed for. That is the workspace's one call of unsafe code: the rule is `deny`
(no longer `forbid`) with `#[allow(unsafe_code)]` on that function alone (the user's answer to SET-055 on 2026-11-21;
the first version that day guessed a 48-point taskbar along the bottom). Other systems, and a failed call, keep that
guess (`TASKBAR_POINTS`). The log line of a fitted window names the area and whether it was the system's.
The kept place (the user's "do what the original does for the window", 2026-11-21): the original keeps the framed
window's place in the distributed values `DisplayXPos` / `DisplayYPos` (MainPrefs.xml and LoginPrefs.xml, -1 and -1 by
default). Its display's destructor (`D3DDisplaySystem_t` FUN_10001e3e -> FUN_10001bd2) writes `GetWindowPlacement`'s
restored place into them, and the windowed start reads them and keeps that place when the framed window lies inside the
virtual screen there (every monitor's rectangle together), else centres it on the work area (on the monitor, where it
does not fit the work area). Now: `viewer::keep_place` writes the framed window's place, when it is neither minimised
nor maximised (winit gives no restored place of those), to `display.ini` beside the settings (`DisplayXPos = x`,
`DisplayYPos = y`) when the game's window closes (`App::exiting`, live games only) and when the login's does
(`Launcher::exiting`, also at the hand-over to the game, whose window then keeps its own on closing).
`viewer::place_window` places a framed window at its start: the login's at the kept place, the game's at the login's
(`MAO_WINDOW_AT`, as before) or else the kept place, each when `on_the_screens` holds, else centred on the work area. No
file yet is the original's -1, -1: centred (the original's -1, -1 is centred too, except with a monitor to the left of
the main one, where it would lie on the screens: not copied). The offline views keep the system's place and keep none.
The original's place is in work-area coordinates (`rcNormalPosition`), which shifts it by the taskbar's width when the
taskbar is on the left or at the top; the client keeps the screen place, so the window comes back exactly where it was.
Seen: with `display.ini` at 300,200 the login's 1280x800 window opened at 300,200 and wrote it back on closing; at
5000,200 (off the screen) it opened centred in the 2560x1504 work area (627,316) and wrote that. Test:
`viewer::window_tests::the_window_opens_where_it_was_kept_when_it_is_on_the_screens`.

Seen in a test window (`mao-client <game> map 655` with `mode = windowed`, 6000x2400, on a 2560x1600 screen at 200%
whose work area is 2560x1504): "the window's size 6000x2400 is larger than the screen: 2534x1433 (the system's work
area 0,0 2560x1504)", the framed window at 0,0 to 2560,1504. Test: `viewer::window_tests::a_framed_window_is_kept_on_its_monitor`.

### The right button's turn turns the character alone (2026-11-21, the tester's "can't turn sharply, drifts")

The tester: "Can't turn sharply, character continues to drift in a direction." Their settings have `camera_mode = lock`.
The right drag's movement across went to the camera (`mouse_look`, `camera::SteeringCamera::turn_and_zoom`) and the
character was then set to face the camera (`keys.face = heading_from_yaw(camera.yaw)`). A locked or rubber camera keeps
its turn as a place relative to the character's back (`UpdateHeadingToPos`), so when the character turned to face it the
camera went round again by the same amount: each motion turned the character by everything the drag had turned so far,
the turn ran on past the mouse, and the offset stayed for the next drag (the press itself also swung the character to
the camera). In the tester's last log the character turns on the spot with 10/13 sent a few times a second while the
heading wanders by up to a radian between them.

The original (GUI.dll FUN_1002ccbe → Gamecode `N3Msg_MouseMovement` 0x100196b3): the counts times
`MouseTurnSensitivity` / 1000 go to the character, the across part as the movement 0x2b that turns it (after the
start messages 10 / 13 when the side changes, which were already ported), and the camera is handed
`MouseCameraControl(0, up)`, the up part only, and that only with `RMBMouseLook1st` in first person or `RMBMouseLook3rd`
otherwise (both true in `LoginPrefs.xml`); in fly mode it gets nothing (the up part pitches the vehicle). The press
(FUN_1002ce31) only sets the look's kind. A locked or rubber camera then comes round behind the character by itself;
the free (trail) one keeps its direction and is pulled along.

Here: `viewer::steered_face` (the heading from the one the character had at the drag's first movement plus the
movements since), the right drag's across movement no longer added to `mouse_look`, its up movement by the two
preferences, and the press leaves `keys.face` empty. With Ctrl the right drag is still the camera's look, as the left
one is. Test: `the_right_drag_turns_the_character_by_the_mouse_alone` (a locked camera, a drag there and back: the
character ends where it began and the camera behind it). Not changed: the original's limit on the turn while stat
224's bits 2 or 4 are set (that and stat 267), and the head turn handed over at a first-person look's end.

### Other characters move between their messages (2026-11-21, the tester's "other characters teleport around")

The tester: "Other characters teleport around, very choppy when moving. When standing, animations" (the sentence is cut
off there; taken to mean that standing characters' animations look wrong too). A moving player sends a movement message
only as its movement changes and a full report after 5 s without one (`movement::FULL`), and the client set each other
character at each message's place and did nothing between: a runner stood still, then jumped up to 60 m on. Its
animation went by its place having changed in the last 0.7 s, so it ran for 0.7 s after each jump and stood between,
and a standing character whose report moved it a few centimetres ran on the spot for 0.7 s.

The original: the handler of a character's CharDCMove (Gamecode FUN_1006bd94 → FUN_1006c20f) keeps where the character
was drawn (`n3VisualDynel_t::UpdateReconcilePos`, N3.dll 0x10019414), sets the message's place and turn, and runs the
message's movement transition on the character's movement status (FUN_1006f51b; FUN_1006cb58's table, the same as for
one's own) while its stat 224 has bit 4 (with bit 2 only, the turns 9 to 0xe). Its vehicle then moves it every frame as
one's own vehicle moves. `n3VisualDynel_t::Run` (0x100196bd) draws it at the reconcile point, which it moves a fifth of
the way to the character's place each frame (0.8 and 0.2) while the two are between 0.2 and 35 apart (the squares 0.04
and 1225 compared), and drops nearer; further apart it is drawn at its place. The announcement's vehicle block
(FUN_1006f587: the velocity, then the status bytes FUN_10070981 reads) gives the status of one already moving as it comes
into sight.

Here: `net::OtherMove` (forward, back, the sidesteps and the key turns as the transitions leave them: a stop ends only
its own direction, a start the other way turns it round, the full stop, sitting, sleeping, lounging and being frozen end
all; the mouse's turn starts turn at no rate, their heading comes in the messages), `World::move_others` each frame (along
the facing at the character's speed: walking 1.5, swimming the swim's, running by its `RunSpeed` as one's own when a `Stat`
or the announcement told it, else the speed its last two messages of one move were apart at, else a `RunSpeed` of 0's 5
forward and 3 back; sideways at the forward speed, together no faster; the height following the tile map's ground where
there is one), `net::reconcile` (per sixtieth of a second, as the camera's smoothing is, `camera::keep_for`), the drawn
place used for the body (`World::drawn_place`), and the movement animations by the status (`World::shows_moving`; the
creatures the server walks along paths keep theirs by their place). `EntityInfo::move_status` reads the four status
bytes after the mode (bytes 13 to 16 of the block, players and creatures alike); a player announced moving starts so.
Tests: `others_move_between_their_messages`, `another_characters_transitions_are_the_tables`, and over the captures
`the_announced_movement_status_is_the_vehicles` (1404 players announced, 191 walking forward, each one's next movement
message a stop, a report, a jump or a turn, never a fresh start). Guessed: the measured speed for a player whose
`RunSpeed` is not told (the original knows every character's); stat 224 is taken to have bit 4 for every other
character (the client does not keep the others' stat 224). Not ported: the others' collision (they walk through walls
until the next message puts them right, as in the original only where the server has them).

### The casting animations: gathering, then the release (2026-11-21, the tester's "no animation when casting nano")

The tester: "No animation when casting nano. Arms just don't seem to move." In their logs the only program their
character cast is Commanding Presence (nano 210920, `25314d6d` then `CharacterAction` 0x62 with its 12100 run), whose
flags (stat 0, 0x90002) hold 0x80000: the original's `FUN_1005141d` runs such a cast at once and never puts it in the
character's list of casts, so its casting state never begins and nothing is played. For that program there is no
animation in the original either (original behaviour; unchanged).

For a program that takes time the client differed. The original's casting state (`CharCastNano_t`, FUN_1007b5cd, its
frame FUN_1007b1e3) loops the program's `GatherAbstractAnim` (stat 376) while the cast gathers; at the cast's end it stops
that (FUN_1003c6cb: the loop count set to 1, so the lap under way finishes), may first turn the caster to face its target
(state 7, when the angle is large), and plays `CastSelfAbstractAnim` (378) at oneself or `CastTargetAbstractAnim` (377)
at another once. No program has the three stats; they are what every item starts with (FUN_10086142): 0xcb `spell-sys`,
0xca `spell-self`, 0xc9 `spell-dir`. The client played `spell-self` / `spell-dir` once as the cast began and the caster
stood idle through the rest of the cast.

Here: `net::CastAnimation` (the cast's beginning, its time as the stance's rule already worked it out, and whether it is
at oneself), `World::casting`, `World::social_now` (the cast's animation before any social), and the animation stack's
looping social, ended with the loop count 1 as the next one is set (`Characters::social_loops`). A cancel (0x89) ends
both. Picture of the gathering (`mao-client <game> player 2 3 da59 --image out.png 300`): both hands up in front, working.
Test: `a_cast_gathers_then_releases`. Not ported: the turn to face the target before the release; a cast whose time comes
to nothing without the flag (the original still plays the release; here none, by the stance's earlier rule).

### "Dropped back to character select when a door opened in battlestation area" (2026-11-21): not in the logs

The tester's two logs (`game-log.txt`: 655 Andromeda, 6007, 152, 540 Old Athen; `game-log.previous.txt`: 790, 560,
565, 566, 655) both end "connection ended: the server closed the connection", but neither is the drop they describe:
their chat log has "Timed logout started. This will take 30 seconds." for every session of the day, and in both logs the
end comes about 30 s after the character sat down (the own place's echo with movement 0x1e) as that logout began. Neither
log visits a battlestation (the playfields 4380 to 4388, `n3Playfield_t::IsBattleStation`), and in neither does the
client fail a hand-off to another server ("can't follow the redirect") or panic.

What a door does here, read for the report: touching a door sends no message of its own; the door opens on the client
(`World::tick_doors`, `door_may_pass` for a locked one) and, within half a metre of its place, one full report (0x16)
starts a teleport try, as the original's `StartTeleportTry` does; the server answers with a teleport (`43197d22`) or,
for another server's playfield, a hand-off (system 0x3c), which `hop_direct` follows with the session key. The ends of
the two runs, named by their class names hashed (`mao-message-numbers`): `25314d6d` CastNanoSpellIIR_t (Commanding
Presence, nano 210920, cast shortly before each logout), `39343c68` BuffIIR_c, `5e477770` CharacterActionIIR_t
(0x62, the program's run); in 6007 `52526858` GenericCmd_t (the uses of the terminals), `41624f0d`
AppearanceUpdateIIR_c, `64582a07` OrgServerIIR_c, `60201d0e` SetWantedDirectionIIR_t, `4d450114` SpellListIIR_t.

So that the next such report can be placed, the game log now says: each door that opens or shuts, by its identity and
number, and whether the character touched it; the logout the player asks for (`/camp`, the quit menu) with its time;
and the time and playfield of the connection's end.

## A tester's interface bugs (2026-11-21)

A tester's list after playing 0.1.8. Each part says what the original does and what changed.

### The control center behind every window

The tester: the Character, Team and Commands buttons "float", and the chat window slides between them and the
NCU/credits strip. The parts of the skin's control center were drawn on three of egui's layers: the wings on the
background, the strips on the windows' own layer and the buttons (menu rows, mini toolbar, rollup, target buttons)
over every window. A window dragged over the bottom of the screen went over the wings, under or over the strips by
which was last touched, and under the buttons.

The original's control center is **one window** (`control_center_window`, FUN_1006f911), and
`ControlCenterModule_c::LoadMenues` (GUI.dll 0x10068580) puts it **behind all the others** (`Window::MoveToBack`) once
it is made. Now every part of it is on the background layer, under every window; within it the wings lie under the
strips and the buttons, which are raised over the rest of the background layer each frame (`vitals::show_docked`,
`ControlCenter::raise`; egui keeps the order among layers raised in the same frame). The menus a button opens stay over
the windows. The buttons' places are the skin's `ControlCenter.xml`'s, as before: the game's own skin has its menu rows
33 pixels above the screen's bottom on the wings (`layout_borders="Rect(12,0,0,33)"`), the AOU skins' at the very
bottom in the strip, as the original's own screenshots show them.

Picture: `MAO_UI_PREVIEW=chat` (now with `MAO_PREVIEW_SIZE=<w>x<h>`, the picture's size, for a screen like the
tester's) with the game's skin and with `aou_slim_cni`: the chat window covers the menu row and the target buttons.

### Adding shortcut bars from the option panel

The tester: new bars "did not appear on screen" when chosen. The option panel (Interface page of the skins' own
`OptionPanel/Root.xml`) has "Number of shortcut bars" (`NumHotbars`), "Lock shortcut bars" (`LockHotbars`) and the
hide/show box (`shortcutbar_window`): they set the control center's values, which nothing read: the bars followed only
this client's settings (`hotbars`, `lock_hotbars`). `ShortcutBarWindow_c` (FUN_100d9e5f) reads `NumHotbars` for the bars
it shows. Now the panel's two options set the settings (saved in `mao.ini`, `shortcutbar::follow_options`), the
settings are written back into the values (so the panel shows what the client's settings page set), and
`shortcutbar_window` off hides the bars (it was listed as not ported). A new bar stands where the original's saved one
does, else stacked up from the bottom left, as before.

Test: `the_option_panel_sets_the_bars_shown_and_their_lock`.

### Shortcut icons whatever window was opened

The tester: "bar animations lack icons, are always just the first letter of the name". The first run's bar holds
Start Combat, Walk, Sit and Suspended Animation (special actions), and the bar showed S, W, S, S: a special action's
picture came only from the Action window's icons, which were loaded the first time that window opened; a nano program
on a bar had no picture at all; an item's only while it was in the inventory. The original's cells are
`ItemIconView_c`s (FUN_100d825b) that show the item's or the nano program's icon, whatever window is open.

Now a shortcut is drawn by the icon kept with it, else by its own record's (stat 79: the item template, the nano
program's record 0xfde85, the action's template; `shortcutbar::icon_id`, looked up once each) through the database's
icons (`gui::pictures`), on the original's bars and on the smart hotbars alike (`ui::shortcut_texture`). A macro is
still `GFX_GUI_ICON_MACRO` with its name; the letter remains only where the database has no icon. There are no social
(emote) shortcuts in the original or here: the Social menu's entries are not dragged onto bars.

Picture: `MAO_UI_PREVIEW=shortcutbar` (Start Combat, Walk, Sit, the Follow macro, Suspended Animation with their icons).

### The pointer over the world

The tester: the mouse pointer does not change over something to use (a grid terminal, a shop). It never did: the
client's pointer followed egui's alone. GUI.dll's `MousePointerModule_t` draws the pointer as sprites of its own: an
arrow (`Pointer_e`, the table at 0x10263170: standard, blue, green, yellow, purple, red, gray, then the drags, the
I-beam, the hand and the wait) and up to two pictures beside it (`Detail_e`, the table at 0x102632a0: attack, pick up,
operate, look at, question, denied, talk, trade) for what the left and the right button would do, each with a small
mouse under it showing its button (`POINTEROVERLAY_LEFTCLICK` / `_RIGHTCLICK`). FUN_100202e6 puts a picture 18 pixels
right of the arrow's point (the second 27 further), a mouse 10 right of and 20 under its picture; one picture for both
buttons gets both mice, the left 9 further left; a denial gets none. `FrameProcess` (FUN_10020629) reads the
`MouseCursorMode` option: 0 no left picture, 1 one picture and no mice, 2 (the game's default) both.

`InputConfig_t::CheckObjectUnderMouse` (0x1001a33f) chooses them for the thing under the pointer (`pointer::world`):

- one's own character: the green arrow;
- an NPC: blue when its state flags (stat 224) have 0x20000000 (it cannot be attacked) or its flags (stat 0) have 2 in
  bits 22-23 (it talks; then a talk on the right button), else red;
- another player: a trade on the right button and, while `DoubleclickAction` is on, on the left; green in one's team,
  else yellow, purple on one's side and red on another where it can be fought (its state flags 0x4000001, else
  `N3Msg_HasVulnerableFightMode`, Gamecode 0x10017052: one's own fight mode 2 by side, 3 by organization, 4 anyone);
- Ctrl over a character: an attack on the left button;
- anything else: the blue arrow, with a pick-up (`Can` bit 0) or a use (bit 3) on the right button and, with
  `DoubleclickAction`, on the left. The game's grid terminals ("Enter The Grid", Can 0x8), mail, market and mission
  terminals (0x8) and doors (0x408) get the use's cog;
- Shift: a look at on the left button of anything.

Here the parts are put together into one picture for the window's cursor (`pointer::compose`, cached by look), set
while egui wants its plain pointer and the pointer is over no window. The thing is the one a right click would take
(`picking::under_line`); its facts come with the pickables from the network thread (`picking::Class`,
`World::pointer_class`): a table dynel's or an announced item's `Can` is its template's. Guessed or left: the camera
mode 0x39 that leaves the pointer alone is not checked; team membership stands for the team stat (6); an NPC's stat 0
and 224 are known only once a `Stat` message has told them. The pointers' hot spots are now GUI.dll's own (the same
table: the hand 5,1, the four-way drag 16,16, the drags 6,16 and 16,6, the wait 16,16).

Tests: `usable_things_get_the_blue_arrow_and_their_use`, `characters_are_coloured_by_what_they_are_to_oneself`,
`the_mode_and_the_parts`, `the_parts_put_together_from_the_games_skin` (with `MAO_GAME`; `MAO_POINTER_PNG=<file>`
writes the composed pointer).

### The inventory widened: items keep their cells (the original's way; nothing changed)

The tester: widening the inventory, the empty squares filled the new width while the items stayed in as many columns
as before. That is the original's grid while its items are placed by hand (`Don't Auto Arrange`, an own container's
default): each item keeps its own cell, a column and a row (`MultiListViewItem_c::SetGridPos`, an `IPoint`, kept in
`item_position_map`), the cells shown follow the width (`RecalcCellCount` 0x10134ad2), and the grid is never narrower
than the columns the items reach (`RecalcVirtualSize` 0x10134944). The items move only when put elsewhere, or when the
grid is arranged (`Order` in the window's menu, which lays them along the rows of the width shown) and then let be
(`SetAutoArrange(false)` 0x101371db: each item keeps the cell the arranging gave it), as here (`invwindows.rs`).

## Lamp halos that were boxes (2026-11-21, branch `fix/lights`)

The tester: "The lights were showing light boxes that don't really resemble", with a screenshot of Old Athen by the
Dreadloch Arms & Gear sign in daylight: big pale-grey see-through rectangles round two street lamps, their edges plain
against the sky and the trees, and a soft white glow at one corner of one of them.

**The cause.** The lamps there are the halos their mesh's connectors name (`eff_haloM...` with a colour and a size:
template 7004, a `_GfxControlSprite_t` billboard 8 m wide of material 8, `light_halo2.png`, white at alpha 0.25 times a
size of 1.004; 56 of Old Athen's 84 statel effects; `MAO_EFFECT_PLACES=1 mao-client <game> statel-effects 540` lists each
place with its colour and size). `effects::sprite_particles` read flag 0x4 of the template (clear in all 16 kind-1012
templates) as "sample a random spot of the texture each frame", so every frame the quad showed the halo's picture moved
by a random amount, its bright middle in a corner or cut in pieces and the quad's edges plain to see.

**What the original does.** Gamecode's FUN_100f64a7 makes the control's visual: `GfxVisualSprite2(material, a, 0, 0,
width, height, additive)` (or `GfxVisualSprite3`), with `a = !(flags >> 2 & 1)` and `additive = !(flags >> 12 & 1)` (read
from the pushes at 0x100f64dc-0x100f6507). DisplaySystem's `GfxVisual` constructor (FUN_10007823) keeps `a` at +0x190, the
second word at +0x194 and the third at +0x198; the sprites' draws (FUN_10023ea4, FUN_10028606) take a random spot only
when +0x198 is not 0, which this control never asks for. So the quad always shows its frame of the sheet (the first, or
the running one with flag 0x20), whole: a soft glow centred on the lamp. Flag 0x4 picks the blending instead: clear (every
template) `GfxVisualSprite2` adds by the source alpha (`SRCALPHA`, `ONE`), which is what the client's particle pass does;
set, it would blend (`SRCALPHA`, `INVSRCALPHA`). The halo texture's border is black and transparent (128x128, corners
`[0, 0, 0, 0]`), so nothing of the quad's edge shows.

**The fix.** `sprite_particles` no longer moves the texture; it draws the sheet's frame. That also mends the other users
of the single quad: the tower beams (61085-61087), the halos over a character (43607, 80006-80009) and the white, red,
green and blue halos (7000-7003). Seen in offline pictures of the tester's spot before and after, by day and at night
(`MAO_SKY=23 MAO_SKY_PLAYFIELD=540 MAO_STATEL_EFFECTS=5 MAO_GFX=<game>/Setupf/gfxtweak.bin
MAO_CAMERA=404.5,370,18,14.43,1.452,-0.069 mao-client <game> map 540 --image out.png`): the rectangle gone, a round glow on
the lamp's head. Test: `a_sprite_quad_cycles_its_size_and_colour_and_stands_when_asked` (the rectangle is the whole frame,
the same every frame, with flag 0x4 set or not).

**Not done.** The client still adds every quad. Template 7100 (flag 0x1000: a `GfxVisualSprite3` not added, which the
original draws `DESTCOLOR`, `SRCCOLOR`, a doubled darkening) and a template with flag 0x4 set (none in the file) would
blend otherwise in the original. Not seen in the live game yet (LIVE_TEST_LIST).

## One's own character as a red box after a zoning (2026-11-21, the user's report)

"When zoning the character is drawn as a red box at first (possible race condition)." A zoning cleared every character's appearance (`state.infos`), one's own too, and set one's own place again at once; the loading picture lets go when the new playfield is drawn, and when that came before the server's full update for one's own character in the new playfield, `overlay_model` had a place and no body and drew the client's stand-in: a 0.6 x 1.8 box, red for one's own (blue for others, green for players). Now one's own appearance is kept across the zoning (the full update replaces it), and one's own character with no body to draw is drawn as nothing, as the original has no such box; the others keep their stand-ins (a creature this client has no body for). Test: `net::door_tests::ones_own_character_without_a_body_is_not_a_red_box`. Not tried live.

## Borealis's mountains upside down (2026-11-21, the user's screenshot)

Borealis's sky (`Tweak_Playfield_800` -> `Tweak_Playfield_Backdrop_Mountain`, the only playfield with it) has a ring of mountains, `backdrop_mountains.abiff`, 200 above the camera and turned by nothing. Its data is stored y-up (x and z run +-950, y -190..451) with its own frame a half turn about x, where the domes, clouds and the horizon's dish are stored z-up with the exporter's quarter turn. The sky's reading (RND-108: frames left out as `FUN_10061505` keeps them, z negated, `[ROT]` the other way) was fitted to the round meshes and hung the ring over the sky upside down; its dark underside hid the sky overhead. A sky mesh whose frame is the half turn (`viewer::sky_draws`, `half_turn`) now has it applied in place of the bare z negation: the ring stands on the horizon the right way up, and the moons (the only other meshes with that frame, spheres) are where they were. `MAO_SKY_DEBUG=1` prints each sky mesh's axis ranges and frame. A guess within RND-108; the band of the ring's side away from the sun is dark in the offline picture (its lighting not checked live).

## The rollup column's pages one width, cut to their rooms (2026-11-21, the user: "rollup panel should be uniform")

The wear page stood wider than the nano page under it, and the nano page over the wear page's foot. A page's view was an egui window that could grow: `body_width` widened the page to what the view drew, and nothing cut the view at the page's height, so the wear view (its minimum width its four tabs in this client's fonts, its cells grown to fill it) ran wider and down over the next page. Now `rollup::page` gives the view the page's rectangle, the column's 192 by the page's height, and clips it there (the original's page frames its view, FUN_10049797; the grip gives more height), and the wear window docked in the column no longer asks for its tabs' width: its tabs are squeezed to the column by `space_out` (`View::SpaceOut`), as the original's are. Seen in a preview (`MAO_UI_PREVIEW=gear MAO_CAPTURE=<net.log> MAO_PREVIEW_SIZE=1280x800`): the wear page 192 wide, its five rows inside it.

## The "Brink" band on Nascense Frontier's ground (2026-11-21, the user's screenshot)

A black band with the word "Brink" printed over and over lay along Nascense Frontier's edge (playfield 4001, z past about 1965 near x 685), flat and up the slopes at its border. It is the terrain: tile index 10, ground texture `0x306` ("shadowbrink", types 0xf6956/0xf6965/0xf6966), whose every texel has alpha 2. Of the game's 2,535 ground textures it is the only one that is not opaque throughout (`cargo run -p mao-formats --example groundalpha -- <game>`). The ground's tiles are now drawn with an alpha test at a half (`terrain.rs`, `alpha_ref`), which leaves that marker out and nothing else; the land ends at the brink. The original's ground draw is randy31's and not ported, so this is a reading of the data, not of its code. `MAO_TILE_AT=x,z` with `map <pf>` prints the tile words along a line of cells, to find a ground texture.

## Name tags that shook over following pets (2026-11-21, the user's report)

Since other characters are drawn moving smoothly between their messages (`World::drawn_place`, the reconcile point easing a fifth of the way a frame), their bodies stood at the drawn place while the name tags, floating combat texts and the spell effects started on them read the raw place (`entities`, `position_of`), which jumps at each message. A following pet, its place told often, had its tag hopping about its body. They now take `World::shown_place` (the drawn place, else an object's place); the sounds and the target bar's distance keep the told place. Test: the end of `others_move_between_their_messages`. Not tried live.

## Casters lying as if dead after a cast (2026-11-21, the user: "lays down, looks like they have died")

A cast's gathering (`CastAnimation`, the looping `spell-sys` at priority 0) drops the movement animation it is set over once it has faded in (the stack's rule: a looping animation fully faded in drops the others before it at its priority, `animstack`; the original's call, `CharCastNano_t` FUN_1007b5cd, is looping at priority 0 too). The gathering then finishes its lap and the release plays once, and the stack was left empty: `Characters::drive` set the movement only when the movement changed, so the body had no animation and lay in its bind pose until the character moved. Every caster did, the buff bots of a city square too (the user's screenshot: bodies on the ground under their names). Now a looping movement that was dropped is set again once nothing else plays at its priority (as the original's movement takes the character back when its casting state is over, the release played out), whole at once since there is nothing to fade from; it is not set again over the looping gathering, which the two would drop in turn. Test: `the_movement_comes_back_after_a_cast`. Not tried live.

## The Shadowlands' vortex as sheets (2026-11-21, the user's screenshots in Jobe Research)

The vortex over the Shadowlands' skies (`shadowlands_vortex.abiff`, opaque with its blob's fog on) showed as pale sheets with straight edges. The sky draws' fog was by each corner's distance from the eye and eased (smoothstep): the vortex's few large triangles each took a fog of their own, creased at their edges. The original's fog is randy31's vertex fog (`SetFogParameters`: `FOGVERTEXMODE` 3, linear, `FOGTABLEMODE` 0) by the vertex's depth along the view, range fog off (`RANGEFOGENABLE` 0 among the device's first states): a disc facing the eye is fogged evenly. `sky_draws` now does the same (this answers the note above that left the distance-or-depth question open). And a sky draw whose blob names `SRCBLEND ONE` with `DESTBLEND ONE` (the vortex flares, `e_SunRays`, in every Shadowlands sky, and the dot stars) is added whole (`Blend::AddOne`, `fs_backdrop_add_one`): its colour is not weighed by its alpha, where the sun-ray fan's flicker lies, so it no longer shows as wedges; a blob that names no source factor (the suns of Rubi-Ka) is drawn as before. Seen off the game with `MAO_SKY_EXPANSION=255 MAO_AREA_FOG=...`: one even fogged disc where the old build had creased shading. Not done: whether sky draws that name no fog state are fogged too (the renderer turns the fog on each frame; only blobs that turn it on are fogged here), which would decide whether the vortex's outline melts into the sphere behind it; the capture of 2026-10-31 in Inferno is the reference to check it against. Not tried live.

Checked against that capture the same day (the user's go-ahead): `arete-20261031-0842` is in Inferno (4605) from 1793454746.6 to 1793455133.9; its camera looks along the ground, and the one shot with the horizon in view (1793454873556, the character at 2720.6, 84.9, 2735.2, heading 0.91) shows one dark orange haze, about (65, 40, 20), over the whole horizon and the top of the view. This client there, with that place and an area fog given by hand, shows the vortex's purple at the top and pale distant hills. Fogging the sky draws that name no fog state (the rule above) changed nothing visible at that place and was not kept: it would change every sky. What differs is more likely the fog's colour and range (the offline fog is set by hand, and the headless fog is known to differ) than which draws are fogged; a live look in Inferno at the same hour is the next step.

2026-11-22, the user's Nascense Frontier shots from the original (light blue sky, a pale blue-white swirl round a white sun with rays, a rainbow halo): this client drew the vortex as a dark blue disc with black edges. The cause was the fog's range, as guessed above. The effect scripts' atmosphere adds its fog to `VisualFog_t` (DisplaySystem FUN_1005b633: `AddFog(AddFogR, AddFogG, AddFogB, AddFogI)`; `AddFogI` is 0.1 in `Tweak_Shadowlands_Atmosphere`, 0.025 in Rubi-Ka's, 0.45 in Pandemonium's), gathered with the area's fogs by the same rules, and Gamecode hands it the camera's front and back planes (`AddClipPlanes(camera +0xb4, +0xb8)`, FUN_100b77cb / FUN_100b79f7; randy31's `RCamera_t` keeps front at 0xb4 and back at 0xb8). So outdoors the original always fogs, linearly from 0.2 to `back − (back − 5.2) × weight`. The client kept only the atmosphere's colour (`Frame::fog`) and fogged the sky from a seventh of the view distance out, the world not at all (RND-185 removed its own haze on the belief that the area's fog was the only one). `Frame::fog` now carries the weight; the live frame gathers it into the area's (`environment::add_fog`), and the world and the sky's fogged draws use that one fog (`sky_fog`). Offline (`MAO_SKY=12 MAO_SKY_PLAYFIELD=4310 MAO_SKY_EXPANSION=3`): the vortex is a pale haze with the swirl faint near the sun, close to the shots' colours (theirs about (148, 193, 223) over the swirl, the fog colour (155, 198, 214)); far land is hazed. Note `MAO_SKY_EXPANSION` must be 2 or more for the Shadowlands' `Expansion EP01` blocks (the vortex meshes), as a live account's stat 389 is. Not tried live; the world's fog still goes by distance where the original's goes by depth.
