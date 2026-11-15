# Player city buildings in the original client (PlayfieldAllCities, 0x59210126)

Read-only research, 2026-11-15. Nothing in either repository was changed. No game was run, no network used.

**Short answer.** The city code is not in Gamecode.dll but in `city.dll` (never decompiled before today; a Rust port of it
already exists at `AnarchyReimagined/dropin/city`). The message is a plain list of 25-byte house records. Each record names a
**house template**, a resource database record of type **0xf4268** (`RDBHouseTemplate_t`), which lists the building's meshes,
doors, guards and its collision tree. The client draws a building as plain `VisualMesh_t` objects (not dynels), one per
"visual" of the template. The sample bytes fit exactly: 80 houses, the first at (868, 5, 1224) turned 90 degrees.

Addresses: `G:` = Gamecode.dll, `C:` = city.dll (image base 0x10000000 both). Where the drop-in port is cited it is
`AnarchyReimagined/dropin/city/src/<file>`; I checked each cited port function against the city.dll decompile.

New scratch material (all in the scratchpad folder, next to this report):

- `city.c`: full Ghidra decompile of city.dll (2121 functions); `cf.py <addr>...` prints functions, `cf.py -g <regex>` greps.
  city.dll is now also imported in the `ghidra_cam/camproj` project.
- `rdbscan.py <type hex> [outdir]`: brute-force lists/saves database records of a type; `rdbhas.py <type> <inst>...`.
- `cityrec/`: the 85 house template records (`f4268_<inst>.bin`) and 20 playfield city records (`f4267_<pf>.bin`).
- `housetpl.py [inst...]`: parses the templates; `citydata.py <pf>`: parses a playfield's city record; `pfdyn.py <pf>`:
  the playfield's static dynel table.
- `cities_templates_table.txt`: the table in the appendix.

---

## 1. Wire layout

### 1.1 The message class

`PlayfieldAllCitiesIIR_t`, vtable 0x1017227c (G). Slot **+0x1c is the reader** (`G:FUN_101304f8`), slot **+0x8 is the
handler** (`G:FUN_101304d5`). (Not +0x8 reader / +0xc handler: +0xc is the shared 0x1013231a. `AOTransportSignalIIR_c` has the
same shape: +0x1c reader 0x1012faab, +0x8 handler 0x1012fbc1.) The constructors (`G:FUN_1013047f`, `G:FUN_10130556`) make a
`BinaryStream(0x800)` at object +0x18.

Reader `G:FUN_101304f8`:

| bytes | type | meaning |
|---|---|---|
| 1 | u8 | the N3 message's lead byte (0x01 in the sample), consumed before this reader. **Guess** as to who consumes it; the Rust tower and city readers already skip it and work. |
| 2 | u16 big-endian | length `n` of the house blob (`operator>>(short)`, masked `& 0xffff`) |
| n | bytes | copied raw into the object's own stream (+0x18), the read position skipped by `n` |

Not compressed, no x3f1 count, one nested stream (a raw copy). An empty city list is `01 0000`.

### 1.2 The blob

Handler `G:FUN_101304d5` calls `PlayfieldCityHolderClient_c::GetPfCityHolderClient()` (`C:0x10016076`) and
`PlayfieldCityHolderClient_c::UpdateNewHouses(stream, false)` (`C:0x10016317`). The `false` means "no build effect".

`UpdateNewHouses` (`C:0x10016317`): nothing if the stream's size is 0; else

| bytes | type | meaning |
|---|---|---|
| 4 | u32 BE | house count, a **plain count** (not `(n+1)*1009`) |
| 25 × count | | house records |

House record = `BaseClientHouseData_t` (0x1c bytes in memory), also read by `operator>>` `C:0x1000dec3` for the later
"house added" signal:

| offset | bytes | type | field | read by |
|---|---|---|---|---|
| 0 | 12 | 3 × f32 BE | position x, y, z: the **minimum corner** of the building's (turned) tile footprint, in playfield coordinates | `C:FUN_1000de03` |
| 12 | 4 | i32 BE | rotation in **degrees** (0, 90, 180, 270) | `operator>>(int)` |
| 16 | 1 | u8 | flag, true only when the byte is exactly 1. The port calls it "open"; `SlotPlayfieldHouseDemolishStarted` (`C:0x10016ab7`) sets it, so it is really "being demolished". **Guess** as to the name. Not used for drawing. | `C:FUN_1000ddae` |
| 17 | 8 | Identity (u32 type, u32 instance) BE | the house template: type **0xf4268**, instance 0x6a00xx | `C:FUN_100274bb` |

There is no building identity, no scale, no state, no item id in the record. A house is keyed by its tile:
`TilePos = (floor(x / 4.0), floor(z / 4.0))` (`BaseClientHouseData_t::GetTilePos` `C:0x100011a7`, tile size double 4.0 at
`C:0x1002a258`).

The original does not check the count against the length (a short blob makes it add default records). In Rust, require
`blob.len() == 4 + 25 * count`.

### 1.3 The sample decoded

`01 07d4 00000050 44590000 40a00000 44990000 0000005a 00 000f4268 ...`

| bytes | value |
|---|---|
| `01` | lead byte |
| `07d4` | blob length 2004 (message 1 + 2 + 2004 = 2007, as logged) |
| `00000050` | 80 houses; 4 + 80 × 25 = 2004, exact |
| `44590000 40a00000 44990000` | position (868.0, 5.0, 1224.0) = tile (217, 306) |
| `0000005a` | rotation 90 |
| `00` | flag clear |
| `000f4268` | template type 0xf4268; its instance is the next 4 bytes, not in the sample |

It fits completely. Cross-check: playfield 6010's city record (section 4) allows building on tiles x 210..290, z 280..390;
(217, 306) is inside.

---

## 2. What the handler does

### 2.1 Objects made

Per city: **nothing from this message.** The cities themselves (`CityAI_c`, 0x480 bytes each) are made at playfield load by
`PlayfieldAnarchy_t::LoadPlayfieldCityData` (`G:0x101226c9`) from the database record `RDBPlayfieldCityData_t`, type
**0xf4267**, instance = playfield number (type set in `ReadPlayfield` `G:0x1012319f`; section 4).

Per house, `PlayfieldCityHolderClient_c::AddNewHouse(record, effect)` (`C:0x10016151`):

1. A `ClientHouseData_t` (0x30 bytes: the record, a surface pointer at +0x1c, a vector of placed visuals at +0x20) in the
   holder's map at +0x18, keyed by the tile. A second house on the same tile overwrites the record and appends its visuals.
2. The template: `CityHouseTemplateRegistry_c::GetTemplateRegistry()->GetTemplate(templateId, rotation)` (virtual slot +4).
   The client's registry is `AOCityHouseTemplateRegistry_c` (vtable `G:0x10171dc4`, installed by `G:FUN_1012d939`); its
   `GetTemplate` is `G:FUN_1012e4e4`: if the rotation-0 template is not registered, it fetches the identity **as given** from
   the resource database (`n3DatabaseHandler_t::Get()->GetResourceDatabase()->Get(id)`), casts to `RDBHouseTemplate_t`, and
   registers the `CityHouseTemplate_c` at record +0x18; then the base `GetTemplate` (`C:0x1000f0fd`) returns the copy for that
   rotation (made on demand by `CityHouseTemplate_c(other, rotation)` `C:0x1000ef65`). No null check: a missing record
   would crash the original.
3. For each of the template's visual records (`HouseVisualData`, 5 words: mesh, x, y, z, rotation):
   - `pos = template.RotateLocalPos(visual.pos, house.rotation)` (`C:0x1000e0db`, argument confirmed in the disassembly at
     `C:0x10016231`), then `pos += house.position` (`C:FUN_1000daed`);
   - `visual.rotation += house.rotation`.
4. If `effect`: virtual +0x1c `RunCreateEffect(template, house position)` = `AOPlayfieldCityHolderClient_c::RunCreateEffect`
   (`G:0x1012e86b`): effect **0x30e8** (12520) by `_EffectHandler_t::CreateEffect2`, sized by the layout (tiles × 4.0). Only
   for houses added later, not for this message.
5. Virtual +0x14 `EnableVisibility(tile, visuals)` = `AOPlayfieldCityHolderClient_c::EnableVisibility` (`G:0x1012eadb`),
   below.
6. `MakeHouseSurfaceAtTilePos(tile)` (`C:0x100168e7`) and virtual +0x20 `AddSurface` (`G:0x1012e975`): collision, 2.4.

No dynel, no `House_c`, no `n3Dynel` subclass is made on the client. `House_c` (`C:0x10012da4`, 0x84 bytes) is the server's
object; the client never constructs one from this message.

### 2.2 How the mesh is chosen and placed

`AOPlayfieldCityHolderClient_c::EnableVisibility` (`G:0x1012eadb`), only when the tile has no meshes yet (map at
holder +0x28, tile → vector of `VisualMesh_t*`). For each visual:

- mesh identity = **(0xf696a, visual.mesh)** (static identity at `G:0x102eac2c`);
- `new VisualMesh_t` (0xc0 bytes); `SetMesh(identity, 0, no texture overrides, 0)`; `EnableVisibility()`;
- `SetPosition(visual.pos)` (already in world coordinates);
- `SetRotation(q)`, `q` = rotation about the axis (0, 1, 0) by `-(visual.rotation × (π/4) / 45)` radians, i.e. by **minus the
  degrees** (`G:FUN_100520d9` builds `(axis × sin(a/2), cos(a/2))`; constants π/4 at `G:0x10168550`, 45.0 at `G:0x10168488`);
- `LODManager::GetSingleton()->AddMesh(mesh)`.

So: **no item, no stat 12, no "house item" lookup.** The mesh number is stored directly in the template's visual record. No
scale is set (1.0). Textures are the mesh's own.

About 0xf696a: that is the low-detail mesh type. The mesh starts as the low-detail twin and the `LODManager` swaps it to the
full mesh 0xf6951 of the same instance when near (`distance² − radius² < 4000`), as it does for statel meshes
(`dropin/displaysystem/src/lodmanager.rs`, `process`). I checked the database for 25 of the buildings' meshes: **all 25 exist
as 0xf6951; only 10 exist as 0xf696a.** What the original shows far away for a mesh with no 0xf696a record I did not trace
(`RDBMesh_t::ReduceMesh`, `dropin/displaysystem/src/rdb.rs`, can synthesize one; **guess** that this is the path). For the Rust
client: load 0xf6951 and treat 0xf696a as the optional twin, exactly as `add_props` does for statels.

### 2.3 The rotation of a local position

`CityHouseTemplate_c::RotateLocalPos(p, degrees)` (`C:0x1000e0db`):

```
q  = quaternion about (0,1,0) by -(degrees × (π/4)/45)        C:FUN_1000e09c -> C:FUN_1000dc16
c  = layout.GetRotCenter() = (tilesX × 4 × 0.5, 0, tilesZ × 4 × 0.5)   C:0x100134cd  (the UNTURNED layout, template +0x34)
p' = rotate(p − c, q) + |rotate(c, q)|   (component-wise absolute value)
```

`rotate` (`C:FUN_1000dcbe`) is the ordinary `v + 2w(u × v) + 2(u × (u × v))`, cross product `u × v` (`C:FUN_1000db11`).

For the four rotations that occur this is (Lx = tilesX × 4, Lz = tilesZ × 4 of the unturned layout):

| rotation | (x, y, z) becomes | a mesh's local +X points to |
|---|---|---|
| 0 | (x, y, z) | +X |
| 90 | (Lz − z, y, x) | +Z |
| 180 | (Lx − x, y, Lz − z) | −X |
| 270 | (z, y, Lx − x) | −Z |

Checked numerically with a Python copy of the functions (template 0x6a0006, 20×15 tiles, visual at (38.2311, 0.0751,
29.6345): 90 → (30.366, 0.075, 38.231)). The layout turns the same way (`HouseLayout_c(layout, rotation)`, port
`layout.rs new_rotated`: tile (x, z) → (tilesZ − z − 1, x) for 90), so the house position stays the minimum corner.

### 2.4 Collision

`MakeHouseSurfaceAtTilePos` (`C:0x100168e7`): if the template `HasKDTree()` (`C:0x1000e5c9`: a blob of 4 bytes or more), a
`HouseSurface_c` (0x2c bytes) holding `template.GetKDTree()` (`C:0x1000eafc`) with `SetTranslation(house.position)`
(`C:0x10014a96`). `AOPlayfieldCityHolderClient_c::AddSurface` (`G:0x1012e975`) registers it with the playfield's tile map
surface: `n3TilemapSurface_t::AddSurfaceToArea(surface, GetSafeMinPoint(), GetSafeMaxPoint())`.

`GetKDTree` (`C:0x1000eafc`): rotation snapped to a multiple of 90 (`((r + 360) % 360) / 90 × 90`); the template's KD blob is
decoded by `C:FUN_100216c5` through the matrix "translate by −c, rotate by q, translate by |rotate(c)|", i.e. the same
transform as 2.3, and cached per (template, rotation).

The blob is the tail of the template record (1.4 below). Per the port (`kd.rs read_v5`, which I compared with
`C:FUN_100216c5` only at the level of its call structure) it is the same range-coded "version 5" tree as the zone collision
surfaces (0xf424d): version symbol, volume count, node count, quantized box, the volumes' meshes at precision 0.01, marker
0x5a, nodes. There is **no** 12-byte head and **no** teleportal/end-mark tail. Not tested against the Rust reader.

### 2.5 The template record (type 0xf4268)

`RDBHouseTemplate_t::ReadBlob` = `G:FUN_101306ee`: `DbObject_t::ReadBlob` (the 12-byte head: type, instance, a word that is
1), `SetID(record identity)`, then `operator>>(BinaryStream&, CityHouseTemplate_c&)` (`C:0x1000f1a7`). **Little-endian** (it
is a database record, not network data). Version 4, which all 83 real records are:

| type | field (template offset) | notes |
|---|---|---|
| i32 | format version = 4 | 1, 2, 3 are older layouts the reader still accepts |
| i32, i32 | identity | used only if non-zero; 0,0 in every record, so the record's own identity stays |
| u32 | inside (+0x30) | `GetInsideTemplateInstance`; values like 4342..4357, 6133, 7018..7020. **Guess:** the interior's playfield number |
| u32 | fixtures allowed (+0x9c) | |
| u16 + bytes | name (+0xc) | `C:FUN_1001080f`; empty in every record |
| i32 | unknown word (+0x94) | 0 in every record |
| f32 | stealth factor (+0x98) | 0 in every record |
| i32 | upkeep (+0x8) | |
| i32 | house type (+0x4) | enum below |
| i32, i32, i32, bytes | layout: bit count, tilesX, tilesZ, ceil(bits/8) bytes | `HouseLayout_c::ReadBlob` `C:0x10013f65`; bit `z × tilesX + x` (`GetTile` `C:0x1001368d`), low bit first (`ReadBlobFixedLen` `C:0x10013acc`). Tile size is fixed 4.0 |
| u32 `(n+1)×1009`, then n × 20 bytes | **visuals** (+0x84) | `C:FUN_10010e28`; each (`C:FUN_1000de5b`): u32 mesh, f32 x, y, z (local), i32 rotation degrees |
| u32 `(n+1)×1009`, then n × 20 bytes | doors (+0xf8) | `C:FUN_10010c07`; each (`C:FUN_1000df2f`): u32 id, f32 x, y, z, i32 rotation, **negated when read** |
| u32 `(n+1)×1009`, then n × 24 bytes | guards | `C:FUN_10011166`; each: 4-character name, i32 level, f32 x, y, z, i32 rotation (negated) |
| u32 `(n+1)×1009`, then n × (u16 + bytes + u32) | advantages | `C:FUN_100113c5`; name string, level; none in the records |
| rest | KD tree blob | `GrabKDTreeBlob` `C:0x1000e5f5` |

The x3f1 counts here do what the tower list does: valid only if a multiple of 1009 and `count/1009 − 1 < 30001`, else the
stream is marked bad.

House types (`fHouseTypeToString` `C:0x10014f62`, the `Is...` predicates at `C:0x1000dfb9`..`0x1000e009`): 0 normal house,
1 HQ, 2 market, 3 grid, 4 guard house, 5 radar station, 6 cloaking device, 7 mini cloaking device, 8 tax office, 9 unique
house. The type changes nothing in how the building is drawn.

The database has 85 records of type 0xf4268: 83 version-4 ones (instances 0x6a0000..0x6a00c4, appendix) and two version-1
ones (0x60000, 0x60001; **guess:** leftovers). `housetpl.py` parses all 83 with nothing left over except the KD blob. Most
have one visual; 0x6a001b has three.

Related: item stat 620 is named `HouseTemplate` in `crates/mao-client/src/gui/stat_names.rs`. **Guess:** the building items'
link to these records. Not needed for drawing.

### 2.6 The entrance door

The client does not make doors from the template. `GetDoorData` is used only by GUI.dll (build view). The door dynels come
from the server, which places them with `House_c::CalculateGlobalDoorPosRot` (`C:0x10012c5b`):

```
door position = house.position + RotateLocalPos(door.pos, house.rotation)          C:0x1000e2d5
door rotation = quaternion about (0,1,0) by -(door.rotation_as_kept + house.rotation) degrees
```

(`door.rotation_as_kept` is the stored value negated, see the table.) The door's `id` is its **item template**: all nine ids I
looked up exist as 0xf4254 records, and 41565 (0xa25d) is also the template of playfield 6010's static door. That the server
uses exactly this function for the 0xc748 doors you receive is a **guess** (the function exists for it, but the server is not
in these files). It gives a free test: every `DoorFullUpdate` position in the city should equal one house's computed door
position.

Guards (`CalculateGlobalGuardPosRot` `C:0x10012c97`) work the same way and are also server-spawned.

---

## 3. Later updates (brief)

All through **AOTransportSignal, N3 0x62741E15** (`AOTransportSignalIIR_c`, vtable `G:0x10171ef8`). Reader `G:FUN_1012faab`:
an `i32` code, then the rest of the message as payload. Handler `G:FUN_1012fbc1` → `G:FUN_1012fdaa` looks the code up in a
registry and hands the payload to each registered signal. Big-endian. Whether a lead byte precedes the code as in other N3
messages is a **guess** (yes).

| code | signal (where the code is set) | payload | client effect |
|---|---|---|---|
| 2 | house added (`G:0x1012ecc0`) | one 25-byte house record (reader `G:FUN_1012f87c` → `C:0x1000dec3`) | `SlotPlayfieldHouseAdded` `C:0x10016a91` → `AddNewHouse(record, true)`: as section 2 plus the build effect 0x30e8 |
| 8 | demolish started (`G:0x1012ed38`) | tile: i32 x, i32 z (`G:FUN_1012f95b` → `C:0x1001b7a3`) | `C:0x10016ab7`: sets the record's flag; nothing visual |
| 9 | house removed (`G:0x1012edb0`) | tile: i32 x, i32 z | `C:0x10016b18`: surface removed (`RemoveSurface` `G:0x1012e9e2`), meshes deleted (`DisableVisibility` `G:0x1012ea59`), record erased |
| 5 | open city terminal session (`G:0x1012ee28`) | schema's `CityInfo` | GUI |
| 6 / 7 | lease upgrade (int) / close session (CityID) | | GUI (`G:FUN_1012be1d`) |
| 10 / 13 | upkeep cost (int) / seconds till bill (int) | schema's `CityCreditsUpkeep`, `CityNextUpkeep` | GUI |
| 14 | cloaking device state (int state, int time) | schema's `CloakInfo` | `CityClientInterface_c::SlotCloakingDeviceState` `C:0x1000a46c`: stores numbers only |
| 15 | controller charge (float) | schema's `CityCharge` | stored |
| 16 | city organization name (string) | | stored |

Codes 2, 8, 9 go to everyone in the playfield (`TransportSignalToPlayfieldClients_c`), the rest to one character.

Others:

- **CityAdvantages 0x365E555B** (`CityAdvantagesIIR_t`, handler `G:FUN_10038e61`): builds a chat text list of advantage items
  ("ActiveAdvantage"/"DisabledAdvantage"). Nothing visual.
- **ToggleCloak 0x3F5E4B46** (`ClientRqToggleCloakingIIR_c`, `G:FUN_101303bc`): the client's request.
- `UpdateDestroyedHouses` (`C:0x1001640e`: u32 count, then tiles) and `UpdateDemolishStart` (`C:0x10016471`) exist in
  city.dll but Gamecode.dll does not import them: unused by the client.
- No message changes one building in place and there is no "HouseOnPlot..." message. A change is remove + add.
- `CentralControllerFullUpdate` / `CentralControllerState` are **not** city messages: `CentralController_t` is the mission
  hazard (0xc742).

The holder is emptied when the playfield goes: `~PlayfieldAnarchy_t` (`G:0x10122f81`) → `PlayfieldCityHolderClient_c::ResetAll`
(`C:0x10016c3b`).

---

## 4. The city controller

**It is a static object of the playfield, not one of the buildings in the message.**

- Playfield 6010's static dynel table (database type 0xf425a, the one `mao_formats::dynels` reads) has three entries:
  `(0xc418, 0xc000177a)` at (996.00, 5.01, 1240.00), item template 0x3cfa7 (249767), with its own mesh stat 12 = **251929**
  (exists as 0xf6951); a `0xc73d` at (1158, 5.01, 1218); and the door `(0xc748, 0xc000177a)` at (1170.61, 15.37, 1221.53),
  template 0xa25d. Identity type 0xc418 = 50200 = `CityController` in the schema.
- The playfield's city record (type 0xf4267, instance 6010; `RDBCityController_c::ReadBlob` `C:0x1000d03a`,
  `CityControllerData_c::ReadBlob` `C:0x1000cf57`) names the same dynel: version 1, 1 controller, tile extents 600 × 600,
  dynel (0xc418, 0xc000177a), house levels 1..1, purchase 500,000,000, upkeep 15,000,000, one landing point at (997.23, 5.01,
  1143.13), 74 zones of 10 × 10 tiles covering tiles x 210..290, z 280..390 (world x 840..1160, z 1120..1560).
  `LoadPlayfieldCityData` (`G:0x101226c9`) remaps the dynel id for instanced playfields (`GetRemappedDynelID`).
- Its class is `CityTerminal_t` (constructor `G:FUN_1012fe5a`), a `SimpleItem_t`. Compared slot by slot with `SimpleItem_t`'s
  vtables, its only overrides are the destructors and five tiny stubs (`G:0x1012fe46`..`0x1012fe57`: a jump to the base,
  `return 0`, `return 2`, plain returns). **Nothing special in how it is drawn**: it is drawn as any simple item, by its mesh.
- Cloaking: I found no world effect. The cloak state only lands in `CityClientInterface_c` (+0x5c state, +0x64 time, +0x60
  "toggle possible") and is read by GUI.dll's city terminal window (`GetCloakingState`, `IsCloakingTogglePossible` are GUI.dll
  imports; Gamecode.dll imports only the setter). No state-dependent texture, no cloak mesh swap in city.dll or in
  Gamecode's city code. Animated parts: only what the `LODManager` does for every 0xf6951 mesh (advances its animation time).
  That there is no cloak visual anywhere else (for example an effect the server triggers by another message) is a **guess**.

The Rust `static_props` should already draw the controller (the record has a mesh stat). Worth checking in the 6010 picture
at (996, 5, 1240).

---

## 5. Implementation outline for the Rust client

Existing pieces that look reusable (nothing was edited):

- `crates/mao-net/src/landcontrol.rs`: `ALL_CITIES`, `all_cities(data)` already returns the blob (lead byte, u16 length). Its
  module comment and `docs/CAPTURE_GAPS.md` section 9 say "always empty in the captures"; the test `hex("010000")` stays valid.
- `crates/mao-client/src/net.rs`: `State::cities: BTreeMap<u32, Vec<u8>>` (line 465, filled at 1615..1618);
  **`State::props: BTreeMap<u32, (mesh, position, rotation [x, y, z, w])>`** (line 483) drawn by `overlay_model` (line 4525),
  which is how announced doors and terminals get their meshes (line 2771); `zone_surfaces` / `surface_collision` (line 4862)
  build `mao_formats::collision::Collision` from triangles.
- `crates/mao-client/src/main.rs`: `load_model(db, Id { kind: 0xf6951, .. })` and the 0xf696a twin handling in `add_props`
  (line 911).
- `crates/mao-formats/src/surface.rs`: the range decoder and `read_tree` (private) for the KD blob; `parse` itself expects
  the 12-byte head and the teleportal tail, so a second entry point for a bare blob is needed.
- `crates/mao-formats/src/resource.rs`: `ResourceDatabase::get(Id)`.
- `AnarchyReimagined/dropin/city/src/{client,template,layout,tile,kd}.rs`: the whole logic already in Rust, in the
  drop-in's ABI style; good as a reference and for test vectors.

Steps:

1. **Parse the blob** (`mao-net`, next to `all_cities`): u32 BE count; require `len == 4 + 25 × count`; per house
   `position [f32; 3]`, `rotation i32`, `flag = byte == 1`, `template (u32, u32)`. Test with the 28 sample bytes padded, and
   with `count = 80`.
2. **Read the template** (`mao-formats`, new small module): `db.get(Id { kind: 0xf4268, instance })`, skip 12 bytes, read
   the version-4 layout of 2.5 (little-endian) into `{ kind, tiles_x, tiles_z, visuals: Vec<(mesh, [f32; 3], i32)>, doors,
   kd_blob }`. Skip a house whose record is missing or not version 4. Cache by instance. A game-data test can assert the 83
   records parse to the appendix values.
3. **Place each visual.** With `Lx = tiles_x × 4.0`, `Lz = tiles_z × 4.0`, `r = house.rotation`:
   - `local' = rotate_local(visual.pos, r)` (table in 2.3; or the general quaternion form for other angles);
   - `world = house.position + local'`;
   - `angle = -(visual.rotation + r)` degrees about +Y; quaternion `[0, sin(a/2), 0, cos(a/2)]`;
   - mesh = `(0xf6951, visual.mesh)` with `(0xf696a, visual.mesh)` as the far twin when it exists; scale 1.
   The matrix is `T(world) × R_y(angle)`; there is no pivot offset on the mesh itself (the pivot handling is only in the
   position).
4. **Draw.** Simplest: insert into `State::props` under a key range of its own (for example `0xfd00_0000 | house index << 4 |
   visual index`), since `overlay_model` already draws `(mesh, position, [x, y, z, w])` in the server's frame. Buildings are
   large and static, so the better home is the scene model with the statel LOD path (`Model::lod_groups`); that is a design
   choice for you. Clear the entries on zoning (the original's `ResetAll`).
5. **Collision.** Decode `kd_blob` to triangles, apply the same local transform (`rotate_local` on every vertex, rotation
   snapped to a multiple of 90), add `house.position`, and add them to the playfield's `Collision`. Until then the buildings
   are walk-through.
6. **Check against the doors.** For every house and template door compute `house.position + rotate_local(door.pos, r)`;
   each should match a received `DoorFullUpdate` position. This validates rotation sense and the pivot rule without the
   original client. If a 90/270 building is mirrored, the sign of the angle in step 3 is the thing to look at: the position
   rule is checked numerically, the quaternion-to-matrix convention of `props` I did not test.
7. **Updates.** Handle AOTransportSignal 0x62741E15 codes 2 (add one record) and 9 (remove by tile); ignore 8.
8. **Capture.** The log only kept 28 bytes. A full dump of the 2007-byte message would let steps 1 to 3 and 6 be tested
   offline.

---

## 6. What is a guess

Collected from above:

- Who consumes the lead byte `01`, and that AOTransportSignal has one too.
- The name/meaning of the record's flag byte ("open" vs "being demolished").
- `inside` = the interior's playfield number.
- That the server places the 0xc748 door dynels with `CalculateGlobalDoorPosRot` (the client side is certain: it makes no doors).
- What the original draws far away for a building mesh that has no 0xf696a record.
- That the two version-1 template records are leftovers.
- Item stat 620 linking building items to templates.
- That no cloak visual exists outside the code I read.
- The KD blob being byte-compatible with `mao_formats::surface`'s reader (taken from the port's description; not run).
- The quaternion convention of `State::props` matching `VisualMesh_t::SetRotation` (both are fed dynel-style x, y, z, w;
  not tested).

Not verified because the data is missing: the template instances used in playfield 6010 (the sample stops before the first
one), and therefore which meshes that city shows.

---

## Appendix: the 83 house templates (type 0xf4268)

Local positions are relative to the unturned footprint's minimum corner; tiles are 4.0 units. Types: {0 normal: 31, 1 HQ: 10,
2 market: 9, 3 grid: 3, 4 guard house: 3, 5 radar: 3, 6 cloak: 3, 7 mini cloak: 3, 8 tax office: 3, 9 unique: 15}.

| instance | type | tiles x*z | visuals: mesh @local pos, rotation | doors: item template @local pos, rotation (as kept, i.e. negated) | inside playfield | KD blob bytes |
|---|---|---|---|---|---|---|
| 0x6a0000 | 1 HQ | 10x10 | 36126 @(10.38, -0.00, 28.87) r90 | 41565 @(17.17, 1.92, 29.00) r-90 | 4342 | 136 |
| 0x6a0001 | 1 HQ | 20x20 | 302570 @(40.90, 0.00, 41.00) r0 | 255604 @(40.40, 6.94, 61.15) r0 | 4349 | 1549 |
| 0x6a0002 | 3 grid | 5x5 | 246841 @(9.69, 0.40, 9.81) r0 | 297174 @(9.77, 1.35, 10.19) r0 | 0 | 507 |
| 0x6a0003 | 9 unique | 15x15 | 250627 @(32.95, 0.00, 28.76) r0 | 255601 @(38.71, 1.45, 47.68) r0 | 4350 | 1369 |
| 0x6a0004 | 9 unique | 10x10 | 246997 @(20.08, -0.08, 19.07) r0 | - | 0 | 2773 |
| 0x6a0005 | 2 market | 10x10 | 247020 @(19.56, 0.02, 19.75) r0 | 255602 @(18.34, 1.31, 18.52) r-225 | 4355 | 704 |
| 0x6a0006 | 9 unique | 20x15 | 250628 @(38.23, 0.08, 29.63) r0 | 41565 @(12.05, 1.52, 29.63) r-275 | 4351 | 1561 |
| 0x6a0007 | 6 cloak | 15x15 | 250634 @(29.61, 0.56, 27.80) r0 | - | 0 | 1629 |
| 0x6a0008 | 1 HQ | 20x20 | 248751 @(39.90, 0.02, 38.68) r0 | 41565 @(39.90, 3.14, 64.96) r0 | 4346 | 1583 |
| 0x6a0009 | 1 HQ | 15x15 | 302571 @(29.70, 0.15, 19.20) r0 | 41565 @(29.69, 3.22, 45.06) r0 | 4343 | 875 |
| 0x6a000a | 4 guard house | 5x5 | 253433 @(10.06, -0.01, 8.55) r0 | - | 0 | 365 |
| 0x6a000b | 9 unique | 10x10 | 253436 @(23.27, -0.11, 18.15) r0 | - | 4342 | 415 |
| 0x6a000c | 0 normal | 10x15 | 253440 @(19.95, -0.19, 30.30) r0 | - | 0 | 3398 |
| 0x6a000d | 0 normal | 15x10 | 253438 @(39.36, 0.88, 20.00) r0 | - | 0 | 4222 |
| 0x6a000e | 5 radar | 5x5 | 254412 @(9.98, 0.00, 9.80) r0 | - | 0 | 119 |
| 0x6a000f | 2 market | 10x10 | 247020 @(19.86, 0.02, 18.18) r0 | 255602 @(18.64, 1.31, 16.96) r-225 | 4356 | 704 |
| 0x6a0010 | 2 market | 10x10 | 247020 @(18.73, 0.02, 18.50) r0 | 255602 @(17.51, 1.31, 17.28) r-225 | 4357 | 704 |
| 0x6a0011 | 7 mini cloak | 10x10 | 256407 @(18.47, 0.13, 18.56) r0 | - | 0 | 1565 |
| 0x6a0014 | 0 normal | 1x1 | 254924 @(2.14, 0.32, 2.13) r0 | - | 0 | 3 |
| 0x6a0015 | 0 normal | 2x1 | 36217 @(3.90, -0.00, 1.76) r0 | - | 0 | 48 |
| 0x6a0016 | 0 normal | 1x1 | 254925 @(1.98, 0.01, 1.85) r180 | - | 0 | 3 |
| 0x6a0017 | 0 normal | 2x2 | 254240 @(4.00, 0.00, 4.03) r0 | 256031 @(4.00, 2.94, 6.27) r0 | 0 | 73 |
| 0x6a0018 | 0 normal | 2x2 | 254244 @(4.04, -0.00, 3.99) r0 | 256042 @(4.04, 2.94, 6.23) r0 | 0 | 73 |
| 0x6a0019 | 0 normal | 2x2 | 255430 @(4.12, 0.00, 3.90) r0 | 256040 @(4.13, 2.94, 6.15) r0 | 0 | 73 |
| 0x6a001a | 0 normal | 2x2 | 255435 @(4.05, -0.00, 3.93) r0 | 256041 @(4.06, 2.94, 6.18) r0 | 0 | 73 |
| 0x6a001b | 0 normal | 1x1 | 38471 @(1.78, 0.00, 2.11) r90; 38472 @(2.03, -0.00, 3.56) r0; 38471 @(2.17, 0.07, 2.02) r270 | - | 0 | 119 |
| 0x6a001c | 0 normal | 1x1 | 41226 @(2.19, -0.00, 1.87) r0 | - | 0 | 108 |
| 0x6a001d | 0 normal | 1x1 | 40607 @(2.02, -0.02, 1.66) r0 | - | 0 | 92 |
| 0x6a001e | 0 normal | 1x1 | 40595 @(2.12, -0.02, 1.52) r0 | - | 0 | 93 |
| 0x6a001f | 0 normal | 1x1 | 40589 @(2.06, -0.02, 1.56) r0 | - | 0 | 98 |
| 0x6a0020 | 0 normal | 1x1 | 38539 @(2.06, -0.00, 2.02) r0 | - | 0 | 37 |
| 0x6a0021 | 0 normal | 1x1 | 38541 @(1.83, -0.00, 2.00) r0 | - | 0 | 37 |
| 0x6a003d | 1 HQ | 20x20 | 302568 @(40.20, 0.00, 41.00) r0 | 255604 @(39.69, 6.94, 61.17) r0 | 4348 | 1549 |
| 0x6a003e | 3 grid | 5x5 | 256175 @(9.92, 0.40, 8.71) r0 | 297182 @(10.00, 1.35, 9.09) r0 | 0 | 507 |
| 0x6a003f | 9 unique | 15x15 | 256177 @(30.97, 0.00, 28.29) r0 | 255601 @(36.73, 1.45, 47.20) r0 | 4350 | 1380 |
| 0x6a0040 | 9 unique | 10x10 | 246997 @(19.94, -0.08, 19.87) r0 | - | 0 | 2772 |
| 0x6a0041 | 2 market | 10x10 | 256160 @(18.85, 0.01, 19.73) r0 | 255602 @(17.63, 1.30, 18.51) r-225 | 4355 | 733 |
| 0x6a0042 | 9 unique | 20x15 | 256163 @(37.51, 0.08, 29.72) r0 | 255603 @(11.33, 1.52, 29.72) r-275 | 4351 | 1561 |
| 0x6a0043 | 6 cloak | 15x15 | 256165 @(28.79, 0.56, 27.90) r0 | - | 0 | 1629 |
| 0x6a0044 | 1 HQ | 20x20 | 256196 @(39.42, 0.02, 39.02) r0 | 255605 @(39.42, 3.14, 65.30) r0 | 4345 | 1586 |
| 0x6a0045 | 1 HQ | 15x15 | 302569 @(29.00, 0.04, 19.40) r0 | 255609 @(29.00, 3.11, 45.30) r0 | 4342 | 871 |
| 0x6a0046 | 4 guard house | 5x5 | 256171 @(10.06, -0.01, 10.54) r0 | - | 0 | 365 |
| 0x6a0047 | 9 unique | 10x10 | 256172 @(23.49, -0.12, 18.44) r0 | - | 4342 | 415 |
| 0x6a0048 | 0 normal | 10x15 | 256173 @(19.13, -0.20, 30.58) r0 | - | 0 | 3398 |
| 0x6a0049 | 0 normal | 15x10 | 256174 @(39.40, 0.88, 19.94) r0 | - | 0 | 4190 |
| 0x6a004a | 5 radar | 5x5 | 256179 @(9.16, 0.00, 9.89) r0 | - | 0 | 119 |
| 0x6a004b | 2 market | 10x10 | 256160 @(19.31, 0.01, 18.27) r0 | 255602 @(18.09, 1.30, 17.05) r-225 | 4356 | 734 |
| 0x6a004c | 2 market | 10x10 | 256160 @(17.91, 0.01, 18.79) r0 | 255602 @(16.69, 1.30, 17.57) r-225 | 4357 | 733 |
| 0x6a004d | 7 mini cloak | 10x10 | 256405 @(18.21, 0.13, 17.74) r0 | - | 0 | 1558 |
| 0x6a0065 | 1 HQ | 20x20 | 302572 @(40.00, 0.00, 40.80) r0 | 255604 @(39.45, 6.94, 61.00) r0 | 4347 | 1549 |
| 0x6a0066 | 3 grid | 5x5 | 256145 @(9.86, 0.40, 9.87) r0 | 297183 @(9.94, 1.35, 10.25) r0 | 0 | 507 |
| 0x6a0067 | 9 unique | 15x15 | 256137 @(30.82, 0.00, 30.14) r0 | 255601 @(36.58, 1.45, 49.05) r0 | 4350 | 1380 |
| 0x6a0068 | 9 unique | 10x10 | 246997 @(20.09, -0.08, 19.29) r0 | - | 0 | 2770 |
| 0x6a0069 | 2 market | 10x10 | 256139 @(18.55, 0.01, 19.36) r0 | 255602 @(17.33, 1.30, 18.14) r-225 | 4355 | 733 |
| 0x6a006a | 9 unique | 20x15 | 256123 @(37.18, 0.08, 28.89) r0 | 255603 @(11.00, 1.52, 28.89) r-275 | 4351 | 1561 |
| 0x6a006b | 6 cloak | 15x15 | 256125 @(28.58, 0.56, 27.32) r0 | - | 0 | 1629 |
| 0x6a006c | 1 HQ | 20x20 | 256292 @(38.93, 0.02, 38.03) r0 | 255605 @(38.93, 3.14, 64.31) r0 | 4344 | 1583 |
| 0x6a006d | 1 HQ | 15x15 | 302573 @(28.60, 0.04, 18.60) r0 | 255609 @(28.62, 3.22, 44.42) r0 | 4341 | 871 |
| 0x6a006e | 4 guard house | 5x5 | 256143 @(9.91, -0.01, 9.97) r0 | - | 0 | 359 |
| 0x6a006f | 9 unique | 10x10 | 256132 @(23.27, -0.12, 17.85) r0 | - | 4342 | 415 |
| 0x6a0070 | 0 normal | 10x15 | 256133 @(18.92, 0.00, 30.46) r0 | - | 0 | 3398 |
| 0x6a0071 | 0 normal | 15x10 | 255892 @(39.40, 0.87, 19.88) r0 | - | 0 | 4190 |
| 0x6a0072 | 5 radar | 5x5 | 256144 @(8.95, 0.00, 9.31) r0 | - | 0 | 119 |
| 0x6a0073 | 2 market | 10x10 | 256139 @(18.74, 0.01, 17.73) r0 | 255602 @(17.52, 1.30, 16.51) r-225 | 4356 | 733 |
| 0x6a0074 | 2 market | 10x10 | 256139 @(17.44, 0.01, 18.05) r0 | 255602 @(16.22, 1.30, 16.83) r-225 | 4357 | 734 |
| 0x6a0075 | 7 mini cloak | 10x10 | 256406 @(18.09, 0.13, 17.89) r0 | - | 0 | 1558 |
| 0x6a00b4 | 9 unique | 5x5 | 287247 @(10.24, -0.00, 9.60) r180 | 287554 @(13.71, 0.79, 4.78) r-180 | 6133 | 209 |
| 0x6a00b5 | 9 unique | 5x5 | 287247 @(10.15, -0.00, 9.60) r180 | 287554 @(13.62, 0.79, 4.78) r-180 | 6133 | 209 |
| 0x6a00b6 | 9 unique | 5x5 | 287247 @(9.97, 0.00, 9.63) r180 | 287554 @(13.44, 0.79, 4.81) r-180 | 6133 | 209 |
| 0x6a00b7 | 0 normal | 2x2 | 292558 @(5.29, 0.02, 4.00) r0 | - | 0 | 685 |
| 0x6a00b8 | 0 normal | 10x10 | 291992 @(20.70, -0.00, 19.61) r0 | - | 0 | 2577 |
| 0x6a00b9 | 0 normal | 10x10 | 291990 @(20.28, 1.51, 19.83) r0 | - | 0 | 1927 |
| 0x6a00ba | 0 normal | 10x10 | 291994 @(20.41, 1.47, 19.34) r0 | - | 0 | 2074 |
| 0x6a00bb | 8 tax office | 15x15 | 296424 @(25.16, 0.10, 30.78) r0 | 296417 @(25.17, 0.10, 34.07) r0 | 7019 | 868 |
| 0x6a00bc | 8 tax office | 15x15 | 296413 @(25.62, 0.08, 30.61) r0 | 296417 @(25.62, 0.08, 33.89) r0 | 7018 | 868 |
| 0x6a00bd | 8 tax office | 15x15 | 296426 @(24.59, 0.10, 30.60) r0 | 296417 @(24.60, 0.10, 33.89) r0 | 7020 | 864 |
| 0x6a00be | 0 normal | 1x2 | 293201 @(2.02, 0.07, 2.42) r0 | - | 0 | 164 |
| 0x6a00bf | 0 normal | 1x1 | 291983 @(2.13, 0.02, 2.04) r0 | - | 0 | 46 |
| 0x6a00c0 | 0 normal | 1x1 | 291984 @(2.11, 0.02, 1.97) r0 | - | 0 | 46 |
| 0x6a00c1 | 0 normal | 1x1 | 291985 @(1.96, 0.02, 1.89) r0 | - | 0 | 46 |
| 0x6a00c2 | 0 normal | 4x2 | 291129 @(8.14, 0.16, 3.92) r0 | - | 0 | 642 |
| 0x6a00c3 | 0 normal | 2x2 | 292124 @(4.26, 0.14, 4.14) r0 | - | 0 | 1181 |
| 0x6a00c4 | 0 normal | 1x1 | 291187 @(2.04, 0.03, 2.04) r0 | - | 0 | 106 |
