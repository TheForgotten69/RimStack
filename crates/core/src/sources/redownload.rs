//! Workshop mods downloaded with SteamCMD as an anonymous user: SteamCMD
//! itself installed, a mod downloaded again from clean in the place of
//! the copy on disk, and mods never subscribed to put in the game's own
//! `Mods` folder.

use crate::base::error::{Error, Kind, Result, Sentence};
use crate::said;

use std::path::{Path, PathBuf};

use crate::rimworld::paths::APP_ID as GAME;

/// SteamCMD: the one RimStack installed under `data_dir`, or the one
/// the system's `PATH` has.
pub fn steamcmd(data_dir: &Path) -> Option<PathBuf> {
    let own = ["steamcmd.sh", "steamcmd.exe"].map(|name| data_dir.join("steamcmd").join(name));
    if let Some(own) = own.into_iter().find(|file| file.is_file()) {
        return Some(own);
    }
    let path = std::env::var_os("PATH")?;
    let named = |dir: PathBuf| ["steamcmd", "steamcmd.exe"].map(|name| dir.join(name));
    let mut places = std::env::split_paths(&path).flat_map(named);
    places.find(|file| file.is_file())
}

/// Downloads Valve's SteamCMD and unpacks it under `data_dir`. Uses the
/// network and the system's `tar`. SteamCMD fetches the rest of itself
/// the first time it runs.
pub fn install(data_dir: &Path) -> Result<PathBuf> {
    // Windows has shipped a tar that also reads zip files since 2018.
    let (archive, program) = match std::env::consts::OS {
        "linux" => ("steamcmd_linux.tar.gz", "steamcmd.sh"),
        "macos" => ("steamcmd_osx.tar.gz", "steamcmd.sh"),
        "windows" => ("steamcmd.zip", "steamcmd.exe"),
        other => {
            return Err(Error::refused(said!(
                "RimStack does not install SteamCMD on {}",
                other
            )))
        }
    };
    let dir = data_dir.join("steamcmd");
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::caused(Kind::File, said!("its folder could not be made"), e))?;
    let agent = crate::base::web::agent(120);
    let url = format!("https://steamcdn-a.akamaihd.net/client/installer/{archive}");
    let mut response = agent
        .get(&url)
        .call()
        .map_err(|e| Error::caused(Kind::Network, said!("the download failed"), e))?;
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|e| Error::caused(Kind::Network, said!("the download failed"), e))?;
    let packed = dir.join(archive);
    std::fs::write(&packed, bytes)
        .map_err(|e| Error::caused(Kind::File, said!("the download could not be kept"), e))?;
    let listed = crate::base::program::run(
        std::process::Command::new("tar").arg("-tf").arg(&packed),
        crate::base::program::LOCAL,
    )
    .map_err(|e| Error::caused(Kind::Program, said!("tar could not be run"), e))?;
    if let Err(problem) = only_inside(&String::from_utf8_lossy(&listed.stdout), program) {
        let _ = std::fs::remove_file(&packed);
        return Err(problem);
    }
    let unpacked = crate::base::program::run(
        std::process::Command::new("tar")
            .arg("-xf")
            .arg(&packed)
            .arg("-C")
            .arg(&dir),
        crate::base::program::NETWORK,
    )
    .map_err(|e| Error::caused(Kind::Program, said!("tar could not be run"), e))?
    .status;
    let _ = std::fs::remove_file(&packed);
    let program = dir.join(program);
    match unpacked.success() && program.is_file() {
        true => Ok(program),
        false => Err(Error::program(said!(
            "the download did not unpack to SteamCMD"
        ))),
    }
}

/// Whether an archive's listing (one entry a line) unpacks only inside
/// its folder and holds the program: Valve publishes nothing to check the
/// download against, so this is what is checked.
fn only_inside(listing: &str, program: &str) -> Result<()> {
    let outside = listing.lines().find(|entry| {
        let entry = Path::new(entry.trim());
        entry.has_root()
            || entry.components().any(|part| {
                !matches!(
                    part,
                    std::path::Component::Normal(_) | std::path::Component::CurDir
                )
            })
    });
    if let Some(entry) = outside {
        return Err(Error::refused(said!(
            "the download would unpack outside its folder: {}",
            entry
        )));
    }
    let holds = |entry: &str| entry.trim().trim_start_matches("./") == program;
    match listing.lines().any(holds) {
        true => Ok(()),
        false => Err(Error::program(said!(
            "the download did not unpack to SteamCMD"
        ))),
    }
}

fn not_an_id(id: &str) -> Error {
    Error::refused(said!("{} is not a Workshop id", id))
}

fn is_workshop_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
}

/// Downloads Workshop items under `scratch` in one run of SteamCMD, as
/// an anonymous user. Returns, per item, its folder or why there is none.
/// Uses the network; takes as long as the mods are large.
pub fn fetch_all(
    steamcmd: &Path,
    ids: &[String],
    scratch: &Path,
) -> Vec<(String, Result<PathBuf>)> {
    let (asked, refused): (Vec<&String>, Vec<&String>) =
        ids.iter().partition(|id| is_workshop_id(id));
    // A home of its own: with the user's, SteamCMD finds their Steam
    // through `~/.steam` and writes its own state over it, the list of
    // library folders included (it did, twice, on 2026-10-12).
    let home = scratch.join("home");
    let _ = std::fs::create_dir_all(&home);
    let mut command = std::process::Command::new(steamcmd);
    command
        .env("HOME", &home)
        .env_remove("XDG_DATA_HOME")
        .env_remove("XDG_CONFIG_HOME")
        .arg("+force_install_dir")
        .arg(scratch)
        .args(["+login", "anonymous"]);
    for id in &asked {
        command.args(["+workshop_download_item", GAME, id, "validate"]);
    }
    let ran = match asked.is_empty() {
        true => Ok(String::new()),
        false => crate::base::program::run(command.arg("+quit"), crate::base::program::DOWNLOAD)
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .map_err(|e| Error::program(said!("SteamCMD could not be run: {}", e))),
    };
    let content = scratch.join("steamapps/workshop/content").join(GAME);
    let outcome = |id: &String| {
        let said = ran.clone()?;
        let fresh = content.join(id);
        if fresh.join("About").is_dir() {
            return Ok(fresh);
        }
        let last = said.lines().rev().find(|l| !l.trim().is_empty());
        Err(Error::program(match last {
            Some(line) => said!("SteamCMD did not download it: {}", line.trim()),
            None => said!("SteamCMD did not download it"),
        }))
    };
    let not_an_id = |id: &&String| ((*id).clone(), Err(not_an_id(id)));
    let mut all: Vec<_> = refused.iter().map(not_an_id).collect();
    all.extend(asked.iter().map(|id| ((*id).clone(), outcome(id))));
    all
}

/// Downloads a Workshop item under `scratch` and returns its folder.
pub fn fetch(steamcmd: &Path, workshop_id: &str, scratch: &Path) -> Result<PathBuf> {
    let mut all = fetch_all(steamcmd, &[workshop_id.to_string()], scratch);
    all.pop().map_or(
        Err(Error::refused(said!("nothing was asked"))),
        |(_, outcome)| outcome,
    )
}

/// Puts a downloaded item in the game's own `Mods` folder under its
/// Workshop id, in the place of an earlier download of it, and notes the
/// id where the game and other tools look for it.
pub fn put_in_local(fresh: &Path, workshop_id: &str, local_mods: &Path) -> Result<PathBuf> {
    if !is_workshop_id(workshop_id) {
        return Err(not_an_id(workshop_id));
    }
    let to = local_mods.join(workshop_id);
    match to.exists() {
        true => replace(&to, fresh)?,
        false => std::fs::rename(fresh, &to)
            .or_else(|_| copy_all(fresh, &to))
            .map_err(|e| Error::file(&to, e))?,
    }
    let noted = to.join("About").join("PublishedFileId.txt");
    if !noted.exists() {
        std::fs::write(&noted, workshop_id).map_err(|e| Error::file(&noted, e))?;
    }
    Ok(to)
}

/// Puts `fresh` where `installed` is. The copy on disk is moved aside
/// first and deleted only once the new one is in place; if that fails it
/// is moved back.
pub fn replace(installed: &Path, fresh: &Path) -> Result<()> {
    // Beside it under its whole name: a folder may have a dot in its own.
    let mut aside = installed.as_os_str().to_os_string();
    aside.push(".rimstack-old");
    let aside = PathBuf::from(aside);
    let failed = |what: Sentence, e: std::io::Error| Error::caused(Kind::File, what, e);
    // Left by a run that could not finish deleting it: in the way of
    // this one, and a second copy of the mod to the game.
    if aside.is_dir() && installed.is_dir() {
        std::fs::remove_dir_all(&aside)
            .map_err(|e| failed(said!("the old copy could not be deleted"), e))?;
    }
    std::fs::rename(installed, &aside)
        .map_err(|e| failed(said!("the mod's folder could not be moved aside"), e))?;
    let moved = std::fs::rename(fresh, installed).or_else(|_| copy_all(fresh, installed));
    if let Err(problem) = moved {
        let _ = std::fs::remove_dir_all(installed);
        let what = match std::fs::rename(&aside, installed) {
            Ok(()) => said!("the new copy could not be put in place"),
            Err(_) => said!(
                "the new copy could not be put in place, and the old one is now {}",
                aside.display()
            ),
        };
        return Err(failed(what, problem));
    }
    std::fs::remove_dir_all(&aside)
        .map_err(|e| failed(said!("the old copy could not be deleted"), e))
}

/// Folders inside folders, as far as a mod goes; a link to a folder
/// above itself would otherwise be followed for ever.
const DEEPEST: usize = 40;

/// For a download on another disk, where a folder cannot be renamed
/// across. What a link points at is copied in its place.
pub(crate) fn copy_all(from: &Path, to: &Path) -> std::io::Result<()> {
    copy_under(from, to, DEEPEST)
}

fn copy_under(from: &Path, to: &Path, deeper: usize) -> std::io::Result<()> {
    let Some(deeper) = deeper.checked_sub(1) else {
        return Err(std::io::Error::other(format!(
            "{}: folders more than {DEEPEST} deep",
            from.display()
        )));
    };
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        match source.is_dir() {
            true => copy_under(&source, &target, deeper)?,
            false => drop(std::fs::copy(&source, target)?),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_archive_is_unpacked_only_when_every_entry_stays_inside_and_it_holds_the_program() {
        assert!(only_inside("steamcmd.sh\nlinux32/\nlinux32/steamcmd\n", "steamcmd.sh").is_ok());
        assert!(only_inside("./steamcmd.sh\n./linux32/libstdc++.so.6\n", "steamcmd.sh").is_ok());
        let escaping = only_inside("steamcmd.sh\n../../.bashrc\n", "steamcmd.sh").unwrap_err();
        assert_eq!(escaping.kind(), Kind::Refused);
        assert!(only_inside("/etc/profile.d/x.sh\nsteamcmd.sh\n", "steamcmd.sh").is_err());
        assert_eq!(
            only_inside("readme.txt\n", "steamcmd.sh")
                .unwrap_err()
                .kind(),
            Kind::Program
        );
    }

    #[test]
    fn the_fresh_copy_takes_the_place_of_the_one_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let (installed, fresh) = (tmp.path().join("123"), tmp.path().join("new"));
        std::fs::create_dir_all(installed.join("About")).unwrap();
        std::fs::write(installed.join("stale.txt"), "old").unwrap();
        std::fs::create_dir_all(fresh.join("About")).unwrap();
        std::fs::write(fresh.join("About/About.xml"), "new").unwrap();
        replace(&installed, &fresh).unwrap();
        assert_eq!(
            std::fs::read_to_string(installed.join("About/About.xml")).unwrap(),
            "new"
        );
        assert!(!installed.join("stale.txt").exists());
        assert!(!tmp.path().join("123.rimstack-old").exists());
    }

    #[test]
    fn a_folder_with_a_dot_in_its_name_is_set_aside_under_that_name() {
        let tmp = tempfile::tempdir().unwrap();
        let (installed, other) = (
            tmp.path().join("Mod.v2"),
            tmp.path().join("Mod.rimstack-old"),
        );
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("theirs.txt"), "kept").unwrap();
        let fresh = tmp.path().join("new");
        std::fs::create_dir_all(&fresh).unwrap();
        replace(&installed, &fresh).unwrap();
        assert!(other.join("theirs.txt").is_file());
    }

    #[test]
    fn the_copy_on_disk_stays_when_there_is_nothing_to_put_in_its_place() {
        let tmp = tempfile::tempdir().unwrap();
        let installed = tmp.path().join("123");
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::write(installed.join("kept.txt"), "old").unwrap();
        assert!(replace(&installed, &tmp.path().join("missing")).is_err());
        assert_eq!(
            std::fs::read_to_string(installed.join("kept.txt")).unwrap(),
            "old"
        );
    }

    #[test]
    fn a_folder_left_aside_earlier_does_not_stop_the_next_download() {
        let tmp = tempfile::tempdir().unwrap();
        let (installed, fresh) = (tmp.path().join("123"), tmp.path().join("new"));
        let left = tmp.path().join("123.rimstack-old");
        for folder in [&installed, &fresh, &left] {
            std::fs::create_dir_all(folder.join("About")).unwrap();
        }
        assert_eq!(replace(&installed, &fresh), Ok(()));
        assert!(!left.exists());
    }

    #[test]
    fn a_download_is_never_put_outside_the_mods_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let (fresh, mods) = (tmp.path().join("fresh"), tmp.path().join("game/Mods"));
        std::fs::create_dir_all(fresh.join("About")).unwrap();
        std::fs::create_dir_all(&mods).unwrap();
        let put = put_in_local(&fresh, "../../escaped", &mods);
        assert_eq!(put.unwrap_err().kind(), Kind::Refused);
        assert!(!tmp.path().join("escaped").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_folder_is_copied_as_what_it_points_at_and_a_loop_ends() {
        let tmp = tempfile::tempdir().unwrap();
        let (from, shared) = (tmp.path().join("mod"), tmp.path().join("shared"));
        std::fs::create_dir_all(&from).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(shared.join("a.png"), "png").unwrap();
        std::os::unix::fs::symlink(&shared, from.join("Textures")).unwrap();
        let to = tmp.path().join("copy");
        copy_all(&from, &to).unwrap();
        assert_eq!(
            std::fs::read_to_string(to.join("Textures/a.png")).unwrap(),
            "png"
        );
        assert!(!to.join("Textures").is_symlink());

        std::os::unix::fs::symlink(&from, from.join("itself")).unwrap();
        assert!(copy_all(&from, &tmp.path().join("again")).is_err());
    }

    #[test]
    fn only_a_number_is_asked_of_steamcmd() {
        let tmp = tempfile::tempdir().unwrap();
        let asked = fetch(Path::new("/nonexistent/steamcmd"), "12; rm -rf", tmp.path());
        assert_eq!(asked.unwrap_err().kind(), Kind::Refused);
        let missing = fetch(Path::new("/nonexistent/steamcmd"), "123456", tmp.path());
        assert_eq!(missing.unwrap_err().kind(), Kind::Program);
    }

    /// A stand-in for SteamCMD that writes down the home it was given.
    #[cfg(unix)]
    #[test]
    fn steamcmd_is_run_with_a_home_of_its_own_never_the_user_s() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("steamcmd.sh");
        let seen = tmp.path().join("seen");
        let script = format!(
            "#!/bin/sh\necho \"$HOME|$XDG_DATA_HOME\" > '{}'\n",
            seen.display()
        );
        std::fs::write(&fake, script).unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let scratch = tmp.path().join("scratch");
        let _ = fetch_all(&fake, &["123456".to_string()], &scratch);
        let given = std::fs::read_to_string(&seen).unwrap();
        assert_eq!(given.trim(), format!("{}|", scratch.join("home").display()));
        assert_ne!(
            Some(scratch.join("home")),
            std::env::var_os("HOME").map(PathBuf::from)
        );
    }

    #[test]
    fn a_download_lands_in_the_local_mods_under_its_id() {
        let tmp = tempfile::tempdir().unwrap();
        let (fresh, local) = (tmp.path().join("scratch/123"), tmp.path().join("Mods"));
        std::fs::create_dir_all(fresh.join("About")).unwrap();
        std::fs::create_dir_all(&local).unwrap();
        let put = put_in_local(&fresh, "123", &local).unwrap();
        assert_eq!(
            std::fs::read_to_string(put.join("About/PublishedFileId.txt")).unwrap(),
            "123"
        );
        // A later download takes the place of the earlier one.
        std::fs::create_dir_all(fresh.join("About")).unwrap();
        std::fs::write(fresh.join("new.txt"), "2").unwrap();
        put_in_local(&fresh, "123", &local).unwrap();
        assert!(local.join("123/new.txt").is_file());
    }

    #[test]
    fn rimstack_s_own_steamcmd_is_found_before_the_system_s() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("steamcmd")).unwrap();
        std::fs::write(tmp.path().join("steamcmd/steamcmd.sh"), "").unwrap();
        assert_eq!(
            steamcmd(tmp.path()),
            Some(tmp.path().join("steamcmd/steamcmd.sh"))
        );
    }

    /// Installs SteamCMD into RIMSTACK_SCRATCH and downloads one small mod
    /// (Harmony's Workshop item) with it. Several hundred megabytes.
    #[test]
    #[ignore = "needs the network and RIMSTACK_SCRATCH: an empty folder"]
    fn real_steamcmd_is_installed_and_downloads_a_mod() {
        let scratch = PathBuf::from(std::env::var_os("RIMSTACK_SCRATCH").unwrap());
        let program = steamcmd(&scratch)
            .map_or_else(|| install(&scratch), Ok)
            .unwrap();
        println!("installed {}", program.display());
        let fresh = fetch(&program, "2009463077", &scratch.join("downloads")).unwrap();
        println!("downloaded {}", fresh.display());
        let local = scratch.join("Mods");
        std::fs::create_dir_all(&local).unwrap();
        let put = put_in_local(&fresh, "2009463077", &local).unwrap();
        assert!(put.join("About/About.xml").is_file());
    }
}
