#!/usr/bin/env bash
# Builds target/packages/RimStack-<version>-macos.dmg: RimStack.app for
# Apple silicon and Intel in one program (lipo), with Qt's frameworks,
# plugins and QML modules inside, put there by macdeployqt.
#
# Needs Xcode's command line tools, Qt for macOS with its bin folder on
# PATH, both Rust targets (rustup target add aarch64-apple-darwin
# x86_64-apple-darwin) and rsvg-convert for the icon.
#
# MAC_SIGN_IDENTITY ("Developer ID Application: …") signs it with the
# hardened runtime and packaging/macos/entitlements.plist, which lets it
# load the game's Steamworks library, signed by Valve and not by us.
# MAC_NOTARY_PROFILE (a `xcrun notarytool store-credentials` profile)
# then has Apple notarize it; without both, Gatekeeper refuses to open
# a downloaded copy.
set -euo pipefail
cd "$(dirname "$0")/../.."
version=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "rimstack-app"))')
app=target/packaging/RimStack.app
out=target/packages
mkdir -p "$out"

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --profile dist -p rimstack-app --target "$target"
done
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/rimstack" \
    target/aarch64-apple-darwin/dist/rimstack target/x86_64-apple-darwin/dist/rimstack
sed "s/@VERSION@/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"

iconset=target/packaging/rimstack.iconset
rm -rf "$iconset" && mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    rsvg-convert -w $size -h $size crates/app/assets/rimstack.svg -o "$iconset/icon_${size}x${size}.png"
    rsvg-convert -w $((size * 2)) -h $((size * 2)) crates/app/assets/rimstack.svg -o "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns -o "$app/Contents/Resources/rimstack.icns" "$iconset"

if [ -n "${MAC_SIGN_IDENTITY:-}" ]; then
    macdeployqt "$app" -qmldir=crates/app/qml -hardened-runtime -timestamp -codesign="$MAC_SIGN_IDENTITY"
    # The program itself again, with what it is allowed: macdeployqt
    # signs it without entitlements.
    codesign --force --timestamp --options runtime \
        --entitlements packaging/macos/entitlements.plist \
        --sign "$MAC_SIGN_IDENTITY" "$app"
    codesign --verify --strict --verbose=2 "$app"
else
    macdeployqt "$app" -qmldir=crates/app/qml
    # Apple silicon runs nothing unsigned, and macdeployqt changed what
    # the linker signed: signed again, with no identity ("-").
    codesign --force --deep --sign - "$app"
fi

if [ -n "${MAC_SIGN_IDENTITY:-}" ] && [ -n "${MAC_NOTARY_PROFILE:-}" ]; then
    zip="target/packaging/RimStack-notarize.zip"
    ditto -c -k --keepParent "$app" "$zip"
    xcrun notarytool submit "$zip" --keychain-profile "$MAC_NOTARY_PROFILE" --wait
    xcrun stapler staple "$app"
fi

dmg="$out/RimStack-$version-macos.dmg"
hdiutil create -volname RimStack -srcfolder "$app" -ov -format UDZO "$dmg"
echo "$dmg"
