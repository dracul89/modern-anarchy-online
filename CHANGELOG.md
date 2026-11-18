# Changes

What each release changed, as a player meets it. The commits have the detail, and `docs/CLIENT_UI.md` says how each
thing was read from the original client. The release workflow puts a version's section at the top of that release's
notes (`docs/RELEASING.md`). Releases before 0.1.4 have their notes on the Releases page only.

## Unreleased

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

### Windows

- The NCU, nano program and mission lists keep their column widths and the column they are sorted by between
  sessions.
- The factions window starts with the standings' titles, as the game's does, and remembers the choice between
  titles and numbers.

### Missions

- A terminal with no missions says so in the chat and empties the window's old offers.
- A level-up says the experience that made it, and a new title when one comes with it.
- For characters of level 200 to 220 a mission's page shows its pay in shadow knowledge when read in the
  Shadowlands.
- The mission terminal's window remembers where its sliders were left.

### Flight

- In the air the sit key, the walk/run key and the sit, pace, sneak and crawl tiles say "Action is not available."
  and do nothing, as in the game; `/camp` asks you to sit. A seated character gets up when it starts to fly.
- A flight item is refused where flying is not allowed (Arete Landing among them), with the game's line, instead
  of being tried and refused by the server.
- Steps are quiet while sneaking.

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
