#!/usr/bin/env bash
# Builds RimStack-<version>-x86_64.AppImage in target/packages: the
# program built with the `dist` profile (or RIMSTACK_PROFILE: the beta
# workflow packs the `release` build its checks made), Qt's libraries and plugins
# beside it, put together by linuxdeploy and its Qt plugin.
#
# The two tools are downloaded once into target/packaging/tools, pinned
# to a release. QMAKE names Qt's qmake where `qmake6` is not it.
#
# The AppImage runs on systems whose C library is as new as the one it
# was built on, or newer: build it on the oldest Linux it is for.
set -euo pipefail
cd "$(dirname "$0")/../.."
root=$PWD
version=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "rimstack-app"))')
tools=target/packaging/tools
app=target/packaging/AppDir
out=target/packages
mkdir -p "$tools" "$out"

fetch() { # name url
    [ -x "$tools/$1" ] && return
    curl --fail --location --silent --show-error --output "$tools/$1" "$2"
    chmod +x "$tools/$1"
}
fetch linuxdeploy https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage
fetch linuxdeploy-plugin-qt https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/1-alpha-20250213-1/linuxdeploy-plugin-qt-x86_64.AppImage

profile="${RIMSTACK_PROFILE:-dist}"
cargo build --profile "$profile" -p rimstack-app
rm -rf "$app"
mkdir -p "$app/usr/share/icons/hicolor/scalable/apps"
cp crates/app/assets/rimstack.svg "$app/usr/share/icons/hicolor/scalable/apps/"
# AppRun starts the program through a link at the AppDir's top, by its
# whole path; Qt then takes that folder for the program's own and looks
# for its plugins from there.
printf '[Paths]\nPrefix = usr\nPlugins = plugins\nQml2Imports = qml\n' > "$app/qt.conf"

# The plugin finds the QML the program imports by reading these sources.
export QML_SOURCES_PATHS="$root/crates/app/qml"
qmake="${QMAKE:-$(command -v qmake6 || command -v qmake)}"
# Qt's own plugins only (`libq…`, and Wayland's shells): a desktop's
# installed among them, KDE's image formats or its theme, would be
# shipped with dependencies of their own. The plugin reads the folder
# from qmake, so a qmake that names the filtered one stands in for it.
plugins="$root/target/packaging/qt-plugins"
rm -rf "$plugins"
for file in "$("$qmake" -query QT_INSTALL_PLUGINS)"/*/*.so; do
    kind=$(basename "$(dirname "$file")")
    case "$kind/$(basename "$file")" in
        */libq* | wayland-*/*) mkdir -p "$plugins/$kind" && ln -s "$file" "$plugins/$kind/" ;;
    esac
done
# linuxdeploy-plugin-qt asks `qmake -query` and reads its KEY:value lines.
printf '#!/bin/sh\n"%s" "$@" | sed "s|^QT_INSTALL_PLUGINS:.*|QT_INSTALL_PLUGINS:%s|"\n' "$qmake" "$plugins" > target/packaging/qmake
chmod +x target/packaging/qmake
export QMAKE="$root/target/packaging/qmake"
export EXTRA_QT_MODULES="svg"
# Wayland beside X11; offscreen so the package itself can be checked
# with no display: `QT_QPA_PLATFORM=offscreen <AppImage> --check=screens`
# on a made-up install (tools/check-app.sh shows how).
export EXTRA_PLATFORM_PLUGINS="libqwayland.so;libqoffscreen.so"
export LDAI_OUTPUT="$out/RimStack-$version-x86_64.AppImage"
# The tools are AppImages themselves; this runs them where FUSE is not.
export APPIMAGE_EXTRACT_AND_RUN=1
# Their own strip predates the libraries of a current system; the
# program is stripped by its profile already.
export NO_STRIP=1
PATH="$PWD/$tools:$PATH" linuxdeploy \
    --appdir "$app" \
    --executable "target/$profile/rimstack" \
    --desktop-file crates/app/assets/rimstack.desktop \
    --icon-file crates/app/assets/rimstack.svg \
    --plugin qt \
    --output appimage
echo "$LDAI_OUTPUT"
