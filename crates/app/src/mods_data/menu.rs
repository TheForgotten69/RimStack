use super::qobject;
use crate::qml::values::to_qml;
use crate::workspace::load_order::Clicked;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QVariant;
use rimstack_core::mods::library::ModId;

impl qobject::ModsData {
    pub(super) fn row_menu(
        mut self: Pin<&mut Self>,
        active: bool,
        index: i32,
        mod_id: i32,
    ) -> QVariant {
        let app = self.app();
        let selected = self.chosen.ids().to_vec();
        let Ok(index) = usize::try_from(index) else {
            return to_qml(&Vec::<crate::workspace::load_order::Entry>::new());
        };
        let at = Clicked {
            active,
            index,
            id: ModId::from_qml(mod_id),
            selected: &selected,
        };
        let entries = self.as_mut().rust_mut().get_mut().order.row_menu(&app, &at);
        to_qml(&entries)
    }

    pub(super) fn sub_menu(&self, active: bool, row: i32) -> QVariant {
        let entries = usize::try_from(row)
            .map(|row| self.order.sub_menu(active, row))
            .unwrap_or_default();
        to_qml(&entries)
    }
}
