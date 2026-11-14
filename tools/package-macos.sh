#!/bin/sh
# Builds the release client on macOS and puts it together as an application: dist/Modern Anarchy Online.app, which
# starts the launcher when opened (the client with no arguments), with the icon in the Dock and the Finder; and as a
# disk image to hand out, dist/ModernAnarchyOnline-<version>-macos-<processor>.dmg, which opens as a window with the
# application and a shortcut to the Applications folder to drag it onto.
#
#   Modern Anarchy Online.app/Contents/
#     Info.plist                 the name, the version (the workspace's, from Cargo.toml), the icon, macOS 11 or newer
#     MacOS/mao-client           the client
#     Resources/mao.icns         the icon, made here from assets/icon/mao_1024.png
#     Resources/assets/          the bundled skins (the client looks there: theme.rs, bundled_gui_dir)
#     Resources/README.txt, LICENSE
#
#   tools/package-macos.sh [out-dir] [version]          (default: dist, and the version in Cargo.toml)
#
# The application is signed ad hoc (no certificate), which lets it run where it was built; one downloaded from
# elsewhere is still held back by the system until allowed (right click, Open; or System Settings, Privacy & Security).
# It is for the machine's own processor (Apple silicon on the build machines). The folder and .tar.gz for a terminal
# are tools/package.sh's; see docs/PLATFORMS.md.
set -eu
out=${1:-dist}
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
version=${2:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}
cargo build --release --locked -p mao-client
app="$root/$out/Modern Anarchy Online.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/mao-client "$app/Contents/MacOS/"
chmod +x "$app/Contents/MacOS/mao-client"
cp -R assets "$app/Contents/Resources/"
cp tools/package-readme.txt "$app/Contents/Resources/README.txt"
for f in LICENSE*; do
  if [ -f "$f" ]; then cp "$f" "$app/Contents/Resources/"; fi
done

# The icon: the sizes the system asks for, each scaled from the 1024 picture.
set_dir=$(mktemp -d)/mao.iconset
mkdir -p "$set_dir"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon/mao_1024.png --out "$set_dir/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) assets/icon/mao_1024.png --out "$set_dir/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$set_dir" -o "$app/Contents/Resources/mao.icns"
rm -rf "$(dirname "$set_dir")"

# (CFBundleShortVersionString takes numbers and dots only: 0.2.0-rc.1 is 0.2.0 there, and whole in the long text)
short=${version%%-*}
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Modern Anarchy Online</string>
  <key>CFBundleDisplayName</key><string>Modern Anarchy Online</string>
  <key>CFBundleIdentifier</key><string>io.github.dracul89.modern-anarchy-online</string>
  <key>CFBundleExecutable</key><string>mao-client</string>
  <key>CFBundleIconFile</key><string>mao</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleShortVersionString</key><string>$short</string>
  <key>CFBundleVersion</key><string>$short</string>
  <key>CFBundleGetInfoString</key><string>Modern Anarchy Online $version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.role-playing-games</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
PLIST
plutil -lint "$app/Contents/Info.plist" >/dev/null
codesign --force --deep --sign - "$app"

# The disk image: the application and a link to /Applications, compressed and read-only.
stage=$(mktemp -d)
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
image="$root/$out/ModernAnarchyOnline-$version-macos-$(uname -m).dmg"
rm -f "$image"
hdiutil create -volname "Modern Anarchy Online" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$image" >/dev/null
rm -rf "$stage"
echo "packaged: $app ($version)"
echo "disk image: $image"
