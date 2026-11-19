# Changes

What each release changed, as a player meets it. The commits have the detail, and `docs/CLIENT_UI.md` says how each
thing was read from the original client. The release workflow puts a version's section at the top of that release's
notes (`docs/RELEASING.md`). Releases before 0.1.4 have their notes on the Releases page only.

## Unreleased

### World

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

### Items

- A worn item can be locked to its place: hold the right button on it and choose Lock Item. It then cannot be taken
  off, swapped or have another item put in its place by mistake, and shows a small lock. Unlock Item, or Unlock All
  Items in the wear window's menu, frees it. "Delete" is no longer offered for an item you are wearing.
- The bank's items can be placed by hand: its menu has "Don't Auto Arrange", and it starts that way. Each item stays
  in the cell you put it in.
- `/pet` takes pet names before the command (`/pet Rex guard`, `/pet all attack`) and the commands with an argument
  (`cycle a|w`, `social <name>`, `chat`, `script`), as in the original, and says when you have no pet.
- A mission's item (Arete's "Plant a Bug" recording device and the like) now comes into your inventory.

### The launcher

- The character you pick in the list is alive: it breathes, and now and then waves, bows, scratches its head, as in
  the original.

### Fights

- While your character lies dead the game takes no input, as the original: no keys, no clicks, no typing, until the
  character is back.
- Brought back by another player's resurrection (from level 15), your character is in resurrection shock: skills
  and abilities are lowered, by half at first, and recover over about six minutes.

### Combat

- The special attacks shown are confirmed to be built the way the original builds them (the server's three plus
  what each weapon in a hand gives); two small corner cases of it were made the original's.
- A special attack out of reach says so at once instead of going to the server: Brawl reaches about 5 m and Dimach about 12 m
  between the two characters, as the original's client checks.
- The attack key refuses while swimming or flying with the original's lines, and a refusal is followed by "Unable to attack target" as in the original.
- A hit followed late by the target's health no longer shows a second "unknown damage" line for the same damage.
- Attack swings vary at random as in the original instead of in a fixed order.

### Launcher

- Character creation: the far suns, the stars and the intro's shuttle move, and the character being made stands
  breathing and fidgeting instead of frozen. A short or tall character is shown at the original's size, and the
  heads offered are the original's list (more of them with Shadowlands, a few odd ones gone).
- The dimension list marks a dimension newer or older than your game, and PLAY becomes PATCH (it tells you to patch with Funcom's launcher).
- The game's licence is shown once, until you accept it.

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

### Graphics

- A few playfields without water of their own (the Uturn and Foundry playfields among them) no longer show a flat sheet of water over their low ground.
- Far campfires, chimneys and lamps keep their fire, smoke and halo out to the view distance instead of only within 150 m.
- Effect meshes and trails attached to a body part (an arm, a leg, the spine) now show there.
- Some placed objects stored with a slant (ramps and tilted pieces of the second statel layout) now lean as in the original instead of standing level.
- Effects that the game shows only on certain playfields now appear there.
- Interiors (shops, dungeons) are seen against black, as in the original, not a very dark blue.

### Interface

- Shift+Enter also opens the chat input and the numeric pad's / also starts a chat command, as in the original (new settings files; an existing mao.ini keeps its keys).
- Bold and italic interface texts (the BOLD, HUGE, CC17 and ITALIC fonts of the skins) are drawn bold and italic, as in the original.

### Chat

- /ignore follows the original: /ignore alone ignores your target, /ignore list shows who is ignored, and the messages are the game's own.
- /tell with no name does nothing, as in the original (it no longer opens a window for the target).
- /inspect alone no longer describes the target; it takes a character's number, as in the original.

### Movement

- Other characters' walk and run animations follow their walk/run mode instead of their measured speed.
- One's own character is no longer pulled back to a place the server's echo names; the server's own put-backs (teleports) still move it.

### Nanos

- The NCU in use no longer counts hostile nanos or heals running on the character.
- The NCU window's list shows self-only nanos as such.
- A nano's cast effect lasts as long as the cast itself; instant nanos show no cast effect.

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
