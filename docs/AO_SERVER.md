# ao-server: the test server, built from the captures

`crates/ao-server` is the test server the original client can log in to. It speaks the same messages the captures show, and
the client's own parsers (`mao-net`'s schema) read what it sends. This page says what works, how it was checked, and what is
still missing.

## How the messages are made

* **The schema encodes as well as parses.** `mao-net`'s `Schema::encode_n3` writes an N3 message from its named fields, the
  inverse of `parse_n3` (same layouts, same flags). Round trip over the four captures: 120,749 of 120,844 messages that parse
  whole come back byte for byte. The 95 that differ are `QuestFullUpdate`: its strings lose their trailing NUL bytes on the way in
  (the reader trims them), so re-encoded text is 1 to 3 bytes shorter. A message the server builds itself is consistent with
  its own length prefixes, so the client reads it correctly.
* **Templates.** `crates/ao-server/data/templates.txt` holds one captured message of each kind the server answers with
  (`mao-net/examples/capture_templates.rs` makes it). A server message is the template with the fields it changes set, encoded
  again. Anything the schema does not describe at the end of a message is kept as captured. The templates contain no text and no
  account names.
* **Monsters (`SimpleCharFullUpdate`, `271b3a6b`).** The schema still does not parse this message whole (it fails at its
  active-nano list). The client's own decoder (`n3::entity_info`) does read the parts it needs: the playfield, the place, the
  name, and for a monster its template (the appearance) and its health. So a monster is a captured monster's message with
  those parts changed (`n3::set_monster`), the rest kept as captured. The stored template has the server's own name in it
  (`Monster`), so no captured name is kept. The agency receptionist (`knubot.rs`) is placed by the same route.
* **The mission offers' reply (`QuestAlternative` from the server)** is built in the layout the 2026-11-08 capture shows (the
  ICC Mission Central terminal's five offers, `captures/events-20261108-1811`): the request's first byte, sliders, scope and
  terminal echoed, then each offer with its reward descriptor (version 6, the "MSRE" word and the character's level in the
  44-byte chunk), one reward item at the character's level, the icon 11330, the 48-hour deadline (minutes, and as a unix
  time in the 120-byte chunk), the destination playfield and place. The mission log (`QuestFullUpdate`) follows the captured
  entry too: the giver, the reward, the deadline 2880 twice, one action naming the destination (the client's "Upload to map"
  uses it), the character as the only player, icon 11329. The texts are still the server's own.
* **Schema fix:** `KnuBotAppendTextMessage` ends with its text. The definition's trailing word is in none of the 230 captured
  messages, and with it the client could not read an NPC's lines (`gen_protocol_schema.py` drops it when regenerating).

## What works (tested)

| Feature | What the server does | Tested by |
|---|---|---|
| Login | Account, challenge, character list, ticket to the game server | `tests/network.rs` (over sockets) |
| Character creation and deletion | Stored in the database, listed at the next login | `tests/network.rs`; `tests/protocol.rs` |
| World entry | Place, game time, position, the **stats** (level, health, max health, XP, next and last level's XP, credits) and the **inventory** | unit test in `game.rs`; `tests/network.rs`; the real client's headless `net action` reads them (`4e536976`, `2b333d6e` appear in its parsed list) |
| Progress saved | Level, XP, health, credits, place and the backpack are kept in SQLite and read back at the next login | `tests/network.rs` (the level after a re-login) |
| Spawn items | `/gm item <template> [count] [quality]` puts an item in the backpack; the inventory message lists it | `game.rs` tests; `gm.rs` tests |
| Experience and levels | `/gm xp <amount>` gives XP; levels follow the curve; a level-up fills health and says so in chat | `game.rs`; `progress.rs`; `tests/network.rs` |
| Credits | `/gm credits <amount>` (stat 61) | `game.rs` |
| Shops | Using the Old Athens vending machine sends its stock (`ShopUpdate`, the captured stock) and opens the trade; buying (trade action 5 naming the machine and the slot) takes credits and puts the item in the backpack | `game.rs` tests (open, buy, refuse without credits) |
| Missions | A terminal's request (`QuestAlternative`) gets the server's three offers in the captured layout; taking one (`CreateQuest`) puts it in the log (`QuestFullUpdate`, the captured entry's layout, naming the terminal as the giver) and says so in chat; `/gm complete <n>` finishes it as the Arbiter capture shows: `CharacterAction` 0x3b (the client's "mission accomplished"), the log's `Quest` Delete, the XP and credits | `game.rs` tests (offers, accept, refuse a second, complete); `missions.rs` tests (the layouts parse whole, the chunks' sizes and marks). The client's own reading of the offers and the log is not verified live |
| NPC dialogue (`Knubot`) | The agency receptionist stands by the spawn in Old Athens. Opening her window is echoed; she greets by the character's name and offers a job; "I'm applying for a job." then "Straight forward, please." uploads the daily mission (the log is sent, the window closes with word 5); coming back with the mission completes it (the 0x3b action, the Delete, the reward) and leaves "Goodbye" | `game.rs` test (the whole talk, there and back); `knubot.rs` tests (the messages' captured shapes, the client's requests read) |
| Mail | `/gm mail <template> [quality] [credits]` puts a shop delivery in the inbox; stat 0x289 is 1 while unread mail waits. At a terminal the client's `Mail` requests are answered as captured: the inbox list (action 0), the opened mail (2, flags gain bit 0), the attachment taken (3: `TemplateAction` Added, `ContainerAddItem`, the credits, flags gain bit 1), the deletion (5). Mail is kept in SQLite | `game.rs` test (list, open, take, delete, the stat); `mail.rs` tests (the client's own `mao_net::mail` parsers read the server's messages; the list's captured layout) |
| Using items | A `GenericCmd` Use is echoed (header 0, first word 1, as the original answers); a backpack item the server knows is used: the lottery ticket (template 303463) gives "Congratulations!" and a Victory Point token (294604) in a new slot; the token gives "New Victory Points gained." and 100 Victory Points (stat 0x29d). The used item leaves with `TemplateAction` Used and the server's `CharacterAction` 0x70 | `game.rs` test (ticket, then token); `useitem.rs` tests (every message byte-equal to the capture's; the packed `FormatFeedback` reads back through the client's decoder) |
| Combat (the training dummies and the rollerrat) | Five monsters in Old Athens come with the world entry. Attacking one starts a fight: a swing every two seconds (`AttackInfo`, `HealthDamage`), the monster's health counts down in the client, and at zero it is taken away (`Despawn`), the fight stops (`StopFight`), the character gets XP and an item from the stock (loot). The rollerrat fights back each swing (its `AttackInfo` about itself, the character's `HealthDamage`, the health stat); a character it kills gets `StopFight`, "You were killed by ...", full health and a move to the spawn within the playfield (`N3Teleport` of the same-playfield type) | `game.rs` tests (fight to the kill, XP and loot, an attack on nothing, the rat's swings and the reclaim); the real client's headless `net attack` run: swings every 2 s, 120 to 0, "You kill ...", "You receive item ..." |
| Arrival | The playfield message is the captured shape (four zero bytes, a 4, the position at 5), the playfield is the identity's instance with type `0x9c50`. A new character starts in Old Athens at the captured spawn | `game.rs` test; the real client reads "playfield 540 at [490.45, 8.01, 447.38]" |
| Zoning (the client's own zone line) | The server answers a teleport to another playfield with the world entry for it (changed this session) | not tested yet |

The chat lines the server sends (for example "You are now level 2.") use the game's `ChatText` message.

## Approximations (to replace when the originals are known)

* **Experience curve.** `progress::xp_for_level` is the server's own (`500 · (L−1) · L`). The original's table is not in the
  captures.
* **Health per level.** `100 + 10 · level`.
* **Monster numbers.** The training dummies have 120 health, 400 XP and the stock item as loot.
* **Shop price.** Every item costs 50 credits. The original prices by the item's value and the shop's rates (stats 426 and 427).
* **Stock.** The captured machine's stock, not a table from the game files.
* **Game-master commands** (`/gm`) are the test server's own. The original's client has no such command, so the headless
  client cannot send them yet (it sends only plain speech); the tests send them as chat frames.

## Not done

* **Combat is partial.** Only the rollerrat attacks back (5 a swing, the server's number); the monsters do not respawn and are
  kept per session: another character does not see a monster that one character killed. A death is the server's stand-in
  (the character is reclaimed at the spawn at once with full health): the original's death and reclaim messages are not in
  any capture. The damage is 10 plus twice the level, not the original's formula. The swing identities (`AttackInfo` about
  the attacker, `HealthDamage` about the one hurt) follow the captures, but the client's reading of the fight as a whole is
  checked only with its headless attack tool.
* **Mission objectives.** The three offers are a stop-gap: the server does not yet run a terminal mission (no kill or delivery
  goal); it ends with `/gm complete`. The daily mission ends by returning to the receptionist. Mission keys (the capture's
  `SimpleItemFullUpdate` of type `0xc76d` after an accept) are not given.
* **Guards.** The capture shows ICC guards acting against a player who attacked them (`2049527c`, "Action", and the PvP flag);
  the server's monsters still do not fight back, so no guard does either.
* **The Victory Points a token gives** are the server's number (the capture shows the total after, not the gain).
* **The event characters** (the Arbiter speaking in chat, the seasonal monsters around ICC HQ) are ordinary characters and
  monsters on the wire; the server has no scripted event yet.
* **Missions from the original.** The captured mission log contains the game's own mission texts, so none of it is kept in the
  repository: the server's titles and descriptions are its own.
* **Zones and doors.** Entering a door sends the client's teleport; the server's side of a zone line to another playfield needs
  the destinations (the captures' `N3Teleport` messages have them) and the second game server's ticket.
* **The login's password.** The client's encrypted response needs the server's private key, which is not known, so the login
  accepts any password for the account. It is a test server.
* **The inventory at world entry** is the captured message's layout with the items the server has (the original's 4 KB
  world-entry inventory message `29304349` is not built yet).

## How to check it

```bash
cargo test -p ao-server                       # unit, protocol and network tests
MAO_CAPTURES=captures cargo test -p mao-net encode -- --nocapture   # the round trip over the captures
```

To run the server and log in with the real client's headless tool (the account name and password come from `.env`):

```bash
cargo run -p ao-server                        # login on 127.0.0.1:7105, game on 127.0.0.1:7500
MAO_SERVER=127.0.0.1:7105 mao-client net <game folder> chars
MAO_SERVER=127.0.0.1:7105 mao-client net <game folder> create <name> 1 1 0 100 0
MAO_SERVER=127.0.0.1:7105 mao-client net <game folder> action <name> 0 8
```

The server's database is `ao-server-data.db` in the working folder, or `MAO_DB`.

## Testing with the original client

1. Start the server with an admin password (the panel does not start without one):

   ```bash
   MAO_ADMIN_PASSWORD=choose-a-password cargo run -p ao-server
   ```

2. Open <http://127.0.0.1:8080>, sign in, and make an account (name and password). The login accepts only accounts made here.
3. Start the original client as you did: `AnarchyOnline.exe IA16777343 IP7105 DU127.0.0.1:7500`, log in with that account, and
   try to create a character.
4. Everything the server sends and receives is written to `ao-server-trace.log` in the folder the server runs from (set
   `MAO_TRACE` to choose another file). Lines marked `UNANSWERED` or `UNHANDLED` are messages the server did not act on. The
   panel also shows the end of the trace, and <http://127.0.0.1:8080/trace> gives the whole file.

The passwords are stored as typed and are not checked yet (the login's encrypted answer needs the server's private key, which
is not known). The admin panel listens on this machine only.

## Slots and character administration (the admin panel)

* **Character slots** are per account, set in the panel (default 6). The character list carries them after the characters, with
  the account's expansion flags and a zero: the captures show this trailer (`frames::character_list_slots`), and the original
  client shows "x/y slots available" from it. A creation past the slots is refused with code 8, `CantCreateMoreChars` in the
  client's own table. An unknown account is refused with code 1, `PlayerNotFound`.
* **Bug fixed:** the character list had an extra integer after every entry. The original's reader takes it as the next entry's
  version, so a second character could not be read. One character alone hid it.
* **Characters** can be given XP (levels follow), credits and items (by template number) and can be deleted, from the panel.
  The change takes effect at the character's next login.
* **Unverified:** the meaning of the trailer's first number (the slots are assumed; the captured lists show 8, 10, 11 and 18
  with different accounts) and the expansion flags' values (the captures show 1 and 171).

## Entering the world (the "Welcome to Rubi-Ka" screen)

The original client stopped on its loading screen because the game server sent nothing after `EnterGameServer` (system `0x1b`).
The captures show what the original sends then:

* a **control packet** (sequence `0xdfdf`, kind `0x7f00`, one, size 16, source `0x01000000`): from here on the server's frames
  are one zlib stream. Inside the stream each frame is **not** padded to four bytes. The client's reader already reads this
  (`mao-net`'s `Reader`); the server now writes it (`server::Compressed`), and the test that reads it back passes.
* the **chat server** (system `0x43`), so the client can open its chat connection, then the **playfield** (`0x5f4b1a39`), as
  the first two messages of the stream. The world entry proper (time, position, stats, items, monsters) follows the client's
  `READY`, as before.
* The **ticket** now ends with the event server type (0) and the player id, the last eight bytes of the captured ticket. The
  ticket was 22 bytes; the captured one is 30.

Verified: the headless client's `action` run gets the whole entry over the compressed stream. **Not verified:** whether the
original client then leaves "Welcome to Rubi-Ka". Its chat connection (after `0x43`) goes to the login listener, which does
not answer chat packets yet; the trace shows what it asks.

## Logs (send these when the client misbehaves)

Run the server from the `modern-anarchy-online` folder. It writes:

* `ao-server-trace.log`: one line per event, tagged with the connection (`c0003 game`). Each connection starts with `begin`,
  lists every frame in and out with its name and body, and ends with `end after N s` and why (the client's end of stream, a
  read or write error, or a frame that could not be decoded).
* `ao-server-logs/`: for each connection, the raw bytes as they crossed the socket: `cNNNN-login-in.bin` and
  `cNNNN-login-out.bin` (and the game equivalents). The game's `-out.bin` holds the control packet and the zlib stream exactly
  as sent.

Send the trace and the `ao-server-logs/` folder after a failed run.

