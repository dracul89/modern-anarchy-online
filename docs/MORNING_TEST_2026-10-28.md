# What changed overnight (2026-10-28) and what to test

The overnight run went through effects (particle system), then the GUI controls, then combat. Each item was read from the
decompiled original (`exports_*`) and the drop-ins, ported, unit-tested and checked offscreen where it can be drawn. Little of
it has been seen live: that is what the morning test is for. Commits in order, oldest first. The three hoverboard commits in
between (1960b16, 10bd999, 9a7beba) came from the other session.

## 1. Effects (nano and item visuals)

| Commit | Effect kind | Where to see it |
|---|---|---|
| 94fce6f | 3034 Shield2: a shell over the target's body | shield/reflect nanos on a character |
| b17c213 | 3025 EffectMesh: a named mesh as a shell | buffs that wrap a mesh round the body |
| 55086b0 | 3000 ShockWave: ground rings and light cones | AoE nukes, landings |
| bd5405e | 3039 Trail2: exhaust ribbons | hoverboards/hoverbikes moving |
| d891a40 | 1029 VulcanRocks: rocks thrown up that bounce and rest | earth/rock nukes |
| 869f2e4 | 3024 BParticle (kind-8 pops) | hits that "pop" |
| 7b5a8f8, d925729, 1a6902b | 1025/1019 flechette streaks, 1022 Tracer3, 1007 Nano0 | projectile nanos in flight |
| d601b51 | 3036 Toggle: effects that start when a vehicle moves off | hoverboards starting to move |
| 3130396 | 3020 TParticle streaks | various hits |
| b5945df | 3006 SkyFlash: cones coming down from the sky | "from above" nukes |
| b91adf9 | 3033 Spiral2: ribbons spiralling round the point | buff casts |
| 689ed7e | 3031 TParticle2: particles as streaks | sparks |
| a0fcadd | 1004 Fire: the Rage buffs' flames off the back | Berserk / Incandescent / Fell Rage |
| 226a1d9 | 3032 GroundShake: the camera shakes near the effect | big nukes landing near you |
| d3df9c3 | 3030 GroundGrid: a grid sheet on the ground | some area effects |

**Test:** cast or watch nanos and specials and look for anything badly placed or stuck. The ledger
(`docs/ORIGINAL_AUDIT.md`, item 10) lists the kinds still not ported, all used by fewer than eight programs.

## 2. GUI controls in the original's art (`chrome.rs`)

| Commit | Control | Test |
|---|---|---|
| de24640 | Scroll bars (`chrome::scroll`): `SIMPLE_SB` arrows and knob, no track | Skills, Options key list, chat, inventory backpack: click the arrows (5 points, holding repeats), click above or below the knob (20 points), drag the knob; the wheel as before |
| 9aa227f | Tabs (`chrome::tab_view`): the TabView strip, overlapping tabs, the border round the content, Verdana 13 | Actions (Combat/Perks/Social), inventory wear tabs (Weap/Cloth/Imp/Social), chat tabs: a tab opens on the press |
| 50175ed | Buttons (`chrome::button`): `BORDER01` at rest, `BORDER04` pressed, the `BORDER03` frame under the pointer | mail, shop, missions, loot, friends, NCU, options. Button rows are taller (about 42 points, as the original sizes them) |
| e963950 | Check boxes (`chrome::checkbox`) | Options: auto-loot, original chat font |
| 73ea999 | Text fields (`chrome::text_edit`): the `INSET` frame | find boxes, friends "Add", mail To/Subject/body, the skin folder |
| e9986bc | Dark themes (AOU Translucent): controls in the theme's cyan, inactive tab names readable | switch between the AOU and original themes |

**Things to look at:** the tab text (the original's font 5 is Verdana 13, read from Windows' fonts); whether the inventory's
four tabs are too cramped (the wear column is narrow, so their names are cut); the taller button rows.

## 3. Combat

| Commit | What | Test |
|---|---|---|
| 6256efa | **Attack swings**: `AttackInfo` and misses make the attacker swing its weapon (pistol shot, rifle shot, blade slash/stab, two-handed cuts, martial-arts punches and kicks), at the original's speed scaling | fight something, watch yourself and others |
| 178f42c | **Special attacks** swing their own class (Burst, Full Auto, Aimed Shot, Fling Shot, Fast Attack, Sneak Attack) | use specials from the hotbar |
| bb4cb96, b356f25 | **Weapon stance in a fight**: a fighting character with a weapon stands in the weapon's idle (`idle-smallarms`, `idle-blade`, `idle-rifle` ...). Fights tracked from `Attack`/`StopFight` | start and stop a fight, standing still |
| 8d1871e | **Draw and holster**: starting a fight plays the weapon's start once, ending it the stop | as above |
| 64a056c | **Out of a fight**: a rifle is held two-handed (`idle-2h`) and walked/run with `walk-2h`/`run-2h`; bows and bazookas their own idles | wield a rifle and walk |
| 1f8c538 | **Deaths**: the dying play their death (`die-knees`, `die-shot` ... as the server says) and lie until despawned (~10 s), instead of standing on | kill something |
| 888a011 | **Creature swings**: monsters attacking play their own attack animation | get hit by a monster |
| 2c2d490 | **Combat tab in the original's words** (`text.mdb`): "You hit X for N points of melee damage.", "X hit you for ...", "You tried to hit X, but missed!", "You hit X with nanobots for ...", "You were healed for N points.", "Critical hit!" | fight; compare the Combat tab with the original client's |
| e1b8a69 | **Shields, absorbs, XP in the original's words**: "Your damage shield hit X for N points of damage.", "You absorbed N points of cold damage.", "You received N xp." | kill something with a damage shield up; check the XP line on a kill |
| d129213 | **Facing the target**: a standing fighter turns to face its target (every 0.67 s, when more than ~15° off, at 7.2 rad/s) | start a fight with the monster beside or behind you and stand still |
| 0b24018 | **NPC animation sets read exactly**: an NPC's set now keeps every variant of an action (a humanoid NPC's 90 socials, its 3 deaths ...), each NPC picking its own | look at a crowd of the same NPC: their idles/deaths may differ now |
| 8d71c02 | **Rifle wield out of a fight**: equipping a rifle plays `2h-start` before `idle-2h`, unequipping `2h-stop` | wear and remove a rifle while not fighting |
| b3316c0 | **Reload / stop lines**: "Reloading..." then "Reloading completed" (System tab) as the original; no more "You stop fighting."; NPCs swing at their animation's own speed (c8c2594) | reload a gun |
| b55e7f0 | **Nano casts**: casting tells "Executing Nano Program: <nano>." in the Combat tab (the original's line); the client's own "X casts Y on you" lines are gone (the original has none) | cast a nano |
| 2c2d490 | **Fix: `HealthDamage` direction**: the floating numbers and health bars now follow the one hurt (the message's sender), not the source | your health bar should no longer jump to a monster's health in a fight |

**Looked for, not found: a muzzle flash on held guns.** The `muzzleflash` effect (an `eff_muzzleflash` connector → effect
template 2000, kind 1006, `_GfxControlFlare1_t`, which bursts on its own timer while its mesh is shown) belongs to statel
meshes such as turrets. The guns' held meshes (the Peregrine's 264695, the Shark's 262556) have only `Attractor01_weaponfire`,
and their templates name no effect: their stat 420 picks the fire *sound* (FUN_1009d1f1). **Not done:** the
hit/impact sounds (**the client has no audio at all yet**: sounds need a new audio dependency and the sound formats, so that is left for you to decide), and the nano-cast lines in the original's wording (still the client's own).

## 4. Sound (rodio, the original's SandyInterface)

Added after the overnight run, on request: audio through rodio, ported from the `sandyinterface` drop-in. Newest first.

| Commit | What | What to try |
|---|---|---|
| (next) | **Shots and hits**: each swing's `attack` note plays the weapon's sound; hits play the impact and the one hit's grunt (by sex) or a creature's own impact | fight a monster with a gun and with your fists |
| 9d338fc | **Steps, swims, landings and swishes** from the animations' notes; NPCs with animation sets play their own (idles, attack starts) | walk and run past others, jump, punch; listen to monsters |
| 1893cf0 | **The sound system**: the source files, the engine (13 channels, priorities), distance volume, variants and children, the frame's play list; the listener follows your character | `mao-client "<game>" sound SM_Sandy_Gui_Mission_Complete` should be heard; `MAO_NO_SOUND=1` silences the client |

## How to run

The release client was rebuilt and restarted after every commit:
`target/release/mao-client.exe "<game>" play`. Unit tests:
`MAO_GAME="<game>" cargo test --target-dir target-dev -p mao-client -p mao-formats -p mao-net` (all passing).
