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
| Water | `RDBPlayfieldAnarchy_t::ReadBlob` (FUN_10124692: the water polygons after N3's zones, fanned or indexed into triangles with `flags & 0x1f` the liquid type) → FUN_100b84b9 → `VisualWater_t::Create` (displaysystem `liquid.rs`, `fxliquid.rs`) | `playfield::anarchy`, `Water::triangles`, the water-mesh pipeline | done for placement and type colours; the original's liquid passes (reflection, falls, the wide fifth layer) are not ported |
| Playfield environment | the same reader: two words, a size and the `EnvironmentData_t` at the end of the `0xf4241` object (found this pass; the earlier note that none existed was wrong) | `areas::blend_with_weather(.., base)` | done: the base table the areas blend into, with its own weather |
| Weather | the R250 generator (FUN_1013de47 / FUN_1013dec9), the schedule FUN_100b1821 / FUN_100b1277, the roll FUN_100bcec6, the fogs of the weather (FUN_100becbe), mist (FUN_100b9da8, colours FUN_100b98fe / FUN_100b9775) and precipitations (FUN_100b9ba5, FUN_100b9b10's colours) gathered as `VisualFog_t` does | `mao_formats::weather`, `environment.rs` (the fog gather) | done for what reaches the fog and the sky's dimming; the rain/snow/dust sprites (`VisualRain_t`), clouds (`VisualEnvFX_t` +0x114), wind, lightning and thunder are not drawn |
| Rooms (shops, dungeons) | N3.dll: `n3Tilemap_t`'s constructor FUN_100173f8 / FUN_100169ab (the cell words from the `DCGA`/`IRHA`/`TAHA` images, the wall patterns FUN_10017306 over `.data` 0x1005bce0, the piece lookup FUN_10016967 in `RDBTilemap_t`'s type tables), `n3GroundRenderer_t::CreateDungeonRoom` FUN_10008563 → `DungeonMesh_t` FUN_10001a91 (which pieces) / FUN_10001c3b (laid on the heights with the cubic corrections FUN_100023ad / FUN_10002479), `VisualTile_t` (type 0xf695d, displaysystem `rdbtile.rs`), the corner rule FUN_1001769d, the floor height FUN_10010db5 (`n3Room_t` +0x38 = −1), the room reader FUN_10012803 (water surfaces, light map) | `mao_formats::dungeon`, `rooms.rs` (`Grid`, `room_parts`, `water`), `mao-db dungeon <pf>` | done: the walls, corners, doorways and ceilings as the original's pieces on the original's floor (the floor a unit above the room's position, on the samples before each cell, cells laid from the centre by the odd width); the rooms' water. Not: the vertex welding FUN_10001645 and so the light map's vertex colours, the `CHGA` (old) maps' branch |
| Character shadows | `GfxVisualSimpleShadow_c` (displaysystem `gfxshadow.rs`: FUN_1001f0e9, `Update` FUN_1001e556, the vertex gather FUN_1001e75d, the draw FUN_1001e90e) under Gamecode's shadow control (FUN_100ed21e per CAT mesh, colour `0x5f000000` / `0x2f000000` in a dungeon; FUN_100ed439 per frame: the ground light's direction from `VisualEnvFX_t`, the camera, the ground point and normal under the character; FUN_100ed0c0 lays the points on the heightmap + 0.02) | `blobshadow.rs`, `Scene::blobs` and the blob pipeline in `mao-render` | done: the black translucent disc with its fading rim, stretched along the light, laid on the terrain, gone past ~50 units; the characters and machines no longer throw shadow-map shadows (the original has none). The playfield's own shadow map stays (the allowed backend choice; see item 7) |
| Animation layers | `VisualCATMesh_t`'s animation list (displaysystem `catloader.rs`: `SetAnimation`, `apply_animation` FUN_10074180 and its masking FUN_100729f9; `visualcatmesh.rs`: `SetTime` / `set_range` FUN_10072cc7, `anim_time`; `catrun.rs`: `advance` FUN_10074228, `run_anim` FUN_10072fde, `blend_weight` FUN_10072562) over randy31's `CATKeyframeAnim_t` (tracks enabled by layer) and `CATAnimBlend_t` (per-track mode: first, second or slerped); the callers: FUN_1006c370 (movement at priority 0 claiming layer 2), FUN_10010929 (the range from the notes), FUN_10010a46 (a social once, claiming nothing), FUN_10010dee / FUN_10010cc2 (an attack at priority −2, claiming `~table & 3`), the table FUN_100c0720 (layer 2 for `wield` and the `op1h`/`op2h` actions, 3 for the rest) | `mao_formats::animstack` (`Stack`), `Characters::drive` in `net.rs` | done: the list by priority, the fade in over 200–300 ms and out again for a counted one, the masking by claims, the chain of blends, the time mapping by direction; every player animation tags its tracks layer 1 or 2 (the root both), so a social plays over moving legs as the original's `local_14 = 2` leaves it. Not: attacks (the client plays none yet), the crowd phase offset the client had |
| Statel level of detail | `LODManager::Process` (displaysystem `lodmanager.rs`, its globals at 0x100ae1a4: cull 2500, far 6000, near 4000, 20 meshes a frame) over the statels' `VisualMesh_t`: hidden beyond 50 radii (`distance² / radius² > 2500`), switched to the low-detail mesh 0xf696a of the same instance beyond `distance² − radius² > 6000`, back to 0xf6951 below 4000 | `Model::lod_groups` (`add_props` loads the 0xf696a twin of each statel mesh: 668 exist for 11228 meshes), `Lod` in `mao-render` decides per instance per frame | done; the 20-a-frame cadence is not kept (every mesh is decided every frame), the radius is the mesh's world box's half diagonal rather than the original's bounding volume |
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
2. **Terrain texturing and lighting** — read this pass: `AnarchyGround_t` (`anarchyground.rs`) gives every ground vertex
   an *up* normal (`put_vertex`), so the original's terrain has no slope shading of its own; its relief came from a
   per-playfield set of six 4-bit shadow PNGs (resource type 0xf4247, blended across the day's windows by FUN_100b7fca
   with the palette darkened by the cloud cover, `SetShadowMap`) — and **this install has no 0xf4247 objects**, so the
   original here drew flat-lit ground plus the FXS ground-light projection (`GroundTextureProjection`). The per-tile
   textures turned by the tile's top bits match the client's; the original also averages far patches to one texel.
   The client keeps its slope lighting and shadow map rather than flatten to a look that depends on FXS data it lacks.
3. **Water** — ported (see the table): the playfield's own triangles with their liquid types. Left: the original's
   passes (`fxliquid.rs`: reflection, falling water strips, the fifth layer on the wide grid). The rooms' water surfaces
   are read and placed like the rooms' statels (item 6); no shop has any, so a dungeon with water should be looked at.
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
6. **Rooms (shops, dungeons)** — ported (see the table; N3.dll's dumps are in the session scratchpad noted in memory, the
   constants read from the DLL: 0x1003c968 = 2.0, 0x1003c888 = 4.0, 0x1003c950 = 0.25, 0x1003ce48 = −1.0, 0x1003d358 = π/2).
   What was found: a room's floor is the tile-map's height surface (no piece), and every cell whose flags name an empty
   neighbour gets a piece from its type's table — one wall (0x01), a corner (0x03), four walls (0x0f), the door variants
   (0x11, 0x31 …); a piece holds up to six tiles, each textured by the cell's layer of the same number (`XTHA` has six layers
   here: floor, ceiling, the wall's two halves, …). The pieces are 8 units tall with a ceiling over the wall cells; the inner
   cells have no ceiling, so the sky shows (the original drew interiors against black — the client's sky should go dark in
   a room). Left: the welding of coinciding vertices (FUN_10001645) and with it the light map (one 565 colour per welded
   vertex, zlib-packed in the room record; `DepackLightmap` 0x10010eb7, applied by FUN_100082a7) — the shading of the
   walls; the old `CHGA` tile-maps' branch of FUN_100169ab (no shop uses one); the room camera attractors (item 4).
7. **Shadows** — the characters' blobs are ported (see the table). What the original has: only `GfxVisualSimpleShadow_c`
   under CAT meshes; randy31's `RShadow` (planar projected shadows, `shadow.rs`) is made from mesh archives only and no
   statel archive here carries one; the ground's own shadows were the 0xf4247 maps (absent, item 2). So the original showed
   no cast shadows from buildings or props. The client's sun shadow map still darkens the ground under the playfield's
   statels — the allowed backend choice, but it is a look the original did not have; `Renderer::shadows = false` gives the
   original's. Left: the light's direction is the client's fixed sun, not the FXS `GroundLightDirection` (item 1); the
   ground under a character is the heightmap only (the original asks the collision, so a blob on a bridge or a statel's
   floor lies on it); in a dungeon the projection is taken as vertical onto the floor (FUN_100ed439's dungeon branch passes
   an upward light and the room's floor query, whose normal sense was not confirmed).
8. **Character rendering** — the animation layers are ported (see the table): a character's animations are a stack
   (`mao_formats::animstack`) kept and blended as `VisualCATMesh_t` keeps its list. Left: attack animations (FUN_10010dee
   at priority −2 on the layer the table leaves, the animation named `<sex>_<action>_01_01.ani` by FUN_10010cc2 — the
   client has no attack event to drive them from yet), `wield` and the `op1h`/`op2h` actions, the camera attractor's
   smoothing; `catmesh.rs` skinning and attractors were already as the original's.
9. **Statel LOD and culling** — the LOD manager is ported (see the table; `MAO_NO_LOD=1` draws every full mesh to
   compare). It does change the look a little: small props (a 1-unit bin) are gone 50 units out, as in the original, and the
   far meshes of the 668 statels that have a low-detail twin are the coarser ones. Left: the height-map occluder
   (`HMOccluder_t`, randy31 `hmocc_data.rs`; `fxoccluder.rs` is only its debug view) — pure culling, no look; the CAT
   meshes' own level by distance (`run_function`: 0 / 1 / 3 / 6 at 16, 32 and 48 units, `RCATMesh_t` +0x21c), which thins the
   skinning, not the look.
10. **Effects** — mapped this pass, not ported. `_EffectHandler_t::CreateGfxControl` (Gamecode FUN_100d0656) makes a control
    class per template kind; the classes were resolved through each constructor's vtable, and `mao-client <game> gfx-usage`
    counts, over the 10964 nano programs, which kinds their cast (stat 428), hit (361), running (413), tracer (419), gather
    (366) and impact (414) effects name. What the players see most, and what the client draws for it:

    | kind | class | nanos naming it (root) | client |
    |---|---|---|---|
    | 2007 | `_GfxControlMeta_t` (a bundle of up to four) | 4485 hits, 3932 casts, 3020 running, 5405 tracers, 9181 gathers | shows its first child with a look |
    | 2004 | `_GfxControlStars_t` | 3534 hits, 2760 tracers | modes 3, 4, 6, 7, 8, 10, 11 and 19 (81% of the 877 templates) ported as `gfx::Stars` / `effects::stars_particles` (3: a swirl in a sphere; 4: thrown from the point, slowed and falling; 7/8: a ring of all 128 swelling word 31 times; 10: standing on a sphere, living word 28 s, one channel each when word 30 is set; 11 as 6): 128 billboard stars of material word 9, each a random colour between words 18–21 and 22–25 (FUN_10108663), a random late frame, sized by word 28 along a parabola over word 30 ms, born within word 29 of the attach point — mode 6 on a circle, rising and swirling by word 31; mode 19 standing in a sphere at the hit. The other 21 modes (22 next in use, 37 templates: stars along the hit location's edges, tables at 0x102ea8e4) still show the kind-1001 child's spiral. The Ghidra text of the run function is mangled (register reuse), so the two modes follow its shape, not its arithmetic word for word |
    | 1010 | the cast effect (six emitter parts) | 5237 casts | the emitters' particles |
    | 1001 | `_GfxControlBPHFSM_t` | 2255 running | done (the buff pulses) |
    | 3004 | `_GfxControlSequencer_t` | 511 running | shows its child |
    | 1018 | `_GfxControlSparks_t` | 686 hits | ported as `gfx::Sparks` / `effects::sparks_particles`: FUN_100f1758 reads words 9 (material), 12–15 (width and height, start to end), 16–23 (the two colours), 24 (the gate), 25–30 (azimuth, elevation and speed ranges), 31 (the count), 32 (the speed scale), 34–35 (the life range), 40 (gravity); FUN_100f1bbe throws each spark through `GfxVisualSprite2Type0::NewSprite`, FUN_100f24c8 throws word 31 at once and up to fifteen more over the gated frames. Not: the push by the handler's force vector (word 38's curve, `ProcessSprites` kinds 1–6), the alpha-blend of the 0x800 flag (the client's particles add) |
    | 2001 / 2002 | (not made by the factory) | 282 hits / 611 tracers | a flash / an orb (the client's own) |
    | 2005 / 2006 | `_GfxControlSuns_t` / `_GfxControlElectra_t` | 188 running, 251 tracers / 138 hits | as spirals |
    | 3003 | `_GfxControlShield_t` | 232 running, 154 hits | a glow under the feet |
    | 3001 | `_GfxControlDeformer_t` (FUN_100d842e; FUN_100d823f the template, FUN_100d7caa the run, FUN_100d776f the vertex callback on the target's `CATRender_t`) | 160 hits | mode 1 (the three hit templates) ported as `gfx::Deform` / `effects::deform_parts`: the target's skinned vertices pushed along their normals by `amplitude × sin² × strength`, the strength ramped in over word 12 and out over word 13 before the duration (word 8) ends; `World::deforms` holds each character's live hit. Not: mode 0 (pulses round named bones, 31101) and mode 4 (a second body mesh faded over, 12276), and the callback's two phase constants on the vertex's place, which the decompile does not show legibly (the ripple runs up the body instead) |
    | 1024 | `_GfxControlTracer4_t` (made by `CreateGfxControlTracer` FUN_100d176c, which also makes 1013 → FUN_100fe22f, 1019 → FUN_100fec4e, 1021 → FUN_100ff48e, 1022 → FUN_100ff933, 1025 → FUN_10100b0d, 1026 → FUN_101018ef, 1027 → FUN_101021eb, 3026 → FUN_10114e83; FUN_100ffcac reads the words, FUN_100ffd67 makes three `GfxVisualCord4`s of twenty links, FUN_100ffa7b winds them) | 124 tracers | ported as `gfx::Cords` / `effects::cords_particles`: three cords wound round the line from caster to target, the links' width word 11, the winding's radius word 12 under a half-sine envelope, the colour words 13–16, the phase at 12.56 rad/s stepping 0.314 a link, a quarter of the links jittered. Not: the cord ribbon itself (each link is a sprite of its width), the locator's travel (word 10: the original's links ride a locator moving at 100 a second; here they span the whole line for the flight) |
    | 3038 | `GfxControlVolGrid_t` (FUN_10116562; FUN_10115f8c the words, FUN_101166f2 each curve, FUN_1011689d / FUN_1011678b sample them, FUN_10116292 the run; displaysystem `gfxvolgrid.rs`, FUN_1002f86d builds the slices, FUN_1002f69e fades them) | 31 hits, 27 casts, 24 tracers, 17 running | ported as `gfx::VolGrid` / `effects::volgrid_particles`: the 13 templates' slice counts (words 14–16), spin (13) and five curves (from word 17: height, width, depth, middle and top colour, linear between keys, the last value held) sampled at the time's fraction of the duration (word 8); `across` slices across x and `deep` across z standing the box's height, their bottoms half the width out and their tops half the depth (the original's asymmetry kept), the middle colour below and the top colour above, a row of the texture across each; `up` square slices by height in the blended colour; flag 0x1000's random turn, flag 0x4000's fade by facing (on the card: `Particle::quad`, a quad laid in the world, new in the particle shader). Not: the fan's gradient within a slice (two flat-coloured quads stand in), the locator's own rotation, the alpha blend (the client adds), flag 0x400's turned texture (no template sets it) |
    | 1009 | `_GfxControlSmoke_t` (FUN_100f12bf; FUN_100f0b6b the words, FUN_10106be2 the emitter's offset and turn, FUN_100f0cd4 the `GfxVisualSprite2Type0`, FUN_100f04bd the run) | 63 running | ported as `gfx::Smoke` / `effects::smoke_particles`: the 17 templates' rate (word 10, sprites until `rate × elapsed` exist while `elapsed < duration − life`), birth disc (words 11–12 out, 31–32 up, a random turn and a random quarter turn's cosine), the drift `(0.3 x, 0.3 z, −1) × word 13 / life` in the emitter's frame (the original's own component order; its turn of word 4, π/2 in most, points −z up), life word 26 (×1 + 0.3 random), widths 14–15 and heights 16–17 (×1 + 0.3 random), colours 18–21 to 22–25, the sheet's frames over the life; push kind 5's rise of 1.5 a second and kind 7's random flight. Not: the other kinds' push by the handler's force (words 27, 28, 30: the client keeps no force), the quads' spin, the alpha blend |
    | 1012 | `_GfxControlSprite_t` (FUN_100f6b86; FUN_100f6369 the words, FUN_100f6483 picks `GfxVisualSprite2` or `GfxVisualSprite3` by word 10's low bits, FUN_100f64a7 makes it, FUN_100f6d2b the run; displaysystem `gfxsprites.rs`) | 27 running | ported as `gfx::Sprite` / `effects::sprite_particles`: the 16 templates' one quad at the attach point plus words 1–3, cycling over word 23 for word 24 cycles (flag 0x40) or the duration; flag 0x100's width 11→12 and height 13→14 with 0x800's `word 25 × sin(2π word 26 t)` swell, 0x80's colour 15–18 → 19–22, 0x20's frame across the sheet, 0x4 clear's random spot of the texture each frame; a billboard for Sprite2, and for Sprite3 a quad standing on its height turned to face the camera (flag 0x2000; new in the particle shader: a `Particle` with `up` alone). Not: a standing quad set in the locator's own frame (flags 0x8/0x10 without 0x2000: 7100, it faces the camera instead), the height of a billboard (the client's billboards are square: the width), the alpha blend of flag 0x1000 |
    | 3028 | `GfxControlBParticle2_t` (`gfxbparticle.rs`) | 21 hits | nothing |

    The other classes (`Fire` 1004, `Flare` 1005/1006, `Cord` 1003, `VulcanRocks` 1029, `Highlight` 2011, `Notum` 2015, `Splash`
    3002, `SkyFlash` 3006, `CrazyCone` 3009, `Mesh` 3010, `GroundRing` 3012, `Delay` 3013, `Vein` 3014, `LavaBall` 3015,
    `Bubble` 3016, `GlobalSmoke` 3017, `SkyRise` 3018, `Trail` 3019, `TParticle` 3020, `Beam` 3022, `EnergyBall` 3023,
    `BParticle` 3024, `EffectMesh` 3025, `MParticle` 3027, `Scatter` 3029, `GroundGrid` 3030, `TParticle2` 3031,
    `GroundShake` 3032, `Spiral2` 3033, `Shield2` 3034, `AParticle` 3035, `Toggle` 3036, `Trail2` 3039) are named by a handful
    of programs each. Each port is the size of the camera's: the control's constructor and run function in Gamecode (the
    template words it reads) and the visual it drives in displaysystem (`gfx*.rs`). Order of value: Stars 2004, Sparks 1018,
    Deformer 3001, the 1024 tracer, VolGrid 3038, Smoke 1009, Sprite 1012.
