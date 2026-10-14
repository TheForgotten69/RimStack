//! The load order's columns, as the Mods screen keeps them in two lines
//! of its settings: see `rimstack_core::mods::columns`.

use super::*;
use rimstack_core::mods::columns;

impl qobject::Backend {
    pub(super) fn columns_toggled(&self, kept: &QString, column: &QString) -> QString {
        QString::from(&columns::toggled(&kept.to_string(), &column.to_string()))
    }

    pub(super) fn columns_moved(&self, kept: &QString, column: &QString, to: i32) -> QString {
        QString::from(&columns::moved(&kept.to_string(), &column.to_string(), to))
    }

    pub(super) fn column_widths(&self, kept: &QString) -> QString {
        crate::reports::said(&columns::widths(&kept.to_string()))
    }

    pub(super) fn column_sized(&self, kept: &QString, column: &QString, width: f64) -> QString {
        QString::from(&columns::sized(
            &kept.to_string(),
            &column.to_string(),
            width,
        ))
    }

    pub(super) fn column_pressed(
        &self,
        by: &QString,
        descending: bool,
        asked: &QString,
    ) -> QString {
        let asked = asked.to_string();
        let now = columns::pressed(&by.to_string(), descending, &asked);
        crate::reports::said(&now)
    }
}
