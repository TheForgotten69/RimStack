# RimStack

A fast, open-source mod manager for RimWorld. Rust core, Qt Quick interface.

Status: early, usable. Every section has a working version laid out as in the mockup:

- **Mods.** The load order and the inactive mods; problems, texture conflicts with a comparison to choose each texture; Sort as a preview (the mods' own rules, community rules, rules of your own, and rules detected from compiled code); Undo, Redo, saved lists, tags and notes, filter terms, keyboard, Ctrl+K; Save to the game's `ModsConfig.xml` (the file as it was is kept beside it).
- **Downloads.** Which Workshop mods are behind their Workshop version; looking up pasted links; suggested replacements. Steam does the downloading. SteamCMD, when you ask for it, is fetched from Valve's own server over HTTPS; Valve publishes nothing to check it against.
- **Textures.** PNG to DDS conversion built in, with a record of every file made so only those are ever removed.
- **Saves.** Each save's mods compared with the load order; RimSaves folders, previews and colony facts when that mod is used.
- **Logs.** Errors in the game's last two logs, grouped, with the mods in each stack trace; a record of launches.
- **Settings.** Every folder and file RimStack uses.

## Build and run

Needs Rust 1.85 or later, Qt 6 (base and declarative) and a C++ compiler.

```bash
cargo test --workspace
cargo build -p rimstack-app --release
./target/release/rimstack              # your installed mods
./target/release/rimstack --demo       # 2,300 made-up mods instead
./target/release/rimstack --selftest   # drives the demo, measures, and quits
./target/release/rimstack --section=3 --shot=/tmp/saves.png   # open on a section, save a picture, quit
tools/check-app.sh                     # every action of the window, no window shown; saves only to a copy
```

`tools/check-app.sh` drives the window from QML on a copy of your game's settings, or on a made-up install where there is none (`RIMSTACK_FIXTURE=1` asks for it): the Mods screen action by action, the mouse, every other section on a made-up install (`--check=screens`), what changes on disk while the window is open, and broken setups. It fails on any QML error. `RIMSTACK_LANGUAGE=Français` runs it in that language.

## Packages and systems

Every push to `main` builds all three on GitHub (`.github/workflows/beta.yml`) and puts them on the [`beta` prerelease](https://github.com/TheForgotten69/RimStack/releases/tag/beta), replacing the last ones. A system whose checks or build fail keeps its previous build there, and the release notes say from which commit each system's files are.

`just package` builds the package for the system it runs on into `target/packages`, from the `dist` profile:

| System | Package | Script | State |
|---|---|---|---|
| Linux | AppImage | `packaging/linux/package.sh` | Built and checked on a made-up install. Build it on the oldest Linux it is for: it needs that system's C library or a newer one. |
| Windows | zip, and an installer with Inno Setup | `packaging/windows/package.ps1` (PowerShell 7, `pwsh`) | Not yet run. MSVC only. Unsigned, SmartScreen warns. |
| macOS | dmg of a universal `RimStack.app` | `packaging/macos/package.sh` | Not yet run. Signing and notarization when `MAC_SIGN_IDENTITY` and `MAC_NOTARY_PROFILE` are set. |

The core library and its tests compile for Windows; nothing has run there or on macOS yet. On macOS RimStack does not run SteamCMD, until it is seen to keep out of Steam's own folder there.

The game is found through Steam's library list. If it isn't, set `RIMSTACK_GAME` (the game folder), `RIMSTACK_CONFIG` (the folder holding `Config/` and `Saves/`) or `RIMSTACK_WORKSHOP`. Timings print to the terminal:

```
scan_ms         reading every mod's About.xml
mods_shown_ms   from process start to the lists being filled
startup_ms      from process start to the first frame
```

The same lines, every problem that was worked around and any crash with its trace are kept in `diagnostics.log` in RimStack's data folder (on Linux `~/.local/share/RimStack/`; Settings, RimStack shows where). Past one megabyte it is set aside as `diagnostics.log.old`.

## Languages

RimStack comes in English, Deutsch, Español, Français, Italiano, Polski and Português; Settings, Appearance, Language chooses one, and the change shows at once. Five of them were translated by machine and read through only in part, so corrections are welcome.

To correct a text for yourself, without building anything:

1. In Settings, Appearance, Language, choose the language and press **Edit translations**. This opens `<settings folder>/RimStack/languages/<Language>.json` (on Linux `~/.config/RimStack/languages/`), written with every English text and an empty place beside it.
2. Put your wording beside the English text and save. It shows at once and stands over what RimStack ships; a place left empty keeps the shipped wording.

To add a language of your own, put a file `<Its name>.json` in that folder: it appears in the Language menu. To remove one you added, delete its file.

English is a language like the others: `English.json` holds each text beside itself, and a wording changed there, or in your own `languages/English.json`, replaces it.

To correct a language for everyone, edit `crates/app/assets/languages/<Language>.json` in this repository and build. Each line is `"English text": "translation"`:

- Never change the English text on the left: it is how the text is found.
- `{}` stands for a number or a name. Keep as many as the English has, in the same order.
- Keep a leading or trailing space, and a final `…`.
- A counted text comes as a pair, `"{} mod"` and `"{} mods"`. Which form a count takes comes from the plural rules of the file's `"@locale"` (`"fr"`, `"pl"`): the first form is for "one", and the second holds a form for each of the language's other categories, with a bar between. Polish writes `"{} mody|{} modów"`.
- After changing a text in the code, `tools/words.py` writes `crates/app/assets/words.json` again, the list of every English text; a language file lacking one of them shows that text in English.

## Licence

GPL-3.0-or-later. See [`LICENSE`](LICENSE).

Additional permission under GNU GPL version 3 section 7: if you modify this
program, or any covered work, by linking or combining it with the Steamworks
SDK (or a modified version of that library), containing parts covered by the
terms of the Steamworks SDK licence, the licensors of this program grant you
additional permission to convey the resulting work.
