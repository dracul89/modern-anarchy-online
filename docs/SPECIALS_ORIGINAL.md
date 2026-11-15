# Special actions: the original client against the Rust client

Read-only audit, 2026-11-15. Sources: Gamecode.dll and GUI.dll decompiles and disassembly (addresses below are
Gamecode's unless marked GUI), the game's item templates (names via `mao-client <game> template <id>`), and the four
capture dumps. Nothing was run against the game or the network. Line numbers in `net.rs` are as of this audit (the
file was being edited while I read it; search for the quoted marker if they have moved).

Words used: the **holder** is the character's special-action object (character +0x1d0, made by FUN_10058d5d /
FUN_10043441); its list is at holder +0x84, its number-to-template map at +0x88, the selected target at +0x5c/+0x60.
The **lock list** is the character's skill-lock vector at +0x1bc (16-byte entries: stat at +4, total seconds at +8,
seconds left at +0xc). The **attack part** is character +0x1d4.

## 1. Every special action of the original

### 1.1 The table (FUN_10042b17) and `actions::TEMPLATES`

`TEMPLATES` matches FUN_10042b17 entry for entry: all 26 pairs are right, none is wrong. Two entries of the original
are not in the Rust table: numbers 0x8a and 0x8b with template 0 (see "alternate state" below). Leaving them out
changes nothing, since the Rust code filters template 0 anyway.

| number | template | name (template) | owner | toggles with |
|---|---|---|---|---|
| 1 | 0xc1aa | Pick-Up | character | - |
| 3 | 0xc1a3 | Use | character | - |
| 0x0b (11) | 0xc1a5 | Start Combat | character | 0x4e |
| 0x4e (78) | 0xc1ab | End Combat | character | 0x0b |
| 0x11 (17) | 0xc1a2 | Walk | character | 0x12 |
| 0x12 (18) | 0xc1a9 | Run | character | 0x11 |
| 0x13 (19) | 0xc1a7 | Sneak | character | 0x4f |
| 0x4f (79) | 0x1412c | Stop Sneaking | character | 0x13 |
| 0x4c (76) | 0xc1a8 | Sit | character | 0x4d |
| 0x4d (77) | 0xc1a6 | Stand | character | 0x4c |
| 0x51 (81) | 0x14124 | Suspended Animation | character | 0x52 |
| 0x52 (82) | 0x1412b | Disrupt Suspended Animation | character | 0x51 |
| 0x14 (20) | 0x39c7d | Crawl | character | 0x8d |
| 0x8d (141) | 0x39c94 | Crawl (the stop-crawling one) | character | 0x14 |
| 0x86 (134) | 0x1d9d7 | Search | character | - |
| 0x6e (110) | 0x3bd0b | Reload | character | - |
| 0x8e (142) | 0x14122 | Brawl | character | - |
| 0x90 (144) | 0x14125 | Dimach | character | - |
| 0x94 (148) | 0x14123 | Burst | weapon | - |
| 0x96 (150) | 0x14120 | Fling Shot | weapon | - |
| 0xa7 (167) | 0x14121 | Full Auto | weapon | - |
| 0x97 (151) | 0x14127 | Aimed Shot | weapon | - |
| 0x92 (146) | 0x1f83a | Sneak Attack | weapon | - |
| 0x93 (147) | 0x14126 | Fast Attack | weapon | - |
| 0x79 (121) | 0x1e159 | Bow special attack | weapon | - |
| 0x1e9 (489) | 0x32ba3 | Backstab | weapon | - |
| 0x8a / 0x8b | 0 | alternate state start / stop | character | each other |
| perk number (> 999) | from the server | perk action | (0xc78e, four-letter code) | - |

There is no further special attack in the original: no Parry, Riposte, shield or martial-arts entry. The only other
number the perform function knows is 0x89 (Forage, `N3Msg_Forage`), which is not in the table and is never added.

### 1.2 When each is added and removed

An entry is added by FUN_10042f09(number, owner, toggle): only for the client's own character, and only if the number
is in the table. Its duplicate check (FUN_1003f38b) compares the entry's action identity with the *owner* identity,
so it never matches: the original does not de-duplicate (see 1.3). If the number is above 0x8d and its stat is in the
lock list, the new entry is marked unavailable at once (FUN_1003f32d).

- **Character actions** (FUN_1004326f, when the character is made; arguments read from the disassembly at
  0x1004329a-0x10043322): Use, Pick-Up, Start Combat/End Combat, Sit/Stand, Walk/Run, Sneak/Stop Sneaking, Suspended
  Animation/Disrupt, Crawl/Crawl, Search, in that order. Never removed. Each pair is one list entry that swaps its
  number and template (FUN_1004300e -> FUN_10042eaa, GUI signal +0xa8) when the state changes: sneaking starts or ends
  (FUN_1003b7ae / FUN_1003b806), combat starts or stops (FUN_100690c8, FUN_1006a1b1, FUN_1005991a), the movement
  mode changes (FUN_1006d6df ... FUN_1006ee92), camping starts or stops (N3Msg_StartCamping 0x1001c9c6,
  N3Msg_StopCamping 0x1001cb63, and the server's CharacterAction 0x7a).
- **Alternate state** (0x8a/0x8b): added only if the profession has a row in the table at 0x102e3fc8 (FUN_1003e48a,
  profession stat 60). The table is all zero in this build, so it never appears. Dead code.
- **Brawl, Dimach** (and any other listed stat that is in the table): from the server's `SpecialAttackWeapon` list,
  FUN_1006b14c -> FUN_1006a166: once per stat (the stat-to-item map of the attack part de-duplicates), owner the
  character. Never removed. Stat 100 has no table row, so no action; its item becomes the unarmed weapon
  (FUN_1006a61a, slot 0).
- **Weapon attacks** (FUN_1009e84b, `WeaponItem_t` being put on; `actions::WEAPON_CAN` matches it entry for entry):
  for the client's own character and a wear place below 0x30, by the weapon's "can" flags (stat 30): 0x800 Burst,
  0x1000 Fling Shot, 0x2000 Full Auto, 0x4000 Aimed Shot, 0x20000 Sneak Attack, 0x8000 Bow special attack, 0x40000
  Fast Attack; owner the weapon. Removed by FUN_1009d39a (taking the weapon off a wear place): each bit the weapon
  has and the weapon in the *other hand* (place 8 if this one's stat 0xdc is not 8, else 6) does not have.
- **Backstab**: added beside Sneak Attack when bit 20 of the character's flag word at +0x138 is set (FUN_1009e84b);
  also added the moment the server sets that bit, for a sneak-attack weapon already in place 6 or 8 (CharacterAction
  0xb5 with a second parameter not zero -> FUN_10068976 -> FUN_1009c5c8). Removed together with Sneak Attack
  (FUN_1009d39a removes 0x92 and 0x1e9 both). Nothing clears the bit.
- **Reload**: FUN_1009e84b, when the weapon's stat 26 is not -1 (a missing stat counts as not -1); owner the
  character. Removed by FUN_1009d39a when this weapon's stat 26 is not -1 and the other hand is empty or its weapon's
  stat 26 is -1.
- **Perk actions**: CharacterAction 0xb4 -> FUN_10043080(number, code, template): sets the table row and adds the
  entry with owner (0xc78e, code). CharacterAction 0xb6 -> FUN_1003f80d -> FUN_1003f67b removes the entry with that
  number.

No profession, breed, stat, ammunition-count, shield or two-hand condition is checked on the client when adding. The
only state conditions are the ones above.

### 1.3 Differences in the add/remove rules (small)

- The original keeps one entry per weapon: two Burst pistols give two Burst tiles, and a swap can leave a stale one
  (removal takes one entry only when the other hand lacks the bit). The Rust list is recomputed and de-duplicated.
  I would keep the Rust behaviour; it is the original's bug, not a feature.
- The original adds on any wear place below 0x30; Rust looks at places 6 and 8 only. Weapons go in the hands, so
  this is the same in practice (not verified that no `WeaponItem_t` can sit elsewhere).

## 2. What triggering does in the original

### 2.1 The path

- Actions window click: GUI FUN_100dc033 -> `N3Msg_PerformSpecialAction(Identity)` (0x10027479) -> FUN_100427d6.
- Hotbar slot of kind 6: GUI FUN_100d833f -> the same.
- Key bindings (GUI `ControlCenterModule_c::SlotSpecialAction` 0x10068816, set up in 0x1006951c): ActionAttack 0xb,
  ActionUse 3, ActionSneak 0x13, ActionSit 0x4c, ActionSneakAttack 0x92, ActionFullAuto 0xa7, ActionBurst 0x94,
  ActionBrawl 0x8e, ActionReload 0x6e, ActionFastAttack 0x93, ActionDimach 0x90, ActionFlingShot 0x96,
  ActionAimedShot 0x97, ActionBowSpecialAttack 0x79 -> `N3Msg_PerformSpecialAction(Action_e)` (0x10027454) ->
  FUN_10042fc9: finds the entry whose number or toggle number is that; none -> `Feedback_ActionIsNotAvailable`.

### 2.2 FUN_100427d6 (perform, by action identity)

1. Entry not in the list -> `Feedback_ActionIsNotAvailable`, nothing sent.
2. Entry marked unavailable (flag bit 0 at entry +0x1c):
   - its stat is in the lock list -> the wait line (FUN_10064847: text category 1000, name
     `UnableToPerformActionSkill`, fed the skill's name from category 2003, then hours, minutes, seconds);
   - else a locked perk -> the same line with the perk's name (FUN_10052fc4 -> FUN_10052f2e);
   - else `Feedback_ActionIsNotAvailable`. Nothing sent in all three.
3. The target is the holder's selected target (+0x5c).
4. Owner is the character: 1 `N3Msg_GetItem(target)`; 3 `N3Msg_UseItem(target, false)`; 0xb and 0x4e
   `N3Msg_DefaultAttack(target, false)`; 0x11 / 0x12 / 0x4f `N3Msg_MovementChanged` 0x18 / 0x19 / 0x24; 0x13
   `N3Msg_TryEnterSneakMode`; 0x14 and 0x8d `N3Msg_CrawlToggle`; 0x4c and 0x4d `N3Msg_SitToggle`; 0x51 / 0x52
   `N3Msg_StartCamping` / `StopCamping`; 0x6e Reload (FUN_100699e3); 0x86 Search (FUN_1003fc96); 0x8e and 0x90
   `N3Msg_SecondarySpecialAttack(target, stat)`.
5. Owner type 0xc78e (perk): CharacterAction 0xb3 with the character as target and (number, code) as parameters.
6. Numbers 0x79, 0x92, 0x93, 0x94, 0x96, 0x97, 0xa7, 0x1e9: `N3Msg_SecondarySpecialAttack(target, number)`.

### 2.3 `N3Msg_SecondarySpecialAttack` (0x100281fc): every special attack

In this order; each refusal sends nothing.

1. Character flag byte +0x21d must be 0 (what it is: not identified).
2. Target: while fighting (attack part +0x44 not 1) the **fight target** (+0x4c), not the selection; otherwise the
   holder's selected target.
3. FUN_10069a9f(target, with feedback): no such dynel or not in the world -> silent; oneself ->
   `Feedback_YouCantAttackYourself`; flag 0x20000000 -> `Feedback_CantbeAttacked`; swimming ->
   `Feedback_YouCantAttackWhileSwimming`; flag 0x400 or movement state 7 -> `Feedback_YoureUnableToAttack`; dead
   target -> `Feedback_TargetIsAlreadyDead`; surrender states -> `Feedback_YouAcceptedSurrender`,
   `Feedback_TargetIsSurrendering`, `Feedback_TargetHasSurrendered`; an item that is not attackable ->
   `Feedback_YouCantAttackThisItem`; `Feedback_TargetIsInADistrictWithHigherSuppression` is told without refusing;
   own team/pets -> a numbered feedback (FUN_100692b4(5)).
4. The attack part's queue of pending specials (+0x84) must be empty (FUN_1002a027): a special already echoed by the
   server and not yet swung blocks the next one silently.
5. The stat is in the lock list -> the wait line with the time left (FUN_1006414f, FUN_10064847). No message.
6. Weapons: for the weapon in place 8 and in place 6 (FUN_10068c44 -> FUN_1009aced): does its "can" word have the
   special's bit (Burst 0x800, Fling Shot 0x1000, Full Auto 0x2000, Aimed Shot 0x4000, Bow 0x8000, Sneak Attack
   0x20000, Fast Attack 0x40000, **Brawl 0x2000000, Dimach 0x4000000**, Backstab = the sneak attack bit and the
   character's backstab bit), and is the target within that weapon's range (else
   `Feedback_TargetIsOutsideSpecialAttackRange`). If a hand holds a weapon and neither weapon passes ->
   `Feedback_SpecialAttackIsUnavailable`. With both hands empty this step passes. So Brawl and Dimach work unarmed
   (the Martial Arts item has both bits: "can" 0x66000009) or with a weapon that has the Brawl/Dimach bit, and are
   refused with any other weapon.
7. The special's own item from the server's list (FUN_10068c19; Brawl Item range 3, Dimach Item range 10): target
   outside its range (FUN_10067f0a: distance less both body radii against FUN_1009abd5 = the item's stat 287 scaled
   by the character's stat 380, capped at 40; the scaling formula is a guess, the float code was not decompiled) ->
   `Feedback_TargetIsOutsideSpecialAttackRange`.
8. Line of sight (FUN_10058e4f; only checked when one side is a player or in a dungeon) -> `Feedback_NoLineOfSight`.
9. Send `CharSecSpecAttack` (FUN_10072d55): the target identity (type and instance) and the stat.

Not checked on the client: being in a fight, sneaking or hidden for Sneak Attack/Backstab, ammunition, skill values.

### 2.4 Reload, Search, Sneak

- Reload (FUN_100699e3): only the pending-special queue check (step 4), then CharacterAction 0xd2, all else zero.
- Search (FUN_1003fc96): stat 0x88 (136) in the lock list -> the wait line; else `Feedback_SearchingForHiddenObjects`,
  effect 0xb01c on the character, CharacterAction **0x42** (the push at 0x1003fd32).
- Sneak (0x10029581): own stat 0x296 not 0 -> `Feedback_YouCantSneakInVehicle`; else CharacterAction 0xa3. Stop
  Sneaking is movement action 0x24. `Feedback_YouStartSneaking` / `Feedback_YouEndSneaking` when the state changes.

### 2.5 The server's answers (CharacterAction handler FUN_1005d61f; case table read from 0x1005f526)

| action | case | what the client does |
|---|---|---|
| 0xaa SpecialUsed (stat, seconds) | 0x3c | FUN_10064fb4: adds a lock entry (seconds unchanged for 142, 144, 146-148, 150, 151, 167, 579, 673; other stats scaled by the character's stat 382, FUN_10063de2), marks every action with that number unavailable (FUN_1003f32d, GUI signal +0xa4), raises signal +0x1ec |
| 0x14 (stat, seconds) | 2 | FUN_10065b19: the same, or lengthens a running lock |
| 0xa4 SpecialAvailable (-, stat) | 0x38 | chat line `Feedback_SkillAvailable` fed the stat's text of category 2002; removes the lock (FUN_10064e7a); marks the action available (FUN_1003f2cf) |
| 0xb2 (-, number) | 0x42 | marks the action available (FUN_1003f2cf). Not in any capture |
| 0x84 (stat, seconds, perk in the target instance) | 0x2a | the wait line (FUN_10064847) |
| 0xb4 / 0xb6 | 0x43 / 0x45 | perk action added / removed |
| 0xb5 | 0x44 | backstab bit |
| 0xcf / 0xce / 0xfc | 0x59 / 0x58 / 99 | perk lock starts / ends / all perk locks cleared (FUN_10053af2) |

The lock list counts down by itself, one per tick, and drops entries at zero (FUN_100649db).

In the captures 0xaa is a general skill lock, not only specials: Brawl 142 (15 s), Dimach 144 (1800 s), Fast Attack
147 (9 s), Burst 148 (18-44 s), and also 54, 114, 123, 124, 140, 164, 566 and 16. 0x84 was seen twice.

### 2.6 What the icon shows (GUI)

- State: `N3Msg_GetSpecialActionState` (0x1002742e -> FUN_1003f3c0): for numbers 0x8e-999 "locked" is "the stat is
  in the lock list"; for the others the entry's flag or a perk lock. GUI FUN_1003f5d4 disables the icon while locked
  (Actions window and hotbar).
- Recharge: `N3Msg_GetActionProgress` (0x10027773) gives seconds left / total. With the option `IconTimers` the icon
  shows the progress (GUI FUN_1003f40d); with `IconTimerText` it shows the time left as "%2ds" up to 60 s, else
  "%2dm" (GUI FUN_1003f8c2).
- The swing: the server's `CharSecSpecAttack` echo queues the special (FUN_10068cd9) and `SpecialAttackInfo` plays
  it. The Rust client already has this part. I found no sound in the trigger path.

## 3. Gaps in the Rust client, most visible first

1. **Wrong tiles for the six toggling character actions.** The original shows Sit or Stand, Walk or Run, Sneak or
   Stop Sneaking, Start Combat or End Combat, Suspended Animation or Disrupt, Crawl or Crawl-stop, by state
   (FUN_1004300e). The Rust catalogue (`actions_data.rs`, read from a sitting character) always shows Stand, Walk,
   Sneak, Start Combat, Suspended Animation, Crawl. Fix: `actions_data.rs` / `ActionsWindow::show` pick the template
   from the pair by the character's state (the viewer already knows sitting, walking, fighting, camping).
2. **Five character tiles do nothing.** Use, Pick-Up, Walk, Suspended Animation and Crawl fall to "its network
   message is not captured yet" (`net.rs` ~7319), and so does the `action_pick_up` key (`settings::QUICK`). The
   original runs UseItem / GetItem on the selection, the walk/run switch, StartCamping / StopCamping and CrawlToggle
   (2.2 step 4). The client already has use, walk toggle, sit/stand and camp as key actions; the tiles only need
   routing in the `keys.special` block (`net.rs` ~7261). "Stand" sends only `STAND`; the original's is SitToggle, and
   "Sneak" never leaves sneak mode (movement 0x24).
3. **Search sends the wrong action.** `net.rs` ~7272 sends CharacterAction 0x66; the original sends 0x42
   (FUN_1003fc96). No capture has a client Search, so 0x66 was never confirmed. Also missing: the 0x88 lock check,
   `Feedback_SearchingForHiddenObjects`, effect 0xb01c.
4. **No recharge shown for specials in the Actions window, and none on the hotbar for four of them.** The Actions
   window passes a countdown only for perks (`actions.rs` 249-265). `hotbar::special_stat` (124-134) maps by name and
   lacks Aimed Shot 151, Sneak Attack 146, Bow special attack 121 and Backstab 489, so their slots never follow
   0xaa/0xa4. Fix: key cooldowns by the action's number (`actions::number_of(id)`) instead of the name, keep a
   stat -> (left, total) map from `special_events`, and give it to `ActionsWindow::show`. The original greys the
   icon, shows the progress and the "%2ds"/"%2dm" text (2.6).
5. **A locked special is still sent, with no wait line.** From the Actions window nothing is checked; the hotbar
   has its own "is not ready: queued" text and queue, which the original does not have. The original prints
   `UnableToPerformActionSkill` (category 1000) with the time left and sends nothing (2.2 step 2, 2.3 step 5). The
   server's own wait message, CharacterAction 0x84, is not handled in `net.rs` either. For a locked perk the Rust
   window ignores the click silently (`actions.rs` 299-303); the original prints the same wait line with the perk's
   name.
6. **`Feedback_SkillAvailable` is not told.** On 0xa4 the original writes a chat line (category 2002 text of the
   stat); `net.rs` 2274 only clears the cooldown.
7. **Missing checks before a special attack is sent** (`net.rs` ~7279-7306; a new `combat::special_attack` beside
   `default_attack` would hold them):
   - the target is the fight target while fighting, not the selection (2.3 step 2);
   - FUN_10069a9f's refusals with their feedback keys; with no target the original says nothing, Rust prints its
     own "needs a target" text;
   - `Feedback_SpecialAttackIsUnavailable` when a held weapon lacks the special's "can" bit. This matters for Brawl
     and Dimach, which the Rust client offers and sends with any weapon (bits 0x2000000 / 0x4000000 are not in
     `WEAPON_CAN` because they do not add tiles; they gate the use);
   - `Feedback_TargetIsOutsideSpecialAttackRange` and `Feedback_NoLineOfSight`;
   - the silent block while a special is queued (also for Reload).
8. **A special that is no longer in the list is still sent.** A hotbar entry or quick key for Burst, Fling Shot,
   Full Auto, Brawl, Dimach or Reload sends without looking at `specials_told` (only the "that action" branch
   checks). The original prints `Feedback_ActionIsNotAvailable` (FUN_10042fc9, FUN_100427d6 step 1).
9. **Key bindings missing** for Sneak Attack, Fast Attack, Aimed Shot and Bow special attack (`settings::QUICK`);
   the original binds all of them (2.1).
10. **Small protocol points.** `n3::special_attack` hard-codes the target type 50000; the original sends the
    target's own identity, which can be an attackable item. CharacterAction 0xb2 (action available), 0x14 (lock
    start/extend) and 0xfc (all perk locks cleared) are not handled; 0xaa for non-special stats (first aid,
    treatment and so on) is only used for hotbar specials.
11. **No change needed:** `TEMPLATES`, `WEAPON_CAN`, the Backstab rule, the Reload rule, Brawl/Dimach from the
    server's list, perk add/remove (0xb4/0xb6), the perk message 0xb3, Reload's 0xd2, Sneak's 0xa3, the
    `CharSecSpecAttack` layout and stats, and 0xaa/0xa4 parameter order all agree with the original.

Guesses and things not verified: the range scaling formula (2.3 step 7); what character byte +0x21d is; whose flag
0x20000000 / 0x400 FUN_10069a9f reads (own or target, the decompile dropped the object); that no weapon item can be
worn outside the hands; the chat group of the `Feedback_SkillAvailable` line (a 0x1e is pushed).

## 4. Special-like actions outside the Actions window

The original's Actions window (`SpecialActionView_c`) holds only the list above; pet commands, assist and emotes
are not in it. Assist is `N3Msg_AssistFight` (0x100274f0, `/assist`; feedback `Feedback_NoTargetToAssist`,
`Feedback_CantAssistYourself`, `Feedback_TargetIsNotInFight`), which the Rust client has as a chat command. The
social actions are the control center's menu, as in the Rust client. The remaining key-bound "#Action" entries are
ActionCreateRef, ActionLookAt and ActionSwitchTarget, which are not special actions. Pet commands were not looked at.
