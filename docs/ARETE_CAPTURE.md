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

# The Shadowlands capture (2026-10-31)

The original client again (`tools/capture_session.ps1`), 18 minutes with a level-220 character (`0x4bc80`): files in
`captures/arete-20261031-0842/` (`dump.txt` from `mao-client net msg-dump net.log all 1000000`).

Route (playfield messages): Andromeda's ICC HQ (655) → an instance and back → another instance → Old Athens (540) → the
Wailing Wastes (551) → the Garden of Lord Galahad (4692) → Inferno (4605) → Caina (4328) → Dante's Sanctuary (4696).
Every zoning is the server's `43197d22` (the place, the rotation and the playfield to go to) then the usual redirect.

- **Flying** (Andromeda, 1.4–5.7 min, a Phasefront Banshee): no fly-mode (0x1d) or mouse-steer (0x2b) message; the full
  reports carry the pitched rotation and the height follows it (21 to 185). The server's spell list of the program
  (#305, ended by #526) carries spell 0xcf92, which is what switches the character to fly mode (FUN_100a8107;
  `spelllist::Outcome::Fly`, `mao_formats::item::grants_flight`): the hoverbikes' programs have it, the hoverboards' do
  not (the 0xcf4c with 73 = 2 that every vehicle program has only takes a state flag away). Ported (commit 2f817f7; the
  signal corrected 2026-11-13).
- **Perk actions** (the Inferno fights): `CharacterAction` 0xb4 lists each perk action at entry (item template, number,
  four-letter code); the client's 0xb3 uses one; the server's 0x50 queues it (the attack time), 0xcf locks a group (the
  template's lock-skill spell 0xcfc3: Quick Bash group 201 for 25 s) and 0xce frees it. Ported (commit 448d6b5).
- **Using an item on something** (into the Garden and on to Caina): a key in an open bag, `(0x6b, 0x00700006)`, used on a
  statue `(0xc73d, ...)` with `52526858` action 5 (`N3Msg_UseItemOnItem`); "The Key to the Garden of Lord Galahad" fires
  its spells on event 14. Ported: an item dragged out of the inventory or a bag and let go over the world is used on what is
  under the pointer.
- The other zonings were plain uses (`52526858` action 3 on a `0xc73d` portal or door).

# The trade, perks, pets and tradeskill capture (2026-10-31 15:05)

`captures/arete-20261031-1505/` (41 minutes, character `0x4bc82`, 14 zonings).

- **Perks**: `CharacterAction` 0xbb trains a rank and 0xbc untrains one, the rank's id second (`0x7d`), echoed; the perk
  actions an untraining takes away come as 0xb6 (the same fields as 0xb4). Ported (b192f41).
- **Tradeskill**: `CharacterAction` 0x51 (`UseItemOnItem`) with two backpack items (`(0x68, 0x56)` on `(0x68, 0x43)`, pattern
  pieces); the server sends the result (`TemplateAction`, `ContainerAddItem`) and deletes the pieces (0x70). Ported (c5f9d17).
- **Pets**: `PetCommand` (`6b333303`) 7 (attack) to all pets; `AddPet` names them after each zoning. Ported as `/pet` (1160fdd).
- **Player trade** with `0x57d8`: Open naming the partner, AddItem `(0x68, 0x54)`, credits 100000 then 1000000, Confirm, the
  partner's Accept, ours, Complete for both. Ported (ad6dbbe). The partner put nothing in, so their items' messages are unknown.
- **Team**: two invitations accepted (`TeamRequestReply` 0x1c, already handled); sending an invitation and kicking are not in it.
- Also seen, not yet used: `SetStat` 264 stepped 0 to 100 (a slider), `N3Teleport` before each zoning, `DoorFullUpdate`,
  `PlayfieldAllTowers` / `AllCities`, `ResearchUpdate`, `OrgServer`.

# The Arbiter / daily-mission / mail capture (2026-11-08)

`captures/events-20261108-1811/` (9.5 minutes, character `0x39c16ee4` at level 116, 5 connections, 14,491 messages; its
`NOTES.md` has the timeline). The original client crashed at the end: the Unicorn shuttle pad in ICC HQ answered its use with
an `N3Teleport` of destination type `0xc7a0` and every id zero (an instance not made yet); the log stops the same second, so
whether a redirect would have followed is unknown. The new client takes it as a move within the playfield (test
`the_shuttle_pads_teleport_is_a_move_within_the_playfield`).

- **Mission terminal offers** (`QuestAlternative`, the reply the server docs listed as never captured): the request's first byte
  4, scope 1, terminal `(0xdac1, c000028f)`; five offers, each with a reward descriptor (version 6; a 44-byte chunk of "MSRE"
  and the level; a 120-byte chunk with the deadline 2880 minutes, a mission code and the deadline's unix time), one reward item
  at the character's level, icon 11330, the destination playfield and place. Taking one (`CreateQuest`, the offer's identity)
  is answered by a `SimpleItemFullUpdate` (type `0xc76d`: the mission key), `ContainerAddItem` and the whole log. Ported to
  ao-server (`missions.rs`).
- **Daily-mission agency** (Freelancers Inc. HQ, 7012): `KnubotOpenChatWindow` echoed, `KnubotAppendText` lines, `KnubotAnswerList`,
  `KnubotAnswer` 0 twice, the log with the daily, `KnubotCloseChatWindow` with word 5. `KnubotAppendText` has no trailing word
  (the schema's `Unknown3` is in no capture: removed, which fixes the client's reading of NPC lines). Ported to ao-server
  (`knubot.rs`: the agency receptionist).
- **Mission completion** (RALPH in Palmiero's Office, 7001): the server's `CharacterAction` 0x3b naming the mission, then the
  log's `Quest` Delete. Ported (`missions::accomplished`).
- **Mail** at the Old Athen terminal: `Mail` action 1 (inbox), 1 with the id (open: action 2 reply, flags 0x5c to 0x5d), 3 (take:
  flags 0x5f, `TemplateAction` Added + `ContainerAddItem`), 5 (delete); stat 0x289 goes 1 then 0. Ported to ao-server (`mail.rs`).
- **Item use**: an item-shop lottery ticket (303463) used from the backpack: `FormatFeedback` "Congratulations!", the prize
  (294604) added, the ticket used (`TemplateAction` 0x03) and deleted (0x70); the prize used: "New Victory Points gained.",
  stat 0x29d. Ported to ao-server (`useitem.rs`).
- **Zoning types**: `0xc79c` a playfield, `0xc79e` the Grid (152, from the Old Athen Grid Access terminal), `0xc7a0` a move
  within the playfield (and the shuttle's instance-to-be).
- **Guards**: `2049527c` ("Action") from `0xc749` guards toward a player who attacked them, with the PvP flag. Not ported.
- Also in it: seasonal monsters and their corpse around ICC HQ, an event character speaking in chat, Andromeda's tower fields
  and cities on entry, `SetWantedDirection` on the office's NPCs.

