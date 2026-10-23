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
Collector). Not done: per-playfield sky and fog, bloom.

## 4b. Shadows and water
**Shadows (done):** a 4096x4096 sun shadow map is drawn each frame around the
player (70 units; the whole scene in an overview), snapped to texels so it
doesn't shimmer, with a 3x3 soft comparison, a normal offset against acne, a fade
at the map's edge, and plants bending and cut-out leaves in the shadow pass as
in the main one. Parts outside the box are skipped. Verified offscreen (the
whompah plaza); `Renderer.shadows = false` turns them off.

**Water (partly done):** a translucent animated sheet (crossing swells, Fresnel
sky reflection, sun glint, fog; it flattens with distance so it doesn't
shimmer). *Where the water is* is a guess: the original builds its liquid from
"water triangles" (`VisualWater_t`) this client can't read yet. The heuristic
(`Terrain::water`): a playfield whose ground has a flat floor holding a fifth of
its points, at the bottom of the height range and well below the land, is a
seabed and the water lies 1.5 units above it. That finds Coast of Peace and
draws the shore along its beaches; Old Athens has none. `MAO_WATER=<level>`
overrides the level. Not done: shoreline foam, depth tint, caustics, underwater
view, and the true water data.

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
confined while a button is held. Not done: strafing (A/D turn; the server's
strafe messages aren't decoded), camera collision with the ground and walls,
a first-person view, an option for inverted mouse. Not tried in the live window.

## 4e. Controls added for playing
`Z` / `C` strafe left / right (the position is reported with ordinary updates; the strafe start and stop
types, probably 5-8, are a guess behind `MAO_STRAFE_TYPES=1`); `Ctrl+Shift+C` toggles walking through props (was `C`);
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
(the game's own), `aou_translucent | aou_cyt | aou_slim_cni | aou_slim_cnt`
(under `aou_mod = <the mod's folder>` or `MAO_AOU_MOD`), or any folder. It is
switchable in the options window. The mod is GPL and this client is MIT/Apache,
so nothing of the mod is bundled: the client reads the player's own copy.
**Not integrated:** the skin images (`Graphics.UVGA`) and the XML views. The
interface is `egui` (matches the renderer's `wgpu 30` / `winit 0.30` exactly),
not the original GUI engine; reproducing AO's own windows from the XML + skin
archive is a separate, large job. The AOU menus (`ActionMenu/*.xml`) are the
natural first target for a menu bar.

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

## 4j. Shop window (use a booth with `E`)

Booths (vending machines, identity `0xc75b`) are targetable (Ctrl+Tab) and usable (`E`). Using one makes the server send
`ShopUpdate` (the stock: low id, high id, quality) and a `Trade` open. The window shows the stock as an icon grid with the
selected item's name and description from the item templates; Close sends a `Trade` decline, as the original does.
**Buying and selling are from the fifth capture** (`capture-225436.log`): `Trade` action 5 (add) with
`(0xc75b, booth, 0x6f, stock index)` to buy or `(0xc350, character, 0x68, inventory slot)` to sell, then action 1 (accept);
the server answers action 4 (complete), a credits update and the item changes, and the client uses the booth again for the
next trade (so does this one). Prices are not in the messages, so none are shown. The Sell list shows the backpack (slots
from 64). Not yet checked live. Preview: `MAO_UI_PREVIEW=shop MAO_CAPTURE=<log>`.

## 4k. Chat server

A second connection to `chat.d1.funcom.com:7105` (`mao_net::chat`, decoded from the captures, login tested live): salt,
login with the same challenge response, then names, channel lists, notices and channel messages. Channel messages and
notices go to the on-screen log. Vicinity, tell and sending are not decoded yet (no capture with a spoken line);
`MAO_NO_CHAT=1` disables it, `net <game> chat [seconds]` prints what arrives.

## 4l. Action window (`K`) and the map (`O`)

**Actions.** The catalogue is what the running original client reports (`docs/GAME_MECHANICS.md`): 3 weapon attacks, 12 character actions, 35 perks, 63 social ids with 9 dances, each tile with the original's `action_*` icon (perks get a `perk_*` family by name, a guess). Click a tile or drag it onto a hotbar slot (the hotbar file now stores `special` and `social` entries). Doing one sends Stand, Sneak, Search, Reload and Start Combat; the rest (the weapon attacks, Brawl, Dimach, perks, emotes) say their network message is not captured yet. Preview: `MAO_UI_PREVIEW=actions`.

**Map.** Diablo-style fog of war: the network thread classifies the cells around the character as it walks (floor, or wall from the collision data and steep ground) and sends only what changed; walls are bright outlines on a faint floor. A corner map (210 px, 150 units across, turning with the camera) is always shown; `O` opens the translucent overlay (drag to pan, wheel to zoom, Space re-centres). Dots: players blue, friendly NPCs green, monsters red, target yellow, Whom-Pah booths cyan rings, the character a gold arrow. Preview: `MAO_UI_PREVIEW=map` (`MAO_MAP_OVERLAY=1` for the overlay). Not saved between sessions yet.

## Next
- Decode the key-binding providers (verify the assumed defaults).
- Decode the zone-line records so every booth's destination is known up front.
- Chat window, target and hotbars on the same `egui` layer.
- Real water data (decode the ground's water triangles), per-playfield sky; the AOU skin archive.

**Stats are computed.** The Stats window shows the original's values: the stored stats the server sends plus the modifiers of what is worn (`derived.rs`, from `docs/STAT_ENGINE.md`). Worn places are the weapon/utility slots, clothes and implants; each item's effects are read from its template and interpolated between its low and high template by quality; effects with a level requirement (`item::modifiers_at`: operator 1 = below, 2 = at least) are kept only when the character's level qualifies. Armour classes, attack rating and the bonuses to max health and nano therefore appear (preview: `MAO_UI_PREVIEW=stats MAO_CAPTURE=<log> MAO_LEVEL=<level>`). The perks the world-entry message lists (`FullCharacter.Perks`) are included too: each perk id's record (type `0xf4264`, instance = id) names an item template whose effects are summed (`derived::perk_modifiers`). Not included yet: research ranks and the perk lines the server has not been seen to send (see `docs/STAT_ENGINE.md`), active nano buffs, and the attribute trickle-down into skills.

**Vitals.** A small movable panel (top left, `vitals.rs`) is always on screen once the character's stats are known: health (red) and nano energy (blue), then the experience bar (gold: XP between the last level's experience, stat `LastXP`, and `NextXP`) and the alien experience bar (purple: `AlienXP` against the server's `AlienNextXP`, with the alien level underneath). Max health and max nano are the computed values of the Stats window. Current nano, alien experience and `NextXP` appear only when the server sends them (they show a dash otherwise); the alien bar counts from zero because the previous alien level's threshold is not known to this client. Preview: `MAO_UI_PREVIEW=stats`.
