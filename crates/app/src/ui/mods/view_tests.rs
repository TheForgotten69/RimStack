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

#[test]
fn the_mod_tab_and_filter_left_come_back_and_a_new_choice_is_kept() {
    let (_tmp, mut cx, mods) = opened_with(|root| {
        std::fs::create_dir_all(root.join("settings/RimStack")).unwrap();
        std::fs::write(
            root.join("settings/RimStack/RimStack.conf"),
            "[Mods]\nlastMod=fixture.mod010\nlastTab=2\nlastFilter=is:changed fixture\n",
        )
        .unwrap();
    });
    cx.run_until_parked();
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            assert_eq!(
                mods.filter_text(cx),
                "fixture",
                "is:changed is not brought back"
            );
            let app = crate::ui::app(cx);
            let chosen = mods
                .selected_id()
                .and_then(rimstack_core::mods::library::ModId::from_qml);
            let key = chosen.and_then(|id| mods.order.package_of(&app, id));
            assert_eq!(key.as_deref(), Some("fixture.mod010"));
            assert_eq!(mods.details_view.read(cx).tab, 2);
            let other = mods
                .active
                .rows
                .iter()
                .find(|r| !r.is_header && r.mod_id != chosen.unwrap().to_qml())
                .unwrap()
                .mod_id;
            mods.choose(Side::Active, other, cx);
            let kept = app.mods_view().last_mod;
            assert_ne!(kept, "fixture.mod010");
            assert!(!kept.is_empty(), "the mod chosen now is kept");
        })
    });
}

#[test]
fn a_column_is_sized_by_its_edge_and_moved_by_its_heading() {
    use super::columns::ColumnHeld;
    use gpui::px;
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| {
        let grow = f32::from(cx.global::<crate::ui::theme::Theme>().sized(1.));
        mods.update(cx, |mods, cx| {
            mods.column_drag = Some((px(500.), px(100.)));
            mods.size_column("author", px(470.), cx);
            let wide = (130. / grow).round() as u32;
            assert_eq!(
                mods.view.column_sizes,
                format!("author={wide}"),
                "further left is wider"
            );
            mods.column_drag = Some((px(500.), px(100.)));
            mods.drop_column(ColumnHeld("author", 0), px(530.), cx);
            assert_eq!(
                mods.view.columns, "author,version,load",
                "less than half its width stays"
            );
            mods.column_drag = Some((px(500.), px(100.)));
            mods.drop_column(ColumnHeld("author", 0), px(600.), cx);
            assert_eq!(mods.view.columns, "version,author,load");
            assert_eq!(
                crate::ui::app(cx).mods_view().columns,
                "version,author,load"
            );
        })
    });
}

#[test]
fn filter_terms_become_chips_and_are_taken_off_one_by_one() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            mods.filter_by("is:warning fixture tag:x", cx)
        })
    });
    cx.update(|cx| {
        let mods = mods.read(cx);
        assert_eq!(mods.filter_terms, ["is:warning", "tag:x"]);
        assert_eq!(mods.filter.read(cx).text(), "fixture");
        assert_eq!(mods.filter_text(cx), "is:warning tag:x fixture");
    });
    let field = cx.update(|cx| mods.read(cx).filter.clone());
    cx.update(|cx| field.update(cx, |f, cx| f.set_text("fixture !is:error mod", cx)));
    cx.update(|cx| {
        let mods = mods.read(cx);
        assert_eq!(
            mods.filter_terms,
            ["is:warning", "tag:x", "!is:error"],
            "a finished term becomes a chip"
        );
        assert_eq!(mods.filter.read(cx).text(), "fixture mod");
    });
    cx.update(|cx| mods.update(cx, |mods, cx| mods.drop_term(0, cx)));
    cx.update(|cx| assert_eq!(mods.read(cx).filter_text(cx), "tag:x !is:error fixture mod"));
    cx.update(|cx| mods.update(cx, |mods, cx| mods.clear_filter(cx)));
    cx.update(|cx| assert_eq!(mods.read(cx).filter_text(cx), ""));
}

#[test]
fn a_term_toggled_and_a_reread_keep_the_chips() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| mods.update(cx, |mods, cx| mods.filter_by("tag:x fixture", cx)));
    cx.update(|cx| mods.update(cx, |mods, cx| mods.toggle_term("is:warning", cx)));
    cx.update(|cx| assert_eq!(mods.read(cx).filter_text(cx), "tag:x is:warning fixture"));
    cx.update(|cx| mods.update(cx, |mods, cx| mods.toggle_term("tag:x", cx)));
    cx.update(|cx| assert_eq!(mods.read(cx).filter_text(cx), "is:warning fixture"));
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            mods.load(true, cx);
            mods.order = crate::workspace::load_order::LoadOrder::demo(0, 0);
        })
    });
    let started = Instant::now();
    while cx.update(|cx| mods.read(cx).count(Side::Active) == 0) {
        assert!(started.elapsed() < Duration::from_secs(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    let shown = cx.update(|cx| mods.read(cx).order.filter_summary());
    let whole = cx.update(|cx| {
        let mods = mods.read(cx);
        (mods.count(Side::Active), mods.matching(Side::Active))
    });
    assert!(
        whole.1 < whole.0,
        "the reread list is still filtered: {shown}"
    );
}

#[test]
fn the_hints_go_one_after_the_other_and_are_not_shown_again() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            mods.start_hints(cx);
            assert_eq!(mods.hint_step, 1);
            mods.next_hint(cx);
            mods.next_hint(cx);
            assert_eq!(mods.hint_step, 3);
            mods.next_hint(cx);
            assert_eq!(mods.hint_step, 0, "Done ends them");
            mods.start_hints(cx);
            assert_eq!(mods.hint_step, 0, "seen once, they do not start again");
        })
    });
}

#[test]
fn an_inactive_mod_shown_while_the_list_is_folded_unfolds_it() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            mods.set_hidden(true, cx);
            let id = mods
                .inactive
                .rows
                .iter()
                .find(|r| !r.is_header)
                .unwrap()
                .mod_id;
            mods.show_mod(id, cx);
            assert!(!mods.view.inactive_hidden, "the list is pinned again");
            assert_eq!(mods.selected_id(), Some(id));
        })
    });
}

#[test]
fn select_all_chooses_only_the_mods_the_filter_shows() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| mods.update(cx, |mods, cx| mods.filter_by("Fixture Mod 04", cx)));
    cx.update(|cx| mods.update(cx, |mods, cx| mods.select_all(Side::Inactive, cx)));
    cx.update(|cx| {
        let mods = mods.read(cx);
        let mut shown: Vec<i32> = mods
            .inactive
            .rows
            .iter()
            .filter(|r| !r.is_header && r.mod_id >= 0)
            .map(|r| r.mod_id)
            .collect();
        let mut chosen = mods.selection.clone();
        shown.sort_unstable();
        chosen.sort_unstable();
        assert!(!shown.is_empty());
        assert_eq!(chosen, shown);
        assert!(chosen.len() < mods.count(Side::Inactive));
    });
}

#[test]
fn a_mod_dropped_below_the_last_row_goes_to_the_end_of_the_load_order() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            let list = mods.order.list(Side::Active).mods();
            let (first, count) = (list[3].installed.unwrap(), list.len());
            mods.drop_on((Side::Active, 3, vec![first]), (Side::Active, count), cx);
            let list = mods.order.list(Side::Active).mods();
            assert_eq!(list.last().and_then(|m| m.installed), Some(first));
        })
    });
}
