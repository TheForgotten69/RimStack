//! What Ctrl + K offers, as `Main.qml` listed it: the settings, the sections, the load order's
//! commands, the chosen mod's, filters and folders; and the mods the text names.

use super::commands::{Command, Commands, Run};
use super::lists::Which;
use super::shell::Shell;
use super::theme::{Theme, ThemeName};
use crate::words::put;
use gpui::{AppContext, Bounds, Context, Window, point, size};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::say;
use std::rc::Rc;

/// Qt's `Theme.issues`.
const ISSUES: &str = "https://github.com/TheForgotten69/RimStack/issues";

fn command(name: String, group: &Sentence, keys: String, run: Run) -> Command {
    Command {
        name,
        group: put(group),
        keys,
        run,
    }
}

impl Shell {
    pub(crate) fn open_commands(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let commands = self.commands_now(cx);
        let shell = cx.entity().downgrade();
        self.commands = Some(cx.new(|cx| Commands::new(shell, commands, window, cx)));
        cx.notify();
    }

    pub(crate) fn close_commands(&mut self, cx: &mut Context<Self>) {
        self.commands = None;
        cx.notify();
    }

    /// The mods `text` names, to show one.
    pub(crate) fn found_mods(&self, text: &str, cx: &gpui::App) -> Vec<Command> {
        let found = self.mods.read(cx).found(text, 12, cx);
        found
            .into_iter()
            .map(|found| {
                let group = match found.active {
                    true => say!("Active mod"),
                    false => say!("Inactive mod"),
                };
                let id = found.mod_id;
                command(
                    found.name,
                    &group,
                    String::new(),
                    Rc::new(move |shell, _, cx| {
                        shell.go(0, cx);
                        shell.mods.update(cx, |mods, cx| mods.show_mod(id, cx));
                    }),
                )
            })
            .collect()
    }

    fn commands_now(&self, cx: &gpui::App) -> Vec<Command> {
        let demo = self.demo;
        let app = super::app(cx);
        let folders = (!demo).then(|| app.settings_report());
        let mods = self.mods.read(cx);
        let mut all = Vec::new();
        let setting = say!("Setting");
        let settings: [(Sentence, usize); 27] = [
            (say!("RimWorld folder"), 0),
            (say!("Config and saves folder"), 0),
            (say!("Workshop mods folder"), 0),
            (say!("Play starts the game"), 0),
            (say!("Launch arguments"), 0),
            (say!("Git for Workshop mods"), 0),
            (say!("Theme"), 1),
            (say!("Language"), 1),
            (say!("Colours"), 1),
            (say!("Font"), 1),
            (say!("Text size"), 1),
            (say!("Mod rows"), 1),
            (say!("Motion"), 1),
            (say!("Contrast"), 1),
            (say!("First-run hints"), 1),
            (say!("Workshop pages open in"), 1),
            (say!("Community databases"), 2),
            (say!("Game version mismatch"), 2),
            (say!("Patch and def hints"), 2),
            (say!("Mods marked as working"), 2),
            (say!("Steam Web API key"), 3),
            (say!("Show mods Steam marks as mature"), 3),
            (say!("Verify game files with Steam"), 4),
            (say!("Set mod settings aside"), 4),
            (say!("List from before RimStack"), 4),
            (say!("RimStack's data folder"), 4),
            (say!("Version and licence"), 4),
        ];
        for (name, part) in settings {
            all.push(command(
                put(&name),
                &setting,
                String::new(),
                Rc::new(move |shell, _, cx| shell.show_settings(part, cx)),
            ));
        }
        let help = say!("Help");
        let game = say!("Game");
        let order = say!("Load order");
        let go_to = say!("Go to");
        let view = say!("View");
        let open = say!("Open");
        let filter = say!("Filter");
        let chosen = say!("Selected mod");
        let session = (!demo).then(|| {
            let session = app.session();
            (session.is_dirty(), session.can_undo(), session.can_redo())
        });
        let (dirty, can_undo, can_redo) = session.unwrap_or_default();
        let (errors, warnings) = mods.problems();
        let mut push = |when: bool, name: Sentence, group: &Sentence, keys: Sentence, run: Run| {
            if when {
                all.push(command(put(&name), group, put(&keys), run));
            }
        };
        let none = Sentence::Plain(String::new());
        push(
            true,
            say!("Keyboard shortcuts"),
            &help,
            Sentence::Plain("?".into()),
            Rc::new(|shell, window, cx| shell.open_keys(window, cx)),
        );
        push(
            true,
            say!("Report a problem"),
            &help,
            none.clone(),
            Rc::new(|_, _, cx| cx.open_url(ISSUES)),
        );
        push(
            !demo && !self.hunt.active,
            say!("Find the broken mod"),
            &game,
            none.clone(),
            Rc::new(|shell, _, cx| shell.start_hunt(cx)),
        );
        push(
            errors + warnings > 0,
            say!("All problems"),
            &order,
            none.clone(),
            Rc::new(|shell, _, cx| {
                shell.go(0, cx);
                shell.mods.update(cx, |mods, cx| mods.open_problems(cx));
            }),
        );
        push(
            !demo,
            say!("What RimStack changed"),
            &help,
            none.clone(),
            Rc::new(|shell, window, cx| shell.open_popup(Which::History, window, cx)),
        );
        push(
            !demo,
            say!("Sort"),
            &order,
            none.clone(),
            Rc::new(|shell, _, cx| {
                shell.go(0, cx);
                shell.mods.update(cx, |mods, cx| mods.preview_sort(cx));
            }),
        );
        push(
            dirty,
            say!("Save"),
            &order,
            say!("Ctrl + S"),
            Rc::new(|shell, _, cx| shell.mods.update(cx, |mods, cx| mods.save(None, cx))),
        );
        push(
            can_undo,
            say!("Undo"),
            &order,
            say!("Ctrl + Z"),
            Rc::new(|shell, _, cx| shell.mods.update(cx, |mods, cx| mods.undo(cx))),
        );
        push(
            can_redo,
            say!("Redo"),
            &order,
            say!("Ctrl + Shift + Z"),
            Rc::new(|shell, _, cx| shell.mods.update(cx, |mods, cx| mods.redo(cx))),
        );
        push(
            true,
            say!("Play"),
            &game,
            none.clone(),
            Rc::new(|shell, _, cx| shell.press_play(cx)),
        );
        push(
            errors + warnings > 0,
            say!("Next problem"),
            &order,
            none.clone(),
            Rc::new(|shell, _, cx| {
                shell.go(0, cx);
                shell.mods.update(cx, |mods, cx| mods.next_problem(cx));
            }),
        );
        push(
            !demo,
            say!("Saved lists"),
            &order,
            none.clone(),
            Rc::new(|shell, window, cx| shell.open_popup(Which::Lists, window, cx)),
        );
        let backup = folders.as_ref().is_some_and(|f| f.backup.is_some());
        push(
            backup,
            say!("Use the list from before RimStack"),
            &order,
            none.clone(),
            Rc::new(|shell, _, cx| {
                if let Some(ids) = super::app(cx).original_list() {
                    shell.go(0, cx);
                    shell.mods.update(cx, |mods, cx| mods.use_list(ids, cx));
                }
            }),
        );
        if let Some(about) = mods.chosen_details().filter(|_| mods.selected_count() == 1) {
            let name = about.name.clone();
            let toggle = match mods.selected_on_active() {
                true => say!("Deactivate {}", name.clone()),
                false => say!("Activate {}", name.clone()),
            };
            push(
                !mods.previewing(),
                toggle,
                &chosen,
                none.clone(),
                Rc::new(|shell, _, cx| shell.mods.update(cx, |mods, cx| mods.toggle_selected(cx))),
            );
            let path = about.path.clone();
            push(
                !path.is_empty(),
                say!("Open folder of {}", name.clone()),
                &chosen,
                none.clone(),
                Rc::new(move |_, _, cx| cx.open_with_system(std::path::Path::new(&path))),
            );
            let folder = about.folder.clone();
            push(
                about.workshop,
                say!("Workshop page of {}", name.clone()),
                &chosen,
                none.clone(),
                Rc::new(move |_, _, cx| cx.open_url(&super::workshop_page(&folder))),
            );
            let package = about.package_id.clone();
            push(
                !package.is_empty(),
                say!("Copy package id of {}", name),
                &chosen,
                none.clone(),
                Rc::new(move |_, _, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(package.clone()))
                }),
            );
        }
        push(
            mods.selected_id().is_some(),
            say!("Clear selection"),
            &chosen,
            say!("Esc"),
            Rc::new(|shell, _, cx| {
                shell
                    .mods
                    .update(cx, |mods, cx| mods.select(None, true, cx))
            }),
        );
        push(
            !demo,
            say!("Use the list on the clipboard"),
            &order,
            none.clone(),
            Rc::new(|shell, _, cx| shell.lists.update(cx, |lists, cx| lists.use_copied(cx))),
        );
        push(
            true,
            say!("Copy the load order as text"),
            &order,
            none.clone(),
            Rc::new(|shell, _, cx| {
                let listing = shell.mods.read(cx).listing(cx);
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(listing));
            }),
        );
        let filtered = |text: &'static str| -> Run {
            Rc::new(move |shell, _, cx| {
                shell.go(0, cx);
                shell.mods.update(cx, |mods, cx| mods.filter_by(text, cx));
            })
        };
        push(
            errors > 0,
            say!("Mods with errors"),
            &filter,
            none.clone(),
            filtered("is:error"),
        );
        push(
            warnings > 0,
            say!("Mods with warnings"),
            &filter,
            none.clone(),
            filtered("is:warning"),
        );
        push(
            mods.conflicts() > 0,
            say!("Mods sharing textures"),
            &filter,
            none.clone(),
            filtered("is:conflict"),
        );
        push(
            self.behind > 0,
            say!("Mods with an update"),
            &filter,
            none.clone(),
            filtered("is:updated"),
        );
        push(
            true,
            say!("Mods with code"),
            &filter,
            none.clone(),
            filtered("has:code"),
        );
        push(
            true,
            say!("Local mods"),
            &filter,
            none.clone(),
            filtered("is:local"),
        );
        push(
            true,
            say!("Clear filter"),
            &filter,
            none.clone(),
            filtered(""),
        );
        push(
            !demo,
            say!("Read the mod folders again"),
            &say!("Mods"),
            none.clone(),
            Rc::new(|_, _, cx| {
                let app = super::app(cx);
                crate::jobs::spawn("rescan", move || app.rescan());
            }),
        );
        push(
            !demo,
            say!("Check for updates"),
            &say!("Downloads"),
            none.clone(),
            Rc::new(|shell, _, cx| {
                shell.go(1, cx);
                let downloads = shell.downloads(cx);
                downloads.update(cx, |downloads, cx| downloads.check_updates(cx));
            }),
        );
        push(
            !demo,
            say!("Browse the Workshop"),
            &go_to,
            none.clone(),
            Rc::new(|shell, _, cx| shell.go(3, cx)),
        );
        let section = |at: usize| -> Run { Rc::new(move |shell, _, cx| shell.go(at, cx)) };
        push(true, say!("Mods"), &go_to, say!("Ctrl + {}", 1), section(0));
        push(
            true,
            say!("Workshop"),
            &go_to,
            say!("Ctrl + {}", 4),
            section(3),
        );
        push(
            true,
            say!("Downloads"),
            &go_to,
            say!("Ctrl + {}", 2),
            section(1),
        );
        push(
            true,
            say!("Textures"),
            &go_to,
            say!("Ctrl + {}", 3),
            section(2),
        );
        push(
            true,
            say!("Saves"),
            &go_to,
            say!("Ctrl + {}", 5),
            section(4),
        );
        push(
            true,
            say!("Search mods' files"),
            &go_to,
            say!("Ctrl + Shift + F"),
            Rc::new(|shell, window, cx| {
                shell.go(6, cx);
                let find = shell.find(cx);
                find.update(cx, |find, cx| find.focus_search(window, cx));
            }),
        );
        for (name, view_at) in [
            (say!("Logs: errors"), 0),
            (say!("Logs: mods"), 1),
            (say!("Logs: full log"), 2),
        ] {
            push(
                true,
                name,
                &go_to,
                none.clone(),
                Rc::new(move |shell, _, cx| shell.show_logs(view_at, cx)),
            );
        }
        for (name, part) in [
            (say!("Settings: game and folders"), 0),
            (say!("Settings: appearance"), 1),
            (say!("Settings: sorting"), 2),
            (say!("Settings: RimStack"), 4),
        ] {
            push(
                true,
                name,
                &go_to,
                none.clone(),
                Rc::new(move |shell, _, cx| shell.show_settings(part, cx)),
            );
        }
        let current = cx.global::<Theme>().appearance.theme;
        for (name, theme) in [
            (say!("Theme: dark"), ThemeName::Dark),
            (say!("Theme: OLED"), ThemeName::Oled),
            (say!("Theme: light"), ThemeName::Light),
            (say!("Theme: Rim"), ThemeName::Rim),
        ] {
            push(
                current != theme,
                name,
                &view,
                none.clone(),
                Rc::new(move |_, _, cx| super::keep_appearance("theme", theme.name(), cx)),
            );
        }
        let comfortable = cx.global::<Theme>().appearance.comfortable;
        let rows = match comfortable {
            true => say!("Compact rows"),
            false => say!("Comfortable rows"),
        };
        push(
            true,
            rows,
            &view,
            none.clone(),
            Rc::new(move |_, _, cx| {
                super::keep_appearance(
                    "comfortable",
                    if comfortable { "false" } else { "true" },
                    cx,
                )
            }),
        );
        let opened = |path: Option<String>| -> Option<Run> {
            let path = path.filter(|p| !p.is_empty())?;
            Some(Rc::new(move |_, _, cx| {
                cx.open_with_system(std::path::Path::new(&path))
            }))
        };
        if let Some(folders) = &folders {
            for (name, path) in [
                (say!("RimWorld folder"), folders.game.clone()),
                (say!("Workshop mods folder"), folders.workshop.clone()),
                (
                    say!("Config and saves folder"),
                    folders.config_folder.clone(),
                ),
                (say!("Game log"), folders.game_log.clone()),
                (
                    say!("RimStack data folder"),
                    Some(folders.data_folder.clone()),
                ),
            ] {
                if let Some(run) = opened(path) {
                    push(true, name, &open, none.clone(), run);
                }
            }
        }
        push(
            true,
            say!("RimWorld store page"),
            &open,
            none,
            Rc::new(|_, _, cx| cx.open_url("steam://store/294100")),
        );
        all
    }

    /// The Lists or History popup opened from where its button is.
    pub(crate) fn open_popup(&mut self, which: Which, window: &mut Window, cx: &mut Context<Self>) {
        let theme = cx.global::<Theme>().clone();
        let viewport = window.viewport_size();
        let below = match which {
            Which::Lists => Bounds::new(
                point(theme.sized(214.), theme.top_bar() - theme.control()),
                size(theme.sized(120.), theme.control()),
            ),
            Which::History => Bounds::new(
                point(
                    viewport.width - theme.sized(130.),
                    viewport.height - theme.strip(),
                ),
                size(theme.sized(120.), theme.strip()),
            ),
        };
        self.lists.update(cx, |lists, cx| {
            if !lists.is_open(which) {
                lists.toggle(which, below, window, cx)
            }
        });
    }

    pub(crate) fn show_settings(&mut self, part: usize, cx: &mut Context<Self>) {
        self.part = part;
        self.go(7, cx);
        if let Some(settings) = &self.settings {
            settings.update(cx, |settings, cx| settings.show_part(part, cx));
        }
    }

    pub(crate) fn show_logs(&mut self, view: usize, cx: &mut Context<Self>) {
        self.view = view;
        self.go(5, cx);
        if let Some(logs) = &self.logs {
            logs.update(cx, |logs, cx| logs.show_view(view, cx));
        }
    }
}
