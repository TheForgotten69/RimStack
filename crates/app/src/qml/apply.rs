use super::diff::Step;
use core::pin::Pin;

pub trait Shown: Sized {
    type Item: Clone;
    fn shown(self: Pin<&mut Self>) -> &mut Vec<Self::Item>;
    /// # Safety
    /// Each `begin_*` is followed by its `end_*` once the rows are changed.
    unsafe fn begin_insert(self: Pin<&mut Self>, first: i32, last: i32);
    unsafe fn end_insert(self: Pin<&mut Self>);
    unsafe fn begin_remove(self: Pin<&mut Self>, first: i32, last: i32);
    unsafe fn end_remove(self: Pin<&mut Self>);
    unsafe fn begin_move(self: Pin<&mut Self>, from: i32, before: i32);
    unsafe fn end_move(self: Pin<&mut Self>);
    fn changed(self: Pin<&mut Self>, first: i32, last: i32);
}

pub fn apply<M: Shown>(mut model: Pin<&mut M>, steps: &[Step], items: &[M::Item]) {
    for step in steps {
        // SAFETY: each change is announced before the rows change and ended right after.
        unsafe {
            match *step {
                Step::Remove { first, last } => {
                    model.as_mut().begin_remove(first as i32, last as i32);
                    model.as_mut().shown().drain(first..=last);
                    model.as_mut().end_remove();
                }
                Step::Insert { first, last } => {
                    model.as_mut().begin_insert(first as i32, last as i32);
                    let coming = items[first..=last].iter().cloned();
                    model.as_mut().shown().splice(first..first, coming);
                    model.as_mut().end_insert();
                }
                Step::Move { from, to } => {
                    // Qt names the place as the row it goes before, counted before the move.
                    let before = if to > from { to + 1 } else { to };
                    model.as_mut().begin_move(from as i32, before as i32);
                    let item = model.as_mut().shown().remove(from);
                    model.as_mut().shown().insert(to, item);
                    model.as_mut().end_move();
                }
            }
        }
    }
}

/// Fills the rows again in place: the delegates are rebound to their new rows, not rebuilt.
pub fn renew<M: Shown>(mut model: Pin<&mut M>, items: Vec<M::Item>) {
    let old = model.as_mut().shown().len();
    let new = items.len();
    // SAFETY: each change is announced before the rows change and ended right after.
    unsafe {
        if new > old {
            model.as_mut().begin_insert(old as i32, new as i32 - 1);
            model.as_mut().shown().extend(items[old..].iter().cloned());
            model.as_mut().end_insert();
        } else if new < old {
            model.as_mut().begin_remove(new as i32, old as i32 - 1);
            model.as_mut().shown().truncate(new);
            model.as_mut().end_remove();
        }
    }
    *model.as_mut().shown() = items;
    if new > 0 {
        model.changed(0, new as i32 - 1);
    }
}

macro_rules! shown_rows {
    ($model:ty, $item:ty) => {
        impl $crate::qml::apply::Shown for $model {
            type Item = $item;
            fn shown(self: Pin<&mut Self>) -> &mut Vec<$item> {
                &mut self.rust_mut().get_mut().shown
            }
            unsafe fn begin_insert(self: Pin<&mut Self>, first: i32, last: i32) {
                unsafe { self.begin_insert_rows(&QModelIndex::default(), first, last) }
            }
            unsafe fn end_insert(self: Pin<&mut Self>) {
                unsafe { self.end_insert_rows() }
            }
            unsafe fn begin_remove(self: Pin<&mut Self>, first: i32, last: i32) {
                unsafe { self.begin_remove_rows(&QModelIndex::default(), first, last) };
            }
            unsafe fn end_remove(self: Pin<&mut Self>) {
                unsafe { self.end_remove_rows() }
            }
            unsafe fn begin_move(self: Pin<&mut Self>, from: i32, before: i32) {
                let root = QModelIndex::default();
                unsafe { self.begin_move_rows(&root, from, from, &root, before) };
            }
            unsafe fn end_move(self: Pin<&mut Self>) {
                unsafe { self.end_move_rows() }
            }
            fn changed(self: Pin<&mut Self>, first: i32, last: i32) {
                let root = QModelIndex::default();
                let (top, bottom) = (self.index(first, 0, &root), self.index(last, 0, &root));
                self.data_changed(&top, &bottom, &QVector::default());
            }
        }
    };
}
pub(crate) use shown_rows;
