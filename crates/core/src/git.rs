//! A mod kept as a Git checkout: where its repository is on the web.

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

/// The folder a repository is cloned into, under `mods`: its last part.
pub fn folder_for(link: &str, mods: &Path) -> std::path::PathBuf {
    mods.join(link.rsplit('/').next().unwrap_or_default())
}

/// Clones a repository into the folder `folder_for` gives; one that is
/// there already is left alone. Uses the network.
pub fn clone(link: &str, mods: &Path) -> Result<std::path::PathBuf, String> {
    let into = folder_for(link, mods);
    if into.exists() {
        return Err(say!("{} is there already", into.display()));
    }
    let ran = std::process::Command::new("git")
        .arg("clone")
        .arg(link)
        .arg(&into)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| say!("git could not be run: {}", e))?;
    if !ran.status.success() {
        let said = String::from_utf8_lossy(&ran.stderr);
        let last = said.lines().last().map(|line| line.trim().to_string());
        return Err(last.unwrap_or_else(|| say!("git failed")));
    }
    Ok(into)
}

/// What `git --version` answers ("2.47.1"), or nothing where Git cannot
/// be run: asking a checkout for updates needs it.
pub fn installed() -> Option<String> {
    let ran = std::process::Command::new("git")
        .arg("--version")
        .output()
        .ok()?;
    let said = String::from_utf8_lossy(&ran.stdout);
    ran.status
        .success()
        .then(|| said.trim().trim_start_matches("git version ").to_string())
}

/// How many commits a checkout is behind its remote, after asking the
/// remote. Uses the network.
pub fn behind(mod_root: &Path) -> Result<usize, String> {
    let git = |args: &[&str]| {
        let ran = std::process::Command::new("git")
            .arg("-C")
            .arg(mod_root)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|e| say!("git could not be run: {}", e))?;
        match ran.status.success() {
            true => Ok(String::from_utf8_lossy(&ran.stdout).trim().to_string()),
            false => {
                let said = String::from_utf8_lossy(&ran.stderr);
                Err(said
                    .lines()
                    .last()
                    .map_or_else(|| say!("git failed"), |line| line.trim().to_string()))
            }
        }
    };
    git(&["fetch", "--quiet"])?;
    let count = git(&["rev-list", "--count", "HEAD..@{upstream}"])?;
    count.parse().map_err(|_| say!("git said {}", count))
}

/// Brings a checkout up to its remote, only by moving forward: local
/// changes or commits stop it. Answers with the commit it is now at.
pub fn update(mod_root: &Path) -> Result<String, String> {
    let ran = std::process::Command::new("git")
        .arg("-C")
        .arg(mod_root)
        .args(["pull", "--ff-only", "--quiet"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| say!("git could not be run: {}", e))?;
    if !ran.status.success() {
        let said = String::from_utf8_lossy(&ran.stderr);
        return Err(said
            .lines()
            .last()
            .map_or_else(|| say!("git pull failed"), |line| line.trim().to_string()));
    }
    let head = std::process::Command::new("git")
        .arg("-C")
        .arg(mod_root)
        .args(["log", "-1", "--format=%h %s"])
        .output();
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
}
