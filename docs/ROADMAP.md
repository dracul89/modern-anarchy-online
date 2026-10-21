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
