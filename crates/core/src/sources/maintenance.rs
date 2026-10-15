//! Things done to the files of mods and of the game on request: a
//! Workshop mod copied to the game's own `Mods` folder so that Steam no
//! longer updates it, the settings mods keep, set aside, and the game
//! started without Steam.

use crate::base::error::{Error, Result};
use crate::said;

use std::path::{Path, PathBuf};

/// Copies a mod's folder into `local_mods` under its own folder name.
/// Nothing is replaced: a folder of that name stops it.
pub fn copy_to_local(mod_root: &Path, local_mods: &Path) -> Result<PathBuf> {
    let name = mod_root
        .file_name()
        .ok_or_else(|| Error::refused(said!("the mod's folder has no name")))?;
    let to = local_mods.join(name);
    if to.exists() {
        return Err(Error::refused(said!("{} is there already", to.display())));
    }
    crate::base::walk::copy(mod_root, &to).map_err(|e| {
        let _ = std::fs::remove_dir_all(&to);
        Error::file(&to, e)
    })?;
    Ok(to)
}

/// Moves every mod's settings file (`Mod_*.xml` in the game's `Config`
/// folder) into `aside`, all of them or none. Returns how many; they can
/// be moved back by hand.
pub fn set_mod_settings_aside(config: &Path, aside: &Path) -> Result<usize> {
    let failed = |e: std::io::Error| Error::file(aside, e);
    let files = std::fs::read_dir(config.join("Config")).map_err(|e| Error::file(config, e))?;
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
    // Every file is copied before any is taken away: one that cannot be
    // copied leaves the mods with all their settings, not with half.
    std::fs::create_dir_all(aside).map_err(failed)?;
    let copied = settings.iter().try_for_each(|file| {
        let to = aside.join(file.file_name().unwrap_or_default());
        std::fs::copy(file, to).map(drop)
    });
    if let Err(problem) = copied {
        let _ = std::fs::remove_dir_all(aside);
        return Err(failed(problem));
    }
    for file in &settings {
        std::fs::remove_file(file).map_err(|e| Error::file(file, e))?;
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Process {
    pub program: Option<PathBuf>,
    pub command: Vec<std::ffi::OsString>,
}

pub fn processes() -> Vec<Process> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut system = System::new();
    let wanted = ProcessRefreshKind::nothing()
        .with_exe(UpdateKind::Always)
        .with_cmd(UpdateKind::Always);
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, wanted);
    system
        .processes()
        .values()
        .map(|process| Process {
            program: process.exe().map(Path::to_path_buf),
            command: process.cmd().to_vec(),
        })
        .collect()
}

fn same_file(listed: &Path, wanted: &Path) -> bool {
    if listed == wanted {
        return true;
    }
    let lower = |path: &Path| path.to_string_lossy().to_lowercase();
    !cfg!(target_os = "linux") && lower(listed) == lower(wanted)
}

/// Whether one of `processes` is the game: started from its program, or
/// the .NET host given the game's `RimWorld.dll`.
pub fn game_among(processes: &[Process], game_dir: &Path) -> bool {
    let Some(program) = game_program(game_dir) else {
        return false;
    };
    let program = dunce::canonicalize(&program).unwrap_or(program);
    let code = game_dir.join("RimWorld.dll");
    let code = dunce::canonicalize(&code).unwrap_or(code);
    processes.iter().any(|process| {
        // A program started as "./RimWorld" is still listed by its file.
        let started = process
            .program
            .as_deref()
            .is_some_and(|listed| same_file(listed, &program));
        let named = process.command.iter().map(Path::new);
        started
            || named
                .into_iter()
                .any(|part| same_file(part, &program) || same_file(part, &code))
    })
}

pub fn game_running(game_dir: &Path) -> bool {
    game_among(&processes(), game_dir)
}

/// A line of launch options as its arguments: divided at spaces, but
/// not inside quotes, which are taken off. `-savedatafolder="/my saves"`
/// is one argument. A quote left open runs to the end.
pub fn arguments(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut now = String::new();
    let (mut quoted, mut begun) = (None, false);
    for c in line.chars() {
        match quoted {
            Some(quote) if c == quote => quoted = None,
            Some(_) => now.push(c),
            None if c == '"' || c == '\'' => (quoted, begun) = (Some(c), true),
            None if c.is_whitespace() => {
                if begun {
                    found.push(std::mem::take(&mut now));
                }
                begun = false;
            }
            None => {
                now.push(c);
                begun = true;
            }
        }
    }
    if begun {
        found.push(now);
    }
    found
}

/// Starts the game without Steam, with these arguments, and lets it run.
pub fn start_game(game_dir: &Path, arguments: &str) -> Result<()> {
    let program = game_program(game_dir)
        .ok_or_else(|| Error::program(said!("the game's program was not found in its folder")))?;
    std::process::Command::new(&program)
        .args(self::arguments(arguments))
        .current_dir(game_dir)
        .spawn()
        .map(drop)
        .map_err(|e| Error::program(format!("{}: {e}", program.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_options_are_divided_at_spaces_outside_quotes() {
        assert_eq!(
            arguments("  -logfile   -popupwindow "),
            ["-logfile", "-popupwindow"]
        );
        assert_eq!(
            arguments(r#"-savedatafolder="/my saves/rim" -quicktest"#),
            ["-savedatafolder=/my saves/rim", "-quicktest"]
        );
        assert_eq!(arguments("'a b' \"\" c"), ["a b", "", "c"]);
        assert_eq!(arguments(r#"-x "left open"#), ["-x", "left open"]);
        assert!(arguments("   ").is_empty());
    }

    #[test]
    fn the_game_runs_when_a_process_was_started_from_its_program_or_its_code() {
        let tmp = tempfile::tempdir().unwrap();
        let game = dunce::canonicalize(tmp.path()).unwrap().join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("RimWorld"), "").unwrap();
        let process = |program: Option<PathBuf>, command: String| Process {
            program,
            command: command.split(' ').map(Into::into).collect(),
        };
        let editor = process(
            Some("/usr/bin/editor".into()),
            "/usr/bin/editor notes-about-RimWorld.txt".to_string(),
        );
        assert!(!game_among(&[], &game));
        assert!(!game_among(std::slice::from_ref(&editor), &game));
        let dotnet = process(
            Some("/runtime/dotnet".into()),
            format!(
                "/runtime/dotnet {} -quicktest",
                game.join("RimWorld.dll").display()
            ),
        );
        assert!(game_among(&[editor.clone(), dotnet], &game));
        let named = process(
            None,
            format!("{} -popupwindow", game.join("RimWorld").display()),
        );
        assert!(game_among(&[named], &game));
        let relative = process(None, "./RimWorld".to_string());
        assert!(!game_among(std::slice::from_ref(&relative), &game));
        let started = Process {
            program: Some(game.join("RimWorld")),
            ..relative
        };
        assert!(game_among(std::slice::from_ref(&started), &game));
        assert!(!game_among(&[started], &tmp.path().join("no-game")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_game_started_now_is_seen_running_and_not_once_it_ended() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::copy("/bin/sleep", game.join("RimWorld")).unwrap();
        let mut running = (0..50)
            .find_map(|_| {
                match std::process::Command::new(game.join("RimWorld"))
                    .arg("30")
                    .spawn()
                {
                    Err(e) if e.raw_os_error() == Some(26) => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        None
                    }
                    other => Some(other.unwrap()),
                }
            })
            .unwrap();
        let seen = game_running(&game);
        running.kill().unwrap();
        running.wait().unwrap();
        assert!(seen);
        assert!(!game_running(&game));
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
        assert_eq!(
            copy_to_local(&workshop, &local).unwrap_err().kind(),
            crate::base::error::Kind::Refused
        );
    }

    #[test]
    fn settings_that_cannot_all_be_copied_all_stay_where_they_are() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("Config");
        std::fs::create_dir_all(config.join("Mod_2_NotAFile.xml")).unwrap();
        std::fs::write(config.join("Mod_1_A.xml"), "kept").unwrap();
        let aside = tmp.path().join("aside");
        assert!(set_mod_settings_aside(tmp.path(), &aside).is_err());
        assert_eq!(
            std::fs::read_to_string(config.join("Mod_1_A.xml")).unwrap(),
            "kept"
        );
        assert!(!aside.exists());
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
