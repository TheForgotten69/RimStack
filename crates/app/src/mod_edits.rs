//! What you note about a mod and the rules of your own: tags, a note, a
//! category or group chosen by hand, "works on this game version". Each
//! is kept on disk at once; the lists are told to read it by QML.

use rimstack_core::list::Category;
use rimstack_core::locks::Shared;
use rimstack_core::notes;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, rules_version, cxx_name = "rulesVersion")]
        type ModEdits = super::ModEditsRust;
    }

    extern "RustQt" {
        /// Keep your tags (separated by commas) and a note for the mod
        /// with this library id.
        #[qinvokable]
        #[cxx_name = "setNote"]
        fn set_note(self: &ModEdits, mod_id: i32, tags: &QString, note: &QString);

        #[qinvokable]
        #[cxx_name = "addTag"]
        fn add_tag(self: &ModEdits, mod_id: i32, tag: &QString);

        /// Stop, or start again, warning that this mod does not list the
        /// game's version.
        #[qinvokable]
        #[cxx_name = "setVersionOk"]
        fn set_version_ok(self: &ModEdits, mod_id: i32, ok: bool);

        /// File this mod under the category with this label; an empty or
        /// unknown one leaves it to RimStack again.
        #[qinvokable]
        #[cxx_name = "setCategory"]
        fn set_category(self: &ModEdits, mod_id: i32, label: &QString);

        /// Put this mod in a group of yours inside its category; an empty
        /// name takes it out.
        #[qinvokable]
        #[cxx_name = "setSub"]
        fn set_sub(self: &ModEdits, mod_id: i32, name: &QString);

        /// A rule of your own: the second mod loads after the first, both
        /// by library id. `rulesVersion` goes up when one was added.
        #[qinvokable]
        #[cxx_name = "addRule"]
        fn add_rule(self: Pin<&mut ModEdits>, first: i32, then: i32);

        #[qinvokable]
        #[cxx_name = "removeRule"]
        fn remove_rule(self: Pin<&mut ModEdits>, first: i32, then: i32);
    }
}

use core::pin::Pin;
use cxx_qt_lib::QString;

#[derive(Default)]
pub struct ModEditsRust {
    /// Goes up when a rule of your own was added or removed.
    rules_version: i32,
}

/// Changes what is noted about the mod with this library id, and keeps it.
pub(crate) fn change_note(mod_id: i32, change: impl FnOnce(&mut notes::Note)) {
    let loaded = crate::app().library();
    let Ok((library, _)) = &*loaded else {
        return;
    };
    let installed = usize::try_from(mod_id).ok();
    let Some(key) = installed.and_then(|i| library.installed.get(i)?.key()) else {
        return;
    };
    let mut all = crate::app().notes().writing();
    let mut note = all.get(&key).cloned().unwrap_or_default();
    change(&mut note);
    all.set(&key, note);
    if let Err(problem) = all.write(&crate::app().notes_file()) {
        rimstack_core::problem!("{problem}");
    }
}

impl qobject::ModEdits {
    fn set_note(&self, mod_id: i32, tags: &QString, note: &QString) {
        let tags = notes::tags_in(&tags.to_string());
        let text = note.to_string().trim().to_string();
        change_note(mod_id, |kept| {
            kept.tags = tags;
            kept.text = text;
        });
    }

    fn add_tag(&self, mod_id: i32, tag: &QString) {
        let tag = tag.to_string().trim().to_string();
        change_note(mod_id, |note| {
            if !tag.is_empty() && !note.tags.contains(&tag) {
                note.tags.push(tag);
            }
        });
    }

    fn set_version_ok(&self, mod_id: i32, ok: bool) {
        change_note(mod_id, |note| note.version_ok = ok);
    }

    fn set_category(&self, mod_id: i32, label: &QString) {
        let label = label.to_string();
        let known = Category::from_label(&label).map_or("", Category::key);
        change_note(mod_id, |note| note.category = known.to_string());
    }

    fn set_sub(&self, mod_id: i32, name: &QString) {
        let name = name.to_string().trim().to_string();
        change_note(mod_id, |note| note.set_group(&name));
    }

    fn add_rule(self: Pin<&mut Self>, first: i32, then: i32) {
        self.change_rule(first, then, true);
    }

    fn remove_rule(self: Pin<&mut Self>, first: i32, then: i32) {
        self.change_rule(first, then, false);
    }

    fn change_rule(mut self: Pin<&mut Self>, first: i32, then: i32, add: bool) {
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return;
        };
        let key = |id: i32| library.installed.get(usize::try_from(id).ok()?)?.key();
        let (Some(first), Some(then)) = (key(first), key(then)) else {
            return;
        };
        let mut yours = crate::app().user_rules().writing();
        let changed = match add {
            true => yours.add(&first, &then),
            false => yours.remove(&first, &then),
        };
        if !changed {
            return;
        }
        if let Err(problem) = yours.write(&crate::app().user_rules_file()) {
            rimstack_core::problem!("{problem}");
        }
        drop(yours);
        let next = self.rules_version + 1;
        self.as_mut().set_rules_version(next);
    }
}
