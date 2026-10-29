# What changed overnight (2026-10-28) and what to test

The overnight run went through effects (particle system), then the GUI controls, then combat. Each item was read from the
decompiled original (`exports_*`) and the drop-ins, ported, unit-tested and checked offscreen where it can be drawn. Little of
it has been seen live: that is what the morning test is for. Commits in order, oldest first. The three hoverboard commits in
between (1960b16, 10bd999, 9a7beba) came from the other session.

## 1. Effects (nano and item visuals)

| Commit | Effect kind | Where to see it |
|---|---|---|
| 94fce6f | 3034 Shield2: a shell over the target's body | shield/reflect nanos on a character |
| b17c213 | 3025 EffectMesh: a named mesh as a shell | buffs that wrap a mesh round the body |
| 55086b0 | 3000 ShockWave: ground rings and light cones | AoE nukes, landings |
| bd5405e | 3039 Trail2: exhaust ribbons | hoverboards/hoverbikes moving |
| d891a40 | 1029 VulcanRocks: rocks thrown up that bounce and rest | earth/rock nukes |
| 869f2e4 | 3024 BParticle (kind-8 pops) | hits that "pop" |
| 7b5a8f8, d925729, 1a6902b | 1025/1019 flechette streaks, 1022 Tracer3, 1007 Nano0 | projectile nanos in flight |
| d601b51 | 3036 Toggle: effects that start when a vehicle moves off | hoverboards starting to move |
| 3130396 | 3020 TParticle streaks | various hits |
| b5945df | 3006 SkyFlash: cones coming down from the sky | "from above" nukes |
| b91adf9 | 3033 Spiral2: ribbons spiralling round the point | buff casts |
| 689ed7e | 3031 TParticle2: particles as streaks | sparks |
| a0fcadd | 1004 Fire: the Rage buffs' flames off the back | Berserk / Incandescent / Fell Rage |
| 226a1d9 | 3032 GroundShake: the camera shakes near the effect | big nukes landing near you |
| d3df9c3 | 3030 GroundGrid: a grid sheet on the ground | some area effects |

**Test:** cast or watch nanos and specials and look for anything badly placed or stuck. The ledger
(`docs/ORIGINAL_AUDIT.md`, item 10) lists the kinds still not ported, all used by fewer than eight programs.

## 2. GUI controls in the original's art (`chrome.rs`)

| Commit | Control | Test |
|---|---|---|
| de24640 | Scroll bars (`chrome::scroll`): `SIMPLE_SB` arrows and knob, no track | Skills, Options key list, chat, inventory backpack: click the arrows (5 points, holding repeats), click above or below the knob (20 points), drag the knob; the wheel as before |
| 9aa227f | Tabs (`chrome::tab_view`): the TabView strip, overlapping tabs, the border round the content, Verdana 13 | Actions (Combat/Perks/Social), inventory wear tabs (Weap/Cloth/Imp/Social), chat tabs: a tab opens on the press |
| 50175ed | Buttons (`chrome::button`): `BORDER01` at rest, `BORDER04` pressed, the `BORDER03` frame under the pointer | mail, shop, missions, loot, friends, NCU, options. Button rows are taller (about 42 points, as the original sizes them) |
| e963950 | Check boxes (`chrome::checkbox`) | Options: auto-loot, original chat font |
| 73ea999 | Text fields (`chrome::text_edit`): the `INSET` frame | find boxes, friends "Add", mail To/Subject/body, the skin folder |
| e9986bc | Dark themes (AOU Translucent): controls in the theme's cyan, inactive tab names readable | switch between the AOU and original themes |

**Things to look at:** the tab text (the original's font 5 is Verdana 13, read from Windows' fonts); whether the inventory's
four tabs are too cramped (the wear column is narrow, so their names are cut); the taller button rows.

## 3. Combat

| Commit | What | Test |
|---|---|---|
| c962627 | **Position reports as the original** (every 5 s running straight, on a turn of the view; not every 0.4 s); the walk/run mode told to the server on entering (it remembered walk from a test and pulled the character back) | run far in a straight line and with the mouse; no rubber-banding |
| (next) | **Drop-down lists** (`DropdownMenu_c`) and the pop-up list (`PopupMenu_c`) in the XML windows | open the looking-for-team window: each drop-down opens its list, hover highlights, a click chooses, a click outside closes |
| e490c0b | **Scroll views in the XML windows** (`ScrollView_c`/`ScrollViewChild_c`: bar, wheel, clipping) | a long list in an XML window (research, character selection) scrolls by wheel, arrows and knob |
| 889349c | **Text fields** as GUI.dll's `TextInputView_c` (focus, typing, selection, numeric fields, labels) | open the mail or market deposit window: click a field, type, Tab between fields, Escape; game keys must not move the character while typing |
| 01780a8 | **Doors follow the server's open/closed word** (DoorStatusUpdate decoded) | in a shop, see whether doors open and close by themselves when others use them |
| c3720bf | **Interior floors for the vehicle**: shops and apartments stand on their rooms' floors (they had none on the new path) | walk round a shop, up its ramps |
| 72e6ba6 | **Movement as the original's vehicle** (sphere sweep and slide, ground from three rays, 60° footing limit, steps, falling, the original's jump height from Strength + Agility, gravity 20) | walk into walls at angles, up steps and slopes, off ledges; jump (height by stats); no rubber-banding |
| 61f03f2 | **Swimming** as the original (deeper than 1.19 m, swim speed `RunSpeed/440+3`, swim animations, 0x1a/0x23 to the server); **ways out** of zones decoded and used for zoning; doors zone within 0.5 m | swim in the Old Athens lake; walk out; walk through a door and a zone exit |
| bb3decf | **Collision from the original's surfaces** (type 0xf424d, all 237,877 decode): walls and floors as the game has them instead of guessed from the visible meshes; blocked steps slide along walls | walk into building walls, cliffs and rocks; foliage no longer blocks; no new rubber-banding |
| 837a448 | **Movement as the original's vehicle**: half a second to reach full speed, instant stops, a slight curve when turning on the run; the original's turn rates (3.5 standing, 1.5 moving, or from TurnSpeed); walk/run toggle on Numpad / (walk 1.5 m/s, the walk animation) | run, stop, turn while running; toggle walk with Numpad / |
| b1412ba | **New Character, finished off**: the preview shows the chosen head, build and height; Create goes straight into the game with the new character, with the original's "Welcome to Rubi-Ka" loading picture (two times in three) | make a character and watch it enter the game |
| 83b1792 | **New Character**: the character screen's New Character button: breed, profession, appearance (head, height, build), name with Suggest Name, Create; the original's texts; made with the original's message (tried live from the command line: Maotestone) | make a character, try a taken name, try Suggest Name |
| df8aaf2 | **Loading screen**: the original's (the AO logo on black, "..Anarchy Online is loading.."), tinted blue; fades in while entering the game and out over 7 s once the world is drawn; also covers zoning | log in; zone |
| d97f57a | **No shuffle when you stop**: walking and running play only their loop and switch straight to standing on a stop, as the original; the character no longer walks in place for a moment or slides on and steps back | walk and run, then stop, from several speeds |
| d97f57a | **The camera after zoning**: a new camera just behind and above the character, which then pulls back, as the original (it used to fly from the old place) | zone through a door or a zone line; log in |
| 5e5c076 | **Smoother frames around characters**: their meshes are written into the last update's buffers instead of new ones (half the upload time, fewer hitches) | characters and monsters should look and animate as before |
| a312778 | **About three times the frame rate**: the sun's shadow map is kept until you move 8 m or the sun turns (55 → ~165 fps standing, ~140 walking, measured live); the network handles every message that has come before the frame's work (no queue in busy areas); `MAO_PROFILE=1` prints where the frame time goes | play as usual: shadows should look the same; watch for shadows lagging or popping as you walk |
| 79523b7 | **X just sits** (and `/sit`): sit down or stand up, no logout; /camp is the only way to log out | press X, wait a minute, press X |
| 0cd8a0e | **/camp logs out as the original**: sits down, "Timed logout started. This will take 30 seconds.", a "Logout" timer bar in the middle of the screen; X or moving stands up and aborts it; refused in a fight; window positions saved when the game closes | /camp and wait; /camp then press X; /camp while a monster hits you |
| bd86cca | **Corpses named as the server names them**: "Remains of Leet" in the target bar, over the body and on the loot window | kill something and select the body |
| 3e1a3c5 | **Dead monsters stay dead**: a killed creature keeps lying where it fell (and can be looted) until its corpse rots, instead of standing up when the server swaps the body for the corpse; despawned characters no longer linger | kill something and wait half a minute; press E on it |
| 315aa7b | **Creatures walk between server updates**: the server's `FollowTarget` paths (wandering at 1.5 m/s, faster ones learnt from their paths) and `SetPos` are now followed, so monsters move smoothly instead of jumping, and Tab distances, /follow and corpse matching use where they really are; an empty corpse opens and closes after a second as the original's | watch monsters wander; fight one that runs at you |
| 12770b4 | **Combat fixed live (with Sadlee)**: Tab and Q work again (egui no longer takes the keys once a button has focus; the chat gives the keys back after a line is sent); the server's range lines ("You are too far away") show in chat; /follow on the monster you fight closes to 2 m (3.5 m was past a fist's reach: no hits, only "too far"); the connection no longer drops mid-fight (Windows read time-outs left the socket broken: error 997 or a garbled frame; reads now poll) | Tab, Q, then /follow: it walks up and the hits land |
| 12770b4 | **Looting**: using (E, double click) a dead body opens its corpse (the server despawns the body just before announcing the corpse; the corpse is named "Remains of ..."); the loot window is the original's container view, an icon grid three to a row (`InventoryView_c`), titled with the corpse's name, double-click an icon (or its menu) to take, closing a second after it empties | kill something, press E on the body, double-click the item |
| 4a930f2 | **Target's target**: with Options "Show Target's Target" on, each target bar shows under it whom that character is fighting (`Targetstarget`, FUN_100741d8) | turn it on and watch a monster fighting someone |
| ae6d488 | **Two target bars as the original control center** (`CCTargetControl_c`): a friendly bar for your selection ("Nano Target" / "Selection") and a hostile one for whom you fight ("Fighting Target", "Nano & Fighting Target" when the same), the fought name in red, the health bar in the consider colour; both start where the original puts them (a quarter and three quarters across the top) and can be dragged | select a friend and fight a monster: two bars; drag them |
| 898adbf | **Casting checks as the original** (Gamecode FUN_1004fbe4): moving, jumping/falling or dead stops a cast with the original's lines ("Unable to execute nano program. You must stand still.", "You can't execute nanoprograms while falling!"), and too little nano says "You need at least N remaining nano energy..." (the program's cost × your cost modifier, at least 50/45/55% by breed) instead of sending | cast while running; cast with too little nano |
| e8ae7ca | **Tags over characters as the original (`Indicator_t`)**: the selected character gets a tag with the selection brackets and a health bar coloured by "consider" (green → yellow → red by its level against yours and your XP kill range, grey below it); the one you fight gets the attack brackets; the old names over everyone are gone (Options: "Show all names" brings plain tags back, "org name over head" adds the organization line). Your own death stops your movement until you are revived or zone | select a monster and look at its tag and bar; fight it; turn on Show all names; die and try to move |
| cc66296 | **The attack key and Tab as the original**: Q while fighting stops the fight (`StopFight`), else attacks the selection, with the original's refusals ("You can't attack yourself", "Target is already dead", "Unable to attack target", "Please wait until previous action..."); X (sit) stops a fight first; Tab/Shift+Tab step through the hostile characters within 100 m by distance past the current target and wrap, Ctrl+Tab the friendly ones; hostile/friendly is the side (the full update's packed word: monsters 3, Clan 1, Omni 2, neutral 0), also for the map's dots | press Q at a monster, again to stop; Tab through a crowd and watch it step outward; Ctrl+Tab in town picks guards of your side |
| 6256efa | **Attack swings**: `AttackInfo` and misses make the attacker swing its weapon (pistol shot, rifle shot, blade slash/stab, two-handed cuts, martial-arts punches and kicks), at the original's speed scaling | fight something, watch yourself and others |
| 178f42c | **Special attacks** swing their own class (Burst, Full Auto, Aimed Shot, Fling Shot, Fast Attack, Sneak Attack) | use specials from the hotbar |
| bb4cb96, b356f25 | **Weapon stance in a fight**: a fighting character with a weapon stands in the weapon's idle (`idle-smallarms`, `idle-blade`, `idle-rifle` ...). Fights tracked from `Attack`/`StopFight` | start and stop a fight, standing still |
| 8d1871e | **Draw and holster**: starting a fight plays the weapon's start once, ending it the stop | as above |
| 64a056c | **Out of a fight**: a rifle is held two-handed (`idle-2h`) and walked/run with `walk-2h`/`run-2h`; bows and bazookas their own idles | wield a rifle and walk |
| 1f8c538 | **Deaths**: the dying play their death (`die-knees`, `die-shot` ... as the server says) and lie until despawned (~10 s), instead of standing on | kill something |
| 888a011 | **Creature swings**: monsters attacking play their own attack animation | get hit by a monster |
| 2c2d490 | **Combat tab in the original's words** (`text.mdb`): "You hit X for N points of melee damage.", "X hit you for ...", "You tried to hit X, but missed!", "You hit X with nanobots for ...", "You were healed for N points.", "Critical hit!" | fight; compare the Combat tab with the original client's |
| e1b8a69 | **Shields, absorbs, XP in the original's words**: "Your damage shield hit X for N points of damage.", "You absorbed N points of cold damage.", "You received N xp." | kill something with a damage shield up; check the XP line on a kill |
| d129213 | **Facing the target**: a standing fighter turns to face its target (every 0.67 s, when more than ~15° off, at 7.2 rad/s) | start a fight with the monster beside or behind you and stand still |
| 0b24018 | **NPC animation sets read exactly**: an NPC's set now keeps every variant of an action (a humanoid NPC's 90 socials, its 3 deaths ...), each NPC picking its own | look at a crowd of the same NPC: their idles/deaths may differ now |
| 8d71c02 | **Rifle wield out of a fight**: equipping a rifle plays `2h-start` before `idle-2h`, unequipping `2h-stop` | wear and remove a rifle while not fighting |
| b3316c0 | **Reload / stop lines**: "Reloading..." then "Reloading completed" (System tab) as the original; no more "You stop fighting."; NPCs swing at their animation's own speed (c8c2594) | reload a gun |
| b55e7f0 | **Nano casts**: casting tells "Executing Nano Program: <nano>." in the Combat tab (the original's line); the client's own "X casts Y on you" lines are gone (the original has none) | cast a nano |
| 2c2d490 | **Fix: `HealthDamage` direction**: the floating numbers and health bars now follow the one hurt (the message's sender), not the source | your health bar should no longer jump to a monster's health in a fight |

**Looked for, not found: a muzzle flash on held guns.** The `muzzleflash` effect (an `eff_muzzleflash` connector → effect
template 2000, kind 1006, `_GfxControlFlare1_t`, which bursts on its own timer while its mesh is shown) belongs to statel
meshes such as turrets. The guns' held meshes (the Peregrine's 264695, the Shark's 262556) have only `Attractor01_weaponfire`,
and their templates name no effect: their stat 420 picks the fire *sound* (FUN_1009d1f1). **Not done:** the
hit/impact sounds (**the client has no audio at all yet**: sounds need a new audio dependency and the sound formats, so that is left for you to decide), and the nano-cast lines in the original's wording (still the client's own).

## 4. Sound (rodio, the original's SandyInterface)

Added after the overnight run, on request: audio through rodio, ported from the `sandyinterface` drop-in. Newest first.

| Commit | What | What to try |
|---|---|---|
| 2ac6ed8 | **Item Shop and Daily Rewards** from the character menu open the original's web pages in the same window | open each from the menu (the shop says it is unavailable; the rewards site may be down) |
| 1d9144c | **The market terminal**: right-click (or E on) the Market Terminal and the "Market" window opens the original's market web page (WebView2, the game's login headers sent to the site through a local proxy); the page's "Deposit to Market" opens the deposit window (drag backpack items in, credits, Send) | use a Market Terminal (Old Athens has them); search the market; if the page shows "400 Bad Request" the headers were refused: tell me what it says. Try Deposit to Market with a cheap item |
| 8e03852 | **Picking everything, T, terminals' answers, the shop window**: the mouse picks along the camera's line (the original's `n3Camera_t::SetMousePos`) and now reaches the playfield's own terminals, mail terminals, mission booths, vending machines and yourself; right click uses them (a mail terminal opens the mail window, a mission booth the mission window); **T** shows the target in the Info window (a terminal's description, a character's level/profession/health); the server's packed feedback (`~&...`, LDB.dll's `RemoteFormat`) reads as text ("Target side must be clan!"); vending machines are drawn; the shop window is the skin's `TradeGUI.xml` `ShopTrade` with Shop / Bought / Sold lists and the original's prices | click an insurance terminal, a mail terminal, a mission booth, yourself; press T on each; right-click a vending machine: double-click stock to buy, drag an inventory item to Sold Items, Accept |
| 8bdd9fc | **The original mouse**: left click selects, left drag looks round (camera), right click uses, right drag turns the character (Shift/Ctrl: camera only), wheel zooms, middle button resets the camera; both buttons no longer run; the AOReloaded options page removed | try each button; hold both: you stay put |
| 65f40e1 | **Option panel settings act**: Effects switches (buffs, nano effects, tracers, environment, others), Shadows, Mouse look inverted | Options > Video > Effects: untick Show Buffs (nano visuals vanish), Ground-shadows; Controls > Mouse: invert |
| 783b3c0 | **Right click uses** what is under the pointer (GUI.dll's `ActionViewMouseHandler_c`); a held right button still steers | right-click a door, a terminal, an NPC |
| a18a7c3 | **F10 opens the skin's option panel** (AOU skins), with a Client settings link; Escape closes it | press F10 |
| d324f49 | **The skin's option panel**: Options in the control center opens `OptionPanel/Root.xml`'s pages (Audio, Video, GUI, Controls ...) with check boxes, sliders and radio groups; changes are saved to `prefs.xml`; the sound volumes and switches work | AOU theme, Options > Audio Settings: move Master/Music volume, switch music off; restart: the values stay |
| 526a2e9 | **The strips' contents**: left strip NCU used/max and credits; right strip the AGG/DEF slider, sent to the server when let go (`SetStat` 51) | look at the strips (AOU: left strip top-left); drag the slider, check the server keeps it (relog) and attack speed changes |
| 2168275 | **Movable bars and compass**: health, nano, XP and alien XP are separate bars (the original's CharBar windows) you drag anywhere, under the target frame at first; the compass drags too; places are remembered | drag each bar and the compass, restart: they stay where you left them |
| d8573e8 | **The wings, bars, strips, target buttons and rollup placed by the skin's ControlCenter.xml** (its docks) and shown by its values; the rollup folds the wings | switch themes (original, AOU): each lays the control center out its own way; click the rollup button above the right wing |
| 9c24e07 | **The skins' own control center**: the AOU skins bundled (`assets/gui`); `ControlCenter.xml` and the menu scripts drive the bottom menus (AOU: View/Team/Maps/Options/Skills/AOU text buttons; Default: the icon menus), menus open above, entries open the client's windows | theme `aou_translucent` (or any AOU) in the options; click View, then Inventory/Skills/Stats; `/script` entries in AOU's menu run as chat commands |
| 0111474 | **Chat markup and item links**: chat lines with `<font>` colours and `<a href>` links drawn as the original's text view draws them; clicking an item link opens its Info page | someone links an item in chat (or `MAO_UI_PREVIEW=chat`); click it |
| b99ab62 | **Info view, DPS and nano defaults**: weapons show "Dps: min-max(crit)" from your initiative; nano pages use the game's defaults for missing stats and the duration of their repeating spells | Info on a weapon; Info on a heal-over-time nano |
| 45708bd | **Info view, nano pages**: right-click a program in **Nano programs**, **Info** — nano cost (red when short), NCU, school, stacking order, nanolines, duration, range, speed, cooldowns, attack/defence skills, on death/cancel, requirements; weapons now list attack/defence skills too | Nano programs window, right-click, Info; a weapon's Info |
| e2b01c0 | **Info view, armour and implant pages** (no charges) | Info on armour or an implant |
| e190df2 | **Info view, weapon page**: ammunition, damage, speed, equip delay, initiative, range, dual wield, specials, damage type | Info on a weapon |
| ff0eaef | **Info view modifiers**: the item page lists what the item modifies ("On User: Modify Fire AC 819") | Info on armour or an implant |
| 49ef4da | **The info view**: GUI.dll's info window with item pages (name, rarity, QL, requirements in the original's words, location, description) | right-click an item in the inventory, **Info**; try Back/Forward after opening two items |
| 8b79b1a | **GUI.dll's view system, step 3**: the text views' markup, word wrap and links (`HTMLParser_c`, `TextRenderer_c`) | `MAO_UI_PREVIEW=view MAO_VIEW=Skills.xml ... --image x.png`: the Skills window's help text as the original shows it |
| 28a030f | **GUI.dll's view system, step 2**: XML windows drawn and clickable (border views, buttons, check boxes, pictures, plain text) | `MAO_UI_PREVIEW=view MAO_VIEW=Skills.xml mao-client <game> map 566 --image x.png` |
| ed626d8 | **GUI.dll's view system, step 1**: the view tree, its horizontal/vertical layouts and the XML loader for the skin's `Views/*.xml` (`gui/`); not drawn yet | nothing to see yet |
| e963eaf | **The game's resource index repaired** (your install, not the code): 1836 of the 2006 index entries that pointed at no record or another texture now point at their own records; the old index is kept as `ResourceDatabase.idx.before_repair`. 170 textures have no record anywhere and stay missing | textures that were white or wrong (e.g. some sky/pinging glows, sewer tiles): in the new client and in the original; if anything breaks, copy the `.before_repair` file back over `ResourceDatabase.idx` |
| 8106cd3 | **Cloud wind drift**: the single clouds drift with the weather's wind; texture transforms followed (the clouds' and horizon's stay still, as in the original) | in a windy weather, watch the cloud puffs move while standing still |
| 41fafd5 | **Single clouds and the scripts' light**: cloud puffs round the camera; the world lit in the scripts' ambient and sun colours, hazed in the atmosphere's colour; the sky scaled by the view distance setting | look up and around outdoors; compare dawn, noon, dusk and night lighting; move fast and watch the clouds drift |
| db3b5ef | **The scripted sky**: star dome, moons, atmosphere gradient, two suns, battle stations, clouds and horizon from the FXS scripts; the sun direction (and shadows) from the solar model | look up outdoors at day, dusk and night; watch the suns set around 20:30 game time; shadows should follow the sun |
| 1c56f81 | **FXS effect scripts**: the playfield's tweak scripts load and run each frame; server script events (alien flybys) run and their sounds play (nothing drawn yet) | `MAO_FXS=1`: the log lists the effects started on login and any script errors; listen for flyby engine sounds during an alien event |
| 05e25c1 | **Land control music**: with Notum Wars, a district with a land control range plays `Landcontrol_neutral` | walk into a tower field district (e.g. a Notum Wars area) |
| 219c623 | **Landing splash and district music overrides**: a landing at a water's edge splashes; a server music override (`ScriptIIR_t`, e.g. at a city event) plays instead of the district's music | jump at a shallow shore; watch for "ScriptIIR" events in a city (none in the captures yet) |
| eb604d5 | **Weather music and liquid steps**: the district plays its rain/snow/storm music past 0.4 of the weather; steps in water sound wet (8–11), no landing sound in water | stand out in a storm and listen for the music change; wade into a lake and walk |
| ac6b5b4 | **Lightning's splash**: a bolt over water splashes on the surface; rocks falling into water splash | watch a storm over a lake or the sea |
| fdec0fc | **Emotes**: `/wave`, `/bow`, `/emote dance-name`, the hotbar's social actions; `/me` text | `/wave`; `/me waves`; drag a social action to the hotbar and press it |
| 857166d | **A bolt's rocks**: lightning bolts throw little rocks from where they hit, which bounce away | watch a strike near you in a storm |
| a58ce7e | **Vicinity speech sent**: plain text, `/s` and `/w` are said to the people around; `/voice` lines go there too | type a line; `/s hello`; `/voice heal`; check that others (or the line coming back) show it |
| 2fe5f13 | **/voice**: speak a voice line to your org channel; you and org mates with "hear guild" on hear it | `/voice heal` (or help, inc, no, run, yes) while in an organization |
| 5f8561a | **Lightning's light and jolt**: strikes flash across the sky with bolts to the ground; a near one shakes the camera and rumbles | stand outdoors in a storm; watch the sky |
| acff03e | **Sprung blast**: a sprung mine or trap bursts in a flare of streaks as it explodes | spring a mine or trap |
| 6f8b98f | **Thunder**: storms thunder, near strikes loud and soon, far ones quiet and late | stand outdoors in a storm (rain with lightning weather) |
| 649af9a | **Vicinity speech and its voice lines**: others' vicinity chat shows as `Name: text`; a voice line in it plays when "hear vicinity" voices are on | stand by someone who says something and sends a voice line (turn on `VoiceSndFxHearVicinityOn`) |
| a433a4a | **Mine bleep**: an armed mine bleeps once a second while you stand in it, higher the closer you are | walk into a mine in a mission or dungeon |
| 5929aa8 | **Chat voice lines**: a voice command another player sends in a group channel plays its breed/sex/mood voice | have someone send a voice line (heal, help, run...) in a group you're in, from the original client |
| 44fea07 | **Hazard sounds**: mines and traps explode when sprung (with the line), disarm lines, a mine's malfunction, central controllers' idle hum and shutdown | find a mission or dungeon with mines or traps and spring or disarm one |
| 7901b85 | **Level-up and mission jingles, the quake**: the level-up and shadow-level jingles, the mission-complete jingle by side; quakes in quake-prone areas shake the camera and rumble | level up; finish a mission; stand in an area with quakes |
| 41be03c | **Nano and effect sounds**: gather, cast, travel and hit sounds of each program; effects' own sounds | cast nanos on yourself and on a monster |
| 41be03c | **Environment sounds**: the district's background by the time of day, the rain, the winds (and sand and fallout storms) | stand outdoors by day and by night; in bad weather |
| 36b49df | **Music**: the original's interactive music (SIMPlayer): startup music at login, the district's music by the time of day, battle music in fights, cross-fades and pauses | walk between districts; fight; `mao-client "<game>" music "plains\day"` |
| 742e3a9 | **Death sounds**: a creature's own, else a man's or a woman's | kill something |
| 9f7aa87 | **Shots and hits**: each swing's `attack` note plays the weapon's sound; hits play the impact and the one hit's grunt (by sex) or a creature's own impact | fight a monster with a gun and with your fists |
| 9d338fc | **Steps, swims, landings and swishes** from the animations' notes; NPCs with animation sets play their own (idles, attack starts) | walk and run past others, jump, punch; listen to monsters |
| 1893cf0 | **The sound system**: the source files, the engine (13 channels, priorities), distance volume, variants and children, the frame's play list; the listener follows your character | `mao-client "<game>" sound SM_Sandy_Gui_Mission_Complete` should be heard; `MAO_NO_SOUND=1` silences the client |

## How to run

The release client was rebuilt and restarted after every commit:
`target/release/mao-client.exe "<game>" play`. Unit tests:
`MAO_GAME="<game>" cargo test --target-dir target-dev -p mao-client -p mao-formats -p mao-net` (all passing).
