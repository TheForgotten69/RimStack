#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++Qt" {
        include!("rimstack-app/src/mod_rows.cxxqt.h");
        type QAbstractListModel = crate::mod_rows::qobject::QAbstractListModel;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qvector.h");
        type QVector_i32 = cxx_qt_lib::QVector<i32>;
    }

    extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qml_uncreatable]
        type WorkshopRows = super::WorkshopRowsRust;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[namespace = "rust::cxxqtlib1"]
        fn make_workshop_rows() -> UniquePtr<WorkshopRows>;
    }

    extern "RustQt" {
        #[qinvokable]
        #[cxx_override]
        fn data(self: &WorkshopRows, index: &QModelIndex, role: i32) -> QVariant;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &WorkshopRows, parent: &QModelIndex) -> i32;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &WorkshopRows) -> QHash_i32_QByteArray;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[qsignal]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut WorkshopRows>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QVector_i32,
        );

        #[inherit]
        fn index(self: &WorkshopRows, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;
    }

    extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginInsertRows"]
        unsafe fn begin_insert_rows(
            self: Pin<&mut WorkshopRows>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endInsertRows"]
        unsafe fn end_insert_rows(self: Pin<&mut WorkshopRows>);
        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        unsafe fn begin_remove_rows(
            self: Pin<&mut WorkshopRows>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endRemoveRows"]
        unsafe fn end_remove_rows(self: Pin<&mut WorkshopRows>);
        #[inherit]
        #[cxx_name = "beginMoveRows"]
        unsafe fn begin_move_rows(
            self: Pin<&mut WorkshopRows>,
            source_parent: &QModelIndex,
            source_first: i32,
            source_last: i32,
            destination_parent: &QModelIndex,
            destination_child: i32,
        ) -> bool;
        #[inherit]
        #[cxx_name = "endMoveRows"]
        unsafe fn end_move_rows(self: Pin<&mut WorkshopRows>);
        #[inherit]
        #[cxx_name = "beginResetModel"]
        unsafe fn begin_reset_model(self: Pin<&mut WorkshopRows>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        unsafe fn end_reset_model(self: Pin<&mut WorkshopRows>);
    }
}

use crate::qml::diff::steps;
use crate::qml::values::to_qml;
use crate::workspace::browse::Row;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant, QVector,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub row: Row,
    pub preview: String,
}

#[derive(Default)]
pub struct WorkshopRowsRust {
    shown: Vec<Item>,
}

pub const ROLES: [&str; 10] = [
    "id",
    "title",
    "author",
    "versions",
    "installed",
    "onList",
    "badge",
    "canAdd",
    "preview",
    "stars",
];
const FIRST_ROLE: i32 = 257;

fn cell(item: &Item, field: usize) -> Option<QVariant> {
    let row = &item.row;
    let text = |t: &str| QVariant::from(&QString::from(t));
    Some(match field {
        0 => text(&row.id),
        1 => text(&row.title),
        2 => text(row.author.as_deref().unwrap_or_default()),
        3 => to_qml(&row.versions),
        4 => QVariant::from(&row.installed),
        5 => QVariant::from(&row.on_list),
        6 => QVariant::from(
            &row.badge
                .as_ref()
                .map_or_else(QString::default, crate::words::said),
        ),
        7 => QVariant::from(&row.can_add),
        8 => text(&item.preview),
        9 => to_qml(&row.stars),
        _ => return None,
    })
}

crate::qml::apply::shown_rows!(qobject::WorkshopRows, Item);

impl qobject::WorkshopRows {
    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let field = usize::try_from(role - FIRST_ROLE).ok();
        let item = usize::try_from(index.row())
            .ok()
            .and_then(|at| self.shown.get(at));
        item.zip(field)
            .and_then(|(item, field)| cell(item, field))
            .unwrap_or_default()
    }

    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        self.shown.len() as i32
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut names = QHash::<QHashPair_i32_QByteArray>::default();
        for (at, name) in ROLES.iter().enumerate() {
            names.insert(FIRST_ROLE + at as i32, QByteArray::from(*name));
        }
        names
    }

    pub(crate) fn show(mut self: Pin<&mut Self>, items: Vec<Item>) {
        let before: Vec<_> = self.shown.iter().map(|i| i.row.id.clone()).collect();
        let after: Vec<_> = items.iter().map(|i| i.row.id.clone()).collect();
        match steps(&before, &after) {
            Some(steps) => {
                crate::qml::apply::apply(self.as_mut(), &steps, &items);
                self.changed_rows(items);
            }
            // Results found anew start at the top, as a reset puts the grid.
            None => {
                unsafe { self.as_mut().begin_reset_model() };
                self.as_mut().rust_mut().shown = items;
                unsafe { self.as_mut().end_reset_model() };
            }
        }
    }

    pub(crate) fn set_preview(mut self: Pin<&mut Self>, id: &str, preview: &str) {
        let Some(at) = self.shown.iter().position(|i| i.row.id == id) else {
            return;
        };
        if self.shown[at].preview == preview {
            return;
        }
        self.as_mut().rust_mut().shown[at].preview = preview.to_string();
        let index = self.index(at as i32, 0, &QModelIndex::default());
        self.data_changed(&index, &index, &QVector::default());
    }

    pub(crate) fn set_stars(mut self: Pin<&mut Self>, id: &str, stars: &[bool]) {
        let Some(at) = self.shown.iter().position(|i| i.row.id == id) else {
            return;
        };
        if self.shown[at].row.stars == stars {
            return;
        }
        self.as_mut().rust_mut().shown[at].row.stars = stars.to_vec();
        let index = self.index(at as i32, 0, &QModelIndex::default());
        self.data_changed(&index, &index, &QVector::default());
    }

    pub(crate) fn reworded(self: Pin<&mut Self>) {
        let Some(last) = self.shown.len().checked_sub(1) else {
            return;
        };
        let root = QModelIndex::default();
        let (top, bottom) = (self.index(0, 0, &root), self.index(last as i32, 0, &root));
        self.data_changed(&top, &bottom, &QVector::default());
    }

    fn changed_rows(mut self: Pin<&mut Self>, items: Vec<Item>) {
        let changed: Vec<usize> = (0..items.len())
            .filter(|&at| self.shown.get(at) != Some(&items[at]))
            .collect();
        self.as_mut().rust_mut().shown = items;
        let (Some(&first), Some(&last)) = (changed.first(), changed.last()) else {
            return;
        };
        let root = QModelIndex::default();
        let (top, bottom) = (
            self.index(first as i32, 0, &root),
            self.index(last as i32, 0, &root),
        );
        self.data_changed(&top, &bottom, &QVector::default());
    }
}
