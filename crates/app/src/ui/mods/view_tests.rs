use super::edit_tests::opened_with;
use crate::workspace::load_order::Side;
use rimstack_core::mods::list::SortKey;
use std::time::{Duration, Instant};

#[test]
fn the_view_chosen_is_kept_and_read_again_with_the_mods() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            mods.set_headings(false, cx);
            assert!(!mods.order.headings());
            mods.toggle_column("updated", cx);
            mods.order_inactive("name down", cx);
            assert_eq!(mods.order.inactive_by(), Some((SortKey::Name, true)));
            mods.heading_pressed("name", cx);
            assert_eq!(mods.orders.active_key, "name");
            let kept = crate::ui::app(cx).mods_view();
            assert!(!kept.headings);
            assert_eq!(kept.columns, "author,version,load,updated");
            assert_eq!(kept.inactive_order, "name down");
            mods.load(true, cx);
            mods.order = crate::workspace::load_order::LoadOrder::demo(0, 0);
        })
    });
    let started = Instant::now();
    while cx.update(|cx| mods.read(cx).count(Side::Active) == 0) {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the mods were not read again"
        );
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    cx.update(|cx| {
        let mods = mods.read(cx);
        assert!(!mods.order.headings(), "a reread keeps one numbered list");
        assert_eq!(mods.order.inactive_by(), Some((SortKey::Name, true)));
        assert_eq!(
            mods.order.order_of(Side::Active).map(|(key, _)| key),
            Some(SortKey::Name),
            "a reread keeps the column the load order is shown by"
        );
    });
}

#[test]
fn the_inactive_mods_fold_into_the_rail_and_the_dividers_keep_their_least() {
    use super::panes::{Grip, pane_widths};
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| {
        let theme = cx.global::<crate::ui::theme::Theme>().clone();
        mods.update(cx, |mods, cx| {
            mods.set_hidden(true, cx);
            assert!(crate::ui::app(cx).mods_view().inactive_hidden);
            let window = gpui::px(1440.);
            let (inactive, active, details) = pane_widths(&theme, window, mods.panes_wanted());
            assert_eq!(inactive, gpui::px(0.));
            assert_eq!(
                inactive
                    + active
                    + details
                    + super::panes::rail_width(&theme)
                    + crate::ui::theme::HANDLE,
                window
            );
            let area = gpui::Bounds::new(
                gpui::point(gpui::px(0.), gpui::px(0.)),
                gpui::size(window, gpui::px(800.)),
            );
            mods.drag_grip(
                Grip::Details,
                gpui::point(gpui::px(1400.), gpui::px(10.)),
                area,
                cx,
            );
            assert_eq!(
                mods.details_width,
                theme.sized(260.),
                "the details keep their least width"
            );
            mods.drag_grip(
                Grip::Inactive,
                gpui::point(gpui::px(500.), gpui::px(10.)),
                area,
                cx,
            );
            assert_eq!(
                mods.inactive_width,
                gpui::px(500.) - crate::ui::theme::HANDLE / 2.
            );
            mods.set_hidden(false, cx);
            assert!(!mods.panes_wanted().hidden);
        })
    });
}
