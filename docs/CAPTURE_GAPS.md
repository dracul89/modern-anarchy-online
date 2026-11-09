# What the captures have that the client does not handle

From the three recordings of the original client (`captures/arete-20261030-1655`, `arete-20261031-0842`,
`arete-20261031-1505`, see `ARETE_CAPTURE.md`; not in git). Every server-to-client N3 message kind and every
`CharacterAction` (`5e477770`) action code was counted (the counts below are per capture: 16:55 / 08:42 / 15:05) and
checked against `crates/mao-client/src/net.rs` (`World::add`, the play loop), `hazards.rs`, `npcchat.rs` and `mao-net`.

The Gamecode handler of a message is its `...IIR` class: slot 2 of the vtable applies it, slot 7 reads it (the same pattern
as `DoorStatusUpdateIIR_t`: apply FUN_100a015a, read FUN_100a0330). A server `CharacterAction` is applied by FUN_1005d61f,
a switch through the byte table at 0x1005f526 (`case = table[action - 1]`, jump table 0x1005f38a). In the layouts, a
`CharacterAction`'s body is: a byte, the action (`u32`), a word, the target identity (type, instance), parameter 1,
parameter 2, a short (offsets 0, 1, 5, 9/13, 17, 21, 25). Message bodies start with the N3 header's byte.

## Ranked by what a player would notice

| # | kind / name | what it is | count | layout | Gamecode | proposed port |
|---|---|---|---|---|---|---|
| 1 | `4d450114` SpellList | a spell list run on a character: NPC AI event loops, duel buffs, environment effects ("Effect of Inferno Environment"), item effects | 3858 / 171 / 99 | byte; list (`x3f1`) of whole spells (`SpellData_t`, big-endian: type, instance, version, criteria, values); caster identity; target identity; byte (ending); short-length string (the list's name); byte flag (then the source nano/item identity, `cf1b`); byte; int | `SpellListIIR_t`: read FUN_100aabcb (spells FUN_100a71ae), apply FUN_100aae0e → `Beholder_t` +0x30 FUN_100026d0 → FUN_100a5f4b per spell | **ported** (`spelllist.rs`): effects of 0xcf26/0xcfd4 spells drawn on the character with their sounds, criteria (`requirements.rs`), queued runs, targets, the stun (one's own character held still) and stat sets. Most lists (0xcf3b "Cast nano") are empty on the client. Still to do: see section 1 |
| 2 | `43197d22` N3Teleport | the server moves a character: before every zoning, and in a generated mission (identity type `0xc7a0`) a move inside the playfield with no playfield message | 111 / 16 / 16 (own: 24 / 9 / 14) | byte; destination (3 floats); rotation (4 floats); byte (`0x61`); playfield identity (type `c79c` plain, `c79e` instanced, `c79f` generated, `c7a0` the same playfield); server id; group id; `9c50` identity; 2 ints; a second playfield identity; a tail length byte-int (8, 0x0c or 0x10) and that many bytes (`3f000000 0`, or the destination again) | N3.dll `n3TeleportIIR_t::Activate` (vtable 0x1003e68c) | **ported** (`mao_net::n3::teleport`, `World::add`): within the playfield (the proxy's instance zero) the dynel is put at the destination with the rotation at once; own zoning: the playfield stops (its dynels gone, the character still) until the playfield message. Left: the two identities at +0x4c/+0x54 |
| 3 | `CharacterAction` 0xdf / 0xe0 / 0xe1 / 0xe3 / 0xe4 (with the client's 0xdc / 0xdd / 0xde) | the tradeskill window: source and target accepted, "not a valid combination", the skill requirements, the result's template | 6 / 12 / 2 / 6 / 4 (16:55) | 0xdf, 0xe0: parameter 2 a slot or count; 0xe3: parameter 1 the skill, parameter 2 the value needed (`a0`/`a5`: 0x19, 0x3c); 0xe4: target `(1, template)`, parameter 1/2 `(1, template)` | FUN_1005d61f cases 92..97 (0x1005f1a1, 0x1005f1b8, 0x1005f1cf, 0x1005f1f7, 0x1005f212) | **ported** (`tradeskill.rs`): the Tradeskill Kit window with its source, target and result slots, Clear and Build and the quality slider; the client's 0xdc/0xdd/0xde and the server's answers as the window's texts (section 3) |
| 4 | `194e4f76` AddPet, `58742a0f` RemovePet, `0d381f02` PetToMaster | the character's pets: which creatures are one's own, and their reports | 0 / 3 / 28, 0 / 1 / 2, 0 / 0 / 4 | AddPet / RemovePet: byte, the pet's identity. PetToMaster: byte, the pet's identity, a command (1: with a value, 2: without), the value (`0x0a`), an identity | `AddPetIIR_c` read FUN_10071972, apply FUN_100719a7 (FUN_1005299c adds the pet); `RemovePetIIR_c` FUN_10076dc0 / FUN_10076df5; `PetToMasterIIR_c` FUN_10076917 / FUN_1007699d (signal FUN_10011fa1) | **ported** (`pets.rs`): the pet list in `World`, the Pet Window, and `PetToMaster`'s red flashing of the pet's name. Left: the window's icon menu, the pets' nano icons |
| 5 | Research: client `3115534d` (ResearchRequest), `253d0240` ResearchUpdate, `435f7023` "PerkUpdate", client `SetStat` 263 / 264 / 265 | the Research window: the request when it opens (always answered by ResearchUpdate), the global research lines, the personal research progress, and the window's sliders and chosen goal | client 2 / 0 / 1; 24 / 10 / 15; 156 / 4 / 0; client `SetStat` 20 / 0 / 16 | ResearchRequest: one byte 0. ResearchUpdate: byte, byte, 54 lines of (id, 3 ints) (all zero in the captures), int. "PerkUpdate": byte, research id, the goal (stat 265), the experience still needed (0xb93b, 0xb593, 0xab6f: falling). `SetStat` 263 / 264: 0..100 (the slider percentages), 265: the goal (0x988, 0x989, 0x992) | `ResearchUpdateIIR` read FUN_1003a7a0, apply FUN_1003a81c (signal +0x244); `PerkUpdateIIR` read FUN_100624bf, apply FUN_10063e88 (sets stat 0x109, the progress FUN_10052d86, "got_tech" when done) | **ported** (`research.rs`): the three-tab Research window; `3115534d` on opening, `SetStat` 263/264/265 from the sliders and the goal, the progress from `435f7023` and the world entry's perks (section 5) |
| 6 | `365a5071` DoorFullUpdate | the doors of a generated mission, with their places | 108 / 0 / 93 | a `SimpleItemFullUpdate` (byte, version 0x0b, owner 0/0, position, rotation, playfield, the `f424f` state machine, a short, stats `x3f1`: stat 0 `c0081483`, 23, 701-703, 412, 252, 192, 193, 195, 259) then the lockable/door part (int 0, int 2, int 0x32, an empty `x3f1` list, int 2, two shorts) | `DoorFullUpdateIIR_t` read FUN_1009ff81, apply FUN_1009fff9 (creates the door dynel, opens or closes it by flag 0x80) | **ported**: the door is a usable object at its place, drawn by its template's model (stat 23, its `Mesh` 12) turned by its rotation. Still to do: its open state |
| 7 | `CharacterAction` 0xcc UploadNano | a nano crystal used: the nano is learnt | 5 / 0 / 0 | target own; parameter 1 `cf1b`, parameter 2 the nano | case 86 (0x1005ed2d): FUN_10050100 on the nano holder (+0x1c0), signal 6 | **ported**: the nano joins the Nano Programs list |
| 8 | `CharacterAction` 0x89, 0x92 | sent on every move start: cancel the nano being cast (0x89) and the item being used (0x92) | 36 / 40 / 299, 36 / 45 / 300 | no parameters (0x92 can name the item used: `(0x6b, handle << 16 \| place)`) | case 43 (0x1005d78f): FUN_1004f7b8 empties the casting list (+0x1c0), each cast's timer and its `cf1b` signal; case 48 (0x1005e33a): FUN_1004949c ends the use timer (+0x198) and signals the item's container changed (a refresh) | **ported**: 0x89 ends the character's cast (its sounds and casting animation); 0x92 ends the item use timer (the kind-4 entry a use starts) and refreshes the container it names (it does not close it) |
| 9 | `55220726` PlayfieldAllTowers, `59210126` PlayfieldAllCities | the Notum Wars towers of the playfield, and its organization cities | 21 / 10 / 14 each | Towers: byte, `x3f1` count, then per tower: (`c354`, field id), (`c350`, the tower's id), position (3 floats), low and high template, an int, a scale float, a side int (Andromeda: 0x1a1ca/0x3f1 - 1 = 105 towers). Cities: byte, a short length (0 in the captures), raw house data | `PlayfieldAllTowersIIR_t` read FUN_1012aeab, apply FUN_1012ae74; `PlayfieldAllCitiesIIR_t` read FUN_101304f8, apply FUN_101304d5 (`UpdateNewHouses`) | tower markers on the planet map (`planetwindow.rs`) and the land control info; cities: nothing to do until a capture has one |
| 10 | Other characters' animations: `CharacterAction` 0x6b, 0x57, 0xa7, 0x83; `51492120` CharSecSpecAttack from the server | casting ends, standing up, a stance flag, a weapon equipped, a special attack performed (by others) | 0x6b 19 / 38 / 35; 0x57 168 / 27 / 1; 0xa7 6 / 12 / 10; 0x83 6 / 0 / 0; 112 / 26 / 13 | 0x6b: parameters `(1, nano)`; 0xa7: parameter 2 0 or 1; 0x83: target the weapon `(c74a, id)`, parameter 2 6; CharSecSpecAttack: byte, target identity, the special's stat (0x94, 0x96) | case 29 (0x1005de04, FUN_1004f542); case 22 (0x1005dc76: stance stat 0x1ae picks animation 0x29/0x2a/0x25); case 57 (0x1005e919: flag 0x800 on the stats at +0xe8); case 41 (0x1005dcfe) | **ported**: 0x56/0x57 sit and stand others (0x57 sits a sleeping or lounging one up), 0x83 puts the weapon in the hand, 0xa7 keeps the flag 0x800 that 0x83 reads, `CharSecSpecAttack` queues the attacker's special so its `SpecialAttackInfo` swings the special's class. 0x6b has nothing drawn. See section 10 |
| 11 | `CharacterAction` 0x50 QueuePerk | a perk action queued | 0 / 44 / 0 | parameter 1 2, parameter 2 the delay (0x64, 0xc8: hundredths) | case 19 (0x1005f12e) | **ported**: the original's "Special" timer bar (`StartTimerBar` kind 2) runs for the delay; the game timer bars are drawn (`ui::TIMERS`) |
| 12 | `CharacterAction` 0x9b, 0xa2 | sneaking: one 0x9b per creature that sees the character, then 0xa2 "started sneaking" | 17 / 0 / 0, 2 / 0 / 0 | 0x9b: target the creature | cases 51 (0x1005e361), 55 (0x1005e54d) | **ported**: 0x9b tells "You have been detected by X!", 0xa2 (and movement 0x1c) put the character in the sneak mode: its animation and walking's speed |
| 13 | `2a293d0f` Fov | a creature's field of view (a generated mission's guards) | 59 / 0 / 0 | byte; 2 floats (0.384, pi: the cone); byte; 5 floats; int (0x14) | `FovIIR_c` read FUN_10073ba2, apply FUN_10073c32 (sets the creature's view, FUN_100524f4, and stat 533 NPCFovStatus = 1) | **not ported**: the original draws nothing from it (no reader of the view at the creature's +0x24 traced; `NPCToggleFov` has no user) |
| 14 | `3129233b` AreaFormula | an area effect landing on a list of characters | 4 / 9 / 0 | byte; `x3f1` list of character identities; the formula (an int: 0x3e380, 0x18762); the source identity | `AreaFormulaIIR_t` read FUN_100721dc, apply FUN_1007212d → FUN_100a9a99 (FUN_10051c98 on each character) | **ported**: the formula is a nano (`0xcf1b`, read from 0xfde85 records): its landing glow on each listed character the client has |
| 15 | `485e7202` InventoryUpdated | the server says a container changed | 2 / 0 / 0 | byte; an int (5) | `InventoryUpdatedIIR_t` read FUN_1007535f, apply FUN_10075392 (on the own character: signal 5, FUN_1001143e) | **ported**: on one's own character the inventory windows are drawn again |
| 16 | `CharacterAction` 0x64, 0x6e, 0x2f, 0x61, 0x54, 0x15, 0xab, 0xd0, 0x84, 0x81/0x8a/0x8c, 0x93, 0x56, 0x1b, 0x23, 0x7a | small or unknown: see below | small | see below | see below | see below |

Handled and so not in the table (checked in `net.rs`): `SetWantedDirection` (`60201d0e`, turns the character),
`StopMovingCmd` (`742e2314`, ends its path), `OrgServer` (`64582a07`: only type 6, the organization benefit, occurs in the
captures, and it is read), `ItemReplaced`, `TemplateAction`, `AddTemplate`, `Skill`, the team messages, `Mail`, `Bank`,
the hazards (`MineFullUpdate`, `TrapItemFullUpdate`, `CentralController*`, `Action`), `FightModeUpdate` (`371d0542`, read
as the travel list in `mao-net/destinations.rs`), and the `CharacterAction` codes 0x1a, 0x20, 0x3b, 0x62 (own), 0x63, 0x70,
0xa4, 0xaa, 0xb4, 0xb6, 0xbb, 0xbc, 0xce, 0xcf, 0xba.

Not counted as a gap: `CharInPlay` (`570c2039`) from the server (154: a byte on another character when it is in play; the
client already shows characters by their full updates).

## The gaps

### 1. SpellList (`4d450114`) — ported

Read by `SpellListIIR_t` (FUN_100aabcb): a byte; the spells (FUN_100a71ae: an `x3f1` count, then each a whole `SpellData_t`
as `GameData::operator>>` reads it, the database's spell format in big-endian order: `mao_formats::spell::read_network`);
the caster (+0x1c, zero for the effect lists); the target (+0x28); a byte, the list is ending (+0x24, each run's bit 0); the
list's name (a short length, +0x30); a flag and the source nano or item (`cf1b`, +0x54); a flag (+0x4d); an int (+0x50, 0 in
all captures). The apply (FUN_100aae0e) calls the character's `Beholder_t` +0x30, FUN_100026d0, which runs each spell with one
hit and no delay at once, and queues the others, through FUN_100a5f4b (the spell types' jump table at 0x100a6cb6) on the
dynel its target stat (0x20) names (see below).

All 4128 captured messages read whole. By spell type: 0xcf3b "Cast nano" 2059 (the NPC event loops, "Arete Landing",
"Effect of Inferno Environment", the AI lists), 0xcf82 + 0xcf56 1436 (the duel buffs), 0xcf26 573, 0xcf0a 81, 0xcf22 36,
others under 12. What the client does with them (FUN_100a5f4b):

- **0xcf3b**: FUN_100a7814 is empty: the commonest lists show nothing on the client (the server casts the nano itself).
- **0xcf26** (FUN_100a55d9): the effect of stat 39 made on the character (`CreateEffect2`), for stat 49 hundredths when set.
  Ported: the 16:55 capture's Malfunctioning Cleaning Robots at Arete Landing get 0xa8e3 (a flare, the fire seen burning on
  them in the screenshots) and 0xa871 (a star burst at the pelvis); also 0xa9a1 and 0xa8e0 once each. A flare burns where the
  character stood (it does not follow it); other effects run on the character with its programs' effects (`BuffsFX`).
- **0xcfd4** (FUN_100a9159): the same effect, ported without its colours, scale and repetitions; the only captured ones
  (Shadowland Anvian smoke and sparks) carry criteria ("On Caster", stat 410 above 0), which now pass: the criteria
  evaluator (FUN_100065d8, `requirements.rs`) checks below / above only on the client's own character.
- **0xcf82 "Stun"** (FUN_100a879c, the duel buffs' begin and end): state flags of stat 224 (counted, FUN_10044aac /
  FUN_10044c71, started from the stat by FUN_1004496c): 2 and 4 taken, 0x400 given (an NPC also 0x100, 0x4200); the
  ending list gives them back. Ported: one's own character cannot move while neither 2 nor 4 is set (FUN_1006bd94,
  `N3Msg_MouseMovement`).
- **0xcf22** (FUN_100a4f8a): stat 0 set to stat 39 (689 on pets, 359 on two NPCs), except 0x3c, 0x25, 0x2f, 4, 0x3b, 0x16d,
  0x1b, 0xd6. Ported for one's own character (the client keeps no stats of others).
- **0xcf0a, 0xcf56, 0xcf4a, 0xcf70, 0xcf1b, 0xd003**: nothing on the client.

The targets (FUN_100026d0, spell stat 0x20): 3 the list's caster (+0x1c; without it in the playfield the rest of the
list stops), 0xe the character's fighting target (+0x1d4's +0x4c; an ending list: the caster), 0x17 its use target
(+0x1e4's +0x14, not kept by the client: left out), anything else the character. Runs with several hits or a delay are
queued (FUN_10002595) and run each frame (FUN_10002fc7 → FUN_100a5f4b: a fresh run is put back the delay's whole seconds,
then hits every |delay| hundredths; failing criteria spend a hit). Every captured spell has one hit and no delay, so the
queue is tested on made-up lists only. All 4128 captured lists run: 576 effects, 720 stuns and 716 ends, 36 stat sets.

Still to do: an ending list (+0x24 set) deletes the effect whose handle the spell keeps in stat 0x42, which a spell fresh
from the message never has (none ends an effect in the captures); 0xcf16 runs (always queued, FUN_100a517a / FUN_1005c831);
0xcf92 (FUN_100a8107: a vehicle's animations), 0xcf4c, 0xcf44, 0xcf7d (feedback lines), 0xcf35; the stun's call to the
vehicle (FUN_1005a02c(1)) and what flags 0x400 / 0x100 / 0x4200 do; the criteria operators that read state the client does
not keep (times 5/6, slots and items 7/8/0x1f-0x24, nanos 0x5b/0x5c/0x65/0x66, perk locks 0x5e/0x61, teams, organizations,
playfields, positions, vehicles: they give "unknown", taken as failing), the stat-against-stat operators 0x82-0x85 and the
"has perk" side-table for perks with entry flag bit 7 (FUN_1002be1a).

### 2. N3Teleport (`43197d22`)

Read by N3.dll's `n3TeleportIIR_t` (not Gamecode). Every own zoning starts with one (then the redirect and the playfield
message, already handled). Two cases are not covered:

- In a generated mission, the server moves the character inside the playfield (playfield identity type `0xc7a0`, all
  zero ids, tail `0x10`: the destination again): 4 in the 16:55 capture (c10, c22), e.g. to (121.4, 148.4, 87.8). No
  playfield message follows; the server also sends the character's `SimpleCharFullUpdate` at the new place and echoes a
  position, which the play loop's "the server put us back" check (`own_report`) happens to apply, without the heading.
- In-connection moves with a tail of 0x0c (a vector): the 16:55 capture's move at 68 min and the 08:42 capture's
  Shadowlands moves (followed by a playfield message in the same playfield).

Others' `N3Teleport` (87 / 7 / 2): another player leaving by a whom-pah or a door.

**Ported.** N3.dll (decompiled: `n3TeleportIIR_t` vtable 0x1003e68c) reads (`ReadSubClass` 0x10029e7f): the destination,
the rotation, a `PlayfieldProxy_t` (FUN_10038402: version byte `'a'`, playfield identity, server id, group id, an identity
`9c50` with the playfield id or all zero), two identities and a byte string (int length, under 0x3e9). `Activate`
(0x10029f87) tests the proxy identity's instance (+0x48): zero, any dynel the client has is put there
(`UpdateLastAllowedPosition`, `SetRelPosRot`; for one's own the camera is put back, FUN_10022b2d, and `ToClientDynelAlive`
tells the client's own modules: nothing goes to the server); not zero, only one's own character matters
(`n3EngineClient_t::StartTeleport` 0x100076ad → `StopPlayfield` 0x1000ccbd: every dynel dies and the playfield stops; no
loading screen is raised there, `GetTeleportStatus` has no user). All the others' teleports in the captures go elsewhere and
are followed by their `Despawn`; the 08:42 capture has one other character moved within a mission. The client
(`mao_net::n3::teleport`, `World::add`, the play loop): the move within the playfield at once with its heading; a zoning
stops the character and clears the others until the playfield message (the client lets go after 30 s, a safety of its own).
Left: the identities at +0x4c (zero in the captures) and +0x54 (`186a1`, `186a2`, `186a3`, `186a7`), and the tail.

### 3. The tradeskill window (`CharacterAction` 0xdc..0xe4)

The 16:55 capture used the original's Tradeskill window: the client sends 0xdc (source changed) and 0xdd (target changed)
with the items and 0xde (build); the server answers 0xdf / 0xe0 (accepted, parameter 2 a slot or count: 3, 0x154, 0x4ff,
1), 0xe1 (not a valid combination), 0xe3 (a requirement: skill 0xa0 / 0xa5 and the value, 0x19 / 0x3c) and 0xe4 (the
result, `(1, 0x26ad0)` twice). The client combines by dropping one item on another (0x51, as the 15:05 capture's pattern
pieces did), which the server takes too; what is missing is the window with its requirement lines and the predicted
result.

### 4. Pets (`AddPet`, `RemovePet`, `PetToMaster`) — ported

`AddPet` names a pet after each zoning (the 08:42 capture's `0x7b1de89e`; the 15:05 capture's two, `0x7b1e6d64/65`);
`RemovePet` when one goes; both keep `World::pets`, shown in the Pet Window (`pets.rs`, GUI.dll's `PetWindow_c`).

`PetToMaster` (byte, the pet's identity, a command, a value, an identity): `PetToMasterIIR_c`'s apply (FUN_1007699d), for a
pet the client has and one's own character, raises global signal +0x40 (FUN_10011fa1) with the pet and, for command 1, the
value (0x0a in the captures, beside "Your pet is out of range"), for command 2 zero. `PetView_c` (FUN_1007d817) takes it in
FUN_1007d273: its counter +0x140 becomes twice the value; its 500 ms timer (FUN_1007d72b) takes one off while it is not
below zero, and while it is odd (not -1) the name is `#ff0000` (FUN_1007d67c) and the bar the other way round (FUN_1007cf95:
empty in the playfield, full away). **Ported**: the name flashes red ten times over ten seconds.

Left: the window's icon menu (FUN_1007e75e: "Show buttons" on the window's +0x78, "Fixed Pet Buttons" on the distributed
value `EnableFixedPetButtons`, "Remember Pet Names" on `AutoPetRenaming`, each a boolean entry; the client's windows have no
icon menu yet), and the nano icons under each pet.

### 5. Research — ported

The research window's traffic: the client sends `3115534d` (one byte, 0) when the window opens, and the server answers with
`ResearchUpdate` (the 54 global research lines, all zero for these characters); the sliders are `SetStat` of stat 263 and
264 (0..100, the schema names them `PersonalResearchLevel` / `GlobalResearchLevel`) and the chosen goal stat 265
(`PersonalResearchGoal`: 0x988, 0x989, 0x992). aosharp's "PerkUpdate" (`435f7023`) is the personal research progress:
`PerkUpdateIIR`'s apply (FUN_10063e88) sets stat 0x109 (265) and the progress, and sets the "got_tech" value when it is
complete. Its 13 bytes are the leading 1, the rank, the goal and what is left of the rank; `ResearchUpdate`'s lines are a
global rank and what is left of it for side 0, 1 and 2 (FUN_1003a88c writes them so); the world entry's `Perks` carry
what is left of each personal rank in their last field. The global goal is stat 266 (`SetStat` from the window's global
radio buttons, FUN_100f5382; not seen in the captures). Ported: `research.rs` (`docs/GUI_DLL_PARITY.md`). Not captured:
whether the server echoes `SetStat` 265/266, and a `ResearchUpdate` with global research still running (every captured
line is 0, all finished).

### 6. DoorFullUpdate (`365a5071`) — ported

Only in generated missions (the ship `0x20da1b` and `0x20e000`). Each door: identity `(0xc748, 0x10c32dc1..)`, a
`SimpleItemFullUpdate` body (position, rotation, playfield, stats: stat 0 `0xc0081483`/`0xc0081403`, `StaticInstance` 23
`0x3de4d`, the ACG stats 701-703, 412, 252, 192/193 the external playfield and door, 195 the access key, 259 the door
flags), then the lockable part. Ported: the door is a usable object at its place, named "Door", with the door type, so it
can be targeted and used, and it is drawn by its template's model (stat 23's item template, its `Mesh` stat 12) turned by
the announcement's rotation, as the announced terminals are (test `a_generated_missions_doors_are_usable`). Not ported:
its open state (flag 0x80 of stat 259).

### 7. UploadNano (`CharacterAction` 0xcc) — ported

Using a nano crystal: the server sends 0xcc with `(0xcf1b, nano)`, then `TemplateAction` and `DeleteItem` (0x70) for the
crystal. Ported: the nano joins `program_ids` (test `an_uploaded_nano_joins_the_programs`).

### 8. Cancel on moving (`CharacterAction` 0x89, 0x92)

After nearly every client move that starts walking the server sends 0x89 then 0x92 (299 / 300 in the 15:05 capture). In
Gamecode, 0x89 empties the character's list of nano casts under way (each cast's timer stopped and a `cf1b` signal sent:
the casting is interrupted; with the list empty FUN_1007c8af leaves the casting animation state 5) and 0x92 ends the item
use timer (+0x198: started by FUN_100482c7 on a use, the item's stat 211) and signals the container of the item it names as
changed (FUN_1001173e, the signal that also follows `N3Msg_DropItem` and `N3Msg_ContainerAddItem`: a refresh, not a close).
The one 0x92 with a target (08:42) names `(0x6b, 0x70 << 16 | 1)`, an item used out of an open corpse. **Ported**: 0x89 drops
the character's cast (its gather/travel sounds) and its casting animation; 0x92 ends the item use timer (the kind-4 entry
`start_item_timer` makes from the item's delays, its bar with it) and refreshes the open container of that handle.

### 9. PlayfieldAllTowers / PlayfieldAllCities

Sent at every playfield entry with the identity `(0x9c50, playfield)`. Towers only on Notum Wars playfields (Andromeda 655:
105 towers, the Wailing Wastes 551: 34). Decoded (`mao_net::landcontrol`, from Gamecode.dll's `LandControl_t`): a header byte,
the towers counted `(n + 1) * 0x3f1` (FUN_1012b192), each (FUN_1012bb14) the site `(0xc354, ..)`, the tower `(0xc350, ..)`, x,
height, z, the model, the side (0 neutral, 1 Clan, 2 Omni), the destroyed model, the scale and the kind (1 control tower,
2 turret, 4 conductor); the list replaces the one before. `PlayfieldTowerUpdateClient` (`5b1e052c`, FUN_1012b38a: site, kind
1 destroyed / 2 planted plus a tower) changes it; none captured. Cities (FUN_101304f8) are an `i16` length and a blob for
`PlayfieldCityHolderClient_c::UpdateNewHouses`, always empty (length 0) in the captures, kept and not drawn.

Drawn as GUI.dll's `PFMapRenderer_c` (the PF Map window, FUN_100ea243) draws them, on the corner map and the overlay
(`automap.rs`): a 16-pixel `GFX_GUI_MAP_CONTROLLER/GUARDTOWER/BUFFTOWER_RED/BLUE/GREY` icon per tower (Clan red, Omni blue,
the rest grey), and with the land-control button on (`RenderLandControlData`, the expansion stat 389 bit 0 needed) an
80-pixel `GFX_GUI_MAP_CIRCLE` over each, `0xffff04` for control towers and turrets, `0x00dcff` for the rest. The planet
map window (`PlanetMapView_c`) does not draw towers in the original. The icon ids FUN_100ea4b9 packs (0x10a-0x116) are
AFCM.dll's GFX names (the table at RVA 0x16060, id 0x8a `GFX_GUI_CHECKBOX_CHECKED`): `MAP_ARROW`, `ARROW_SELECTED`,
`BIG_ARROW`, `BUFFTOWER_BLUE/GREY/RED`, `CIRCLE`, `CONTROLLER_BLUE/GREY/RED`, `GUARDTOWER_BLUE/GREY/RED`, as the client had
inferred (tested against the dll). The circles' alpha 0 (FUN_100467e8's vertex colour) is never read: their state block
(+0x57c, FUN_100eb7d5) takes alpha from the texture alone (`ALPHAOP SELECTARG1 TEXTURE`, blend `ONE`/`INVSRCALPHA`), and the
skin's circle is white on the green key, so it draws opaque in its tint.

The land-control view (the button on): the playfield's land-control map (`LandControlMap_t`, record `0xf4248`, loaded by
Gamecode's FUN_1011f406 with the area and district info; GameData's `ReadBlob` 0x10008680: version 1, flags (bit 0 zlib),
width and height in tiles of 4 world units, a bit per tile, `mao_formats::landcontrolmap`; all 39 records read, none
compressed) comes to the GUI by `N3Msg_GetLandControlBitmap` (MapData_c +0x34, FUN_10042f00) and is drawn over the map
(FUN_100ea88a, normal alpha blending) as FUN_100ebc3f colours it in A4R4G4B4: a tile a tower may stand on `0xf392` green, any
other `0x4000` faint black on a playfield with a map picture (MapData_c +0x80) or `0xf555` opaque grey without one; the
circles come only with that picture made (+0x4b4). The character's district (`N3Msg_GetLCDistrictInfo`, the zone's
`DistrictData_t`: name +8, land control range +0x44/+0x48) shows as "%s<br>Lvl Rng: %d - %d" when both ends are 1 or more
(FUN_100eb04b, every half second); the original puts that text at 5, 5 of the map, the client under the button. The
button is MapControlView_c's toggle (FUN_100ecdf7): text.mdb 10000 `Info`, tip `ShowLConMap` ("Show land control areas on
the map"), disabled without the expansion, its state archived as `RenderLandControlData` (FUN_100ebfdc) and kept by the
client in `automap.ini`. Not ported: the control's `Build` and `War` buttons (the tower build and war menus) and the
placement circle at the character (MapData_c +0x75, `0xdcdcdc`). Unknown: the bitmap's texture filter (none is set; the
client takes Direct3D's default, the nearest texel).

### 10. Others' animations — ported

`CharacterAction` 0x6b (FinishNanoCasting, `(1, nano)`), 0x57 (StandUp, sent for others and echoed for oneself), 0xa7
(a stance flag, 0x800, on or off), 0x83 (a weapon equipped, its identity), and the server's `CharSecSpecAttack` (the
attacker, the target and the special's stat) drive what other characters are seen doing. The client times the casting
animation itself and does not play stand-ups or special attacks of others.

**Ported** (`World::add`, test `others_stand_up_wield_and_swing_their_specials`):

- 0x56 (case 21, 0x1005dc6a) and 0x57 (case 22, 0x1005dc76) hand the character's movement controller (+0x50 then +0x178,
  vtable +0x18) a movement: 0x56 sitting (0x1e), 0x57 standing up (0x25 `LeaveSit`, or 0x29 / 0x2a when `WaitState`, stat
  430, is 0xf / 0x10: lounging and sleeping, not kept by this client). Others sit and stand by them; one's own sitting stays
  the play loop's (the server echoes the client's own 0x57).
- 0x83 (case 41, 0x1005dcfe, FUN_1006b2dd): the weapon named in the target is equipped in the place in parameter 2 and drawn
  in the hand (FUN_1006ac49 under place 0x30); out of a vehicle only.
- `CharSecSpecAttack` (`CharSecSpecAttackIIR_t`: read FUN_10072cbc, apply FUN_10072d11, then the attack part's FUN_10068cd9):
  with no special queued (+0x9c) and the special not in the list at +0x1bc, its stat is pushed on the queue at +0x8c, whose
  front (FUN_10068aa3) gives the next `SpecialAttackInfo` its class. Every captured one comes just before its attacker's
  `SpecialAttackInfo` with the same stat.

- `WaitState` (stat 430): a `SocialActionCmd` of type 1 with 0x44 or 0x45 (FUN_1007aec0) hands the movement controller
  `SwitchToSleepMode` (0x21) or `SwitchToLoungeMode` (0x22): FUN_1006ec48 sets `WaitState` 0xf and plays 0xed `sleep-ground`
  into 0xee, FUN_1006edf2 sets 0x10 and plays 0xef `lounging` into 0xf0. 0x57 then asks `LeaveSleepMode` (0x29) or
  `LeaveLoungeMode` (0x2a), which go back to sitting (mode 8; FUN_1006ece8 and FUN_1006ee92 play the lie-down backwards into
  0xd7 `idle-ground`), not standing; one's own sit toggle sends 0x57 then (`N3Msg_SitToggle`). **Ported**: `World::resting`
  holds the lie-down's last pose (like sitting's `ground-start`); the backwards plays are not drawn.
- 0xa7 (case 57, 0x1005e919) sets (parameter 2 not zero, through the stats' vtable +0x18: an OR into +0x138) or clears the
  dynel flag 0x800; a full update's flag bit 21 sets it too (FUN_1007835c; none of the captured full updates has it). Its
  readers are the weapon holder's: FUN_1006ac49, run by 0x83 (FUN_1006b2dd), empties the place 0 shown in the hand when the
  weapon goes to another place and the flag is clear; FUN_1006ab10 and FUN_1006b14c (the weapon list, FUN_10079de3) show the
  weapon of place 100 in place 0 when it is set. **Ported**: the 0x83 part (`World::weapon_flag`). Not ported: the place-100
  weapon (no capture has one).
- 0x6b (case 29, FUN_1004f542 on the nano holder +0x1c0) clears the "still casting" byte (+0x1c) of the cast of that nano and
  keeps parameter 1 at +0x20. The byte's only reader is FUN_1004f8ac: `N3Msg_CastNanoSpell`'s check (FUN_1004fbe4 →
  FUN_1004fb8d, for a nano with flag 0x800000) looks among the finished casts on oneself (kind 2) for one whose nano line
  (stats 75, 546..551) the new nano can't replace, and refuses with "Feedback_NCUErrorNanoprogramCantReplaceOther". Nothing
  is drawn from it; that refusal is not ported (the client keeps no cast list with the nanos' lines).
- Sneaking (mode 6): see section 11-15.

### 11-15

- 0x50 QueuePerk (08:42), **ported**: answers the client's perk use (0xb3). Case 19 (0x1005f12e): parameters `(2, delay)`
  add an entry of kind 2 for the delay (hundredths) to the character's timer list (+0x1e8: FUN_1003d9e0, FUN_1003d8c3, which
  signals the interface for one's own character); `(0, 0)` removes the kind-2 entries (FUN_1003d496). GUI.dll's
  `StartTimerBar` (0x1005214f) shows kind 2 as the "Special" bar (text `Timer_Special`, 0xaaffff). The client draws the game
  timer bars (`ui::TIMERS`, `TimerBar_c`'s 128 by 16 bars from (40, 40), 20 apart); only kind 2 is started so far (the other
  timers' sources, FUN_1003bbf8/0x1003bc84 and the item use FUN_100482c7, are not ported).
- 0x9b / 0xa2, **partly ported**: 0x9b (case 51, 0x1005e361), for a creature the client has, tells
  `Feedback_YouHaveBeenDetected` ("You have been detected by %s!") with its name; 0xa2 (case 55, FUN_10044ff9) gives the
  movement controller the sneak mode (0x1c). The client keeps who sneaks (`World::sneaking`, also from movement 0x1c, until
  another mode or 0x24). **Ported**: in mode 6 (`SwitchToSneakModeTransitionAction_t`, FUN_1006cb58's case 0x1c) FUN_1006c370
  plays the character's sneak action (the +0x1dc part's +0x14, 0x66 `sneakcool`, set by the constructor FUN_1005d0b1) moving
  (backwards too; the original plays it reversed, this client forwards) and standing, a sidestep still `walk-left`/`walk-right`;
  FUN_1006f9eb has no case for mode 6 and gives it walking's speed, 1.5 (`_DAT_1015e7ac`), forwards and backwards.
- Fov, **not ported**: `FovIIR_c`'s apply keeps the view (a mode byte 0..2, angles; FUN_100524f4 at the creature's +0x24) and
  sets stat 533; nothing that draws it was found (the string `NPCToggleFov` has no reference).
- AreaFormula, **ported**: the formula is a nano id: FUN_100a9a99 runs FUN_10051c98 on each listed character's nano holder
  (+0x1c0) with the source, the nano looked up as a `0xcf1b` record (FUN_100a4b10), as a cast landing (FUN_1005141d). The
  client shows the landing glow on each.
- InventoryUpdated, **ported**: FUN_10075392 acts only on one's own character, raising the interface signal of the value
  (FUN_1001143e: 5 is +0xd8): the inventory windows are drawn again.

### 16. The small and unknown `CharacterAction` codes

| code | count | seen | Gamecode case | reading |
|---|---|---|---|---|
| 0x64 | 936 / 6 / 4 | on NPCs, next to `SpellList` and `SetNanoDuration`; parameter 1 0x0d..0x2f | 26 (0x1005ddba: FUN_1003c68e stores it at +0x1c of the +0x1dc part) | an NPC state value (unknown) |
| 0x6e | 151 / 5 / 27 | right after the hazards `Action` (`2049527c`) on a corpse or chest that was used, and after the client's 0x8e | 32 (0x1005ddf4: FUN_10046e7a → FUN_1003f61c on the +0x1b8 part) | the end of an action's progress (the client's 0x8e starts one: `N3Msg_GetActionProgress` checks 0x8e) |
| 0x2f | 5 / 0 / 0 | target a terminal, weapon or item, parameter 2 0x71 or 1 | 12 (0x1005da2c) | unknown |
| 0x61 | 4 / 0 / 0 | parameter 2 6, before `ContainerAddItem` | 23 (0x1005dcd7) | unknown |
| 0x54 | 10 / 0 / 0 | after the client's 0x51 (an item used on an item), parameters 2, 2, then `Feedback` | 20 (0x1005dc56) | a combine refused: FUN_10042451 (on +0x1d0) tells a requirement line by the target's type bits (2..0x400; 0x20 and 0x80 a skill, category 2002, named by the parameter); the captured type is 0, which tells nothing of its own: the `Feedback` that follows carries the text (handled) |
| 0x15 | 0 / 0 / 1 | parameter 2 0x18, after an `InfoPacket` | 3 (0x1005d7cb → FUN_10065c04: signals for values 0x11..0x14, 0x18) | unknown |
| 0xab | 8 / 2 / 0 | at zoning: parameters `(0xf4254, 0x48396)` | 61 (0x1005e965) | unknown |
| 0xd0 | 0 / 3 / 0 | after a perk lock (0xcf): `(0x2f6, own)`, `(0x1b, 0xcfce)` | 90 (0x1005edfc) | a perk's effect (unknown) |
| 0x84 | 1 / 1 / 0 | SpecialUnavailable | 42 | a special unavailable |
| 0x81, 0x8a, 0x8c | 1, 1, 3 | target own or the pet, parameter 2 0x1e or 0xc8, after `HealthDamage` | 39, 44, 46 | unknown (healing?) |
| 0x93 | 1 | parameter 2 2 | 49 | unknown |
| 0x56 | 1 | on an NPC, no parameters | 21 | **ported**: the character sits (movement 0x1e) |
| 0x1b, 0x23 | 1, 1 | team: 0x1b the team level, 0x23 TeamRequestResponse (who, `dea9` team) | 6, 9 | **ported** (`team.rs`). 0x1b (0x1005d7fc): for one's own character, interface message 0x10 with the target, whose instance is the level (the 16:55 capture's: 2); GUI.dll's `TeamViewModule_c::SlotTeamEvent` (0x1007ac17) keeps it (+0xec, `GetTeamLevel`, the window's header "Team level: %u") and in a team signals the window (+0x14) and tells "TeamLevel_isX" ("Team level is %u."). 0x23 (0x1005d8cb): the target's team part takes the team identity (FUN_10065c8b, and stat 6 set to it), one's own team part takes the target (FUN_10065c5e), then message 0x17: `SlotTeamEvent` keeps the target as the leader (+0xe0; `IsTeamLeader` is whether it is oneself) and signals +0x1c, whose slot FUN_10078bc2 colours the leader's name 0x11eeaa (FUN_10077b3d; the others white in the playfield, 0x888888 away). Left: message 0x36 (the team side, `TeamSide_isX`) has no sender found; the leader's own button; the grey of members away |
| 0x7a | 0 / 0 / 1 | StopLogout | 37 | **ported**: interface message 0x2a, GUI.dll AFCM 0x20 `CancelCampMessage`: the logout bar goes and "Timed logout aborted." is told (when the client had not already done so) |

### Client messages the original sends that this client does not

| what | count | layout | reading |
|---|---|---|---|
| `3115534d` | 2 / 0 / 1 | one byte, 0 | ResearchRequest: always followed by the server's `ResearchUpdate` (opening the Research window) |
| `SetStat` 263 / 264 / 265 | 20 / 0 / 16 | value, stat | the Research window's sliders (0..100) and its goal |
| `CharacterAction` 0x6a | 0 / 0 / 1 | no parameters | `N3Msg_UseItem`'s branch for the bank (`0xdead`, FUN_10072a88 with 0x6a) |
| `CharacterAction` 0x8e | 35 / 2 / 18 | parameter 1/2 `(c76a, 0x0103e8xx)`: a corpse and a slot | starts an action with progress on a corpse slot (answered by 0x6e); `N3Msg_GetActionProgress` tests 0x8e |
| `CharacterAction` 0x98 | 10 / 9 / 16 | the target that just died (or none) | sent when the attack's target dies (repeated), answered by `StopFight` |
| `CharacterAction` 0x107 / 0x108 | 1 each per login | own identity | at entry (`N3Msg_RequestClaims` uses 0x107, `N3Msg_RequestClaimsDBCheck` 0x108) |
| `CharacterAction` 0x80, 0x35, 0x34, 0x41 | 6, 3, 1, 2 | 0x80: the pet's identity; 0x35: two backpack places; 0x34: a place and a count; 0x41: a nano | 0x41 RemoveFriendlyNano and 0x34 SplitItem are named by the schema; 0x35 moves (echoed by the server); 0x80 a pet command |

## Not decoded

- The meaning of `CharacterAction` 0x64, 0x2f, 0x61, 0x15, 0xab, 0xd0, 0x81/0x8a/0x8c, 0x93, 0x56: only their Gamecode cases
  are known.
- `PetToMaster`'s command values (1 with 0x0a, 2).
- The spells inside a `SpellList` list (the database records of types `cf3b`, `cf82`, `cf26`, `cf0a`, `cf4a`, `cf70` were not
  read here).
- `N3Teleport`'s fields between the playfield identities (server id, group, the `9c50` identity) beyond what the captures
  show.
