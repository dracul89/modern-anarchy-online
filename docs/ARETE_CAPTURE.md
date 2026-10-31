# The Arete Landing capture (2026-10-30)

The ORIGINAL client (the Steam install), recorded with `tools/capture_session.ps1 -Name arete`: a new character
("Wheresmyncu", character `0x7b1a0021`) made and played through Arete Landing for 2 h 30 min. The files are in
`captures/arete-20261030-1655/` (kept out of git: the log holds the account name): `net.log` (the proxy's log, 6.7 MB,
23 connections), `dump.txt` (`net msg-dump net.log all`: every message, with its time), `shots/` (4 636 screenshots, file
names in Unix ms on the log's clock), `client/` (what the client wrote: prefs, `stderr.txt`).

## Where it went (the playfield messages, `5f4b1a39`)

| min | connection | playfield | instance id | arrival |
|---|---|---|---|---|
| 0.0 | c3 | 6553 (Arete Landing) | `0x20da15` | (3609, 52, 787) |
| 9.8 | c5 | 7101 (a room) | `0x1bbd` | (64, 1, 94) |
| 10.4 | c6 | 6553 | `0x20da15` | (3609, 52, 786) |
| 68.0 | c6 | 6553 (in-connection move) | `0x20da15` | (3464, 9, 814) |
| 78.3 | c7 | 8009 | `0x1f49` | (40, 0, 31) |
| 80.3 | c8 | 6553 | `0x20da2a` | (3888, 6, 249) |
| 83.8 | c10 | ? (`0x20da1b`, a 129-byte message) | | (2, 21, 205) |
| 90.3 | c12 | 6553 | `0x20e00c` | (3888, 6, 249) |
| 138.8 | c18 | ? (`0x20e000`, 153 bytes) | | (238, 21, 145) |
| 150.2 | c23 | 8009 | `0x1f49` | (40, 0, 50) |

Every zoning but one is a **new connection**: the game server sends system `0x43` (a length and the next server's
address as text, `37.18.192.41`, and a port, `0x1bc1`) and `0x3c` (`[ip u32][port u16]`: `37.18.193.19:7511..7518`); the
client connects there and sends `0x1b` with the **same game key** it had (`18cc318f0800d479`) and is in, **no new login**.
This client's `hop` logs in again at `cm.d1.funcom.com` instead (slower; the old note that the `0x3c` address "closes
the connection" came from trying a login there, not the `0x1b` greeting).

## Character creation (login connection c2)

After the character list, system `0x0f` (124 bytes: the name "Wheresmyncu" in it, the breed/profession/look fields),
answered by `0x11` (`7b1a0021 f21d5200`: the new character's id), then the usual `0x16` / `0x17` hand-off. Compare with
`mao-net`'s character-creation messages (see the memory note "MAO character creation").

## Server messages this client does not handle

| kind | count | name (schema) | what it looks like |
|---|---|---|---|
| `4d450114` | 3858 | SpellList (?) | `7e2`, a nano identity (`cf3b`, `0x48329`), counts, the caster and target: a nano effect running on a character, repeated as it ticks |
| `35505644` | 235 | TemplateAction | an item's low/high id, quality, then `0x57`, `0x6e`: something done with an item template (picked up, made) |
| `60201d0e` | 202 | SetWantedDirection (aosharp) | on characters: a byte and floats (`bf80000…`): a facing/turn? |
| `435f7023` | 156 | PerkUpdate (aosharp) | `01`, `0x988` twice, a short id: on one's own character |
| `365a5071` | 108 | DoorFullUpdate | an instance's doors with their places and rotations (identity `c748:10c32dc1..`): the doors of an instance announced |
| `2a293d0f` | 59 | Fov (aosharp): a creature's sight cone | floats (`3ec49809 40490fdb`: pi): an arc or a cone on a character |
| `742e2314` | 53 | StopMovingCmd (aosharp) | `01` and an identity's type: on characters |
| `3a223b50` | 36 | ItemReplaced (aosharp): place, item before, item after; Arete's worn starter armour goes up a quality per level | pairs of template ids (`0x4a09b`/`0x4a09c`), count 2: a reward or mission item list |
| `052e2f0c` | 23 | AddTemplate | an item (low, high, quality, count) added: loot or a mission reward into the inventory |
| `3e205660` | 18 | Skill | the skills' values after a raise (both ways: the client sends its raises, see below) |
| `28784248` / `46312d2e` | 8 / 2 | TeamMemberInfo / TeamMember | a team |
| `3129233b`, `485e7202` | 4 / 2 | AreaFormula / InventoryUpdated (aosharp) | |

## Client messages this client never sends

| kind | count | name | |
|---|---|---|---|
| `3e205660` | 18 | Skill | raising skills with IP (pairs of skill number, points) |
| `77230927` | 3 | GiveQuestToMembers (aosharp) | `00 5628c914` |
| `3115534d` | 2 | ? | `00` |

Everything else the original sent (moves, look-at, use, attack, special attacks, NPC chat and trade, trades, container
moves, set-stat) this client already sends.

## To do, in order

1. ~~Zoning by `0x1b` with the kept key~~: the live client already did (`hop_direct`, a new login only as the fallback); the headless `walk` now does too.
2. Instance doors from `DoorFullUpdate` (places and rotations; open/close by `DoorStatusUpdate`).
3. ~~Item messages~~: done (`add_to_backpack`; test `items_come_into_the_backpack` on the captured bytes).
4. ~~Skill raising~~: done (Skills window +/-, Accept; `n3::raise_skills`, test on the captured bytes). The values sent are the new trained values (Strength 15 -> 18 sent as 18); the reply adds stat 53, IP left. The IP cost tables (`N3Msg_GetSkillCost`, FUN_10062521: breed table `0xd6e2` x value for abilities, profession table `0xd6df` x value / a constant for skills) are not in the resource database, so the window does not price a raise.
5. Nano effects running (`4d450114`) for the buff display.
6. The unknown ones, against the screenshots at their times.
7. ~~Character creation~~: our request matches the original's byte for byte (test `the_request_matches_the_original`).
8. ~~Teams~~: done (`team.rs`): invitation `CharacterAction` 0x1a, answer 0x1c with 1/0 (declining is 0x1c with 0, not aosharp's 0x23), `TeamMember` (team, level, profession, name), `TeamMemberInfo` (health, max health, max nano, nano), leave 0x18, left 0x20.
9. ~~Generated mission dungeons~~ (proxy type `0xc79f`/`0xc7a1`): done. The playfield message's object is
   `ACGBuildingGeneratorData_t` (Gamecode FUN_100c773e: identity, a word, version 3, width, height and floor step as shorts,
   the template playfield, the ambient colour, then `BuildingRoomInfo_t` records: room, floor, x, y, turn, FUN_100cad28); the
   playfield factory (FUN_100c9a60) loads the template's definition and tile-map and FUN_100ca6f8 places a copy of each
   record's template room: centre = block corner `(x*10, h*10 - y*10 - scale*size)` + the room's half `(oddW/2, 0, oddH/2)` x
   scale + `(1, 0, 1)` turned, plus the turn's shift; height + floor step per floor. `Message::generated`,
   `rooms::generate` (+ `move_between` for the zones' collision surfaces), `generated_rooms` standing in for `load_rooms`.
   Checked: the ship's six doors (`DoorFullUpdate`) each fall on a generated room's edge (test
   `the_ship_rooms_meet_at_the_servers_doors`); `MAO_GENERATED=<message hex> mao-client <game> map 382 --image out.png` draws it.

Done since: SetWantedDirection turns the character (`rotations`, forward `(sin h, cos h)`), StopMovingCmd ends its walked
path, ItemReplaced updates the worn item (test in `items_come_into_the_backpack`).
