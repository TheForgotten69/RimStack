//! A record of each time the game was started from RimStack: when, with
//! which load order, and how many kinds of error its log showed.

use serde_json::{json, Value};
use std::path::Path;

const KEPT: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// Seconds since 1970.
    pub time: u64,
    pub ids: Vec<String>,
    /// Kinds of error in the log afterwards; `None` until a log written
    /// after the launch has been read.
    pub errors: Option<usize>,
}

/// The launches on record, oldest first. No file, or one that cannot be
/// read, is no launches: the record is a convenience, not a source.
pub fn read(path: &Path) -> Vec<Launch> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(Value::Array(items)) = serde_json::from_str(&text) else {
        return Vec::new();
    };
    let launch = |item: &Value| -> Option<Launch> {
        let ids = item.get("ids")?.as_array()?;
        Some(Launch {
            time: item.get("time")?.as_u64()?,
            ids: ids
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            errors: item
                .get("errors")
                .and_then(Value::as_u64)
                .map(|n| n as usize),
        })
    };
    items.iter().filter_map(launch).collect()
}

pub fn write(path: &Path, launches: &[Launch]) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    let from = launches.len().saturating_sub(KEPT);
    let items: Vec<Value> = launches[from..]
        .iter()
        .map(|l| json!({ "time": l.time, "ids": l.ids, "errors": l.errors }))
        .collect();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    std::fs::write(path, Value::Array(items).to_string()).map_err(failed)
}

pub fn record(path: &Path, ids: &[String], now: u64) -> Result<(), String> {
    let mut launches = read(path);
    launches.push(Launch {
        time: now,
        ids: ids.to_vec(),
        errors: None,
    });
    write(path, &launches)
}

/// Gives the newest launch its outcome, if it has none yet and the log
/// was written after it started. Returns whether anything changed.
pub fn note_outcome(launches: &mut [Launch], log_written: u64, errors: usize) -> bool {
    match launches.last_mut() {
        Some(last) if last.errors.is_none() && log_written >= last.time => {
            last.errors = Some(errors);
            true
        }
        _ => false,
    }
}

pub fn last_clean(launches: &[Launch]) -> Option<&Launch> {
    launches.iter().rev().find(|l| l.errors == Some(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn launches_are_recorded_and_read_back_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data").join("launches.json");
        assert!(read(&path).is_empty());
        record(&path, &ids(&["a"]), 10).unwrap();
        record(&path, &ids(&["a", "b"]), 20).unwrap();
        let launches = read(&path);
        assert_eq!(launches.len(), 2);
        assert_eq!(launches[1].ids, ids(&["a", "b"]));
        assert_eq!(launches[1].errors, None);
    }

    #[test]
    fn only_the_newest_launches_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("launches.json");
        for time in 0..KEPT as u64 + 5 {
            record(&path, &ids(&["a"]), time).unwrap();
        }
        let launches = read(&path);
        assert_eq!(launches.len(), KEPT);
        assert_eq!(launches[0].time, 5);
    }

    #[test]
    fn an_outcome_goes_to_the_newest_launch_once_and_only_from_a_later_log() {
        let launch = |time, errors| Launch {
            time,
            ids: Vec::new(),
            errors,
        };
        let mut launches = vec![launch(10, Some(0)), launch(20, None)];
        assert!(!note_outcome(&mut launches, 15, 3));
        assert!(note_outcome(&mut launches, 25, 3));
        assert_eq!(launches[1].errors, Some(3));
        assert!(!note_outcome(&mut launches, 30, 0));
        assert_eq!(last_clean(&launches).unwrap().time, 10);
        assert!(!note_outcome(&mut [], 30, 0));
    }

    #[test]
    fn a_damaged_record_reads_as_no_launches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("launches.json");
        std::fs::write(&path, "{not json").unwrap();
        assert!(read(&path).is_empty());
    }
}
