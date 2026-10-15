#!/usr/bin/env bash
# Makes a made-up RimWorld install in DIR for tools/check-app.sh, where
# no real one is (a CI machine) or RIMSTACK_FIXTURE is set:
#   DIR/game      the game: Core, two expansions, 100 local mods
#   DIR/config    its settings folder, with a ModsConfig.xml of 40 mods
#   DIR/workshop  an empty Workshop folder
# The load order is out of order where rules say otherwise, so a sort
# has moves to show; mods have Defs, Patches or Textures, so they fall
# under several categories. Each run gives the same files.
set -euo pipefail
dir="$1"
game="$dir/game"
mkdir -p "$game/Data" "$game/Mods" "$dir/config/Config" "$dir/workshop"
echo "1.6.4871 rev598" > "$game/Version.txt"

about() { # folder packageId name [extra xml]
    mkdir -p "$1/About"
    printf '<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n  <packageId>%s</packageId>\n  <name>%s</name>\n  <author>Fixture</author>\n  <supportedVersions><li>%s</li></supportedVersions>\n%s</ModMetaData>\n' \
        "$2" "$3" "${5:-1.6}" "${4:-}" > "$1/About/About.xml"
}

about "$game/Data/Core" ludeon.rimworld Core
about "$game/Data/Royalty" ludeon.rimworld.royalty Royalty
about "$game/Data/Ideology" ludeon.rimworld.ideology Ideology
mkdir -p "$game/Data/Core/Defs"

kinds=(Defs Patches Textures)
# One white pixel.
png="iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
for n in $(seq -w 1 100); do
    folder="$game/Mods/fixture-$n"
    extra=""
    # Every seventh mod loads after the one before it, by its own rule.
    if [ $((10#$n % 7)) = 0 ]; then
        extra="  <loadAfter><li>fixture.mod$(printf %03d $((10#$n - 1)))</li></loadAfter>\n"
    fi
    # Every eleventh lists only an older game version.
    version="1.6"
    [ $((10#$n % 11)) = 0 ] && version="1.4"
    # The third needs a mod that is not installed: Downloads offers it.
    if [ "$n" = 003 ]; then
        extra="$extra  <modDependencies><li><packageId>fixture.missing</packageId><displayName>Fixture Library</displayName><steamWorkshopUrl>steam://url/CommunityFilePage/123456789</steamWorkshopUrl></li></modDependencies>\n"
    fi
    about "$folder" "fixture.mod$n" "Fixture Mod $n" "$(printf "$extra")" "$version"
    kind=${kinds[$((10#$n % 3))]}
    mkdir -p "$folder/$kind"
    case $kind in
        Defs) printf '<Defs><ThingDef><defName>Fixture%s</defName></ThingDef></Defs>\n' "$n" > "$folder/Defs/Things.xml" ;;
        Patches) printf '<Patch><Operation Class="PatchOperationAdd"><xpath>Defs</xpath><value/></Operation></Patch>\n' > "$folder/Patches/Patch.xml" ;;
        Textures) base64 -d <<< "$png" > "$folder/Textures/Fixture$n.png" ;;
    esac
done

{
    printf '<?xml version="1.0" encoding="utf-8"?>\n<ModsConfigData>\n  <version>1.6.4871 rev598</version>\n  <activeMods>\n'
    printf '    <li>ludeon.rimworld</li>\n    <li>ludeon.rimworld.royalty</li>\n    <li>ludeon.rimworld.ideology</li>\n'
    # Forty mods, the ones with a loadAfter rule ahead of the mod they follow.
    for n in $(seq 40 -1 1); do printf '    <li>fixture.mod%03d</li>\n' "$n"; done
    printf '  </activeMods>\n  <knownExpansions>\n    <li>ludeon.rimworld.royalty</li>\n    <li>ludeon.rimworld.ideology</li>\n  </knownExpansions>\n</ModsConfigData>\n'
} > "$dir/config/Config/ModsConfig.xml"

# Two saves: one with the load order as it is, one with a mod that is
# not installed.
save() { # file mod...
    mkdir -p "$dir/config/Saves"
    {
        printf '\xef\xbb\xbf<?xml version="1.0" encoding="utf-8"?>\n<savegame>\n\t<meta>\n\t\t<gameVersion>1.6.4871 rev598</gameVersion>\n\t\t<modIds>\n'
        for id in "${@:2}"; do printf '\t\t\t<li>%s</li>\n' "$id"; done
        printf '\t\t</modIds>\n\t\t<modNames>\n'
        for id in "${@:2}"; do printf '\t\t\t<li>%s</li>\n' "$id"; done
        printf '\t\t</modNames>\n\t</meta>\n\t<game></game>\n</savegame>\n'
    } > "$dir/config/Saves/$1.rws"
}
active=(ludeon.rimworld ludeon.rimworld.royalty ludeon.rimworld.ideology)
for n in $(seq 40 -1 1); do active+=("$(printf 'fixture.mod%03d' "$n")"); done
save "Colony One" "${active[@]}"
save "Colony Two" "${active[@]}" fixture.gone
touch -d '2026-01-02 10:00' "$dir/config/Saves/Colony Two.rws"
touch -d '2026-01-01 10:00' "$dir/config/Saves/Colony One.rws"

# The game's log of its last start, with one error from a mod.
cat > "$dir/config/Player.log" <<'LOG'
Mono path[0] = '/game/RimWorldLinux_Data/Managed'
RimWorld 1.6.4871 rev598
Error in static constructor of FixtureMod005.Init: System.NullReferenceException: Object reference not set to an instance of an object
[Ref 0A1B2C3D]
  at FixtureMod005.Init..cctor () [0x00001] in <abc>:0
Could not resolve cross-reference to Verse.ThingDef named Fixture999 (wanter=thingDef)
LOG
