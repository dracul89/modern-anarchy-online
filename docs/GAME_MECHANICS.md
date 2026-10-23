# Game mechanics read from the running original client (via ao-bridge)

Everything here was read from a live `AnarchyOnline.exe` (18.8.62_EP1) with the `ao-bridge` agent: the engine object is the pointer variable `Interfaces.dll!?m_pcN3@N3InterfaceModule_t@@0PAVn3EngineClientAnarchy_t@@A`, and the numbers are what the game's own `N3Msg_*` methods return. The character was a level-25 Soldier (Brasscasings, client id 877709289).

## 1. Special actions (`N3Msg_GetSpecialActionList`, 50 of them)

A std::list; each node: `+0 next, +4 prev, +8 action identity (type 0xdeb0, instance = action id), +0x10 template identity (what owns it), +0x18/+0x1c/+0x20 numbers, +0x24 on cooldown`. Names from `N3Msg_GetName(action, own character)`; readiness from `N3Msg_GetSpecialActionState` (0 ready). A perk's template identity is type `0xc78e` (PerkHash) whose instance is a four-letter hash.

### Weapon attacks (template: the wielded weapon, type 0xc74a)
| id | name | skill |
|---|---|---|
| 0x14123 | Burst | Burst |
| 0x14120 | Fling Shot | FlingShot |
| 0x14121 | Full Auto | FullAuto |

### Character actions (template: the character, type 0xc350)
| id | name | numbers (skill or command code) |
|---|---|---|
| 0x3bd0b | Reload | 110, 110, 0 |
| 0xc1a3 | Use | 3, 3, 0 |
| 0xc1aa | Pick-Up | 1, 1, 0 |
| 0xc1a5 | Start Combat | 11, 11, 78 |
| 0xc1a6 | Stand | 77, 76, 77 |
| 0xc1a2 | Walk | 17, 17, 18 |
| 0xc1a7 | Sneak | 19, 19, 79 |
| 0x14124 | Suspended Animation | 81, 81, 82 |
| 0x39c7d | Crawl | 20, 20, 141 |
| 0x1d9d7 | Search | 134, 134, 134 |
| 0x14125 | Dimach | 144, 144, 0 |
| 0x14122 | Brawl | 142, 142, 0 |

The numbers are the gating skill for Brawl (142), Dimach (144) and Reload (110: Heavy Weapons slot); for the basic actions they are command codes (Pick-Up 1 and Use 3 are the generic-command ids; Start Combat 11, Walk 17, Sneak 19, Crawl 20, Stand 77, Suspended Animation 81, Search 134).

### Perks (template type 0xc78e; this character had 35)
| id | name | hash | perk id |
|---|---|---|---|
| 0x40f9d | Reconditioned | SUBD | 12854 |
| 0x40fcc | Successful Targeting | CVSH | 12840 |
| 0x40fe7 | Fuse Body Armor | RSME | 12839 |
| 0x40fcf | Gear Assault Absorption | CETM | 12833 |
| 0x40f2f | Concussive Shot | AGAM | 12832 |
| 0x40f32 | Emergency Bandages | ERAM | 12822 |
| 0x40f9a | Deep Six Initiative | EMBA | 12811 |
| 0x40f6d | Shoot Artery | SRAY | 12810 |
| 0x40f38 | Furious Ammunition | INAM | 12809 |
| 0x40fe4 | Graze Jugular Vein | RTMO | 12806 |
| 0x40f35 | On The Double | FUAM | 12797 |
| 0x40f70 | Target Acquired | GRDR | 12795 |
| 0x3dcb3 | Fire Frenzy | FRFR | 11179 |
| 0x3dcb0 | Fuzz | CXWZ | 11174 |
| 0x3c989 | Muzzle Overload | MZOV | 11029 |
| 0x3c984 | Clipfever | CLFE | 11025 |
| 0x3c977 | Supressive Horde | SPHO | 11020 |
| 0x3780a | Guardian | ARDI | 10739 |
| 0x37808 | Violence | EEED | 10737 |
| 0x37393 | Contained Burst | CNBU | 10735 |
| 0x37388 | Tracer | TRAC | 10732 |
| 0x37801 | Field Bandage | FEBN | 10730 |
| 0x37777 | Detonate Stoneworks | DTSN | 10544 |
| 0x37776 | Encase in Stone | ENIS | 10542 |
| 0x3758c | Napalm Spray | NPSR | 10486 |
| 0x3758b | Triangulate Target | TRTA | 10484 |
| 0x3724d | Weapon Bash | WEBS | 10482 |
| 0x37580 | Laser Paint Target | LAPI | 10481 |
| 0x37982 | Grip of Colossus | GIOC | 10432 |
| 0x37981 | Bearhug | EARH | 10431 |
| 0x37980 | Grasp | BFEI | 10430 |
| 0x37797 | Heal | BDNB | 10401 |
| 0x34e25 | Draw Blood | DRBO | 10278 |
| 0x34e24 | Lifeblood | LIFE | 10274 |
| 0x34e1a | Blessing of Life | BNOI | 10272 |

## 2. Social actions and dances (`N3Msg_DoSocialAction(AbstractAnimID_e)`)

A name to id table sits in `Gamecode.dll` (also in `GUI.dll`): entries of `{const char* name, int id, const char* alias}` starting at `0x6aadfbd8`. The animations are `male_social-<name>` / `female_social-<name>` (type 0xf6953). The ids:

1 allah (alias prostrate), 2 angry, 3 apachi (dance), 4 applause, 5 ass (alias itch), 6 backflip, 7 ballet (dance), 8 blowkiss, 9 bow, 10 bulge, 11 chicken (dance), 12 cross, 13 crossarm, 14 crotch (alias adjust), 15 curt, 16 disco (dance), 17 drink, 18 eat, 19 fblock, 20 fishsize, 21 flamenco (dance), 22 flip, 23 giggle, 24 gloat, 25 greet, 26 italian (dance), 27 kneel, 28 laugh-b, 29 laugh-s, 30 legshake, 31 lookout, 32 moon, 33 nod, 34 nono, 35 pointba, 36 pointfor, 37 pointlef, 38 pointrig, 39 pointup, 40 pray, 41 puke, 42 pulp, 43 read, 44 rocky (dance), 45 salute, 46 scared, 47 scratch, 48 shake, 49 shrug, 50 slap, 51 speech, 52 spit, 53 strong1, 54 strong2, 55 strong3, 56 strong4, 57 surprised, 58 surrender, 59 swroyal (dance), 60 thinker, 61 thumbs, 62 wave, 63 ymca (dance)

Dances among them: apachi, ballet, chicken, disco, flamenco, italian, rocky, swroyal, ymca (the animation set also has `boogie`, `backflip`, `flip`, `strong1-4` and a lounge/sleep pair that are not in this table).

## 3. Attack and movement entry points on the engine object

`N3Msg_DefaultAttack(target, bool)`, `N3Msg_StopAttack`, `N3Msg_IsAttacking`, `N3Msg_SecondarySpecialAttack(target, stat)`, `N3Msg_IsSecondarySpecialAttackAvailable(stat)`, `GetAttackRange` (a float, returned in ST0), `N3Msg_PerformSpecialAction(action)`, `N3Msg_GetSpecialActionState(action)`, `N3Msg_CastNanoSpell(nano, target)`, `N3Msg_GetNanoSpellList`, `N3Msg_GetNanoTemplateInfoList(character)` (the active buffs), `N3Msg_GetSkill(character, stat, detail, junk)`, `N3Msg_GetSkillMax(stat)`, `N3Msg_IsMoving`, `N3Msg_MovementChanged(action, x, y, bool)`, `N3Msg_UseItem`, `N3Msg_UseItemOnItem`, `N3Msg_UseItemOnCharacter`, `N3Msg_SplitItem`, `N3Msg_TeamJoinRequest`, `N3Msg_GetQuestWorldPos`, `N3Msg_GetPerkName`, `N3Msg_IsPerk`, `N3Msg_HasPerk`.

## 4. What to add to the new client

* A **special-action bar**: the 3 weapon attacks, Brawl, Dimach, Reload and the perks, with ready/not-ready from the cooldown flag (our hotbar already takes actions with cooldowns).
* The **basic actions** (use, pick up, start combat, walk/sneak/crawl, stand, search) as keybound commands.
* The **social table** as `/emote` chat commands and a menu, played through the `social-*` animations the client already loads.
* How these go over the network is still to capture: do the actions with the original client behind the proxy while `ao-bridge` triggers them (`aoctl call ... N3Msg_DoSocialAction` etc.).
