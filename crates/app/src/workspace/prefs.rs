//! The appearance and the screens' places, kept where Qt's `Settings` kept them: `RimStack.conf`
//! in the settings folder, so a Qt build's choices carry over on Linux.

use super::App;
use crate::changes::Changes;
use rimstack_core::base::error::Error;
use rimstack_core::kept::prefs::Prefs;
use std::path::PathBuf;

impl App {
    fn prefs_file(&self) -> PathBuf {
        self.settings_dir.join("RimStack.conf")
    }

    pub fn prefs(&self) -> Prefs {
        Prefs::read(&self.prefs_file())
    }

    /// Keeps `key` of `group`; the appearance is told to every view.
    pub fn keep_pref(&self, group: &str, key: &str, value: &str) -> Result<(), Error> {
        let mut prefs = self.prefs();
        if prefs.get(group, key).as_deref() == Some(value) {
            return Ok(());
        }
        prefs.set(group, key, value);
        prefs.write(&self.prefs_file())?;
        if group == "Appearance" {
            self.publish(Changes::THEME);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::workspace::tests::installed;

    #[test]
    fn a_pref_kept_is_read_back_and_the_theme_is_told() {
        let (_tmp, app) = installed();
        let told = std::sync::Arc::new(std::sync::Mutex::new(crate::changes::Changes::NONE));
        let heard = told.clone();
        app.listen(move |changes| *heard.lock().unwrap() = changes);
        app.keep_pref("Appearance", "theme", "light").unwrap();
        assert_eq!(
            app.prefs().get("Appearance", "theme").as_deref(),
            Some("light")
        );
        assert!(
            told.lock()
                .unwrap()
                .contains(crate::changes::Changes::THEME)
        );
    }
}
