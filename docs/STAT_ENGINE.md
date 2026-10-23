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
