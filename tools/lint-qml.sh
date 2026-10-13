#!/bin/sh
# Lints the QML with Qt's qmllint against the module the last build made.
# Build first: `cargo build -p rimstack-app --release`.
set -eu
cd "$(dirname "$0")/.."
lint=${QMLLINT:-/usr/lib/qt6/bin/qmllint}
built=$(ls -dt target/release/build/rimstack-app-*/out/qt-build-utils/qml_modules/RimStack/Ui 2>/dev/null | head -1)
[ -n "$built" ] || { echo "no built QML module: build the app first" >&2; exit 2; }
# The built module lists the files by path but does not hold them; give
# qmllint a copy of the listing beside the sources.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/RimStack/Ui"
cp "$built/qmldir" "$built"/*.qmltypes "$work/RimStack/Ui/" 2>/dev/null || true
ln -s "$PWD/crates/app/qml" "$work/RimStack/Ui/qml"
find crates/app/qml -name '*.qml' -print0 | xargs -0 "$lint" -I "$work" "$@"
