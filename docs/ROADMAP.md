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
