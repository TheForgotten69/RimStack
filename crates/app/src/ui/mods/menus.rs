//! `RowMenu.qml` and the menus of a row's right click: a mod's, a tag group's, a category's;
//! each entry the load order gives runs here, or is asked of the window.

use super::Mods;
use crate::ui::menu::{self, Entry as Shown};
use crate::ui::notices;
use crate::ui::prompt::{self, Line, Question};
use crate::ui::text_field::TextField;
use crate::ui::theme::{CATEGORY_NAMES, SWATCHES, Theme};
use crate::words::put;
use crate::workspace::load_order::{Action, Clicked, Entry, Side};
use crate::workspace::upkeep::Line as AskLine;
use gpui::{AppContext, Context, Pixels, Point, Window};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::mods::library::ModId;
use rimstack_core::{say, say_n};

/// What a name typed is for.
#[derive(Clone, Copy)]
pub enum Naming {
    NewSub,
    Tag,
    Group,
}

/// Asked of the window: what the Downloads tab or the Workshop does for a mod.
pub enum Wanted {
    Subscribe(String),
    Redownload(i32),
    Unsubscribe(i32),
    UpdateFromGit(i32),
    /// The Workshop searched for this, as a fix that names a page asks.
    Browse(String),
}

impl gpui::EventEmitter<Wanted> for Mods {}

impl Mods {
    pub(super) fn open_folder(&mut self, mod_id: i32, cx: &mut Context<Self>) {
        let Some(id) = ModId::from_qml(mod_id) else {
            return;
        };
        let app = crate::ui::app(cx);
        let path = self.order.details(&app, id).path;
        if !path.is_empty() {
            cx.open_with_system(std::path::Path::new(&path));
        }
    }

    /// The menu of the mod at `index` of a list, the mod chosen first when it was not.
    pub(super) fn row_menu(
        &mut self,
        side: Side,
        index: i32,
        mod_id: i32,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        if !self.is_chosen(side, mod_id) {
            self.select(Some(mod_id), side == Side::Active, cx);
        }
        let app = crate::ui::app(cx);
        let selected: Vec<ModId> = self
            .selection
            .iter()
            .filter_map(|id| ModId::from_qml(*id))
            .collect();
        let clicked = Clicked {
            active: side == Side::Active,
            index,
            id: ModId::from_qml(mod_id),
            selected: &selected,
        };
        let entries = self.order.row_menu(&app, &clicked);
        self.open_entries(entries, at, window, cx);
    }

    pub(super) fn sub_menu(
        &mut self,
        side: Side,
        row: usize,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = self.order.sub_menu(side == Side::Active, row);
        self.open_entries(entries, at, window, cx);
    }

    /// A category's colour, among the swatches, or its own again.
    pub(super) fn category_menu(
        &mut self,
        category: i32,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(category) = usize::try_from(category) else {
            return;
        };
        let theme = cx.global::<Theme>().clone();
        let chosen = theme.appearance.swatch_of(category).map(str::to_string);
        let own = SWATCHES.get(category).map_or("", |s| s.0);
        let current = chosen.clone().unwrap_or_else(|| own.to_string());
        let paint = move |name: &'static str| {
            move |_: &mut Window, cx: &mut gpui::App| {
                let painted = cx.global::<Theme>().appearance.painted(category, name);
                crate::ui::keep_appearance("categoryColours", &painted, cx);
            }
        };
        let mut entries = vec![Shown::Title(
            put(&Sentence::said(
                CATEGORY_NAMES[category.min(CATEGORY_NAMES.len() - 1)],
                Vec::new(),
            ))
            .into(),
        )];
        for (name, _, _) in SWATCHES {
            let mut text = put(&Sentence::said(name, Vec::new()));
            if name == current {
                text += "  ✓";
            }
            let mut item = menu::Entry::item(text, paint(name));
            if let Some(colour) = theme.swatch(name) {
                item = item.colour(colour);
            }
            entries.push(item.into());
        }
        entries.push(Shown::Line);
        entries.push(
            menu::Entry::item(put(&say!("Its own colour")), paint(""))
                .icon("undo-2")
                .off(chosen.is_none())
                .into(),
        );
        menu::open_at(at, entries, window, cx);
    }

    fn open_entries(
        &mut self,
        entries: Vec<Entry>,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let me = cx.entity().downgrade();
        let shown: Vec<Shown> = entries
            .into_iter()
            .filter_map(|entry| {
                if let Some(title) = entry.title {
                    return Some(Shown::Title(put(&title).into()));
                }
                if entry.line {
                    return Some(Shown::Line);
                }
                let (text, action) = (entry.text?, entry.action?);
                let me = me.clone();
                let action = std::rc::Rc::new(action);
                let mut item = menu::Entry::item(put(&text), move |window, cx| {
                    let action = action.clone();
                    let _ = me.update(cx, |mods, cx| mods.run(&action, window, cx));
                })
                .off(entry.off);
                if !entry.icon.is_empty() {
                    item = item.icon(entry.icon);
                }
                if entry.danger {
                    item = item.danger();
                }
                Some(item.into())
            })
            .collect();
        menu::open_at(at, shown, window, cx);
    }

    /// What an entry of the load order's menus does, as `RowMenu.qml`'s `doers` did.
    fn run(&mut self, action: &Action, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            Action::MoveSelected => self.move_selected(cx),
            Action::MoveAcross { active, index } => self.take(Side::of(*active), *index, None, cx),
            Action::OpenFolder { path } => cx.open_with_system(std::path::Path::new(path)),
            Action::OpenUrl { url } => cx.open_url(url),
            Action::CopyText { value } => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()))
            }
            Action::ApplyFix { mod_id, fix } => {
                self.apply_fix(*mod_id, fix.action, &fix.target, fix.page.is_some(), cx)
            }
            Action::Subscribe { workshop_id } => cx.emit(Wanted::Subscribe(workshop_id.clone())),
            Action::CopyToLocal { ids, ask } => {
                let ids = ids.clone();
                let me = cx.entity().downgrade();
                ask_with(ask, false, window, cx, move |cx| {
                    let _ = me.update(cx, |mods, cx| mods.copy_to_local(ids, cx));
                });
            }
            Action::Redownload { mod_id, ask } => {
                let me = cx.entity().downgrade();
                let id = mod_id.to_qml();
                ask_with(ask, false, window, cx, move |cx| {
                    let _ = me.update(cx, |_, cx| cx.emit(Wanted::Redownload(id)));
                });
            }
            Action::Unsubscribe { mod_id } => self.ask_unsubscribe(mod_id.to_qml(), window, cx),
            Action::UpdateFromGit { mod_id } => cx.emit(Wanted::UpdateFromGit(mod_id.to_qml())),
            Action::PutInSub { mod_id, tag } => self.group(vec![*mod_id], tag, cx),
            Action::NewSub { mod_id } => self.ask_name(vec![*mod_id], Naming::NewSub, window, cx),
            Action::Trust { ids, yes } => self.trust(ids.clone(), *yes, cx),
            Action::Bin { name, path, mod_id } => self.ask_bin(name, path, *mod_id, window, cx),
            Action::ToggleSub { active, row } => self.toggle_sub(Side::of(*active), *row, cx),
            Action::RemoveSub { active, row } => self.remove_sub(Side::of(*active), *row, cx),
        }
    }

    /// A problem's fix: done here when it can be, else the Workshop asked or its page opened.
    pub(super) fn apply_fix(
        &mut self,
        mod_id: i32,
        action: &str,
        target: &str,
        page: bool,
        cx: &mut Context<Self>,
    ) {
        if self.order.previewing() {
            return;
        }
        let app = crate::ui::app(cx);
        match action {
            "activate" => {
                let loaded = app.library();
                let found = loaded.as_ref().as_ref().ok().and_then(|(library, _)| {
                    let list = self.order.list(Side::Inactive);
                    let id = list.mods().iter().filter_map(|m| m.installed).find(|id| {
                        library
                            .get(*id)
                            .and_then(|m| m.key())
                            .is_some_and(|k| k == target)
                    })?;
                    list.position_of(id)
                });
                drop(loaded);
                if let Some(at) = found {
                    self.take(Side::Inactive, at, None, cx);
                }
            }
            "deactivate" => {
                if let Ok(at) = target.parse::<usize>() {
                    self.take(Side::Active, at, None, cx);
                }
            }
            "trust" => {
                let ids = ModId::from_qml(mod_id).into_iter().collect();
                self.trust(ids, true, cx);
            }
            "sort" => self.preview_sort(cx),
            "subscribe" => {
                let (app, target) = (app.clone(), target.to_string());
                crate::jobs::run(
                    "add missing mod to the download list",
                    cx,
                    move || app.add_missing_to_list(&target).note(),
                    |_, note, cx| {
                        if let Some((said, failed)) = note {
                            let tone = if failed {
                                notices::Tone::Err
                            } else {
                                notices::Tone::Ok
                            };
                            notices::say(put(&said), tone, cx);
                        }
                    },
                );
            }
            _ if page => {
                cx.emit(Wanted::Browse(target.to_string()));
                cx.emit(super::ShowSection(3));
            }
            _ => cx.open_url(&crate::ui::workshop_page(target)),
        }
    }

    pub(super) fn group(&mut self, ids: Vec<ModId>, name: &str, cx: &mut Context<Self>) {
        let app = crate::ui::app(cx);
        app.set_group(&ids, name);
        self.order.sync(&app);
        self.show(cx);
    }

    pub(super) fn add_tag(&mut self, ids: Vec<ModId>, tag: &str, cx: &mut Context<Self>) {
        let app = crate::ui::app(cx);
        app.add_tag(&ids, tag);
        self.order.sync(&app);
        self.show(cx);
        let said = say_n!(
            ids.len(),
            "Tagged {} mod: {}",
            "Tagged {} mods: {}",
            tag.to_string()
        );
        notices::say(put(&said), notices::Tone::Ok, cx);
    }

    fn remove_sub(&mut self, side: Side, row: usize, cx: &mut Context<Self>) {
        let Some((tag, ids)) = self.order.group_on(side, row) else {
            return;
        };
        let app = crate::ui::app(cx);
        app.remove_tag(&ids, &tag);
        self.order.sync(&app);
        self.show(cx);
    }

    fn copy_to_local(&mut self, ids: Vec<ModId>, cx: &mut Context<Self>) {
        let app = crate::ui::app(cx);
        crate::jobs::run(
            "copy to the game folder",
            cx,
            move || {
                let copied = app.copy_to_local(&ids).err();
                app.rescan();
                copied
            },
            |mods, problem, cx| {
                mods.save_problem = problem
                    .map(|p| crate::words::said_of(&p))
                    .unwrap_or_default();
                cx.notify();
            },
        );
    }

    /// A name typed for the mods: a new tag group for one, or for the mods chosen a tag added
    /// to each (`Naming::Tag`) or one to group them under.
    pub(super) fn ask_name(
        &mut self,
        ids: Vec<ModId>,
        naming: Naming,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = cx.global::<Theme>().clone();
        let field = cx.new(|cx| TextField::new(theme.body(), cx));
        let (me, typed, count) = (cx.entity().downgrade(), field.clone(), ids.len());
        let (title, label) = match naming {
            Naming::NewSub => (put(&say!("Group under a new tag")), put(&say!("Create"))),
            Naming::Tag => (
                put(&say_n!(count, "Tag for {} mod", "Tag for {} mods")),
                put(&say!("Apply")),
            ),
            Naming::Group => (
                put(&say_n!(
                    count,
                    "Tag to group under, for {} mod",
                    "Tag to group under, for {} mods"
                )),
                put(&say!("Apply")),
            ),
        };
        prompt::ask(
            Question {
                title,
                lines: vec![Line::Field {
                    text: field,
                    placeholder: put(&say!("Name")),
                }],
                label,
                danger: false,
                run: Box::new(move |_, cx| {
                    let name = typed.read(cx).text().trim().to_string();
                    let _ = me.update(cx, |mods, cx| match naming {
                        Naming::Tag => mods.add_tag(ids, &name, cx),
                        Naming::NewSub | Naming::Group => mods.group(ids, &name, cx),
                    });
                }),
            },
            window,
            cx,
        );
    }

    fn ask_bin(
        &mut self,
        name: &str,
        folder: &str,
        id: ModId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let me = cx.entity().downgrade();
        prompt::ask(
            Question {
                title: put(&say!("Move to the bin: {}", name.to_string())),
                lines: vec![
                    Line::Path(folder.to_string()),
                    Line::Text(put(&say!(
                        "Steam downloads a Workshop mod again while you stay subscribed to it."
                    ))),
                ],
                label: put(&say!("Move to the bin")),
                danger: true,
                run: Box::new(move |_, cx| {
                    let _ = me.update(cx, |mods, cx| {
                        let app = crate::ui::app(cx);
                        let binned = app.bin_mod(id);
                        mods.save_problem = crate::words::why_not(&binned);
                        if binned.is_ok() {
                            crate::jobs::spawn("rescan", move || app.rescan());
                        }
                        cx.notify();
                    });
                }),
            },
            window,
            cx,
        );
    }

    fn ask_unsubscribe(&mut self, mod_id: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = ModId::from_qml(mod_id) else {
            return;
        };
        let app = crate::ui::app(cx);
        let about = self.order.details(&app, id);
        let me = cx.entity().downgrade();
        prompt::ask(
            Question {
                title: put(&say!("Unsubscribe from {}", about.name.clone())),
                lines: vec![
                    Line::Text(put(&say!(
                        "Steam must be running. It unsubscribes and then deletes the mod's folder:"
                    ))),
                    Line::Path(about.path.clone()),
                ],
                label: put(&say!("Unsubscribe")),
                danger: true,
                run: Box::new(move |_, cx| {
                    let _ = me.update(cx, |_, cx| cx.emit(Wanted::Unsubscribe(mod_id)));
                }),
            },
            window,
            cx,
        );
    }
}

/// A question the load order wrote, its answer `then`.
fn ask_with(
    ask: &crate::workspace::load_order::Ask,
    danger: bool,
    window: &mut Window,
    cx: &mut Context<Mods>,
    then: impl FnOnce(&mut gpui::App) + 'static,
) {
    let lines = ask
        .lines
        .iter()
        .map(|line| match line {
            AskLine::Said(said) => Line::Text(put(said)),
            AskLine::Path { path } => Line::Path(path.clone()),
        })
        .collect();
    prompt::ask(
        Question {
            title: put(&ask.title),
            lines,
            label: put(&ask.label),
            danger,
            run: Box::new(move |_, cx| then(cx)),
        },
        window,
        cx,
    );
}
