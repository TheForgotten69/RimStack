//! The window: views that show what the `App` hands them and forward what is done.

pub mod assets;
pub mod button;
pub mod card;
pub mod controls;
pub mod downloads;
pub mod field;
pub mod focus;
pub mod menu;
pub mod mods;
pub mod notices;
pub mod page;
pub mod problem;
pub mod prompt;
pub mod rows;
pub mod shell;
pub mod status_bar;
pub mod text_field;
pub mod theme;
pub mod tip;
pub mod top_bar;
pub mod widgets;

use crate::App;
use gpui::{
    App as Cx, AppContext, Bounds, Global, TitlebarOptions, WindowBounds, WindowOptions, font, size,
};
use std::sync::Arc;
use theme::{Appearance, Theme, ThemeName};

pub struct Shared(pub Arc<App>);

impl Global for Shared {}

pub fn app(cx: &Cx) -> Arc<App> {
    cx.global::<Shared>().0.clone()
}

/// What the command line asks of the window.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub demo: bool,
    pub section: usize,
    pub theme: Option<ThemeName>,
    pub text_scale: Option<f32>,
    /// Four mods of the load order chosen, as `--several` asks.
    pub several: bool,
    /// The filter's text at the start, as `--filter=` gives it.
    pub filter: Option<String>,
    /// A row of the load order chosen, and the tab shown, as `--pick=` and `--tab=` give them.
    pub pick: Option<usize>,
    pub tab: usize,
    /// The scripted scroll of `--selftest`.
    pub bench: bool,
    /// Two toasts shown at the start, as `--notices` asks, for pictures.
    pub notices: bool,
    /// What is open at the start, as `--open=` names it, for pictures.
    pub open: Option<String>,
}

impl Options {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut options = Self::default();
        for arg in args {
            let (name, value) = arg.split_once('=').unwrap_or((&arg, ""));
            match name {
                "--demo" => options.demo = true,
                "--selftest" => (options.demo, options.bench) = (true, true),
                "--section" => options.section = value.parse().unwrap_or(0),
                "--theme" => options.theme = ThemeName::named(value),
                "--text-scale" => options.text_scale = value.parse().ok(),
                "--several" => options.several = true,
                "--filter" => options.filter = Some(value.to_string()),
                "--pick" => options.pick = value.parse().ok(),
                "--tab" => options.tab = value.parse().unwrap_or(0),
                "--notices" => options.notices = true,
                "--open" => options.open = Some(value.to_string()),
                _ => {}
            }
        }
        options
    }
}

pub use theme::WINDOW;

/// The weights fontconfig has for a family, as CSS numbers.
#[cfg(target_os = "linux")]
fn weights_of(family: &str) -> Vec<f32> {
    let mut asked = rimstack_core::base::program::command("fc-list");
    asked.args([family, "weight"]);
    let Ok(told) = rimstack_core::base::program::run(&mut asked, std::time::Duration::from_secs(2))
    else {
        return Vec::new();
    };
    // fontconfig's scale: 50 light, 80 regular, 100 medium, 180 semibold, 200 bold, 210 black.
    const SCALE: [(f32, f32); 9] = [
        (0., 100.),
        (40., 200.),
        (50., 300.),
        (80., 400.),
        (100., 500.),
        (180., 600.),
        (200., 700.),
        (205., 800.),
        (210., 900.),
    ];
    let mut found: Vec<f32> = String::from_utf8_lossy(&told.stdout)
        .lines()
        .filter_map(|line| line.trim().strip_prefix(":weight=")?.parse::<f32>().ok())
        .filter_map(|fc| SCALE.iter().find(|(at, _)| *at == fc).map(|(_, css)| *css))
        .collect();
    found.sort_by(f32::total_cmp);
    found.dedup();
    found
}

#[cfg(not(target_os = "linux"))]
fn weights_of(_family: &str) -> Vec<f32> {
    Vec::new()
}

/// The family Qt takes for its texts: the system's sans-serif.
fn system_family() -> String {
    system_family_for("sans-serif").unwrap_or_else(|| ".SystemUIFont".to_string())
}

/// The family fontconfig gives for a generic name, as Qt asks it.
fn system_family_for(generic: &str) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let mut asked = rimstack_core::base::program::command("fc-match");
        asked.args(["-f", "%{family[0]}", generic]);
        let told = rimstack_core::base::program::run(&mut asked, std::time::Duration::from_secs(2));
        if let Ok(told) = told
            && told.status.success()
            && let Ok(family) = String::from_utf8(told.stdout)
            && !family.trim().is_empty()
        {
            return Some(family.trim().to_string());
        }
    }
    let _ = generic;
    None
}

pub fn appearance(options: &Options) -> Appearance {
    let mut appearance = Appearance::default();
    if let Some(theme) = options.theme {
        appearance.theme = theme;
    }
    if let Some(scale) = options.text_scale {
        appearance.text_scale = scale;
    }
    appearance
}

/// Sets what every view reads: the `App` and the Theme.
pub fn install(app: Arc<App>, options: &Options, cx: &mut Cx) {
    let family = system_family();
    let weights = weights_of(&family);
    let mut theme = Theme::new(appearance(options), family).with_weights(&weights);
    if let Some(mono) = system_family_for("monospace") {
        theme.mono = mono.into();
    }

    let text = cx.text_system();
    let id = text.resolve_font(&font(theme.family.clone()));
    let at = theme::PROBE;
    theme.ascent = text.ascent(id, at) / at;
    theme.descent = text.descent(id, at).abs() / at;
    cx.set_global(theme);
    cx.set_global(Shared(app));
    text_field::bind_keys(cx);
    focus::bind_keys(cx);
    cx.bind_keys([gpui::KeyBinding::new(
        crate::command!("f"),
        mods::FocusFilter,
        None,
    )]);
}

pub fn run(app: Arc<App>, options: Options) {
    gpui_platform::application()
        .with_assets(assets::Assets)
        .run(move |cx: &mut Cx| {
            install(app, &options, cx);
            let bounds = Bounds::centered(None, size(WINDOW.0, WINDOW.1), cx);
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("RimStack".into()),
                        ..Default::default()
                    }),
                    app_id: Some("rimstack".into()),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| shell::Shell::new(&options, window, cx)),
            );
            if let Err(problem) = opened {
                rimstack_core::problem!("the window did not open: {problem}");
                cx.quit();
            }
            cx.activate(true);
        });
}

#[cfg(test)]
mod gates;
#[cfg(test)]
pub(crate) mod layout_checks;
#[cfg(test)]
mod widget_tests;
