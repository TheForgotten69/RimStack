use super::Mods;
use crate::App;
use crate::ui::{Options, install};
use crate::workspace::load_order::Side;
use std::sync::Arc;

fn demo(cx: &mut gpui::TestAppContext) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let path = root.path();
    let app = App::new(
        path.join("home"),
        &path.join("settings"),
        &path.join("data"),
        Default::default(),
    );
    let options = Options {
        demo: true,
        ..Options::default()
    };
    cx.update(|cx| install(Arc::new(app), &options, cx));
    root
}

#[gpui::test]
fn typing_in_the_filter_narrows_both_lists_and_clearing_it_shows_all(
    cx: &mut gpui::TestAppContext,
) {
    let _root = demo(cx);
    let (mods, cx) = cx.add_window_view(|_, cx| Mods::new(true, cx));
    let all = mods.read_with(cx, |mods, _| {
        (mods.matching(Side::Active), mods.matching(Side::Inactive))
    });
    cx.update(|window, cx| {
        let focus = mods.read(cx).focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("ctrl-f");
    let focused = cx.update(|window, cx| mods.read(cx).filter.read(cx).is_focused(window));
    assert!(focused, "Ctrl+F gives the filter the keys");
    cx.simulate_input("Weapons 1");
    let (typed, narrowed) = mods.read_with(cx, |mods, cx| {
        (mods.filter_text(cx), mods.matching(Side::Active))
    });
    assert_eq!(typed, "Weapons 1");
    assert!(narrowed > 0 && narrowed < all.0, "{narrowed} of {}", all.0);
    cx.simulate_keystrokes("backspace backspace");
    assert_eq!(
        mods.read_with(cx, |mods, cx| mods.filter_text(cx)),
        "Weapons"
    );
    cx.simulate_keystrokes("escape");
    let back = mods.read_with(cx, |mods, _| {
        (mods.matching(Side::Active), mods.matching(Side::Inactive))
    });
    assert_eq!(back, all);
}

#[gpui::test]
fn a_category_heading_folds_its_mods_away(cx: &mut gpui::TestAppContext) {
    let _root = demo(cx);
    let (mods, cx) = cx.add_window_view(|_, cx| Mods::new(true, cx));
    let rows = |mods: &Mods| mods.active.rows.len();
    let before = mods.read_with(cx, |mods, _| rows(mods));
    let first = mods.read_with(cx, |mods, _| mods.active.rows[0].clone());
    assert!(first.is_header && first.count > 0);
    mods.update(cx, |mods, cx| {
        mods.toggle_category(Side::Active, first.category, cx)
    });
    let after = mods.read_with(cx, |mods, _| rows(mods));
    assert_eq!(after, before - first.count as usize);
}

#[gpui::test]
fn a_click_on_a_category_heading_folds_it(cx: &mut gpui::TestAppContext) {
    use gpui::{Modifiers, point, px};
    let _root = demo(cx);
    let (mods, cx) = cx.add_window_view(|_, cx| Mods::new(true, cx));
    cx.run_until_parked();
    let (before, first) = mods.read_with(cx, |mods, _| {
        (mods.active.rows.len(), mods.active.rows[0].clone())
    });
    // At text size 1: the inactive pane and its edge, then the title row, the filter and the headings.
    let x = px(290. + 7. + 120.);
    let y = px(42. + 28. + 8. + 24. + 4. + 16.);
    cx.simulate_click(point(x, y), Modifiers::none());
    let after = mods.read_with(cx, |mods, _| mods.active.rows.len());
    assert_eq!(after, before - first.count as usize);
}

#[gpui::test]
fn a_fold_keeps_the_list_where_it_was_scrolled(cx: &mut gpui::TestAppContext) {
    use gpui::{ListOffset, px};
    let _root = demo(cx);
    let (mods, cx) = cx.add_window_view(|_, cx| Mods::new(true, cx));
    cx.run_until_parked();
    let at = ListOffset {
        item_ix: 300,
        offset_in_item: px(0.),
    };
    mods.update(cx, |mods, _| mods.active.list.scroll_to(at));
    let last = mods
        .read_with(cx, |mods, _| {
            mods.active.rows.iter().rev().find(|r| r.is_header).cloned()
        })
        .unwrap();
    mods.update(cx, |mods, cx| {
        mods.toggle_category(Side::Active, last.category, cx)
    });
    let kept = mods.read_with(cx, |mods, _| mods.active.list.logical_scroll_top());
    assert_eq!(kept.item_ix, 300);
}

#[gpui::test]
fn the_wheel_scrolls_the_load_order(cx: &mut gpui::TestAppContext) {
    use gpui::{MouseMoveEvent, ScrollDelta, ScrollEvent, point, px};
    let _root = demo(cx);
    let (mods, cx) = cx.add_window_view(|_, cx| Mods::new(true, cx));
    cx.run_until_parked();
    let over = point(px(600.), px(500.));
    cx.simulate_event(MouseMoveEvent {
        position: over,
        ..MouseMoveEvent::default()
    });
    for _ in 0..40 {
        let wheel = ScrollEvent {
            position: over,
            delta: ScrollDelta::Pixels(point(px(0.), px(-200.))),
            ..ScrollEvent::default()
        };
        cx.simulate_event(wheel);
        cx.run_until_parked();
    }
    let top = mods.read_with(cx, |mods, _| mods.active.list.logical_scroll_top().item_ix);
    assert!(top > 100, "the load order shows row {top} at its top");
}
