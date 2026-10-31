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
| 1 | `4d450114` SpellList | a spell list run on a character: NPC AI event loops, duel buffs, environment effects ("Effect of Inferno Environment"), item effects | 3858 / 171 / 99 | byte; list (`x3f1`) of whole spells (`SpellData_t`, big-endian: type, instance, version, criteria, values); caster identity; target identity; byte (ending); short-length string (the list's name); byte flag (then the source nano/item identity, `cf1b`); byte; int | `SpellListIIR_t`: read FUN_100aabcb (spells FUN_100a71ae), apply FUN_100aae0e → `Beholder_t` +0x30 FUN_100026d0 → FUN_100a5f4b per spell | **ported** (`spelllist.rs`): effects of 0xcf26/0xcfd4 spells drawn on the character with their sounds. Most lists (0xcf3b "Cast nano") are empty on the client. Still to do: see section 1 |
| 2 | `43197d22` N3Teleport | the server moves a character: before every zoning, and in a generated mission (identity type `0xc7a0`) a move inside the playfield with no playfield message | 111 / 16 / 16 (own: 24 / 9 / 14) | byte; destination (3 floats); rotation (4 floats); byte (`0x61`); playfield identity (type `c79c` plain, `c79e` instanced, `c79f` generated, `c7a0` the same playfield); server id; group id; `9c50` identity; 2 ints; a second playfield identity; a tail length byte-int (8, 0x0c or 0x10) and that many bytes (`3f000000 0`, or the destination again) | N3.dll `n3TeleportIIR_t::Activate` (vtable 0x1003e68c) | own, `c7a0`: put the character at the destination with the rotation at once (today only the "server put us back" check catches it, without the heading); own, another playfield: start the loading fade then; others: drop the path, they vanish (a `Despawn` follows) |
| 3 | `CharacterAction` 0xdf / 0xe0 / 0xe1 / 0xe3 / 0xe4 (with the client's 0xdc / 0xdd / 0xde) | the tradeskill window: source and target accepted, "not a valid combination", the skill requirements, the result's template | 6 / 12 / 2 / 6 / 4 (16:55) | 0xdf, 0xe0: parameter 2 a slot or count; 0xe3: parameter 1 the skill, parameter 2 the value needed (`a0`/`a5`: 0x19, 0x3c); 0xe4: target `(1, template)`, parameter 1/2 `(1, template)` | FUN_1005d61f cases 92..97 (0x1005f1a1, 0x1005f1b8, 0x1005f1cf, 0x1005f1f7, 0x1005f212) | a Tradeskill window (`TradeskillView`): the client sends 0xdc/0xdd with the dragged items and 0xde to build; show the requirement lines from 0xe3 and the result from 0xe4. Today items are only combined by dropping one on the other (0x51) |
| 4 | `194e4f76` AddPet, `58742a0f` RemovePet, `0d381f02` PetToMaster | the character's pets: which creatures are one's own, and their reports | 0 / 3 / 28, 0 / 1 / 2, 0 / 0 / 4 | AddPet / RemovePet: byte, the pet's identity. PetToMaster: byte, the pet's identity, a command (1: with a value, 2: without), the value (`0x0a`), an identity | `AddPetIIR_c` read FUN_10071972, apply FUN_100719a7 (FUN_1005299c adds the pet); `RemovePetIIR_c` FUN_10076dc0 / FUN_10076df5; `PetToMasterIIR_c` FUN_10076917 / FUN_1007699d (signal FUN_10011fa1) | a pet list in `World` (add/remove by these); the Pets window and friendly colouring of one's pets; `/pet` already sends commands |
| 5 | Research: client `3115534d` (ResearchRequest), `253d0240` ResearchUpdate, `435f7023` "PerkUpdate", client `SetStat` 263 / 264 / 265 | the Research window: the request when it opens (always answered by ResearchUpdate), the global research lines, the personal research progress, and the window's sliders and chosen goal | client 2 / 0 / 1; 24 / 10 / 15; 156 / 4 / 0; client `SetStat` 20 / 0 / 16 | ResearchRequest: one byte 0. ResearchUpdate: byte, byte, 54 lines of (id, 3 ints) (all zero in the captures), int. "PerkUpdate": byte, research id, the goal (stat 265), the experience still needed (0xb93b, 0xb593, 0xab6f: falling). `SetStat` 263 / 264: 0..100 (the slider percentages), 265: the goal (0x988, 0x989, 0x992) | `ResearchUpdateIIR` read FUN_1003a7a0, apply FUN_1003a81c (signal +0x244); `PerkUpdateIIR` read FUN_100624bf, apply FUN_10063e88 (sets stat 0x109, the progress FUN_10052d86, "got_tech" when done) | a Research window: send `3115534d` on opening, `SetStat` for the sliders and the goal; fill the progress bar from `435f7023` |
| 6 | `365a5071` DoorFullUpdate | the doors of a generated mission, with their places | 108 / 0 / 93 | a `SimpleItemFullUpdate` (byte, version 0x0b, owner 0/0, position, rotation, playfield, the `f424f` state machine, a short, stats `x3f1`: stat 0 `c0081483`, 23, 701-703, 412, 252, 192, 193, 195, 259) then the lockable/door part (int 0, int 2, int 0x32, an empty `x3f1` list, int 2, two shorts) | `DoorFullUpdateIIR_t` read FUN_1009ff81, apply FUN_1009fff9 (creates the door dynel, opens or closes it by flag 0x80) | **ported**: the door is a usable object at its place. Still to do: draw it (its template, stat 23) and its open state |
| 7 | `CharacterAction` 0xcc UploadNano | a nano crystal used: the nano is learnt | 5 / 0 / 0 | target own; parameter 1 `cf1b`, parameter 2 the nano | case 86 (0x1005ed2d): FUN_10050100 on the nano holder (+0x1c0), signal 6 | **ported**: the nano joins the Nano Programs list |
| 8 | `CharacterAction` 0x89, 0x92 | sent on every move start: cancel the nano being cast (0x89) and the item being used (0x92) | 36 / 40 / 299, 36 / 45 / 300 | no parameters (0x92 can name a container) | case 43 (0x1005d78f): FUN_1004f7b8 empties the casting list (+0x1c0), each cast's timer and its `cf1b` signal; case 48 (0x1005e33a): FUN_1004949c ends the use timer (+0x198) and closes the container it names (0x6b: a bag) | end the casting bar / use progress in the client when they come |
| 9 | `55220726` PlayfieldAllTowers, `59210126` PlayfieldAllCities | the Notum Wars towers of the playfield, and its organization cities | 21 / 10 / 14 each | Towers: byte, `x3f1` count, then per tower: (`c354`, field id), (`c350`, the tower's id), position (3 floats), low and high template, an int, a scale float, a side int (Andromeda: 0x1a1ca/0x3f1 - 1 = 105 towers). Cities: byte, a short length (0 in the captures), raw house data | `PlayfieldAllTowersIIR_t` read FUN_1012aeab, apply FUN_1012ae74; `PlayfieldAllCitiesIIR_t` read FUN_101304f8, apply FUN_101304d5 (`UpdateNewHouses`) | tower markers on the planet map (`planetwindow.rs`) and the land control info; cities: nothing to do until a capture has one |
| 10 | Other characters' animations: `CharacterAction` 0x6b, 0x57, 0xa7, 0x83; `51492120` CharSecSpecAttack from the server | casting ends, standing up, a stance flag, a weapon equipped, a special attack performed (by others) | 0x6b 19 / 38 / 35; 0x57 168 / 27 / 1; 0xa7 6 / 12 / 10; 0x83 6 / 0 / 0; 112 / 26 / 13 | 0x6b: parameters `(1, nano)`; 0xa7: parameter 2 0 or 1; 0x83: target the weapon `(c74a, id)`, parameter 2 6; CharSecSpecAttack: byte, target identity, the special's stat (0x94, 0x96) | case 29 (0x1005de04, FUN_1004f542); case 22 (0x1005dc76: stance stat 0x1ae picks animation 0x29/0x2a/0x25); case 57 (0x1005e919: flag 0x800 on the stats at +0xe8); case 41 (0x1005dcfe) | stop the "spell" animation on 0x6b instead of by time, play the stand-up, play the special attack's animation on its attacker |
| 11 | `CharacterAction` 0x50 QueuePerk | a perk action queued | 0 / 44 / 0 | parameter 1 2, parameter 2 the delay (0x64, 0xc8: hundredths) | case 19 (0x1005f12e) | show the queued perk on its Action window button |
| 12 | `CharacterAction` 0x9b, 0xa2 | sneaking: one 0x9b per creature that sees the character, then 0xa2 "started sneaking" | 17 / 0 / 0, 2 / 0 / 0 | 0x9b: target the creature | cases 51 (0x1005e361), 55 (0x1005e54d) | the sneak state (the original's sneak bar/stance) after `/sneak` (0xa3, sent today) |
| 13 | `2a293d0f` Fov | a creature's field of view (a generated mission's guards) | 59 / 0 / 0 | byte; 2 floats (0.384, pi: the cone); byte; 5 floats; int (0x14) | `FovIIR_c` read FUN_10073ba2, apply FUN_10073c32 (sets the creature's view, FUN_100524f4, and stat 533 NPCFovStatus = 1) | only matters for sneaking past guards; draw nothing until sneaking is ported |
| 14 | `3129233b` AreaFormula | an area effect landing on a list of characters | 4 / 9 / 0 | byte; `x3f1` list of character identities; the formula (an int: 0x3e380, 0x18762); the source identity | `AreaFormulaIIR_t` read FUN_100721dc, apply FUN_1007212d → FUN_100a9a99 (FUN_10051c98 on each character) | play the formula's effect on each listed character (as `CastNanoSpell`'s landing glow) |
| 15 | `485e7202` InventoryUpdated | the server says a container changed | 2 / 0 / 0 | byte; an int (5) | `InventoryUpdatedIIR_t` read FUN_1007535f, apply FUN_10075392 (on the own character: signal 5, FUN_1001143e) | refresh the inventory windows (they already follow the item messages) |
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
hit and no delay at once through FUN_100a5f4b (the spell types' jump table at 0x100a6cb6) on the character (target stat 0x20
other than 3, 0xe, 0x17).

All 4128 captured messages read whole. By spell type: 0xcf3b "Cast nano" 2059 (the NPC event loops, "Arete Landing",
"Effect of Inferno Environment", the AI lists), 0xcf82 + 0xcf56 1436 (the duel buffs), 0xcf26 573, 0xcf0a 81, 0xcf22 36,
others under 12. What the client does with them (FUN_100a5f4b):

- **0xcf3b**: FUN_100a7814 is empty: the commonest lists show nothing on the client (the server casts the nano itself).
- **0xcf26** (FUN_100a55d9): the effect of stat 39 made on the character (`CreateEffect2`), for stat 49 hundredths when set.
  Ported: the 16:55 capture's Malfunctioning Cleaning Robots at Arete Landing get 0xa8e3 (a flare, the fire seen burning on
  them in the screenshots) and 0xa871 (a star burst at the pelvis); also 0xa9a1 and 0xa8e0 once each. A flare burns where the
  character stood (it does not follow it); other effects run on the character with its programs' effects (`BuffsFX`).
- **0xcfd4** (FUN_100a9159): the same effect, ported without its colours, scale and repetitions; the only captured ones
  (Shadowland Anvian smoke and sparks) carry criteria, which are not evaluated (FUN_100065d8), so they are left out.
- **0xcf0a, 0xcf56, 0xcf4a, 0xcf70, 0xcf1b, 0xd003**: nothing on the client.

Still to do: an ending list (+0x24 set) deletes the effect whose handle the spell keeps in stat 0x42, which a spell fresh
from the message never has (none ends an effect in the captures); the queued runs (several hits or a delay, FUN_10002595);
the criteria; 0xcf82 "Stun" (FUN_100a879c: state flags 6 / 0x400, the duel buffs' begin and end), 0xcf22 (FUN_100a4f8a: a
stat set, 689 on pets, 359 `MonsterData` on two NPCs), 0xcf92 (FUN_100a8107: a vehicle's animations), 0xcf4c, 0xcf44,
0xcf7d (feedback lines), 0xcf35.

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

Port: in `World::add`, own teleport: a pending `(position, rotation, playfield)` the play loop applies at once (and raises
the loading fade when the playfield differs); others: clear their path.

### 3. The tradeskill window (`CharacterAction` 0xdc..0xe4)

The 16:55 capture used the original's Tradeskill window: the client sends 0xdc (source changed) and 0xdd (target changed)
with the items and 0xde (build); the server answers 0xdf / 0xe0 (accepted, parameter 2 a slot or count: 3, 0x154, 0x4ff,
1), 0xe1 (not a valid combination), 0xe3 (a requirement: skill 0xa0 / 0xa5 and the value, 0x19 / 0x3c) and 0xe4 (the
result, `(1, 0x26ad0)` twice). The client combines by dropping one item on another (0x51, as the 15:05 capture's pattern
pieces did), which the server takes too; what is missing is the window with its requirement lines and the predicted
result.

### 4. Pets (`AddPet`, `RemovePet`, `PetToMaster`)

`AddPet` names a pet after each zoning (the 08:42 capture's `0x7b1de89e`; the 15:05 capture's two, `0x7b1e6d64/65`);
`RemovePet` when one goes; `PetToMaster` reports (command 1 with 0x0a, command 2). The client has `/pet` but no list of
its pets: the Pets window (GUI.dll) and colouring them as friends need it. Port: a `pets: Vec<u32>` in `World`.

### 5. Research

The research window's traffic: the client sends `3115534d` (one byte, 0) when the window opens, and the server answers with
`ResearchUpdate` (the 54 global research lines, all zero for these characters); the sliders are `SetStat` of stat 263 and
264 (0..100, the schema names them `PersonalResearchLevel` / `GlobalResearchLevel`) and the chosen goal stat 265
(`PersonalResearchGoal`: 0x988, 0x989, 0x992). aosharp's "PerkUpdate" (`435f7023`) is the personal research progress:
`PerkUpdateIIR`'s apply (FUN_10063e88) sets stat 0x109 (265) and the progress, and sets the "got_tech" value when it is
complete. Port: a Research window (`docs/GUI_DLL_PARITY.md`); the stats are already kept.

### 6. DoorFullUpdate (`365a5071`) — ported

Only in generated missions (the ship `0x20da1b` and `0x20e000`). Each door: identity `(0xc748, 0x10c32dc1..)`, a
`SimpleItemFullUpdate` body (position, rotation, playfield, stats: stat 0 `0xc0081483`/`0xc0081403`, `StaticInstance` 23
`0x3de4d`, the ACG stats 701-703, 412, 252, 192/193 the external playfield and door, 195 the access key, 259 the door
flags), then the lockable part. Ported: the door is a usable object at its place, named "Door", with the door type, so it
can be targeted and used (test `a_generated_missions_doors_are_usable`). Not ported: drawing it from its template.

### 7. UploadNano (`CharacterAction` 0xcc) — ported

Using a nano crystal: the server sends 0xcc with `(0xcf1b, nano)`, then `TemplateAction` and `DeleteItem` (0x70) for the
crystal. Ported: the nano joins `program_ids` (test `an_uploaded_nano_joins_the_programs`).

### 8. Cancel on moving (`CharacterAction` 0x89, 0x92)

After nearly every client move that starts walking the server sends 0x89 then 0x92 (299 / 300 in the 15:05 capture). In
Gamecode, 0x89 empties the character's list of nano casts under way (each cast's timer stopped and a `cf1b` signal sent:
the casting is interrupted) and 0x92 ends the item use under way and closes the container it names. Port: clear the
casting bar and any use progress when they come.

### 9. PlayfieldAllTowers / PlayfieldAllCities

Sent at every playfield entry. Towers only on Notum Wars playfields (Andromeda 655: 105 towers, the Wailing Wastes 551),
each with its field, id, place, template, scale and side; the planet map's land-control view would show them. Cities were
always empty (length 0).

### 10. Others' animations

`CharacterAction` 0x6b (FinishNanoCasting, `(1, nano)`), 0x57 (StandUp, sent for others and echoed for oneself), 0xa7
(a stance flag, 0x800, on or off), 0x83 (a weapon equipped, its identity), and the server's `CharSecSpecAttack` (the
attacker, the target and the special's stat) drive what other characters are seen doing. The client times the casting
animation itself and does not play stand-ups or special attacks of others.

### 11-15

- 0x50 QueuePerk (08:42): parameter 2 the delay; the perk is queued behind the attack.
- 0x9b / 0xa2: the sneak check (creatures that see the character, then "started sneaking").
- Fov: guards' sight cones in a generated mission (stat 533); for sneaking.
- AreaFormula: an area effect: the listed characters get the formula's effect from the source (FUN_10051c98).
- InventoryUpdated: on oneself and on a shop (`c75b`) after a sale, value 5: the inventory windows refresh.

### 16. The small and unknown `CharacterAction` codes

| code | count | seen | Gamecode case | reading |
|---|---|---|---|---|
| 0x64 | 936 / 6 / 4 | on NPCs, next to `SpellList` and `SetNanoDuration`; parameter 1 0x0d..0x2f | 26 (0x1005ddba: FUN_1003c68e stores it at +0x1c of the +0x1dc part) | an NPC state value (unknown) |
| 0x6e | 151 / 5 / 27 | right after the hazards `Action` (`2049527c`) on a corpse or chest that was used, and after the client's 0x8e | 32 (0x1005ddf4: FUN_10046e7a → FUN_1003f61c on the +0x1b8 part) | the end of an action's progress (the client's 0x8e starts one: `N3Msg_GetActionProgress` checks 0x8e) |
| 0x2f | 5 / 0 / 0 | target a terminal, weapon or item, parameter 2 0x71 or 1 | 12 (0x1005da2c) | unknown |
| 0x61 | 4 / 0 / 0 | parameter 2 6, before `ContainerAddItem` | 23 (0x1005dcd7) | unknown |
| 0x54 | 10 / 0 / 0 | after the client's 0x51 (an item used on an item), parameters 2, 2, then `Feedback` | 20 (0x1005dc56) | a combine refused |
| 0x15 | 0 / 0 / 1 | parameter 2 0x18, after an `InfoPacket` | 3 (0x1005d7cb → FUN_10065c04: signals for values 0x11..0x14, 0x18) | unknown |
| 0xab | 8 / 2 / 0 | at zoning: parameters `(0xf4254, 0x48396)` | 61 (0x1005e965) | unknown |
| 0xd0 | 0 / 3 / 0 | after a perk lock (0xcf): `(0x2f6, own)`, `(0x1b, 0xcfce)` | 90 (0x1005edfc) | a perk's effect (unknown) |
| 0x84 | 1 / 1 / 0 | SpecialUnavailable | 42 | a special unavailable |
| 0x81, 0x8a, 0x8c | 1, 1, 3 | target own or the pet, parameter 2 0x1e or 0xc8, after `HealthDamage` | 39, 44, 46 | unknown (healing?) |
| 0x93 | 1 | parameter 2 2 | 49 | unknown |
| 0x56 | 1 | on an NPC, no parameters | 21 | unknown |
| 0x1b, 0x23 | 1, 1 | team: 0x23 TeamRequestResponse (who, `dea9` team) | 6, 9 | the team's answer (the client handles 0x1a/0x20) |
| 0x7a | 0 / 0 / 1 | StopLogout | 37 | the logout was cancelled: the logout bar should go |

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
