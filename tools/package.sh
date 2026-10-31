#!/bin/sh
# Builds the release client on Linux or macOS and puts together a folder (and a .tar.gz) to play from anywhere:
# dist/ModernAnarchyOnline. The Windows build's is tools/package.ps1; see docs/PLATFORMS.md.
#
# The folder holds mao-client, the bundled skins (assets/, which the client looks for beside itself) and play.sh, which
# finds the game (the folder with cd_image: a path given to it, else $MAO_GAME, else the usual Steam, Proton and Wine
# places) and starts the launcher. The client keeps its settings, hotbars, crash.log and the like in
# $XDG_CONFIG_HOME/ModernAnarchyOnline (~/.config/ModernAnarchyOnline), or mao.ini's folder when one sits where it runs.
# Nothing from the game is copied: the player needs an Anarchy Online install.
#
#   tools/package.sh [out-dir]          (default: dist)
#
# Building needs Rust, a C linker and, on Linux, ALSA's development files and pkg-config (libasound2-dev pkg-config on
# Debian and Ubuntu, alsa-lib-devel pkgconf on Fedora).
set -eu
out=${1:-dist}
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
cargo build --release -p mao-client
dir="$root/$out/ModernAnarchyOnline"
rm -rf "$dir"
mkdir -p "$dir"
cp target/release/mao-client "$dir/"
cp -R assets "$dir/"
cat > "$dir/play.sh" <<'PLAY'
#!/bin/sh
# Starts Modern Anarchy Online: the launcher (log in, pick a character), then the game.
# The game folder (the one with cd_image) is the first argument, else $MAO_GAME, else the first of the usual places
# that has it: Steam's library (native, Flatpak or macOS), the Wine and CrossOver prefixes' Funcom and Steam folders.
here=$(cd "$(dirname "$0")" && pwd)
game=${1:-${MAO_GAME:-}}
if [ -z "$game" ]; then
  for g in \
    "$HOME/.steam/steam/steamapps/common/Anarchy Online" \
    "$HOME/.local/share/Steam/steamapps/common/Anarchy Online" \
    "$HOME/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/Anarchy Online" \
    "$HOME/Library/Application Support/Steam/steamapps/common/Anarchy Online" \
    "$HOME/.wine/drive_c/Funcom/Anarchy Online" \
    "$HOME/.wine/drive_c/Program Files (x86)/Funcom/Anarchy Online" \
    "$HOME/.wine/drive_c/Program Files (x86)/Steam/steamapps/common/Anarchy Online" \
    "$HOME/Games/Anarchy Online" \
    "$HOME/Library/Application Support/CrossOver/Bottles/"*"/drive_c/Funcom/Anarchy Online" \
    "$HOME/Library/Application Support/CrossOver/Bottles/"*"/drive_c/Program Files (x86)/Steam/steamapps/common/Anarchy Online"
  do
    if [ -d "$g/cd_image" ]; then game=$g; break; fi
  done
fi
if [ -z "$game" ] || [ ! -d "$game/cd_image" ]; then
  echo "Anarchy Online was not found. Start this with its folder (the one with cd_image), for example:"
  echo "  ./play.sh \"\$HOME/Games/Anarchy Online\""
  exit 1
fi
"$here/mao-client" "$game" play || {
  echo "The client stopped with an error; see crash.log in ${XDG_CONFIG_HOME:-$HOME/.config}/ModernAnarchyOnline."
  exit 1
}
PLAY
chmod +x "$dir/play.sh" "$dir/mao-client"
system=$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)
archive="$root/$out/ModernAnarchyOnline-$system.tar.gz"
rm -f "$archive"
tar -C "$root/$out" -czf "$archive" ModernAnarchyOnline
echo "packaged: $dir ($archive)"
