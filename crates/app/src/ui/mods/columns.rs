//! The View menu: the load order's columns, their widths and order, the
//! headings, and how each list is ordered, as `[Mods]` keeps them.

use super::Mods;
use super::list::bar_room;
use super::rows::Columns;
use crate::ui::menu::{self, Entry};
use crate::ui::notices;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{btn_width, measure};
use crate::words::put;
use crate::workspace::ModsView;
use crate::workspace::load_order::{OrderEntry, Orders, Side};
use gpui::{
    AppContext, Bounds, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Pixels, Point, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, px,
};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::mods::columns as kept;
use rimstack_core::mods::library::ModId;
use rimstack_core::{say, say_n};
use std::cell::Cell;
use std::rc::Rc;

/// Where the View and the inactive order buttons were last drawn.
#[derive(Default, Clone)]
pub struct Placed {
    pub view: Rc<Cell<Bounds<Pixels>>>,
    pub order: Rc<Cell<Bounds<Pixels>>>,
}

/// `control` with where it is drawn kept in `at`.
pub(super) fn placed(control: impl IntoElement, at: Rc<Cell<Bounds<Pixels>>>) -> gpui::Div {
    div().relative().child(control).child(
        canvas(move |bounds, _, _| at.set(bounds), |_, _, _, _| {})
            .absolute()
            .inset_0(),
    )
}

/// What Tab reaches beside the lists, by name: a column's heading, a divider. Each keeps its
/// keys' handle and where it was last drawn, which a menu opened by a key is put under.
pub(super) type Reachable =
    std::collections::HashMap<&'static str, (gpui::FocusHandle, Rc<Cell<Bounds<Pixels>>>)>;

/// A column's left edge dragged to size it.
#[derive(Clone, Copy)]
pub struct ColumnEdge(pub &'static str);

/// A column's heading dragged to move it: its key and its place among those shown.
#[derive(Clone, Copy)]
pub struct ColumnHeld(pub &'static str, pub usize);

/// The line left of a column's heading, and the room either side of it that sizes the column.
pub(super) fn edge(
    theme: &Theme,
    key: &'static str,
    width: Pixels,
    cx: &mut Context<Mods>,
) -> gpui::Stateful<gpui::Div> {
    let c = &theme.colours;
    let group = SharedString::from(format!("edge-{key}"));
    let focus = c.focus;
    let tip = put(&say!("Drag to size the column"));
    crate::ui::tip::tipped(
        div()
            .id(group.clone())
            .group(group.clone())
            .absolute()
            .left(-(theme::GAP_LARGE / 2.) - theme::GAP_TINY)
            .top_0()
            .bottom_0()
            .w(theme::GAP_TINY * 2. + px(1.))
            .flex()
            .justify_center()
            .cursor_col_resize()
            .child(
                div()
                    .mt(px(2.))
                    .mb(px(2.))
                    .w(px(1.))
                    .h_full()
                    .bg(c.field_line)
                    .group_hover(group, move |s| s.w(px(2.)).bg(focus)),
            )
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |mods, event: &gpui::MouseDownEvent, _, cx| {
                    mods.column_drag = Some((event.position.x, width));
                    cx.stop_propagation();
                }),
            )
            .on_drag(ColumnEdge(key), |_, _, _, cx| {
                cx.new(|_| super::rows::Ghost)
            }),
        tip.into(),
    )
}

/// Every column the View menu offers, in its order.
const ALL: [&str; 7] = [
    "author",
    "version",
    "load",
    "updated",
    "downloaded",
    "activated",
    "size",
];
/// The order columns are left out in when the name would have too little room.
const DROPPED_FIRST: [&str; 7] = [
    "size",
    "downloaded",
    "activated",
    "updated",
    "load",
    "author",
    "version",
];

pub(super) fn column_name(key: &str) -> Sentence {
    match key {
        "author" => say!("Author"),
        "version" => say!("Version"),
        "load" => say!("Load"),
        "updated" => say!("Updated"),
        "downloaded" => say!("Downloaded"),
        "activated" => say!("Activated"),
        _ => say!("Size"),
    }
}

/// The widest "… ago" a time column shows.
fn ago_widest(window: &Window, theme: &Theme) -> Pixels {
    [
        put(&say_n!(59, "{} minute ago", "{} minutes ago")),
        put(&say_n!(47, "{} hour ago", "{} hours ago")),
        put(&say_n!(59, "{} day ago", "{} days ago")),
        put(&say_n!(11, "{} month ago", "{} months ago")),
        put(&say_n!(99, "{} year ago", "{} years ago")),
    ]
    .iter()
    .map(|text| measure(window, theme, text, theme.body(), FontWeight::NORMAL))
    .fold(px(0.), Pixels::max)
}

/// `Columns.widths` and `ModList.fitting` for a load order this wide.
pub(super) fn columns(
    theme: &Theme,
    orders: &Orders,
    view: &ModsView,
    width: Pixels,
    window: &Window,
) -> Columns {
    let heading = |key: &str| {
        let text = format!("{} ▾", put(&column_name(key)));
        measure(window, theme, &text, theme.caption(), theme.medium)
    };
    let widest = |key: &str, size: Pixels| {
        let text = orders.widest.get(key).cloned().unwrap_or_default();
        measure(window, theme, &text, size, FontWeight::NORMAL)
    };
    let sized = kept::widths(&view.column_sizes);
    let column_width = |key: &str| match sized.get(key) {
        Some(chosen) => theme.sized(*chosen as f32),
        None => match key {
            "author" | "version" => heading(key).max(widest(key, theme.body())),
            "load" => heading(key).max(widest(key, theme.small())),
            "size" => heading(key).max(measure(
                window,
                theme,
                &put(&say!("{} MB", 999)),
                theme.small(),
                FontWeight::NORMAL,
            )),
            _ => heading(key).max(ago_widest(window, theme)),
        },
    };
    let timed = orders.active_by.contains_key("load");
    let chosen: Vec<&'static str> = kept::chosen(&view.columns)
        .into_iter()
        .filter_map(|key| ALL.into_iter().find(|known| *known == key))
        .filter(|key| *key != "load" || timed)
        .collect();
    let wide = width >= theme.sized(620.);
    let roomy = width >= theme.sized(440.);
    let view_button = btn_width(window, theme, &put(&say!("View")), false, true);
    let quick = view_button.max(theme.sized(if wide { 124. } else { 44. }));
    let number = theme.sized(28.);
    let room_beside = |shown: &[&str]| {
        let taken = shown.iter().fold(px(0.), |all, key| {
            all + column_width(key) + theme::GAP_LARGE
        });
        width
            - bar_room(theme)
            - quick
            - number
            - theme::GAP_WIDE * 2.
            - theme::GAP_LARGE * 2.
            - taken
    };
    let mut shown: Vec<&'static str> = if roomy { chosen } else { Vec::new() };
    for dropped in DROPPED_FIRST {
        if room_beside(&shown) >= theme.sized(180.) {
            break;
        }
        shown.retain(|key| *key != dropped);
    }
    let tile = theme.appearance.comfortable && width - bar_room(theme) >= theme.sized(420.);
    let fitting: Vec<(&'static str, Pixels)> = shown
        .into_iter()
        .map(|key| (key, column_width(key)))
        .collect();
    let beside = fitting
        .iter()
        .fold(px(0.), |all, (_, width)| all + *width + theme::GAP_LARGE);
    let tile_room = if tile {
        theme.sized(64.) + theme::GAP_SMALL
    } else {
        px(0.)
    };
    let name_room = width
        - bar_room(theme)
        - theme::GAP_WIDE * 2.
        - number
        - theme::GAP_LARGE * 2.
        - beside
        - quick
        - theme.sized(13.)
        - theme::GAP_SMALL * 2.
        - tile_room;
    Columns {
        number,
        name_room,
        fitting,
        quick,
        wide,
        tile,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    View,
    InactiveOrder,
}

impl Mods {
    /// The menu under its button; false while the button is not drawn yet.
    pub fn open_menu(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let at = match menu {
            Menu::View => self.menu_buttons.view.get(),
            Menu::InactiveOrder => self.menu_buttons.order.get(),
        };
        if at.size.width <= px(0.) {
            return false;
        }
        match menu {
            Menu::View => self.view_menu(at, window, cx),
            Menu::InactiveOrder => self.inactive_order_menu(at, window, cx),
        }
        true
    }

    /// The view changed by `change` and kept, as `ModsPrefs` kept each change.
    pub(super) fn keep_view(&mut self, change: impl FnOnce(&mut ModsView), cx: &mut Context<Self>) {
        let before = (*self.view).clone();
        change(Rc::make_mut(&mut self.view));
        if self.demo || *self.view == before {
            return;
        }
        if let Err(problem) = super::super::app(cx).keep_mods_view(&self.view) {
            notices::say(crate::words::said_of(&problem), notices::Tone::Err, cx);
        }
    }

    /// The view changed now and kept once the dragging that changes it rests.
    pub(super) fn keep_view_soon(
        &mut self,
        change: impl FnOnce(&mut ModsView),
        cx: &mut Context<Self>,
    ) {
        change(Rc::make_mut(&mut self.view));
        if self.demo {
            return;
        }
        self.view_kept.after(super::panes::SETTLED, cx, |mods, cx| {
            if let Err(problem) = super::super::app(cx).keep_mods_view(&mods.view) {
                notices::say(crate::words::said_of(&problem), notices::Tone::Err, cx);
            }
        });
    }

    pub(super) fn set_headings(&mut self, headings: bool, cx: &mut Context<Self>) {
        self.keep_view(|view| view.headings = headings, cx);
        let app = super::super::app(cx);
        self.order.set_headings(&app, headings);
        self.show(cx);
    }

    /// The load order shown by a column, as `then` keeps it; "" for the load order itself.
    pub(super) fn order_active(&mut self, then: &str, cx: &mut Context<Self>) {
        self.order.set_active_by(kept::from_kept(then));
        self.show(cx);
    }

    pub(super) fn order_inactive(&mut self, then: &str, cx: &mut Context<Self>) {
        let then = then.to_string();
        let order = kept::from_kept(&then);
        self.keep_view(|view| view.inactive_order = then, cx);
        let app = super::super::app(cx);
        self.order.set_inactive_by(&app, order);
        self.show(cx);
    }

    /// The column `key` as wide as its edge has been dragged to `x`: further left is wider.
    pub(super) fn size_column(&mut self, key: &'static str, x: Pixels, cx: &mut Context<Self>) {
        let Some((from_x, from_width)) = self.column_drag else {
            return;
        };
        let grow = f32::from(cx.global::<Theme>().sized(1.));
        let wide = f32::from(from_width - (x - from_x)) / grow;
        self.keep_view_soon(
            |view| view.column_sizes = kept::sized(&view.column_sizes, key, f64::from(wide)),
            cx,
        );
        self.sync_lists(cx);
        cx.notify();
    }

    /// A heading dropped more than half its width away moves one place that way.
    pub(super) fn drop_column(&mut self, held: ColumnHeld, x: Pixels, cx: &mut Context<Self>) {
        let Some((from_x, width)) = self.column_drag.take() else {
            return;
        };
        let moved = x - from_x;
        if moved.abs() <= width / 2. {
            return;
        }
        let ColumnHeld(key, at) = held;
        let to = if moved > px(0.) {
            at as i32 + 1
        } else {
            at as i32 - 1
        };
        self.keep_view(
            |view| view.columns = kept::moved(&view.columns, key, to),
            cx,
        );
        self.sync_lists(cx);
        cx.notify();
    }

    pub(super) fn toggle_column(&mut self, column: &'static str, cx: &mut Context<Self>) {
        self.keep_view(
            |view| view.columns = kept::toggled(&view.columns, column),
            cx,
        );
        self.sync_lists(cx);
        cx.notify();
    }

    /// A heading pressed: the list by its column, turned round, then back to the load order.
    pub(super) fn heading_pressed(&mut self, column: &str, cx: &mut Context<Self>) {
        let then = self
            .orders
            .active_by
            .get(column)
            .map(|entry| entry.then.clone())
            .unwrap_or_default();
        self.order_active(&then, cx);
    }

    fn order_entries(
        &self,
        entries: &[OrderEntry],
        run: fn(&mut Mods, &str, &mut Context<Mods>),
        cx: &Context<Self>,
    ) -> Vec<Entry> {
        let me = cx.entity().downgrade();
        entries
            .iter()
            .map(|entry| {
                let (me, then) = (me.clone(), entry.then.clone());
                Entry::item(put(&entry.label), move |_, cx| {
                    let _ = me.update(cx, |mods, cx| run(mods, &then, cx));
                })
                .icon(entry.icon)
                .into()
            })
            .collect()
    }

    /// The columns, by name; `noted` puts how load times are had under Load, as a menu shows it.
    fn column_entries(&self, noted: bool, cx: &Context<Self>) -> Vec<Entry> {
        let me = cx.entity().downgrade();
        let chosen = kept::chosen(&self.view.columns);
        let how = self.load_times_how();
        let mut named: Vec<(&'static str, String)> = ALL
            .into_iter()
            .map(|column| (column, put(&column_name(column))))
            .collect();
        named.sort_by(|a, b| a.1.cmp(&b.1));
        named
            .into_iter()
            .map(|(column, name)| {
                let me = me.clone();
                let item = Entry::item(name, move |_, cx| {
                    let _ = me.update(cx, |mods, cx| mods.toggle_column(column, cx));
                })
                .icon(if chosen.contains(&column) {
                    "check"
                } else {
                    ""
                });
                match (column, &how, noted) {
                    ("load", Some(how), true) => item.note(how.clone()).into(),
                    ("load", Some(how), false) => item.tip(how.clone()).into(),
                    _ => item.into(),
                }
            })
            .collect()
    }

    /// How load times are had, while there are none.
    fn load_times_how(&self) -> Option<String> {
        self.overview
            .as_ref()
            .filter(|o| !o.load_times)
            .map(|o| put(&o.load_times_how))
    }

    /// The View menu: the headings, the order shown, and the columns.
    pub(super) fn view_menu(
        &self,
        below: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let me = cx.entity().downgrade();
        let headings = |on: bool, text: Sentence, checked: bool| -> Entry {
            let me = me.clone();
            Entry::item(put(&text), move |_, cx| {
                let _ = me.update(cx, |mods, cx| mods.set_headings(on, cx));
            })
            .icon(if checked { "check" } else { "" })
            .into()
        };
        let mut entries = vec![
            Entry::Title(put(&say!("Load order")).into()),
            headings(true, say!("Under category headings"), self.view.headings),
            headings(false, say!("One numbered list"), !self.view.headings),
            Entry::Title(put(&say!("Order")).into()),
        ];
        entries.extend(self.order_entries(&self.orders.active, Mods::order_active, cx));
        entries.push(Entry::Title(put(&say!("Columns")).into()));
        entries.extend(self.column_entries(false, cx));
        if let Some(how) = self.load_times_how() {
            entries.push(Entry::Note(how.into()));
        }
        entries.push(Entry::Note(
            put(&say!(
                "A heading orders the list when clicked and moves when dragged."
            ))
            .into(),
        ));
        menu::open_buttons_under_end(below, Some(40), entries, window, cx);
    }

    /// The columns alone, as a right click on a heading offers them.
    pub(super) fn columns_menu(
        &self,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        menu::open_at(at, self.column_entries(true, cx), window, cx);
    }

    pub(super) fn inactive_order_menu(
        &self,
        below: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = self.order_entries(&self.orders.inactive, Mods::order_inactive, cx);
        menu::open_buttons_under_end(below, None, entries, window, cx);
    }

    /// Every mod installed since the game last started from RimStack, activated, as one change.
    pub(super) fn activate_fresh(&mut self, cx: &mut Context<Self>) {
        let app = super::super::app(cx);
        let fresh: Vec<ModId> = self.order.fresh(&app, Side::Inactive);
        if fresh.is_empty() || self.order.previewing() {
            return;
        }
        self.order.take_all(&app, Side::Inactive, &fresh, None);
        self.changed(cx);
        self.done_said(
            say_n!(fresh.len(), "Activated {} new mod", "Activated {} new mods"),
            cx,
        );
        self.filter_by("", cx);
    }
}

impl Mods {
    pub(super) fn reachable(
        &mut self,
        name: &'static str,
        cx: &mut Context<Self>,
    ) -> (gpui::FocusHandle, Rc<Cell<Bounds<Pixels>>>) {
        self.keys
            .entry(name)
            .or_insert_with(|| (cx.focus_handle().tab_stop(true), Rc::default()))
            .clone()
    }

    /// A heading's keys: the menu key opens the columns' menu,
    /// Ctrl + Alt + Left or Right moves the column. Enter and Space press it, as GPUI presses a
    /// control that holds the keys.
    pub(super) fn heading_key(
        &mut self,
        column: &'static str,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keys = &event.keystroke;
        let held = keys.modifiers;
        match keys.key.as_str() {
            "menu" | "f10" if keys.key == "menu" || held.shift => {
                let drawn = self.reachable(column, cx).1.get();
                let at = gpui::point(drawn.left(), drawn.bottom() + px(2.));
                self.columns_menu(at, window, cx);
            }
            "left" | "right" if held.control && held.alt => {
                self.move_column(column, if keys.key == "right" { 1 } else { -1 }, cx);
            }
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Mods {
    /// A column moved `by` places among those shown.
    pub(super) fn move_column(&mut self, column: &'static str, by: i32, cx: &mut Context<Self>) {
        let Some(at) = kept::chosen(&self.view.columns)
            .iter()
            .position(|shown| *shown == column)
        else {
            return;
        };
        self.keep_view(
            |view| view.columns = kept::moved(&view.columns, column, at as i32 + by),
            cx,
        );
        self.sync_lists(cx);
        cx.notify();
    }
}
