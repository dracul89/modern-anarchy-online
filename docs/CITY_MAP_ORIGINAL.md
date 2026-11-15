# The city map (`BuildingMapView_c`) in the original client

Sources: Ghidra decompiles of GUI.dll (`U:`), Gamecode.dll (`G:`), city.dll (`C:`), checked against disassembly wherever
Ghidra dropped arguments (marked "disasm"). `Rect`/`IRect` semantics are Utils.dll's (port: `crates/ao-utils/src/geometry.rs`):
a `Rect` is four edges, `Width = right - left`, `Size = (right-left, bottom-top)`, `Resize(a,b,c,d)` **adds** to each edge,
`Scale(x,y)` multiplies edges, `IRect(l,t,r,b)` are edges, `IsValid = l<=r && t<=b`, `&` is the edge-wise intersection.
`ftol` below is `U:0x10173b80` = truncation toward zero (`cvttsd2si`).

Real data used for checks: playfield 6010's city record `(0xf4267, 6010)` (re-dumped with `rawget`, identical to the saved
copy, 1256 bytes, parsed with nothing left over), its tile-map `(0xf4249, 6010)`, and the 83 version-4 house templates.

## 0. Class layout (certain)

`BuildingMapView_c` (0x5f0 bytes, ctor `U:0x100e5c31(mode, cityID)`, dtor `U:0x100e4bec`) derives from
**`PFMapGroundRenderer_c`** (ctor `U:0x10047208`, view name `playfield_map_view`), the ground renderer of the "PF Map".

| Offset | Field | Set by |
|---|---|---|
| +0x128 | marker atlas `RTexture_t*` (32 x 32) | base `U:0x10046d0b` |
| +0x130 | byte "draw ground", 1 | base ctor; `U:0x100e5c31` sets it to 1 again |
| +0x134 | `MapData_c*` singleton (`U:0x1002d3ed`, ctor `U:0x100439f1`) | base ctor |
| +0x138 | `Point` scale, pixels per world unit: **(2.0, 2.0)** | `U:0x100464d1` from the ctor (constant `U:0x101ae184` = 2.0; base default 1,1) |
| +0x140 | `Rect` **margins** in world units: left (+0x140), low-z (+0x144), right (+0x148), high-z (+0x14c) | `U:0x100e375d` via `U:0x10046520` |
| +0x150.. | base state blobs (ground passes) | base ctor |
| +0x468 | signal "can construct changed (bool)" | emitted by `U:0x100e44a2` |
| +0x46c | signal "selection changed (Node* or 0)" | emitted by `U:0x100e39d6` |
| +0x470 | mode: 0 build, 1 demolish, 2 view | ctor, `U:0x100e34ac` |
| +0x474 | `CityID_t`, 5 words **in memory order**: city identity (type, instance), playfield **model** identity (type, instance), playfield instance | ctor (`U:0x100e3484`); built in `G:0x101226c9` |
| +0x488 | `MapData_c*` (same singleton) | ctor |
| +0x48c | overlay `RTexture_t*`, 128 x 128, format 10 | ctor |
| +0x490 | `IRect` overlay window (left, top, left+127, top+127), in tile columns / flipped rows | `U:0x100e3fe7` |
| +0x4a0 | `std::list<Node>` head (list node: next, prev, value at +8) | |
| +0x4ac | selected list node (== head when nothing is selected) | `U:0x100e39d6` |
| +0x4b0 | `Point` drag anchor | `U:0x100e56a8` |
| +0x4b8 | drag state: 0 none, 1 dragging a pending node, 2 scrolling | `U:0x100e56a8`, `U:0x100e34e9` |
| +0x4bc | `int[10]` node count by house type | `U:0x100e4cef` / `U:0x100e479f` |
| +0x4e4 | byte "can construct" | `U:0x100e44a2` |
| +0x4e8 | overlay state blob | ctor |
| +0x56c | marker state blob | ctor |

Node (0x20 bytes, `U:0x100e9099(item, template, tile, rotation, demolishing)`): +0 item identity (`(0,0)` = existing
building, test `U:0x100e90db`), +8 template identity, +0x10 tile x, +0x14 tile z, +0x18 rotation (degrees), +0x1c
"demolishing" (`U:0x100e8f8d`, set by `U:0x100e8f88`), +0x1d "valid" (`U:0x100e8f9e`, set by `U:0x100e8f91`; the ctor sets 1).
Helpers: `U:0x100e9073` = `registry->GetTemplate(node.template, node.rotation)` (the **rotated** template copy),
`U:0x100e908c` = that template's `GetLayout()` (the **rotated** layout), `U:0x100e900f` = `Point(tileX * T.x, tileZ * T.y)`,
`U:0x100e905e` = the tile, `U:0x100e8fca(p)` = `tile := (ftol(p.x / T.x), ftol(p.y / T.y))`, `U:0x100e8fb6` = rotate.
`T` is `MapData_c +0xc4` (see 1.1).

Virtuals overridden (primary vtable `U:0x101bfd5c`): `Render` +0x34 = `U:0x100e3be3`; `MouseDown(Point, int, int)` +0x3c =
`U:0x100e56a8`; `MouseUp` +0x40 = `U:0x100e34e9`; `MouseMove` +0x44 = `U:0x100e4abf`; `FrameSized` +0x6c = `U:0x100e46e8` and
`ViewScrolled` +0x70 = `U:0x100e46f2` (both `redraw(0)`); `CalculatePreferredSize(bool)` +0x7c = `U:0x100e36a7`. Inherited
from the base: `GetContentSize` +0x84 = `U:0x10046429`, `ProvideToolTipText` +0x98 = `U:0x100478c7`.

---

## 1. Geometry and background

### 1.1 The map data and the world rectangle (certain)

`MapData_c::SetPlayfield(ground, tilemap)` = `U:0x1004339a` (a slot of the singleton):

- `+0xc4` (`T`, the "tile" Point) = **(4.0, 4.0)** when the playfield has a ground (`AnarchyGround_t*` at +8 non-null;
  constant `U:0x101b0848`), else (2.0, 2.0).
- `+0xb4` (the world Rect) = `(0, 0, tilesX * T.x - 1, tilesZ * T.y - 1)`, `tilesX/Z` = the `RDBTilemap_t`'s words at
  +0x825c / +0x8260 (the two u16 sizes of its `CHGA` chunk; `dropin/displaysystem/src/rdbtilemap.rs`).
- 6010's tile-map record `(0xf4249, 6010)`: `CHGA`, version 1, sizes **600 x 600**, tile size 4.0 (read from the record).
  So the world Rect is `(0, 0, 2399, 2399)` and `Size()` is `(2399, 2399)`.

Write `W4 = Size.w + 1 = 4 * tilesX` (2400) and `H4 = Size.h + 1 = 4 * tilesZ` (2400), `H = tilesZ` (600).

### 1.2 What is under the overlay (certain)

Not an image and not the planet map. `Render` (`U:0x100e3be3`) does, in order:

1. `U:0x10046ec8`: the base material and state.
2. A quad over the view's whole rect (normalized bounds, right and bottom + 1), no texture, colour 0: black
   (`U:0x10046f00(viewport, 0, uv, rect, 0)`).
3. `U:0x10047f7c` → `U:0x10047964`: **the ground drawn from above** by the base class: the same routine the PF Map window
   uses (each ground patch's hi texture multiplied over the white veil `0xbbffffff`; this is what `automap.rs`
   `ground_picture` + `VEIL` already reproduce). For a playfield of rooms it would be `U:0x100474b7`; city playfields have a
   ground.
4. The tile overlay (1.5).
5. The door and guard markers (section 4).
6. `U:0x10047101`: one more untextured quad with colour 0 under state blob +0x3e4 (alpha blending on, source alpha 0), then
   the material is reset. No visible effect that I can see (**guess**: a depth write).

There are no names, no legend and no grid lines.

### 1.3 Margins, size, scale, axes (certain)

`U:0x100e375d` (ctor): `U` = union of every zone's rectangle `(offX, offZ, offX + tilesX, offZ + tilesZ)` (the 10 x 10
rectangles, not the set tiles; start value `IRect(2000000000, 2000000000, 0, 0)`), then

```
left   = U.left   * T.x                 - 50
lowZ   = U.top    * T.y                 - 50
right  = Size.w - U.right  * T.x        - 50
highZ  = Size.h - U.bottom * T.y        - 50        each clamped to >= 0   (Resize(-50,-50,-50,-50), constant U:0x101bfd10)
```

| | 6010 |
|---|---|
| zone rectangles' union | tiles x 210..290, z 280..390 |
| margins left / lowZ / right / highZ | 790 / 1070 / 1189 / 789 |
| `GetContentSize` `U:0x10046429` = `((Size.w + 1 - left - right) * sx - 1, (Size.h + 1 - lowZ - highZ) * sy - 1)` | (841, 1081): **842 x 1082 pixels** |
| world shown | x 790..1211, z 1070..1611 (the zone area plus 50 units on each side) |

- Scale: 2 pixels per world unit, **8 x 8 pixels per tile** (a tile is 4 units).
- `+x` runs right, **`+z` runs up** (the highest z is at the top).
- The map sits in a `ScrollView_c(bounds, "", 2, 2, -1, 0, 0)` (`U:0x100e8108`); the window is 499 x 499 so it scrolls.
- `CalculatePreferredSize(largest)` `U:0x100e36a7`: the base's value (content size when `largest`, else (-1, -1)) with each
  component capped at `T * 128 * scale.y - 1` = **1023** (constant 128.0 at `U:0x101bfd08`): the visible part never spans
  more than 128 tiles, so it always fits the overlay texture. (Both components use `scale.y`.)

Mappings, with `p` a point in the view's content coordinates (pixels), `m` = margins, `s` = scale:

| Mapping | Formula | Function |
|---|---|---|
| pixel → world (mouse) | `x = p.x / s.x + m.left` ; `z = H4 - p.y / s.y - m.highZ` (plus the Rect's left/top, 0 here) | `U:0x1004657d` (disasm) |
| ... snapped | `x = floor(x / T.x) * T.x` ; `z = floor(z / T.y) * T.y` | same, third argument 1 |
| world → pixel (node anchor) | `p.x = (x - m.left) * s.x` ; `p.y = (H4 - (z + m.highZ)) * s.y` | `U:0x1004666d` |
| world → pixel (markers) | `p.x = (x - m.left) * s.x` ; `p.y = (H4 - 1 - (z + m.highZ)) * s.y` (no "+1": 2 pixels higher) | `U:0x100467e8` (disasm) |
| visible rect → tile space | `(bounds / s + (m.left, m.highZ)) / T` : columns are tile x, rows are **flipped**: row `r = H - 1 - tileZ` | `U:0x100e3fe7`, `U:0x100e3be3` |

So tile `(X, Z)` occupies content pixels `x in [(4X - m.left) * 2, +8)`, `y in [(4 (H - 1 - Z) - m.highZ) * 2, +8)`.
(Mouse quirk that follows from the formulas: the top pixel row of a tile maps to the tile above, `z = 4Z + 4` exactly.)

### 1.4 Scrolling (certain except the direction)

- Scroll bars of the scroll view.
- Drag: `MouseDown` stores the point and state 2 **in every mode** whenever the press does not start a node drag (mode 2
  always; modes 0/1 on empty map, on an existing building, and in mode 1 on anything). `MouseMove` in state 2 calls
  `View::ScrollBy(point - anchor, true)` (`U:0x100e4abf`, virtual +0xe4). The anchor is not updated, which is only stable if
  the content follows the pointer (grab and drag): **inferred**, `ScrollBy`'s sign was not traced.
- `MouseUp` (`U:0x100e34e9`): state 0, `MakeFocus(false)`. `MouseDown` ends with `MakeFocus(true)` (virtual +0xac).

### 1.5 The overlay texture (certain)

- `AllocateTexture(0x80, 0x80, false, 10, 0)`; filled by `RTexture_t::Load(tex, buf, 10, 0x100 /*pitch*/, 0, 0, true)` from a
  0x8000-byte buffer of 16-bit words, zeroed first (`U:0x100e3fe7`). Format 10 is **A4R4G4B4** (randy31's table,
  `dropin/randy31/src/d3dx_blit.rs` line 273; nibbles expand as `n * 17`).
- State blob +0x4e8: ALPHABLENDENABLE (0x1b) = 1, SRCBLEND (0x13) = 5 SRCALPHA, DESTBLEND (0x14) = 6 INVSRCALPHA, stage 0
  MAGFILTER (0x10) = 1 and MINFILTER (0x11) = 1 (**point sampling**), plus `U:0x100463bb` (z test, no z write). Vertex colour
  0xffffffff. Result: `out = rgb * a + dst * (1 - a)`, `a = A / 15`.
- Window: the texture covers tile columns `org.x .. org.x + 127` and flipped rows `org.y .. org.y + 127` (+0x490/+0x494).
  `redraw(force)` `U:0x100e3fe7` computes the visible tile rect
  `vis = IRect(ftol(l), ftol(t), ftol(ceil(r)), ftol(ceil(b)))` of the tile-space rect in 1.3, and only if `force` or `vis`
  is not inside the window: `org.x = vis.left - (127 - vis.Width()) / 2`, `org.y = vis.top - (127 - vis.Height()) / 2`,
  then `org.x = max(org.x, ftol(m.left / T.x))`, `org.y = max(org.y, ftol(m.highZ / T.y))`, and the buffer is repainted.
- Tile → texel (`U:0x100e3505`, disasm): `total = ftol((Size.h + 1) / T.y)` = `H`; for a layout placed at `pos`:
  `x0 = offX + ftol(pos.x / T.x)`, `z0 = offZ + ftol(pos.y / T.y)`; rows `H-1-(z0+tilesZ-1) ..= H-1-z0`, clipped to the
  window; for each cell `(c, r)`: if `layout.GetTile(c - x0, (H-1-z0) - r)` then
  **`buf[(r - org.y) * 128 + (c - org.x)] = colour`**. I.e. texel `(u, v) = (X - org.x, (H - 1 - Z) - org.y)`.
- Texture → screen (`U:0x100e3be3`): one quad over the whole view rect with UVs `(tile-space visible rect - org) / 128`
  (constant 0.0078125 at `U:0x101bfd44`). For a port the window is only a cache: draw each tile as the 8 x 8 square of 1.3.

Paint order and colours (later ones overwrite; `U:0x100e3fe7`):

| Order | What | Word | RGBA (0..255) |
|---|---|---|---|
| 1 | every set tile of every zone (layout = zone + 4, `pos` (0,0)) | 0x400f | (0, 0, 255, 68) |
| 2 | existing nodes, not the selected one | 0x8880 | (136, 136, 0, 136) |
| 2 | ... with the demolishing flag | 0x8088 | (0, 136, 136, 136) |
| 3 | pending nodes, not the selected one, valid | 0x8080 | (0, 136, 0, 136) |
| 3 | ... not valid | 0x8800 | (136, 0, 0, 136) |
| 4 | the selected node: existing | 0x8ff0 | (255, 255, 0, 136) |
| 4 | the selected node: pending valid / not valid | 0x80f0 / 0x8f00 | (0, 255, 0, 136) / (255, 0, 0, 136) |

Only the set tiles of each (rotated) layout are painted; a tile is replaced, not blended, inside the texture.

---

## 2. A building's footprint (certain)

`HouseLayout_c` (0x28 bytes): +0 tile size (4.0, `C:0x1002a2a8`), +4 tilesX, +8 tilesZ, +0xc / +0x10 offset tile x / z,
+0x14 `vector<bool>`. A template's layout has offset (0, 0); a zone's layout has the zone's corner as offset.

**It is a per-tile mask, not a plain rectangle.** In the 0xf4268 record (little-endian, `HouseLayout_c::ReadBlob`
`C:0x10013f65`, bits by `ReadBlobFixedLen` `C:0x10013acc`), right after the house type:

| Record offset (empty name) | Type | Field |
|---|---|---|
| 46 | i32 | house type (`kind`) |
| 50 | i32 | bit count `n` (= tilesX * tilesZ in all 83 records) |
| 54 | i32 | tilesX |
| 58 | i32 | tilesZ |
| 62 | `ceil(n / 8)` bytes | the mask: tile `(x, z)` is bit `z * tilesX + x` (`GetTile` `C:0x1001368d`), bit `i` = byte `i / 8`, bit `i % 8` (low bit first) |

The Rust `Template::parse` reads the three ints and **skips the mask bytes** (`r.take(bits.div_ceil(8))`); it also skips
the guards (section 4). Both are needed for the map. In today's data only one template has clear tiles: **0x6a0000**
(HQ, 10 x 10, 75 set; mask bytes `1f 7c f0 c1 07 1f fc ff ff ff ff ff 0f`: rows z 0..4 have x 0..4 only, rows z 5..9 are
full). A port that treats the others as full rectangles is still right for them, but the code path is the mask.

Rotation, `HouseLayout_c(layout, rotation)` `C:0x10013bc9` (the registry's `GetTemplate(id, rotation)` `G:0x1012e4e4` →
`C:0x1000f0fd` makes a rotated template copy whose `GetLayout()` is this turned layout):

| Rotation | Source tile `(x, z)` goes to | Turned size |
|---|---|---|
| 0 | `(x, z)` | tilesX x tilesZ |
| 90 | `(tilesZ - 1 - z, x)` | tilesZ x tilesX |
| 180 | `(tilesX - 1 - x, tilesZ - 1 - z)` | tilesX x tilesZ |
| 270 | `(z, tilesX - 1 - x)` | tilesZ x tilesX |
| other | nothing set (all clear, sizes unswapped) | |

**Anchor: the least corner.** A node's tile (the house record's `floor(position / 4)`, or the cursor tile of a drop) is
local tile (0, 0) of the turned layout. Footprint of a node at `(ax, az)` with rotation `r`:

```
{ (ax + x', az + z') : (x', z') = turn_r(x, z), mask(x, z) set }
```

This agrees with `RotateLocalPos` (90: `(x, z) -> (Lz - z, x)`), so `Template::rotate_local` places doors and guards on the
same turned footprint. Checked with the one real house of the log: HQ 0x6a0000 at (868, 5, 1224), rotation 90, tile
(217, 306): all 75 turned tiles lie on set zone tiles and the 10 x 10 rectangle lies inside zone rectangles.

`HouseLayout_c::Intersects(other, delta)` `C:0x100138fc`, called as `A.Intersects(B, B.tile - A.tile)`: true when some
set tile of `B`, moved by `delta`, lies inside `A`'s rectangle on a set tile of `A`. In plain words: **the two masks share
a tile in world tile coordinates.** (`ContainsTilePos` `C:0x10013557`: `off <= t < off + tiles` on both axes.)

---

## 3. Zones: where building is allowed

### 3.1 Source (certain)

The zones are **not sent by the server**. They are in the playfield's city record, resource type **0xf4267**
(`RDBPlayfieldCityData_t`), instance = the playfield's resource instance (the same number as its 0xf425a dynel table;
`PlayfieldAnarchy_t::ReadPlayfield` `G:0x1012319f` builds `(0xf4267, n)` and calls `LoadPlayfieldCityData` `G:0x101226c9`,
which copies each `CityControllerData_c` into a `CityAI_c` at +0x50). GUI.dll reads them through
`PlayfieldCityHolderServer_c::GetCityHolderForPF(proxy)->GetCity(cityIdentity) + 0x50` → `CountZoneData` / `GetZoneData`.

### 3.2 Record layout (certain; little-endian; verified against the real 6010 record, 1256 bytes, nothing left over)

`RDBCityController_c::ReadBlob` `C:0x1000d03a`, `CityControllerData_c::ReadBlob` `C:0x1000cf57`:

| Offset | Type | Field | 6010 |
|---|---|---|---|
| 0 | 12 bytes | object head (type, instance, 1) | 0xf4267, 6010, 1 |
| 12 | i32 | version | 1 |
| 16 | i32 | controller count | 1 |
| 20 | i32 | tile extents x | 600 |
| 24 | i32 | tile extents z | 600 |
| per controller: | | | |
| | u32, u32 | controller dynel identity | (0xc418, 0xc000177a) |
| | u16, u16 | house level min, max | 1, 1 |
| | u32 | **purchase cost** (+0xc) | 500,000,000 |
| | u32 | **area upkeep** (+0x10) | 15,000,000 |
| | u32 `(n+1) * 1009`, n x 16 bytes | landing points: u32 id, f32 x, y, z | 1: (997.23, 5.01, 1143.13) |
| | u32 `(n+1) * 1009`, n x 16 bytes | **zones**: u16 id, u8 type, 13 bytes = 100 bits | 74 zones, all type 0 |

A zone (`ZoneData_c`, 0x2c bytes: +0 id, +2 type, +4 `HouseLayout_c`) is a **10 x 10-tile mask**: bit `z * 10 + x`, low bit
first, as in section 2. Its corner comes from its id (`C:0x1000cf57`, after reading):

```
perRow = extentsX / 10            (60 in 6010)
offX   = (id % perRow) * 10
offZ   = (id / perRow) * 10
```

6010: ids 1703, 1763, then 1821..1828, 1881..1888, ... 2301..2308 (nine rows of eight). The rectangles span tiles
x 210..290, z 280..390; **2569 tiles are set**, between x 213..284 and z 288..386. The masks are far from full (the plots
are separated by lanes, and there are single tiles at corners): see the appendix. The zone type byte's meaning is unknown
(0 everywhere here; neither the painter nor the test looks at it).

### 3.3 The test (certain)

`CityAI_c::CanBuildLayout(template, tile)` `C:0x10002882` with `HouseLayout_c::LayoutTileAllowedAtGlobalTpos`
`C:0x100139ea`, `L` = the template's **turned** layout:

```
for z in 0..L.tilesZ, x in 0..L.tilesX:            // every cell of the bounding rectangle, set or not
    ok = false
    for each zone Zn:
        gx = tile.x - Zn.offX + x ;  gz = tile.z - Zn.offZ + z
        if 0 <= gx < 10 and 0 <= gz < 10 and ( !L.mask(x, z) or Zn.mask(gx, gz) ): ok = true
    if !ok: return false
return true
```

So: every cell of the turned bounding rectangle must fall inside **some zone's 10 x 10 rectangle**, and every set cell must
be on a **set** zone tile. (An unset cell still fails where the record has no zone at all.)

Related: `CityAI_c::CanBuildAtPos(x, z)` `C:0x100027ff` = some zone has a set tile under the position; it decides which
houses belong to the city in `FillCityHouseDataVector` (the house's tile corner `(tileX * 4, tileZ * 4)`).

### 3.4 Costs (certain)

`CityClientInterface_c::GetPurchaseAreaCost` `C:0x1000a21f` and `GetAreaUpkeepCost` `C:0x1000a233` return the city's
`CityControllerData_c` +0xc and +0x10: the two u32 of the record above. Nothing from the server.

---

## 4. Markers (`U:0x100e3be3`)

After the overlay, under state blob +0x56c with the base's 32 x 32 atlas (`U:0x100462c4(viewport, this+0x128, 0)`):

1. for **every node** (existing and pending, list order), for each door of its rotated template (`GetDoorData`, 0x14-byte
   records, position at +4): marker 11;
2. then for every node, for each guard (`GetGuardData`, 0x44-byte records, position at +0x34): marker 12.

Position: `world = (tileX * T.x, 0, tileZ * T.y) + template.RotateLocalPos(local, template.GetRotation())` (`C:0x1000e0db`;
the rotated copy's rotation is the node's). That is `Template::rotate_local(local, node.rotation)` added to the footprint's
least corner. Screen: the "markers" row of 1.3.

Shape (`U:0x100470bf(viewport, pos, index)` → `U:0x100467e8(viewport, pos, u, v, 8, 8, 32, 32, 0xffffff)`, disasm): an
**8 x 8 pixel quad centred on the point** (`±4`), UV cell `u = (index % 4) * 8`, `v = (index / 4) * 8` of the 32 x 32 atlas.
The atlas (`U:0x10046d0b`) holds 13 GUI textures, each one's top-left **4 x 4 texels** copied to offset (2, 2) of its 8 x 8
cell, the rest transparent: so a marker is a 4 x 4-pixel dot. Blend: ALPHABLENDENABLE 1, SRCBLEND 2 (ONE), DESTBLEND 6
(INVSRCALPHA), stage 0 ALPHAOP 2 (SELECTARG1), ALPHAARG1 2 (TEXTURE): `out = tex.rgb + dst * (1 - tex.a)`.

| Index | GUI texture id | Name (AFCM.dll's GFX table) | Used for |
|---|---|---|---|
| 11 | 0xfd | `GFX_GUI_MAPDOT_BLUE` | **doors** |
| 12 | 0x100 | `GFX_GUI_MAPDOT_RED` | **guards** |

(Whole atlas, index 0..12: `MAPSQUARE_CLAN` 0x102, `_OMNI` 0x106, `_NEUTRAL` 0x105, `_MONSTER` 0x104, `_SHOP` 0x108,
`MAPDOT_CYAN` 0xfe, `MAPFLAG_CYAN` 0x101, `MAPSQUARE_TEAMMEMBER` 0x109, `_MISSION` 0x103, `_SELECTED` 0x107, `MAPDOT_GREEN`
0xff, `MAPDOT_BLUE` 0xfd, `MAPDOT_RED` 0x100.) The pixels of the two dots were not read.

Template fields (record, little-endian, after the visuals list; counts are `(n + 1) * 1009`):

| List | Record | Fields |
|---|---|---|
| doors (already parsed by Rust) | 20 bytes | u32 item template, f32 x, y, z (local), i32 rotation (negated when read) |
| **guards (skipped by Rust: `r.take(24)`)** | 24 bytes | 4 bytes name code (e.g. `MAGU`, `XEPR`), i32 level, **f32 x, y, z (local)**, i32 rotation (negated when read) |

38 of the 83 templates have one door, 45 none; 11 have guards (the HQ 0x6a0000 has two, at (18.96, 0.40, 26.51) and
(19.07, 0.40, 31.62)). Markers are drawn for existing buildings too, and regardless of mode.

---

## 5. Mouse

`MouseDown` `U:0x100e56a8(point, ...)`: the two button arguments are never read (disasm): any button.

1. Mode 2: anchor = point, state 2 (scroll drag), `MakeFocus(true)`. Nothing else.
2. Mode 0 **and** `View::GetDragObject()` non-null: the drop handler (section 6). It returns without touching the selection.
3. Otherwise (modes 0 and 1): hit test, then
   - if the node found differs from the selection: `U:0x100e39d6(node)` and `redraw(1)`;
   - mode 0 and a node was found and it is **pending**: anchor = `point - U:0x1004666d(node.tile * T)` (the pointer's
     offset from the footprint's least corner on screen, i.e. its bottom-left corner), state 1;
   - else anchor = point, state 2 (scroll drag).
   - `MakeFocus(true)`.

Hit test `U:0x100e3afe` (disasm): `w` = pixel → world, **snapped** (1.3). For each node in list order (existing buildings
first, then pending in the order dropped; first match wins): `d = w - node.tile * T`; if `d.x >= 0` and `d.y >= 0`:
`t = (ftol(d.x / T.x), ftol(d.y / T.y))`; hit if the node's turned layout `ContainsTilePos(t)` **and** `GetTile(t)`. So only
set tiles of the mask are clickable. No hit → the list head ("nothing").

Selecting `U:0x100e39d6(node)`: nothing if it is already selected; the head → deselect, signal(0); a node whose
**demolishing flag is set** → ignored (the old selection stays); else select, signal(node).

- Clicking empty map deselects.
- **Existing buildings can be selected in both mode 0 and mode 1**, pending ones too. The buttons decide what can be done
  (`U:0x100e7ec9`): Rotate and Delete need a pending selection, Demolish an existing one that is not being demolished.
  Nothing is selectable in mode 2.

`MouseMove` `U:0x100e4abf(point)`:

- state 1 and something selected: `w = pixel→world(point - anchor, snapped)`; `node.tile = (ftol(w.x / T.x), ftol(w.y / T.y))`
  (`U:0x100e8fca`); `U:0x100e44a2()`; `redraw(1)`. The node's **least corner** follows the pointer in 8-pixel steps (floor).
  No bounds check: it can be dragged anywhere (it just turns invalid).
- state 2: `ScrollBy(point - anchor, true)`.

`MouseUp` `U:0x100e34e9`: state 0, `MakeFocus(false)`. A mode change to 1 or 2 while dragging a node ends the drag
(`U:0x100e34ac`); it does not clear the selection.

---

## 6. Dropping an inventory item (`U:0x100e56a8`, mode 0 only)

Drag object reading `U:0x1003d6f2(drag, &item_type, &item_id, &container_id, &split_count)`: mime must be
`inventory/item`; message fields `container_id` (Identity), `item_type` (int32), `item_id` (Identity), `split_count`
(int32, optional). Only `item_type` and `item_id` are used here.

Where they come from: `U:0x1003d4a4` (called by the inventory's pick-up `U:0x100cc6e8`) builds the drag from the
`InventoryListViewItem_c`: `item_id` = the list item's Variant at +0x20 as an Identity, `item_type` = its int at +0xf4,
plus `container_id`, `container_position`, `split_count`.

**The identity in the build request is that `item_id`: the item's place identity, `(0x68, slot)` for an item in the
character's inventory.** Evidence: (a) the same `item_id` is what the NPC trade bar passes to
`N3Msg_NPCChatAddTradeItem` (`U:0x10057adc`), captured on the wire as `(0x68, slot)`; (b) `N3Msg_GetSkill(Identity, stat,
int, Identity)` `G:0x100270a3` treats an identity whose type is 101..249 as "an item of the client character's
containers" (`G:0x10049794`), which is how stat 620 is read from it; (c) the map listens to the global "item changed
identity" signal and rewrites a pending node's item (`U:0x100e46fc`), which only makes sense for place identities. Not
traced: the inventory window code that fills +0x20. It is not the item's template ids.

Order of checks:

| # | Check | On failure |
|---|---|---|
| 1 | `item_type == 1` (0 when the drag is not `inventory/item`) | `Cancel()`, dialog 10000 `OnlyDropHousesOnMap` |
| 2 | `tmpl = (0xf4268, N3Msg_GetSkill(item_id, 620, 2, (0,0)))`; the registry has it (`AOCityHouseTemplateRegistry_c` virtual +8 `G:0x1012e547`: registered, or the record exists in the resource database) | same |
| 3 | `side = N3Msg_GetSkill(item_id, 33, 2, (0,0))`; if `side != 1234567890` and `side != N3Msg_GetSkill(33, 2)` (own) | `Cancel()`, dialog 10002 `CityTerminal_HouseCanOnlyBeBuiltBy` fed with text (2005, side) |
| 4 | `U:0x100e4d71(point, &item_id, &tmpl)`: the limits of section 8 | a limit dialog, no node; **the drag is still `Accept(2)`ed** |
| | success | node added, `Accept(2)` |

All dialogs: `DialogBox_c` titled "Error", one button with text 10000 `MsgBox_OK`.

The new node (`U:0x100e4d71`, disasm): `w = pixel→world(point, snapped)`; **tile = `(ftol(w.x / T.y), ftol(w.z / T.y))`:
the tile under the cursor is the anchor** (least corner); rotation 0; item = `item_id`; template = `tmpl`. Then
`U:0x100e4cef(node, 1)`: appended to the list, `count[type]++` (type < 10), the item **locked**, `U:0x100e44a2()`,
`redraw(1)`. The new node is not selected.

"Marked in use" `U:0x1003d103(item)` on the `InventoryItemLocker_c` singleton (`U:0x100092ce`, ctor `U:0x1003d1f6`): adds
the identity to the locker's set and emits its signal `(item, 1)`. It is the same lock as the wear pages' Lock Item
(`IsLocked` `U:0x1003d070`): a locked item cannot be picked up, clicked or dropped on (`INVENTORY_ORIGINAL.md`). Unlock is
`U:0x1003cfc1` (signal `(item, 0)`), called for a node's item whenever the node is removed (`U:0x100e479f`) and for all
remaining pending nodes in the destructor (`U:0x100e4bec`).

Two global item signals keep nodes in step: `U:0x100e46fc(old, new)` → the node with item `old` gets `new`;
`U:0x100e4b49(item)` → that node is removed (with recompute and redraw).

---

## 7. The buttons

### Construct `U:0x100e498a`

Nothing unless the "can construct" byte (+0x4e4) is set. Then, with the city from the holder:

1. The **first pending node** (list order) whose rotated template `IsHQ()` (house type 1): send, remove, stop looking.
2. Every remaining pending node, in list order: send, remove.
3. `redraw(1)`.

Send = `U:0x100e90fc` → `CityAI_c::ClientRequestBuild(character, node.template, node.tile, node.rotation, node.item)`
(`C:0x10002a0c`) → `G:0x1012e62f` → `ClientRequestBuildIIR_c` (`G:0x101300a6`, writer `G:0x10130053`):
`i32 cityInstance`, `Identity template`, `i32 tileX`, `i32 tileZ`, `i32 rotation`, `Identity item`; the character's identity
is in the header.

Remove = `U:0x100e479f(node, 0)`: deselect if selected, `count[type]--`, **unlock the item**, erase. No recompute: the
"can construct" byte stays set (stale) until the next `U:0x100e44a2`, e.g. when the server's "house added" arrives; a
second press then finds no pending nodes and sends nothing. The HQ count also drops to 0 until the server answers.
Nothing else is done to the items locally (the server takes them).

### Rotate `U:0x100e4843`

Selected and pending: `rotation = (rotation + 90) % 360` (`U:0x100e8fb6`), `U:0x100e44a2()`, `redraw(1)`. The tile stays:
the footprint turns about its least corner's tile (sizes swap).

### Delete `U:0x100e4882`

Selected and pending: `U:0x100e479f(node, 1)`: deselect (signal 0), `count[type]--`, unlock the item, erase, recompute,
redraw.

### Demolish `U:0x100e48bb`

Selected and **existing**: `U:0x100e9146` → `CityAI_c::ClientRequestDemolish(character, node.tile)` → `G:0x1012e695` →
`ClientRequestDemolishIIR_c` (writer `G:0x10130312`): `i32 cityInstance`, `i32 tileX`, `i32 tileZ`: **the node's tile as
stored** (the house record's `floor(position / 4)`). Then deselect (`U:0x100e39d6(head)`) and `redraw(1)`. The node stays
until the server's signals: demolish started `U:0x100e4748` (flag set, `redraw(1)`), house removed `U:0x100e4b98`
(`U:0x100e479f(node, 1)`), both matched by tile.

### Existing houses

`U:0x100e5b50` (ctor): `FillCityHouseDataVector(cityID)`; all existing nodes are dropped, then one node per record: item
(0,0), template = record +0x14, tile = `GetTilePos`, rotation = record +0xc, demolishing = record byte +0x10. House added
(`U:0x100e5b06`): the same for one record, appended.

### Enabling, mode, texts

- Construct: `U:0x100e7eb2(bool)` from the +0x468 signal. Rotate, Delete, Demolish: `U:0x100e7ec9(Node*)` from the +0x46c
  signal (section 5). All four start disabled (`U:0x100e8108`).
- "Can construct" `U:0x100e44a2` (disasm):
  1. `flag = false`; every pending node: `valid = 1`, `flag = true`.
  2. Every pair `A` before `B` in the list (existing ones included): if `A.layout.Intersects(B.layout, B.tile - A.tile)`:
     `A.valid = 0`, `B.valid = 0`, `flag = false`.
  3. `count[1] == 0` (no HQ existing or pending) → `flag = false`.
  4. Every pending node still valid: if `!CanBuildLayout(rotated template, tile)`: `valid = 0`, `flag = false`.
  5. If the flag changed: store, emit.
- Mode `U:0x100e7f62(mode)` (tab change `U:0x100e7169`: tab 1 → 0, tab 2 → 1): the control view is removed from its
  parent in mode 2 and added back otherwise; mode 0: `InfoTextView` = 10002 `CityTerminal_BuildMapInfoText`,
  `ButtonSelector` page 0 (Rotate, Delete, Construct); mode 1: 10002 `CityTerminal_DemolishInfoText`, page 1 (Demolish);
  then the map's mode = 2 if read-only, else the mode (`U:0x100e34ac`). Pending nodes and the selection survive a tab
  change (both tabs are the same view).

---

## 8. House types and limits (certain)

The type is `CityHouseTemplate_c::GetHouseType()` = template +4 = the record's i32 right before the layout: **the Rust
`Template::kind`**. Counts (+0x4bc + 4 * type) include existing and pending nodes.

`U:0x100e4d71`, tested in this order after the node is built (text keys in category 10000, disasm):

| Type | Refused when | Text key |
|---|---|---|
| 1 HQ | `count[1] >= 1` | `AlreadyHaveHQhere` |
| 3 grid | `count[3] >= 1` | `AlreadyHaveGridHere` |
| 5 uplink | `count[5] >= 1` | `AlreadyHaveUplinkHere` |
| 6 cloaking device | `count[6] >= 1` | `AlreadyHaveCloakDev` |
| 2 market | `count[2] >= 1` | `AlreadyHaveMarket` |
| 2 market | else, the playfield **model identity type** (+0x47c) `== 0xc79e` | `NoMarketPFType` |
| 8 tax office | `count[8] >= 1` | `AlreadyHaveTaxOfficeHere` |
| 4 guard house | `count[4] >= 1` | `AlreadyHaveUniqueHouse` |
| 9 unique | some node (existing or pending) has the **same template identity** (`U:0x100e398f`) | `AlreadyHaveUniqueHouse` |
| 0, 7, others | never | |

Templates by type in today's data: 0: 31, 1: 10, 2: 9, 3: 3, 4: 3, 5: 3, 6: 3, 7: 3, 8: 3, 9: 15.

---

## 9. Anything else shown

- **Tooltip**: the base's `ProvideToolTipText` `U:0x100478c7` is inherited: title = text 10000 `PointerPosition`, body =
  `"%.1fx%.1f"` of the world x and z under the pointer (pixel → world, not snapped). Whether the tooltip machinery asks
  this view was not traced.
- No names, no legend, no per-building hover text, no highlight on hover.
- Info text and button pages: section 7. In mode 2 (buy dialog's `CityMap` tab, the inspect panel, or read-only) the
  control view is not shown at all.

---

## Certain and guessed

Certain (read from code, constants or records):

- Base class, field offsets, every formula of sections 1.3 and 1.5, scale 2 px per unit, 8 px per tile, +z up.
- Background = the base ground renderer (the PF Map's), not an image.
- Overlay format A4R4G4B4, point sampled, SRCALPHA / INVSRCALPHA, the colour words and their paint order.
- Footprint = per-tile mask, record offsets, rotation table, least-corner anchor, `Intersects`.
- Zones: record type 0xf4267 / playfield instance, full layout (parsed exactly on the 6010 record), offset from id,
  `CanBuildLayout` including the rule for unset cells; the two costs.
- Markers: which lists, positions, 8 x 8 quad, atlas cells 11 and 12 = `GFX_GUI_MAPDOT_BLUE` / `_RED`, blend.
- Hit test, node drag, selection rules per mode, any mouse button.
- Drop: fields read, check order, text keys, cursor tile as anchor, the lock/unlock calls, `Accept(2)` even after a limit
  dialog.
- Construct / Rotate / Delete / Demolish, request arguments, the stale "can construct" byte.
- Limits table and its order.

Guessed or not traced:

1. **`(0x68, slot)` as the item identity**: strongly supported (section 6) but the code that fills the inventory list
   item's +0x20 was not read; an item inside a bag would carry that bag's container type instead of 0x68.
2. `ScrollBy`'s sign for the drag scroll (assumed grab and drag).
3. The pixels of the blue and red map dots (only their names and the 4 x 4 size are known).
4. The last quad of `Render` (`U:0x10047101`) having no visible effect.
5. The zone type byte (always 0 in 6010) and whether other city playfields use other values.
6. What the inventory shows after `Accept(2)` (its handler `U:0x100cbcb5` was not read) beyond the lock.
7. Whether the pointer-position tooltip actually appears over this view.
8. What playfield kind has model identity type 0xc79e (no market there).
9. Only one real house (the log's first) was available to check footprint against zones; the other four in
   `city/houses.hex` are made-up test houses and do not fit the zones.

## Appendix: playfield 6010's zone mask

Tiles x 210..289 left to right, z 389 at the top down to 280. `#` set (buildable), `.` clear inside a zone's rectangle,
blank = no zone there.

```
389 ................................................................................
388 ................................................................................
387 ................................................................................
386 ...#......................................................................#.....
385 ....##..................................................................##......
384 ....##..................................................................##......
383 .........#...............#..#....................#..#...............#...........
382 ....##....###############....####################....###############....##......
381 ....##....###############....####################....###############....##......
380 ..........###############....####################....###############............
379 ....##....###############....####################....###############....##......
378 ....##....###############....####################....###############....##......
377 ..........###############....####################....###############............
376 ....##....###############....####################....###############....##......
375 ....##....###############....####################....###############....##......
374 ..........###############....####################....###############............
373 ....##....###############....####################....###############....##......
372 ....##....###############....####################....###############....##......
371 ..........###############....####################....###############............
370 ....##....###############....####################....###############....##......
369 ....##....###############....####################....###############....##......
368 ..........###############....####################....###############............
367 ....##....###############....####################....###############....##......
366 ....##....###############....####################....###############....##......
365 ..........###############....####################....###############............
364 ....##....###############....####################....###############....##......
363 ....##....###############....####################....###############....##......
362 .........#...............#..#.....#........#........................#...........
361 ..................................#........#....................................
360 ..................................#........#....................................
359 .................................#..........#...................................
358 ......#.....#..#.....#..#.......#............#.......#..#.....#..#.....#........
357 .......#####....########..............................########....#####.........
356 .......#####....#####....................................#####....#####.........
355 .......#####....#####....................................#####....#####.........
354 .......#####....#####....................................#####....#####.........
353 .......#####....########..............................########....#####.........
352 ......#.....#..#.....#..........#............#.......#..#.....#..#.....#........
351 .................................#..........#...................................
350 ..................................#........#....................................
349 ......#..........#..#..........#..............#..........#..#..........#........
348 .......##########....##########................##########....##########.........
347 .......##########....##########................##########....##########.........
346 .......##########....##########...#........#...##########....##########.........
345 .......##########....#############..........#############....##########.........
344 .......##########....##########................##########....##########.........
343 .......##########....##########................##########....##########.........
342 .......##########....##########................##########....##########.........
341 .......##########....#############..........#############....##########.........
340 .......##########....##########...#........#...##########....##########.........
339 .......##########....##########................##########....##########.........
338 ......#..........#..#..........#..............#..........#..#..........#........
337 ................................................................................
336 ................................................................................
335 ......#...............#..#..#..#..............#..#..#..#...............#........
334 .......###############....##.##................##.##....###############.........
333 .......###############....##.##................##.##....###############.........
332 .......###############...#..#..#..............#..#..#...###############.........
331 .......###############............#........#............###############.........
330 .......###############...........#..........#...........###############.........
329 .......################.........#............#.........################.........
328 .......###############..................................###############.........
327 .......###############..................................###############.........
326 .......###############..................................###############.........
325 .......################.........#............#.........################.........
324 .......###############...........#..........#...........###############.........
323 .......###############............#........#............###############.........
322 .......###############...#..#..#..............#..#..#...###############.........
321 .......###############....##.##................##.##....###############.........
320 .......###############....##.##................##.##....###############.........
319 ......#...............#...##.##................##.##...#...............#........
318 ..........................##.##................##.##............................
317 ..........................##.##................##.##............................
316 ......#...............#...##.##................##.##...#...............#........
315 .......###############...#..#..#..............#..#..#...###############.........
314 .......###############............#........#............###############.........
313 .......###############...........#..........#...........###############.........
312 .......################.........#............#.........################.........
311 .......###############..................................###############.........
310 .......###############..................................###############.........
309 .......###############..................................###############.........
308 .......################.........#............#.........################.........
307 .......###############...........#..........#...........###############.........
306 .......###############...#..#..#..#........#..#..#..#...###############.........
305 ......#...............#...##.##................##.##...#...............#........
304 .........................#..#..#..............#..#..#...........................
303 ................................................................................
302 ...#..............................#........#..............................#.....
301 ................................................................................
300 ................................................................................
299                     ..........                                                  
298                     ..........                                                  
297                     ..........                                                  
296                     ..........                                                  
295                     ..........                                                  
294                     ..........                                                  
293                     ..........                                                  
292                     ........##                                                  
291                     ..........                                                  
290                     ..........                                                  
289                     ..........                                                  
288                     ....##....                                                  
287                     ..........                                                  
286                     ..........                                                  
285                     ..........                                                  
284                     ..........                                                  
283                     ..........                                                  
282                     ..........                                                  
281                     ..........                                                  
280                     ..........                                                  
```
