# Client visuals, window and interface

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
chains and 16x anisotropic filtering for every texture, and character textures
drawn opaque (their alpha channel is not a cut-out mask: that fixed the Waste
Collector). The sky and fog are the effect scripts' per playfield now (see "The sky"). Bloom is not a look the original
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

## 4d. Camera (WoW style, live view)
The camera follows the character. **Left button drag** orbits it (the character
doesn't turn); **right button drag** turns the camera and the character with it
(the character faces where the camera looks); **both buttons** walk forward;
**scroll** zooms. While walking or turning with the keys the camera swings round
behind the character (unless the mouse is steering). The cursor is hidden and
confined while a button is held. Strafing is on `Z` / `C` (A/D turn, as the original's defaults do; see 4e). Not done:
camera collision with the ground and walls, an option for inverted mouse.

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
dragged too and keeps its place. The strips hold what GUI.dll's bar views hold: the
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
the order. Seen live: the shop's window opens with its list and prices, and a double click on a row and Accept buys
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
picture. Not done: the same scale for the humanoid NPCs and for other players (whose updates were not read for it),
the remains of a scaled creature (not looked at). `MAO_COMBAT_LOG=1` prints a "no template" line with the bytes of
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
the inventory before; `/open inventory` does that). `/tower` and `/raid`'s commands are not carried out.
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
gone. The original's two hiding modes (+0x74) are not ported. Not seen live.
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
Not done: turning the character with the mouse, the hall behind the creation screens (they have their own cameras
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

- **Backpack count**: the inventory's Backpack heading shows how many of the 30 places are used ("Backpack  12/30").

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

Every window of the interface (17: inventory, stats, skills, NCU, nano programs, perks, friends, missions, mail and message, shop, loot, help, actions, Whom-Pah, options, chat) is made by `chrome::window(ctx, palette, name, title, &mut open, |w| w.<size/place>, |ui| ...)` instead of `egui::Window` directly. With the skin's pictures the window has no egui title bar: it is drawn with the original's frame pieces (`GFX_GUI_WINDOW3_BORDER_*`: thin top and sides, rounded bottom corners; greyscale masks tinted with the theme's border colour, as the original tints them from `GUIColors.xml`), a translucent panel fill, the title inside the frame at the top left, and the original's pin and close buttons (`GFX_GUI_WINDOW_PIN`, `GFX_GUI_WINDOW_CLOSE_X`, with their hover and pressed pictures) at the top right. Dragging anywhere on the window moves it (egui 0.37's own fallback for a window without a title bar); the frame is drawn under the content from the window's rectangle of the previous frame. The pin does what the original's does: a pinned window is left open by Escape (`chrome::pinned`, read by `Ui::close_with_escape`). Without the pictures the plain egui window is used as before. The windows' ids are unchanged, so `layout.ini` positions carry over. Checked offscreen (stats, chat, inventory, options; both themes); dragging and the pin not yet tried live. Not done: the launcher's windows, the original's titles where ours differ (Mission, Wear), tabs/buttons in the original's art, the question-mark button.

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

**Shortcut bar in the original's slot art.** With the skin the ten slots are 38 px `GFX_GUI_MULTILISTVIEW_SLOT_32_CLOSED` frames (the `OPEN` one for an empty slot) and the bar is stepped through with the control center's big arrows (`CONTROLCENTER_BIGARROW_*`, hover and pressed pictures) around the bar's number; Alt+1-6 still pick a bar directly. Everything else of the smart hotbar (queued presses, cooldown sweeps, drag-and-drop, the menu) is unchanged; without the skin the old 46 px look stays.

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

- **Modes** (`PreferredCameraMode`, the `3rdPersonCamera` switch): first person; the free camera (1, the default), which keeps its direction and is pulled along at the preferred distance, so it trails behind the character as it walks and stays put as it turns; the rubber camera (2), which steers to the spot behind the character; the locked camera (3), which is held there (easing out at a tenth a frame when a blocked view opens). The settings' `trail` / `rubber` / `lock` are 1 / 2 / 3; F8 toggles first person.
- **The vehicle:** mass 20; each frame the speed is the character's run speed × 6 (at most 16), the maximum force `mass × speed / 0.3`, the brake distance `0.3 × speed`; `SteeringArrive` wants `distance / brake × speed` (at most the speed) and pushes with `(wanted − velocity) × mass × 4`; the position integrates in steps of at most 0.4 s. Behind the character the preferred spot is (0, 1.5, −4.5) from the look target (the `PreferredCamPos*` / `PreferredCamDist` settings), and when the look target cannot see it a bisection finds the nearest seen point of the way.
- **Turning and zoom:** the rotate keys turn the camera round the target by 0.02 rad a frame (scaled to the original's 60 frames); the mouse look is the same turn (the client's 0.005 rad a pixel); the zoom keys and the wheel move the camera along the view at `ZoomSpeed` (10 units a second here; a wheel notch is a quarter second), the distance kept within 0.8 and 25; zooming in past 0.8 goes to first person and zooming out of first person comes back. Reset (Numpad5) puts the camera behind the character at its horizontal distance.
- **The view** is 90° across (`SetViewPlaneWindow(1.5708, aspect)`), the vertical field following the window's aspect — wider than the client's 0.9 rad vertical was.
- The free camera's obstacle sensors, its line of sight (`n3Playfield_t::LineOfSight`: the zones' surfaces and the tile-map ground tried both ways, a dungeon's rooms and their doors; the props' meshes where a playfield has no surfaces, `Collision::line`), its lift within 0.4 of the surface's closest point and its own collision (`Vehicle_t::EnsureSurfaceAlignment` after every step of its vehicle, a body without gravity) and the rooms' camera attractors are ported (`camera::Surface`, the viewer's `CameraSurface`). The look target is the body's camera attractor as `n3Camera_t` smooths it (FUN_10020bdb; `camera::LookTarget`): `Attractor31_camera`, which no player body has, so the head's (`AttractorMesh::GetName(0)`), its x and z held as first found and its y live, a fifth of the way a frame in third person, at once in first person, or held until it drifts over 0.25 and then chased a hundredth a frame with `UseNoBobCamera`; never under 0.3 above the feet. Not ported: the two-shot framing. See `ORIGINAL_AUDIT.md`. Tried live (2026-11-09, `MAO_CAMERA_SCRIPT=<seconds>:<action>[/<seconds held>],…` presses the camera keys by themselves, `screenshot` and `log` among them, for a test without a keyboard): Beetlez in Old Athens's plaza orbits at the preferred distance looking at the head (1.54 up), pitched under the character the camera is held on the ground a tenth above it, zooms out along the ground and up overhead; Poisonouss, drawn at its Scale stat beside a structure, had the camera pulled in to two units by the blocked line of sight with the target at its scaled head; a sweep past the whompah's pillars never reached them, so the sweep along a wall is unseen live. Two of the day's live sessions ended with the server closing the connection with no message before: once right after a door teleport try at (2611, 1482) in Greater Tir County, once after ten minutes with the character standing still; the cause is unknown. Captures of the original at those doors the same day (`captures/door*`, kept out of git): the border doors between Tir County and Greater Tir County at (2449, 1461), (2612, 1458) and (2880, 1330) zone inside the connection with the same 0x16 report our client sends, crossed eleven times without trouble; the Tir city door at (2207, 937) hands off by system message 0x3c to another game server, to which the original logs in straight away with the character and its session key (what `hop_direct` does — seen working live the same evening: Humility's door out of Borealis at (680, 73, 538) sent us to another server and she arrived in Stret West Bank at (1283, 0, 2893) by the Whom-Pahs and played on); the kind-14 operator frames go unanswered by the original too; and under trace our client crossed the border doors three times and stood by for ninety seconds without a close, so the drop is not reproduced.

## The map, from the original's data

The original's planet map (`cd_image/textures/PlanetMap/normal/`: an index of zoom levels, 128×128 PNG tiles stored column by column in `PlanetMapGfxNormal.bin`, and `MapCoordinates.xml` placing each playfield) is read by `mao_formats::planetmap`. A world position lands on it as GUI.dll's `PlanetMapView_c` places it (FUN_1004bedd): the playfield's `x`/`z` plus the position, less the map's origin (31022, 24880), scaled by the level's `MapRect` over the map's extent (18751 × 24115) and the playfield's scales, north up. For any playfield in that file the corner map and the Ctrl+6 overlay now show this picture (about 21 world units to a pixel, so the overlay zooms between half and eight times that and the corner map spans 600–8000 units); interiors and instances keep the client's walked map. The `PlanetMap` window of the original (its own zoom levels, centre on quest) and the Shadowlands set are not done.

## The weather, from the original's code

`mao_formats::weather` is `Environment_t`'s weather object: an R250 generator (the 250-word table from Gamecode's `.rdata`, the seed added to one word), seeds from the area's environment bytes and the day count, a 19440 s schedule in three thirds of states 120–1200 s long, each state's own 16 floats and 8 words, and FUN_100bcec6's roll: clear, fog, mist, clouds or one of four precipitations by the area's weights (bytes 0–7, doubled plus 100 when above 1), faded in over a quarter to all of the state's length, the wind, darkness, thunder and lightning values after. `environment.rs` turns the blended table into one fog as `VisualFog_t` gathers them — the weather fog (grey 0.7 × [1]), the mist (blue-grey by day, grey when dark, × 0.7/0.8/0.9, weight 0.8 × [0]), the four precipitations' fogs (orange, dark brown, dark red, green at × 0.8, weight 0.7) and the area's own fog lit by `min(day, 1 − darkness × 0.8)` — and dims the sky by `1 − darkness / 2.333` (the original's `VisualEnvFX_t` +0x1134). The sprites of rain and dust, the cloud layer and the lightning are not drawn. Because the seeding barely changes between days, an area's weather pattern repeats daily; Mort's areas, for instance, are under some dust or fog most of the time by their weights.

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

The single-quad effects (`_GfxControlSprite_t`, kind 1012: 27 running effects name one of its 16 templates, the tower beams and the halos among them) are played as the original runs them (`mao_formats::gfx::Sprite`, `effects::sprite_particles`, from Gamecode's FUN_100f6d2b and displaysystem's `gfxsprites.rs`): one quad of the template's material at the attach point plus its offset, cycling over the template's period for its count of cycles or its duration, its width, height, colour and sheet frame running between the template's start and end values as its flags ask, with the sinusoidal swell of the pulsing ones and the random texture spot of the shimmering ones. A `GfxVisualSprite2` template is a billboard; a `GfxVisualSprite3` one stands on its height and turns about it to face the camera, which the renderer's particles now do when given an up without a right. Not done: a standing quad fixed in the body's own frame, the separate height of a billboard, and alpha blending.

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
`VoiceSndFxHearGuildOn`, the team by `VoiceSndFxHearTeamOn`, from `LoginPrefs.xml`), and the sender was not heard in the last
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
sex's voice in your `VoiceSndFxType` mood (`CharPrefs.xml`, simple by default), refuses one you don't have ("You do not have that
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
- **Walk and run** (`MOVEMENT_TOGGLEWALK`, default key here Numpad /, rebindable as `toggle_walk`): walking is 1.5 a second
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
  Whom-Pah") open as the character comes within 4 m and close 5 s after, the other doors on a use as before.
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
  two has the item in hand); Ctrl+double click still puts an item into an open trade.
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
