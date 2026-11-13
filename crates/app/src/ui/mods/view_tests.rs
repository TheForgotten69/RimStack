use super::edit_tests::opened_with;
use crate::workspace::load_order::Side;
use rimstack_core::mods::library::ModId;
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
        crate::ui::made_up::step(&cx);
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
                mods.layout.details_width,
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
                mods.layout.inactive_width,
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
            let chosen = mods.selected_id();
            let key = chosen.and_then(|id| mods.order.package_of(&app, id));
            assert_eq!(key.as_deref(), Some("fixture.mod010"));
            assert_eq!(mods.details_view.read(cx).tab, 2);
            let other = mods
                .active
                .rows
                .iter()
                .find(|r| !r.is_header && r.mod_id != chosen)
                .unwrap()
                .mod_id
                .unwrap();
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
            mods.layout.column_drag = Some((px(500.), px(100.)));
            mods.size_column("author", px(470.), cx);
            let wide = (130. / grow).round() as u32;
            assert_eq!(
                mods.view.column_sizes,
                format!("author={wide}"),
                "further left is wider"
            );
            mods.layout.column_drag = Some((px(500.), px(100.)));
            mods.drop_column(ColumnHeld("author", 0), px(530.), cx);
            assert_eq!(
                mods.view.columns, "author,version,load",
                "less than half its width stays"
            );
            mods.layout.column_drag = Some((px(500.), px(100.)));
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
        assert_eq!(mods.filtering.read(cx).terms(), ["is:warning", "tag:x"]);
        assert_eq!(mods.filter.read(cx).text(), "fixture");
        assert_eq!(mods.filter_text(cx), "is:warning tag:x fixture");
    });
    let field = cx.update(|cx| mods.read(cx).filter.clone());
    cx.update(|cx| field.update(cx, |f, cx| f.set_text("fixture !is:error mod", cx)));
    cx.update(|cx| {
        let mods = mods.read(cx);
        assert_eq!(
            mods.filtering.read(cx).terms(),
            ["is:warning", "tag:x", "!is:error"],
            "a finished term becomes a chip"
        );
        assert_eq!(mods.filter.read(cx).text(), "fixture mod");
    });
    cx.update(|cx| {
        let filtering = mods.read(cx).filtering.clone();
        filtering.update(cx, |filter, cx| filter.drop_term(0, cx))
    });
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
        crate::ui::made_up::step(&cx);
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
            assert_eq!(mods.hints.step, 1);
            mods.next_hint(cx);
            mods.next_hint(cx);
            assert_eq!(mods.hints.step, 3);
            mods.next_hint(cx);
            assert_eq!(mods.hints.step, 0, "Done ends them");
            mods.start_hints(cx);
            assert_eq!(mods.hints.step, 0, "seen once, they do not start again");
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
                .mod_id
                .unwrap();
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
        let mut shown: Vec<ModId> = mods
            .inactive
            .rows
            .iter()
            .filter(|r| !r.is_header)
            .filter_map(|r| r.mod_id)
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

#[test]
fn a_column_and_a_divider_move_by_their_keys() {
    use super::panes::Grip;
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    cx.update(|cx| {
        let step = cx.global::<crate::ui::theme::Theme>().sized(24.);
        mods.update(cx, |mods, cx| {
            mods.move_column("author", 1, cx);
            assert_eq!(mods.view.columns, "version,author,load");
            mods.move_column("author", -1, cx);
            assert_eq!(mods.view.columns, "author,version,load");
            let before = mods.layout.inactive_width;
            let right = gpui::KeyDownEvent {
                keystroke: gpui::Keystroke::parse("right").unwrap(),
                is_held: false,
                prefer_character_input: false,
            };
            mods.nudge_grip(Grip::Inactive, &right, cx);
            assert_eq!(mods.layout.inactive_width, before + step);
        })
    });
}

#[test]
fn enter_on_a_heading_holding_the_keys_orders_the_list_once() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    let window = cx.update(|cx| cx.windows()[0]);
    let draw = |cx: &mut gpui::HeadlessAppContext| {
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    };
    draw(&mut cx);
    let wanted = cx.update(|cx| mods.read(cx).orders.active_by["name"].then.clone());
    cx.update_window(window, |_, window, cx| {
        let handle = mods.read(cx).keys["name"].0.clone();
        window.focus(&handle, cx);
    })
    .unwrap();
    draw(&mut cx);
    let enter = gpui::Keystroke::parse("enter").unwrap();
    cx.update_window(window, |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: enter.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke: enter }),
            cx,
        );
    })
    .unwrap();
    draw(&mut cx);
    cx.update(|cx| {
        assert_eq!(
            mods.read(cx).order.order_of(Side::Active),
            rimstack_core::mods::columns::from_kept(&wanted),
            "one press, one step"
        );
    });
}

#[test]
fn the_row_under_the_pointer_is_the_one_pointed_at_as_the_list_scrolls() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    let window = cx.update(|cx| cx.windows()[0]);
    let draw = |cx: &mut gpui::HeadlessAppContext| {
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    };
    draw(&mut cx);
    let list = cx.update(|cx| mods.read(cx).list(Side::Inactive).clone());
    let (y, x) = cx.update(|cx| (list.read(cx).drawn_at(3) + gpui::px(2.), gpui::px(100.)));
    cx.update_window(window, |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                position: gpui::point(x, y),
                pressed_button: None,
                modifiers: Default::default(),
            }),
            cx,
        );
    })
    .unwrap();
    draw(&mut cx);
    assert_eq!(cx.update(|cx| list.read(cx).pointed()), Some(3));
    let row = cx.update(|cx| {
        let list = list.read(cx);
        let step = list.drawn_at(4) - list.drawn_at(3);
        list.scroll().borrow_mut().set_top(step);
        list.pointed()
    });
    assert_eq!(row, Some(4), "the list moved under a pointer that stayed");
}

#[test]
fn a_mod_moved_slides_from_where_it_was() {
    use crate::ui::widgets::watch;
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    let window = cx.update(|cx| cx.windows()[0]);
    let draw = |cx: &mut gpui::HeadlessAppContext| {
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    };
    draw(&mut cx);
    let (name, from, to) = cx.update(|cx| {
        let mods = mods.read(cx);
        let list = mods.order.list(Side::Active).mods();
        (list[5].name.clone(), 5, 6)
    });
    let top_of = |cx: &mut gpui::HeadlessAppContext| {
        watch::start();
        cx.update(|cx| {
            mods.read(cx)
                .list(Side::Active)
                .clone()
                .update(cx, |_, cx| cx.notify())
        });
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        watch::take()
            .into_iter()
            .find(|seen| seen.text.as_ref() == name)
            .map(|seen| seen.bounds.top())
    };
    let before = top_of(&mut cx).expect("the mod is drawn");
    cx.update(|cx| mods.update(cx, |mods, cx| mods.move_mod(Side::Active, from, to, cx)));
    let sliding = top_of(&mut cx).expect("the mod is drawn while it slides");
    cx.advance_clock(std::time::Duration::from_millis(500));
    draw(&mut cx);
    let landed = top_of(&mut cx).expect("the mod is drawn where it went");
    assert!(landed > before, "it went down");
    assert!(
        sliding < landed - gpui::px(1.),
        "it starts above where it lands: {sliding:?} then {landed:?}"
    );
}

#[test]
fn scrolling_the_load_order_draws_the_list_alone_again() {
    use super::frame::RENDERS;
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    let window = cx.update(|cx| cx.windows()[0]);
    let draw = |cx: &mut gpui::HeadlessAppContext| {
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    };
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            let chosen = mods.order.list(Side::Active).mods()[4].installed.unwrap();
            mods.choose(Side::Active, chosen, cx);
        })
    });
    for _ in 0..3 {
        cx.run_until_parked();
        draw(&mut cx);
    }
    let list = cx.update(|cx| mods.read(cx).list(Side::Active).clone());
    RENDERS.with(|renders| renders.set(0));
    for _ in 0..30 {
        cx.update(|cx| {
            list.update(cx, |list, cx| {
                let top = list.scroll().borrow().top();
                list.scroll().borrow_mut().set_top(top + gpui::px(3.));
                cx.notify();
            })
        });
        draw(&mut cx);
    }
    let renders = RENDERS.with(|renders| renders.get());
    assert!(
        renders < 5,
        "the Mods tab was drawn again {renders} times in 30 frames of its list scrolling"
    );
}

#[test]
fn the_window_around_the_mods_tab_is_not_drawn_again_for_what_it_does_not_show() {
    use crate::ui::shell::RENDERS;
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    let window = cx.update(|cx| cx.windows()[0]);
    let draw = |cx: &mut gpui::HeadlessAppContext| {
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    };
    for _ in 0..3 {
        draw(&mut cx);
    }
    RENDERS.with(|renders| renders.set(0));
    for at in 0..5 {
        cx.update(|cx| {
            mods.update(cx, |mods, cx| {
                let chosen = mods.order.list(Side::Active).mods()[at].installed.unwrap();
                mods.choose(Side::Active, chosen, cx);
            })
        });
        draw(&mut cx);
    }
    let chosen = RENDERS.with(|renders| renders.replace(0));
    for at in 0..5 {
        cx.update(|cx| crate::ui::tip::point_at(format!("tip {at}").into(), cx));
        draw(&mut cx);
    }
    let hinted = RENDERS.with(|renders| renders.get());
    assert_eq!(
        (chosen, hinted),
        (0, 0),
        "the window was drawn again for mods chosen and for hints"
    );
}

#[test]
fn what_the_window_shows_of_the_list_follows_an_edit_its_save_and_its_undo() {
    let (_tmp, mut cx, mods) = opened_with(|_| {});
    let around = |cx: &mut gpui::HeadlessAppContext| {
        cx.run_until_parked();
        cx.update(|cx| crate::ui::around::read(cx))
    };
    assert!(!around(&mut cx).dirty);
    cx.update(|cx| {
        mods.update(cx, |mods, cx| {
            let last = mods.count(Side::Active) - 1;
            mods.take(Side::Active, last, None, cx);
        })
    });
    let edited = around(&mut cx);
    assert!(edited.dirty && edited.unsaved.is_some());
    assert!(
        edited.in_the_way.len() > usize::from(edited.errors) + usize::from(edited.sort_pending)
    );
    cx.update(|cx| mods.update(cx, |mods, cx| mods.save(None, cx)));
    let started = Instant::now();
    while cx.update(|cx| mods.read(cx).status.saving) {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the save did not end"
        );
        crate::ui::made_up::step(&cx);
    }
    let saved = around(&mut cx);
    assert!(!saved.dirty && saved.unsaved.is_none());
    cx.update(|cx| mods.update(cx, |mods, cx| mods.undo(cx)));
    assert!(
        around(&mut cx).dirty,
        "an undo after a save leaves the list unsaved"
    );
}
