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
| Game clock, day intensity | `GameTime_t` (0x1000afdb, `RunFunction` 0x1000b27e, `GetDayIntensity` 0x1000af23; the `GameTime` message 0x5f52412e → `Update` 0x1000b590) | `gametime.rs`, `n3::GAME_TIME` | done; the server here never sends the message, so the real clock stands in at the game's pace |
| Area environments (light, fog) | `Environment_t` FUN_100be0bb / FUN_100becbe / FUN_100beacd / FUN_100bf96e over `AreaData_t`'s `EnvironmentData_t` (the `0xf425d` object) | `mao_formats::areas` (`blend`), `Scene::{ambient, area_fog}` | done for clear weather; the weather roll (FUN_100bcec6, the schedule FUN_100b1821) is not ported |
| Buff pulses | `_GfxControlBPHFSM_t`, `gfxtweak.bin` | `pulses.rs`, `effects.rs` | done earlier |

## To do, in order of what the player sees

1. **Environment: sky, sun, weather** — what is left after the area environments and the clock (above). Read this pass:
   - **Where the look comes from.** `Environment_t`'s state table (32 floats, FUN_100bf907) holds the *weather* ([0] fog
     from the first weather, [1] fog density, [2] cloud cover, [3]–[6] the four precipitations, [7] wind, [8] darkness, [9]–[0xd]
     thunder/lightning rates) and the *area* values ([0xf]–[0x11] ambient, [0x12]–[0x15] fog colour and density, [0x1b]–[0x1d]
     the centre, [0x16] the reach). Per frame (FUN_100becbe) it adds the ambient (`AddAmbientLight`, a per-channel max with
     whatever else adds one), the weather fog (`AddFog([1]×0.7 grey, [1])`), the area fog (`AddFogLightIntensity(min(day,
     1 − [8]×0.8) ≥ 0.5)` then `AddFog(colour, density)`), the rain/snow visuals (FUN_100b9da8, FUN_100b9ba5 with `VisualRain_t`),
     the wind (FUN_100bff68, FUN_100bfc80), and four numbers into `VisualEnvFX_t`'s frame block (+0xf0 the precipitation max,
     +0xf4 [0], +0x114 [2], +0x1134 = 1 − [8]/2.333; +0xd8..+0xec the wind). The lightning (`+0x54`, FUN_100bd902) shakes the
     camera (`n3Camera_t::UpdateTargetEye`).
   - **The sky, the sun and the day's colours are not in Gamecode at all.** They are FXS scripts ("Fanatic", `FXS.dll`):
     FUN_100bd930 registers `SM_Sandy_Env_BackgroundNight/Dawn/Day/Dusk` with time windows (dawn/dusk from `GameTime_t`, a
     300 s fade, FUN_100ba8d7) and FUN_100ba779 fades them by the clock; DisplaySystem's `EnvironmentLightObject_t`
     (`fxvisual.rs` `environment_frame`) reads the variables `AmbientLightCurrent`, `GroundLightDirection`, `GroundLightCurrent`,
     `GroundLightColor`, `CloudLight…`, `WaterLight…`, `SpaceLight…`, the three texture projections, and sets the ambient
     (`AddAmbientLight`, so the area's is a floor under it) and the one directional light (`ambient.rs set_light`). The sky
     backgrounds and suns are FXS visuals too (`fxsun.rs`, `fxskyline.rs`, `fxsinglecloud.rs`). **Porting the sky means
     porting the FXS object format and the scripts' time curves** — find the FXS objects in the database (the names above
     are not in the instance index under the types tried) and read `FXS.dll`'s loader. Until then the client keeps its own
     sun and sky, scaled by the day's intensity (`Scene::daylight`, a fifth left at night).
   - **Weather** is a deterministic schedule per area and day: FUN_100b1821 builds it from three seeds (FUN_100bce27: the
     sum of the environment bytes plus `GameTime_t`+0x4c the day count, plus 1/0/−1), a period of 19440 s, states of 120–1200
     s, 16 entries; FUN_100bd555 steps it, FUN_100bcec6 rolls a state: the weather bytes 0–7 (each ×2+100 when above 1)
     weight *clear* (byte 3), *fog* (0), *mist* (1, ×0.65), *clouds* (2), and the four precipitations (4–7, with a 0.5
     intensity); bytes 8–9 the wind range (×2.75), 10–11 the darkness range, 12–13 the thunder/lightning chances (×10),
     14–18 the rest. Then a second table is cross-faded (FUN_100bf96e) over the state's fade time. Port this next for the
     fog/rain/clouds to appear where the data has them (Mort, the swamps, Tir County…).
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
