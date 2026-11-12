use super::made_up::made_up;
use super::shell::Shell;
use super::{Options, install};
use crate::workspace::load_order::Side;
use gpui::{AppContext, HeadlessAppContext, WindowHandle, size};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The window on the made-up install, its load order read.
fn opened() -> (tempfile::TempDir, HeadlessAppContext, WindowHandle<Shell>) {
    let (tmp, app) = made_up();
    let text = gpui_platform::current_platform(true).text_system();
    let mut cx = HeadlessAppContext::with_asset_source(text, Arc::new(super::assets::Assets));
    let options = Options {
        still: true,
        ..Options::default()
    };
    cx.update(|cx| install(Arc::new(app), &options, cx));
    let shell = cx
        .open_window(size(super::WINDOW.0, super::WINDOW.1), |window, cx| {
            cx.new(|cx| Shell::new(&options, window, cx))
        })
        .unwrap();
    let started = Instant::now();
    while active(&mut cx, shell) == 0 {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the made-up mods were not read"
        );
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    (tmp, cx, shell)
}

fn active(cx: &mut HeadlessAppContext, shell: WindowHandle<Shell>) -> usize {
    cx.update(|cx| {
        shell
            .read(cx)
            .map(|shell| shell.mods.read(cx).count(Side::Active))
            .unwrap()
    })
}

#[test]
fn the_search_for_the_broken_mod_halves_the_list_and_gives_it_back() {
    let (_tmp, mut cx, shell) = opened();
    let all = active(&mut cx, shell);
    shell
        .update(&mut cx, |shell, _, cx| shell.start_hunt(cx))
        .unwrap();
    let (hunting, asking) = cx.update(|cx| {
        let shell = shell.read(cx).unwrap();
        (shell.hunt.active, shell.hunt.asking)
    });
    assert!(hunting && asking);
    assert!(active(&mut cx, shell) < all, "a round deactivates suspects");
    shell
        .update(&mut cx, |shell, _, cx| shell.answer_hunt(true, cx))
        .unwrap();
    assert!(cx.update(|cx| shell.read(cx).unwrap().hunt.active));
    shell
        .update(&mut cx, |shell, _, cx| shell.end_hunt(false, cx))
        .unwrap();
    assert!(!cx.update(|cx| shell.read(cx).unwrap().hunt.active));
    assert_eq!(
        active(&mut cx, shell),
        all,
        "the list the search began with comes back"
    );
}

#[test]
fn play_shows_what_is_in_its_way_before_starting() {
    let (_tmp, mut cx, shell) = opened();
    shell
        .update(&mut cx, |shell, _, cx| shell.press_play(cx))
        .unwrap();
    let (open, ways) = cx.update(|cx| {
        let shell = shell.read(cx).unwrap();
        (shell.before_play, shell.mods.read(cx).in_the_way_said(cx))
    });
    assert!(!ways.is_empty(), "the made-up load order has an error");
    assert!(open);
}
