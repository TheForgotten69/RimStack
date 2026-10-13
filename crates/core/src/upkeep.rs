//! Things done to the files of mods and of the game on request: a
//! Workshop mod copied to the game's own `Mods` folder so that Steam no
//! longer updates it, the settings mods keep, set aside, and the game
//! started without Steam.

use crate::say;

use std::path::{Path, PathBuf};

/// Copies a mod's folder into `local_mods` under its own folder name.
/// Nothing is replaced: a folder of that name stops it.
pub fn copy_to_local(mod_root: &Path, local_mods: &Path) -> Result<PathBuf, String> {
    let name = mod_root
        .file_name()
        .ok_or_else(|| say!("the mod's folder has no name"))?;
    let to = local_mods.join(name);
    if to.exists() {
        return Err(say!("{} is there already", to.display()));
    }
    crate::redownload::copy_all(mod_root, &to).map_err(|e| {
        let _ = std::fs::remove_dir_all(&to);
        format!("{}: {e}", to.display())
    })?;
    Ok(to)
}

/// Moves every mod's settings file (`Mod_*.xml` in the game's `Config`
/// folder) into `aside`. Returns how many; they can be moved back by hand.
pub fn set_mod_settings_aside(config: &Path, aside: &Path) -> Result<usize, String> {
    let failed = |e: std::io::Error| format!("{}: {e}", aside.display());
    let files = std::fs::read_dir(config.join("Config"))
        .map_err(|e| format!("{}: {e}", config.display()))?;
    let settings: Vec<PathBuf> = files
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("Mod_") && name.ends_with(".xml")
        })
        .collect();
    if settings.is_empty() {
        return Ok(0);
    }
    std::fs::create_dir_all(aside).map_err(failed)?;
    for file in &settings {
        let to = aside.join(file.file_name().unwrap_or_default());
        std::fs::rename(file, &to)
            .or_else(|_| std::fs::copy(file, &to).and_then(|_| std::fs::remove_file(file)))
            .map_err(failed)?;
    }
    Ok(settings.len())
}

/// The program that is the game, in its folder: what Steam itself runs.
pub fn game_program(game_dir: &Path) -> Option<PathBuf> {
    // From 1.7 the program is `RimWorld`; under Unity it was named by system.
    [
        "RimWorld",
        "RimWorld.exe",
        "RimWorldLinux64",
        "RimWorldLinux",
        "RimWorldWin64.exe",
        "RimWorldWin.exe",
        // On macOS the bundle lies in the game's folder, or is it.
        "RimWorldMac.app/Contents/MacOS/RimWorld by Ludeon Studios",
        "RimWorld.app/Contents/MacOS/RimWorld",
        "RimWorld.app/Contents/MacOS/RimWorld by Ludeon Studios",
        "Contents/MacOS/RimWorld",
        "Contents/MacOS/RimWorld by Ludeon Studios",
    ]
    .iter()
    .map(|name| game_dir.join(name))
    .find(|file| file.is_file())
}

/// Whether the game is running: a process started from its folder whose
/// program is the game's, or the .NET host given the game's `RimWorld.dll`.
/// `proc` is `/proc`; anywhere it cannot be read, the answer is no.
pub fn game_running(proc: &Path, game_dir: &Path) -> bool {
    let Some(program) = game_program(game_dir) else {
        return false;
    };
    let code = game_dir.join("RimWorld.dll");
    let is_game = |command: &[u8]| {
        // The parts of a command line are divided by zero bytes.
        let mut parts = command
            .split(|byte| *byte == 0)
            .map(String::from_utf8_lossy);
        parts.any(|part| Path::new(part.as_ref()) == program || Path::new(part.as_ref()) == code)
    };
    let processes = std::fs::read_dir(proc).into_iter().flatten().flatten();
    processes
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .any(|entry| {
            // A program started as "./RimWorld" still links to its file.
            let linked =
                std::fs::read_link(entry.path().join("exe")).is_ok_and(|exe| exe == program);
            linked || matches!(std::fs::read(entry.path().join("cmdline")), Ok(command) if is_game(&command))
        })
}

/// Starts the game without Steam, with these arguments, and lets it run.
pub fn start_game(game_dir: &Path, arguments: &str) -> Result<(), String> {
    let program = game_program(game_dir)
        .ok_or_else(|| say!("the game's program was not found in its folder"))?;
    std::process::Command::new(&program)
        .args(arguments.split_whitespace())
        .current_dir(game_dir)
        .spawn()
        .map(drop)
        .map_err(|e| format!("{}: {e}", program.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_runs_when_a_process_was_started_from_its_program_or_its_code() {
        let tmp = tempfile::tempdir().unwrap();
        let (game, proc) = (tmp.path().join("game"), tmp.path().join("proc"));
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("RimWorld"), "").unwrap();
        let process = |id: &str, command: String| {
            std::fs::create_dir_all(proc.join(id)).unwrap();
            std::fs::write(proc.join(id).join("cmdline"), command.replace(' ', "\0")).unwrap();
        };
        assert!(!game_running(&proc, &game));
        process("12", "/usr/bin/editor notes-about-RimWorld.txt".to_string());
        process("self", format!("{}", game.join("RimWorld").display()));
        assert!(!game_running(&proc, &game));
        process(
            "40",
            format!(
                "/runtime/dotnet {} -quicktest",
                game.join("RimWorld.dll").display()
            ),
        );
        assert!(game_running(&proc, &game));
        std::fs::remove_dir_all(proc.join("40")).unwrap();
        process(
            "41",
            format!("{} -popupwindow", game.join("RimWorld").display()),
        );
        assert!(game_running(&proc, &game));
        std::fs::remove_dir_all(proc.join("41")).unwrap();
        process("42", "./RimWorld".to_string());
        assert!(!game_running(&proc, &game));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(game.join("RimWorld"), proc.join("42/exe")).unwrap();
            assert!(game_running(&proc, &game));
        }
        assert!(!game_running(&proc, &tmp.path().join("no-game")));
    }

    #[test]
    fn a_mod_is_copied_whole_and_never_over_another() {
        let tmp = tempfile::tempdir().unwrap();
        let (workshop, local) = (tmp.path().join("ws/123"), tmp.path().join("Mods"));
        std::fs::create_dir_all(workshop.join("About")).unwrap();
        std::fs::write(workshop.join("About/About.xml"), "x").unwrap();
        std::fs::create_dir_all(&local).unwrap();
        let made = copy_to_local(&workshop, &local).unwrap();
        assert_eq!(made, local.join("123"));
        assert_eq!(
            std::fs::read_to_string(made.join("About/About.xml")).unwrap(),
            "x"
        );
        assert!(workshop.join("About/About.xml").is_file());
        assert!(copy_to_local(&workshop, &local).is_err());
    }

    #[test]
    fn only_mod_settings_are_set_aside() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("Config");
        std::fs::create_dir_all(&config).unwrap();
        for name in ["Mod_1_A.xml", "Mod_2_B.xml", "ModsConfig.xml", "Prefs.xml"] {
            std::fs::write(config.join(name), name).unwrap();
        }
        let aside = tmp.path().join("aside");
        assert_eq!(set_mod_settings_aside(tmp.path(), &aside), Ok(2));
        assert!(config.join("ModsConfig.xml").is_file() && config.join("Prefs.xml").is_file());
        assert!(!config.join("Mod_1_A.xml").exists());
        assert_eq!(
            std::fs::read_to_string(aside.join("Mod_2_B.xml")).unwrap(),
            "Mod_2_B.xml"
        );
        assert_eq!(set_mod_settings_aside(tmp.path(), &aside), Ok(0));
    }

    #[test]
    fn the_game_s_program_is_found_by_the_name_each_system_gives_it() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(game_program(tmp.path()), None);
        assert!(start_game(tmp.path(), "").is_err());
        std::fs::write(tmp.path().join("RimWorldWin64.exe"), "").unwrap();
        assert_eq!(
            game_program(tmp.path()),
            Some(tmp.path().join("RimWorldWin64.exe"))
        );
    }
}
