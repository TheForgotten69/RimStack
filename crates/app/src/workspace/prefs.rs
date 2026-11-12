//! The appearance and the screens' places, kept where Qt's `Settings` kept them: `RimStack.conf`
//! in the settings folder, so a Qt build's choices carry over on Linux.

use super::App;
use crate::changes::Changes;
use rimstack_core::base::error::Error;
use rimstack_core::kept::prefs::Prefs;
use std::path::PathBuf;

/// How the Mods tab is shown, under `[Mods]`, as Qt's `ModsPrefs` kept it.
#[derive(Debug, Clone, PartialEq)]
pub struct ModsView {
    /// As `columns::chosen` reads it.
    pub columns: String,
    /// As `columns::widths` reads it.
    pub column_sizes: String,
    /// The load order under category headings; Qt kept 1, 0 or -1, and only 0 turns them off.
    pub headings: bool,
    /// As `columns::from_kept` reads it; "" by category.
    pub inactive_order: String,
    /// The inactive mods folded into a bar at the left edge.
    pub inactive_hidden: bool,
    /// The panes' widths in pixels as last dragged; 0 until then.
    pub inactive: f32,
    pub details: f32,
    /// Where the tab was left: the mod chosen by package id, its details' tab, the filter, and
    /// how far the load order was scrolled.
    pub last_mod: String,
    pub last_tab: usize,
    pub last_filter: String,
    pub last_scroll: f32,
}

impl Default for ModsView {
    fn default() -> Self {
        Self {
            columns: "author,version,load".into(),
            column_sizes: String::new(),
            headings: true,
            inactive_order: String::new(),
            inactive_hidden: false,
            inactive: 0.,
            details: 0.,
            last_mod: String::new(),
            last_tab: 0,
            last_filter: String::new(),
            last_scroll: 0.,
        }
    }
}

impl ModsView {
    const GROUP: &str = "Mods";

    pub fn read(prefs: &Prefs) -> Self {
        let first = Self::default();
        let text = |key: &str, or: String| prefs.get(Self::GROUP, key).unwrap_or(or);
        let number = |key: &str| prefs.number(Self::GROUP, key).unwrap_or(0.) as f32;
        Self {
            columns: text("columns", first.columns),
            column_sizes: text("columnSizes", first.column_sizes),
            headings: prefs.number(Self::GROUP, "headings") != Some(0.),
            inactive_order: text("inactiveOrder", first.inactive_order),
            inactive_hidden: prefs.flag(Self::GROUP, "inactiveHidden").unwrap_or(false),
            inactive: number("inactive"),
            details: number("details"),
            last_mod: text("lastMod", first.last_mod),
            last_tab: prefs.number(Self::GROUP, "lastTab").unwrap_or(0.) as usize,
            last_filter: text("lastFilter", first.last_filter),
            last_scroll: number("lastScroll"),
        }
    }

    fn write(&self, prefs: &mut Prefs) {
        let flag = |on: bool| if on { "true" } else { "false" };
        let pairs = [
            ("columns", self.columns.clone()),
            ("columnSizes", self.column_sizes.clone()),
            ("headings", if self.headings { "1" } else { "0" }.into()),
            ("inactiveOrder", self.inactive_order.clone()),
            ("inactiveHidden", flag(self.inactive_hidden).into()),
            ("inactive", self.inactive.round().to_string()),
            ("details", self.details.round().to_string()),
            ("lastMod", self.last_mod.clone()),
            ("lastTab", self.last_tab.to_string()),
            ("lastFilter", self.last_filter.clone()),
            ("lastScroll", self.last_scroll.round().to_string()),
        ];
        for (key, value) in pairs {
            if prefs.get(Self::GROUP, key).as_deref() != Some(value.as_str()) {
                prefs.set(Self::GROUP, key, &value);
            }
        }
    }
}

impl App {
    pub fn mods_view(&self) -> ModsView {
        ModsView::read(&self.prefs())
    }

    /// Keeps what differs from the file, in one write.
    pub fn keep_mods_view(&self, view: &ModsView) -> Result<(), Error> {
        let mut prefs = self.prefs();
        if ModsView::read(&prefs) == *view {
            return Ok(());
        }
        view.write(&mut prefs);
        prefs.write(&self.prefs_file())
    }

    fn prefs_file(&self) -> PathBuf {
        self.settings_dir.join("RimStack.conf")
    }

    pub fn prefs(&self) -> Prefs {
        Prefs::read(&self.prefs_file())
    }

    /// The interface size of RimStack before 0.6 (`interfaceScale` in the settings) carried into
    /// the text size once, as `Main.qml` carried it, and forgotten.
    pub fn carry_interface_scale(&self) -> Result<(), Error> {
        let size = self
            .settings()
            .get("interfaceScale")
            .and_then(|kept| kept.parse::<f64>().ok())
            .filter(|size| *size > 0. && *size != 1.);
        let Some(size) = size else {
            return Ok(());
        };
        let text = self.prefs().number("Appearance", "textScale").unwrap_or(1.);
        let scale = (text * size * 20.).round() / 20.;
        self.keep_pref("Appearance", "textScale", &scale.to_string())?;
        self.keep_setting("interfaceScale", "")
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
    use super::ModsView;
    use crate::workspace::tests::installed;

    #[test]
    fn the_mods_view_qt_kept_is_read_and_kept_again() {
        let (_tmp, app) = installed();
        assert_eq!(app.mods_view(), ModsView::default(), "nothing kept yet");
        app.keep_pref("Mods", "headings", "-1").unwrap();
        app.keep_pref("Mods", "inactiveHidden", "true").unwrap();
        app.keep_pref("Mods", "inactive", "479.5").unwrap();
        let read = app.mods_view();
        assert!(
            read.headings,
            "-1 is headings when the order is close to category order"
        );
        assert!(read.inactive_hidden);
        assert_eq!(read.inactive, 479.5);
        let view = ModsView {
            columns: "version,author".into(),
            headings: false,
            inactive_order: "name down".into(),
            ..read
        };
        app.keep_mods_view(&view).unwrap();
        let again = app.mods_view();
        assert_eq!(again.columns, "version,author");
        assert!(!again.headings);
        assert_eq!(again.inactive_order, "name down");
        assert_eq!(again.inactive, 480., "a width is kept in whole pixels");
        let left = ModsView {
            last_mod: "ludeon.rimworld".into(),
            last_tab: 2,
            last_filter: "is:new author:x".into(),
            last_scroll: 312.4,
            ..again
        };
        app.keep_mods_view(&left).unwrap();
        let again = app.mods_view();
        assert_eq!(
            (
                again.last_mod.as_str(),
                again.last_tab,
                again.last_filter.as_str(),
                again.last_scroll
            ),
            ("ludeon.rimworld", 2, "is:new author:x", 312.)
        );
    }

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

    #[test]
    fn the_interface_size_of_old_is_carried_into_the_text_size_once() {
        let (_tmp, app) = installed();
        app.keep_pref("Appearance", "textScale", "1.1").unwrap();
        app.keep_setting("interfaceScale", "1.5").unwrap();
        app.carry_interface_scale().unwrap();
        assert_eq!(app.prefs().number("Appearance", "textScale"), Some(1.65));
        assert_eq!(app.settings().get("interfaceScale"), None);
        app.carry_interface_scale().unwrap();
        assert_eq!(app.prefs().number("Appearance", "textScale"), Some(1.65));
    }
}
