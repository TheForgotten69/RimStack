//! Changes to the load order and to what is noted about a mod: notes,
//! rules of your own, undo and redo, saving, collapsing, the filter,
//! and moves within and between the two lists.

use super::*;

impl qobject::ModListModel {
    pub(crate) fn set_note(mut self: Pin<&mut Self>, mod_id: i32, tags: &QString, note: &QString) {
        let Ok((library, _)) = crate::library() else {
            return;
        };
        let Ok(installed) = usize::try_from(mod_id) else {
            return;
        };
        let Some(key) = library.installed.get(installed).and_then(|m| m.key()) else {
            return;
        };
        let tags = notes::tags_in(&tags.to_string());
        let mut all = crate::NOTES.write().unwrap_or_else(|e| e.into_inner());
        let mut kept = all.get(&key).cloned().unwrap_or_default();
        kept.tags = tags.clone();
        kept.text = note.to_string().trim().to_string();
        all.set(&key, kept);
        if let Err(problem) = all.write(&crate::notes_file()) {
            eprintln!("{problem}");
        }
        drop(all);
        self.as_mut()
            .reset_with(|list| list.set_tags(installed, &tags));
    }

    pub(crate) fn set_version_ok(self: Pin<&mut Self>, mod_id: i32, ok: bool) {
        change_note(mod_id, |note| note.version_ok = ok);
        self.recheck();
    }

    pub(crate) fn set_category(self: Pin<&mut Self>, mod_id: i32, label: &QString) {
        let label = label.to_string();
        let known = Category::from_label(&label).map_or("", Category::label);
        change_note(mod_id, |note| note.category = known.to_string());
    }

    pub(crate) fn set_sub(self: Pin<&mut Self>, mod_id: i32, name: &QString) {
        let name = name.to_string().trim().to_string();
        change_note(mod_id, |note| note.set_group(&name));
    }

    fn sub_on(&self, row: i32) -> Option<usize> {
        match self.list.rows().get(usize::try_from(row).ok()?)? {
            Row::Sub { first, .. } => Some(*first),
            _ => None,
        }
    }

    pub(crate) fn toggle_sub(mut self: Pin<&mut Self>, row: i32) {
        if let Some(first) = self.sub_on(row) {
            self.as_mut().reset_with(|list| list.toggle_sub(first));
        }
    }

    pub(crate) fn remove_sub(self: Pin<&mut Self>, row: i32) {
        let Some(first) = self.sub_on(row) else {
            return;
        };
        let mods = self.list.mods();
        let tag = mods[first].sub.clone();
        let members = self.list.in_sub_of(first).into_iter();
        let ids: Vec<usize> = members.filter_map(|at| mods[at].installed).collect();
        for id in ids {
            change_note(id as i32, |note| note.remove_tag(&tag));
        }
    }

    pub(crate) fn subs_beside(&self, mod_id: i32) -> QString {
        let listed = |m: &&Mod| m.installed == usize::try_from(mod_id).ok();
        let category = self.list.mods().iter().find(listed).map(|m| m.category);
        let subs = category.map(|c| self.list.subs(c)).unwrap_or_default();
        QString::from(&subs.join("\n"))
    }

    pub(crate) fn show_by(mut self: Pin<&mut Self>, column: &QString, descending: bool) {
        use rimstack_core::list::SortKey;
        let key = match column.to_string().as_str() {
            "name" => Some(SortKey::Name),
            "author" => Some(SortKey::Author),
            "version" => Some(SortKey::Version),
            "load" => Some(SortKey::Load),
            _ => None,
        };
        self.as_mut()
            .reset_with(|list| list.show_by(key.map(|key| (key, descending))));
    }

    pub(crate) fn begin_batch(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().batching = true;
    }

    pub(crate) fn end_batch(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().batching = false;
        self.as_mut().reset_with(|_| ());
    }

    pub(crate) fn recheck(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().checked_order = Vec::new();
        self.as_mut().reset_with(|_| ());
    }

    pub(crate) fn add_rule(self: Pin<&mut Self>, first: i32, then: i32) {
        self.change_rule(first, then, true);
    }

    pub(crate) fn remove_rule(self: Pin<&mut Self>, first: i32, then: i32) {
        self.change_rule(first, then, false);
    }

    pub(crate) fn change_rule(mut self: Pin<&mut Self>, first: i32, then: i32, add: bool) {
        let Ok((library, _)) = crate::library() else {
            return;
        };
        let key = |id: i32| library.installed.get(usize::try_from(id).ok()?)?.key();
        let (Some(first), Some(then)) = (key(first), key(then)) else {
            return;
        };
        let mut yours = crate::USER_RULES.write().unwrap_or_else(|e| e.into_inner());
        let changed = match add {
            true => yours.add(&first, &then),
            false => yours.remove(&first, &then),
        };
        if !changed {
            return;
        }
        if let Err(problem) = yours.write(&crate::user_rules_file()) {
            eprintln!("{problem}");
        }
        drop(yours);
        // The problem markers do not depend on rules; the details do.
        let next = self.rules_version + 1;
        self.as_mut().set_rules_version(next);
    }

    pub(crate) fn unsaved_kept(&self) -> QString {
        QString::from(&crate::session().changes().kept())
    }

    pub(crate) fn session_changed(mut self: Pin<&mut Self>) {
        let (dirty, undo, redo, unsaved) = {
            let session = crate::session();
            let unsaved = session.changes().in_words();
            (
                session.is_dirty(),
                session.can_undo(),
                session.can_redo(),
                unsaved,
            )
        };
        self.as_mut().set_unsaved(QString::from(&unsaved));
        self.as_mut().set_dirty(dirty);
        self.as_mut().set_can_undo(undo);
        self.as_mut().set_can_redo(redo);
    }

    /// The mod at `from` to the top or bottom of its category's stretch.
    pub(crate) fn move_to_edge(mut self: Pin<&mut Self>, from: i32, top: bool) {
        let Some(at) = usize::try_from(from)
            .ok()
            .filter(|at| *at < self.list.mods().len())
        else {
            return;
        };
        let to = rimstack_core::list::edge_of_run(self.list.mods(), at, top);
        self.as_mut().move_mod(from, to as i32);
    }

    /// Back to the list as it is on disk, as one step that Undo takes back.
    pub(crate) fn discard_changes(mut self: Pin<&mut Self>) {
        let saved = crate::session().saved().to_vec();
        crate::session().record(saved);
        self.as_mut().sync();
    }

    pub(crate) fn undo(mut self: Pin<&mut Self>) {
        if crate::session().undo().is_some() {
            self.as_mut().sync();
        }
    }

    pub(crate) fn redo(mut self: Pin<&mut Self>) {
        if crate::session().redo().is_some() {
            self.as_mut().sync();
        }
    }

    pub(crate) fn use_list(mut self: Pin<&mut Self>, ids: &QString) {
        let Ok(ids) = serde_json::from_str::<Vec<String>>(&ids.to_string()) else {
            return;
        };
        crate::session().record(ids);
        self.as_mut().sync();
    }

    pub(crate) fn sync(mut self: Pin<&mut Self>) {
        let (true, Ok((library, _))) = (self.installed, crate::library()) else {
            return;
        };
        let now = arranged(library);
        if self.checked {
            if self.preview.is_some() {
                self.as_mut().end_preview();
            }
            let (mods, _) = sorted_by_category(library, now.active);
            let grouped = self.wants_headings();
            self.as_mut().reset_with(|list| {
                list.set_grouped(grouped);
                list.set_mods(mods);
            });
            self.as_mut().session_changed();
        } else {
            let by_name = self.by_name;
            self.as_mut().reset_with(|list| {
                list.set_mods(now.inactive);
                list.arrange(by_name);
            });
        }
    }

    pub(crate) fn save(mut self: Pin<&mut Self>) -> QString {
        let Some(path) = crate::PATHS.mods_config() else {
            return QString::from(&say!("RimWorld's settings folder was not found."));
        };
        // An id is empty for a folder that is not a readable mod.
        let ids: Vec<String> = crate::session()
            .current()
            .iter()
            .filter(|id| !id.is_empty())
            .cloned()
            .collect();
        if let Err(problem) = modsconfig::write(&path, &ids) {
            return QString::from(&problem);
        }
        crate::session().mark_saved();
        self.as_mut().session_changed();
        QString::default()
    }

    pub(crate) fn toggle_category(mut self: Pin<&mut Self>, category: i32) {
        let Some(category) = Category::ALL.get(category as usize).copied() else {
            return;
        };
        // Qt must hear about a row change before it happens, so find the range first.
        let headers: Vec<(usize, usize, bool)> = self
            .list
            .rows()
            .iter()
            .enumerate()
            .filter_map(|(row, r)| match r {
                Row::Header {
                    category: c,
                    count,
                    collapsed,
                } if *c == category => Some((row, *count, *collapsed)),
                _ => None,
            })
            .collect();

        let parent = QModelIndex::default();
        match headers.as_slice() {
            [(row, count, was_collapsed)] if *count > 0 => {
                let first = (*row + 1) as i32;
                let last = (*row + *count) as i32;
                unsafe {
                    if *was_collapsed {
                        self.as_mut().begin_insert_rows(&parent, first, last);
                    } else {
                        self.as_mut().begin_remove_rows(&parent, first, last);
                    }
                }
                self.as_mut().rust_mut().list.toggle(category);
                unsafe {
                    if *was_collapsed {
                        self.as_mut().end_insert_rows();
                    } else {
                        self.as_mut().end_remove_rows();
                    }
                }
                let header = self.index(*row as i32, 0, &parent);
                self.as_mut()
                    .data_changed(&header, &header, &QVector::default());
            }
            _ => {
                self.as_mut().reset_with(|list| list.toggle(category));
            }
        }
    }

    pub(crate) fn set_filter(mut self: Pin<&mut Self>, text: &QString) {
        let text = text.to_string();
        self.as_mut().reset_with(|list| list.set_filter(&text));
    }

    /// A move can split or merge category groups, so every row may change.
    pub(crate) fn move_mod(mut self: Pin<&mut Self>, from: i32, to: i32) {
        if from < 0 || to < 0 {
            return;
        }
        self.as_mut().end_preview();
        self.as_mut()
            .reset_with(|list| list.move_mod(from as usize, to as usize));
    }

    /// Remove the mod at `index` from another model. QML hands the other model
    /// over as a raw pointer; the bridge cannot pass a QObject by reference.
    pub(crate) fn take_out(self: Pin<&mut Self>, other: *mut Self, index: i32) -> Option<Mod> {
        let own: *const Self = &*self;
        if index < 0 || other.is_null() || std::ptr::eq(other, own) {
            return None;
        }
        // SAFETY: non-null, not `self`, and QML keeps the object alive for the call.
        let mut other = unsafe { Pin::new_unchecked(&mut *other) };
        other.as_mut().end_preview();
        other.reset_with(|list| list.remove(index as usize))
    }

    pub(crate) fn take_from(mut self: Pin<&mut Self>, other: *mut Self, index: i32) {
        let Some(m) = self.as_mut().take_out(other, index) else {
            return;
        };
        self.as_mut().end_preview();
        let rules_apply = self.checked;
        // The inactive mods keep the order chosen for them.
        let rearrange = (!self.checked && self.installed).then_some(self.by_name);
        self.as_mut().reset_with(|list| {
            let at = list.insert_in_category(m);
            if rules_apply {
                settle(list, at);
            }
            if let Some(by_name) = rearrange {
                list.arrange(by_name);
            }
        });
    }

    pub(crate) fn take_from_at(mut self: Pin<&mut Self>, other: *mut Self, index: i32, to: i32) {
        if to < 0 {
            return;
        }
        let Some(m) = self.as_mut().take_out(other, index) else {
            return;
        };
        self.as_mut().end_preview();
        self.as_mut().reset_with(|list| {
            let at = list.insert_in_category(m);
            let last = list.mods().len() - 1;
            list.move_mod(at, (to as usize).min(last));
        });
    }
}
