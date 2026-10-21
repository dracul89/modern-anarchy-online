# Modern Anarchy Online (MAO)

A new 64-bit, multiplatform client for Anarchy Online: Windows, Linux x86-64
and Apple-silicon macOS, rendering through `wgpu` (Vulkan, Metal, Direct3D 12).

This repository is the clean build. Its source of truth for how the original
client behaves is the sibling repository `AnarchyReimagined` (ABI-faithful Rust
drop-ins of the original DLLs, checked against them with difftests). Read from
it; don't copy its 32-bit, Windows-only scaffolding here.

Game assets are never committed: the tools take the path of an installed game
(the folder with `cd_image`).

## What works (the offline world explorer)
```
mao-client <game> list [name]              the playfields (zones), with what's drawable
mao-client <game> map "Old Athen"          a playfield's ground and its static objects, in a window
mao-client <game> map 596 --image out.png  the same, rendered headless to a PNG
mao-client <game> f6951 1893               a single mesh by database type and instance (hex)
```
Window controls: drag to orbit, scroll to zoom, WASD to move the view point,
Q/E down and up, Esc to quit. `MAO_CAMERA=x,y,z,distance,yaw,pitch` places the
camera for headless renders; `MAO_NO_PROPS=1` draws only the ground.

336 playfields draw (ground, textures, and their statels: trees, rocks,
buildings, 2 million objects across the game); the other 294 are interiors and
other layouts not read yet. `mao-db` (in `mao-tools`) inspects the database:
`info`, `verify`, `list`, `get`, `texture`, `mesh-preview`, `dump-mesh`, ...

## Layout
- `mao-formats` – the resource database (c-tree, 64-bit positions), textures,
  object archives, meshes, tile-maps, terrain, statels, playfields
- `mao-net` – the network protocol (empty so far)
- `mao-render` – the `wgpu` renderer
- `mao-client` – window, input, the viewer
- `mao-tools` – `mao-db`

Tests that need game data run when `MAO_GAME` points at an installed game.
See `docs/ROADMAP.md` for findings and what's next.
