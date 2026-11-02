# ao-server: the test server, built from the captures

`crates/ao-server` is the test server the original client can log in to. It speaks the same messages the captures show, and
the client's own parsers (`mao-net`'s schema) read what it sends. This page says what works, how it was checked, and what is
still missing.

## How the messages are made

* **The schema encodes as well as parses.** `mao-net`'s `Schema::encode_n3` writes an N3 message from its named fields, the
  inverse of `parse_n3` (same layouts, same flags). Round trip over the captures: 106,946 of 107,031 messages that parse whole
  come back byte for byte. The 85 that differ are `QuestFullUpdate`: its strings lose their trailing NUL bytes on the way in
  (the reader trims them), so re-encoded text is 1 to 3 bytes shorter. A message the server builds itself is consistent with
  its own length prefixes, so the client reads it correctly.
* **Templates.** `crates/ao-server/data/templates.txt` holds one captured message of each kind the server answers with
  (`mao-net/examples/capture_templates.rs` makes it). A server message is the template with the fields it changes set, encoded
  again. Anything the schema does not describe at the end of a message is kept as captured. The templates contain no text and no
  account names.
* **Blocked: the NPC spawn message (`SimpleCharFullUpdate`, `271b3a6b`).** The schema's layout does not parse the captured
  messages. The name's length prefix is one byte (found from the captures, now in `SIZE_OVERRIDES`), but the message then fails
  at its active-nano list. Until that layout is worked out, the server cannot place NPCs or monsters on the client's screen, so
  **combat against NPCs and NPC mission givers is not possible yet.**
* **Blocked: the mission offers' reply (`QuestAlternative` from the server).** No capture of it is in the repository (the
  client's mission window was built from an older capture that is no longer here). The offers have to be built from the
  schema's `MissionInfo` layout, with its unknown chunks left zero.

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
| Zoning (the client's own zone line) | The server answers a teleport to another playfield with the world entry for it (changed this session) | not tested yet |

The chat lines the server sends (for example "You are now level 2.") use the game's `ChatText` message.

## Approximations (to replace when the originals are known)

* **Experience curve.** `progress::xp_for_level` is the server's own (`500 · (L−1) · L`). The original's table is not in the
  captures.
* **Health per level.** `100 + 10 · level`.
* **Shop price.** Every item costs 50 credits. The original prices by the item's value and the shop's rates (stats 426 and 427).
* **Stock.** The captured machine's stock, not a table from the game files.
* **Game-master commands** (`/gm`) are the test server's own. The original's client has no such command, so the headless
  client cannot send them yet (it sends only plain speech); the tests send them as chat frames.

## Not done

* **Combat.** The attack and damage messages (`Attack`, `AttackInfo`, `HealthDamage`, `StopFight`) have captured templates, but
  there is no visible target until NPCs can be spawned (see the blocked item above). The death and respawn of the character,
  and loot from kills, depend on that too.
* **Missions.** Offers (`QuestAlternative`), accepting (`CreateQuest`), the mission log (`QuestFullUpdate`, which can be built
  from a template's quest entry) and completion (a delete, `Quest` action 1). The log can be built, but the offers are not
  verified against the client.
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
