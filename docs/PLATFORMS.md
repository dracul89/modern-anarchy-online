# Linux and macOS

The client is developed and played on Windows. It also builds for Linux x86-64 (`x86_64-unknown-linux-gnu`) and
Apple-silicon macOS (`aarch64-apple-darwin`): both type-check clean (`cargo check --workspace --all-targets`, the same
warnings as on Windows), but **neither has been linked, run or played on a real Linux or Mac machine yet**. Everything
below that is about run time is what the code does, not something seen working.

## What differs by platform

| Part | Windows | Linux / macOS |
|---|---|---|
| Rendering | wgpu: Direct3D 12 or Vulkan, as wgpu picks | wgpu: Vulkan on Linux (OpenGL ES as wgpu's fallback), Metal on macOS. No code changes. |
| Window and input | winit | winit (X11 and Wayland on Linux: `egui-winit`'s `x11` and `wayland` features) |
| Sound and music | rodio / cpal on WASAPI | rodio / cpal on ALSA (Linux; PulseAudio and PipeWire through their ALSA plug-ins) and CoreAudio (macOS). No code changes. |
| In-game web pages (item store, market, customer service, daily rewards) | WebView2 through `wry`, with the game's headers passed on by `ureq` | **Not available.** The window opens with a line saying so and the page's address. `wry` is a `cfg(windows)` dependency (`ureq` is on every system: the launcher downloads the dimension list with it). |
| Screenshot key | the window as it shows on the screen (`xcap`), web page included | the scene and the HUD rendered offscreen (the fallback Windows uses when the capture fails). `xcap` is a `cfg(windows)` dependency. |
| `mao-client <game> web store\|market` (the page alone in a window) | yes | compiled out |
| Interface font (Verdana) | `%WINDIR%\Fonts\verdana.ttf` | macOS: `/System/Library/Fonts/Supplemental`, `/Library/Fonts`, `~/Library/Fonts`. Linux: the Microsoft core fonts packages' folders (`/usr/share/fonts/truetype/msttcorefonts`, `/usr/share/fonts/msttcore`, `/usr/share/fonts/TTF`, ...) and `~/.local/share/fonts`, `~/.fonts`; `Verdana.ttf` or `verdana.ttf`. Without it, egui's own font, as on a Windows without Verdana. |
| Settings, hotbars, crash.log, screenshots, chat logs | `%APPDATA%\ModernAnarchyOnline` | `$XDG_CONFIG_HOME/ModernAnarchyOnline`, else `~/.config/ModernAnarchyOnline` (macOS too). `MAO_SETTINGS` or a `mao.ini` in the working folder override it on every system. |
| Screenshot file names | local time | local time when the `time` crate can read the offset; on Linux and macOS it refuses once threads are running, so the name falls back to the UTC stamp the chat log uses. |

## The game folder and file-name case

The client needs an installed Anarchy Online folder, the one holding `cd_image` (and `version.id`, `Setupf`). It reads the
resource database, sounds, music, GUI files, FXS scripts and the rest from it; nothing is bundled. On Linux or macOS that
folder comes from:

- a Windows install copied over (the whole folder; only `cd_image`, `Setupf` and `version.id` are read),
- Steam with Proton (`~/.steam/steam/steamapps/common/Anarchy Online`, the Flatpak Steam's
  `~/.var/app/com.valvesoftware.Steam/.local/share/Steam/...`), or Wine / CrossOver (`drive_c/Funcom/Anarchy Online`).
  The client only reads the files; the original client and its patcher don't need to run.

The game's files have mixed-case names (`cd_image/sound/sfx/player`, `Setupf/gfxtweak.bin`,
`gui/Default/Graphics.uvga`), and the original client, and this one, open many of them by names built in code or taken
from the game's own data, with `\` between folders. Windows takes `sfx\Player\x.wav` and `sfx/player/x.wav` as the same
file; a case-sensitive file system (Linux's, and macOS's when formatted case-sensitive) takes neither.

So every name the client builds into the game folder, a skin folder or the AOU mod goes through
`mao_formats::files::find` (and its `read`, `read_to_string`, `open` and `exists`): the path is used as written when it
exists; otherwise each part that isn't there is matched against its folder's entries ignoring ASCII case, and `\` counts
as a folder separator. A part with no match is kept as written, so a missing file stays missing. On Windows `find`
returns the path unchanged, so the Windows client behaves exactly as before. The resolving costs one `read_dir` per
mismatched part, only on the lookups that miss.

The places that go through it: the sound engine's bank and sample files, `music/env/anarchy.sws` and its layers'
samples, the FXS scripts (`twk/...` and their `!include`s), the GUI's views, option panels, action menus, preference
files and colours (game, bundled and AOU skins), `gfx` pictures, bitmap fonts, help and tip pages, the voice effects,
`text.mdb`, statels (`data/statels/<n>.pf`), the planet maps, `Setupf` files, `version.id` and the resource database's
folder. The bundled `assets/` folder is ours and already matches what the code asks for.

Not converted: the developer commands and environment-variable tools that take a full path from the command line
(`MAO_GFX`, `gfx ...` subcommands, capture logs), which open exactly what they're given.

## Running

- **Linux:** a Vulkan driver (Mesa's RADV / ANV, or NVIDIA's), `libasound2` (ALSA), and X11 or Wayland
  (`libxkbcommon`; winit loads the X11 / Wayland libraries at run time). Optional: Verdana (`ttf-mscorefonts-installer`
  on Debian/Ubuntu).
- **macOS:** 11 or newer on Apple silicon (Metal). The binary isn't signed or notarized: clear the quarantine flag
  (`xattr -dr com.apple.quarantine ModernAnarchyOnline`) or allow it in System Settings, Privacy & Security.
- Start it with `./play.sh [game folder]` from the package (below), or `mao-client "<game folder>" play`. `play.sh` takes
  the folder from its argument, else `$MAO_GAME`, else the first of the usual Steam, Flatpak Steam, Wine and CrossOver
  places that has `cd_image`. A Steam library on another drive (listed in `libraryfolders.vdf`) isn't searched: pass
  the folder.

## Building and packaging

On the target system: Rust (stable), a C linker, and on Linux `pkg-config` and ALSA's headers (`libasound2-dev`;
Fedora `alsa-lib-devel`), which `alsa-sys` (under cpal) needs at build time. Then:

```sh
tools/package.sh          # cargo build --release -p mao-client, then dist/ModernAnarchyOnline/ and a .tar.gz
```

The folder holds `mao-client`, `assets/` and `play.sh` (the Windows package, `tools/package.ps1`, has `mao-client.exe` and
`play.bat` the same way).

### Type-checking from Windows

The targets can't be linked from Windows, but they can be checked:

```sh
rustup target add x86_64-unknown-linux-gnu aarch64-apple-darwin
cargo check --target aarch64-apple-darwin --workspace --all-targets
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

The Linux check stops in `alsa-sys`'s build script, which runs `pkg-config` for ALSA (it has no pure-Rust or
feature-flag way round it; cpal's ALSA host is its only Linux backend without the `jack` feature, and the `pulseaudio`
host also needs C headers). For a check only, point `PKG_CONFIG` at a stand-in that succeeds, with cross use allowed:
a `fake-pkg-config.bat` of

```bat
@echo off
if "%1"=="--modversion" echo 1.2.0
exit /b 0
```

and `PKG_CONFIG=<path to it> PKG_CONFIG_ALLOW_CROSS=1 cargo check --target x86_64-unknown-linux-gnu ...`. Nothing else in
the tree needs a C toolchain for either target at check time.

## Untested

Everything at run time on Linux and macOS: that it links, starts, finds a GPU adapter, draws (the shaders are WGSL
through naga, so Vulkan and Metal should take them, but no frame has been seen), plays sound, reads a real game
folder on a case-sensitive file system, connects and plays. Specifically worth checking first on a real machine:

- the game folder from Proton or a copied install opening: the database, the login screen's GUI files and sounds;
- sound: the cpal device and the 44.1 kHz streams on PipeWire / PulseAudio-through-ALSA and CoreAudio;
- Wayland: window size and scaling (`pixels_per_point`), cursor grab for mouse look;
- macOS: Retina scaling, the Cmd key where the code reads Ctrl, and the quarantine prompt. (Seen on a Mac: the game
  builds, links, logs in and loads the world. Its window, made hidden until its first frame, never showed: wgpu's Metal
  backend gives a window that is not on the screen no frame (`CurrentSurfaceTexture::Occluded`), so the window is now
  shown when that is the answer. `MAO_GPU_REPORT=1` prints what the device holds; `MAO_ALWAYS_REDRAW=1` draws the
  offline views frame after frame.)
- `play.sh`'s search on a real Steam / Proton install.
