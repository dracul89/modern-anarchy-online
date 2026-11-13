# Flight for a worn item in the original client (Ganimedes Personal Jetpack, 290215)

Read-only analysis of Gamecode.dll, GUI.dll, N3.dll and Vehicle.dll decompiles, the game database record of the item,
the 2026-10-31 capture, and the new client's sources. Addresses are Gamecode.dll's unless a DLL is named.
"Not found" means I looked and did not find it; "inferred" means the code makes it very likely but I saw no direct proof.

## Short answer

- **The spell that makes a character fly is 0xcf92, not 0xcf4c.** 0xcf92's handler (FUN_100a8107) fires movement action
  0x1d (`SwitchToFlyModeTransitionAction_t`, mode 7) when it is applied and 0x27 (`LeaveFlyModeTransitionAction_t`) when it
  ends. 0xcf4c (FUN_100a7c79) *clears* bits of the state-flag stat 224 and never touches the movement mode.
- **Flight is entered automatically**, with no player action: the mode switches the moment the spell runs. The original
  then lifts the body 0.5, turns falling off, plays the hover animation (action 0xb3 `hover-norm`) and makes a flash.
  Nothing lifts it further.
- **Height is controlled only by pitching the body with the right mouse button's vertical movement** and then moving
  forward or back along the pitched facing. Jump is ignored in fly mode. No key or input action for up/down exists in
  GUI.dll.
- **The server is not told about the mode.** The client sends its ordinary position reports, with the pitched rotation.
- **So "stays down" is also what the original does until the player pitches up.** I found no defect in the new client's
  fly branch by reading it. What the new client lacks is the visible part (hover animation, flash), the fly speed, the
  right signal (the server's spell list), the ways flight ends, and everything for a Yalmaha.

## The item's data (read from ResourceDatabase.dat, record found by scanning for the key)

Jetpack 290215, wear event 14, eleven spells (all with stats 3=1, 4=0, 32=2, 35=9):

| # | Spell | Criteria | Arguments |
|---|---|---|---|
| 1 | 0xcf92 | none | none |
| 2 | 0xcf35 | none | stat 156, value 150 |
| 3 | 0xcf4c | none | stat 73 = 2, stat 25 = 0 |
| 4-11 | 0xcf2d (eight of them, not eleven) | breed / sex / stat 47 | mesh 0x46d60..0x46d67 (290144..290151) |

Its action requirements (the `22, 36, n×1009` table after the spells):

- action 8: `not (operator 70)`, stat 438 op 107 value 1, stat 139 > 80, stat 410 = 0, stat 359 = 0, stat 389 bit 1,
  stat 531 = 0, operator 112.
- action 136: stat 438 op 107 value 1, stat 359 = 0, stat 531 = 0, operator 112.

Operator 70 (0x46) is "is flying": FUN_100065d8 case 0x46 is `FUN_10070a2f() == 7`. Operator 112 (0x70) is "flying must
be allowed on the playfield": case 0x70 fails when the playfield resource's `+0x58 & 0x80000` is set (feedback string
`FlyingMustMeAllowedOnThePlayfield`). The new client's `requirements.rs` has neither operator.

Yalmaha XL 96088, wear event 14, five spells: 0xcf35 (156, +900), 0xcf4c (73 = 2), 0xcf76 (stat 360 = 20), 0xcf92,
0xcf44 with **no criteria** and template 0x1848e (99470). "Yalmaha - The Poniard" 202453 has the same plus 0xcfaa
(146, 10) and template 0x31999.

## 1. What the spells do

**Which spells the client runs at all**: FUN_100a57a0 lists 0xcf35, 0xcf44, 0xcf4c, 0xcf76 and 0xcf92 (returns 1).
0xcf2d is not in the list and has no case in the executor FUN_100a5f4b: the client does nothing for it as a spell.

**0xcf92 → FUN_100a8107** (dispatch at FUN_100a5f4b `case 0xcf92`):

```
iVar1 = FUN_10070a2f();                    // the movement mode
if (iVar1 == 5) uVar3 = 0x28;              // crawling: LeaveCrawl
else { if (iVar1 != 8) { if (iVar1 == 0xb) uVar3 = 0x29; else { if (iVar1 != 0xc) goto LAB; uVar3 = 0x2a; }
         (**(code **)(*piVar2 + 0x18))(uVar3); }      // sleeping / lounging left first
       uVar3 = 0x25; }                                 // then LeaveSit
(**(code **)(*piVar2 + 0x18))(uVar3);
LAB: if ((*(byte *)(param_1 + 0x24) & 1) == 0) uVar3 = 0x1d; else uVar3 = 0x27;
(**(code **)(*piVar2 + 0x18))(uVar3);
```

`piVar2` is the character's vehicle's movement status (`CharMovementStatus_t`, vehicle +0x178, FUN_1006f2e1); its
+0x18 is FUN_1006c9a9 → FUN_10070be8, which asks the status's +0xc (FUN_1006c6f4, "may this action happen in this
mode") and builds the transition in +0x20 (FUN_1006cb58). In FUN_1006cb58, `case 0x1d` sets the mode to 7 with
`SwitchToFlyModeTransitionAction_t`, `case 0x27` sets it back to the remembered speed mode (`extraout_ECX[0xc]`, walk or
run) with `LeaveFlyModeTransitionAction_t`. Bit 0 of the run's +0x24 is "the list is ending" (FUN_100026d0 sets it from
its ending argument).

**0xcf4c → FUN_100a7c79**: applied, `FUN_10044c71(GetStat(0x49))`; ending, `FUN_10044aac(GetStat(0x49))`.
FUN_10044c71 decrements a per-bit counter and **clears** the bit in stat 0xe0 (224); FUN_10044aac counts up and sets
it. (0xcf4b, FUN_100a7bd4, is the mirror image: it sets.) The stun spell 0xcf82 confirms the direction: it calls
`FUN_10044c71(6)` when it begins. With mask 2 the only things FUN_10044c71 can do are clear bit 2 and, if bits 2 and 4
are then both gone, touch the CAT mesh; the mode switches in it hang on bit 4 and bit 8. So on the jetpack it clears
state flag 2 and nothing visible follows. What flag 2 is for was not identified (MouseMovement and FUN_1006bd94 let a
character with flag 2 but not 4 turn only).

**0xcf35 → FUN_100a517a**: modifies stat 156 (the run-speed stat FUN_1006f2fc reads) by +150.

**0xcf2d**: no client code runs it (see above). How the back mesh reaches the client was not traced.

**0xcf76 → FUN_100a95c7** (Yalmaha): sets the stat and remembers the old value (FUN_10064d77), restores it at the end
(FUN_1006420f). With stat 360 = 20 the stat handler FUN_1005a3b1 (`iVar8 == 0x168`) clamps to at least 20 and marks the
mesh dirty; the body scale is `stat 360 / 100` (`SetBodyScale` in FUN_1005c3ed and FUN_10052686). So the rider is drawn
at 20 %.

**0xcf44 → FUN_100a7aa2**: for a non-NPC with stat 0x167 (359) still 0, sets stat 359 to the template. The stat handler
(`iVar8 == 0x167`) calls FUN_10052686, which loads the template (FUN_10052271) and marks the mesh dirty. Note: in
FUN_10052686 the copying of the template's stats (FUN_100525f3), the template's event 14 and its `SetBodyScale` are
inside `if (*(char *)(dynel + 0x21c) != 0)`, the NPC flag, so a player gets the template's mesh but not its stats.

## 2. How flight is entered

**Automatically, when spell 0xcf92 runs.** Every source of action 0x1d in Gamecode:

- FUN_100a8107 (the spell), directly on the status. No message is built.
- FUN_1006bd94, the movement message's apply (`CharDCMove`), which **refuses** it for ordinary players:
  `if (action == 0x1d && *(char *)(dynel + 0x21c) == 0 && (GetStat(0xd7) & 1) == 0) return;` (stat 215; the jump code
  FUN_10058994 lifts a cap for the same stat, so presumably a GM stat).
- Nothing else. No caller passes 0x1d to `N3Msg_MovementChanged`; GUI.dll calls it only from `SlotMovementForward`,
  `Back`, `Left`, `Right`, `StrafeLeft`, `StrafeRight` and `Jump`. GUI.dll's input action names are
  `MOVEMENT_FORWARD/BACK/LEFT/RIGHT/STRAFELEFT/STRAFERIGHT/JUMP/AUTORUN/TOGGLEWALK` only: no fly, takeoff, up or down.

**Who runs the spell.** The item's own wear function does not. FUN_1009e84b (item worn at a place below 0x30) walks
event 14 and runs only spells with flag 0x80: `if ((*(uint *)(spell + 0x24) >> 7 & 1) != 0) FUN_100a5f4b(...)`. The
spell constructor FUN_100a597f sets 0x80 only for 0xcf14, 0xcf26, 0xcf2f, 0xcf35, 0xcf93-0xcf95, 0xcf9e, 0xcfc0,
0xcfe8 and 0xcfeb. So of the jetpack's spells the client runs only the 0xcf35 itself.

The rest comes from the server as a spell list (`SpellListIIR_t`, message 4d450114, FUN_100aae0e → FUN_100026d0 →
FUN_100a5f4b). **Verified for a vehicle program** in `captures/arete-20261031-0842/dump.txt`: line #305 is the list
"Phasefront Banshee - Red" with 0xcf35 (156, 800), 0xcf4c, 0xcf92 and 0xcf44, ending flag 0; #526 is the same list with
the ending flag 1 (also #621 / #1527 for the Wraith). **For a worn item this is inferred**: no capture holds an item
with 0xcf92 being put on, so I did not see the server send the list on equip, nor what it sends at login or zoning to
a character already wearing one.

**What the switch does** (FUN_1006e4f8, `SwitchToFlyModeTransitionAction_t`, vtable 0x101618a8):

- `FUN_1006f9eb()`: the fly speeds (section 3).
- `(**(code **)(*(int *)(dynel + 0xe8) + 0x10))(0x20000000)`: sets bit 0x20000000 of stat 0 (the clear is
  FUN_1005895c, which writes stat 0).
- `Vehicle_t::DisableFalling`.
- If the vehicle's `GetZone()` is not negative: `SetRelPos` by (0, 0.5, 0) (`_DAT_1015e0e4` = 0.5).
- `Vehicle_t::SetOrientationMode(this, 4)`.
- For the client's own character, eight entries of its action list are disabled (FUN_1003f32d with 0x4d, 0x4c, 0x11,
  0x12, 0x13, 0x4f, 0x14, 0x8d; 0x11 / 0x12 are the walk / run pair that FUN_1004300e swaps on leaving).
- The animation: `(**(code **)(**(dynel + 0x1dc) + 0x14))(FUN_1003c653(), 1.0, 0, 0, -1, 2, 0, 0, 0)`.
- An effect: `CreateEffect2(2000, vehicle position)`, `SetColor(0xfff0e0ff)`, the handler's +0x54 = 0x20.

## 3. Control in flight

**Pitch: the right button's vertical mouse movement.** GUI.dll FUN_1002ccbe (the action view's mouse handler) scales
the movement by the `MouseTurnSensitivity` value and, in mode 2 (the right button without the qualifier bits 0xc),
calls `N3Msg_MouseMovement(x, y)`. In `N3Msg_MouseMovement` (0x100196b3):

```
*(bool *)(ebp - 0xd) = FUN_10070a2f() == 7;            // fly mode
... if not flying: the RMBMouseLook1st / 3rd options → MouseCameraControl(camera, 0.0, y)
    else:          n3Camera_t::MouseCameraControl(camera, 0.0, 0.0);
...
if (*(char *)(ebp - 0xd) == 0) fVar8 = 0.0; else fVar8 = (float)s_nInverted * y;
N3Msg_MovementChanged(this, 0x2b, x, fVar8, true);
```

The first event of a drag starts a mouse turn (action 10 or 0xd, with 0, 0); the following ones are 0x2b. In
`N3Msg_MovementChanged` (0x10018bc4) 0x2b is not sent: it calls the dynel's +0x74,
`n3Dynel_t::VehicleForwardUpdate(&zero, rot, x, y)`, and sets the engine's +0xfc ("turned"). N3.dll's
`VehicleForwardUpdate` (0x10004ebd) turns the rotation by `x` about (0, 1, 0) and by `y` about the body's own right
axis, and keeps the pitched result only while the new forward's horizontal part still points the old way
(`new.x*old.x + new.z*old.z > 0.001`, `_DAT_1003c8a0`). So the pitch stops just short of straight up or down.
Outside fly mode `y` is zeroed, so only a flyer can be pitched.

**Thrust: the ordinary forward / back keys, along the pitched body.** FUN_10071537 → `Vehicle_t::SteeringForward`
(Vehicle.dll 0x1000ca73) is `GetBodyForward() × max force`; `SteeringReverse` the same times -1.0. Strafing works
(FUN_1006db3d / FUN_1006dc33 pass speed class 7 in fly mode). Turning with the keys is unchanged.

**Jump does nothing.** `N3Msg_MovementChanged`: `if ((mode != 7) || (action != 0xf)) { ...everything... }`.

**An unused up/down exists.** Actions 0x11 / 0x12 (`ElevateUpStart/Stop`, FUN_1006de91 / FUN_1006dee5; 0x11 is only
allowed in mode 7) set the vehicle's +0x36c to 3.0 and to -0.8, which FUN_100716d5 adds as (0, 1, 0) × that to the
sidestep. Nothing in GUI.dll or Gamecode.dll sends 0x11 or 0x12, so the player has no way to trigger them.

**Speeds** (FUN_1006f9eb, mode 7): reference speed 7.0; top speed `7.0 + runspeed × (1/275)`, at most 15.0, at least
1.5; force `2 × mass × speed` capped at 100000 then 10000; then `DisableFalling`. Running is `5.0 + runspeed/275`, at
most 13.0. The mode-7 branch has no test of the direction, so backwards is as fast as forwards. There is no separate
climb speed: climbing is the forward speed times the sine of the pitch.

**Leaving fly mode.** Every source of action 0x27 I found:

- FUN_100a8107 when the spell list ends (the program stops; for a worn item, presumably when it is taken off).
- FUN_10045020, called for character action 0xac in FUN_1005d61f (byte table at 0x1005f526 indexed by action - 1;
  checked against 0x70, the delete-item case).
- FUN_100450c0, called every tick for non-NPC characters from FUN_1005b55d: flying and inside an area whose record's
  +0x38 is 5 (FUN_1012a2e6) → FUN_10045020. What kind 5 is was not identified.
- FUN_10044aac: state flag 0x2000000 becoming set while in mode 7.
- A movement message carrying 0x27 through FUN_1006bd94 (no local sender found).

**Touching the ground does not end it**: no such code was found. `LeaveFlyModeTransitionAction_t` (FUN_1006e65e)
clears the 0x20000000 flag, calls `EnableFalling`, moves the body by (0, 0.1, 0), zeroes +0x36c, sets the orientation
mode back to stat 0xc5, re-enables the eight actions, and restarts the walk or run animation if moving. The character
then falls under the vehicle's ordinary gravity.

In fly mode FUN_1006c6f4 (case 7) refuses crawl 0x1b, sneak 0x1c, sit 0x1e, 0x21, 0x22, 0x25 and 0x28-0x2a.

## 4. What the server is told

- **Nothing about the mode.** The spell path calls the status directly; `SendIIRToServer` is only in
  `N3Msg_MovementChanged`. A 0x1d sent by a player would be refused by the same FUN_1006bd94 rule on the other side
  (inferred for the server). The capture's flight has no 0x1d and no 0x2b from the client.
- **Position reports as always.** 0x2b only marks the engine "turned"; `CheckMotionUpdate` (0x10018f20) then sends a
  full report 0x16 with the position and the whole rotation, pitch included. The forward / back / strafe / turn starts
  and stops go out as usual. The new client already does this (`position_update_pitched`).

## 5. What is visible

- **Animation**: action 0xb3. The `SimpleChar_t` constructor FUN_1005d0b1 sets the animation holder's slots:
  `FUN_1003c635(0x65)` run, `FUN_1003c646(0xb3)` fly, `FUN_1003c657(100)` walk, `FUN_1003c66c(0x66)` sneak.
  FUN_1006e4f8 plays slot +8 on entering; FUN_1006c5ae (the "standing" chooser) plays it again when mode is 7; and
  `ForwardStart` (FUN_1006ef8d), `ReverseStart` (FUN_1006f122) and the strafes start no animation in mode 7. So a flyer
  shows 0xb3 standing and moving alike. The new client's table names it `hover-norm` (0xb2 `idle-hover`, 0xb4
  `hover-fast`), with fallback 100 (walk) when a body has none (`appearance::action_fallback`).
- **Effect**: effect 2000 at the body, colour 0xfff0e0ff, once on entering. What effect 2000 looks like was not looked up.
- **Body**: lifted 0.5 on entering; pitched with the mouse (orientation mode 4).
- **Yalmaha**: the body is the NPC template's mesh (stat 359 from 0xcf44) at 20 % scale (stat 360 from 0xcf76), with
  the same fly mode and animation slot. Whether the template's mesh holds a pilot, and its animation names, were not
  looked up.

## 6. The new client against the original

What the new client does (`crates/mao-client/src/net.rs`, game loop near line 7504; `viewer.rs` near 1041):
`worn_flight` / `nano_flying` come from `mao_formats::item::grants_flight` (a spell 0xcf4c with stat 73 & 2) on the
worn items and running programs. When the flag turns on it adds `FLY_LIFT` 0.5, drops gravity, and moves along
`keys.pitch`, which only the right button's vertical drag changes (`viewer.rs`, `self.hud.flying && self.buttons[1]`,
clamped to `FLY_PITCH_LIMIT` 1.4). The latest commit says a forced trial climbs.

**Why the player "stays down".** By the code, the original also only rises 0.5 and stays level until the right button
pitches it up and it moves forward. The new client needs the same input and has it. I found no code defect in the fly
branch that would hold a pitched, moving flyer down; I could not run it, so that is from reading only. What makes the
new client look like nothing happened is listed below. If the player did drag up with the right button and move
forward and still did not climb, the next thing to check is whether `hud.flying` reached the viewer (`MAO_FLY_TRACE=1`
prints the pitch each second).

Differences, most important first:

1. **No hover animation.** The original shows 0xb3 `hover-norm` the instant fly mode starts and keeps it standing and
   moving. `appearance::Action` has no such action; a jetpack wearer stands, walks and runs in the air.
   Change: add the action (0xb3, fallback 100) in `mao-formats/src/appearance.rs` and choose it for a flying character
   without a vehicle body where `net.rs` picks the movement action. Verified: FUN_1005d0b1, FUN_1006e4f8, FUN_1006c5ae.
2. **Wrong signal.** Flight is taken from static data and from the wrong spell. The original switches on the server's
   spell list carrying 0xcf92 and switches back when that list ends. `spelllist.rs` lists 0xcf92 under "Not ported".
   Change: in `spelllist.rs` `fire`, give 0xcf92 an outcome (fly on / off by `run.ending`), set `own_flying` from it in
   `World::spell_outcomes` in `net.rs`, and keep it per character for others. Keep the worn-item test only as a
   fallback, keyed on 0xcf92 instead of 0xcf4c, and correct the comment on `item::grants_flight`.
   Verified: FUN_100a8107, FUN_100a7c79 / FUN_10044c71, capture #305 / #526. Not found: the server sending the list
   for a worn item, and at login or zoning.
3. **Fly speed.** The loop uses `movement_speeds` (run: 5 + rs/275 up to 13; back: 3 + rs/392.86 up to 9.1). Fly mode
   is 7 + rs/275 between 1.5 and 15, the same backwards, and for the sidestep. Change: a fly case beside
   `movement_speeds` and use it for `SPEED` / `BACK_SPEED` when `flying`. Verified: FUN_1006f9eb.
4. **Yalmaha not handled at all.** `riding` is filled from running programs only, and `item::is_vehicle_program` /
   `vehicle_template` want criteria on the body, which the Yalmaha's 0xcf44 does not have. The 0xcf76 scale is not
   read. Change: take 0xcf44 (stat 359) and 0xcf76 (stat 360) from the server's list, or from the worn item's event
   14, and draw the template's mesh at stat 360 / 100. Verified: FUN_100a7aa2, FUN_100a95c7, FUN_1005a3b1, item 96088.
5. **Ways flight ends.** Not in the new client: character action 0xac, the kind-5 area test, the ending spell list.
   Change: handle 0xac where `net.rs` reads character actions; the list's ending comes with change 2.
   Verified: FUN_1005d61f / FUN_10045020, FUN_100450c0.
6. **Entering effect.** Effect 2000 with colour 0xfff0e0ff. Verified: FUN_1006e4f8. Minor.
7. **Pitch limit and sensitivity.** Original: almost ±90°, scaled by `MouseTurnSensitivity`. New: ±1.4 rad at a fixed
   0.005 rad a pixel. Verified: N3.dll 0x10004ebd, GUI.dll FUN_1002ccbe. Minor.
8. **Leaving.** The original moves the body up 0.1 and lets it fall; it also resets the orientation mode. The new
   client resets its body and zeroes the pitch. Whether the original zeroes the pitch at once was not found. Minor.
9. **Criteria.** Operators 70 (is flying) and 112 (flying allowed on the playfield) are missing from
   `requirements.rs`; the jetpack's wear requirement uses both. Verified: FUN_100065d8 cases 0x46 and 0x70.
10. **Mode rules.** In fly mode the original refuses sit, sneak and crawl and disables the walk/run toggle. Whether the
    new client blocks them while flying was not checked.

Same in both: jump ignored while flying; the 0.5 lift; forward thrust along the pitch; no mode message to the server;
pitched rotation in the position reports.

## Scratch files

In the scratchpad: `fl_item.py` / `fl_name.py` (find item records by id / name in the database files, read only),
`fl_dump.py` (print a record's spell lists), `fl_item_290215.bin`, `fl_item_96088.bin`, `fl_item_202453.bin` (the
records), `fl_vt.py`, `fl_scan.py`, `fl_const.py`, `fl_str.py`, `fl_rtti.py`, `fl_lists.py`, `fl_1006cb58.txt`,
`fl_1005d61f.txt`, `fl_crit.txt`.
