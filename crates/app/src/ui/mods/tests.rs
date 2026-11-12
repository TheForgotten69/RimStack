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
