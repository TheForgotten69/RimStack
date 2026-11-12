# RimStack: what is run again and again while working on it.
# `just` alone lists the recipes.

app := "target/release/rimstack"

default:
    @just --list --unsorted

# Build the release app
build:
    cargo build --release -q -p rimstack-app

# Build and open RimStack on the installed mods
run *args: build
    {{app}} {{args}}

# Open on 2,300 made-up mods instead of the installed ones
demo *args: build
    {{app}} --demo {{args}}

# Format the Rust code
fmt:
    cargo fmt

# Clippy on every target
lint:
    cargo clippy --release -q --all-targets

# Both crates' tests, on the release build's profile; `just test list::` runs those matching a name
test *filter:
    cargo test --release --workspace -q {{filter}}

# The package for this system, into target/packages: an AppImage, a zip and installer, or a dmg
package:
    {{ if os() == "windows" { "pwsh packaging/windows/package.ps1" } else { "packaging/" + os() + "/package.sh" } }}

# Known advisories against the crates in Cargo.lock
audit:
    cargo audit

# After a text was added or changed: wrap it and write words.json and English.json again
words:
    python3 tools/words.py

# Texts each language file lacks, or holds and no longer needs
languages:
    #!/usr/bin/env python3
    import json, glob, os
    words = set(json.load(open("crates/app/assets/words.json", encoding="utf-8")))
    bad = 0
    for path in sorted(glob.glob("crates/app/assets/languages/*.json")):
        own = set(json.load(open(path, encoding="utf-8")))
        lacks, extra = sorted(words - own), sorted(own - words)
        bad += len(lacks) + len(extra)
        print(os.path.basename(path), "ok" if not lacks and not extra else "")
        for text in lacks: print("  lacks:", text)
        for text in extra: print("  no longer used:", text)
    raise SystemExit(1 if bad else 0)

# The window's picture at 2x, with no window and its own settings folder: `just shot /tmp/mods.png --demo --theme=light --text-scale=1.6`
shot out *args:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release -q -p rimstack-app --example shot
    scratch="$(mktemp -d -p target)"
    trap 'rm -rf "$scratch"' EXIT
    XDG_CONFIG_HOME="$scratch/settings" XDG_DATA_HOME="$scratch/data" target/release/examples/shot {{out}} {{args}}

# The Qt build's picture of the Mods tab, to set beside it: `just qt-shot /tmp/qt.png rim 1 --several`
qt-shot out theme="rim" scale="1" *args:
    target/qt-reference/shot.sh {{out}} {{theme}} {{scale}} {{args}}

# The scripted ten-second scroll on made-up mods, in a real window: frames, slow frames, the worst
scroll: build
    {{app}} --selftest 2>&1 | grep -E "scroll frames|_ms="

# A release: the version in Cargo.toml and Cargo.lock, `just ready`, the commit and its tag; pushing them stays by hand
release version:
    #!/usr/bin/env bash
    set -euo pipefail
    v="{{version}}"
    [[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] || { echo "not a version: $v"; exit 1; }
    [ "$(git branch --show-current)" = main ] || { echo "not on main"; exit 1; }
    [ -z "$(git status --porcelain)" ] || { echo "the working tree has changes"; exit 1; }
    ! git rev-parse -q --verify "refs/tags/v$v" > /dev/null || { echo "v$v exists already"; exit 1; }
    perl -pi -e 's/^version = ".*"/version = "'"$v"'"/ if !$done && /^version = / && ++$done' Cargo.toml
    cargo update -q --workspace
    just ready
    git commit -q -am "RimStack $v"
    git tag "v$v"
    packaging/changes.sh HEAD "v$v"
    echo
    echo "Tagged v$v. To publish: git push origin main v$v"

# Everything before a commit: format, lint, tests
ready: fmt lint test audit
