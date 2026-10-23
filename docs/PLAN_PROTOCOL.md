# Plan: protocol from the AOSharp projects, and driving the original client

Written after reading the three repositories cloned beside this one:
`aosharp` (a plugin framework injected into the original client), `aosharp.clientless`
(a client without the game, in C#) and `aosharp.utils` (plugins that use them).

## What they contain that matters
* **`aosharp/AOSharp.Common/SmokeLounge/AOtomation/Messaging`**: the protocol as typed C# classes: 104
  N3 messages (`[AoContract(kind)]`, `[AoMember(n)]` fields, flags and arrays), the system messages,
  the chat-server messages, and the enums (`N3MessageType`, `CharacterActionType`, `GenericCmdAction`,
  `IdentityType`, `SystemMessageType`, `ChatPacketType` ...). These files carry the WTFPL header of the
  SmokeLounge project, so deriving this client's definitions from them is fine. **The three repositories
  have no licence file of their own**: copy no code from them (only protocol facts and the SmokeLounge
  definitions).
* **`aosharp.clientless`**: a working login / zone / chat-server flow (`NetworkSession`,
  `Chat/ChatClient`): the reference for the parts of the protocol this client skips.
* **`aosharp` (the injector)** with `AOSharp.Core`: reads and drives the original client from inside it
  (dynels, inventory, nano, missions, perks, packets in and out, chat commands).
* **`aosharp.utils`**: example plugins (mission helper, trade-skill leveler, bank bots).

## What this corrects in what the client already does
| was | is |
|---|---|
| action `0x78` = sit | `CharacterActionType.Logout`: it starts the logout countdown (sitting is part of it); `0x57` = StandUp cancels it; the server's `0x7a` is StopLogout. **The X key can log the character out after ~30 s**: rename it, warn, and show the countdown. |
| `371d0542` = a whompah destination list | `FightModeUpdate`: `(identity, [(id, name, flag, flag)])`: area / mode names of a playfield (Pandemonium's "Styx Magma" ...). Not destinations. Remove that part of the Whom-Pah window. |
| identity `0xc76a` = usable object | `Corpse`: lootable. `0xc75b` VendingMachine (a shop), `0xc73d` Terminal, `0xc748` Door, `0xc749` Container, `0xc74a` dropped weapon, **`0xc773` MailTerminal**, **`0xdac1` MissionTerminal**, `0xc767` TempBag, `0xcf1b` NanoProgram, `0x65-0x73` the equipment pages (`0x68` = inventory: the hotbar's item use is right). Label targets by these. |
| `36284f6e` = terminal handshake, `58362220` = terminal contents, `333b2867` = request | `Trade`, `ShopUpdate` (a vendor's stock), `Mail`. |
| `4d450114` = a nano effect | `SpellList` (the NCU's effect list); `CastNanoSpell` is `25314d6d`; `StopFight`, `AttackInfo`, `Buff` ... all named. |
| system `0x43` = the game server hands us over | `ChatServerInfo`: the **chat server** (`chat.d1.funcom.com:7105`). The original client opens that second connection after entering the world. Its failing is a candidate reason the original "stops on login" (the proxy never sees it). |
| the whompah has no client message | still true as far as the protocol shows: nothing in these sources names a whompah message; zone lines are server side. |

## Plan
### Phase 0: small fixes now (no new decoding)
1. X: logout countdown semantics, warning, cancel; `E` on corpses = loot (`GenericCmd Get`) once checked.
2. Name targets by identity type (corpse, mail terminal, mission terminal, shop, door).
3. Remove the "travel list" from the Whom-Pah window (keep the Go button); fix `GAME_WINDOWS.md`.

### Phase 1: generated protocol layer (the foundation)
A Python generator reads the SmokeLounge C# (`[AoContract]`, `[AoMember]`, `[AoFlags]/[AoUsesFlags]`,
array size kinds `X3F1` / `Int16` / `Int32`, nested classes, enums) and writes Rust modules into `mao-net`:
message structs with `parse` / `write`, the enums, and an `all_messages` table. Acceptance test: **every
message in `capture-180803.log` parses without leftover bytes** (the capture is real server output), which
checks the generator and the C# definitions against ground truth. Then `net msg-dump` prints named fields.
Rejected for now: porting by hand (104 messages, error prone), and linking the C# (no .NET in the client).

### Phase 2: the chat server (7105)
Port `Clientless.Chat`: salt / login / select character / character list, then vicinity, private, group
and channel messages, friends, ping. Gives the chat window the user asked for (tells, vicinity, org,
channels), and the system text (`ChatText`). Rate limits: one connection per session; no retries in a loop.

### Phase 3: windows from real, named messages
Inventory live updates (`InventoryUpdate`, `ContainerAddItem`, moves), shop (`ShopUpdate`), trade, **mail**,
**mission terminal** (`Quest*`, `CreateQuest`, `MissionInfo`), **NCU and nano** (`SpellList`, `Buff`,
`NanoEffect`, `CastNanoSpell`; the hotbar gets nano casting: `CharacterAction 0x13`), skills (`Stat`, the
tables in `FullCharacter`), perks, NPC dialogue (`KnuBot*`), loot. The skills / perk / bank windows follow.

### Phase 4: ground truth from the original client (optional, needs the user's go-ahead)
* Cheapest and safest: keep using the proxy, extended to relay the chat server too (rewrite
  `ChatServerInfo` to the loopback and listen on 7105), and read the captures with the Phase 1 parsers.
* Heavier: build `aosharp` (Visual Studio 18 is installed; the projects target .NET Framework 4.8 and only
  the 4.7.2 targeting pack is installed, so retarget or install the 4.8 pack) and write a small **recorder
  plugin** that logs the parsed packets and the client's own state (cooldowns, key bindings, window
  layout) to JSON, for diffing against this client. Injection into a live game is the kind of thing
  Funcom's rules treat strictly (automation): use it on the user's own account, passively, and only if
  the user wants it. AOReloaded (already in the game folder as `version.dll`) can already dump the
  client's 268 settings and window configs without an injector.
* `aosharp.clientless` as a test oracle: build it and run it against the test account to try
  hypotheses without a window (use a mission terminal, buy, mail). Same caution as above.

### Phase 5: faithfulness
Use the original's window layouts (from AOReloaded's DValue dump and screenshots) and its key bindings
(Phase 4 can read them) in the egui windows; shadows / water / UI polish continue in parallel.

## Order and effort
Phase 0 (an hour) -> Phase 1 (the main piece, a day of work, verified by the capture) -> Phase 2 -> Phase 3
window by window (mail and mission terminal first, because the user's capture already contains them) ->
Phase 4 only where a question remains.

## Questions for the user
1. OK to build the recorder / clientless tools, given the terms-of-service caution, or stay with the proxy?
2. Priority among mail, mission terminal, chat and NCU?
3. Should the original client's "stops on login" be looked at now (proxy the chat server and see)?

## Status (first cut of Phase 1 and the proxy)
* `tools/gen_protocol_schema.py` turns the SmokeLounge definitions into `crates/mao-net/schema/protocol.json`
  (233 classes, 78 enums, 94 N3 messages) and `mao_net::schema` interprets it: `Schema::global().parse_n3(kind, data)`
  gives named fields with enum members named.
* `mao-client net msg-check <capture.log>` parses a whole capture and reports per message kind how many parsed
  exactly: **4,598 of 5,078 (90.6 %) on the first capture**. `net msg-parse <log> <kind> [n]` prints messages as
  named fields.
* What still fails, and why: `SimpleCharFullUpdate` (a custom serializer in the C#; `EntityInfo` covers it by hand),
  `Mail` (the C# models only sending: the list request is `action = 1` and eight zero bytes, the reply an empty
  array; the items of a non-empty mailbox need a capture with mail in it), `SpellList` (the C# class is empty),
  a few trailing unknown fields (4 bytes on item / chest / corpse updates), and messages the C# names but doesn't
  define (`AddPet`, `Action`, `SetWantedDirection`).
* The capture proxy now also relays the **chat server**: it rewrites the system `0x43` ChatServerInfo (plain or
  compressed frames, the address kept the same length with NUL padding) and relays that connection untouched.
  To use it: stop the old proxies, run `python tools/capture_proxy.py`, start the original as before; the log then holds
  the chat login (chat packet types are in `SmokeLounge/.../ChatMessages`) and whatever makes the client stop.
