//! Sort as a preview: worked out, shown with where each mod moves from,
//! then kept or discarded.

use super::*;

impl qobject::ModListModel {
    pub(crate) fn preview_sort(mut self: Pin<&mut Self>) {
        // Demo mods are not installed; asking for the library would scan the disk.
        if self.preview.is_some() || self.list.mods().iter().all(|m| m.installed.is_none()) {
            return;
        }
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return;
        };
        let started = std::time::Instant::now();
        let before = self.list.mods().to_vec();
        let (mods, sorted) = sorted_by_category(crate::app(), library, before.clone());
        let moved = sorted.moved();
        rimstack_core::note!("sort_us={}", started.elapsed().as_micros());

        let moves = moved.iter().filter(|m| **m).count();
        let mut note = match moves {
            0 => say!("Already in order."),
            n => say_n!(
                n,
                "Sorting would move {} mod.",
                "Sorting would move {} mods."
            ),
        };
        // Rules come before groups: say which groups they keep apart.
        let apart = rimstack_core::mods::list::split_groups(sorted.order.iter().map(|&i| &mods[i]));
        if !apart.is_empty() {
            note += " ";
            note += &say_n!(
                apart.len(),
                "A rule keeps {} group in more than one place: {}.",
                "A rule keeps {} groups in more than one place: {}.",
                apart.join(", ")
            );
        }
        // A mod kept off the bottom by a stronger rule is expected, not news.
        let dropped = sorted.dropped.iter();
        let each: Vec<String> = dropped
            .filter(|r| !matches!(r.kind, Kind::CommunityBottom | Kind::Inferred))
            .map(|r| describe::rule(r, &mods))
            .collect();
        if !each.is_empty() {
            note += " ";
            note += &say_n!(
                each.len(),
                "Ignored {} rule that contradicts another: {}.",
                "Ignored {} rules that contradict another: {}.",
                each.join("; ")
            );
        }
        self.as_mut().set_sort_note(QString::from(&note));
        if moves == 0 {
            return;
        }
        let found = all_rules(crate::app(), library, &mods);
        let names: Vec<&str> = mods.iter().map(|m| m.name.as_str()).collect();
        let categories: Vec<Category> = mods.iter().map(|m| m.category).collect();
        let why = sorted
            .order
            .iter()
            .zip(&moved)
            .map(|(&old, moved)| {
                moved.then(|| {
                    rimstack_core::mods::sort_reasons::moved(old, &found, &names, &categories)
                })
            })
            .collect();
        let moved_from = sorted
            .order
            .iter()
            .zip(moved)
            .map(|(&old, moved)| if moved { old + 1 } else { 0 })
            .collect();
        let after: Vec<Mod> = sorted.order.iter().map(|&i| mods[i].clone()).collect();
        let grouped_before = self.list.is_grouped();
        self.as_mut().rust_mut().preview = Some(Preview {
            before,
            grouped_before,
            moved_from,
            why,
        });
        let grouped = self.wants_headings();
        self.as_mut().reset_with(|list| {
            list.set_grouped(grouped);
            list.set_mods(after);
        });
        self.as_mut().set_previewing(true);
    }

    pub(crate) fn apply_sort(mut self: Pin<&mut Self>) {
        if self.preview.is_some() {
            // The markers go; the order stays. The preview must be gone
            // before the view re-reads its rows.
            self.as_mut().end_preview();
            self.as_mut().reset_with(|_| ());
        }
    }

    pub(crate) fn discard_sort(mut self: Pin<&mut Self>) {
        if let Some(preview) = self.as_mut().rust_mut().preview.take() {
            self.as_mut().reset_with(|list| {
                list.set_grouped(preview.grouped_before);
                list.set_mods(preview.before);
            });
            self.as_mut().end_preview();
        }
    }

    /// Any edit keeps the previewed order and drops its markers.
    pub(crate) fn end_preview(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().preview = None;
        self.as_mut().set_previewing(false);
        self.as_mut().set_sort_note(QString::default());
    }

    /// Whether the load order should sit under headings.
    /// Under category headings unless one numbered list was asked for.
    pub(crate) fn wants_headings(&self) -> bool {
        self.headings != 0
    }

    pub(crate) fn reshape(mut self: Pin<&mut Self>) {
        if !self.installed {
            return;
        }
        if self.checked {
            let grouped = self.wants_headings();
            if grouped != self.list.is_grouped() {
                self.as_mut().reset_with(|list| list.set_grouped(grouped));
            }
        } else {
            // By name is the one arrangement without headings.
            let by_name = self.by_name;
            self.as_mut().reset_with(|list| list.arrange(by_name));
        }
    }

    /// Files each listed mod again without moving any. The category a
    /// mod is shown under follows from the rules known, and more become
    /// known after the list is first shown: once the mods' files are
    /// read, once the community rules arrive. Without this the headings
    /// would change at the next unrelated refresh.
    pub(crate) fn refile(mut self: Pin<&mut Self>) {
        let applies = self.checked && self.preview.is_none();
        let loaded = crate::app().library();
        let Some((library, _)) = loaded.as_ref().as_ref().ok().filter(|_| applies) else {
            self.as_mut().reset_with(|_| ());
            return;
        };
        let (mods, _) = sorted_by_category(crate::app(), library, self.list.mods().to_vec());
        self.as_mut().reset_with(|list| list.set_mods(mods));
    }

    pub(crate) fn sort_pending(&self) -> i32 {
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return 0;
        };
        if self.preview.is_some() || !self.checked {
            return 0;
        }
        let (_, sorted) = sorted_by_category(crate::app(), library, self.list.mods().to_vec());
        sorted.moved().iter().filter(|moved| **moved).count() as i32
    }

    /// Each mod the sort on show moves: its place now, the mod, its place
    /// before (from 1) and why.
    fn previewed_moves(
        &self,
    ) -> Vec<(usize, &Mod, usize, &rimstack_core::mods::sort_reasons::Why)> {
        let Some(preview) = &self.preview else {
            return Vec::new();
        };
        let moved = self.list.mods().iter().zip(&preview.moved_from).enumerate();
        let reasoned = moved.filter_map(|(now, (m, &from))| {
            let why = preview.why.get(now)?.as_ref()?;
            (from > 0).then_some((now, m, from, why))
        });
        reasoned.collect()
    }

    pub(crate) fn sort_move_count(&self) -> i32 {
        self.previewed_moves().len() as i32
    }

    pub(crate) fn sort_moves(&self) -> QString {
        let reasoned = self.previewed_moves().into_iter();
        let mut groups = rimstack_core::base::grouped::by_key(reasoned, |(_, _, _, why)| why.group);
        groups.sort_by_key(|(_, moves)| std::cmp::Reverse(moves.len()));
        let said: Vec<crate::reports::SortGroup> = groups
            .iter()
            .map(|(name, moves)| crate::reports::SortGroup {
                name,
                moves: moves
                    .iter()
                    .map(|(now, m, from, why)| crate::reports::SortMove {
                        name: m.name.clone(),
                        from: *from,
                        to: now + 1,
                        mod_id: m.installed.map_or(-1, ModId::to_qml),
                        why: why.text.clone(),
                    })
                    .collect(),
            })
            .collect();
        crate::reports::said(&said)
    }

    /// Why Sort moves the mod at this position of the order on show.
    pub(crate) fn why_moved(&self, index: usize) -> &str {
        let why = self
            .preview
            .as_ref()
            .and_then(|p| p.why.get(index)?.as_ref());
        why.map_or("", |w| w.text.as_str())
    }

    pub(crate) fn first_moved(&self) -> i32 {
        let Some(preview) = &self.preview else {
            return -1;
        };
        let row = self.list.rows().iter().position(
            |r| matches!(r, Row::Mod { index } if preview.moved_from.get(*index).is_some_and(|&n| n > 0)),
        );
        row.map_or(-1, |r| r as i32)
    }
}
