#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;

        include!("rimstack-app/src/workshop_rows.cxxqt.h");
        type WorkshopRows = crate::workshop_rows::qobject::WorkshopRows;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(*mut WorkshopRows, rows, READ, CONSTANT)]
        #[qproperty(i32, total)]
        #[qproperty(bool, more)]
        #[qproperty(bool, searching)]
        /// Empty for none.
        #[qproperty(QString, problem)]
        #[qproperty(bool, ready)]
        #[qproperty(bool, key_mode, cxx_name = "keyMode")]
        /// `[{ id, label, chosen }]`
        #[qproperty(QVariant, sorts)]
        #[qproperty(QString, sort)]
        #[qproperty(QString, sort_label, cxx_name = "sortLabel")]
        #[qproperty(QVariant, sidebar)]
        #[qproperty(bool, collections)]
        /// Empty for none; cleared by the next `search`.
        #[qproperty(QString, notice)]
        #[qproperty(QString, query)]
        /// `{ active, which, id, title }`; `which` is `requires`, `neededBy`, `sameAuthor` or
        /// `collection` while `active`.
        #[qproperty(QVariant, related)]
        #[qproperty(QVariant, details)]
        #[qproperty(bool, details_busy, cxx_name = "detailsBusy")]
        /// `{ rows, count, sizeSaid, failures, action }`.
        #[qproperty(QVariant, list)]
        type WorkshopData = super::WorkshopDataRust;
    }

    impl cxx_qt::Threading for WorkshopData {}

    extern "RustQt" {
        /// Tone is "ok" or "err".
        #[qsignal]
        fn told(self: Pin<&mut WorkshopData>, said: QString, tone: QString);

        #[qinvokable]
        fn search(self: Pin<&mut WorkshopData>, text: &QString);

        #[qinvokable]
        fn browse(self: Pin<&mut WorkshopData>, text: &QString);

        #[qinvokable]
        fn open(self: Pin<&mut WorkshopData>);

        #[qinvokable]
        #[cxx_name = "toggleTag"]
        fn toggle_tag(self: Pin<&mut WorkshopData>, id: &QString, include: bool);

        #[qinvokable]
        #[cxx_name = "toggleDlc"]
        fn toggle_dlc(self: Pin<&mut WorkshopData>, id: &QString, include: bool);

        #[qinvokable]
        #[cxx_name = "clearFilter"]
        fn clear_filter(self: Pin<&mut WorkshopData>, id: &QString);

        #[qinvokable]
        #[cxx_name = "clearAll"]
        fn clear_all(self: Pin<&mut WorkshopData>);

        #[qinvokable]
        #[cxx_name = "chooseSort"]
        fn choose_sort(self: Pin<&mut WorkshopData>, id: &QString);

        #[qinvokable]
        #[cxx_name = "showIncompatible"]
        fn show_incompatible(self: Pin<&mut WorkshopData>, on: bool);

        #[qinvokable]
        #[cxx_name = "hideUnowned"]
        fn hide_unowned(self: Pin<&mut WorkshopData>, on: bool);

        #[qinvokable]
        #[cxx_name = "showCollections"]
        fn show_collections(self: Pin<&mut WorkshopData>, on: bool);

        #[qinvokable]
        #[cxx_name = "wantMore"]
        fn want_more(self: Pin<&mut WorkshopData>, near: bool);

        #[qinvokable]
        #[cxx_name = "showRelated"]
        fn show_related(self: Pin<&mut WorkshopData>, id: &QString, which: &QString);

        #[qinvokable]
        fn back(self: Pin<&mut WorkshopData>);

        /// An empty id closes the details.
        #[qinvokable]
        fn select(self: Pin<&mut WorkshopData>, id: &QString);

        /// The link to open, or empty when Browse handles it itself or it is no web link.
        #[qinvokable]
        fn followed(self: Pin<&mut WorkshopData>, link: &QString, in_workshop: bool) -> QString;

        #[qinvokable]
        fn add(self: Pin<&mut WorkshopData>, id: &QString, with_requirements: bool);

        #[qinvokable]
        #[cxx_name = "addMissing"]
        fn add_missing(self: Pin<&mut WorkshopData>, workshop_id: &QString);

        #[qinvokable]
        fn remove(self: &WorkshopData, id: &QString);

        #[qinvokable]
        fn retry(self: &WorkshopData);

        #[qinvokable]
        fn empty(self: &WorkshopData);
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[namespace = "rust::cxxqtlib1"]
        fn make_workshop_data() -> UniquePtr<WorkshopData>;
    }
}

pub(crate) mod view;

use crate::App;
use crate::changes::Changes;
use crate::qml::values::to_qml;
use crate::workshop_rows::Item;
use crate::workspace::browse::{
    Asked, Followed, Results, Row, Shown, Sidebar, Sort, chosen, effective, follow, offers, sort_of,
};
use crate::workspace::download_list::ListShown;
use core::pin::Pin;
use cxx::UniquePtr;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QVariant};
use qobject::WorkshopRows;
use rimstack_core::base::error::Error;
use rimstack_core::base::locks::Held;
use rimstack_core::rimworld::workshopsearch::Query;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
mod details;
use view::{RelatedShown, View, related_of};

const SCREENSHOTS: usize = 8;

pub struct WorkshopDataRust {
    rows: *mut WorkshopRows,
    owned_rows: UniquePtr<WorkshopRows>,
    total: i32,
    more: bool,
    searching: bool,
    problem: QString,
    ready: bool,
    key_mode: bool,
    sorts: QVariant,
    offered: Vec<Sort>,
    wanted_sort: String,
    sort: QString,
    sort_label: QString,
    sidebar: QVariant,
    collections: bool,
    notice: QString,
    query: QString,
    related: QVariant,
    details: QVariant,
    details_busy: bool,
    list: QVariant,
    app: crate::hub::Given,
    demo: bool,
    asked: Option<Asked>,
    in_related: Option<(crate::workspace::browse::Related, String)>,
    search_loaded: usize,
    reached: usize,
    current: Vec<Row>,
    previews: HashMap<String, String>,
    stars: HashMap<String, Vec<bool>>,
    settled: HashSet<String>,
    selected: String,
    shown: Option<Shown>,
    detail_preview: (String, String),
    detail_shots: (Vec<String>, Vec<String>),
    detail_images: HashMap<String, String>,
    seeking: crate::jobs::Lane,
    previewing: crate::jobs::Lane,
    opening: crate::jobs::Lane,
    detail_previewing: crate::jobs::Lane,
    shooting: crate::jobs::Lane,
    imaging: crate::jobs::Lane,
    listing: crate::jobs::Lane,
    badging: crate::jobs::Lane,
    defaulting: crate::jobs::Lane,
}

impl Default for WorkshopDataRust {
    fn default() -> Self {
        Self {
            rows: std::ptr::null_mut(),
            owned_rows: UniquePtr::null(),
            total: 0,
            more: false,
            searching: false,
            problem: QString::default(),
            ready: false,
            key_mode: false,
            sorts: QVariant::default(),
            offered: Vec::new(),
            wanted_sort: String::new(),
            sort: QString::from("recent"),
            sort_label: QString::default(),
            sidebar: QVariant::default(),
            collections: false,
            notice: QString::default(),
            query: QString::default(),
            related: QVariant::default(),
            details: QVariant::default(),
            details_busy: false,
            list: QVariant::default(),
            app: crate::hub::Given::default(),
            demo: false,
            asked: None,
            in_related: None,
            search_loaded: 0,
            reached: 0,
            current: Vec::new(),
            previews: HashMap::new(),
            stars: HashMap::new(),
            settled: HashSet::new(),
            selected: String::new(),
            shown: None,
            detail_preview: (String::new(), String::new()),
            detail_shots: (Vec::new(), Vec::new()),
            detail_images: HashMap::new(),
            seeking: crate::jobs::Lane::default(),
            previewing: crate::jobs::Lane::default(),
            opening: crate::jobs::Lane::default(),
            detail_previewing: crate::jobs::Lane::default(),
            shooting: crate::jobs::Lane::default(),
            imaging: crate::jobs::Lane::default(),
            listing: crate::jobs::Lane::default(),
            badging: crate::jobs::Lane::default(),
            defaulting: crate::jobs::Lane::default(),
        }
    }
}

type Found = Result<(Results, String), Error>;

impl qobject::WorkshopData {
    fn app(&self) -> Arc<App> {
        self.app.app()
    }

    fn model(self: Pin<&mut Self>) -> Option<Pin<&mut WorkshopRows>> {
        self.rust_mut().get_mut().owned_rows.as_mut()
    }

    pub(crate) fn connect(mut self: Pin<&mut Self>, app: Arc<App>, demo: bool) {
        let mut rows = crate::workshop_rows::qobject::make_workshop_rows();
        {
            let me = self.as_mut().rust_mut().get_mut();
            me.rows = crate::hub::address(&mut rows);
            me.owned_rows = rows;
            me.app = crate::hub::Given::of(app.clone());
            me.demo = demo;
        }
        self.as_mut().set_list(to_qml(&ListShown::empty()));
        self.as_mut().set_related(to_qml(&RelatedShown::none()));
        self.as_mut().set_sidebar(to_qml(&Sidebar::empty()));
        if demo {
            return;
        }
        crate::hub::hear(&app, self.qt_thread(), Self::heard);
        self.heard(Changes::COMMUNITY | Changes::DOWNLOAD_LIST);
    }

    fn heard(mut self: Pin<&mut Self>, changes: Changes) {
        if changes.contains(Changes::COMMUNITY | Changes::SOURCE | Changes::KEY) {
            self.as_mut().read_defaults();
        }
        if changes.contains(Changes::KEY) {
            self.as_mut().note_refusal();
        }
        if changes.contains(Changes::LANGUAGE) {
            if let Some(model) = self.as_mut().model() {
                model.reworded();
            }
            self.as_mut().settle_sort();
        }
        if changes.contains(Changes::COMMUNITY | Changes::SOURCE | Changes::LANGUAGE) {
            self.as_mut().reload();
        } else if changes.contains(Changes::DOWNLOAD_LIST | Changes::LIBRARY | Changes::MOD_FILES) {
            self.as_mut().rebadge();
        }
        if changes.contains(
            Changes::DOWNLOAD_LIST
                | Changes::LIBRARY
                | Changes::MOD_FILES
                | Changes::COMMUNITY
                | Changes::SETTINGS
                | Changes::SOURCE
                | Changes::LANGUAGE,
        ) {
            self.as_mut().read_list();
            self.reopen_details();
        }
    }

    fn read_defaults(self: Pin<&mut Self>) {
        let app = self.app();
        self.defaulting.run(
            "workshop defaults",
            self.qt_thread(),
            move |_| {
                let defaults = app.browse_defaults();
                app.sweep_if_due();
                defaults
            },
            |mut me, defaults| {
                me.as_mut().set_ready(defaults.ready);
                me.as_mut().rust_mut().get_mut().offered = defaults.sorts;
                me.as_mut().set_key_mode(defaults.key_mode);
                if !defaults.key_mode {
                    me.as_mut().set_collections(false);
                }
                me.settle_sort();
            },
        );
    }

    fn note_refusal(self: Pin<&mut Self>) {
        let app = self.app();
        crate::jobs::run(
            "steam key state for browse",
            self.qt_thread(),
            move || app.key_state(),
            |mut me, state| {
                if let Some(note) = state.note.filter(|_| state.refused) {
                    me.as_mut().set_notice(crate::words::said(&note));
                }
            },
        );
    }

    fn settle_sort(mut self: Pin<&mut Self>) {
        let words = self.words();
        let offered = offers(&self.offered, words, self.key_mode);
        let id = effective(&self.wanted_sort, &offered, words);
        self.as_mut().set_sort(QString::from(id));
        let label = offered.iter().find(|offer| offer.id == id);
        let label = label.map_or_else(QString::default, |offer| crate::words::said(&offer.label));
        self.as_mut().set_sort_label(label);
        self.as_mut().set_sorts(to_qml(&chosen(&offered, id)));
        let collections = self.collections;
        if let Some(asked) = self.as_mut().rust_mut().get_mut().asked.as_mut() {
            asked.sort = sort_of(id);
            asked.collections = collections;
        }
    }

    fn words(&self) -> bool {
        let text = self.asked.as_ref().map_or("", |asked| asked.text.as_str());
        !Query::parse(text).words.is_empty()
    }

    fn read_list(self: Pin<&mut Self>) {
        let app = self.app();
        self.listing.run(
            "download list",
            self.qt_thread(),
            move |_| app.list_shown(),
            |mut me, shown| me.as_mut().set_list(to_qml(&shown)),
        );
    }

    fn current_view(&self) -> Option<View> {
        match &self.in_related {
            Some((which, id)) => Some(View::Related(*which, id.clone())),
            None => self.asked.clone().map(View::Search),
        }
    }

    fn reload(self: Pin<&mut Self>) {
        let Some(view) = self.current_view() else {
            return;
        };
        let count = self.current.len().max(self.app().browse_page());
        self.load(view, 0, count, false);
    }

    fn rebadge(self: Pin<&mut Self>) {
        let app = self.app();
        self.badging.run(
            "workshop badges",
            self.qt_thread(),
            move |_| app.marks(),
            |mut me, marks| {
                let rows = &mut me.as_mut().rust_mut().get_mut().current;
                rows.iter_mut().for_each(|row| marks.mark(row));
                me.show_rows();
            },
        );
    }

    fn load(mut self: Pin<&mut Self>, view: View, offset: usize, count: usize, append: bool) {
        self.as_mut().set_searching(true);
        let app = self.app();
        self.seeking.run(
            "workshop results",
            self.qt_thread(),
            move |_| -> Found {
                match view {
                    View::Search(asked) => {
                        let found = app.browse_search(&asked, offset, count);
                        found.map(|results| (results, String::new()))
                    }
                    View::Related(which, id) => {
                        let found = app.browse_related(&id, which);
                        found.map(|results| (results, app.browse_title(&id)))
                    }
                }
            },
            move |me, found| me.arrived(found, append, offset + count),
        );
    }

    fn arrived(mut self: Pin<&mut Self>, found: Found, append: bool, reached: usize) {
        match found {
            Ok((mut results, title)) => {
                self.as_mut().set_problem(QString::default());
                if let Some(sidebar) = results.sidebar.take() {
                    self.as_mut().set_sidebar(to_qml(&sidebar));
                }
                if let Some(notice) = &results.notice {
                    self.as_mut().set_notice(crate::words::said(notice));
                    self.as_mut().read_defaults();
                }
                let (total, more) = (results.total, results.more);
                let me = self.as_mut().rust_mut().get_mut();
                me.reached = reached;
                if append {
                    let present: HashSet<String> =
                        me.current.iter().map(|r| r.id.clone()).collect();
                    let fresh = results
                        .rows
                        .into_iter()
                        .filter(|r| !present.contains(&r.id));
                    me.current.extend(fresh);
                } else {
                    me.current = results.rows;
                }
                if let Some((which, id)) = &me.in_related {
                    let related = to_qml(&RelatedShown::of(*which, id, &title));
                    self.as_mut().set_related(related);
                }
                self.as_mut()
                    .set_total(i32::try_from(total).unwrap_or(i32::MAX));
                self.as_mut().set_more(more);
            }
            Err(problem) => {
                self.as_mut().rust_mut().get_mut().current.clear();
                self.as_mut().set_problem(crate::words::said_of(&problem));
                self.as_mut().set_total(0);
                self.as_mut().set_more(false);
            }
        }
        self.as_mut().show_rows();
        self.as_mut().set_searching(false);
        self.fetch_previews();
    }

    fn show_rows(mut self: Pin<&mut Self>) {
        let items: Vec<Item> = self
            .current
            .iter()
            .map(|row| Item {
                row: Row {
                    stars: self.stars.get(&row.id).cloned().unwrap_or_default(),
                    ..row.clone()
                },
                preview: self.previews.get(&row.id).cloned().unwrap_or_default(),
            })
            .collect();
        if let Some(model) = self.as_mut().model() {
            model.show(items);
        }
    }

    fn fetch_previews(self: Pin<&mut Self>) {
        let ids: Vec<String> = self
            .current
            .iter()
            .filter(|row| !self.settled.contains(&row.id))
            .map(|row| row.id.clone())
            .collect();
        if ids.is_empty() {
            return;
        }
        let app = self.app();
        let landing = Mutex::new(self.qt_thread());
        self.previewing.run(
            "workshop previews",
            self.qt_thread(),
            move |turn| {
                app.browse_previews(&ids, &|id, path, stars| {
                    if !turn.current() {
                        return false;
                    }
                    let id = id.to_string();
                    let path = path.map(|path| path.to_string_lossy().into_owned());
                    let told = landing
                        .held()
                        .queue(move |me| me.preview_came(&id, path, stars));
                    told.is_ok()
                });
            },
            |_, ()| {},
        );
    }

    fn preview_came(mut self: Pin<&mut Self>, id: &str, path: Option<String>, stars: Vec<bool>) {
        let me = self.as_mut().rust_mut().get_mut();
        me.settled.insert(id.to_string());
        if !stars.is_empty() {
            me.stars.insert(id.to_string(), stars.clone());
            if let Some(model) = self.as_mut().model() {
                model.set_stars(id, &stars);
            }
        }
        let me = self.as_mut().rust_mut().get_mut();
        let Some(path) = path else {
            return;
        };
        me.previews.insert(id.to_string(), path.clone());
        if let Some(model) = self.model() {
            model.set_preview(id, &path);
        }
    }

    fn ask(mut self: Pin<&mut Self>, change: impl FnOnce(&mut Asked)) {
        let mut asked = self.asked.clone().unwrap_or_default();
        change(&mut asked);
        self.as_mut().set_query(QString::from(asked.text.as_str()));
        self.as_mut().set_notice(QString::default());
        let me = self.as_mut().rust_mut().get_mut();
        me.asked = Some(asked);
        me.in_related = None;
        self.as_mut().settle_sort();
        self.as_mut().set_related(to_qml(&RelatedShown::none()));
        if self.app().browse_opened() {
            self.as_mut().read_defaults();
        }
        let asked = self.asked.clone().unwrap_or_default();
        let page = self.app().browse_page();
        self.load(View::Search(asked), 0, page, false);
    }

    fn search(self: Pin<&mut Self>, text: &QString) {
        let text = text.to_string();
        self.ask(|asked| asked.text = text);
    }

    fn browse(mut self: Pin<&mut Self>, text: &QString) {
        self.as_mut().select(&QString::default());
        self.search(text);
    }

    fn open(self: Pin<&mut Self>) {
        if self.asked.is_none() {
            self.ask(|_| {});
        }
    }

    fn toggle_tag(self: Pin<&mut Self>, id: &QString, include: bool) {
        let id = id.to_string();
        self.ask(|asked| asked.picked.tag(&id, include));
    }

    fn toggle_dlc(self: Pin<&mut Self>, id: &QString, include: bool) {
        let id = id.to_string();
        self.ask(|asked| asked.picked.dlc(&id, include));
    }

    fn clear_filter(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        self.ask(|asked| asked.picked.clear(&id));
    }

    fn clear_all(self: Pin<&mut Self>) {
        self.ask(|asked| asked.picked = Default::default());
    }

    fn choose_sort(mut self: Pin<&mut Self>, id: &QString) {
        self.as_mut().rust_mut().get_mut().wanted_sort = id.to_string();
        self.ask(|_| {});
    }

    fn show_incompatible(self: Pin<&mut Self>, on: bool) {
        self.ask(|asked| asked.picked.show_incompatible = on);
    }

    fn hide_unowned(self: Pin<&mut Self>, on: bool) {
        self.ask(|asked| asked.picked.hide_unowned = on);
    }

    fn show_collections(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().set_collections(on);
        let Some(mut asked) = self.asked.clone() else {
            return;
        };
        asked.collections = on;
        let me = self.as_mut().rust_mut().get_mut();
        me.asked = Some(asked.clone());
        me.in_related = None;
        self.as_mut().set_related(to_qml(&RelatedShown::none()));
        let page = self.app().browse_page();
        self.load(View::Search(asked), 0, page, false);
    }

    fn want_more(self: Pin<&mut Self>, near: bool) {
        let Some(asked) = self
            .asked
            .clone()
            .filter(|_| near && self.more && !self.searching && self.in_related.is_none())
        else {
            return;
        };
        let offset = self.reached;
        let page = self.app().browse_page();
        self.load(View::Search(asked), offset, page, true);
    }

    fn show_related(mut self: Pin<&mut Self>, id: &QString, which: &QString) {
        let Some(which) = related_of(&which.to_string()) else {
            return;
        };
        let id = id.to_string();
        let me = self.as_mut().rust_mut().get_mut();
        if me.in_related.is_none() {
            me.search_loaded = me.current.len();
        }
        me.in_related = Some((which, id.clone()));
        self.as_mut()
            .set_related(to_qml(&RelatedShown::of(which, &id, "")));
        let page = self.app().browse_page();
        self.load(View::Related(which, id), 0, page, false);
    }

    fn back(mut self: Pin<&mut Self>) {
        if self.in_related.is_none() {
            return;
        }
        let page = self.app().browse_page();
        let me = self.as_mut().rust_mut().get_mut();
        me.in_related = None;
        let count = me.search_loaded.max(page);
        let view = View::Search(me.asked.get_or_insert_with(Asked::default).clone());
        self.as_mut().set_related(to_qml(&RelatedShown::none()));
        self.load(view, 0, count, false);
    }

    fn select(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let me = self.as_mut().rust_mut().get_mut();
        me.selected = id.clone();
        if id.is_empty() {
            me.shown = None;
            self.opening.cancel();
            self.imaging.cancel();
            self.detail_previewing.cancel();
            self.as_mut().set_details_busy(false);
            self.set_details(QVariant::default());
            return;
        }
        self.as_mut().set_details_busy(true);
        self.open_details(id);
    }

    fn followed(mut self: Pin<&mut Self>, link: &QString, in_workshop: bool) -> QString {
        if !in_workshop {
            return link.clone();
        }
        match follow(&link.to_string()) {
            Followed::Item(id) => {
                self.as_mut().select(&QString::from(id));
                QString::default()
            }
            Followed::Outside(url) => QString::from(url),
            Followed::Nothing => QString::default(),
        }
    }

    fn add(self: Pin<&mut Self>, id: &QString, with_requirements: bool) {
        let (app, id) = (self.app(), id.to_string());
        crate::jobs::run(
            "add to the download list",
            self.qt_thread(),
            move || app.add_to_list(&id, with_requirements).note(),
            |me, note| me.tell(note),
        );
    }

    fn add_missing(self: Pin<&mut Self>, workshop_id: &QString) {
        let (app, id) = (self.app(), workshop_id.to_string());
        crate::jobs::run(
            "add a missing mod to the download list",
            self.qt_thread(),
            move || app.add_missing_to_list(&id).note(),
            |me, note| me.tell(note),
        );
    }

    fn tell(self: Pin<&mut Self>, note: Option<(rimstack_core::base::sentence::Sentence, bool)>) {
        if let Some((said, failed)) = note {
            let tone = if failed { "err" } else { "ok" };
            self.told(crate::words::said(&said), QString::from(tone));
        }
    }

    fn remove(&self, id: &QString) {
        self.app().remove_from_list(&id.to_string());
    }

    fn retry(&self) {
        self.app().clear_list_failures();
    }

    fn empty(&self) {
        self.app().empty_list();
    }
}
