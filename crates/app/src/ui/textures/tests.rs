use super::Textures;
use crate::ui::layout_checks::check_scene;
use crate::ui::made_up::made_up;
use crate::ui::shell::Shell;
use crate::ui::theme::Theme;
use crate::ui::widgets::watch::{self, Seen};
use crate::ui::{Options, install};
use crate::workspace::textures::shown;
use gpui::{AppContext, Entity, HeadlessAppContext, Pixels, Size, WindowHandle, size};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The window a scene is drawn in, shaped by the Theme.
type Shape<'a> = &'a dyn Fn(&Theme) -> Size<Pixels>;

/// Textures on the made-up install, its textures counted.
fn opened(
    text_scale: f32,
    window: Shape<'_>,
) -> (
    tempfile::TempDir,
    HeadlessAppContext,
    WindowHandle<Shell>,
    Entity<Textures>,
) {
    let (tmp, app) = made_up();
    let text = crate::ui::made_up::text_system();
    let mut cx = HeadlessAppContext::with_asset_source(text, Arc::new(crate::ui::assets::Assets));
    let options = Options {
        section: crate::ui::section::Section::Textures,
        text_scale: Some(text_scale),
        ..Options::default()
    };
    cx.update(|cx| install(Arc::new(app), &options, cx));
    let shape = cx.update(|cx| window(cx.global::<Theme>()));
    let shell = cx
        .open_window(shape, |window, cx| {
            cx.new(|cx| Shell::new(&options, window, cx))
        })
        .unwrap();
    let textures = cx
        .update(|cx| shell.read(cx).map(|shell| shell.textures.clone().unwrap()))
        .unwrap();
    let started = Instant::now();
    while cx.update(|cx| textures.read(cx).rows.is_empty()) {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the made-up textures were not counted"
        );
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    cx.run_until_parked();
    (tmp, cx, shell, textures)
}

#[test]
fn the_made_up_install_counts_its_textures_and_none_has_a_dds() {
    let (_tmp, mut cx, _shell, textures) =
        opened(1.0, &|_| size(crate::ui::WINDOW.0, crate::ui::WINDOW.1));
    let (have, total, remaining) = cx.update(|cx| {
        let rows = textures.read(cx).rows.clone();
        let shown = shown(&rows, true);
        (shown.have, shown.total, shown.remaining.len())
    });
    assert_eq!((have, total, remaining), (0, 13, 13));
}

fn drawn(text_scale: f32, window: Shape<'_>) -> Vec<Seen> {
    let (_tmp, mut cx, shell, textures) = opened(text_scale, window);
    watch::start();
    cx.update(|cx| textures.update(cx, |_, cx| cx.notify()));
    cx.update_window(shell.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    watch::take()
}

#[test]
fn textures_fits_at_every_text_size() {
    check_scene(
        "Textures, first window",
        20,
        &|_| size(crate::ui::WINDOW.0, crate::ui::WINDOW.1),
        drawn,
    );
    check_scene(
        "Textures, smallest window",
        20,
        &|theme| size(theme.sized(1000.), theme.sized(560.)),
        drawn,
    );
}
