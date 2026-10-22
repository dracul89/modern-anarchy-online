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

## What works (the live client)
Log in to a real server with an account of your own. The credentials come from
the environment only, never from arguments or files that are committed:
`MAO_USER` and `MAO_PASSWORD` (a `.env` in this folder is git-ignored; load it
with `set -a; . ./.env; set +a`). `MAO_SERVER=host:port` overrides Rubi-Ka.
```
mao-client net <game> chars                       the account's characters
mao-client net <game> enter <name> --live         the live window: walk the world
mao-client net <game> enter <name> [secs] [--show [--image out.png]]   a snapshot of the world
mao-client net <game> walk <name> <x> <z> [secs]  headless: walk a line, report zoning
mao-client net <game> attack <name> [secs] [x z ...]   headless: patrol and fight a creature
mao-client net <game> hop <name> <addr>           log in as a redirect does, report the area
```
Live window: W/Up, S/Down walk, A/D or Left/Right turn, Space jump, C toggles
walking through props, drag to orbit the camera, scroll to zoom. `MAO_TRACE=1`
prints the server's non-bulk messages and the walked position.

What the client does today: logs in (the Diffie-Hellman and TEA exchange of the
original), enters a character, loads the playfield (ground, props, collision),
shows players, NPCs and creatures with their meshes, clothes and walk cycles,
walks and jumps with the server accepting the movement, zones through whompahs
and shop doors (including the 0x3c redirect to another game server), answers
pings, and sends the original's select/target/attack messages.

Hard-won protocol facts (see `crates/mao-net` and the notes in the other repo):
- ready (`570c2039`) goes out only after the world burst has arrived;
- every login starts SITTING: the server ignores movement until the client
  sends the stand-up action (`5e477770`, action 0x57);
- the address in a 0x3c redirect is not a login server: log in again at the
  normal one and take the ticket it hands out;
- a creature's mesh and animations come from its NPC template (object type
  0xfde97), whose id sits in the entity message;
- too many logins in a row are throttled (system message 0xd): wait a few minutes.

## Layout
- `mao-formats` – the resource database (c-tree, 64-bit positions), textures,
  object archives, meshes, tile-maps, terrain, statels, playfields
- `mao-net` – the network protocol: framing, login crypto, system and N3 messages, sessions
- `mao-render` – the `wgpu` renderer
- `mao-client` – window, input, the viewer
- `mao-tools` – `mao-db`

Tests that need game data run when `MAO_GAME` points at an installed game.
See `docs/ROADMAP.md` for findings and what's next.
