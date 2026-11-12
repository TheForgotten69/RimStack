use super::Settings;
use crate::App;
use crate::ui::layout_checks::check_scene;
use crate::ui::shell::Shell;
use crate::ui::theme::{Theme, ThemeName};
use crate::ui::widgets::watch::{self, Seen};
use crate::ui::{Options, install};
use gpui::{
    AppContext, Entity, HeadlessAppContext, Pixels, Size, TestAppContext, VisualTestContext, size,
};
use std::sync::Arc;

fn settings(
    cx: &mut TestAppContext,
    part: usize,
) -> (tempfile::TempDir, Entity<Settings>, &mut VisualTestContext) {
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
        section: 7,
        part,
        ..Options::default()
    };
    cx.update(|cx| install(Arc::new(app), &options, cx));
    let (shell, cx) = cx.add_window_view(|window, cx| Shell::new(&options, window, cx));
    let settings = shell.read_with(cx, |shell, _| shell.settings.clone().unwrap());
    cx.run_until_parked();
    (root, settings, cx)
}

fn shown_titles(settings: &Entity<Settings>, cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|window, cx| {
        settings.update(cx, |s, cx| {
            let lines = s.lines(window, cx);
            lines
                .into_iter()
                .filter_map(|row| match row {
                    super::line::Row::Line(line) if s.shows(&line) => Some(line.title),
                    _ => None,
                })
                .collect()
        })
    })
}

#[gpui::test]
fn a_search_shows_the_settings_it_names_from_every_part(cx: &mut TestAppContext) {
    let (_root, settings, cx) = settings(cx, 0);
    assert!(shown_titles(&settings, cx).contains(&"Launch arguments".to_string()));
    let field = settings.read_with(cx, |s, _| s.fields.find.clone());
    field.update(cx, |field, cx| field.set_text("motion", cx));
    settings.update(cx, |s, cx| {
        s.finding = "motion".into();
        cx.notify();
    });
    assert_eq!(shown_titles(&settings, cx), ["Motion"]);
}

#[gpui::test]
fn a_theme_chosen_is_kept_in_rimstack_conf_and_shown_at_once(cx: &mut TestAppContext) {
    let (root, settings, cx) = settings(cx, 1);
    settings.update(cx, |s, cx| s.keep_look("theme", "light", cx));
    let kept =
        std::fs::read_to_string(root.path().join("settings/RimStack/RimStack.conf")).unwrap();
    assert!(kept.contains("[Appearance]\ntheme=light"), "{kept}");
    assert_eq!(
        cx.update(|_, cx| cx.global::<Theme>().appearance.theme),
        ThemeName::Light
    );
    cx.update(|window, cx| settings.update(cx, |s, cx| s.reset_page(window, cx)));
    assert_eq!(
        cx.update(|_, cx| cx.global::<Theme>().appearance.theme),
        ThemeName::Rim,
        "Reset this page brings it back"
    );
}

/// Settings opened on `part` in a demo window, drawn with the checks on.
/// The window a scene is drawn in, shaped by the Theme.
type Shape<'a> = &'a dyn Fn(&Theme) -> Size<Pixels>;

fn drawn(part: usize) -> impl Fn(f32, Shape<'_>) -> Vec<Seen> {
    move |text_scale, window| {
        let root = tempfile::tempdir().unwrap();
        let path = root.path();
        let app = App::new(
            path.join("home"),
            &path.join("settings"),
            &path.join("data"),
            Default::default(),
        );
        let text = crate::ui::made_up::text_system();
        let mut cx =
            HeadlessAppContext::with_asset_source(text, Arc::new(crate::ui::assets::Assets));
        let options = Options {
            demo: true,
            section: 7,
            part,
            text_scale: Some(text_scale),
            ..Options::default()
        };
        cx.update(|cx| install(Arc::new(app), &options, cx));
        let shape = cx.update(|cx| window(cx.global::<Theme>()));
        watch::start();
        let opened = cx
            .open_window(shape, |window, cx| {
                cx.new(|cx| Shell::new(&options, window, cx))
            })
            .unwrap();
        cx.update_window(opened.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        watch::take()
    }
}

#[test]
fn every_part_of_settings_fits_at_every_text_size() {
    for part in 0..5 {
        check_scene(
            &format!("Settings part {part}, first window"),
            30,
            &|_| size(crate::ui::WINDOW.0, crate::ui::WINDOW.1),
            drawn(part),
        );
        check_scene(
            &format!("Settings part {part}, smallest window"),
            30,
            &|theme| size(theme.sized(1000.), theme.sized(560.)),
            drawn(part),
        );
    }
}

#[gpui::test]
fn the_arrows_go_from_part_to_part_once_tab_reached_them(cx: &mut TestAppContext) {
    let (_root, settings, cx) = settings(cx, 0);
    cx.update(|window, cx| {
        let keys = settings.read(cx).part_keys.clone();
        window.focus(&keys, cx);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("down down up");
    assert_eq!(settings.read_with(cx, |s, _| s.part), 1);
}
