//! The load order as the interface shows it: mods in load order, grouped
//! under category headings that collapse, narrowed by a text filter.

use crate::mods::library::ModId;
use std::collections::HashSet;

/// Where the sorter files a mod. Earlier variants load earlier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Loaders,
    Game,
    Libraries,
    Content,
    Patches,
    Textures,
    /// Code that draws windows or tabs of its own.
    Interface,
    /// Code that changes how the game plays and adds nothing to it:
    /// performance mods and tweaks, which nothing in the code tells apart.
    Tweaks,
    /// Mods the community rules keep at the bottom, and what follows them.
    Last,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::Loaders,
        Category::Game,
        Category::Libraries,
        Category::Content,
        Category::Patches,
        Category::Textures,
        Category::Interface,
        Category::Tweaks,
        Category::Last,
    ];

    pub fn from_label(label: &str) -> Option<Self> {
        // "Performance" is what the category was called before it took
        // the tweaks in: notes written then still say so.
        let label = if label == "Performance" {
            Category::Tweaks.label()
        } else {
            label
        };
        Self::ALL.into_iter().find(|c| c.label() == label)
    }

    /// What a category is kept as in the notes: never reworded, as its
    /// label may be.
    pub fn key(self) -> &'static str {
        match self {
            Category::Loaders => "loaders",
            Category::Game => "game",
            Category::Libraries => "libraries",
            Category::Content => "content",
            Category::Patches => "patches",
            Category::Textures => "textures",
            Category::Interface => "interface",
            Category::Tweaks => "tweaks",
            Category::Last => "last",
        }
    }

    /// A category as the notes hold it: by its key, or by a label, which
    /// is what was kept before there were keys.
    pub fn from_kept(kept: &str) -> Option<Self> {
        let by_key = Self::ALL.into_iter().find(|c| c.key() == kept);
        by_key.or_else(|| Self::from_label(kept))
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Loaders => "Loaders",
            Category::Game => "Game and DLC",
            Category::Libraries => "Libraries",
            Category::Content => "Content",
            Category::Patches => "Patches",
            Category::Textures => "Textures",
            Category::Interface => "Interface",
            Category::Tweaks => "Performance & tweaks",
            Category::Last => "Loads last",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mod {
    /// The package id as `ModsConfig.xml` writes it; empty for demo data
    /// and for a folder that cannot be read as a mod.
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub category: Category,
    /// `None` for a mod that is not installed and for demo data.
    pub installed: Option<ModId>,
    pub tags: Vec<String>,
    /// Installed in the game's own `Mods` folder, not from the Workshop.
    pub local: bool,
    pub code: bool,
    /// The methods its code patches, as `type.method` in lower case.
    pub patched: std::sync::Arc<[String]>,
    /// The tag it is grouped under inside its category: its first tag;
    /// empty without tags.
    pub sub: String,
    /// Microseconds it took at the game's last start, where measured.
    pub load_micros: Option<u32>,
}

/// What a list can be shown ordered by, in place of the load order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Author,
    Version,
    Load,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Header {
        category: Category,
        count: usize,
        collapsed: bool,
    },
    /// A sub-category of yours: the mods after it, up to `count`, are in
    /// it. `first` is the load-order position of the first of them.
    /// Collapsed, the row stands alone and its mods are not rows.
    Sub {
        first: usize,
        count: usize,
        collapsed: bool,
    },
    /// `index` is the mod's position in load order.
    Mod { index: usize },
}

/// What a caller must tell its view after `toggle`. Row numbers refer to
/// `rows()`. After `Inserted` or `Removed`, the header at `first - 1` also
/// changed (its `collapsed` flag).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Reset,
    Inserted { first: usize, count: usize },
    Removed { first: usize, count: usize },
}

/// How many headings a list would need: one per stretch of neighbours in
/// the same category. A list sorted by category has one per category.
pub fn runs(mods: &[Mod]) -> usize {
    let changes = mods.windows(2).filter(|w| w[0].category != w[1].category);
    changes.count() + usize::from(!mods.is_empty())
}

pub struct GroupedList {
    mods: Vec<Mod>,
    /// Lower-cased "name author" per mod, so filtering allocates nothing.
    haystacks: Vec<String>,
    collapsed: HashSet<Category>,
    collapsed_subs: HashSet<(Category, String)>,
    filter: Filter,
    /// Per mod, in load order: see `flag`. Empty until `set_flags`, and
    /// again after the order changes.
    flags: Vec<u16>,
    grouped: bool,
    /// Shown in this order, without headings, the largest first if the
    /// flag is set; the load order itself is untouched.
    shown_by: Option<(SortKey, bool)>,
    rows: Vec<Row>,
}

fn haystack(m: &Mod) -> String {
    format!("{} {}", m.name, m.author).to_lowercase()
}

/// What the filter box asks for: plain words, which must all appear in a
/// mod's name or author, and typed terms.
///
/// - `author:name` the author contains this
/// - `tag:label` one of your tags starts with this
/// - `is:local` installed in the game's `Mods` folder
/// - `has:code` ships compiled code
/// - `patches:pawn.tick` its code patches a method whose `Type.Method`
///   contains this
///
/// - `is:error`, `is:warning`, `is:conflict`, `is:updated` what `set_flags`
///   says about the mod
///
/// A typed term RimStack does not know is treated as a plain word.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Filter {
    words: Vec<String>,
    authors: Vec<String>,
    tags: Vec<String>,
    local: bool,
    code: bool,
    /// Texts of which each must be in a method the mod patches.
    patches: Vec<String>,
    /// Flags a mod must all have; see `flag`.
    flags: u16,
}

/// What can be known about a mod besides its own facts, for `is:` terms.
pub mod flag {
    pub const ERROR: u16 = 1;
    pub const WARNING: u16 = 2;
    /// Shares textures with another active mod.
    pub const CONFLICT: u16 = 4;
    /// The Workshop has a newer version than the one on disk.
    pub const UPDATED: u16 = 8;
    /// Active, and not among the mods of the newest save.
    pub const NOT_IN_SAVE: u16 = 16;
    /// Updated by Steam in the last week.
    pub const RECENT: u16 = 32;
    /// Installed more than once: another folder has the same package id.
    pub const DUPLICATE: u16 = 64;
    /// Installed since the game was last started from RimStack.
    pub const NEW: u16 = 128;
    /// Moved or activated since the list was saved.
    pub const CHANGED: u16 = 256;
}

impl Filter {
    fn parse(text: &str) -> Self {
        let mut filter = Self::default();
        for term in text.to_lowercase().split_whitespace() {
            match term.split_once(':') {
                Some(("author", name)) if !name.is_empty() => filter.authors.push(name.to_string()),
                Some(("tag", label)) if !label.is_empty() => filter.tags.push(label.to_string()),
                Some(("is", "local")) => filter.local = true,
                Some(("has", "code")) => filter.code = true,
                Some(("patches", method)) if !method.is_empty() => {
                    filter.patches.push(method.to_string())
                }
                Some(("is", "error")) => filter.flags |= flag::ERROR,
                Some(("is", "warning")) => filter.flags |= flag::WARNING,
                Some(("is", "conflict")) => filter.flags |= flag::CONFLICT,
                Some(("is", "updated")) => filter.flags |= flag::UPDATED,
                Some(("is", "notinsave")) => filter.flags |= flag::NOT_IN_SAVE,
                Some(("is", "recent")) => filter.flags |= flag::RECENT,
                Some(("is", "duplicate")) => filter.flags |= flag::DUPLICATE,
                Some(("is", "new")) => filter.flags |= flag::NEW,
                Some(("is", "changed")) => filter.flags |= flag::CHANGED,
                _ => filter.words.push(term.to_string()),
            }
        }
        filter
    }

    fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    fn admits(&self, m: &Mod, haystack: &str, flags: u16) -> bool {
        if flags & self.flags != self.flags {
            return false;
        }
        let author = || m.author.to_lowercase();
        let tagged = |wanted: &String| m.tags.iter().any(|t| t.to_lowercase().starts_with(wanted));
        self.words.iter().all(|w| haystack.contains(w))
            && self.authors.iter().all(|a| author().contains(a))
            && self.tags.iter().all(tagged)
            && (!self.local || m.local)
            && (!self.code || m.code)
            && self
                .patches
                .iter()
                .all(|p| m.patched.iter().any(|at| at.contains(p)))
    }
}

impl GroupedList {
    /// `mods` must already be in load order.
    pub fn new(mods: Vec<Mod>) -> Self {
        let haystacks = mods.iter().map(haystack).collect();
        let mut list = Self {
            mods,
            haystacks,
            collapsed: HashSet::new(),
            collapsed_subs: HashSet::new(),
            filter: Filter::default(),
            flags: Vec::new(),
            grouped: true,
            shown_by: None,
            rows: Vec::new(),
        };
        list.rebuild();
        list
    }

    /// A list with no headings, for an order that is not sorted by category.
    pub fn flat(mods: Vec<Mod>) -> Self {
        let mut list = Self::new(mods);
        list.grouped = false;
        list.rebuild();
        list
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn mods(&self) -> &[Mod] {
        &self.mods
    }

    /// Says, per mod in load order, what `is:` filter terms can ask
    /// about. Changing the order forgets it.
    pub fn set_flags(&mut self, flags: Vec<u16>) {
        self.flags = flags;
        if self.filter.flags != 0 {
            self.rebuild();
        }
    }

    /// Orders the list by name with no headings, or by category then
    /// name under headings. For a list whose order means nothing, like
    /// the inactive mods.
    pub fn arrange(&mut self, by_name: bool) {
        let mut mods = std::mem::take(&mut self.mods);
        match by_name {
            true => mods.sort_by_cached_key(|m| m.name.to_lowercase()),
            false => mods.sort_by_cached_key(|m| (m.category, m.name.to_lowercase())),
        }
        self.grouped = !by_name;
        self.set_mods(mods);
    }

    /// Gives the mod new tags. Returns whether the list holds that mod.
    pub fn set_tags(&mut self, installed: ModId, tags: &[String]) -> bool {
        let Some(m) = self
            .mods
            .iter_mut()
            .find(|m| m.installed == Some(installed))
        else {
            return false;
        };
        m.tags = tags.to_vec();
        m.sub = tags.first().cloned().unwrap_or_default();
        self.rebuild();
        true
    }

    /// Folds the sub-category of the mod at `first` away, or shows it again.
    pub fn toggle_sub(&mut self, first: usize) {
        let Some(m) = self.mods.get(first).filter(|m| !m.sub.is_empty()) else {
            return;
        };
        let key = (m.category, m.sub.clone());
        if !self.collapsed_subs.remove(&key) {
            self.collapsed_subs.insert(key);
        }
        self.rebuild();
    }

    /// The load-order positions of every mod in the same category and
    /// sub-category as the mod at `first`.
    pub fn in_sub_of(&self, first: usize) -> Vec<usize> {
        let Some(of) = self.mods.get(first).filter(|m| !m.sub.is_empty()) else {
            return Vec::new();
        };
        let same = |m: &Mod| m.category == of.category && m.sub == of.sub;
        let each = self.mods.iter().enumerate();
        each.filter(|(_, m)| same(m)).map(|(at, _)| at).collect()
    }

    pub fn subs(&self, category: Category) -> Vec<String> {
        let in_it = self.mods.iter().filter(|m| m.category == category);
        let names: std::collections::BTreeSet<&str> = in_it.map(|m| m.sub.as_str()).collect();
        names
            .into_iter()
            .filter(|n| !n.is_empty())
            .map(String::from)
            .collect()
    }

    /// Shows the mods ordered by a column, or with `None` in load order
    /// again. Positions in `Row::Mod` stay load-order positions.
    pub fn show_by(&mut self, order: Option<(SortKey, bool)>) {
        self.shown_by = order;
        self.rebuild();
    }

    pub fn is_grouped(&self) -> bool {
        self.grouped
    }

    pub fn set_grouped(&mut self, grouped: bool) {
        self.grouped = grouped;
        self.rebuild();
    }

    /// Replace the whole load order, keeping the filter and what is collapsed.
    pub fn set_mods(&mut self, mods: Vec<Mod>) {
        self.haystacks = mods.iter().map(haystack).collect();
        self.mods = mods;
        self.flags.clear();
        self.rebuild();
    }

    /// How many mods the filter lets through, folded categories included.
    pub fn matching(&self) -> usize {
        (0..self.mods.len()).filter(|&i| self.matches(i)).count()
    }

    pub fn set_filter(&mut self, text: &str) {
        self.filter = Filter::parse(text);
        self.rebuild();
    }

    /// What `toggle` would change, before it does: Qt must be told which
    /// rows go or come before they do.
    pub fn toggling(&self, category: Category) -> Change {
        // With sub-category rows a category is not one row per mod.
        if self.mods.iter().any(|m| !m.sub.is_empty()) {
            return Change::Reset;
        }
        let mut headers = self.rows.iter().enumerate().filter_map(|(row, r)| match r {
            Row::Header {
                category: c, count, ..
            } if *c == category => Some((row, *count)),
            _ => None,
        });
        match (headers.next(), headers.next()) {
            (Some((row, count)), None) if count > 0 => {
                let first = row + 1;
                match self.collapsed.contains(&category) {
                    true => Change::Inserted { first, count },
                    false => Change::Removed { first, count },
                }
            }
            // Not shown, or split into several runs by a manual order.
            _ => Change::Reset,
        }
    }

    pub fn toggle(&mut self, category: Category) -> Change {
        let change = self.toggling(category);
        if !self.collapsed.remove(&category) {
            self.collapsed.insert(category);
        }
        self.rebuild();
        change
    }

    /// Move a mod within the load order. Out-of-range indices are ignored.
    pub fn move_mod(&mut self, from: usize, to: usize) {
        if from >= self.mods.len() || to >= self.mods.len() || from == to {
            return;
        }
        let m = self.mods.remove(from);
        let h = self.haystacks.remove(from);
        self.mods.insert(to, m);
        self.haystacks.insert(to, h);
        self.flags.clear();
        self.rebuild();
    }

    /// Add a mod after the last mod of its own or an earlier category; at the
    /// end of a flat list. Returns its position in load order.
    pub fn insert_in_category(&mut self, m: Mod) -> usize {
        let at = if self.grouped {
            self.mods
                .iter()
                .rposition(|x| x.category <= m.category)
                .map_or(0, |p| p + 1)
        } else {
            self.mods.len()
        };
        self.haystacks.insert(at, haystack(&m));
        self.mods.insert(at, m);
        self.flags.clear();
        self.rebuild();
        at
    }

    pub fn remove(&mut self, index: usize) -> Option<Mod> {
        if index >= self.mods.len() {
            return None;
        }
        self.haystacks.remove(index);
        let m = self.mods.remove(index);
        self.flags.clear();
        self.rebuild();
        Some(m)
    }

    fn matches(&self, index: usize) -> bool {
        let flags = self.flags.get(index).copied().unwrap_or(0);
        self.filter.is_empty()
            || self
                .filter
                .admits(&self.mods[index], &self.haystacks[index], flags)
    }

    fn rebuild(&mut self) {
        if let Some((key, descending)) = self.shown_by {
            let mut shown: Vec<usize> = (0..self.mods.len()).filter(|&i| self.matches(i)).collect();
            let text = |i: usize| match key {
                SortKey::Author => self.mods[i].author.to_lowercase(),
                SortKey::Version => self.mods[i].version.to_lowercase(),
                _ => self.mods[i].name.to_lowercase(),
            };
            match key {
                SortKey::Load => {
                    shown.sort_by_key(|&i| self.mods[i].load_micros);
                }
                _ => shown.sort_by_cached_key(|&i| text(i)),
            }
            if descending {
                shown.reverse();
            }
            self.rows = shown.into_iter().map(|index| Row::Mod { index }).collect();
            return;
        }
        if !self.grouped {
            self.rows = (0..self.mods.len())
                .filter(|&i| self.matches(i))
                .map(|index| Row::Mod { index })
                .collect();
            return;
        }
        let mut rows = Vec::with_capacity(self.mods.len() + Category::ALL.len());
        let mut start = 0;
        while start < self.mods.len() {
            let category = self.mods[start].category;
            let mut end = start;
            while end < self.mods.len() && self.mods[end].category == category {
                end += 1;
            }
            let visible: Vec<usize> = (start..end).filter(|&i| self.matches(i)).collect();
            if !visible.is_empty() {
                let collapsed = self.collapsed.contains(&category);
                rows.push(Row::Header {
                    category,
                    count: visible.len(),
                    collapsed,
                });
                if !collapsed {
                    self.push_with_subs(&mut rows, &visible);
                }
            }
            start = end;
        }
        self.rows = rows;
    }

    /// The mods of one category run, each stretch of neighbours in the
    /// same sub-category under a row naming it.
    fn push_with_subs(&self, rows: &mut Vec<Row>, visible: &[usize]) {
        let mut at = 0;
        while at < visible.len() {
            let sub = &self.mods[visible[at]].sub;
            let same = visible[at..]
                .iter()
                .take_while(|&&i| self.mods[i].sub == *sub);
            let count = same.count();
            let key = (self.mods[visible[at]].category, sub.clone());
            let collapsed = !sub.is_empty() && self.collapsed_subs.contains(&key);
            if !sub.is_empty() {
                rows.push(Row::Sub {
                    first: visible[at],
                    count,
                    collapsed,
                });
            }
            if !collapsed {
                let stretch = &visible[at..at + count];
                rows.extend(stretch.iter().map(|&index| Row::Mod { index }));
            }
            at += count;
        }
    }
}

/// The groups (a category's mods under one first tag) that are in more
/// than one place in this order, by name, each once: a rule keeps some
/// of their mods apart from the rest.
pub fn split_groups<'a>(mods: impl IntoIterator<Item = &'a Mod>) -> Vec<String> {
    let mut closed: HashSet<(Category, &str)> = HashSet::new();
    let mut split: Vec<String> = Vec::new();
    let mut last: Option<(Category, &str)> = None;
    for m in mods {
        let now = (!m.sub.is_empty()).then_some((m.category, m.sub.as_str()));
        if now == last {
            continue;
        }
        if let Some(left) = last {
            closed.insert(left);
        }
        if let Some(entered) = now {
            if closed.contains(&entered) && !split.iter().any(|name| name == entered.1) {
                split.push(entered.1.to_string());
            }
        }
        last = now;
    }
    split
}

/// Per mod, whether another mod of the list goes by the same name,
/// whatever the case of its letters.
pub fn twins(mods: &[Mod]) -> Vec<bool> {
    let mut named: std::collections::HashMap<String, usize> = Default::default();
    for m in mods {
        *named.entry(m.name.to_ascii_lowercase()).or_default() += 1;
    }
    let shared = |m: &Mod| named[&m.name.to_ascii_lowercase()] > 1;
    mods.iter().map(shared).collect()
}

/// Where the stretch of neighbours in the same category as the mod at
/// `at` begins, or ends.
pub fn edge_of_run(mods: &[Mod], at: usize, top: bool) -> usize {
    let same = |other: &Mod| other.category == mods[at].category;
    match top {
        true => at - mods[..at].iter().rev().take_while(|m| same(m)).count(),
        false => at + mods[at + 1..].iter().take_while(|m| same(m)).count(),
    }
}

mod navigate;
#[cfg(test)]
mod tests;
