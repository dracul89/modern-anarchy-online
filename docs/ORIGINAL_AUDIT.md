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
| Camera | `n3Camera_t` (N3.dll: FUN_10022345 per frame, FUN_1002118c turn/zoom, FUN_10020e0c reset; `CameraVehicle_t` / `CameraVehicleFixedThird_t` steering, `Vehicle_t::Run` / `SteeringArrive` in Vehicle.dll) | `camera.rs` (`SteeringCamera`), 90° field of view in `mao-render` | done: the three third-person modes, first person, key/mouse turn and zoom with the 0.8–25 limits, reset; not the obstacle sensors, the room attractors, nor a line of sight through statels (terrain only) |
| Planet map | GUI.dll `PlanetMapView_c`: FUN_1004d1cf (the index), FUN_1004c6cb (the coordinates), FUN_1004bedd (world → pixel: origin 31022/24880, extent 18751/24115, `MapRect`, the playfield's scales) | `mao_formats::planetmap`, `automap.rs` draws it as the corner map and the overlay | done for Rubi-Ka (the Shadowlands set is chosen by the expansion flag, not ported); interiors keep the client's own map |
| Area environments (light, fog) | `Environment_t` FUN_100be0bb / FUN_100becbe / FUN_100beacd / FUN_100bf96e over `AreaData_t`'s `EnvironmentData_t` (the `0xf425d` object) | `mao_formats::areas` (`blend_with_weather`), `environment.rs`, `Scene::{ambient, area_fog}` | done |
| Weather | the R250 generator (FUN_1013de47 / FUN_1013dec9), the schedule FUN_100b1821 / FUN_100b1277, the roll FUN_100bcec6, the fogs of the weather (FUN_100becbe), mist (FUN_100b9da8, colours FUN_100b98fe / FUN_100b9775) and precipitations (FUN_100b9ba5, FUN_100b9b10's colours) gathered as `VisualFog_t` does | `mao_formats::weather`, `environment.rs` (the fog gather) | done for what reaches the fog and the sky's dimming; the rain/snow/dust sprites (`VisualRain_t`), clouds (`VisualEnvFX_t` +0x114), wind, lightning and thunder are not drawn |
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
     14–18 the rest. Then a second table is cross-faded (FUN_100bf96e) over the state's fade time. The random numbers are an
     R250 generator: FUN_1013de47 copies the 250-word table at `.rdata` 0x101727f8 and adds the seed to the word at
     `seed & 0xff`; FUN_1013dec9 steps it (lags 103 and 147) and returns the word × 2⁻³² (`_DAT_1016bcf0`), FUN_1013de89 the
     raw word. Each schedule entry (0x20 bytes: start time, its own seed, 16 floats and 8 words drawn from it, walked
     cyclically by FUN_100b138c / FUN_100b13b6) is independent, so the weather is reproducible from the area's bytes and the
     day count. **Ported** (`mao_formats::weather`, the table's effects in `environment.rs`); note the seeding adds the
     seed to one table word, so a neighbouring day's weather is all but the same and it is the area's bytes that vary it.
     Left: the rain, snow and dust sprites, the cloud layer, wind, thunder and lightning (the camera shake, FUN_100bd902).
2. **Terrain texturing and lighting** — `AnarchyGround_t` (displaysystem `anarchyground.rs`, `rdbground.rs`, `rdbtile.rs`,
   `grounddata.rs`): per-tile textures with a shadow texture pass, a ground-light pass, ROAM tessellation (the client draws
   the full grid: fine). Compare the texture blending and the lighting fix byte.
3. **Water** — `liquid.rs`, `fxliquid.rs`, `water.rs`: where the surfaces are (the room records carry water surfaces; the
   outdoor liquid is in the ground data) and how they look. The client's placement is a guess (`CLIENT_UI.md` line 67).
4. **Camera** — ported (see the table and `camera.rs`'s header; N3.dll and Vehicle.dll were decompiled for it, the dumps
   are in the session scratchpad noted in memory). Left: the free camera's sensors (`CalculateSensorSteerDir`: flexes of
   0.5, 2, 8 and 32 units up, down, left, right, forward and back until one is seen), the line of sight against statels
   (the viewer has only the terrain; the collision lives on the network thread), the look target from the CAT mesh's
   `Attractor31_camera` (smoothed by FUN_10020bdb), the room camera attractors (`n3Zone_t::GetCameraAttractorList`),
   `MouseTurnSensitivity` / `ZoomSpeed` / `MouseLookInverted` as settings, and the mode-3 eased zoom (+0x204, 3 units a
   second) which nothing in the decompile sets.
5. **Automap / minimap** — ported (see the table): the corner map and the overlay show the planet map's biggest level,
   placed by FUN_1004bedd's formula, for any playfield in `MapCoordinates.xml`; the client's fog-of-war map stays for the
   rest. Left: the `PlanetMap` window itself (its zoom levels, centre-on-quest), the Shadowlands/Dovves sets. The data: `cd_image/textures/PlanetMap/normal/PlanetMapIndexNormal.txt` (`Name`, `Type Rubika`, `CoordsFile`,
   then per zoom level `File <bin>`, `TextureSize 128`, `Size w h`, `Tiles cols rows`, `MapRect l t r b`, and a `FilePos`
   per tile: a 128×128 PNG at that offset in `PlanetMapGfxNormal.bin`; levels 384×640 of 3×5 tiles, 1024×1536 of 8×12, …),
   `MapCoordinates.xml` (`<Playfield id x xscale z zscale>`: where each playfield's origin sits on the map, e.g. Old Athen
   540 at 39621, 40670), the Shadowlands and Dovves sets beside it. GUI.dll's `PlanetMapView_c` (FUN_1004d1cf parses the
   index) draws it with `PlanetMap_ZoomIn/Out`, `PlanetMap_CentreOnChar`, `PlanetMap_CentreOnQuest`; the in-game minimap
   is the same map at the character. Port: read the index and tiles, find the world→pixel scale in `PlanetMapView_c`.
6. **Rooms (shops, dungeons)** — light maps (`n3Room_t`), decals (the blue tile layer), wall textures; `room.rs` in
   displaysystem.
7. **Shadows** — `gfxshadow.rs` (the characters' blob/projected shadows) and randy31 `shadow.rs`; the client uses a shadow
   map (a backend choice the user allowed; the look should match).
8. **Character rendering** — `catrun.rs` animation layers and priorities (several animations at once: upper body / lower
   body), `catmesh.rs` skinning, attractors; the client plays one animation at a time.
9. **Statel LOD and culling** — `lodmanager.rs`, `fxoccluder.rs`; perf, not look.
10. **Effects** — the rest of `gfxtweak.bin`'s kinds (`gfx*.rs` in displaysystem: beams, cones, sprites, shields): only the
    buff pulses are done.
