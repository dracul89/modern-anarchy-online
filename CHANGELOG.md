# Changes

What each release changed, as a player meets it. The commits have the detail, and `docs/CLIENT_UI.md` says how each
thing was read from the original client. The release workflow puts a version's section at the top of that release's
notes (`docs/RELEASING.md`). Releases before 0.1.4 have their notes on the Releases page only.

## Unreleased

### Windows

- A window size larger than the screen (Run In Window) no longer makes a game window that runs off the screen and jumps when a corner is touched: the window is made to fit above the taskbar and centred, as the original does with its work area.

### Launcher

- PATCH no longer just closes the launcher: the game's patcher must run as an administrator, and the launcher started it in a way Windows refuses for such a program, so nothing came and the launcher was gone. It is now started as the original launcher starts it, so Windows asks for that permission; if it still cannot be started, the launcher says why and stays open.
- The launcher's settings are easier to read: their window is no longer see-through over the launcher's picture, the text is lighter, and settings that do not apply (the window size in a borderless mode) are faded less.
- The Anarchy Online folder the client reads the game from is now the first thing on the settings' GUI page, with a line saying it is found by itself and that Browse picks another (it was at the foot of the page, under three other groups).

### Graphics

- A nano program that takes time to cast has its caster work the nano with both hands for the whole of the cast, then throw it at the target (or take it in, cast at oneself), as in the original; before, the throw played once as the cast began and the caster stood still through the rest. (Programs the original casts at once, such as the Bureaucrat's Commanding Presence, still show no casting, as in the original.)

### Movement

- Other players move smoothly between the server's messages, at their own speed along their facing, instead of standing still and then jumping metres on every few seconds; a correction from the server is eased into rather than jumped to.
- Other players show their running or walking animations from the moment they start until they stop, and stand idle while they stand, instead of flickering between running and standing as each message came.
- Turning with the right mouse button turns the character by exactly as far as the mouse goes, and the camera comes round behind it: with the locked or rubber camera the character swung on further than the mouse went and drifted the next time the button was held.

### Interface

- The control center lies behind every window, as in the original: a window moved to the bottom of the screen goes over its buttons, wings and strips alike, no longer between its buttons and the NCU/credits strip.
- The option panel's "Number of shortcut bars", "Lock shortcut bars" and "Hide/show shortcut bars" work: more bars appear when the number is raised (the options set values the bars did not read).
- Shortcuts on the bars show their icons: special actions (Start Combat, Walk, Sit ...) no longer show their names' first letters until the Action window has been opened, and nano programs and used-up items get their icons too.
- The pointer shows what is under it in the world, as in the original: a blue arrow with a cog over terminals, doors and other things to use (grid, mail, market, mission terminals), a hand for things to pick up, coloured arrows over characters (green yourself and your team, red what you can fight, talk and trade pictures), with the mouse buttons that do it.

## 0.1.8 (2026-11-20)

### Performance

- The skills window no longer halves the frame rate while it is open: it took 8 ms of every frame (measuring its whole view twice and searching it by name a thousand times); now about half a millisecond.
- The chat window no longer searches the disk every frame (for the script names its command list offers): the interface takes about 0.6 ms a frame where it took 6.3, and the game ran at about 165 frames a second where it ran at about 80 in the same test.
- Characters coming into sight no longer freeze the game for a tenth to a fifth of a second: their textures are decoded and compressed ahead on every core, not on the frame's thread.
- The performance overlay no longer makes a hitch of its own every second: it reads the processor and memory on a thread of its own (it took about 10 ms of every second's frames, and opened with a 0.6 s freeze).

### Graphics

- Sky flash cones sized by the body (a negative radius) take the body's real torso size, not a fixed half unit.
- Effect meshes held at the feet of a bigger or smaller character are scaled with the body, as on its attractors.
- Fire effects with a gate (rage flames and the like) come in small bursts as in the original, not as an even stream.
- Spiral buff effects are sized by the body: bigger on large creatures and on characters scaled up, smaller on small ones.
- Creatures and players sitting on the ground now loop their idle there instead of freezing on its last frame.
- The ripple of a body struck by a deforming hit now has the original's shape.
- Shield effects' textures and shimmer turn with the character instead of staying fixed to the world.
- The star bursts of mode-2 nano effects flicker through their colours as in the original.
- Mode-3 star effects circle the character as in the original (they turned the other way and drifted off).
- Repeating toggled effects (flag 0x400) play again each time they end instead of only once.
- Thick clouds no longer darken the daylight beyond what the original does.
- The halo suns over a character (e.g. template 43768) are eight turning halos as in the original, not 32 still ones.
- An NPC whose update names its own head no longer shows its template's head through it (Pamela Borroel's dark hair and black wedge in Arete Landing).

### World

- Going away (/afk, or by itself after three idle minutes) on a battlestation leaves the battle, as in the original.
- While your character lies dead it no longer goes "Auto-away" by itself, and a key pressed meanwhile still brings you back from being away.

### Movement

- Sneaking backwards plays the sneak animation in reverse, as the original does.
- Taking off into flight now shows the original's short burst of light around the character, for others too.

### Fights

- Combat text floats over characters again, now as the original shows it: the amount of each hit, nano damage, heal and
  experience gain rises briefly over the one it is about, in that kind of event's colour (your own only in third person).
- A hit that lands just as a creature dies is told with the creature's name, not its remains'.
- The alien experience gained with an alien level-up is told (it was not).
- While crawling, your weapon is drawn, held and fired with the crawling animations.
- A stunned creature stops where it is instead of walking on along its path.
- Entering or leaving a PvP district now gives the original's grace and disgrace periods with their messages, and the server's changes to districts' fight modes are followed.

### Nanos

- Every nano program running on a character shows its effect, not only the first six.

### Items

- With an item in hand, a right click in the world uses it there as a left click does (on a character, an object, or the
  ground's question), as in the original. It did nothing.
- An item used on one of the playfield's built-in objects (a lock pick on a strongbox, a key on a statue) reaches the
  object: it was sent under the client's own number for it, which the server does not know, and nothing happened.
- A bank item dropped on a bag, chest or corpse window now says it can only go to the inventory, as the original does.
- More of the original's reasons are told when an item cannot be worn or used (combat mode, team, side, profession and others).

### Windows

- The knowledge window no longer refuses an item when its 30 cells are full: the board grows a row and scrolls.
- The planet map opens at the zoom level you last used (the first level the very first time), as the original does.
- Research window: a goal's requirements read as in the original, without the "Wear:"/"Use:" headings.
- Skills window: a skill set for a while by a temporary effect can no longer be raised, as in the original.

### Interface

- The camera no longer creeps towards your character on slopes; it rests against the ground as in the original.
- Dragging an entry of the control center's menus that runs a chat command (the Social menu's emotes, for example) makes a macro of it that can be dropped on a hotbar, as in the original.
- The inspect, mail, nano programs, maps, signup terminal, teleport target, laser target and mech select windows and the compass first open where the original client left them for the character.
- The control center's right target button attacks the target, as in the original (it selected the next enemy).
- A new level shows the game's notice buttons near the lower right: new skill points, and when they come, a new perk or research; a click opens that window.
- The skills window lists all its groups one under another, as in the original; a folded group kept its room, so only Abilities and Body & Defense showed, with a gap between.
- An item handed to an NPC leaves its trade bar once the trade is done; the reward that came to the same inventory place showed in the bar instead, and pressing V again would have given it away.

### Chat

- Renaming a chat window uses the original's name box; a name another chat window has is refused and asked again.
- /voice now speaks in the chat window's output group (a channel set with /ch, the team, the organization), as in the original; channels that take no voice lines refuse it.
- Emotes are refused while swimming, as in the original.
- Ctrl-clicking a name or a chat group in a chat line now acts as a plain click; Shift-click still types the tell or /group line, as in the original.
- Chat lines that hold a tab or another control character are shown instead of silently dropped.
- A chat window's menu (right click on it in the friends list) can turn off "Deactivate On Send", so the input bar stays active after a line is sent, as in the original.
- Every chat window frame, not only the first, starts where the original client left it for the character.

### Sound

- Changing the battle music setting in the options takes effect at once, as in the original.

### Documentation

- The README lists the system requirements: what the client needs, what was measured, and which parts of the minimum
  are still an estimate.

### Not yet seen in a live game

Read from the original client and tried offline or by test only: the item use on built-in objects, the bank item
refusals and the wear and use reasons, the effects changes (seen in pictures only), sneaking backwards and the flight
burst, the knowledge, planet map, research and skills windows' changes (the skills list in a picture), the chat window
renaming, /voice and the chat menu, the camera on slopes, the control center's macros and target button, the NPC trade
bar after a hand-in, going away on a battlestation, the PvP districts' grace periods and the battle music setting. The
speed changes were measured in a live game on a test account, and the level's notice buttons were seen there.

## 0.1.7 (2026-11-20)

### World

- Garbage Fleas and Waste Collectors are drawn again: since the last build only their names showed. (A creature skin
  whose picture carries see-through areas was drawn see-through; the original draws it solid.)
- It rains and snows: the weather's rain was not drawn at all (only heard, and seen as fog). Sand, dark, red and
  green storms are wind-blown streaks in their colours and tint the screen, flickering, as in the game.
- Overcast weather dims the sky and the fog; the dimming used to follow the wrong weather value.
- Lamps glow: the lights whose colour and size are set by the building itself (most of Jobe Research's, some in
  Andromeda and elsewhere) have their halos, in their colours.
- Mission buildings and shop interiors have their smoke, fire and lamp effects; missions had none, and a shop's
  were drawn somewhere far off.
- Corpses that were already lying when you arrived (or that you walked away from and back to) are drawn. They
  were only a name floating over empty ground.
- A locked door opens for a character who may pass it, as in the original: a door that needs a key item when you
  carry the item, and the locked doors inside your own apartment or organization building. A door the server
  unlocks also opens when you walk up to it now.
- Your run speed is the original's: the attributes' share of your skills no longer makes you run faster than the
  original client lets you (it did since 0.1.5).
- The mouse turns as fast as in the original (its sensitivity setting, 10 by default, now counts), the wheel zooms by the
  original's step, and letting go of either button ends a mouse look.

### Windows

- A city controller of land that is for sale opens the buying dialog: the price and upkeep, Purchase and Cancel,
  and the city's map. **Purchase buys at once, as in the game: there is no second question.** (Using such a
  controller used to open nothing and block every controller until you zoned.)
- A building item placed on the city map is locked in the inventory (a small padlock) until it is built or taken
  off the map, and the map tells the pointer's coordinates as a tip.
- The inventory, backpacks, the bank, corpses and chests have their window menu (right click beside the items):
  **List Mode** shows the items as a list with name, count and quality, sortable by its header; **Auto arrange**
  orders the grid by name, count or quality, and "Don't Auto Arrange" gives hand placing back with the items
  where they are.
- A corpse's or chest's items are shown sorted by name, as in the game.
- The wear window's menu (right click on an empty slot) has **Labels On Top**.
- Pictures inside text are drawn: the bullets before an NPC's answers, and the item icons of a mission's rewards in
  its description.
- Every window has the small menu button at the left of its title, as in the original: it shows the window's own
  menu (list mode and ordering for the inventory and containers, the labels for the wear and inspect windows) and
  Close Window.
- The inspect window has its own "Labels On Top".
- The battlestation's signup terminal has its window: your place in the queue, the side, Sign up and Leave queue.
  (Built from the original's code alone: it has not met a server yet.)
- The vehicles window (Shift+V): the vehicles your side has researched, with their descriptions, Order Vehicle and
  Leave vehicle. (Built from the original's code alone: ordering has not met a server yet.)
- More of the options act: "Zoom in and out of first person", the mouse wheel over the world (zoom or nothing),
  "Show my character in first person view", the fading of your own character when the camera is close to it, and
  the view distance and character distance sliders.
- Notum Wars: the map has its Build and War buttons, and using a tower, controller or shield disabler item opens
  the build menu with its checklist instead of doing nothing. The battlestation has its small map with the points
  and scores, the invitation to a battle and the end's message. The laser target and airstrike window is there
  too. (All built from the original's code alone: none of it has met a server yet.)
- Shops work by the original's clicks: **one** click (left or right) on a stock item puts it into the purchase
  and one click on a bought or sold item takes it out; Shift + click shows the item. Two clicks buy two. An
  inventory item is offered for sale by Ctrl + right click (or by dropping it on the window, as before).
- A shop's window no longer closes by itself when the character walks away from the shop (the original keeps it
  open; the server refuses what is tried from too far).
- Items are picked up the original's way only: **click to take an item up, click again to put it down**.
  Pressing on an item and dragging it no longer lifts it (the original ignores that gesture).
- With an item in hand, a right click on another item **uses** the one on the other, worn items and items in
  bags included; **Shift + right click is the tradeskill combine** (it was the plain right click). This is the
  original's.
- An item let go over the ground now brings the original's box: for nearly every item "this is not a fixture,
  delete it instead?" with Delete and Cancel (**Delete destroys the item**); furniture is asked whether to drop
  it. Let go over a character, an item is used on that character with the original's message.
- The skill window counts only what a worn item gives while worn: bonuses from an item's use or aura effects no longer
  show, and bonuses that depend on your profession, breed or a skill now do.
- Doors open when you touch them and shut as soon as you leave, as in the original, instead of within 4 m and for five
  seconds. Doors the game marks as locked now stay shut until the server unlocks them.
- Characters show their real bare skin: each breed, sex and Solitus race has its own instead of one tan for everybody.
- Creatures' eyes, lamps and other lights glow from their textures, and see-through parts of characters are see-through.
- The Action window lists only what your character has, in the original's order. The Sit/Stand, Walk/Run, Sneak, combat and
  Suspended Animation tiles change with what you are doing, on the shortcut bars as well.
- Each corpse, chest or other container opens in a window of its own, as in the original, instead of one shared window.
- The shortcut bars are the original's now: up to ten bars (set in the client's settings) of ten slots and ten pages each.
  1 to 0 press the primary bar's slots, Shift+1 to 0 pick its page and Alt+1 to 0 pick the primary bar. The smart hotbars
  are still there as a setting.
- Windows fade when the mouse leaves them, as in the original; the pin keeps a window solid (and the frame menu's
  "Autofade Window" does the same). Escape closes pinned windows too now.
- The windows that have a help page in the game (skills, perks, pets, research and others) have its "?" button.
- Chat tabs can be dragged as in the original: right-click a tab to pick it up, then left-click on another chat
  window's tabs to move it there, or anywhere else to give it a window of its own.
- The map is the original's playfield map window (the control center's Map menu): north up, following you, dragged to
  look round, with the characters near by side. The client's round corner map and overlay are a setting now.
- The round corner map is the default again; the original's square map window is in the control center's Map menu.
- A character the original client never played no longer starts with another character's window layout: its windows open at their own places.
- The wear window is a window of its own again: Ctrl+1 and the Wear icon open it, I opens only the inventory.
- The mission booth window's slider labels and buttons use the game's own texts.
- The control center's Team > Team, Vehicle and Windows > Chat Config entries open their windows.
- The organization's leave and promotion questions come up in the middle of the screen, as in the original.
- The planet map window first opens 300 by 400 at the top left of the screen, as in the original.
- The mail inbox remembers its column widths and sort order.
- The mail window no longer shows its own status line ("Sending ...", "Mail sent.", inbox count text); the "n/30" inbox size remains, as in the original.
- The Inspect window is titled "Inspect" as in the original, and right-clicking an item opens its menu (Look At, Create Reference) instead of its info page.
- The skills window no longer shows "Changes sent." or "Reset asked for."; refusals (disabled skill, cap, not enough IP) still show.
- Inventory, backpacks, bank, loot, missions, info, tradeskill kit, knowledge and actions windows now have tabs: right-click a tab to pick it up, left-click on another window's tabs to dock it there, or elsewhere to give it a window of its own, as in the original.
- Framed chat windows fade when the pointer leaves them like other windows (pin them to keep them solid); borderless chat windows fade as a whole to their "inactive" level unless being typed in.
- The mission journal remembers your active mission between sessions and puts its waypoint back on the map when the journal opens or after zoning.
- "Update Team Members" in a team mission's menu now works (sends the original's request to the server).
- With the AutoCloseWindows preference on, opening a window from the menus closes the other character windows, as in the original.
- The shop window's lists are the original's item lists: icons in a grid with their prices (or a sortable list with Name, Count, Price and Quality columns from the window's menu), the bought and sold items in one row each.
- The NPC give-trade bar is the original's: a "Trade" tab window docked to the right of the NPC chat, an item grid of the NPC's slots, the NPC's request in place of the answers; V gives no more credits than you have and leaves the bar up, X (or Give again) closes it.
- The options window's Key bindings tab is now the original's key editor: a list of actions and their keys with Reset All, Add, Change and Clear, and the "Bind Key" box.
- The rollup column down the right edge works as the original's: 192 wide, the original's page headers, a grip under each page to size it, double click to roll a page, right-click to pick a page up and put it elsewhere (or a window's tab onto the column to dock it), the fade level from the options.
- Windows have the original's inner border and solid dark-teal body under the title.
- The planet map shows the Shadowlands map in every Shadowlands playfield, dungeons included, as the original does.
- With land control shown on the map, a light grey circle marks where a tower would stand while the tower build menu holds an item.
- Right-clicking a mission booth offer opens its menu: Look at Mission and Upload to map.
- With the mouse wheel set to scroll the side bar, the wheel over the world scrolls the rollup column; the column keeps its scroll between sessions.
- The nano programs window lays its school buttons in rows as wide as the window and shows as many program columns as fit, as in the original.
- Windows are moved by their title strip only, as in the original; dragging a window's body no longer moves it.
- A tab strip too narrow for its tabs narrows the long tabs first, as in the original, instead of shrinking every tab alike.
- The pet window's options (show buttons, fixed pet buttons, remember pet names) are on the window frame's menu icon, as in the original.

### Items

- A worn item can be locked to its place: hold the right button on it and choose Lock Item. It then cannot be taken
  off, swapped or have another item put in its place by mistake, and shows a small lock. Unlock Item, or Unlock All
  Items in the wear window's menu, frees it. "Delete" is no longer offered for an item you are wearing.
- The bank's items can be placed by hand: its menu has "Don't Auto Arrange", and it starts that way. Each item stays
  in the cell you put it in.
- `/pet` takes pet names before the command (`/pet Rex guard`, `/pet all attack`) and the commands with an argument
  (`cycle a|w`, `social <name>`, `chat`, `script`), as in the original, and says when you have no pet.
- A mission's item (Arete's "Plant a Bug" recording device and the like) now comes into your inventory.
- Items placed by hand in the inventory and backpacks are remembered for each character on its own.
- An item let go on an empty spot of the wear window goes to its own place on that page.
- Item and nano pages show skill locks (with your own lock time), stat sets and lock reductions.
- Item pages show the effects on nearby friends and foes and the personal effects, and armour and implants show the skills they are used with.
- Starting a trade with a player too far away says the target is out of range, as the original does.
- Wear and use checks now judge requirements written "on self" instead of letting them through.
- Breed and sex requirements in item tips and pages read as names instead of numbers.
- A right click on a corpse's item no longer takes it, as in the original; click it instead.
- Item tips show what the original's show: the name in its rarity colour and the item's info (damage, modifiers, locations ...) under it.
- Destroying an item waits for the server; a bag with something in it is refused with the game's line.
- The tradeskill window names the result as it will be at the chosen quality.
- An item dragged into an open backpack lands in the cell it was let go on.
- Right-click wearing an item always uses its own place, swapping out what is there, as in the original.
- Corpse, chest and overflow windows no longer have a "Take all" button, as in the original; take items by clicking them (or turn on the auto-loot option for corpses).
- Item requirements "on target" are judged against the selected target: with nothing selected such an item is refused, as in the original.
- More item and nano requirements are judged by the client: wielding a given item, having or running a nano, a perk lock, free backpack space.
- Requirement lines in item and nano descriptions read as in the original for items, nanos, perks, pets, teams and places.
- Items placed by hand in the inventory, bank and bags keep their column and row when the window is resized, as in the original.
- The wear window shows the "can't drop" pointer over places an item in hand cannot go, and a refused item stays in hand instead of going back to the inventory.
- Item and nano pages list more of their effects: casts (with a link to the nano), procs, damage and healing, nano drain, faction changes, perk locks, anchors and more.
- Walking away from the bank or from a corpse you are looting now closes its window, as in the original.
- Item pages show the shop's price (from a shop), insurance cost, a building's upkeep and size, a tower's type, fixtures, and an NF crystal's formula.
- A combine's "you need at least ..." lines from the server show in the tradeskill window's build text, not in the chat, as in the original.

### The launcher

- The character you pick in the list is alive: it breathes, and now and then waves, bows, scratches its head, as in
  the original.
- Character creation: the far suns, the stars and the intro's shuttle move, and the character being made stands
  breathing and fidgeting instead of frozen. A short or tall character is shown at the original's size, and the
  heads offered are the original's list (more of them with Shadowlands, a few odd ones gone).
- The dimension list marks a dimension newer or older than your game, and PLAY becomes PATCH, which starts the game's
  own patcher and closes the launcher, as the original's does.
- The game's licence is shown once, until you accept it.
- The Connection page's time-outs accept any number of seconds (5 at the least, as in the original), and the time-out for entering the game now follows the page too.

### Fights

- While your character lies dead the game takes no input, as the original: no keys, no clicks, no typing, until the
  character is back.
- Brought back by another player's resurrection (from level 15), your character is in resurrection shock: skills
  and abilities are lowered, by half at first, and recover over about six minutes.
- The special attacks shown are confirmed to be built the way the original builds them (the server's three plus
  what each weapon in a hand gives); two small corner cases of it were made the original's.
- A special attack out of reach says so at once instead of going to the server: Brawl reaches about 5 m and Dimach about 12 m
  between the two characters, as the original's client checks.
- The attack key refuses while swimming or flying with the original's lines, and a refusal is followed by "Unable to attack target" as in the original.
- A hit followed late by the target's health no longer shows a second "unknown damage" line for the same damage.
- Attack swings vary at random as in the original instead of in a fixed order.
- The attack key refuses team and raid members, and refuses while flying, with the original's lines; a refused attack also says "Unable to attack target" as the original does.
- The friendly targeting keys pick team and raid mates of another side and one's own pets; the hostile keys skip them.
- A special attack at a target behind a wall or out of view says "no line of sight" at once and is not sent, as the original's client checks.
- A refused nano program says why and then "unable to execute on this target", as the original does.
- Creatures' life bars are coloured by their level again, as the original colours them: the level is taken from each creature's announcement, and a monster's info packet (whose level byte is always 1) no longer greys the bar.
- Districts' fight modes are read: coming into a district of another suppression level tells its line, an attack at someone in a district of more suppression says so, and where the gas allows fights between sides a friendly nano goes only to one's own side or team.
- A nano weaker than one of its line already running on the target is refused with the game's "better program running" line.
- Nano programs are checked before they are sent as the original checks them: sitting, crawling, a vehicle, a target that is no character, a creature that is not your pet for a friendly nano, a target out of range or out of sight each have the game's line.
- While the character is feared, its special attacks, nano programs and attacks are held back as the original holds them back (a fight it is in cannot be stopped either), and the game's "fear activated" line shows when the fear begins.
- At one's own death the world turns wholly white at once when the death has played out, and is clear again the moment one lives, as in the original (no slow fade).
- Damage and healing numbers no longer float over characters; the fight is told in the chat's combat lines, as in the original.
- A special attack at an item is sent as at that item (it was sent as at a character), and an item that cannot be attacked says so, as in the original.
- The attack key now refuses as the original does: characters that cannot be attacked, surrendering NPCs, dead targets, and items that are not attackable or whose requirements are not met.
- Tab targeting now counts team mates' pets as friendly and skips characters that are still zoning in.

### Targeting and clicks

- **The mouse now works in the world as in the original.** Characters are clicked by the box of their posed body and
  terminals, doors and booths by their actual mesh; walls and the ground block the click. Your own character is on
  the click line by its distance, so a click through it takes it first and the next click what stands behind.
- **Right click no longer changes your target.** Double click uses the thing clicked; Shift + click shows its info
  page; Ctrl + click on a character switches your attack to it. Shift + right drag turns the character as a plain
  right drag does (only Ctrl keeps it to the camera).

### Login

- Play on an inactive character asks to activate it first, or says no slot is free.
- A character just created goes straight into the game, without the character list being fetched again first.
- Play shows the progress window until the character is logged in, then the loading screen, as in the original; if it takes longer than the game's dimension time-out you are back at the login.
- Deleting a character needs its name typed exactly as it is written, capitals included, as in the original.
- The loading screen's picture is shown in its own colours, as in the original (it was tinted blue).
- Character creation leaves out spaces and signs typed in a name and fixes its capitals, as the original does, instead of refusing it.
- A character the server refuses to enter now brings you back to the login with the game's error page, as in the original, instead of the launcher just closing.
- A login error's page stays until you close it, and is there again when you go back from the character list, as in the original.
- A character creation you leave keeps its choices for the next time, even after the launcher was closed, until a character is made.
- A new account with no characters now opens the character screen (with New Character) instead of being sent back to the login form.
- A character still in the world after a disconnection shows the limbo window with a countdown and Login, as the original, instead of an error.
- Character creation: the opening lines fade in and out slowly as in the original, the two lines about docking and continuing the DNA show on the way down, and after making a character the "boarding shuttle" line and the fade to black follow the original's six seconds.

### Graphics

- Creatures that fight with natural weapons (robots, automatons) swing their own attack animation, as the original does.
- A few playfields without water of their own (the Uturn and Foundry playfields among them) no longer show a flat sheet of water over their low ground.
- Far campfires, chimneys and lamps keep their fire, smoke and halo out to the view distance instead of only within 150 m.
- Effect meshes and trails attached to a body part (an arm, a leg, the spine) now show there.
- Some placed objects stored with a slant (ramps and tilted pieces of the second statel layout) now lean as in the original instead of standing level.
- Effects that the game shows only on certain playfields now appear there.
- Interiors (shops, dungeons) are seen against black, as in the original, not a very dark blue.
- Creatures without a standing or running animation keep playing their last movement, as in the original, instead of borrowing the walk.
- Far scenery no longer fades into a haze in areas without fog; it stays clear to the view distance, as in the original.
- Trees and grass no longer sway in a wind of the client's own or bend away as you walk through them; plants whose meshes have their own animation now play it, as in the original.
- Nano hit effects no longer ease in and out by the client's own fade; their particles fade as the effect itself says.
- Other players are drawn thin, medium or fat as they were made, and robed when they wear a robe; a hoverboard rider does a trick when jumping.
- The Whom-Pah pillars' blue beams and the Shadowlands' veins are drawn, as are the engine trails behind the effect scripts' ships.
- The asteroid that has its own way of being drawn (no fog) is drawn that way.
- Four dungeon layouts (playfields 1186, 1187, 4221, 9070) no longer show stray or missing props: their statel zones are read the way the original reads them.
- Cast sparkles now stream from the caster's moving hands, and hit and buff effects sit on the body part they are made at.
- Effects made by some spell lists (such as the Shadowland Anvian's smoke and sparks) now take the spell's own colours.
- In a heavy thunderstorm at most two lightning flashes, ten bolts and twenty rocks of each kind show at once, as in the original.
- The world is drawn out to the view distance and no further, as in the original (the far plane is the view distance setting).
- Tall halls and low rooms now count as "inside" up to their real height, so which rooms are shown (and the room you count as in) is right when you jump, fly or stand on high floors.
- The night sky over Rubi-Ka's outdoor playfields has its small dot stars again (with the first expansion).
- The sun's rays shimmer as in the original: each ray brightens and fades slowly on its own wave.

### Interface

- Locked actions, nanos and items show their time left over the icon in the Actions, nano and inventory windows too, as on the shortcut bars.
- The Pick-Up and Crawl action tiles work: Pick-Up asks for the selected item (or says the inventory is full), and Crawl starts and stops crawling, its tile showing the other half meanwhile.
- Shift+Enter also opens the chat input and the numeric pad's / also starts a chat command, as in the original (new settings files; an existing mao.ini keeps its keys).
- Bold and italic interface texts (the BOLD, HUGE, CC17 and ITALIC fonts of the skins) are drawn bold and italic, as in the original.
- A character's info page names the profession title and the defender rank instead of showing their numbers.
- Links in info pages, NPC dialogue and tips open as the mouse button goes down, as in the original.
- Dark skins (AOU Translucent) draw buttons, check boxes and tabs in the skin's own Default colour, as the original does.
- In-game web pages are told the system's language and country, as the original tells them.
- Stat names in tooltips and item requirements are the game's own (no more "Stat 123" for stats the game names).
- The control center's experience bar shows shadowknowledge from level 200 and stays full past the next level's figure, as in the original.
- Health and nano bars show the computed maximum even when the current value is above it (the bar stays full), as the original's bars do.
- The Stats window's duel, solo and team score bars fill toward the next PvP rank's score.
- The original-style shortcut bars start where the original client had them for the character, when it has a layout for it.
- The camera's rotate and zoom keys go at the original's speeds and follow the mouse turn and zoom speed settings.
- Talking to an NPC asks it once, as the original does; the client no longer re-asks a silent NPC after two seconds.
- Choosing a camera mode no longer prints a "Camera: ..." line.
- The message box at the bottom left is gone: messages appear in the chat window as feedback lines, as in the original, and the client's own notices (screenshot saved, Whom-Pah destinations, walking/following, nano cancelled, zoning) are no longer shown.
- The CC17 and TT_MIN12 fonts grow with the window's height as in the original (for example the control center's large numbers at 1080 lines).
- The character info page (/inspect, own info) shows the side experience bonus, your tokens, veteran and victory points, ICC commendations, Freelancers Inc. tokens, specializations, and the duel, solo and team ranks.
- Text in info pages and other read-only text views can be selected by dragging the mouse (copying it waits for the right-click menu).
- Zoning no longer shows the loading picture: the screen holds its last picture until the new area is drawn, and "Changing area" is said in the chat, as in the original.
- Window art from the skin is drawn crisp (unfiltered), as in the original, instead of smoothed when scaled.
- Reload and perk tiles show their own pictures, as in the original.
- Selected text in the interface's text views is on the original's light grey.
- Links in NPC chat and info pages answer clicks over the whole height of their line, beside the line's start, and past the end of a line that wraps into the same link, as in the original.
- Scroll bars in the game's own views (info, inventory lists, chat and the like) show the skin's pictures in their own colours, no longer tinted with the theme's colour.
- The logout bar and the attack/nano/item timer bars use the skin's timer bar pictures, with their label on the bar, as in the original.
- Popup menus too long for the screen show a scroll bar, as in the original.
- A selected row in the game's lists (friends, mission, list-mode programs and the like) is filled with the original's solid light blue, no longer a pale tint.
- /viewdist takes the original's percentage (/viewdist 60 = 600 m) and /chardist the original's value, with no limits of the client's own; the option window follows them.
- The original's window keys for the team (Ctrl+5), raid (Ctrl+Shift+R), looking-for-team (Shift+F), research (Shift+O) and knowledge (Ctrl+3) windows work and can be rebound, as can keys for the daily login and item shop.
- A key bound to two actions in the key editor does both, as in the original; OK with no key pressed adds an "Inactive" bind.

### Chat

- /ignore follows the original: /ignore alone ignores your target, /ignore list shows who is ignored, and the messages are the game's own.
- /tell with no name does nothing, as in the original (it no longer opens a window for the target).
- /inspect alone no longer describes the target; it takes a character's number, as in the original.
- Fight lines about your own pet go to the "Your pet hit by other" and "Your pet hit by nano" chat groups, as in the original.
- /help opens the game's command help in the info window, and /help <topic> (org, pet, team, raid, ...) the page for that topic.
- Using a target or a perk no longer adds a "Using ..." line, and the use key with nothing targeted no longer says so (the original says nothing).
- /fxscript with only a script's name no longer runs it (type `/fxscript <name> #`, as in the original), and no longer reports in chat what the script did.
- Sending items or credits to the market no longer adds a "Sent ... to the market." line.
- Inspecting a character no longer adds a level and health line to chat; the info page opens as before.
- /macro, /waypoint, /befriend, /viewdist, /chardist, /char&viewdist and /bug no longer add a confirmation line in chat.
- The NPC chat window no longer says how many items were given back, and its answers disappear once one is clicked, as in the original.
- /version asks the server for its version, as the original client does, instead of printing this client's version.
- The friends list flashes a character's row while their tells are unread, turns its folder's label red, and selects the row while their tell window is open; a press on the row opens or closes the tell window.
- The friends list's "Chat Windows" lists every chat window; right-click one for its menu (show, log, rename, delete, copy, new window).
- A channel's name in a chat line is a link: press it to talk in that channel, or Shift/Ctrl+press to start a /group line.
- An invitation to a private chat group asks which chat windows should show the group, and flashes the inviter's row in the friends list; with the invitation option at 2 the question waits until you press that row.
- A chat window that has no saved place first comes up where the original's does: the main one across the bottom of the screen, others and tell windows in the middle.
- With the original's chat font on, chat lines are as close together as in the original (9 pixels), and accented letters are spaced by the font's own widths.
- A friend logging on or off no longer writes a line in the chat; the friend list shows it, as in the original.
- A line typed while the chat server connection was down is sent once it is back, instead of being lost.
- Game texts with numbers are filled as the original fills them: widths, hex and decimals print right, and doubled spaces in a filled text are one space.
- A chat line from someone whose name the chat server has not sent yet shows just the text, as in the original, instead of a hex number.
- With "fade messages" on, chat lines fade away even while the window is under the mouse or being typed in, as in the original.
- A script's arguments in double quotes are one word (quotes kept), as in the original.
- The arrow keys recall the last 200 lines sent, and a line sent again moves to the newest place; a chat window shows its last 200 lines.
- The chat settings can disband your own private chat group (right button on its row), as in the original.
- Voice lines from ignored players no longer play.

### Movement

- Other characters' walk and run animations follow their walk/run mode instead of their measured speed.
- One's own character is no longer pulled back to a place the server's echo names; the server's own put-backs (teleports) still move it.
- Creatures near death run slower, as players do.
- A character without Shadowlands no longer gets a message box at the Jobe whompah booth; as in the original, nothing happens.
- The /sit command is gone (the original has none); use the X key to sit down and stand up.
- Flight now also ends where the original ends it: near certain Notum Wars tower sites, when the server's action or a state flag says so, and for a jetpack or a program as well.
- Creatures chasing a target now stop about 4 m from it, as in the original client, instead of 1.5 m.

### Nanos

- The NCU in use no longer counts hostile nanos or heals running on the character.
- The NCU window's list shows self-only nanos as such.
- A nano's cast effect lasts as long as the cast itself; instant nanos show no cast effect.
- Nano programs no longer draw the client's own coloured rings over their target; only the program's own effect shows, as in the original.
- The "Nano programs available." chat line after a long recharge is gone (the original has no such line).
- The nano programs window keeps programs where you put them: drag a program onto another cell, and "Don't Auto Arrange" works; each school keeps its own order.
- The NCU window's menu has the original's Auto Arrange choices (sort the icons by any column, ascending or not), kept between sessions.
- A program that changes your character's build or profession now does so (it was ignored).
- Nanos that the game marks as instant (such as the Lend Nano line) no longer show a cast bar or casting pose.
- A program that needs an attackable target now says so when it lands on something you may not attack, as the original does.

### Sound

- The music's random choices are no longer the same every time the game starts.
- Indoors, the district music follows the room you are in.

### Not yet seen in a live game

Read from the original client and tried offline or by test only: being feared (no capture has a fear), the nano
checks before a cast and the stacking refusals, the suppression-field lines and the opposite-side and battlestation
rules (the line is shown in the chat here; the original may show it on screen), natural-weapon swings, a polymorphed or
riding character's head, the Notum Wars, battlestation and vehicle windows, the city controller's purchase, and the
new pick-up, crawl and give-trade behaviour.

## 0.1.6 (2026-11-18)

### The launcher

- The launcher no longer closes a second after opening ("Invalid surface") on machines that draw with Direct3D 12,
  and the login no longer closes there at a refused password.
- The graphics device chosen in the launcher's settings is now used by the game itself, not only by the launcher
  and the login.

### Items

- A right click on an item one cannot wear or wield no longer takes off what is worn in its place. The item is
  judged first, as the original judges it, and the game's own line says what is missing ("Your Computer Literacy
  is required to be at least 76!"). A swap is also judged without what the item coming off adds.
- Requirements are judged on the values the skill window shows (with what worn items, perks, nanos and the
  attributes add), not on the trained values alone.
- An NCU unit is refused when the belt has no deck place for it, and a belt is not taken off or changed while
  units are in it, each with the game's line; both used to be tried and refused by the server.
- A worn item's penalties at a quality between its two templates are rounded as the game rounds them (off by one
  at times).
- An item whose quality lies between two templates shows its own values on its page (deck places, ranges, price),
  and at the top quality it carries the higher template's name and icon, as in the game.
- Stacks can be joined and split inside a backpack: drop a stack on a like one to join them, Ctrl+click a stack
  to split some off into an empty cell.
- An item's timer bar starts when the server allows the use, not at the click.
- Bag, bank, corpse, chest and overflow windows have the item menu: hold the right button on an item for Look at
  Item, Create Reference and Delete Item; on a corpse, in a team, also who may loot it.
- A tradeskill piece used up inside a backpack now leaves the backpack's window.

### Shops

- A shopkeeper who talks can be bought from: the Shop button of the NPC's window opens the shop, as the original's
  does. It was switched off, and Arete Landing's shopkeepers could not be bought from at all.

### Fights

- In a fight a click on one's own character selects it (to use a stim on oneself); the creature fought was always
  taken instead.
- What is locked is shown grey for as long: items in the inventory and on the hotbars while the skill their use
  locks is locked (a stim while First Aid is), special attacks, and the nano programs while one is cast or
  recharges; "Nano programs available." is said when they are free again, as the game says it of a skill.
- Back from a death the character is itself again. It used to come back with no health shown, could not be healed
  and could not log off until the next login.
- Every creature killed turns into its remains: one that died while running at the character kept its own name,
  could not be looted and lay there for good, as did one that left no corpse.
- A creature sent running by the server runs at its own speed, with its run; it used to follow at a walk, metres
  behind where the server had it, and fought from there.
- Creatures that hold no weapon (most of them) swing when they attack, and stand in their fight stance for as long
  as they fight; they used to stand still while they hit.
- Perk actions show their lock: a used perk fades until it can be used again, and "... can be used again" is said
  when it can. (The lock's message was read wrongly, so no perk ever looked locked.)
- A click on a locked special attack or perk in the Actions window says how long it still takes, as the keys and
  the hotbar do; it used to do nothing.
- Lock times follow the character's lock modifier (a stim's wait on a character with -20 % is shown as 32 s, not
  40 s).
- The Actions window's Use, Start/End Combat, Sit/Stand, Walk/Run and Suspended Animation tiles work (they do
  what their keys do); most of them only said their message was not known.
- Brawl and Dimach say "Special attack is unavailable" when only ranged weapons are held, as in the game, instead
  of being sent and refused.
- Nano programs show their real time in the NCU window (it showed a tenth), and your own programs no longer drop
  off the client's list, with their bonuses, after a tenth of their time.
- A program another player casts on you now appears in your NCU window.
- Other players' polymorphs, vehicles and auras end when the program ends instead of lingering for thirty seconds,
  and a target's NCU window shows the programs' real times.
- Your death by a nano program, a damage shield or a special attack now says what you died of (only a weapon's
  hit did).

### Windows

- The NCU, nano program and mission lists keep their column widths and the column they are sorted by between
  sessions.
- The factions window starts with the standings' titles, as the game's does, and remembers the choice between
  titles and numbers.
- The raid window's "Convert To Raid" and "Raid Locks" buttons work, and in a raid it shows all six teams. Outside
  a raid its teams are empty, as in the game.
- The pet window shows the programs running on each pet as a row of icons under its bar; click one for its page.
- "Remember Pet Names" in the pet window's menu: a pet you renamed gets that name again when you call one of its
  kind. `/pet rename <name>` now sends the name.
- Right-click menus are the original's: sub-menus open beside their entry, separators, the tick at the right, and
  typing a few letters jumps to an entry. The chat window's menu is one menu with its sub-menus, and a borderless
  chat window's transparency is set with two sliders.
- Escape closes the raid window, the team search and the city terminal (by their options), and your own backpacks'
  windows.

### Missions

- A terminal with no missions says so in the chat and empties the window's old offers.
- A level-up says the experience that made it, and a new title when one comes with it.
- For characters of level 200 to 220 a mission's page shows its pay in shadow knowledge when read in the
  Shadowlands.
- The mission terminal's window remembers where its sliders were left.
- Zoning while in a team no longer empties the perk list.

### Flight

- In the air the sit key, the walk/run key and the sit, pace, sneak and crawl tiles say "Action is not available."
  and do nothing, as in the game; `/camp` asks you to sit. A seated character gets up when it starts to fly.
- A flight item is refused where flying is not allowed (Arete Landing among them), with the game's line, instead
  of being tried and refused by the server.
- Steps are quiet while sneaking.

### World

- Doors make their sounds as they open and close.
- Other players are drawn at their own height (most are a tenth shorter or taller than the middle size) instead
  of all the same.
- Other players on a Yalmaha or another craft are drawn in it, flying, and polymorphed players as the creature
  they are, when they come into view or mount in view. Players who are sitting when they come into view are drawn
  sitting.

### Chat

- `/raid` now carries out the raid's commands (`create`, `locks`, `list`, `listlocal`, `move`, `lootaccess`) as in
  the game, and the server's lock list is shown in the chat. It no longer opens the raid window: that is
  `/open raid` or the control center's menu.

### Settings

- Keys rebound in the original client are taken over for that character (the keyboard keys of the actions this
  client has), unless the key was changed here too.
- Five default keys now match the game's: walk/run on Backspace (was Numpad /), sneak attack J, fast attack N,
  aimed shot O, bow special attack Period. The map overlay, which had O, is on Ctrl+P. Keys you had changed
  yourself are kept.
- Options that did nothing now act: fog "off" thins the fog to a tenth, "all effects" switches every effect option
  at once, both cloud options off hides the single clouds, and "wait for vertical sync" sets the client's VSync.
- The helmet, shoulder pad and social tab boxes each keep their own setting (ticking one overwrote the others).

### Not yet seen in a live game

Read from the original client and tried offline or by test only: joining and splitting stacks inside a backpack, the
item menu's Delete outside a backpack and its looter entries, everything of a raid (`/raid`, the raid window's
buttons and teams), a pet's kept name and `/pet rename`, other players getting on and off a craft in view, the
death's reason from a nano program or a shield, the keys taken over from the original client, the new right-click
menus (seen in pictures only) and the options that now act. The Direct3D 12 fix was tried here by choosing that
device: the launcher, the login and one short visit to the game.
## 0.1.5 (2026-11-17)

### The launcher

- No flash at its start: the window opens with its picture in it, and the news page comes in whole instead of as a
  white sheet first.
- When the launcher or the login cannot go on, a box says why and where the details are; they used to end with
  nothing on the screen. The launcher's own window keeps a crash log now, as the login and the game did.
- The launcher opens on a machine whose graphics device cannot be started as usual (at the last with the system's
  software device), so that another device can be chosen in its settings; a picture or the news page that fails
  leaves the plain window with its settings and PLAY.

## 0.1.4 (2026-11-17)

### Character creation

- Creating a character is now the original's four scenes aboard the station: breed, appearance, profession and name,
  with the camera's flights between them, the opening sequence (Escape skips it), the sounds and the question before
  leaving. It takes the place of the client's own plain window. Characters have been made with it on a live account.
- The Keeper and the Shade can be chosen only on an account that has Shadowlands.
- The character list says how many of the account's character slots are free, and offers Create only while one is.
- Not there yet: the figures and the ships do not move, the chosen figure has no glow, the expansions' extra heads
  are missing, and the choices are not kept between visits.

### A new character's first hours

- On a new character's first arrival the inventory was one place off from the server's until the next login, so
  wearing things and using the starter nano crystals failed. Fixed, and confirmed in play.
- A thing handed to an NPC leaves the inventory when the trade is over; it used to stay shown.
- Stacks (stims, rechargers) show their count from the start, and using one takes one off.
- A helmet that covers the head is drawn with the face of the head it stands in for: no more blue patch under the
  mining helmets.
- A shop that only the server puts into a playfield is drawn and named (Arete Landing's Junk Shop).
- A creature the server announces on the ground is on the ground: Arete Landing's wounded lie, seated NPCs sit.

### Other players

- A player's info page has the Inspect Equipment link, and `/inspect <character number>` asks the same; the window
  is named after the inspected character.

### Windows

- Mail: cash on delivery, the questions before sending and before taking, and Return To Sender.
- Shops that only sell open in the buying view.
- An item with several actions has every action's requirements read and shown: some 1,900 items (a jetpack among
  them) said nothing of what they ask.
- The organization's question boxes: leaving, disbanding, an invitation, a promotion and the two tax questions.
- The options window has the original's tabs and the skin's fixed keys.
- Equipment is locked by the original's own wear check.
- Every Whom-Pah booth is named from its sign and has no tag over it; Jobe's tells an account without Shadowlands
  that it needs it.
- A planet map can be made of one's own pictures (the `planetmapbuild` example, `tools/planetmap_fit.py`).

### Chat

- The commands `/cc`, `/cc info`, `/rp`, `/name`, `/gfx` and `/bank close`; `/name`, `/invite`, `/kick`, `/leave`
  and `/ignore` send what follows them the way the original cuts a command's line.
- The server's own lines are shown, and a friend taken off the list leaves it.

### Movement

- Turning sends the original's turn messages, by the keys and by the mouse. Every turn used to end, for the server,
  in a left turn begun, and the server walked its own copy of the character in a circle: one could be zoned by a
  Whom-Pah some metres away.
- One's own death is told to the server three seconds after the fall, as the original does; a dead character used
  to lie a minute.
- No take-off before the server has taken the flying item.
- The name over one's own character holds still at a run.

### Not yet seen in a live game

Read from the original client and tried offline or by test only: the inventory after a hand-in, stack counts, the
helmets' faces, the Junk Shop, the wounded lying, the slots line after a real login, the mail's cash on delivery and
the organization's question boxes.
