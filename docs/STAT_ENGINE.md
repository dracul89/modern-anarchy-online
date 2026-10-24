# How the original computes a character's stats (N3Msg_GetSkill), checked against the live client

Character: Conjecture (level 220). Read through ao-bridge with `N3Msg_GetSkill(identity, stat, detail, identity)`; the
decompiled code is in `AnarchyReimagined/exports_gamecode/10026ee2.json` / `100270a3.json` (Gamecode.dll).

## The detail argument
| detail | what | decompiled |
|---|---|---|
| 0 and 4 | the character's stored value (what the server sent in `Stats2` of the world-entry message) | stat container at `SimpleChar+0xe8`, virtual slot 0x3c with mode 2 |
| 1 | the same plus two small corrections (`FUN_10065008` mode 3 and a stat-scale factor `FUN_1006427f`) | `FUN_10065a27` |
| 2 and 3 | **what the windows show**: the stored value plus the modifier map's entry for the stat (`FUN_10064b51`: item and nano bonuses, with the extra attribute-trickle term for skills 100..168 and the six attributes) | `FUN_10065a92`, `FUN_10064d46` |
Health (0x1b) is read straight from the character object (`+0x218`).

## Checked values (detail 0 = server value, detail 2 = shown)
MaxHealth 8265 -> 35187; Strength 764 -> 1175; Agility 944 -> 1026; Stamina 680 -> 1128; Intelligence 764 -> 904;
Sense 912 -> 1001; Psychic 748 -> 873; MaxNanoEnergy 4421 -> 11371; Melee AC 0 -> 14516 (screenshot: 14516);
Projectile 0 -> 15106 (15106); Energy 0 -> 14446 (14446); Chemical 14266; Radiation 14516; Cold 14751; Poison 14376; Fire 14646.
All 383 non-zero stats are in `docs/stat_compare.json` (stat, name, detail 0, detail 2, the server's lists).

## What this means
* The server sends the **stored** value only; most skills sit at the default 5 and are not sent at all.
* Every armour class has stored value 0 and is sent nowhere: the **client builds it** from a per-stat modifier map.
* The 40 worn items' template stats (key/value pairs, `Item::stats`) hold **no** AC entries (stats 90-97 are all 0),
  so the bonuses come from the item's trailing tables (the item actions / spell lists: "modify stat on equip").
  Those tables are read by the rebuilt `GameData.dll` (`SpellData_t`, `spell_formats.rs`), which is the next place to
  look to rebuild the modifier map; the result can be checked against detail 2 for any stat.
* Not rebuilt (not among the drop-ins): `Gamecode.dll`, `N3.dll`, `GUI.dll`, `Interfaces.dll`.

## First rebuild step: what the worn items give

An item template's trailing tables hold its effects. The "modify a stat" effect (spell type 0xcf35) is ten words,
`0xcf35, 0, 4, 0, 1, 0, 2, 9, stat, amount`; `mao_formats::item::modifiers` reads them. Summing the 40 worn items
of the character gives (shown value in brackets): Melee AC 4049 (14516), Projectile 3569 (15106), Energy 3904 (14446),
Strength +197 (+411), Stamina +197 (+448), max health +4269 (+26922), max nano +1750 (+6950).

So the items explain roughly a quarter to a third of each bonus; the rest must come from the active nanos (buffs),
perks and the implants' own effects. Also seen: `AddAllDef` shows 1001 (an all-defence modifier, 0 stored), so some
bonuses are percentages or are applied to several stats at once. Next: the active nano list
(`N3Msg_GetNanoTemplateInfoList`) and the same effect tables of those nanos.

## Correction and second comparison (byte offsets, and the active nanos)

The effect records are **not word-aligned** in the template (they follow the text, whose length is arbitrary), so the
first comparison, which read whole words from the start, missed many records. `item::modifiers` now looks at every
byte offset. Active nanos are objects of type `0xfde85` (the list comes from `N3Msg_GetNanoTemplateInfoList`, 8 buffs
here, identity type 0xcf1b) with the same effect records.

Shown bonus (detail 2 minus detail 0) against the sum of the worn items' effects (40 items, none with a quality
range) plus the 8 active nanos:

| stat | shown | items | nanos | still missing |
|---|---|---|---|---|
| Melee AC | 14516 | 13339 | 0 | 1177 |
| Projectile AC | 15106 | 13499 | 0 | 1607 |
| Energy AC | 14446 | 13124 | 0 | 1322 |
| Chemical AC | 14266 | 13376 | 0 | 890 |
| Radiation AC | 14516 | 13626 | 0 | 890 |
| Cold AC | 14751 | 13159 | 415 | 1177 |
| Poison AC | 14376 | 13054 | 0 | 1322 |
| Fire AC | 14646 | 13324 | 0 | 1322 |
| Strength | 411 | 293 | 0 | 118 |
| Max nano | 6950 | 6450 | 0 | 500 |
| Max health | 26922 | 13054 | 510 | 13358 |

The items account for about 92% of each armour class. What is left (890 to 1607) is not one constant: it is not just
`AddAllDef` (shown 1001, items 869). Candidates: the perks' standing effects, effects of other spell types or with
requirements that this reader skips, the item "armour class" attribute scaling, and the modifier map's own caps. Max
health is far off (the shown value also includes a body-development and level formula, which the attribute trickle
term in `FUN_10064b51` for the six attributes feeds).

## More findings (third pass)

* **Records with requirements.** The effect record is `0xcf35, 0, 4, N, (requirement: stat, value, operator) x N, 1, 0, 2, 9,
  stat, amount`; `N = 0` is the form `item::modifiers` reads. Two worn items (281199) have `N = 1` with the requirement
  `stat 0x36 (Level) value 0xc8/0xc9`, giving NanoResist (0xa8) +500 / +200 at level >= 200. `modifiers` does not read
  these yet.
* **No other encoding of an armour class** exists in the 40 worn items or the 8 active nanos: a scan for stats 0x5a..0x61
  next to a stat marker finds only the records already counted. The active nanos add armour only through Ice Burn
  (cold +415).
* **Items do not carry armour classes as stats.** `N3Msg_GetSkill` with an inventory item's identity `(0x68, slot)`
  returns the sentinel 1234567890 ("no such stat on this item") for every worn slot.
* So the remaining 890-1607 per class is not in items or buffs. Not yet examined: the 35 perks' standing effects, and the
  helper object that `FUN_10064b51` reads (its two maps: a stat-to-node map and a stat-to-modifier map; the modifier
  is `map[stat] - trickle`), which can be read directly from the live game through the bridge.

## The modifier map, read from the live client

`GetSkill`'s detail 2/3 is `stored value + modifier map[stat] - trickle`. The map is a `std::map<int,int>` reached as
**`*(*(*(engine + 0x?`**... concretely: `char = n3EngineClientAnarchy_t::GetClientChar(engine)`, `H = *(char + 0x1bc)`,
`map = *(H + 4)`; the map object has the tree head at `+4` and the size at `+8`; nodes are `{left, parent, right, key
(+0xc), value (+0x10), colour (+0x14), isnil (+0x15)}` (MSVC 2010 layout). For Conjecture it holds **120 entries**
(`docs/modifier_map.json`).

For every armour class, attribute, max health and max nano, the map entry equals the shown bonus exactly (Melee AC 14516,
Strength 411, Max health 26922, ...); for skills the shown bonus is the map entry plus the attribute trickle-down.

How much of the map the worn items (40) and active nanos (8) explain:
* **30 stats exactly**: BeltSlots 6, SharpObject 289, Grenade 32, HeavyWeapons 234, Bow 231, the guns 32/90, ...
* **16 stats only in the map** (no item or nano effect found): Reflect*AC 3 each, ShieldNanoAC 80, ElectricalEngineering,
  QuantumFT and Chemistry 50 each, NanoPool 9, Scale 12.
* **74 stats partly**: the armour classes (89% to 93%), attributes (Strength 293 of 411, Stamina 247 of 448, ...), max
  health (13564 of 26922), the Shield*AC (70 of ~180), AddAllOff/AddAllDef and most skills. The missing part is always
  positive.
So some source other than the worn items and the 8 listed nanos adds to the map. The login message carries no seed values
for it (`Stats3`/`Stats4` hold only the state stats and the absorb values, all 0). Perks are the main suspect, as their
definitions are not in the game database (the names are not in the instance index), so they are the server's or the
client code's. Unread effect records (requirements other than level) may account for a little.

## The hook log: where every modifier comes from (live, after a zone change)

Hooks on `AddModifier` (`FUN_10064a0e`) and `RemoveModifier` (`FUN_10064be1`), logging the holder (`ecx`), the caller chain and
the effect object, for a whole rebuild after zoning. Findings (`docs/modifier_sources.json`, `docs/unexplained_modifiers.json`):

* The client keeps one stat holder per character it can see and re-applies other characters' buffs in bursts; filter by the
  holder of the controlled character (`*(char + 0x1bc)`).
* **On our holder, the net of the logged adds and removes equals the live modifier map for all 120 stats.** So the hooks see every
  source.
* The sources, by caller: items and the other spell lists all go through `FUN_100a517a` (return `0x100a51f4`), a spell executor; a
  second route (`0x1004b974` / `0x1004b937`, `FUN_1004b81e`) adds and removes the same 42 item effects as a pair and nets zero;
  a third route (`0x100a8db3`, `FUN_100a8d0b`) is a level-scaled spell.
* **Named nano buffs** carry their nano id in the spell data (`[effect + 0x20] + 0x10`): Ice Burn 269460, Composite Nano Expertise
  223380, Composite Melee Expertise 223360, Composite Physical Special Expertise 215264, Calling of Thor 202846, Vlad's Revenge
  202858, Improved Essence of Behemoth 273629 (the 8th buff's effects did not show).
* **All 567 item records** (`item::modifiers`) of the 40 worn items appear among the logged events: the item reader is complete.
* **702 more events** (509 non-zero) come from one further effect object with no nano id. They are not items. Their totals per stat
  are *exactly* the part the items and nanos did not explain: Projectile AC 1607, Melee 1177, Energy 1322, Chemical 890,
  Radiation 890, Cold 1177 (after Ice Burn's 415), Poison 1322, Fire 1322, MaxNano 500, AddAllDef 132, MaxHealth 11360. They arrive
  as ladders of rising amounts for the same stat (armour classes +40, +32, +68, +70, +70, +70, +108, +109, +110, +213 ...; health
  +50, +85, +115, +150, +210 ... together with +3 ... +11 heal delta; shield +2, +5, +6, +11, +20 ...), which is the shape of a
  rank-based table, such as personal research or a similar per-level reward list.
* Not yet identified: the data this ladder is read from (`N3Msg_PersonalResearchGoals` returns only the current goal, 10005; the
  completed research would come from the server's research update message). The next step is to read the list's owner and the
  spell data of those events, or to find what the server sends that selects which ranks apply.

## Research check (live)

Calling `N3Msg_GetCompletedPersonalResearchGoals` on the live client returns only 8 ids (2309, 2319, 2329, 2339, 2349, 2359,
2369, 10004). That is far too few to be the source of 702 events, so the ladders are **not** the completed personal research as such.
The executor object seen in the hook (`esi`) is the same for items and for the extras, so it only identifies the shared apply
routine (`FUN_100a5f4b`, called by the iterator `FUN_100026d0`), not the list. Remaining leads: the callers of `FUN_10080fc3`
(`FUN_1004bc13`, `FUN_100532f6`, `FUN_10052cfd`, `FUN_100533aa`, `FUN_100538f7`, `FUN_1004709c`), which hold the container the
iterator walks; and whether the research map (`FUN_10058ff8`, entries keyed by goal id with spell lists) holds a spell list per
rank that all apply up to the completed one (which would multiply 8 goals into hundreds of events).

## The callers of FUN_10080fc3 (read)

`FUN_10080fc3(char, 0, 0, 0)` is the "apply the standing effects of one perk" routine (it is where the unexplained events are added,
return `0x10081087`). It is called from:

* `FUN_100533aa` / `FUN_100532f6` / `FUN_10053241` (perk manager, `this+0xc` = the character): they walk the perk map and the
  per-type lists of owned perks (`FUN_1002be73(type, out)`, `type` = `GetSkill(0x21, 2)` = 1 for this character), look each perk
  up with `FUN_1002bdf5` and apply it if its vtable `+0x5c` check passes. `FUN_10052cfd` and `FUN_100538f7` (which carries the string
  `Feedback_PerkDoesntExist`) do the same for one perk when it is bought or switched.
* `FUN_1004bc13` (equip an item, wraps the call in a stat-`0x37` flag) and `FUN_1004709c` (loop over slots `0x20..0x2f`) apply the
  item side.

A perk's data is an item-type record: `FUN_1002bce7` looks it up as database type `0xf4254` by the id at `perk_record + 4`. So the
702 events are the **standing effects of the owned perks, read from item-type templates**, which the item reader (`item::modifiers`)
can already parse. What remains is the list of owned perks: `N3Msg_GetCompletedPersonalResearchGoals` is not it (8 goals). The perk
manager is the singleton from `FUN_1002c0d5()` (vector of 28-byte records at `+0x34..+0x38`, flags at `+0xc`, bit 7 = perk, owned
count at `+0x1c` of the entry from `FUN_1002be1a`); its records and `GetSkill(0x21, 2)` have not been dumped yet.

## The perk manager (dumped live) and the 702 events

The perk manager singleton is the pointer at `Gamecode.dll + 0x2e35b8` (image address `0x102e35b8`). Its vector at `+0x34..+0x38` holds
**2063 records of 28 bytes**: `perk id, item template id (type 0xf4254), previous rank's id, 0, flags, aux, cost`
(`docs/perk_records.json`). Ids 100-109, 110-113, ... are the ranks of one line (each rank's `previous` is the rank below).
The research goals are the same kind of record (ids 2300-2369 and 10002-10004, flags 64). The three maps at `+0, +0x10, +0x20`
list the 54 research definitions (3000-range, cost in the last field), not what the character owns.

Each template carries its effects in the `0xcf35, 0, 4, N, (stat, value, op)*N, 1, 0, 2, 9, stat, amount` form; for perks the
stat and amount are in the tail after the `1, 0, 2, 9` marker (`N` is 0). Parsed for all of them: `docs/perk_effects.json`
(1951 records with at least one effect; e.g. perk 100 is max health +15 and stat 221 +50, perk 109 is +45 and +280).

Matching against the 702 unexplained events:

* The completed research (the 8 goals from `GetCompletedPersonalResearchGoals`, plus every lower rank in their `previous` chains:
  2300-2369 and 10002-10004) yields 221 events, **every one inside the unexplained set** (no overshoot).
* Nine whole perk lines (170-179, 200-209, 270-279, 420-429, 510-513, 550-554, 1070-1079, 1150-1159, 1180-1189) also fit,
  and together with the research cover 651 of the 702 events (34 over, 51 left, which is the ambiguity of a multiset match:
  several lines share the same amounts). So the source is confirmed: the **rank ladders of research and perk lines**, read from
  the perk records' item templates, applied when the character has them.
* Still unknown is which perk ranks the server says the character owns (the research list is read from a map at the
  `FUN_10058ff8()` object, state flag bit 6 of the entry at `+0x1c`). The perk side needs the same read (bit 7 flag entries of
  `FUN_1002be1a` with count `+0x1c`).

## The owned perk list (read live)

The character's own perk and research state is an object at `*(char + 500)` (built lazily by `FUN_10058ff8`; `+0xc` is the
character, `+4` points at a `std::map` object whose head is at `+4` and size at `+8`). Its nodes are 0x34 bytes: key at `+0xc`,
`id` `+0x10`, template `+0x14`, previous rank `+0x18`, flags `+0x1c`, state `+0x2c` (0 = owned/applied; the current research goal
10005 has a non-zero state). The map had 144 nodes, **143 owned** (`docs/owned_perks.json`): perk lines 170-179, 200-209, 270-279,
420-429, 1070-1079, 1150-1159, 1180-1189 (ten ranks each) and research ranks 2300-2369 and 10002-10004.

Applying exactly these ranks' effects (`docs/perk_effects.json`) gives 651 events, **all inside the 702 unexplained events, none
over**. The 51 left are effects this parse does not read yet (other spell types or requirement-gated records). So the stat engine for
this character is: item effects + active nanos + the effects of every owned perk and research rank. The new client needs the same
owned list from the server (it is rebuilt from the world-entry message and the research update; not captured yet), then it can
sum these templates for the Stats window.

## The 51 leftover events (narrowed, not identified)

* All 651 modify-stat records in the owned templates parse. The 34 `0xcfbe` records in them are perk **actions** (cast a nano such as
  Bio Shield on use), not standing effects, so they add nothing.
* The leftover events are not in any perk or research template (only `(1, 230)` appears in unowned perk 344).
* 50 of the 51 come from one heap block of the live client (spell structs of 0x48 bytes, from about `0x8261ee8`, 218 events in
  that block, 168 of them the owned perks and research): one further list of about 50 modify-stat spells applied in the same batch
  (stat 1 +230, ACs 90-97, skills at 58/87/144, stats 226-234 at 9/14/22, 319 +3, 343 +30, 168 +500, 181 +15 ...).
* A byte search for the pairs `(90,717)`, `(91,287)`, `(96,432)` in every object of the 25 small database types
  (`0xf4241` ... `0xf4312`, including the perk records `0xf4264`) found nothing. So the list is either in an item-type
  (`0xf4254`, 120842 objects, not searched) template, or built by code at run time from the character's level and profession.

## The 51 leftover events: three symbiants

`mao-db <game> grep-pairs 0xf4254 <stat:amount>...` (new) lists the item templates that hold at least two of the given little-endian
stat/amount pairs; it scans all 120842 item templates in about two seconds. With the leftover pairs it found the family of "Xan"
symbiants (template ids 278918-279110, 0x44186-0x44206), and a greedy cover over them gives exactly:

* 278994 Xan Thigh Symbiant, Infantry Unit Alpha (21 events)
* 278995 Xan Feet Symbiant, Infantry Unit Alpha (16 events)
* 278988 Xan Left Hand Symbiant, Infantry Unit Alpha (13 events)

with none over. These are worn items that the earlier worn-item list (40 items, templates 206013 ... 302923) did not contain, so
the hook events looked unexplained. The item reader handles them unchanged.

So **50 of the 51 are explained**; one event is left, `(stat 168, +500)`, in no template of that symbiant range (probably another
worn item or implant that is also missing from the list). The complete stat model for this character is therefore: worn items
(including the symbiants) + active nanos + owned perk and research ranks, which accounts for all but one modifier.
Next for the new client: take the worn items from the server's equipment list (it must include the implant/symbiant slots)
rather than from a fixed list.

## Where the server sends the perk list

* **`FullCharacter` (`29304349`), field `Perks`** (`Perk[]`, 16 bytes each: `SkillId, -254, SkillId, 0`; in the client it is read into the
  perk map by `FUN_10053ce9`, "inconsistent PerkMap!" on a bad count). The captured entries of Conjecture (identity `c350,4bc80`,
  level 220, stored max health 8265, as in the live client) are exactly the first four perk lines of the live list: 170-179,
  200-209, 270-279, 420-429. The ids are the perk-record ids (`docs/perk_records.json`).
* **`ResearchUpdate` (`253d0240`, `ResearchUpdateMessage`)**: one byte, then research lines `(ResearchId, three ints)`, ids 3000-3218
  (the 54 research definitions); every captured copy (the level-8 character's sessions) has all-zero fields. The registered
  info items in the client are `PerkUpdateIIR` and `ResearchUpdateIIR`.
* **Not yet seen on the wire**: the three further perk lines the live Conjecture owns (1070, 1150, 1180) and the 73 research ranks
  (2300-2369, 10002-10004). No capture contains Conjecture's own world entry or a research update with progress, so either they
  come in `FullCharacter.Perks` of a later login (the captured one is from the character list), in a `ResearchUpdate` with
  non-zero fields, or the client derives the ranks from a research level. **One capture of Conjecture entering the world
  through the proxy would settle it.**
* New tool: `mao-client n3 <capture.log> <type hex>` prints every server message of one type in a capture, parsed with the schema
  (`net::capture_n3`).

## Active nano buffs

The server lists a character's active nanos in its own `SimpleCharFullUpdate` (`271b3a6b`): a count word `(n + 1) * 0x3f1`, then per nano
`(type 0xcf1b, nano id, a word, two times)`; the schema parser cannot read the message (custom layout), so
`Message::active_nanos` finds the list by that shape (tested on a captured player: 15 nanos, e.g. Retool NCU 163005, Nano Boost
81885). The nano's effects are the same modify-stat records as items' (database type `0xfde85`), but a buff's record has the
third word of its tail (`1, 0, x, 9`) 2 **or 3** (3 is the most common in nanos: 12549 against 3493 and 2056 for 1); all 51 effects
of the 8 buffs read from the live Conjecture matched with either (`item::nano_modifiers_at`). `mao-db <game> tail-kinds <type>`
counts the kinds. The client keeps the list in `World.nanos` and works the bonuses out again when it, the worn items or the perks
change (`World::refresh_mods`).

Live check (Brasscasings, level 25, 4 composite-expertise/attribute nanos): the nanos add skills and attributes (strength +12
...) but no health or nano energy, so the maxima stay at 734 and 360 against a current 1085 and 365. The 351 health and 5 nano
that are still missing are not in the worn items' modify-stat records (none are skipped for requirements) nor in the nanos; the
vitals bars keep counting a current value above the computed maximum as the maximum.

## The missing health: the belt and its NCU units (found by checking the live character)

The 351 health and 5 nano that were short for Brasscasings were not another effect type: the filter of worn places left out the
**belt (slot 7) and the NCU units on it (slots 9-14)**, which add health and nano like any worn item (belt "Dream Mesh Circuit"
+100 health, +12 stat 181, +5 stat 45; each "Dark Dreams" unit +50 health, +20 stat 181). With them, and with the interpolation
between an item's low and high template rounded to the nearest whole number (it was truncated), the computed maxima come out
**exactly equal to the server's current values: 1085 / 1085 health and 365 / 365 nano**. The other spell types seen in worn items
(`0xcf2b`, `0xcf2d`, `0xcf2f`, `0xcf48`, `0xcf8a`, `0xcf34`, `0xcf49`, `0xcf61`) are not stat modifiers (conditions, damage, text, the
clan-merit machinery), so nothing more needs reading for stat bonuses so far.

## The computed stats against the original, stat by stat (live, Conjecture)

`mao-client <game> stats-check <capture.log> <character instance hex> <inputs.txt> <compare.txt> [level]` works out
`stored + modifiers` with the client's own code (`derived::modifiers`) and lists every stat that differs from what the original
shows. Inputs read from the running original in one pass (`docs/live_inputs_conjecture.txt`: the 80 inventory entries from
`N3Msg_GetInventoryVec`, the 9 active nanos from `N3Msg_GetNanoTemplateInfoList`, the 143 owned perks and research ranks) and every
stat's stored (detail 0) and shown (detail 2) value from `N3Msg_GetSkill` (`docs/live_compare_conjecture.txt`), level 220:

* **190 of 270 stats equal**, including **all eight armour classes**, attack rating's inputs, nano energy and most skills' map parts.
* Reading the modifier map itself (`*(*(char + 0x1bc) + 4)`, 119 entries) shows what the 80 differences are:
  * **Skills 100-168: the shown value is `stored + map + trickle-down`**, with a trickle of about 220-290 per skill from the
    attributes (`FUN_10062a6c`, weights per skill not yet read); the map parts of skills match except 14 small ones.
    This is the missing piece for the skills window.
  * **11 non-skill stats differ in the map itself**: max health +250, strength +29, stamina and agility +9, intelligence and sense
    +15 and +15, psychic +9 (stats 16-21), 277 +50, 343 +19, 364 +39, 318 -6. They look like one more small buff or a few more
    ranks that appear in no worn item, nano or perk template read so far (the pairs are in no single template).
* **Nano buffs** need the tail kinds 1, 2 and 3 (Mongo's Kraken, +20000 health, uses 1: without it max health was 20250 short;
  with it 250 short).

## The attribute trickle-down (modelled)

The shown value of a skill is `stored + map + trickle` where (`FUN_10065a27` / `FUN_10065008`, mode 3) the trickle is
`0.25 * sum(weight_i * attribute_i) / 100` over the six shown attributes (strength, agility, stamina, intelligence, sense,
psychic; shown = stored + map), cut to a whole number; the weights per skill are a table the client holds in a global map
(`FUN_100c5a7e`, 69 skills; most rows sum to 100). `stat 0xf7` (247, read by `FUN_10064b51`) is 0 and adds nothing. With the
original's shown attributes this reproduces **all 69 skills** of the live character exactly (truncation; rounding fails for 26).
The table is read from the running client and embedded as `crates/mao-client/src/trickle_data.rs`; `derived::shown` adds the
trickle to those skills (an untrained skill, absent from the stored stats, starts at 5), `derived::trickle` is the formula.

With the missing small buff of the previous section still unknown, the computed attributes are 9 to 29 short and the skills 2
to 6 short (the 14 skills that were off in the map are the same effect); `stats-check` on Conjecture is 190 equal / 80 off by a
few where it was 189 / 81 off by about 250.

## Hunting the last small buff (hook log of the live Conjecture)

The ao-bridge hooks on `AddModifier`/`RemoveModifier` were still installed in the running original (15236 events logged by now, the
holder of the controlled character changed to `0x7f30bf8` after a zone change). The net of the logged adds and removes on the
current holder equals the live modifier map for **every** stat, so the log has every source; subtracting what the client's model
produces (worn items, 9 nanos, 143 perks and research ranks) leaves 28 events:

* **3 events came from the ordinary item route: a Dreadloch Endurance Booster in inventory place 15** (health +250, strength +20,
  stat 277 +50). The worn places now run 1-16 (place 15 was left out): max health and stat 277 now match the original.
* **25 events come from a different route**: a spell of type `0xcfb7` (`FUN_100a8d0b`, a level-scaled modify-stat spell) run from
  a list that the server's N3 traffic delivers (return chain `0x61a06a3d` <- `0x61962bee` <- `0x61a87175` <- `N3.dll + 0x6643`),
  logged just before the worn items are applied. It is in no template of the character (no worn item, nano or perk holds `0xcfb7`,
  and no database object holds more than 6 of its 24 stat/amount pairs). For Conjecture (level 220) it adds attributes +9 (intelligence
  and sense +6 or +9), skills 122-131 +39, 139/152/161 +59, 156 +99, 123/124 +17, 149 +12, 132 +9, 343 +19, 364 +39 and 318 -6.
  The amounts look like fractions of the level (99 = 0.45 x 220, 39 = 0.18 x 220), so they are probably level-scaled bonuses of an
  info-item (`n3InfoItemRemote_t`) rather than data; they would need a capture or a second character of another level to model.

`stats-check` on Conjecture is now 192 stats equal; the remaining differences are these 25 effects and what they pull through the
attributes into the skills (`docs/live_check_conjecture.txt`).

## The `0xcfb7` handler (`FUN_100a8d0b`, read from the disassembly)

Spell type `0xcfb7` (53175) is a level-scaled modify-stat spell. Its records have the same shape as `0xcf35`'s (`0xcfb7, 0, 4, 0,
1, 0, 2|3, 9, stat, percent`). The handler, in `Gamecode.dll`:

1. takes the target character (`SimpleChar`), reads its **level** (`GetSkill(0x36, 2)`);
2. `amount = round(level * param / 200.0)`, where `param` is the spell's second value (spell stat `0x27`; the divisor is the double
   at `0x10160398` = 200.0, so 100 means "half a point per level"); the stat is the spell's first value (spell stat 0);
3. adds the amount to the stat's modifier through `AddModifier` (spell flag bit 0 clear), or removes it (`RemoveModifier`) when the
   spell is being ended; it also keeps the stat's current value consistent (a `GetSkill` before and after, passed to the info
   signal) and returns the change.

(`mao-db <game> scaled <type> <level>` lists, per object, the `stat:amount` of every `0xcfb7` record at a level; there are 169
item templates and 42 nano templates with such records.)

The 25 unexplained events of the live Conjecture fit that formula: with level 220 an amount is `1.1 x` the spell's value, i.e. the
values behind them are 8 (attributes 16-21 give 9), 5, 35 (skills 122-131 give 39), 15, 54, 90, 11, 17 and -5 (318 gives -6). The
social-tab "Enhanced Dustbrigade Notum Gloves" (place 56) hold exactly the 35 for skills 122-131 and -5 for stat 318 as plain
modify-stat records, which hints that the 25 are the effects of **items in the social tab** applied in the scaled form, but the other
social-tab items hold other stats (the 8 and 54 and 90 are in no social item), and no template of the character holds a `0xcfb7`
record, so the spell is built by the client at run time (the call chain ends in an N3 message handler) and its source list is
still not identified. Modelling it needs the list of (stat, value) pairs; the formula is known.

## The capture of Conjecture entering the world (decoded)

Capture `capture-225436.log`, connection `c41` (Conjecture, identity `c350:4bc80`, level 220): 3140 N3 messages in the whole file; `mao-client
n3 <log> 0` lists every message with its name and sender, `n3 <log> <kind>` one kind.

* **`FullCharacter.Perks` holds the whole list: 144 entries** (`SkillId, -254, SkillId, Unknown3`), the 143 perk and research ranks
  that the live client owned (`docs/owned_perks.json`, the same set exactly: perk lines 170-429, the alien lines 1070/1150/1180 and
  research 2300-2369, 10002-10004) plus research goal 10005, still in progress: its `Unknown3` is a time (34939491), all the others'
  is 0. The "40 entries" seen before was only the schema pretty-printer cutting a list at 40 (`... N more`); the client's parsed
  value always had all of them. So **nothing has to be inferred**: the stats use that list minus the entries with a non-zero
  `Unknown3` (`perk_ids` in `net.rs`), which is exactly the 143 (`docs/conjecture_world_entry_perks.json` has the 144 ids).
* `ResearchUpdate` (`253d0240`, sent 13 times with growing contents): all lines zero. The research state is the perks list's research ranks.
* **The owned perk actions** arrive as 35 `CharacterAction` messages with action number 180 (not in the schema's enum): the target's
  instance is the perk action's nano id (a `0xcfbe` record of the perk's template names it), `Parameter1` something else; they are
  the perk actions the character can use, not needed for the stats. Actions 137 and 146 (28 each, no parameters) and `SpecialUsed`,
  `SpecialAvailable`, `SetNanoDuration`, `StandUp` are the rest of the login burst.
* **Own `SimpleCharFullUpdate`** lists 11 active nanos: 268697, 226283, 269460, 273322, 202846, 202858, 223380, 223360, 215264 and
  two the live read had not shown, **223378** (Composite Attribute Boost) and **293685** (Coalescing Energy Cascade: a `0xcfb7`
  +1000 to max health and nano, which is 1100 at level 220, and +10 to every attribute, +20 to stat 123).
* Nothing in the capture delivers a separate list of level-scaled spells: the `0xcfb7` effects come from nano templates such as
  293685's; spell type `0xcfb7` is now read by `item::modifiers_at` / `nano_modifiers_at` (amount = round(level x value / 200)).
  The 25 events of the earlier hook log (attributes +9, skills +39 / +59 / +99 ...) were therefore a nano of that kind active at
  that time, not one of the nanos listed now; which one stays unknown, but the formula and the reading are in place.

`stats-check` with the server-sent perk list gives the same 192 equal stats as with the live read.

## The last small buff, found: every stat now matches (two level-220 characters, live)

With the bridge on a second original-client session (a different level-220 character, identity `0x4bc7f`) and hooks on `AddModifier`,
`RemoveModifier` and the `0xcfb7` handler, a zone change gave a full log. What it showed, and what was fixed:

* **The level-scaled batch.** The 25 events are real `0xcfb7` spells whose spell records live in the heap (the hook reads them: a stat, then
  key/value pairs `3:1, 4:0, 32:3, 35:9, 39:<value>`; the value is spell stat 39). They are *not* in any database object: the call chain ends in
  `n3Engine_t::AddNetWorkMessage` (N3.dll + 0x6643), so the server hands the client these spells (they are re-run on every zone). The two
  level-220 characters got **the same 25**: values (stat: value) 123 and 124: 16, 149: 11, 19 and 21: 6 and 9, 16/17/18/20: 9, 132: 9,
  122 and 127-131: 36, 364: 36, 318: -6, 139/152/161: 54, 156: 90, 343: 18 (`docs/level_bonus_records.json`). **The amount is
  `level * value / 200` cut toward zero** (16 -> 17, 36 -> 39, 54 -> 59, 90 -> 99, -6 -> -6; the earlier "round" guess was wrong).
  `derived::level_bonus` applies the table to level 205 and up (title level 7); lower levels are unknown, so nothing is added there. It is
  shown as "Level bonus" in the hover.
* **Items use the tail kind 3 as well as 2** (240 item templates; "Research Attunement Device" gives +75 to the nano skills with it). One level-220
  character was 75 short on skills 122, 127-131 only because of it. `item::modifiers_at` reads kinds 2 and 3.
* **A second flat spell type, `0xcf14`** (53012), carries the "Weapon Enhancement" nanos' +5 on stats 278-282, 311 and 317; `item` reads it as a
  flat modify-stat (nano 27189).
* Spell type `0xcfb7` scales by truncation, not rounding (`item::modifiers_at`).

Result with the client's own code against the original's values (`mao-client <game> stats-check ...`):
**Conjecture: 270 of 270 stats equal; the second character: 249 of 249** (inputs and values in `docs/live_inputs_*.txt`,
`docs/live_compare_*.txt`). That includes all armour classes, attributes, skills (with the trickle-down), max health and nano, and the
attack and damage-modifier stats. The hooks are still installed in the running client; `aoctl unhook 0`, `1`, `2` removes them.

## Correction: the "level bonus" is the organization's benefits (and no table is needed)

A third original-client character, level 86 (title level 3), showed that the 25 effects found on the two level-220 characters were not tied to
a level: the same stats were present but only some of them. The spell records that the hook read turned out to belong to **organization
benefit items** in the game database: "Large Org HQ Management Benefit" (attributes 16-21), "Mining Operations Nano Powerup" (skills 122, 127-131),
"Swimming Pool Body Dev" (152), "Grid House Computer Literacy" (161), "Notum Silo Nano Efficiency" (364, 318), "Landing Pad Vehicle Experience"
(156, 139), "Guard House Benefit" (343) and the "Universal Advantage - Harmony / Logic / Medical Expertise" items (132 and 21, 149 and 19, 123 and
124); each exists in four tiers (consecutive template ids, e.g. 0x3e1bc to 0x3e1bf) and the Universal Advantages are interpolated between a
low and a high template by a quality.

The server tells the client which ones apply: **`OrgServer` messages (`64582a07`) of type 6**, one per benefit, 10 for Conjecture's
organization: after the type word (2 bytes), two unknown ints and the organization's identity, a **quality (4 bytes), the low template id and
the high template id (4 bytes each)** and a flag byte. The client runs the benefit item's `0xcfb7` records (level x value / 200, cut toward
zero) at that quality, which is the "message-driven" call chain seen in the hook (`AddNetWorkMessage`).

So the hard-coded table was removed; `World.org_benefits` collects the messages and `derived::org_modifiers` works them out like an item.
With the ten messages of the capture: **Conjecture 270/270 and the second level-220 character 249/249 stats equal** (same organization);
the level-86 character is in other circumstances (no Universal Advantages, no Swimming Pool, Landing Pad or Guard House benefit, no second HQ
tier): 200/220 equal when given the same list, the 20 differences being exactly those benefits it does not have (its own list would come from
its own messages). The hover shows the source as "Organization".
