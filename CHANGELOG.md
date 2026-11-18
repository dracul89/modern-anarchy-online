# Changes

What each release changed, as a player meets it. The commits have the detail, and `docs/CLIENT_UI.md` says how each
thing was read from the original client. The release workflow puts a version's section at the top of that release's
notes (`docs/RELEASING.md`). Releases before 0.1.4 have their notes on the Releases page only.

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
