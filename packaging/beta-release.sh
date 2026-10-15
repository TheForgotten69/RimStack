#!/usr/bin/env bash
# Puts the packages in DIR on the `beta` prerelease, moved to COMMIT:
#   packaging/beta-release.sh DIR COMMIT
# Run by .github/workflows/beta.yml after each push to main; needs `gh`
# with a token that may write the repository's contents.
#
# Each system's files are replaced only by new ones: where its build
# failed this time, its last good one stays, and the notes say from which
# commit each system's files are. GitHub's "latest release" leaves a
# prerelease out, so RimStack's own update check never offers a beta.
set -euo pipefail
dir="$1"
commit="$2"
tag=beta
short=${commit:0:7}

shopt -s nullglob
built=("$dir"/*)
fresh=$(mktemp)
trap 'rm -f "$fresh"' EXIT
[ ${#built[@]} -gt 0 ] || { echo "no package was built" >&2; exit 1; }

# The files of each system, by how the packaging scripts name them.
system_of() {
    case "$1" in
        *.AppImage) echo linux ;;
        *-windows-*.zip | *-setup.exe) echo windows ;;
        *-macos.dmg) echo macos ;;
        *) echo other ;;
    esac
}

git tag -f "$tag" "$commit"
git push -f origin "refs/tags/$tag"

if ! gh release view "$tag" > /dev/null 2>&1; then
    gh release create "$tag" --prerelease --title "RimStack beta" --notes "Being built."
fi

# Replaced, system by system, with what this run built.
mapfile -t kept < <(gh release view "$tag" --json assets --jq '.assets[].name')
for system in linux windows macos; do
    new=()
    for file in "${built[@]}"; do
        if [ "$(system_of "$file")" = "$system" ]; then new+=("$file"); fi
    done
    [ ${#new[@]} -gt 0 ] || continue
    for name in "${kept[@]}"; do
        if [ "$(system_of "$name")" = "$system" ]; then gh release delete-asset "$tag" "$name" --yes; fi
    done
    gh release upload "$tag" "${new[@]}" --clobber
    echo "$system $short" >> "$fresh"
done

# What each system's files are built from: this commit, or, where its
# build failed, the commit its files came from, kept in the notes.
before=$(gh release view "$tag" --json body --jq .body || true)
line_for() {
    local system=$1 from
    from=$(grep -E "^$system " "$fresh" | cut -d' ' -f2 || true)
    if [ -n "$from" ]; then
        echo "- $system: built from $from"
    else
        from=$(grep -oE "^- $system: built from [0-9a-f]+" <<< "$before" | grep -oE '[0-9a-f]+$' || true)
        echo "- $system: ${from:+built from $from, }not rebuilt from $short (its build failed)"
    fi
}
notes="Built automatically from \`main\` at $short on $(date -u '+%Y-%m-%d %H:%M UTC'). A beta: it may break.

$(line_for linux)
$(line_for windows)
$(line_for macos)

Until the repository holds signing keys, the programs are not signed:
Windows: SmartScreen warns about an unsigned program; choose More info, then Run anyway.
macOS: the app is not notarized; right-click it and choose Open the first time.
Linux: make the AppImage executable (\`chmod +x\`) and run it."
gh release edit "$tag" --prerelease --title "RimStack beta ($short)" --notes "$notes"
