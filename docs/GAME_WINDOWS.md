# The game's windows: what is understood

Where the data for inventory, NCU / nano, missions and NPC conversations comes
from, as far as the proxy capture of a real session (`capture-180803.log`, read
with `mao-client net msg-stats | msg-dump | inventory`) and the game database
show. Everything marked **done** is in the client; the rest is a map of what to
decode next.

## Where the original's windows live
The GUI folder (`cd_image/gui/Default`) holds only 27 small XML views (login,
character selection, mail, trade, research, raid, skills ...). The inventory,
equipment, NCU, nano and chat windows are **built in code** (GUI.dll), so there
is no layout to copy: this client designs them (egui) and fills them from the
server's messages and the database.

## Inventory / equipment (done)
The server sends the character's items once, in the world-entry message
`29304349` (about 4 KB; the same kind arrives for other characters in view).
After 9 header bytes come 32-byte big-endian entries until the stat tables start
(`mao_net::inventory`):

| field | meaning |
|---|---|
| u32 slot | placement: 3-5 utility, 6 right hand, 7 belt, 9-13 NCU units, 17-32 armour and clothing, 33-45 implants, 46-63 social, 0x40+ backpack |
| u16 flags, u16 count | item / bag flags (0x01 0x21 0x81 0xa1 item; 0x02 0x22 bag); stack size |
| u32 type, u32 id | a bag's identity (type 0xc749); else 0 |
| u32 low id, u32 high id | the item templates the stack spans (type 0xf4254) |
| u32 quality, u32 extra | QL |

The slot names were read off the capture (the belt holds the NCU units, an
"Eye implant" sits in 33 ...); slots 1, 2, 8, 29 and 46+ are guesses.

An **item template** (type `0xf4254`, 99.96 % parse): LE words `[type, id,
1000000, 0xc74e, (stat, value) pairs ..., (name_len | desc_len << 16)]` then the
name and description. Stat 79 is the icon (a 48x48 texture, type `0xf6958`),
stat 12 the item's mesh. `mao-db item <id>` / `item scan`.

The window (key `I`, rebindable `inventory`) shows equipment sections, the
backpack, icons, stack counts and tooltips. Not yet: bag contents (a bag's items
arrive in other messages), drag and drop, equip/unequip (the move messages),
live updates (`4e536976` carries item adds with the same entry layout).

## NPC conversations (decoded, not shown yet)
`5d70532a`: a line the NPC says: speaker identity (type 0xc350, id), two words,
a length, the text. `55704d31`: the reply options: speaker identity, a count,
then length-prefixed strings. The reply the player picks goes back as a client
message (not yet captured). A conversation window is straightforward once the
reply message is known.

## NCU and nano (partly mapped)
* The NCU **units** are items in slots 9-13 of the belt (shown in the inventory).
* The nano **programs** known and the active **effects** (the NCU bar) are not
  in the captured traffic: that session never opened the windows. The cast
  message `4d450114` carries the program's name ("Summon Rage Materialization")
  and its identities (nano id type `0xcf1b`, per the AOReloaded notes).
* The big stat tables at the end of `29304349` (after the items) hold the
  character's skills; decoding them gives the skills window.

To map the rest: capture a session that opens the NCU / nano window, casts a
nano and cancels one, and run `net msg-stats` to see which kinds appear.

## Missions (not started)
Missions are server-side: the mission terminal offers a list, accepting adds an
entry, and the details (text, rewards, destination) arrive as messages not in
the capture. The conversation text of mission NPCs uses the same `5d70532a` /
`55704d31` pair. Needs a capture that uses a mission terminal.

## Capturing more
`tools/capture_proxy.py` records a real client's session (the original client
through a local proxy). Doing one pass in the original with the windows opened
(NCU, nano programs, a mission terminal, a tell and a shout) and then running
`msg-stats` on it is the fastest way to find the remaining messages.

## Second capture (the debug session with screenshots)
`capture-180803.log` grew with a session that used terminals and cast nanos. Screenshots of the original
windows gave the exact **Wear** layout (four tabs of three columns: Weap, Cloth, Impl, Social; the belt
"Deck" holds Deck 1-6; the social tab starts at placement 49), now used by the inventory window.
New messages, as far as they are understood:

| message | direction | what it is |
|---|---|---|
| `5e477770` action `0x13` | client | **cast a nano**: data = target identity (c350, id) then the nano identity (0xcf1b, nano id) |
| `5e477770` action `0x41` | client | a nano-related action with only a nano identity (0xcf1b, id): probably learn / remove (unconfirmed) |
| `5e477770` action `0x07`, `0x86` | client | seen once each, unknown |
| `4d450114` | server | a nano effect: nano id (0xaad6 = "Summon Rage Materialization"), source and target identities, the name as a length-prefixed string. The NCU window's list is built from these |
| `36284f6e` | both | **using a terminal**: the server answers a `52526858` use on a terminal (identity type 0xc75b, id = serial << 16 | playfield) with this, naming the terminal and the user; the client's `36284f6e` with `02 02` ... answers it (a choice) |
| `371d0542` | server | a **whompah destination list**: entries of (u32 id, u16 name length, name, 2 flag bytes) e.g. "Styx Magma", "Charred Groove", "North of Yuttos", "West of Perdition", "Dust Bank" ... what a booth offers when used |
| `58362220` | server | a list of item stacks (low id, high id, count) attached to a terminal identity: the contents of a mission or shop terminal (unconfirmed which) |
| `5f4b442a` | server | a **system chat line** ("Tarasque is no longer immortal."): u16 0, a length byte, the text, trailer |
| `333b2867` | both | a small request (`00 00 01 ...`) answered with `3f1`: unknown |

Next steps this enables: a **whompah destination window** (use a booth, show `371d0542`, send the choice back; also
fixes the booth labels), a **chat/system log** from `5f4b442a`, **nano casting** as a hotbar action (action `0x13`),
and the **NCU window** from `4d450114` (the original shows "NCU (used/capacity)" with the active programs' icons).
The destination choice message needs a second sample to be sure of its fields.
