//! The window: views that show what the `App` hands them and forward what is done.

pub mod assets;
pub mod button;
pub mod card;
pub mod command_list;
pub mod commands;
pub mod controls;
pub mod downloads;
pub mod field;
pub mod find;
pub mod first_run;
pub mod focus;
pub mod game;
pub mod hunt;
pub mod keys;
pub mod lists;
pub mod logs;
pub mod menu;
pub mod mods;
pub mod notices;
pub mod page;
pub mod play;
#[cfg(all(test, target_os = "linux"))]
mod play_tests;
pub mod problem;
pub mod prompt;
pub mod rich;
pub mod rows;
pub mod saves;
pub mod settings;
pub mod shell;
pub mod status_bar;
pub mod text_field;
pub mod textures;
pub mod theme;
pub mod thumbs;
pub mod tip;
pub mod top_bar;
pub mod widgets;
pub mod workshop;

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
    /// The first two active mods that share textures compared, as `--comparing` asks.
    pub comparing: bool,
    /// The inactive mods' drawer open, when they are folded into the rail.
    pub drawer: bool,
    /// Sort's preview shown once the files are read, as `--sorting` asks.
    pub sorting: bool,
    /// Typed into the Mods filter, which has the keys, as `--typing=` asks.
    pub typing: Option<String>,
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
    /// The part of Settings shown, as `--part=` gives it.
    pub part: usize,
    /// The view of Logs shown, as `--view=` gives it.
    pub view: usize,
    /// A search made at the start, as `--find=` gives its text, for pictures.
    pub find: Option<String>,
    /// What the Workshop shows at the start, as `--browse=` names it: `list`, `details` or
    /// `filters`, for pictures.
    pub browse: Option<String>,
    /// For a picture: the disk is not watched, so nothing changes while it is taken.
    pub still: bool,
    /// The first run shown, as `--first` asks, on the step `--step=` gives.
    pub first: bool,
    pub step: usize,
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
                "--comparing" => options.comparing = true,
                "--drawer" => options.drawer = true,
                "--sorting" => options.sorting = true,
                "--typing" => options.typing = Some(value.to_string()),
                "--filter" => options.filter = Some(value.to_string()),
                "--pick" => options.pick = value.parse().ok(),
                "--tab" => options.tab = value.parse().unwrap_or(0),
                "--notices" => options.notices = true,
                "--open" => options.open = Some(value.to_string()),
                "--part" => options.part = value.parse().unwrap_or(0),
                "--view" => options.view = value.parse().unwrap_or(0),
                "--find" => options.find = Some(value.to_string()),
                "--browse" => options.browse = Some(value.to_string()),
                "--first" => options.first = true,
                "--step" => options.step = value.parse().unwrap_or(0),
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

/// An item's page on the Workshop, as `Theme.openWorkshop` opened it.
pub fn workshop_page(id: &str) -> String {
    format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}")
}

/// An item's change notes on the Workshop, as `Theme.openChangeNotes` opened them.
pub fn change_notes(id: &str) -> String {
    format!("https://steamcommunity.com/sharedfiles/filedetails/changelog/{id}")
}

/// `f` on the view, as a control's callback that is given a value.
pub fn act<V: 'static, T: 'static>(
    cx: &gpui::Context<V>,
    f: impl Fn(&mut V, T, &mut gpui::Window, &mut gpui::Context<V>) + 'static,
) -> impl Fn(T, &mut gpui::Window, &mut Cx) + 'static {
    let me = cx.entity().downgrade();
    move |value, window, cx| {
        let _ = me.update(cx, |view, cx| f(view, value, window, cx));
    }
}

/// The appearance kept in `RimStack.conf`, with the theme's own colours when they are used.
fn kept_appearance(app: &App) -> Appearance {
    Appearance::of(&app.prefs(), |theme| {
        let own = app.own_colours(theme);
        own.into_iter()
            .filter_map(|(key, value)| Some((key, value.as_str()?.to_string())))
            .collect()
    })
}

/// Draws every window again in the appearance kept now.
pub fn retheme(cx: &mut Cx) {
    let appearance = kept_appearance(&app(cx));
    let theme = cx.global::<Theme>().with_appearance(appearance);
    cx.set_global(theme);
    cx.refresh_windows();
}

/// Keeps a preference of `[Appearance]` and shows the window in it.
pub fn keep_appearance(key: &str, value: &str, cx: &mut Cx) {
    match app(cx).keep_pref("Appearance", key, value) {
        Ok(()) => retheme(cx),
        Err(problem) => rimstack_core::problem!("{problem}"),
    }
}

/// The kept appearance, but for what the command line asks for a picture.
pub fn appearance(app: &App, options: &Options) -> Appearance {
    let mut appearance = kept_appearance(app);
    if let Some(theme) = options.theme {
        appearance.theme = theme;
    }
    if let Some(scale) = options.text_scale {
        appearance.text_scale = scale;
    }
    appearance
}

/// `theme` in `family`, the system's when none: its weights, and its ascent and descent read.
fn in_family(theme: Theme, family: Option<&str>, cx: &Cx) -> Theme {
    let family = family.map_or_else(system_family, str::to_string);
    let weights = weights_of(&family);
    let mut theme = Theme::new(theme.appearance.clone(), family).with_weights(&weights);
    let text = cx.text_system();
    let id = text.resolve_font(&font(theme.family.clone()));
    let at = theme::PROBE;
    theme.ascent = text.ascent(id, at) / at;
    theme.descent = text.descent(id, at).abs() / at;
    theme
}

/// Every text in `family` from now on; the system's font when it is empty.
pub fn use_font(family: &str, cx: &mut Cx) {
    let old = cx.global::<Theme>().clone();
    let mut theme = in_family(old.clone(), (!family.is_empty()).then_some(family), cx);
    theme.mono = old.mono;
    cx.set_global(theme);
    cx.refresh_windows();
}

/// Sets what every view reads: the `App` and the Theme.
pub fn install(app: Arc<App>, options: &Options, cx: &mut Cx) {
    let chosen = app
        .settings()
        .get("font")
        .map(str::to_string)
        .filter(|f| !f.is_empty());
    let mut theme = in_family(
        Theme::new(appearance(&app, options), ""),
        chosen.as_deref(),
        cx,
    );
    if let Some(mono) = system_family_for("monospace") {
        theme.mono = mono.into();
    }
    cx.set_global(theme);
    cx.set_global(Shared(app));
    text_field::bind_keys(cx);
    focus::bind_keys(cx);
    keys::bind_keys(cx);
    cx.bind_keys([
        gpui::KeyBinding::new(crate::command!("f"), mods::FocusFilter, None),
        gpui::KeyBinding::new(crate::command!("z"), mods::Undo, None),
        gpui::KeyBinding::new(crate::command!("shift-z"), mods::Redo, None),
        gpui::KeyBinding::new(crate::command!("s"), mods::Save, None),
    ]);
    cx.bind_keys([gpui::KeyBinding::new(
        crate::command!("shift-f"),
        find::FocusSearch,
        None,
    )]);
}

/// Where the window opens: where it was last closed, kept inside the screens there are now.
fn window_start(app: &App, cx: &Cx) -> WindowBounds {
    use rimstack_core::base::window::Area;
    let screens: Vec<Area> = cx
        .displays()
        .iter()
        .map(|display| {
            let b = display.bounds();
            Area {
                x: f32::from(b.origin.x) as i32,
                y: f32::from(b.origin.y) as i32,
                width: f32::from(b.size.width) as i32,
                height: f32::from(b.size.height) as i32,
            }
        })
        .collect();
    let (area, maximized) = app.window_start(&screens);
    let bounds = Bounds::new(
        gpui::point(gpui::px(area.x as f32), gpui::px(area.y as f32)),
        size(gpui::px(area.width as f32), gpui::px(area.height as f32)),
    );
    match maximized {
        true => WindowBounds::Maximized(bounds),
        false => WindowBounds::Windowed(bounds),
    }
}

/// The window's place kept as it closes, for its next start.
pub(crate) fn keep_window(window: &gpui::Window, cx: &Cx) {
    use rimstack_core::base::window::Area;
    let (bounds, maximized) = match window.window_bounds() {
        WindowBounds::Windowed(bounds) => (bounds, false),
        WindowBounds::Maximized(bounds) | WindowBounds::Fullscreen(bounds) => (bounds, true),
    };
    let area = Area {
        x: f32::from(bounds.origin.x) as i32,
        y: f32::from(bounds.origin.y) as i32,
        width: f32::from(bounds.size.width) as i32,
        height: f32::from(bounds.size.height) as i32,
    };
    if let Err(problem) = app(cx).keep_window(area, maximized) {
        rimstack_core::problem!("{problem}");
    }
}

pub fn run(app: Arc<App>, options: Options) {
    gpui_platform::application()
        .with_assets(assets::Assets)
        .run(move |cx: &mut Cx| {
            let started = window_start(&app, cx);
            install(app, &options, cx);
            let least = {
                let theme = cx.global::<theme::Theme>();
                size(theme.sized(1000.), theme.sized(560.))
            };
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(started),
                    window_min_size: Some(least),
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
#[cfg(all(test, target_os = "linux"))]
pub(crate) mod made_up;
#[cfg(test)]
mod widget_tests;
