//! The columns of the load order after the name: which are chosen and
//! in what order ("author,version,load"), how wide each was dragged
//! ("author=180,load=70", at 100% text), and which one the list is
//! shown by. Both lines are kept as written here.

use std::collections::BTreeMap;

/// What a column may be dragged to, at 100% text.
const NARROWEST: f64 = 48.0;
const WIDEST: f64 = 420.0;

pub fn chosen(kept: &str) -> Vec<&str> {
    kept.split(',')
        .filter(|column| !column.is_empty())
        .collect()
}

/// `kept` without the column, or with it at the end.
pub fn toggled(kept: &str, column: &str) -> String {
    let mut now = chosen(kept);
    match now.iter().position(|c| *c == column) {
        Some(at) => drop(now.remove(at)),
        None => now.push(column),
    }
    now.join(",")
}

/// `kept` with the column at place `to` among the others, or at the
/// nearest end.
pub fn moved(kept: &str, column: &str, to: i32) -> String {
    let mut now: Vec<&str> = chosen(kept).into_iter().filter(|c| *c != column).collect();
    let to = usize::try_from(to).unwrap_or(0).min(now.len());
    now.insert(to, column);
    now.join(",")
}

/// A pair that names no width above nothing is not one.
pub fn widths(kept: &str) -> BTreeMap<&str, u32> {
    let pairs = kept.split(',').filter_map(|pair| pair.split_once('='));
    pairs
        .filter_map(|(column, width)| Some((column, width.parse().ok().filter(|w| *w > 0)?)))
        .collect()
}

/// `kept` with the column at this width, held to what a column may take.
pub fn sized(kept: &str, column: &str, width: f64) -> String {
    let mut now = widths(kept);
    now.insert(column, width.clamp(NARROWEST, WIDEST).round() as u32);
    let pairs: Vec<String> = now
        .iter()
        .map(|(column, width)| format!("{column}={width}"))
        .collect();
    pairs.join(",")
}

/// What the list is shown by after its heading `asked` is pressed: the
/// column and whether the largest comes first. A first press shows by
/// the column, a second turns it round, a third goes back to the load
/// order, which is the empty column.
pub fn pressed<'a>(by: &str, descending: bool, asked: &'a str) -> (&'a str, bool) {
    match (by == asked, descending) {
        _ if asked.is_empty() => ("", false),
        (true, true) => ("", false),
        (true, false) => (asked, true),
        (false, _) => (asked, false),
    }
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
        let first = pressed("", false, "name");
        let second = pressed(first.0, first.1, "name");
        let third = pressed(second.0, second.1, "name");
        assert_eq!(
            [first, second, third],
            [("name", false), ("name", true), ("", false)]
        );
        assert_eq!(pressed("name", true, "author"), ("author", false));
        assert_eq!(pressed("name", true, ""), ("", false));
    }
}
