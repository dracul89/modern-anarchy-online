# Roadmap

1. **Data**: open the real `ResourceDatabase.dat` (path given on the command
   line), list its keys, decode a mesh and a texture. Origin: the c-tree block
   reader and the 0xf69xx object formats in `AnarchyReimagined`
   (`dropin/databasecontroller`, `dropin/displaysystem/src/rdb*.rs`).
2. **A window**: `wgpu` + `winit` drawing one decoded mesh, on all three platforms.
3. **Network**: the login and game protocol (`dropin/messageprotocol`, `connection`).
4. **World**: playfields and ground tile-maps, characters, animation.
5. **Interface and play**.

Every format reader records, in its module docs, which original function it
was derived from.

## Findings
- `ResourceDatabase` holds 460,193 objects in 16 one-GiB segments; records
  sit at positions up to 0x3fee582ec (~16 GiB). The original client opens the
  files without c-tree's huge-file mode, so it cannot reach positions past
  4 GiB; this reader uses the full 64-bit positions.
- 6,910 index entries (almost all type 0xf6954 textures) point at a position
  that holds no such record. The original client fails on the same keys too
  (it falls back to types 0xf6960/0xf6961); treat them as absent.
- Textures (types 0xf6954, 0xf6959, 0xf695b, 0xf6960, 0xf6961, 0xf6963,
  0xf6964, 0xf6967, 0xf6968): every readable one (32,892) decodes: 24,598
  JPEG and 8,294 PNG, none BMP. `mao-db texture <type> <instance> <out.png>`.
- Meshes (types 0xf6951, 0xf696a): the object is a small head and an object
  archive (`archive.rs`) of frames. All 11,896 parse. The classes involved are
  few (RRefFrame_t, RTriMesh_t, FAFTriMeshData_t, SimpleMesh, TriList,
  FAFMaterial_t, RDeltaState, FAFTexture_t, AnarchyTexCreator_t, animations,
  lights, collision shapes); `model.rs` turns frames into drawable parts. Every
  vertex buffer is format 0x152 (XYZ, normal, diffuse, one texture pair, 36
  bytes). World matrices put the tree and lamp meshes Z-up; character meshes
  don't assemble yet (their frames animate; to do with the animations).
  `mao-db mesh-preview` draws wireframes for checking.

## Milestone 2 (done): a window
`mao-client <game> <type> <instance>` opens a `wgpu` window with an orbit
camera (drag, scroll); `--image out.png` renders headless. One WGSL pipeline
draws every mesh (position, normal, one texture pair); textures come from the
resource database through the original's fallback order (0xf6954, 0xf6960,
0xf6961); leaves and other cut-outs are alpha-tested. Checked on the tree
(f6951:1893) and a turret (f6951:19a8) on an AMD GPU through Vulkan.

## Milestone 3 (partly done): terrain
`mao-client <game> map <playfield>` draws a playfield's ground. 336 of the 461
tile-maps (type 0xf4249; the instance is the playfield number) are the older
`CHGA` layout and decode: an object archive holding `AnarchyGroundDataDB_t`,
whose patches carry zlib-packed height and tile maps (see `terrain.rs`). Ground
textures are 0xf6966/0xf6965/0xf6956 objects (low to high quality) whose
instance comes from the tile-map's own list. Playfield 0x254 (97x97 points)
shows grass, dirt, rock, cliffs and fences. 125 tile-maps use the newer
`GNDA`/`UOHA` chunk layout, not read yet. The height scale (0.2 per 8-bit step)
is a guess that looks right.
- The 125 `GNDA`/`UOHA` tile-maps (instances such as 0x79, 0x7e, 0x132) parse
  into chunks (`tilemap::parse_new`): `DCGA` and `DHGA` and `HCDA` are 8-bit PNGs the
  size of the map, `IRHA` a table of words, `TAHA` lists plus an 8-bit PNG,
  `XTHA` an RGB PNG (tile layers?) and `HSTA` per-type scales. What each means
  for the ground isn't worked out; they look like indoor/dungeon maps (DHGA is
  room-shaped). `mao-db tilemap-new <playfield> <dir>` dumps the images.
- Playfield contents (types 0xf424d with the playfield in the instance's top
  half, 0xf4254, 0xf4248, ...) are not decoded: 0xf424d records are packed
  bytes behind a small head (not zlib); the format lives in code the drop-ins
  never ported (the statel/dynel loaders in N3/Gamecode).

## Milestone 4 (partly done): props on the ground
A playfield's static objects ("statels") are in `cd_image/data/statels/<n>.pf`,
not in the database (`statels.rs`; read from `N3.dll`'s `n3StatelController_t`
and zone readers). Each statel is a position (Y-up world), packed flags, the
instance of its mesh (type 0xf6951), a scale byte and optional extras. Playfield
100 has 474 and every one names a mesh that exists in the database, which is
what confirms the record layout. `mao-client <game> map <playfield>` (decimal)
draws terrain plus props. 596 shows rocks, trees and bushes in place.
Rotation is unpacked from the flags for the common layout (a pitch, yaw and
roll in degrees); statels with flag bit 0 (a second layout that also squashes
the scale) get the identity rotation. Zones of the other layout (a packed block
and lights) and the room objects at the end of each zone aren't read.
- Statels with flag bit 0 now get their rotation about Y and a stretch along X
  from the flags; the extra matrix the original also builds for them is not
  applied. Props are drawn as instances of one copy of each mesh, and the
  window has WASD/QE movement.

## MVP: the offline world explorer (reached)
`mao-client <game> map "<name>"` opens a playfield by name or number in a
`wgpu` window with ground, textures, and every static object. Statels turned
out to be fully readable once the zone's first list was read as 16-bit values
(not 32): all 515,020 zones in the 624 readable files read exactly to their
ends (2,067,644 statels; 5 files unreadable) and all but a few dozen of the
statels name meshes that exist in the database.
Not in the MVP: interiors and the 294 playfields with other tile-map layouts,
characters and animation, lights, sky, water, sound, UI, and the network.
