#!/bin/sh
# Builds the release client on macOS and puts it together as an application: dist/Modern Anarchy Online.app, which
# starts the launcher when opened (the client with no arguments), with the icon in the Dock and the Finder; and as a
# disk image to hand out, dist/ModernAnarchyOnline-<version>-macos-<processor>.dmg, which opens as a window with the
# application and a shortcut to the Applications folder to drag it onto.
#
#   Modern Anarchy Online.app/Contents/
#     Info.plist                 the name, the version (the workspace's, from Cargo.toml), the icon, macOS 11 or newer
#                                (and plain HTTP allowed: the launcher's news page and the game's sites are not HTTPS)
#     MacOS/mao-client           the client
#     Resources/mao.icns         the icon, made here from assets/icon/mao_1024.png
#     Resources/assets/          the bundled skins (the client looks there: theme.rs, bundled_gui_dir)
#     Resources/README.txt, LICENSE
#
#   tools/package-macos.sh [out-dir] [version]          (default: dist, and the version in Cargo.toml)
#
# Signing. With a "Developer ID Application" certificate in the keychain (an Apple Developer account makes one: Xcode,
# Settings, Accounts, Manage Certificates) the application and the disk image are signed with it, hardened runtime and
# a timestamp; MAO_SIGN_IDENTITY names another certificate, and MAO_SIGN_IDENTITY=- asks for none. With
# MAO_NOTARY_PROFILE as well, the disk image is sent to Apple's notary service and its ticket attached, after which a
# downloaded copy opens without the system's warning. The profile is a name the credentials were stored under, once:
#
#   xcrun notarytool store-credentials mao-notary --apple-id <the account's address> --team-id <the team's ten characters>
#   MAO_NOTARY_PROFILE=mao-notary tools/package-macos.sh
#
# (it asks for an app-specific password, made at account.apple.com, Sign-In and Security, and keeps it in the keychain).
# Without a certificate the application is signed ad hoc, which lets it run where it was built; one downloaded from
# elsewhere is then held back by the system until allowed (right click, Open; or System Settings, Privacy & Security).
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
  <key>NSAppTransportSecurity</key><dict><key>NSAllowsArbitraryLoads</key><true/></dict>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
PLIST
plutil -lint "$app/Contents/Info.plist" >/dev/null

# The certificate to sign with: the one named, else the keychain's first Developer ID Application, else none (ad hoc).
identity=${MAO_SIGN_IDENTITY:-$(security find-identity -v -p codesigning 2>/dev/null | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1)}
identity=${identity:--}
if [ "$identity" = - ]; then
  codesign --force --deep --sign - "$app"
else
  codesign --force --options runtime --timestamp --sign "$identity" "$app"
fi
codesign --verify --strict "$app"

# The disk image: the application and a link to /Applications, compressed and read-only.
stage=$(mktemp -d)
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
image="$root/$out/ModernAnarchyOnline-$version-macos-$(uname -m).dmg"
rm -f "$image"
hdiutil create -volname "Modern Anarchy Online" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$image" >/dev/null
rm -rf "$stage"
signed="ad hoc (no certificate)"
if [ "$identity" != - ]; then
  codesign --force --timestamp --sign "$identity" "$image"
  signed="with a Developer ID certificate"
  if [ -n "${MAO_NOTARY_PROFILE:-}" ]; then
    # (Apple's check of the image and what is in it: some minutes; then the ticket is attached to the image)
    xcrun notarytool submit "$image" --keychain-profile "$MAO_NOTARY_PROFILE" --wait
    xcrun stapler staple "$image"
    signed="$signed, notarized"
  else
    echo "not notarized: MAO_NOTARY_PROFILE is not set (see the top of this script)"
  fi
fi
echo "packaged: $app ($version)"
echo "disk image: $image"
echo "signed: $signed"
