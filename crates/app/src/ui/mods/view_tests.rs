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
