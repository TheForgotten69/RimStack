//! Every rule known about a load order, and what follows from them:
//! where an activated mod goes, and the order sorted by category.

use crate::community::Community;
use crate::infer::Inferred;
use crate::library::Library;
use crate::list::{self, Category, Mod};
use crate::rules::{self, Kind, Rule};
use crate::sort;
use crate::userrules::UserRules;

/// Where rules come from besides the mods' own `About.xml`.
#[derive(Clone, Copy)]
pub struct Sources<'a> {
    pub yours: &'a UserRules,
    /// The community's, when they are downloaded and switched on.
    pub community: Option<&'a Community>,
    /// What the mods' files say about each other, by position in the list.
    pub from_files: &'a [Inferred],
    /// Whether what patches and defs say counts too. What compiled code
    /// says always does: the game needs that order.
    pub hints: bool,
}

pub fn all_rules(library: &Library, mods: &[Mod], sources: Sources) -> Vec<Rule> {
    let abouts = library.abouts(mods);
    let mut found = rules::among(&abouts, &library.game_version);
    found.extend(sources.yours.rules(&abouts));
    let from_files = sources
        .from_files
        .iter()
        .filter(|d| d.code || sources.hints);
    found.extend(from_files.map(|d| Rule {
        first: d.first,
        then: d.then,
        kind: Kind::Inferred,
    }));
    if let Some(community) = sources.community {
        found.extend(community.rules(&abouts));
    }
    found
}

/// Where the mod at `at` of a list of `count` goes by these rules: after
/// what it must follow and before what must follow it, as near `at` as
/// that leaves room for. This is how a double-click activates a mod.
pub fn place(rules: &[Rule], at: usize, count: usize) -> usize {
    // Positions among the other mods, as if this one were not there yet.
    let among_others = |m: usize| if m > at { m - 1 } else { m };
    let must_follow: Vec<usize> = rules
        .iter()
        .filter(|r| r.then == at)
        .map(|r| among_others(r.first))
        .collect();
    let must_precede: Vec<usize> = rules
        .iter()
        .filter(|r| r.first == at)
        .map(|r| among_others(r.then))
        .collect();
    sort::place(at, count.saturating_sub(1), &must_follow, &must_precede)
}

/// Sorts the mods into their categories by every known rule. Returns the
/// mods in their old order with the category each ends up under (a mod
/// that a library must follow is listed with the libraries), and the sort.
pub fn sorted_by_category(
    library: &Library,
    mut mods: Vec<Mod>,
    sources: Sources,
) -> (Vec<Mod>, sort::Sorted) {
    let found = all_rules(library, &mods, sources);
    // A mod the community marks "load at top" counts as a library.
    if let Some(community) = sources.community {
        let abouts = library.abouts(&mods);
        let top = abouts
            .iter()
            .map(|about| about.is_some_and(|a| community.loads_at_top(&a.package_id)));
        let top: Vec<bool> = top.collect();
        for (m, top) in mods.iter_mut().zip(top) {
            if top && m.category > Category::Libraries {
                m.category = Category::Libraries;
            }
        }
    }
    // Each sub-category of yours is a group of its own inside its
    // category, after the mods in none, in the order first met.
    let mut subs: Vec<&str> = Vec::new();
    for m in mods
        .iter()
        .filter(|m| !m.sub.is_empty() && m.category != Category::Last)
    {
        if !subs.contains(&m.sub.as_str()) {
            subs.push(&m.sub);
        }
    }
    let per_category = subs.len() + 1;
    let rank = |m: &Mod| subs.iter().position(|s| *s == m.sub).map_or(0, |at| at + 1);
    let own: Vec<usize> = mods
        .iter()
        .map(|m| m.category as usize * per_category + rank(m))
        .collect();
    let bottom = Category::Last as usize * per_category;
    let mut sorted = sort::sort_grouped(&own, bottom, &found);
    for group in &mut sorted.groups {
        *group /= per_category;
    }
    for (m, &group) in mods.iter_mut().zip(&sorted.groups) {
        m.category = Category::ALL[group];
    }
    (mods, sorted)
}

/// Whether a load order is close enough to category order for headings to
/// help. The game's own order usually is not until it has been sorted.
pub fn tidy(mods: &[Mod]) -> bool {
    list::runs(mods) <= 2 * Category::ALL.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{build, scan};
    use crate::paths::{ModRoot, Source};
    use std::fs;

    /// Four mods with nothing to say about each other, active in the
    /// order a, b, c, d: a and b hold defs, c and d only textures.
    fn four_mods() -> (tempfile::TempDir, Library) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Mods");
        for (id, folder) in [
            ("a", "Defs"),
            ("b", "Defs"),
            ("c", "Textures"),
            ("d", "Textures"),
        ] {
            fs::create_dir_all(root.join(id).join("About")).unwrap();
            fs::create_dir_all(root.join(id).join(folder)).unwrap();
            let about = format!(
                "<ModMetaData><packageId>t.{id}</packageId><name>{id}</name></ModMetaData>"
            );
            fs::write(root.join(id).join("About/About.xml"), about).unwrap();
        }
        let roots = [ModRoot {
            path: root,
            source: Source::Local,
        }];
        let active: Vec<String> = ["t.a", "t.b", "t.c", "t.d"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        (tmp, build(scan(&roots), &active, "1.6"))
    }

    fn yours(pairs: &[(&str, &str)]) -> UserRules {
        let mut rules = UserRules::default();
        for (first, then) in pairs {
            rules.add(first, then);
        }
        rules
    }

    fn inferred(first: usize, then: usize, code: bool) -> Inferred {
        Inferred {
            first,
            then,
            why: String::new(),
            code,
        }
    }

    #[test]
    fn rules_come_from_you_and_from_files_and_hints_only_when_asked() {
        let (_tmp, library) = four_mods();
        let own = yours(&[("t.b", "t.a")]);
        let from_files = [inferred(2, 3, true), inferred(0, 3, false)];
        let mut sources = Sources {
            yours: &own,
            community: None,
            from_files: &from_files,
            hints: false,
        };
        let pairs = |sources| -> Vec<(usize, usize, Kind)> {
            let found = all_rules(&library, &library.active, sources);
            found
                .into_iter()
                .map(|r| (r.first, r.then, r.kind))
                .collect()
        };
        assert_eq!(
            pairs(sources),
            [(1, 0, Kind::Yours), (2, 3, Kind::Inferred)]
        );
        sources.hints = true;
        assert_eq!(pairs(sources).len(), 3);
    }

    #[test]
    fn a_mod_is_placed_after_what_it_must_follow() {
        // Four mods; the one at 0 must follow the one at 2.
        let rules = [Rule {
            first: 2,
            then: 0,
            kind: Kind::Yours,
        }];
        assert_eq!(place(&rules, 0, 4), 2);
        // With nothing to say it stays where it is.
        assert_eq!(place(&[], 1, 4), 1);
    }

    #[test]
    fn sorting_by_category_follows_the_rules_and_keeps_the_old_order_of_the_rest() {
        let (_tmp, library) = four_mods();
        let own = yours(&[("t.b", "t.a")]);
        let sources = Sources {
            yours: &own,
            community: None,
            from_files: &[],
            hints: false,
        };
        let (mods, sorted) = sorted_by_category(&library, library.active.clone(), sources);
        let names: Vec<&str> = sorted
            .order
            .iter()
            .map(|&i| mods[i].name.as_str())
            .collect();
        assert_eq!(names, ["b", "a", "c", "d"]);
        assert!(tidy(&mods));
    }

    #[test]
    fn sorting_keeps_a_sub_category_of_yours_together_inside_its_category() {
        let (_tmp, library) = four_mods();
        let none = UserRules::default();
        let sources = Sources {
            yours: &none,
            community: None,
            from_files: &[],
            hints: false,
        };
        let mut mods = library.active.clone();
        mods[2].category = Category::Content;
        mods[0].sub = "Genes".into();
        mods[2].sub = "Genes".into();
        let (mods, sorted) = sorted_by_category(&library, mods, sources);
        let names: Vec<&str> = sorted
            .order
            .iter()
            .map(|&i| mods[i].name.as_str())
            .collect();
        assert_eq!(names, ["b", "a", "c", "d"]);
        assert_eq!(mods[2].category, Category::Content);
        assert_eq!(sorted.groups[3], Category::Textures as usize);
    }
}
