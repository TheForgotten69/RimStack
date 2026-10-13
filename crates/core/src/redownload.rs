//! Workshop mods downloaded with SteamCMD as an anonymous user: SteamCMD
//! itself installed, a mod downloaded again from clean in the place of
//! the copy on disk, and mods never subscribed to put in the game's own
//! `Mods` folder.

use crate::say;

use std::path::{Path, PathBuf};

const GAME: &str = "294100";

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
pub fn install(data_dir: &Path) -> Result<PathBuf, String> {
    // Windows has shipped a tar that also reads zip files since 2018.
    let (archive, program) = match std::env::consts::OS {
        "linux" => ("steamcmd_linux.tar.gz", "steamcmd.sh"),
        "macos" => ("steamcmd_osx.tar.gz", "steamcmd.sh"),
        "windows" => ("steamcmd.zip", "steamcmd.exe"),
        other => return Err(say!("RimStack does not install SteamCMD on {}", other)),
    };
    let dir = data_dir.join("steamcmd");
    let failed = |what: &str, e: &dyn std::fmt::Display| format!("{what}: {e}");
    std::fs::create_dir_all(&dir).map_err(|e| failed(&say!("its folder could not be made"), &e))?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .build()
        .into();
    let url = format!("https://steamcdn-a.akamaihd.net/client/installer/{archive}");
    let mut response = agent
        .get(&url)
        .call()
        .map_err(|e| failed(&say!("the download failed"), &e))?;
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|e| failed(&say!("the download failed"), &e))?;
    let packed = dir.join(archive);
    std::fs::write(&packed, bytes)
        .map_err(|e| failed(&say!("the download could not be kept"), &e))?;
    let unpacked = std::process::Command::new("tar")
        .arg("-xf")
        .arg(&packed)
        .arg("-C")
        .arg(&dir)
        .status()
        .map_err(|e| failed(&say!("tar could not be run"), &e))?;
    let _ = std::fs::remove_file(&packed);
    let program = dir.join(program);
    match unpacked.success() && program.is_file() {
        true => Ok(program),
        false => Err(say!("the download did not unpack to SteamCMD")),
    }
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
) -> Vec<(String, Result<PathBuf, String>)> {
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
        false => command
            .arg("+quit")
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .map_err(|e| say!("SteamCMD could not be run: {}", e)),
    };
    let content = scratch.join("steamapps/workshop/content").join(GAME);
    let outcome = |id: &String| {
        let said = ran.clone()?;
        let fresh = content.join(id);
        if fresh.join("About").is_dir() {
            return Ok(fresh);
        }
        let last = said.lines().rev().find(|l| !l.trim().is_empty());
        Err(say!(
            "SteamCMD did not download it: {}",
            last.map_or_else(|| say!("no reason given"), |line| line.trim().to_string())
        ))
    };
    let not_an_id = |id: &&String| ((*id).clone(), Err(say!("{} is not a Workshop id", id)));
    let mut all: Vec<_> = refused.iter().map(not_an_id).collect();
    all.extend(asked.iter().map(|id| ((*id).clone(), outcome(id))));
    all
}

/// Downloads a Workshop item under `scratch` and returns its folder.
pub fn fetch(steamcmd: &Path, workshop_id: &str, scratch: &Path) -> Result<PathBuf, String> {
    let mut all = fetch_all(steamcmd, &[workshop_id.to_string()], scratch);
    all.pop()
        .map_or(Err(say!("nothing was asked")), |(_, outcome)| outcome)
}

/// Puts a downloaded item in the game's own `Mods` folder under its
/// Workshop id, in the place of an earlier download of it, and notes the
/// id where the game and other tools look for it.
pub fn put_in_local(fresh: &Path, workshop_id: &str, local_mods: &Path) -> Result<PathBuf, String> {
    let to = local_mods.join(workshop_id);
    match to.exists() {
        true => replace(&to, fresh)?,
        false => std::fs::rename(fresh, &to)
            .or_else(|_| copy_all(fresh, &to))
            .map_err(|e| format!("{}: {e}", to.display()))?,
    }
    let noted = to.join("About").join("PublishedFileId.txt");
    if !noted.exists() {
        std::fs::write(&noted, workshop_id).map_err(|e| format!("{}: {e}", noted.display()))?;
    }
    Ok(to)
}

/// Puts `fresh` where `installed` is. The copy on disk is moved aside
/// first and deleted only once the new one is in place; if that fails it
/// is moved back.
pub fn replace(installed: &Path, fresh: &Path) -> Result<(), String> {
    let aside = installed.with_extension("rimstack-old");
    let failed = |what: &str, e: std::io::Error| format!("{what}: {e}");
    std::fs::rename(installed, &aside)
        .map_err(|e| failed(&say!("the mod's folder could not be moved aside"), e))?;
    let moved = std::fs::rename(fresh, installed).or_else(|_| copy_all(fresh, installed));
    if let Err(problem) = moved {
        let _ = std::fs::remove_dir_all(installed);
        let _ = std::fs::rename(&aside, installed);
        return Err(failed(
            &say!("the new copy could not be put in place"),
            problem,
        ));
    }
    std::fs::remove_dir_all(&aside)
        .map_err(|e| failed(&say!("the old copy could not be deleted"), e))
}

/// For a download on another disk, where a folder cannot be renamed across.
pub(crate) fn copy_all(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        match entry.file_type()?.is_dir() {
            true => copy_all(&entry.path(), &target)?,
            false => drop(std::fs::copy(entry.path(), target)?),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn only_a_number_is_asked_of_steamcmd() {
        let tmp = tempfile::tempdir().unwrap();
        let asked = fetch(Path::new("/nonexistent/steamcmd"), "12; rm -rf", tmp.path());
        assert!(asked.unwrap_err().contains("not a Workshop id"));
        let missing = fetch(Path::new("/nonexistent/steamcmd"), "123456", tmp.path());
        assert!(missing.unwrap_err().contains("could not be run"));
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
