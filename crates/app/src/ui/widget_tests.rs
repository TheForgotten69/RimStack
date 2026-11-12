use super::button::Btn;
use super::controls::{Choice, Toggle};
use super::{Options, focus, install, menu, notices};
use crate::App;
use gpui::{
    Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, prelude::FluentBuilder,
};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
struct Bench {
    first: usize,
    second: usize,
    on: bool,
    picked: usize,
    chosen: Option<&'static str>,
    menu: Option<Entity<menu::Menu>>,
    notices: Option<Entity<notices::Notices>>,
    focus: Option<gpui::FocusHandle>,
}

impl Render for Bench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let me = cx.entity().downgrade();
        let pick = me.clone();
        div()
            .size_full()
            .when_some(self.focus.clone(), |d, focus| d.track_focus(&focus))
            .on_action(|_: &focus::Next, window, cx| window.focus_next(cx))
            .on_action(|_: &focus::Previous, window, cx| window.focus_prev(cx))
            .child(Btn::new("first", "First").on_click(cx.listener(|b, _, _, _| b.first += 1)))
            .child(Btn::new("second", "Second").on_click(cx.listener(|b, _, _, _| b.second += 1)))
            .child(
                Toggle::new("toggle", self.on, "Toggle").on_flip(move |on, _, cx| {
                    let _ = me.update(cx, |b, cx| {
                        b.on = on;
                        cx.notify();
                    });
                }),
            )
            .child(
                Choice::new(
                    "choice",
                    vec!["A".into(), "B".into(), "C".into()],
                    self.picked,
                )
                .on_pick(move |at, _, cx| {
                    let _ = pick.update(cx, |b, cx| {
                        b.picked = at;
                        cx.notify();
                    });
                }),
            )
            .children(self.notices.clone())
            .children(self.menu.clone())
    }
}

fn bench(cx: &mut TestAppContext) -> (tempfile::TempDir, Entity<Bench>, &mut VisualTestContext) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path();
    let app = App::new(
        path.join("home"),
        &path.join("settings"),
        &path.join("data"),
        Default::default(),
    );
    cx.update(|cx| {
        install(
            Arc::new(app),
            &Options {
                demo: true,
                ..Options::default()
            },
            cx,
        )
    });
    let (bench, cx) = cx.add_window_view(|_, cx| Bench {
        menu: Some(menu::install(cx)),
        notices: Some(notices::install(cx)),
        focus: Some(cx.focus_handle()),
        ..Bench::default()
    });
    cx.update(|window, cx| {
        let focus = bench.read(cx).focus.clone().unwrap();
        window.focus(&focus, cx);
    });
    cx.run_until_parked();
    (root, bench, cx)
}

/// Each key down then up, as a keyboard sends them: a control is pressed on the key's release.
pub(crate) fn press(cx: &mut VisualTestContext, keys: &'static str) {
    for key in keys.split_whitespace() {
        cx.simulate_keystrokes(key);
        let keystroke = gpui::Keystroke::parse(key).unwrap();
        cx.simulate_event(gpui::KeyUpEvent { keystroke });
    }
}

#[gpui::test]
fn tab_reaches_each_control_and_enter_or_space_presses_it(cx: &mut TestAppContext) {
    let (_root, bench, cx) = bench(cx);
    press(cx, "tab enter tab space");
    assert_eq!(bench.read_with(cx, |b, _| (b.first, b.second)), (1, 1));
    press(cx, "tab space");
    assert!(bench.read_with(cx, |b, _| b.on), "Space flips the toggle");
    press(cx, "tab right right");
    assert_eq!(
        bench.read_with(cx, |b, _| b.picked),
        2,
        "the arrows move the choice"
    );
    press(cx, "shift-tab space");
    assert!(!bench.read_with(cx, |b, _| b.on), "Shift+Tab goes back");
}

#[gpui::test]
fn a_menu_is_chosen_from_with_the_arrows_and_closes_on_escape(cx: &mut TestAppContext) {
    let (_root, bench, cx) = bench(cx);
    let entries = |bench: &Entity<Bench>| -> Vec<menu::Entry> {
        let (one, two) = (bench.downgrade(), bench.downgrade());
        vec![
            menu::Entry::Title("Things".into()),
            menu::Entry::item("Off", |_, _| {}).off(true).into(),
            menu::Entry::item("One", move |_, cx| {
                let _ = one.update(cx, |b, _| b.chosen = Some("one"));
            })
            .into(),
            menu::Entry::Line,
            menu::Entry::item("Two", move |_, cx| {
                let _ = two.update(cx, |b, _| b.chosen = Some("two"));
            })
            .into(),
        ]
    };
    cx.update(|window, cx| {
        menu::open_at(
            gpui::point(gpui::px(10.), gpui::px(10.)),
            entries(&bench),
            window,
            cx,
        )
    });
    cx.simulate_keystrokes("down down enter");
    assert_eq!(
        bench.read_with(cx, |b, _| b.chosen),
        Some("two"),
        "the arrows skip what is off"
    );
    cx.update(|window, cx| {
        menu::open_at(
            gpui::point(gpui::px(10.), gpui::px(10.)),
            entries(&bench),
            window,
            cx,
        )
    });
    cx.simulate_keystrokes("escape enter");
    assert_eq!(
        bench.read_with(cx, |b, _| b.chosen),
        Some("two"),
        "Escape closes it without a choice"
    );
}

#[gpui::test]
fn a_toast_goes_by_itself_after_a_while(cx: &mut TestAppContext) {
    let (_root, bench, cx) = bench(cx);
    cx.update(|_, cx| notices::say("Saved", notices::Tone::Ok, cx));
    let shown = |cx: &mut VisualTestContext| {
        let notices = bench.read_with(cx, |b, _| b.notices.clone().unwrap());
        notices.read_with(cx, |n, _| n.shown().len())
    };
    assert_eq!(shown(cx), 1);
    cx.executor().advance_clock(Duration::from_secs(6));
    cx.run_until_parked();
    assert_eq!(shown(cx), 0);
}
