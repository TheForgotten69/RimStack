//! What RimStack changed, to look back on and take back.

use super::*;
use rimstack_core::journal;

fn journal_file() -> PathBuf {
    crate::data_dir().join("journal.json")
}

/// Set while a conversion should stop after the mod it is on.
pub(super) static STOP_TEXTURES: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

impl qobject::Backend {
    pub(super) fn note_change(&self, what: &QString, revert: &QString) {
        let entry = journal::Entry {
            time: now(),
            what: what.to_string(),
            revert: revert.to_string(),
        };
        if let Err(problem) = journal::add(&journal_file(), entry) {
            eprintln!("{problem}");
        }
    }

    /// The journal, newest first, as JSON: `{ time, what, revert }`, the
    /// last empty where the entry can no longer be taken back.
    pub(super) fn journal(&self) -> QString {
        let kept: Vec<String> = lists::all(&snapshots_dir())
            .into_iter()
            .map(|l| l.name)
            .collect();
        let still = |revert: &str| match revert.strip_prefix("list ") {
            Some(name) => kept.iter().any(|k| k == name),
            None => true,
        };
        let rows: Vec<Value> = journal::read(&journal_file())
            .iter()
            .rev()
            .map(|e| {
                let revert = if still(&e.revert) {
                    e.revert.as_str()
                } else {
                    ""
                };
                json!({ "time": e.time, "what": e.what, "revert": revert })
            })
            .collect();
        QString::from(&Value::Array(rows).to_string())
    }

    /// Brings back the textures RimStack set aside in this mod folder.
    pub(super) fn unhide_in(&self, folder: &QString) -> QString {
        let file = hidden_file();
        let mut hidden = Hidden::read(&file);
        let path = PathBuf::from(folder.to_string());
        let done = hidden.unhide_all(&path).and_then(|_| hidden.write(&file));
        *crate::CONFLICTS.write().unwrap_or_else(|e| e.into_inner()) = None;
        QString::from(&done.err().unwrap_or_default())
    }

    pub(super) fn cancel_textures(&self) {
        STOP_TEXTURES.store(true, Ordering::Relaxed);
    }
}
