# Audit: the client against the original's code

The user's direction (2026-10-27): go through the existing renderings, animations, maps and so on and implement them **as they
are in the decompiled and drop-in code**, not as fitted to renders or captures. This file is the ledger: each subsystem, where the
original's behaviour is written down, what the client does, and the status. Sources: `AnarchyReimagined/exports_gamecode/*.json`
(Gamecode.dll decompile; function names are stripped of their classes, so search the `decompiled` text for a symbol or a string),
`exports_displaysystem`, `exports_collision`, `exports` (the exe), and the user's Rust ports `dropin/displaysystem/src`,
`dropin/randy31/src` (the renderer: `refframe.rs`, `camera.rs`, `light.rs`, `shadow.rs`), `dropin/gamedata/src`
(`values.rs`: `EnvironmentData_t`), `dropin/collision`, `dropin/city`. Constants in `.rdata` (`_DAT_1015xxxx`) are read from the
DLL with pefile (`pe.get_data(addr - image_base, 8)`; the float or the double, whichever is sane).

Method for each item: find the original's function(s), read them, write the rule into the code with the address in a comment,
then check a render. Never the other way round.

## Done from the original (this pass)

| Subsystem | Original | Client | Status |
|---|---|---|---|
| Dynel mesh placement (doors, machines) | `VisualMesh_t::SetMesh` → `attach` (displaysystem `asyncmesh.rs`, FUN_1006d7b9): mesh frame under the visual's frame at the origin, unrotated; `RRefFrame_t::UpdateWorldMatrix` (randy31 `refframe.rs`) | `model::build_dynel` | done |
| Zoning through a door | collision with a `Door_t` (FUN_100a944a) → `StartTeleportTry` (0x10019002) → `MovementChanged(0x16)`; 30 s `TeleportTrier_t` | teleport try within 2 m of a door | done; the warps need a live check |
| Movement action codes | transition table FUN_1006cb58 (0xf jump start, 0x10 jump stop, 0x16 full report, 0x18/0x19 walk/run mode) | `n3::movement` | done |
| Movement speeds | `CharVehicle_t` FUN_1006f9eb / FUN_1006f2fc | `movement_speeds` | done |
| Movement animation rate | FUN_1007009f, `Setupf/animcalibration.txt` | `movement_rate`, `Motion::clock` | done |
| Which movement animation | FUN_1006c370 over the table FUN_100c0720 | `Action::{WalkBack, RunBack, StrafeLeft, StrafeRight}` | done (walk/run mode for others still by speed) |
| Animation cross-fade | `VisualCATMesh_t::SetAnimation` (displaysystem `catloader.rs`): fade over the cycle's remaining time, 200–300 ms | `PoseState::fade` | done |
| Body scale | `n3VisualDynel_t::SetBodyScale(stat 360 / 100)` (FUN_10058800, FUN_1005c3ed, FUN_1008746a) | `parts()` world scale | done for the own character; the others' stat is not received |
| Texture alpha as light | the alpha-replicate texture stages (`RDeltaState`) | `Material::alpha_lights` | done |
| Buff pulses | `_GfxControlBPHFSM_t`, `gfxtweak.bin` | `pulses.rs`, `effects.rs` | done earlier |

## To do, in order of what the player sees

1. **Environment: sky, fog, ambient, sun, time of day** — the largest visible difference. The client has a fixed sun
   (0.4, −0.5, 0.8), fixed ambient/sun colours in `mesh.wgsl`, a two-colour sky gradient and a fog by view distance.
   The original: `PlayfieldAnarchy_t` holds an `Environment_t` (constructor FUN_100be0bb, 0x90 words) made from the playfield's
   `EnvironmentData_t` (26 bytes, read by `GameData::operator>>` — `dropin/gamedata/src/values.rs environment_read`; the
   `RDBPlayfield_t` blob carries it after the districts, FUN_10124692: `... >> int >> int >> flag; if flag { EnvironmentData }`;
   districts (`AreaData_t`) carry their own, FUN_100bd6d9/FUN_100bd81a copy the 26 bytes into the environment: bytes 0–7 as words
   at +0x20..+0x3c, 8–18 as bytes at +0x44..+0x4e, 19–25 as bytes at +0..+6, and the area's +0x48..+0x50 floats). The per-frame
   update is FUN_100becbe (12.7 KB): `AddAmbientLight` from the state table (+0x2c: 32 floats; +0x3c..+0x44 ambient, +8 daylight,
   [0] a fog/intensity value, +0x14/+0x18/+0x24 sun terms), `GameTime_t` (`GetCurrentRealTime`, `GetDuskEnd`, `GetDayIntensity`),
   a weather `VisualFader_t`, the sky colour setters into `VisualEnvFX_t`'s frame block (+0x7e8: +0xd8..+0xec two colours,
   +0xf0/+0xf4/+0x114 factors, +0x1134 = 1 − daylight/`_DAT_10169510`), fog through `VisualFog_t::AddFog` (displaysystem
   `fog.rs`: the strongest fog takes over, weaker ones blend by weight; `AddClipPlanes`; `AddFogLightIntensity`), the suns
   (`_GfxControlSuns_t`, `fxsun.rs`), the skyline (`fxskyline.rs`) and clouds. Work: (a) parse `EnvironmentData` from the
   playfield object into `mao_formats::playfield`; (b) port `Environment_t`'s state computation (FUN_100bd555 the time-of-day
   driver, FUN_100bf4f9, FUN_100bff68 the sky colours, FUN_100b98fe); (c) feed ambient, sun direction/colour, fog and sky into
   `Scene`. Expect several sessions.
2. **Terrain texturing and lighting** — `AnarchyGround_t` (displaysystem `anarchyground.rs`, `rdbground.rs`, `rdbtile.rs`,
   `grounddata.rs`): per-tile textures with a shadow texture pass, a ground-light pass, ROAM tessellation (the client draws
   the full grid: fine). Compare the texture blending and the lighting fix byte.
3. **Water** — `liquid.rs`, `fxliquid.rs`, `water.rs`: where the surfaces are (the room records carry water surfaces; the
   outdoor liquid is in the ground data) and how they look. The client's placement is a guess (`CLIENT_UI.md` line 67).
4. **Camera** — Gamecode's camera (`n3Camera_t`, the camera attractors in the room records, `VisualCamera_t`
   displaysystem `camera.rs`, randy31 `camera.rs` for the frustum/FOV): distance limits, pitch, collision with the world,
   the three third-person modes. The client's are invented.
5. **Automap / minimap** — the original's map textures and the `PlanetMap` window; the client's fog-of-war map is invented.
6. **Rooms (shops, dungeons)** — light maps (`n3Room_t`), decals (the blue tile layer), wall textures; `room.rs` in
   displaysystem.
7. **Shadows** — `gfxshadow.rs` (the characters' blob/projected shadows) and randy31 `shadow.rs`; the client uses a shadow
   map (a backend choice the user allowed; the look should match).
8. **Character rendering** — `catrun.rs` animation layers and priorities (several animations at once: upper body / lower
   body), `catmesh.rs` skinning, attractors; the client plays one animation at a time.
9. **Statel LOD and culling** — `lodmanager.rs`, `fxoccluder.rs`; perf, not look.
10. **Effects** — the rest of `gfxtweak.bin`'s kinds (`gfx*.rs` in displaysystem: beams, cones, sprites, shields): only the
    buff pulses are done.
