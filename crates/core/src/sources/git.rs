//! A mod kept as a Git checkout: where its repository is on the web.

use crate::base::error::{Error, Result};
use crate::said;
use crate::say;

use std::path::Path;

/// The web page of the checkout's `origin`, read from `.git/config`.
pub fn page(mod_root: &Path) -> Option<String> {
    let config = std::fs::read_to_string(mod_root.join(".git").join("config")).ok()?;
    let mut in_origin = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_origin = line == "[remote \"origin\"]";
        } else if let Some(("url", url)) = line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
            if in_origin {
                return web(url);
            }
        }
    }
    None
}

/// `git@host:owner/repo.git` and `https://host/owner/repo.git` alike.
fn web(remote: &str) -> Option<String> {
    let remote = remote.strip_suffix(".git").unwrap_or(remote);
    if remote.starts_with("https://") || remote.starts_with("http://") {
        return Some(remote.to_string());
    }
    let (host, path) = remote.strip_prefix("git@")?.split_once(':')?;
    Some(format!("https://{host}/{path}"))
}

/// The repositories linked in a text, as `https://host/owner/repo`: an
/// https link's first two parts, whatever follows them. Steam's own pages
/// are not repositories.
pub fn links_in(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for word in text.split(|c: char| c.is_whitespace() || c == ',' || c == '<' || c == '>') {
        let Some(rest) = word.strip_prefix("https://") else {
            continue;
        };
        let mut parts = rest.split(['/', '?', '#']).filter(|part| !part.is_empty());
        let (Some(host), Some(owner), Some(repo)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if host.contains("steam") || !host.contains('.') {
            continue;
        }
        let repo = repo.strip_suffix(".git").unwrap_or(repo);
        let link = format!("https://{host}/{owner}/{repo}");
        if !repo.is_empty() && !found.contains(&link) {
            found.push(link);
        }
    }
    found
}

/// Git, told not to run what a checkout's own settings name: a mod's
/// `.git` folder may be a stranger's, and its `config` and `hooks` are
/// programs to Git. This closes the ways that need no server; it is not
/// a cage.
fn git() -> std::process::Command {
    let nowhere = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut git = std::process::Command::new("git");
    git.args(["-c", "core.fsmonitor=false"])
        .args(["-c", &format!("core.hooksPath={nowhere}")])
        .args(["-c", "core.sshCommand=ssh"])
        .args(["-c", "protocol.ext.allow=never"]);
    git
}

/// Settings under which Git runs a program, as `section.key` or
/// `section.*.key` in lower case. A checkout whose own `.git/config`
/// holds one is not run at all: what its key names would run, and the
/// list of such keys cannot be closed from outside one by one.
const RUN_PROGRAMS: [&str; 22] = [
    "core.fsmonitor",
    "core.sshcommand",
    "core.askpass",
    "core.gitproxy",
    "core.hookspath",
    "core.pager",
    "core.editor",
    "credential.helper",
    "credential.*.helper",
    "remote.*.uploadpack",
    "remote.*.receivepack",
    "filter.*.clean",
    "filter.*.smudge",
    "filter.*.process",
    "include.path",
    "includeif.*.path",
    "gpg.program",
    "gpg.*.program",
    "merge.*.driver",
    "diff.external",
    "url.*.insteadof",
    "uploadpack.packobjectshook",
];

/// The keys of a Git config text that name a program, as `section.key`
/// or `section.subsection.key`.
fn programs_in(config: &str) -> Vec<String> {
    let mut found = Vec::new();
    let (mut section, mut sub) = (String::new(), false);
    for line in config.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.split(']').next()) {
            let mut parts = header.splitn(2, |c: char| c.is_whitespace() || c == '.');
            section = parts.next().unwrap_or_default().to_lowercase();
            sub = parts.next().is_some_and(|rest| !rest.trim().is_empty());
            continue;
        }
        let key = line
            .split(['=', ' ', '\t'])
            .next()
            .unwrap_or_default()
            .to_lowercase();
        if key.is_empty() || key.starts_with(['#', ';']) {
            continue;
        }
        let pattern = match sub {
            true => format!("{section}.*.{key}"),
            false => format!("{section}.{key}"),
        };
        if RUN_PROGRAMS.contains(&pattern.as_str()) && !found.contains(&pattern) {
            found.push(pattern);
        }
    }
    found
}

/// Refuses a checkout whose own settings name a program; see `RUN_PROGRAMS`.
fn safe_to_run(mod_root: &Path) -> Result<()> {
    let dot_git = mod_root.join(".git");
    // A `.git` file points at the repository kept elsewhere.
    let repository = match std::fs::read_to_string(&dot_git) {
        Ok(pointer) => match pointer.trim().strip_prefix("gitdir:") {
            Some(dir) => mod_root.join(dir.trim()),
            None => dot_git,
        },
        Err(_) => dot_git,
    };
    let config = std::fs::read_to_string(repository.join("config")).unwrap_or_default();
    let named = programs_in(&config);
    match named.is_empty() {
        true => Ok(()),
        false => Err(Error::refused(said!(
            "Its Git settings name programs to run ({}): RimStack does not run Git there",
            named.join(", ")
        ))),
    }
}

fn not_run(e: std::io::Error) -> Error {
    Error::program(said!("git could not be run: {}", e))
}

/// The folder a repository is cloned into, under `mods`: its last part.
pub fn folder_for(link: &str, mods: &Path) -> std::path::PathBuf {
    mods.join(link.rsplit('/').next().unwrap_or_default())
}

/// Clones a repository into the folder `folder_for` gives; one that is
/// there already is left alone. Uses the network.
pub fn clone(link: &str, mods: &Path) -> Result<std::path::PathBuf> {
    let into = folder_for(link, mods);
    if into.exists() {
        return Err(Error::refused(said!("{} is there already", into.display())));
    }
    let ran = crate::base::program::run(
        git()
            // A link is never an option, whatever it starts with.
            .args(["clone", "--"])
            .arg(link)
            .arg(&into)
            .env("GIT_TERMINAL_PROMPT", "0"),
        crate::base::program::DOWNLOAD,
    )
    .map_err(not_run)?;
    if !ran.status.success() {
        let said = String::from_utf8_lossy(&ran.stderr);
        let last = said.lines().last().map(|line| line.trim().to_string());
        return Err(Error::program(last.unwrap_or_else(|| say!("git failed"))));
    }
    Ok(into)
}

/// What `git --version` answers ("2.47.1"), or nothing where Git cannot
/// be run: asking a checkout for updates needs it.
pub fn installed() -> Option<String> {
    let ran =
        crate::base::program::run(git().arg("--version"), crate::base::program::LOCAL).ok()?;
    let said = String::from_utf8_lossy(&ran.stdout);
    ran.status
        .success()
        .then(|| said.trim().trim_start_matches("git version ").to_string())
}

/// How many commits a checkout is behind its remote, after asking the
/// remote. Uses the network.
pub fn behind(mod_root: &Path) -> Result<usize> {
    safe_to_run(mod_root)?;
    let git = |args: &[&str]| {
        let ran = crate::base::program::run(
            git()
                .arg("-C")
                .arg(mod_root)
                .args(args)
                .env("GIT_TERMINAL_PROMPT", "0"),
            crate::base::program::NETWORK,
        )
        .map_err(not_run)?;
        match ran.status.success() {
            true => Ok(String::from_utf8_lossy(&ran.stdout).trim().to_string()),
            false => {
                let said = String::from_utf8_lossy(&ran.stderr);
                let last = said.lines().last();
                Err(Error::program(last.map_or_else(
                    || say!("git failed"),
                    |line| line.trim().to_string(),
                )))
            }
        }
    };
    git(&["fetch", "--quiet"])?;
    let count = git(&["rev-list", "--count", "HEAD..@{upstream}"])?;
    count
        .parse()
        .map_err(|_| Error::format(said!("git said {}", count)))
}

/// Brings a checkout up to its remote, only by moving forward: local
/// changes or commits stop it. Answers with the commit it is now at.
pub fn update(mod_root: &Path) -> Result<String> {
    safe_to_run(mod_root)?;
    let ran = crate::base::program::run(
        git()
            .arg("-C")
            .arg(mod_root)
            .args(["pull", "--ff-only", "--quiet"])
            .env("GIT_TERMINAL_PROMPT", "0"),
        crate::base::program::NETWORK,
    )
    .map_err(not_run)?;
    if !ran.status.success() {
        let said = String::from_utf8_lossy(&ran.stderr);
        let last = said.lines().last();
        return Err(Error::program(last.map_or_else(
            || say!("git pull failed"),
            |line| line.trim().to_string(),
        )));
    }
    let head = crate::base::program::run(
        git()
            .arg("-C")
            .arg(mod_root)
            .args(["log", "-1", "--format=%h %s"]),
        crate::base::program::LOCAL,
    );
    let now = head.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    Ok(now.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    #[test]
    fn repositories_are_found_among_other_links() {
        let text = "see https://github.com/Owner/Repo.git and https://gitlab.com/a/b/-/tree/main,\nhttps://steamcommunity.com/sharedfiles/filedetails/?id=1 https://github.com/Owner/Repo";
        assert_eq!(
            super::links_in(text),
            ["https://github.com/Owner/Repo", "https://gitlab.com/a/b"]
        );
        assert!(super::links_in("https://github.com/owner git@github.com:a/b.git").is_empty());
        let into = super::folder_for("https://github.com/Owner/Repo", std::path::Path::new("/m"));
        assert_eq!(into, std::path::Path::new("/m/Repo"));
    }

    use super::*;

    fn checkout(config: &str) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join(".git/config"), config).unwrap();
        tmp
    }

    #[test]
    fn the_origin_s_page_is_read_from_the_checkout() {
        let https = checkout("[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = https://example.org/up.git\n[remote \"origin\"]\n\turl = https://github.com/me/mod.git\n");
        assert_eq!(
            page(https.path()).as_deref(),
            Some("https://github.com/me/mod")
        );
        let ssh = checkout("[remote \"origin\"]\n\turl = git@gitgud.io:me/mod.git\n");
        assert_eq!(
            page(ssh.path()).as_deref(),
            Some("https://gitgud.io/me/mod")
        );
    }

    #[test]
    fn a_folder_that_is_no_checkout_or_has_no_origin_has_no_page() {
        assert_eq!(page(tempfile::tempdir().unwrap().path()), None);
        assert_eq!(page(checkout("[core]\n\tbare = false\n").path()), None);
        assert_eq!(
            page(checkout("[remote \"origin\"]\n\turl = /srv/local\n").path()),
            None
        );
    }

    fn git(dir: &Path, args: &[&str]) {
        let identity = ["-c", "user.name=t", "-c", "user.email=t@example.org"];
        let ran = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(identity)
            .args(args)
            .output()
            .unwrap();
        assert!(
            ran.status.success(),
            "{}",
            String::from_utf8_lossy(&ran.stderr)
        );
    }

    #[test]
    fn a_checkout_moves_forward_to_its_remote_and_not_over_local_commits() {
        let tmp = tempfile::tempdir().unwrap();
        let (origin, clone) = (tmp.path().join("origin"), tmp.path().join("clone"));
        std::fs::create_dir(&origin).unwrap();
        git(&origin, &["init", "--quiet", "-b", "main"]);
        std::fs::write(origin.join("a"), "1").unwrap();
        git(&origin, &["add", "a"]);
        git(&origin, &["commit", "--quiet", "-m", "first"]);
        git(tmp.path(), &["clone", "--quiet", "origin", "clone"]);
        std::fs::write(origin.join("a"), "2").unwrap();
        git(&origin, &["commit", "--quiet", "-am", "second"]);
        assert_eq!(behind(&clone), Ok(1));
        assert!(update(&clone).unwrap().ends_with("second"));
        assert_eq!(behind(&clone), Ok(0));
        assert!(behind(tmp.path()).is_err());
        assert_eq!(std::fs::read_to_string(clone.join("a")).unwrap(), "2");
        // A commit on each side: not a move forward.
        std::fs::write(clone.join("b"), "x").unwrap();
        git(&clone, &["add", "b"]);
        git(&clone, &["commit", "--quiet", "-m", "local"]);
        std::fs::write(origin.join("a"), "3").unwrap();
        git(&origin, &["commit", "--quiet", "-am", "third"]);
        assert!(update(&clone).is_err());
        assert!(update(tmp.path()).is_err());
    }

    #[test]
    fn settings_that_name_a_program_are_found_whatever_their_case_or_subsection() {
        let config = "[core]\n\tbare = false\n\tfsmonitor = ./evil.sh\n[remote \"origin\"]\n\turl = https://x/y\n\tuploadpack = sh -c id\n[Filter \"lfs\"]\n\tSmudge = git-lfs smudge\n; core.pager = less\n[includeIf \"gitdir:/x\"]\n\tpath = ../more\n";
        assert_eq!(
            programs_in(config),
            [
                "core.fsmonitor",
                "remote.*.uploadpack",
                "filter.*.smudge",
                "includeif.*.path"
            ]
        );
        assert!(
            programs_in("[core]\n\tbare = false\n[remote \"origin\"]\n\turl = git@x:y\n")
                .is_empty()
        );
    }

    #[test]
    fn a_checkout_naming_a_program_is_neither_asked_nor_updated() {
        let tmp = checkout("[core]\n\tsshCommand = touch /tmp/owned\n");
        let asked = behind(tmp.path()).unwrap_err();
        assert_eq!(asked.kind(), crate::base::error::Kind::Refused);
        assert!(asked.to_string().contains("core.sshcommand"));
        assert_eq!(
            update(tmp.path()).unwrap_err().kind(),
            crate::base::error::Kind::Refused
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_checkout_s_own_hooks_are_not_run() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let (origin, clone) = (tmp.path().join("origin"), tmp.path().join("clone"));
        std::fs::create_dir(&origin).unwrap();
        git(&origin, &["init", "--quiet", "-b", "main"]);
        std::fs::write(origin.join("a"), "1").unwrap();
        git(&origin, &["add", "a"]);
        git(&origin, &["commit", "--quiet", "-m", "first"]);
        git(tmp.path(), &["clone", "--quiet", "origin", "clone"]);
        std::fs::write(origin.join("a"), "2").unwrap();
        git(&origin, &["commit", "--quiet", "-am", "second"]);

        let ran = tmp.path().join("ran");
        let hook = clone.join(".git/hooks/post-merge");
        std::fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", ran.display())).unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(update(&clone).unwrap().ends_with("second"));
        assert!(!ran.exists());
    }

    /// Clones Harmony's repository into RIMSTACK_SCRATCH, then asks its
    /// remote and updates it: a few megabytes.
    #[test]
    #[ignore = "needs the network, git and RIMSTACK_SCRATCH: an empty folder"]
    fn a_real_repository_is_cloned_asked_and_updated() {
        let mods = std::path::PathBuf::from(std::env::var_os("RIMSTACK_SCRATCH").unwrap());
        let link = links_in("see https://github.com/pardeike/HarmonyRimWorld.git").remove(0);
        let into = clone(&link, &mods).unwrap();
        assert!(into.join("About/About.xml").is_file());
        assert_eq!(page(&into).as_deref(), Some(link.as_str()));
        assert_eq!(
            clone(&link, &mods).unwrap_err().kind(),
            crate::base::error::Kind::Refused
        );
        assert_eq!(behind(&into), Ok(0));
        let now_at = update(&into).unwrap();
        println!("cloned {} at {now_at}", into.display());
        assert!(!now_at.is_empty());
    }
}
