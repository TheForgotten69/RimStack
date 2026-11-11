//! Kept as `author,version,load` (the columns) and `author=180,load=70` (widths at 100% text).

use crate::mods::list::SortKey;
use std::collections::BTreeMap;

const NARROWEST: f64 = 48.0;
const WIDEST: f64 = 420.0;

pub fn chosen(kept: &str) -> Vec<&str> {
    kept.split(',')
        .filter(|column| !column.is_empty())
        .collect()
}

pub fn toggled(kept: &str, column: &str) -> String {
    let mut now = chosen(kept);
    match now.iter().position(|c| *c == column) {
        Some(at) => drop(now.remove(at)),
        None => now.push(column),
    }
    now.join(",")
}

pub fn moved(kept: &str, column: &str, to: i32) -> String {
    let mut now: Vec<&str> = chosen(kept).into_iter().filter(|c| *c != column).collect();
    let to = usize::try_from(to).unwrap_or(0).min(now.len());
    now.insert(to, column);
    now.join(",")
}

pub fn widths(kept: &str) -> BTreeMap<&str, u32> {
    let pairs = kept.split(',').filter_map(|pair| pair.split_once('='));
    pairs
        .filter_map(|(column, width)| Some((column, width.parse().ok().filter(|w| *w > 0)?)))
        .collect()
}

pub fn sized(kept: &str, column: &str, width: f64) -> String {
    let mut now = widths(kept);
    now.insert(column, width.clamp(NARROWEST, WIDEST).round() as u32);
    let pairs: Vec<String> = now
        .iter()
        .map(|(column, width)| format!("{column}={width}"))
        .collect();
    pairs.join(",")
}

/// `None` is the list's own order: the load order, or the inactive mods by category.
/// A key starts the way `SortKey::starts_descending` says, turns round when asked again,
/// and goes back to `None` the third time.
pub fn pressed(now: Option<(SortKey, bool)>, asked: Option<SortKey>) -> Option<(SortKey, bool)> {
    let asked = asked?;
    let first = asked.starts_descending();
    match now {
        Some((by, descending)) if by == asked => (descending == first).then_some((asked, !first)),
        _ => Some((asked, first)),
    }
}

/// Kept as "" for the list's own order, "name", or "name down".
pub fn kept(order: Option<(SortKey, bool)>) -> String {
    match order {
        None => String::new(),
        Some((key, false)) => key.key().to_string(),
        Some((key, true)) => format!("{} down", key.key()),
    }
}

pub fn from_kept(kept: &str) -> Option<(SortKey, bool)> {
    let (key, way) = kept.split_once(' ').unwrap_or((kept, ""));
    SortKey::from_key(key).map(|key| (key, way == "down"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_column_is_taken_out_or_added_last_and_moved_among_the_others() {
        assert_eq!(chosen(""), Vec::<&str>::new());
        assert_eq!(toggled("author,version,load", "version"), "author,load");
        assert_eq!(toggled("author,load", "version"), "author,load,version");
        assert_eq!(toggled("", "load"), "load");
        assert_eq!(
            moved("author,version,load", "load", 0),
            "load,author,version"
        );
        assert_eq!(
            moved("author,version,load", "author", 1),
            "version,author,load"
        );
        assert_eq!(moved("author,version", "author", 99), "version,author");
        assert_eq!(moved("author,version", "version", -3), "version,author");
        assert_eq!(moved("author", "load", 1), "author,load");
    }

    #[test]
    fn a_width_is_kept_per_column_within_what_a_column_may_take() {
        let kept = sized("", "author", 180.4);
        assert_eq!(kept, "author=180");
        let kept = sized(&kept, "load", 9999.0);
        assert_eq!(kept, "author=180,load=420");
        assert_eq!(sized(&kept, "author", 3.0), "author=48,load=420");
        assert_eq!(
            widths("author=180,load=0,=,nonsense,version=x"),
            BTreeMap::from([("author", 180)])
        );
    }

    #[test]
    fn a_heading_pressed_three_times_comes_back_to_the_load_order() {
        let name = Some(SortKey::Name);
        let first = pressed(None, name);
        let second = pressed(first, name);
        let third = pressed(second, name);
        assert_eq!(
            [first, second, third],
            [
                Some((SortKey::Name, false)),
                Some((SortKey::Name, true)),
                None
            ]
        );
        let by_name = Some((SortKey::Name, true));
        assert_eq!(
            pressed(by_name, Some(SortKey::Author)),
            Some((SortKey::Author, false))
        );
        assert_eq!(pressed(by_name, None), None);
    }

    #[test]
    fn the_newest_and_largest_come_first_then_last() {
        let updated = Some(SortKey::Updated);
        let first = pressed(Some((SortKey::Name, false)), updated);
        let second = pressed(first, updated);
        assert_eq!(
            [first, second, pressed(second, updated)],
            [
                Some((SortKey::Updated, true)),
                Some((SortKey::Updated, false)),
                None
            ]
        );
    }

    #[test]
    fn an_order_is_kept_as_its_key_and_way_and_read_back() {
        for order in [
            None,
            Some((SortKey::Size, true)),
            Some((SortKey::PackageId, false)),
        ] {
            assert_eq!(from_kept(&kept(order)), order);
        }
        assert_eq!(kept(Some((SortKey::Size, true))), "size down");
        assert_eq!(from_kept("colour down"), None);
    }
}
