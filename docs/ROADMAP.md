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
