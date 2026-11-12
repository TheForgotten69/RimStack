//! The window: views that show what the `App` hands them and forward what is done.

pub mod shell;
pub mod theme;

use crate::App;
use gpui::{
    App as Cx, AppContext, Bounds, Global, TitlebarOptions, WindowBounds, WindowOptions, font, px,
    size,
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
}

impl Options {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut options = Self::default();
        for arg in args {
            let (name, value) = arg.split_once('=').unwrap_or((&arg, ""));
            match name {
                "--demo" | "--selftest" => options.demo = true,
                "--section" => options.section = value.parse().unwrap_or(0),
                "--theme" => options.theme = ThemeName::named(value),
                "--text-scale" => options.text_scale = value.parse().ok(),
                _ => {}
            }
        }
        options
    }
}

pub const WINDOW: (f32, f32) = (1440., 860.);

/// The family Qt takes for its texts: the system's sans-serif.
fn system_family() -> String {
    #[cfg(target_os = "linux")]
    {
        let mut asked = rimstack_core::base::program::command("fc-match");
        asked.args(["-f", "%{family[0]}", "sans-serif"]);
        let told = rimstack_core::base::program::run(&mut asked, std::time::Duration::from_secs(2));
        if let Ok(told) = told
            && told.status.success()
            && let Ok(family) = String::from_utf8(told.stdout)
            && !family.trim().is_empty()
        {
            return family.trim().to_string();
        }
    }
    ".SystemUIFont".to_string()
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
    let mut theme = Theme::new(appearance(options), system_family());
    let text = cx.text_system();
    let id = text.resolve_font(&font(theme.family.clone()));
    let at = px(1000.);
    theme.ascent = f32::from(text.ascent(id, at)) / 1000.;
    theme.descent = f32::from(text.descent(id, at)).abs() / 1000.;
    cx.set_global(theme);
    cx.set_global(Shared(app));
}

pub fn run(app: Arc<App>, options: Options) {
    gpui_platform::application().run(move |cx: &mut Cx| {
        install(app, &options, cx);
        let bounds = Bounds::centered(None, size(px(WINDOW.0), px(WINDOW.1)), cx);
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
