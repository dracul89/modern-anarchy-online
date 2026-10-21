# Modern Anarchy Online (MAO)

A new 64-bit, multiplatform client for Anarchy Online: Windows, Linux x86-64
and Apple-silicon macOS, rendering through `wgpu`.

This repository is the clean build. Its source of truth for how the original
client behaves is the sibling repository `AnarchyReimagined` (ABI-faithful Rust
drop-ins of the original DLLs, checked against them with difftests). Read from
it; don't copy its 32-bit, Windows-only scaffolding here.

Game assets are never committed: tools take the path of an installed game.

## Layout
- `mao-formats` – the resource database and the objects in it
- `mao-net` – the network protocol
- `mao-render` – the `wgpu` renderer
- `mao-client` – window, input, game loop

See `docs/ROADMAP.md`.
