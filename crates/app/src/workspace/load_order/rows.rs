//! The view reads these as published, never the state behind them.

use super::{LoadOrder, Side, side_index};
use crate::App;
use rimstack_core::base::sentence::Sentence;
use rimstack_core::base::{elapsed, sizes};
use rimstack_core::mods::describe::{Fix, Trouble};
use rimstack_core::mods::library::{Installed, Library, ModId};
use rimstack_core::mods::list::{Category, Mod, Row, SortKey};
use rimstack_core::rimworld::paths::Source;
use rimstack_core::say;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RowKey {
    Header(Category, usize),
    Sub(Category, String, usize),
    /// By its folder, the same over a rescan, which gives every mod a new `ModId`.
    Installed(std::path::PathBuf, usize),
    /// By its package id.
    Missing(String, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowView {
    pub key: RowKey,
    pub is_header: bool,
    pub is_sub: bool,
    pub again: bool,
    pub name: Sentence,
    pub author: String,
    pub version: String,
    /// 0 for a heading.
    pub number: i32,
    pub count: i32,
    pub collapsed: bool,
    pub category: i32,
    /// -1 for a heading.
    pub mod_index: i32,
    /// -1 for a heading or a mod that is not installed.
    pub mod_id: i32,
    /// Where Sort moves the mod from, numbered from 1; 0 when it stays.
    pub moved_from: i32,
    /// 0 nothing, 1 a warning, 2 an error.
    pub trouble: i32,
    pub trouble_text: Sentence,
    pub tags: String,
    pub local: bool,
    pub picture: String,
    pub load_time: String,
    /// Against the slowest mod, from 0 to 1.
    pub load_part: f64,
    pub version_ignored: bool,
    pub outdated: bool,
    /// Empty when not outdated.
    pub made_for: String,
    pub fresh: bool,
    /// 0 Workshop, 1 local, 2 a Git checkout, 3 the game's own.
    pub source: i32,
    pub why_moved: Sentence,
    pub group_size: i32,
    pub twin: bool,
    /// The value the list is ordered by; empty by name or by the list's own order.
    pub ordered_by: Sentence,
    /// Empty in the inactive list and without a time from Steam.
    pub updated_ago: Sentence,
    /// Empty in the inactive list and where the system does not record when a folder was made.
    pub downloaded_ago: Sentence,
    /// Empty in the inactive list and for a mod not saved in the load order yet.
    pub activated_ago: Sentence,
    /// Empty in the inactive list and without a size from Steam.
    pub size_text: Sentence,
    pub accessible_name: Sentence,
    /// In seconds; 0 when not measured.
    pub load_seconds: f64,
    /// The version, then where the mod comes from.
    pub version_line: Sentence,
}

impl RowView {
    fn heading_name(&self) -> Sentence {
        Sentence::listed(vec![
            self.name.clone(),
            Sentence::Number(i64::from(self.count)),
        ])
    }

    fn heading(key: RowKey, name: &str, category: Category) -> Self {
        Self {
            key,
            is_header: true,
            is_sub: false,
            again: false,
            name: Sentence::Plain(name.to_string()),
            author: String::new(),
            version: String::new(),
            number: 0,
            count: 0,
            collapsed: false,
            category: category as i32,
            mod_index: -1,
            mod_id: -1,
            moved_from: 0,
            trouble: 0,
            trouble_text: Sentence::default(),
            tags: String::new(),
            local: false,
            picture: String::new(),
            load_time: String::new(),
            load_part: 0.0,
            version_ignored: false,
            outdated: false,
            made_for: String::new(),
            fresh: false,
            source: 0,
            why_moved: Sentence::default(),
            group_size: 0,
            twin: false,
            ordered_by: Sentence::default(),
            updated_ago: Sentence::default(),
            downloaded_ago: Sentence::default(),
            activated_ago: Sentence::default(),
            size_text: Sentence::default(),
            accessible_name: Sentence::Plain(name.to_string()),
            load_seconds: 0.0,
            version_line: Sentence::default(),
        }
    }
}

struct Around<'a> {
    library: Option<&'a Library>,
    fresh: std::collections::HashSet<ModId>,
    now: u64,
}

/// 2 for an error, 1 for a warning, 0 for none; the game version trusted aside is none.
pub(super) fn trouble_level(own: &[Trouble]) -> i32 {
    let shown = own.iter().filter(|t| t.fix != Fix::Trust);
    shown
        .map(|t| if t.error { 2 } else { 1 })
        .max()
        .unwrap_or(0)
}

pub(super) fn load_time_text(m: &Mod) -> String {
    let took = m.load_micros.filter(|micros| *micros >= 1000);
    took.map(|micros| rimstack_core::rimworld::startup::seconds(f64::from(micros) / 1000.0))
        .unwrap_or_default()
}

fn ago(at: Option<u64>, now: u64) -> Sentence {
    let ago = |at| elapsed::ago(now.saturating_sub(at));
    at.map(ago).unwrap_or_default()
}

fn source_word(source: i32) -> Sentence {
    match source {
        2 => say!("Git"),
        1 => say!("local"),
        _ => say!("Workshop"),
    }
}

fn size_text(m: &Mod) -> Sentence {
    m.size.map(sizes::of_bytes).unwrap_or_default()
}

impl LoadOrder {
    fn ordered_by(&self, m: &Mod, side: Side, now: u64) -> Sentence {
        let Some((key, _)) = self.order_of(side) else {
            return Sentence::default();
        };
        let plain = |text: &str| Sentence::Plain(text.to_string());
        match key {
            SortKey::Updated => ago(m.updated, now),
            SortKey::Downloaded => ago(m.downloaded, now),
            SortKey::Activated => ago(m.activated, now),
            SortKey::Size => size_text(m),
            SortKey::PackageId => plain(&m.id),
            SortKey::Author => plain(&m.author),
            SortKey::Version => plain(&m.version),
            SortKey::Name | SortKey::Load => Sentence::default(),
        }
    }
}

fn key_of(installed: Option<&Installed>, m: &Mod, nth: &mut impl FnMut(RowKey) -> usize) -> RowKey {
    match installed {
        Some(i) => RowKey::Installed(i.path.clone(), nth(RowKey::Installed(i.path.clone(), 0))),
        None => RowKey::Missing(m.id.clone(), nth(RowKey::Missing(m.id.clone(), 0))),
    }
}

fn source_of(installed: Option<&Installed>) -> (i32, String) {
    let source = installed.map_or(0, |i| match i.source {
        Source::Official => 3,
        Source::Local if i.git => 2,
        Source::Local => 1,
        Source::Workshop => 0,
    });
    let picture = installed.and_then(|i| i.preview.as_ref());
    let picture = picture.map_or(String::new(), |p| p.to_string_lossy().into_owned());
    (source, picture)
}

impl LoadOrder {
    pub fn rows(&self, app: &App, side: Side) -> Vec<RowView> {
        let loaded = self.reading(app);
        let around = Around {
            library: loaded.as_ref().as_ref().ok().map(|(l, _)| l),
            fresh: self.fresh(app, side).into_iter().collect(),
            now: crate::workspace::now_secs(),
        };
        let list = self.list(side);
        let mods = list.mods();
        let mut seen: HashMap<RowKey, usize> = HashMap::new();
        let mut nth = |key: RowKey| -> usize {
            let count = seen.entry(key).or_default();
            *count += 1;
            *count - 1
        };
        let mut categories_shown: Vec<Category> = Vec::new();
        list.rows()
            .iter()
            .map(|row| match *row {
                Row::Header {
                    category,
                    count,
                    collapsed,
                } => {
                    let key = RowKey::Header(category, nth(RowKey::Header(category, 0)));
                    let mut view = RowView::heading(key, category.label(), category);
                    view.again = categories_shown.contains(&category);
                    categories_shown.push(category);
                    view.count = count as i32;
                    view.collapsed = collapsed;
                    view.accessible_name = view.heading_name();
                    view
                }
                Row::Sub {
                    first,
                    count,
                    collapsed,
                } => {
                    let of = &mods[first];
                    let same = |m: &Mod| m.category == of.category && m.sub == of.sub;
                    let key = RowKey::Sub(
                        of.category,
                        of.sub.clone(),
                        nth(RowKey::Sub(of.category, of.sub.clone(), 0)),
                    );
                    let mut view = RowView::heading(key, &of.sub, of.category);
                    view.is_sub = true;
                    view.again = mods[..first].iter().any(same);
                    view.count = count as i32;
                    view.collapsed = collapsed;
                    view.group_size = list.in_sub_of(first).len() as i32;
                    view.accessible_name = view.heading_name();
                    view
                }
                Row::Mod { index } => self.mod_row(side, index, &mods[index], &around, &mut nth),
            })
            .collect()
    }

    fn mod_row(
        &self,
        side: Side,
        index: usize,
        m: &Mod,
        around: &Around,
        nth: &mut impl FnMut(RowKey) -> usize,
    ) -> RowView {
        let now = around.now;
        let columns = side == Side::Active;
        let column = |text: fn(&Mod, u64) -> Sentence| match columns {
            true => text(m, now),
            false => Sentence::default(),
        };
        let own: &[Trouble] = match side {
            Side::Active => self
                .troubles
                .get(index)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            Side::Inactive => &[],
        };
        let installed = around.library.zip(m.installed).and_then(|(l, i)| l.get(i));
        let (source, picture) = source_of(installed);
        let took = m.load_micros.filter(|micros| *micros >= 1000);
        let load_part = match (m.load_micros, self.slowest) {
            (Some(own), Some(most)) if most > 0 && side == Side::Active => {
                f64::from(own) / f64::from(most)
            }
            _ => 0.0,
        };
        // A game version is shown as `made_for`.
        let shown = || own.iter().filter(|t| t.fix != Fix::Trust);
        let first = shown().find(|t| t.error).or(shown().next());
        let made_for = self.outdated[side_index(side)]
            .get(index)
            .and_then(Option::as_ref);
        let preview = side == Side::Active;
        let name = match m.installed {
            Some(_) => Sentence::Plain(m.name.clone()),
            None => say!("{} (not installed)", m.name),
        };
        let fresh = m.installed.is_some_and(|i| around.fresh.contains(&i));
        let accessible_name = Sentence::listed(
            std::iter::once(name.clone())
                .chain(first.map(|t| t.short.clone()))
                .chain(fresh.then(|| say!("new")))
                .collect(),
        );
        let version_line = Sentence::listed(
            Some(m.version.as_str())
                .filter(|v| !v.is_empty())
                .map(|v| Sentence::Plain(v.to_string()))
                .into_iter()
                .chain([source_word(source)])
                .collect(),
        );
        RowView {
            key: key_of(installed, m, nth),
            is_header: false,
            is_sub: false,
            again: false,
            name,
            author: m.author.clone(),
            version: m.version.clone(),
            number: index as i32 + 1,
            count: 0,
            collapsed: false,
            category: m.category as i32,
            mod_index: index as i32,
            mod_id: m.installed.map_or(-1, ModId::to_qml),
            moved_from: if preview {
                self.moved_from(index) as i32
            } else {
                0
            },
            trouble: trouble_level(own),
            trouble_text: first.map_or_else(Sentence::default, |t| t.short.clone()),
            tags: m.tags.join(", "),
            local: m.local,
            picture,
            load_time: load_time_text(m),
            load_part,
            version_ignored: preview && self.version_ignored.get(index).copied().unwrap_or(false),
            outdated: made_for.is_some(),
            made_for: made_for.cloned().unwrap_or_default(),
            fresh,
            source,
            why_moved: if preview {
                self.why_moved(index).cloned().unwrap_or_default()
            } else {
                Sentence::default()
            },
            group_size: 0,
            twin: self.twins[side_index(side)]
                .get(index)
                .copied()
                .unwrap_or(false),
            ordered_by: self.ordered_by(m, side, now),
            updated_ago: column(|m, now| ago(m.updated, now)),
            downloaded_ago: column(|m, now| ago(m.downloaded, now)),
            activated_ago: column(|m, now| ago(m.activated, now)),
            size_text: column(|m, _| size_text(m)),
            accessible_name,
            load_seconds: took.map_or(0.0, |micros| f64::from(micros) / 1_000_000.0),
            version_line,
        }
    }
}
