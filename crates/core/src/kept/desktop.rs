//! RimStack among the desktop's applications: a Linux desktop finds a
//! window's icon and name through an entry it is told about, not in the
//! program itself, so the program writes that entry and its icon.

use std::path::Path;

/// The program the entry should start, from the environment RimStack
/// was started in: none where a package (Flatpak, Snap) brings its own
/// entry, the AppImage file rather than the folder it is mounted at for
/// this run, or else the program itself.
pub fn program_to_list(
    exe: &Path,
    var: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Option<std::path::PathBuf> {
    if var("FLATPAK_ID").is_some() || var("SNAP").is_some() {
        return None;
    }
    match var("APPIMAGE") {
        Some(image) => Some(image.into()),
        None => Some(exe.to_path_buf()),
    }
}

/// Writes the entry and the icon under `data_home` (`~/.local/share`),
/// pointing at `exe`; leaves files that already say the same alone.
/// Returns whether the desktop has to be told its icons changed: the
/// icon was written, or was written before without the theme's folder
/// being marked as changed.
pub fn install(data_home: &Path, exe: &Path, entry: &str, icon: &[u8]) -> std::io::Result<bool> {
    let launch = format!("Exec=\"{}\"", exe.display());
    let entry: Vec<&str> = entry
        .lines()
        .map(|line| match line.starts_with("Exec=") {
            true => launch.as_str(),
            false => line,
        })
        .collect();
    let files = [
        (
            data_home.join("applications/rimstack.desktop"),
            (entry.join("\n") + "\n").into_bytes(),
        ),
        (
            data_home.join("icons/hicolor/scalable/apps/rimstack.svg"),
            icon.to_vec(),
        ),
    ];
    let mut wrote = false;
    for (path, wanted) in files {
        if std::fs::read(&path).is_ok_and(|now| now == wanted) {
            continue;
        }
        wrote = true;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, wanted)?;
    }
    // A desktop keeps a list of a theme's icons and reads the folders
    // again only when the theme's own folder is newer than that list
    // (the icon theme specification asks installers to mark it so).
    let theme = data_home.join("icons/hicolor");
    let written = |path: &Path| std::fs::metadata(path).and_then(|m| m.modified());
    let icon = theme.join("scalable/apps/rimstack.svg");
    let stale = wrote || written(&icon)? > written(&theme)?;
    if stale {
        std::fs::File::open(&theme)?.set_modified(std::time::SystemTime::now())?;
    }
    Ok(stale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_points_at_the_program_and_is_written_once() {
        let home = tempfile::tempdir().unwrap();
        let entry = "[Desktop Entry]\nName=RimStack\nExec=rimstack\nIcon=rimstack";
        install(
            home.path(),
            Path::new("/opt/Rim Stack/rimstack"),
            entry,
            b"<svg/>",
        )
        .unwrap();
        let written = home.path().join("applications/rimstack.desktop");
        assert_eq!(
            std::fs::read_to_string(&written).unwrap(),
            "[Desktop Entry]\nName=RimStack\nExec=\"/opt/Rim Stack/rimstack\"\nIcon=rimstack\n"
        );
        let icon = home.path().join("icons/hicolor/scalable/apps/rimstack.svg");
        assert_eq!(std::fs::read(&icon).unwrap(), b"<svg/>");
        let before = std::fs::metadata(&written).unwrap().modified().unwrap();
        let again = install(
            home.path(),
            Path::new("/opt/Rim Stack/rimstack"),
            entry,
            b"<svg/>",
        );
        // Nothing written, and the theme's folder is already the newer.
        assert!(!again.unwrap());
        assert_eq!(
            std::fs::metadata(&written).unwrap().modified().unwrap(),
            before
        );
        install(
            home.path(),
            Path::new("/usr/bin/rimstack"),
            entry,
            b"<svg/>",
        )
        .unwrap();
        assert!(std::fs::read_to_string(&written)
            .unwrap()
            .contains("Exec=\"/usr/bin/rimstack\""));
    }

    #[test]
    fn an_icon_newer_than_its_theme_s_folder_marks_the_folder_changed() {
        let home = tempfile::tempdir().unwrap();
        let exe = Path::new("/usr/bin/rimstack");
        assert!(install(home.path(), exe, "Exec=rimstack", b"<svg/>").unwrap());
        assert!(!install(home.path(), exe, "Exec=rimstack", b"<svg/>").unwrap());
        // The icon redrawn: the folder it is in changes, the theme's own does not.
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(install(home.path(), exe, "Exec=rimstack", b"<svg>new</svg>").unwrap());
        let theme = home.path().join("icons/hicolor");
        let icon = theme.join("scalable/apps/rimstack.svg");
        let at = |path: &Path| std::fs::metadata(path).unwrap().modified().unwrap();
        assert!(at(&theme) >= at(&icon));
    }

    #[test]
    fn a_package_s_own_entry_is_left_alone_and_an_appimage_is_listed_by_its_file() {
        let exe = Path::new("/tmp/.mount_RimStaXyz/usr/bin/rimstack");
        let set = |name: &'static str, value: &'static str| {
            move |asked: &str| (asked == name).then(|| value.into())
        };
        assert_eq!(program_to_list(exe, |_| None), Some(exe.to_path_buf()));
        assert_eq!(
            program_to_list(exe, set("FLATPAK_ID", "io.github.RimStack")),
            None
        );
        assert_eq!(program_to_list(exe, set("SNAP", "/snap/rimstack/1")), None);
        assert_eq!(
            program_to_list(exe, set("APPIMAGE", "/home/a/RimStack.AppImage")),
            Some("/home/a/RimStack.AppImage".into())
        );
    }
}
