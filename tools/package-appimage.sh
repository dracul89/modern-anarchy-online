#!/bin/sh
# Builds the release client on Linux and puts it together as an AppImage: one file that runs as it is on most
# distributions (chmod +x, then start it), dist/ModernAnarchyOnline-<version>-linux-<processor>.AppImage. Started alone
# it is the launcher; arguments go to the client ("<game folder>" play, and the rest).
#
#   AppDir/
#     AppRun                           starts usr/bin/mao-client
#     modern-anarchy-online.desktop    the menu entry (name, icon, category)
#     modern-anarchy-online.png        the icon (assets/icon/mao_256.png)
#     usr/bin/mao-client, usr/bin/assets/     the client and the bundled skins beside it, where it looks for them
#     usr/share/...                    the same entry and icon where desktop tools look, README.txt, LICENSE
#
#   tools/package-appimage.sh [out-dir] [version]          (default: dist, and the version in Cargo.toml)
#
# The system's own libraries are used, not copies: glibc (as new as the build machine's or newer), ALSA (libasound2),
# a Vulkan driver, X11 or Wayland; see docs/PLATFORMS.md. The file is put together by appimagetool: $APPIMAGETOOL when
# set, else the one on the PATH, else the release named below is downloaded into the build's own folder.
# appimagetool wants desktop-file-validate (the desktop-file-utils package) to check the menu entry.
# The folder and .tar.gz for a terminal are tools/package.sh's.
set -eu
out=${1:-dist}
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
version=${2:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}
arch=$(uname -m)
tool_version=1.9.0
cargo build --release --locked -p mao-client

dir="$root/$out/AppDir"
rm -rf "$dir"
mkdir -p "$dir/usr/bin" "$dir/usr/share/applications" "$dir/usr/share/icons/hicolor/256x256/apps" "$dir/usr/share/doc/modern-anarchy-online"
cp target/release/mao-client "$dir/usr/bin/"
chmod +x "$dir/usr/bin/mao-client"
cp -R assets "$dir/usr/bin/"
cp tools/package-readme.txt "$dir/usr/share/doc/modern-anarchy-online/README.txt"
for f in LICENSE*; do
  if [ -f "$f" ]; then cp "$f" "$dir/usr/share/doc/modern-anarchy-online/"; fi
done
cp assets/icon/mao_256.png "$dir/modern-anarchy-online.png"
cp assets/icon/mao_256.png "$dir/usr/share/icons/hicolor/256x256/apps/modern-anarchy-online.png"
cat > "$dir/modern-anarchy-online.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Modern Anarchy Online
Comment=A client for Anarchy Online (needs an installed copy of the game)
Exec=mao-client
Icon=modern-anarchy-online
Categories=Game;RolePlaying;
Terminal=false
X-AppImage-Version=$version
DESKTOP
cp "$dir/modern-anarchy-online.desktop" "$dir/usr/share/applications/"
cat > "$dir/AppRun" <<'RUN'
#!/bin/sh
here=$(dirname "$(readlink -f "$0")")
exec "$here/usr/bin/mao-client" "$@"
RUN
chmod +x "$dir/AppRun"

tool=${APPIMAGETOOL:-$(command -v appimagetool || true)}
if [ -z "$tool" ]; then
  tool="$root/target/appimagetool-$tool_version-$arch.AppImage"
  if [ ! -x "$tool" ]; then
    curl -fsSL -o "$tool" "https://github.com/AppImage/appimagetool/releases/download/$tool_version/appimagetool-$arch.AppImage"
    chmod +x "$tool"
  fi
fi
image="$root/$out/ModernAnarchyOnline-$version-linux-$arch.AppImage"
rm -f "$image"
# (--appimage-extract-and-run: the tool is an AppImage itself, and a build machine may have no FUSE to mount it with)
ARCH=$arch "$tool" --appimage-extract-and-run --no-appstream "$dir" "$image"
chmod +x "$image"
rm -rf "$dir"
echo "packaged: $image ($version)"
