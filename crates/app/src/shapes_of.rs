use crate::App;
use crate::reports::{Answer, ReleaseSaid, Written};
use crate::shapes::Shapes;
use crate::workspace::changed::{NotesState, Only, since_shown};
use crate::workspace::downloads::{Checkout, Queue, Updates, updates_shown};
use crate::workspace::load_order::{Clicked, Holding, LoadOrder, Side};
use crate::workspace::tests::installed;
use crate::workspace::{logs, saves, textures};
use rimstack_core::kept::launches;
use rimstack_core::mods::columns;
use rimstack_core::mods::library::ModId;
use rimstack_core::mods::list::{Row, SortKey};
use std::collections::HashSet;
use std::path::Path;

fn write(at: &Path, text: &str) {
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(at, text).unwrap();
}

fn a_mod(game: &Path, folder: &str, id: &str, more: &str) {
    write(
        &game.join("Mods").join(folder).join("About/About.xml"),
        &format!(
            "<ModMetaData><packageId>{id}</packageId><name>{id}</name><supportedVersions><li>1.6</li></supportedVersions>{more}</ModMetaData>"
        ),
    );
}

fn picture(at: &Path, colour: [u8; 4]) {
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    image::RgbaImage::from_pixel(4, 4, image::Rgba(colour))
        .save(at)
        .unwrap();
}

fn git(dir: &Path, args: &[&str]) {
    let ran = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=a",
            "-c",
            "user.email=a@b",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
}

const SAVE: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<savegame><meta><gameVersion>1.6.4871 rev600</gameVersion>
<modIds><li>ludeon.rimworld</li><li>ann.roads</li><li>gone.mod</li></modIds>
<modNames><li>Core</li><li>Ann.Roads</li><li>Gone</li></modNames></meta><game/></savegame>";

const STARTUP: &str = "<?xml version=\"1.0\"?><StartupImpactSession><sessionData>
<loadingTime>3000</loadingTime><totalImpact>2000</totalImpact><mods>
<li><modName>Ann.A</modName><modPackageId>Ann.A</modPackageId><totalImpact>1240</totalImpact></li>
</mods></sessionData></StartupImpactSession>";

const LOG: &str = "RimWorld 1.6.4871 rev600
Could not load texture for Ann.Roads
  at Verse.ContentFinder.Get
Could not load texture for Ann.Roads
all fine
";

fn made_up() -> (tempfile::TempDir, App) {
    let (tmp, app) = installed();
    let (root, game) = (tmp.path(), tmp.path().join("game"));
    let mods = game.join("Mods");
    write(
        &mods.join("roads/About/About.xml"),
        "<ModMetaData><packageId>Ann.Roads</packageId><name>Ann.Roads</name><supportedVersions><li>1.5</li></supportedVersions></ModMetaData>",
    );
    write(
        &mods.join("roads/Patches/rock.xml"),
        "<Patch><Operation Class=\"PatchOperationAdd\"><xpath>Defs/ThingDef[defName=\"Rock\"]/comps</xpath><value><li/></value></Operation></Patch>",
    );
    git(&mods.join("roads"), &["init", "--quiet"]);
    git(&mods.join("roads"), &["add", "."]);
    git(&mods.join("roads"), &["commit", "--quiet", "-m", "Roads"]);
    a_mod(&game, "a", "Ann.A", "");
    write(
        &mods.join("a/Defs/Things.xml"),
        "<Defs><ThingDef><defName>Rock</defName></ThingDef></Defs>",
    );
    a_mod(
        &game,
        "b",
        "Ann.B",
        "<modDependencies><li><packageId>Ann.Lib</packageId><displayName>Lib</displayName></li><li><packageId>gone.mod</packageId><displayName>Gone</displayName></li></modDependencies>",
    );
    a_mod(&game, "lib", "Ann.Lib", "");
    write(&mods.join("broken/About/About.xml"), "<ModMetaData><name>");
    let workshop = root.join("workshop/900002");
    write(
        &workshop.join("About/About.xml"),
        "<ModMetaData><packageId>Ann.Shop</packageId><name>Ann.Shop</name><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>",
    );
    picture(&workshop.join("About/Preview.png"), [9, 9, 9, 255]);
    write(
        &root.join("workshop/900003/About/About.xml"),
        "<ModMetaData><packageId>Ann.Hats</packageId><name>Ann.Hats</name><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>",
    );
    write(
        &mods.join("hats/About/About.xml"),
        "<ModMetaData><packageId>Ann.Hats</packageId><name>Ann.Hats</name><supportedVersions><li>1.5</li></supportedVersions></ModMetaData>",
    );
    picture(&game.join("Data/Core/About/Preview.png"), [90, 60, 30, 255]);
    picture(&mods.join("a/Textures/Things/rock.png"), [200, 0, 0, 255]);
    picture(&mods.join("b/Textures/Things/rock.png"), [0, 0, 200, 255]);
    write(&mods.join("a/Textures/Things/rock.dds"), "DDS not really");
    picture(&mods.join("b/Textures/Things/pebble.png"), [0, 200, 0, 255]);
    picture(&mods.join("lib/Textures/Things/moss.png"), [0, 90, 0, 255]);
    let config = root.join("config");
    write(
        &config.join("Config/ModsConfig.xml"),
        "<ModsConfigData><activeMods><li>ludeon.rimworld</li><li>ann.roads</li><li>ann.a</li><li>ann.b</li><li>pulled.mod</li><li>lost.mod</li></activeMods></ModsConfigData>",
    );
    write(&config.join("Player.log"), LOG);
    write(&config.join("StartupImpactData.xml"), STARTUP);
    write(&config.join("Saves/Colony.rws"), SAVE);
    write(
        &config.join("RimSaves/Meta/Colony.dat"),
        "faction:New Hope,nbColonist:3,seed:abc",
    );
    write(&config.join("RimSaves/Previews/Colony.jpg"), "jpeg");
    app.rescan();
    launches::record(&app.launches_file(), &["ludeon.rimworld".to_string()], 1).unwrap();
    app.mark_all_seen();
    a_mod(&game, "fresh", "Ann.Fresh", "");
    app.rescan();
    (tmp, app)
}

fn by_folder(app: &App, folder: &str) -> ModId {
    let loaded = app.library();
    let (library, _) = loaded.as_ref().as_ref().unwrap();

    library.ids().find(|(_, m)| m.folder == folder).unwrap().0
}

pub(crate) fn every() -> Shapes {
    let (_tmp, app) = made_up();
    let mut shapes = Shapes::default();
    logs(&app, &mut shapes);
    settings(&app, &mut shapes);
    mods(&app, &mut shapes);
    lists(&app, &mut shapes);
    game(&app, &mut shapes);
    saves(&app, &mut shapes);
    textures(&app, &mut shapes);
    search(&app, &mut shapes);
    downloads(&app, &mut shapes);
    find(&app, &mut shapes);
    workshop(&mut shapes);
    settings_once_set(&app, &mut shapes);
    shapes
}

fn settings_once_set(app: &App, shapes: &mut Shapes) {
    let paths = app.paths();
    for (name, folder) in [
        ("game", &paths.game),
        ("config", &paths.config),
        ("workshop", &paths.workshop),
    ] {
        let folder = folder.as_ref().unwrap().to_string_lossy().into_owned();
        app.keep_setting(name, &folder).unwrap();
    }
    shapes.add("settings.report", &app.settings_report());
}

fn settings(app: &App, shapes: &mut Shapes) {
    for (name, value) in [
        ("replacementsIgnored", "1"),
        ("font", "Noto Sans"),
        ("language", "Polski"),
        ("launchArguments", "-quicktest"),
    ] {
        app.keep_setting(name, value).unwrap();
    }
    app.version_warnings(1).unwrap();
    shapes.add("settings.report", &app.settings_report());
    shapes.add("settings.databases", &app.databases());
    settings_key(shapes);
    let checked = std::collections::BTreeMap::from([(
        "game",
        crate::workspace::judge_folder("game", "/nowhere"),
    )]);
    shapes.add("settings.folderChecks", &checked);
    shapes.add("settings.languages", &app.languages());
    shapes.add("settings.words", &app.speak());
    let (area, maximized) = app.window_start(&[]);
    shapes.add(
        "settings.windowStart",
        &crate::reports::WindowStart::of(area, maximized),
    );
    let colours = serde_json::json!({ "accent": "#336699" });
    let written = app.start_own_theme("light", &colours);
    shapes.add("settings.ownColours", &app.own_colours("light"));
    let written = written.map(|file| Written {
        file: file.to_string_lossy().into_owned(),
    });
    shapes.add("settings.startOwnTheme", &Answer::from(written));
    let refused = app.start_language("../outside").map(|file| Written {
        file: file.to_string_lossy().into_owned(),
    });
    shapes.add("settings.startLanguage", &Answer::from(refused));
    let started = app.start_language("Polski").map(|file| Written {
        file: file.to_string_lossy().into_owned(),
    });
    shapes.add("settings.startLanguage", &Answer::from(started));
    let pair = serde_json::json!({ "ink": "#777777", "win": "#888888" });
    let contrast = crate::workspace::short_contrast(pair.as_object().unwrap());
    shapes.add("settings.shortContrast", &contrast);
    shapes.add("settings.release", &ReleaseSaid::Latest {});
    shapes.add(
        "settings.release",
        &ReleaseSaid::Newer {
            version: "2.0.0".into(),
            page: "https://example.org".into(),
        },
    );
    shapes.add(
        "settings.release",
        &ReleaseSaid::Failed {
            problem: rimstack_core::said!("refused"),
        },
    );
}

fn mods(app: &App, shapes: &mut Shapes) {
    let (roads, a, b) = (
        by_folder(app, "roads"),
        by_folder(app, "a"),
        by_folder(app, "b"),
    );
    let (shop, broken) = (by_folder(app, "900002"), by_folder(app, "broken"));
    let (hats, subscribed_hats) = (by_folder(app, "hats"), by_folder(app, "900003"));
    app.set_note(a, "Stone, Grey", "Read first");
    app.change_rule(a, b, true);
    app.set_version_ok(&[roads], true);
    write(
        &rimstack_core::rimworld::workshopdb::file_in(app.data_dir()),
        "pulled.mod\t900009\tPulled",
    );
    let mut order = LoadOrder::read(app, true, None);
    order.settle(app);
    if let Some(members) = order.files_to_read(app) {
        app.read_active_files(members);
        order.recheck(app);
    }
    shapes.add("mods.summary", &app.summary());
    shapes.add("mods.overview", &order.overview(app));
    shapes.add("mods.fresh", &order.fresh(app, Side::Inactive));
    for at in 0..order.list(Side::Active).mods().len() {
        if let Some(gone) = order.not_installed(app, Side::Active, at) {
            shapes.add("mods.notInstalled", &gone);
            let missing = Clicked {
                active: true,
                index: at,
                id: None,
                selected: &[],
            };
            shapes.add("mods.rowMenu", &order.row_menu(app, &missing));
        }
    }
    for id in [roads, a, b, shop, broken, hats, subscribed_hats] {
        shapes.add("mods.details", &order.details(app, id));
        shapes.add("mods.detailsOf", &order.details(app, id));
        shapes.add("mods.filesOf", &LoadOrder::files(app, id));
        shapes.add("mods.textures", &LoadOrder::textures(app, id));
    }
    let ids = |found: Vec<ModId>| found.into_iter().map(ModId::to_qml).collect::<Vec<_>>();
    let menu_of = |order: &mut LoadOrder, key: ModId, selected: &[ModId]| {
        let list = |side| {
            order
                .list(side)
                .mods()
                .iter()
                .position(|m| m.installed == Some(key))
        };
        let (active, index) = match (list(Side::Active), list(Side::Inactive)) {
            (Some(index), _) => (true, index),
            (_, index) => (false, index.unwrap_or(0)),
        };
        let at = Clicked {
            active,
            index,
            id: Some(key),
            selected,
        };
        order.row_menu(app, &at)
    };
    for (key, selected) in [
        (shop, vec![shop]),
        (a, vec![a, b]),
        (subscribed_hats, vec![subscribed_hats]),
        (roads, vec![roads]),
    ] {
        shapes.add("mods.rowMenu", &menu_of(&mut order, key, &selected));
    }
    if let Some(first) = order
        .list(Side::Active)
        .rows()
        .iter()
        .position(|r| matches!(r, Row::Sub { .. }))
    {
        shapes.add("mods.subMenu", &order.sub_menu(true, first));
    }
    shapes.add(
        "mods.selectionSummary",
        &order.selection_summary(true, &[a, b, broken, roads]),
    );
    shapes.add(
        "mods.selectionSummary",
        &order.selection_summary(false, &[roads, hats]),
    );
    shapes.add("mods.versionWarned", &ids(vec![roads]));
    shapes.add("mods.copyOffer", &app.copy_offer(&[shop]));
    shapes.add("mods.copyOffer", &app.copy_offer(&[shop, roads]));
    shapes.add("mods.packageMods", &ids(vec![a, b]));
    shapes.add("mods.findMods", &order.found(app, "Ann", 5));
    order.preview_sort(app);
    shapes.add("mods.sortMoves", &order.sort_moves());
    let list = order.list(Side::Active);
    shapes.add("mods.selectedIds", &ids(vec![a, b]));
    shapes.add("mods.holding", &Holding::of(Some(a), "", &[a, b]));
    shapes.add("mods.holding", &Holding::of(None, "Missing", &[]));
    let ordered = list.in_load_order(a, b);
    shapes.add(
        "mods.inLoadOrder",
        &ordered.map(|(earlier, later)| [earlier.to_qml(), later.to_qml()]),
    );
    shapes.add("mods.inLoadOrder", &None::<[i32; 2]>);
    let category = list
        .mods()
        .iter()
        .find(|m| m.installed == Some(a))
        .unwrap()
        .category;
    shapes.add("mods.subsBeside", &list.subs(category));
    shapes.add("mods.columnWidths", &columns::widths("name=200,author=90"));
    order.set_inactive_by(app, Some((SortKey::Updated, true)));
    shapes.add("mods.orders", &order.orders());
    for row in order.rows(app, Side::Active) {
        shapes.add("mod_rows.at", &crate::mod_rows::fields_of(&row));
    }
    order.save(app).unwrap();
}

fn lists(app: &App, shapes: &mut Shapes) {
    let ids = vec!["ludeon.rimworld".to_string(), "ann.a".to_string()];
    app.keep_list("mine", Some(ids.clone())).unwrap();
    app.snapshot("@saving", &ids).unwrap();
    shapes.add("lists.saved", &app.saved_lists());
    shapes.add("lists.snapshots", &app.recent_snapshots());
    shapes.add("lists.named", &app.list_named("mine"));
    shapes.add("lists.named", &app.list_named("none"));
    let snapshot = app.recent_snapshots().first().map(|s| s.name.clone());
    let snapshot = snapshot.unwrap_or_default();
    shapes.add("lists.snapshotNamed", &app.snapshot_named(&snapshot));
    shapes.add("lists.original", &app.original_list());
}

fn game(app: &App, shapes: &mut Shapes) {
    shapes.add("game.lastStart", &app.last_start_said());
    shapes.add("game.resetByGame", &app.mods_config_list());
}

fn saves(app: &App, shapes: &mut Shapes) {
    let report = app.saves_report();
    shapes.add("saves.saves", &saves::shown(&report, "", "").saves);
    shapes.add("saves.folders", &report.folders);
}

fn logs(app: &App, shapes: &mut Shapes) {
    app.keep_logs();
    let (report, _) = app.logs_report(false, None).unwrap();
    shapes.add("logs.head", &logs::head(&report));
    shapes.add("logs.culprits", &report.culprits);
    shapes.add("logs.launches", &logs::launches(&report));
    shapes.add("logs.since", &logs::since(&report));
    let open = HashSet::new();
    shapes.add("logs.errors", &logs::listed(&report.errors, "", &open));
    shapes.add("logs.lines", &logs::lines(&report, ""));
    shapes.add("logs.errorsOf", &logs::errors_of(&report, "Ann.Roads"));
}

fn textures(app: &App, shapes: &mut Shapes) {
    let (a, b) = (by_folder(app, "a"), by_folder(app, "b"));
    let stop = std::sync::atomic::AtomicBool::new(false);
    app.convert_textures(&[b], false, &stop, &|_| {});
    let loaded = app.library();
    let (library, _) = loaded.as_ref().as_ref().unwrap();
    let pebble = library
        .get(b)
        .unwrap()
        .path
        .join("Textures/Things/pebble.png");
    drop(loaded);
    picture(&pebble, [0, 0, 0, 255]);
    let rows = app.texture_rows();
    shapes.add("textures.report", &textures::shown(&rows, false));
    shapes.add("textures.refused", &app.refused_dds());
    let compared = app.compare_textures(a, b).unwrap();
    shapes.add("textures.comparison", &compared);
    shapes.add("textures.shared", &compared.named(""));
    let difference = app.texture_difference(&compared.textures[0]);
    shapes.add("textures.difference", &Answer::from(difference));
    let done = app.keep_all_textures(&compared, true);
    let done = done.map(|said| serde_json::json!({ "said": said }));
    shapes.add("textures.keepAll", &Answer::from(done));
    let again = app.compare_textures(a, b).unwrap();
    shapes.add("textures.comparison", &again);
    shapes.add("textures.shared", &again.named(""));
    shapes.add("lists.history", &app.history());
}

fn find(app: &App, shapes: &mut Shapes) {
    use crate::workspace::find::{Look, Place, Request};
    let a = by_folder(app, "a");
    let gone = app
        .library()
        .as_ref()
        .as_ref()
        .unwrap()
        .0
        .get(a)
        .unwrap()
        .path
        .join("Defs/Gone.xml");
    write(&gone, "<Defs>\nRock\n</Defs>");
    let mut request = Request {
        text: "rock".into(),
        place: Place::Installed,
        ..Request::default()
    };
    app.find_remember(&request, false);
    shapes.add("find.recent", &app.find_recent());
    shapes.add("find.options", &app.find_options(&request));
    request.selected = Some(a);
    shapes.add("find.options", &app.find_options(&request));
    request.types = "xml, a/b".into();
    shapes.add("find.options", &app.find_options(&request));
    request.types = "xml".into();
    let counts = app.find_extensions(&request);
    shapes.add(
        "find.typeSuggestions",
        &crate::workspace::find::type_offers(&counts, ""),
    );
    let started = app.find_start(&request).unwrap();
    let summary = app.find_search(&started, |batch| {
        app.find_add(started.generation, batch);
    });
    app.find_finish(started.generation, summary);
    let hit = app.find_results(|r| {
        r.refs()
            .into_iter()
            .rfind(|at| !at.is_mod() && !at.is_file())
    });
    let hit = hit.unwrap();
    let previewed = |at| {
        let (file, line) = app.find_located(at).unwrap();
        app.find_preview(&file, line, Look::default()).shown
    };
    shapes.add("find.preview", &previewed(hit));
    shapes.add("find.mods", &app.find_results(|r| r.found_mods()));
    std::fs::remove_file(&gone).unwrap();
    for at in app.find_results(|r| r.refs()) {
        if app.find_file(at).is_some_and(|f| f == gone) {
            shapes.add("find.preview", &previewed(at));
        }
    }
}

fn search(app: &App, shapes: &mut Shapes) {
    shapes.add("search.list", &app.start_hunt(false).list);
    shapes.add("search.end", &app.end_hunt(false));
}

fn downloads(app: &App, shapes: &mut Shapes) {
    use rimstack_core::sources::downloads::{Asked, Behind, Changed, Replaceable, Repository};
    let (roads, b) = (by_folder(app, "roads"), by_folder(app, "b"));
    shapes.add("downloads.replacements", &app.replaceable());
    let replaced = Replaceable {
        id: "1".into(),
        mod_id: b,
        name: "Ann.B".into(),
        active: true,
        new_name: "Ann.B2".into(),
        new_id: "2".into(),
        new_versions: "1.6".into(),
        have_new: false,
    };
    shapes.add("downloads.replacements", &[replaced]);
    let behind = |active| Behind {
        id: "1".into(),
        mod_id: b,
        name: "Ann.B".into(),
        active,
        have: 1,
        now: 2,
    };
    let checkout = Checkout {
        mod_id: roads,
        name: "Ann.Roads".into(),
        workshop: false,
        checked: true,
        behind: Some(2),
        problem: Some(rimstack_core::said!("refused")),
    };
    let updates = Updates {
        checked: Some(1),
        asked: Some(1),
        answered: 1,
        items: vec![behind(true), behind(false)],
        checkouts: vec![checkout],
        problem: Some(rimstack_core::said!("refused")),
    };
    let asked = HashSet::from(["1".to_string()]);
    shapes.add("downloads.updates", &updates_shown(&updates, &asked));
    shapes.add(
        "downloads.updates",
        &updates_shown(&Updates::default(), &asked),
    );
    let asked = |title: Option<&str>| Asked {
        id: "1".into(),
        title: title.map(String::from),
        updated: title.map(|_| 1),
        megabytes: title.map(|_| 1.5),
        installed: false,
    };
    let queue = Queue {
        items: vec![asked(Some("Ann.B")), asked(None)],
        repositories: vec![Repository {
            link: "https://github.com/ann/roads".into(),
            name: "ann/roads".into(),
            into: Some("/mods/roads".into()),
            installed: true,
        }],
        note: None,
        wanted: vec!["1".into()],
    };
    shapes.add("downloads.queue", &queue);
    shapes.add(
        "downloads.queue",
        &app.look_up("nothing to download here").unwrap(),
    );
    let refused: Result<(), _> = Err(rimstack_core::base::error::Error::refused(
        rimstack_core::said!("refused"),
    ));
    shapes.add("downloads.queue", &Answer::from(refused));
    let mut since = app.since_launch();
    since.items.push(Changed {
        git: false,
        id: "1".into(),
        mod_id: b,
        name: "Ann.B".into(),
        active: true,
        was: 1,
        now: Some(2),
        notes: app.commits_of(roads),
    });
    let mut notes = crate::workspace::changed::ChangeNotes::new();
    notes.insert("1".into(), app.commits_of(roads));
    shapes.add("downloads.since", &since_shown(&since, Only::All, &notes));
    shapes.add("downloads.notes", &notes);
    let state = NotesState {
        done: 1,
        asked: 2,
        resume: 0,
        refused: false,
    };
    shapes.add("downloads.notesState", &state);
    shapes.add("downloads.commitsOf", &app.commits_of(roads));
}

fn settings_key(shapes: &mut Shapes) {
    use crate::reports::KeyShown;
    use crate::workspace::secret::Memory;
    let (_tmp, app) = made_up();
    let app = app.browsing_fixture(true);
    let shown = |app: &App, checking, failure| KeyShown::of(app.key_state(), checking, failure);
    shapes.add("settings.steamKey", &shown(&app, false, None));
    app.check_and_keep("fixture").unwrap();
    shapes.add("settings.steamKey", &shown(&app, true, None));
    let refused = Some(rimstack_core::said!("Steam refused this key"));
    shapes.add("settings.steamKey", &shown(&app, false, refused));
    let why = rimstack_core::say!("No credential store is available to keep the key in");
    let broken = app.with_vault(std::sync::Arc::new(Memory::broken(why)));
    shapes.add("settings.steamKey", &shown(&broken, false, None));
}

fn workshop(shapes: &mut Shapes) {
    use crate::workshop_data::view::{DetailsShown, RelatedShown};
    use crate::workspace::browse::{Related, Shown};
    use crate::workspace::download_list::ListShown;
    use rimstack_core::sources::browse::HARMONY;
    const EXTRAS: &str = "3100000002";
    const ROYAL: &str = "3100000004";
    let (_tmp, app) = made_up();
    let app = app.browsing_fixture(true);
    use crate::workspace::browse::{Asked, chosen, offers};
    let sorts = |app: &crate::App, words: bool| {
        let defaults = app.browse_defaults();
        chosen(&offers(&defaults.sorts, words, defaults.key_mode), "recent")
    };
    shapes.add("workshop.sorts", &sorts(&app, false));
    let mut asked = Asked::default();
    asked.picked.tag("Mod", true);
    asked.picked.tag("1.5", false);
    asked.picked.dlc("1149640", true);
    asked.picked.hide_unowned = true;
    shapes.add(
        "workshop.sidebar",
        &app.browse_search(&asked, 0, 10).unwrap().sidebar,
    );
    shapes.add(
        "workshop.sidebar",
        &app.browse_search(&Asked::default(), 0, 10).unwrap().sidebar,
    );
    app.check_and_keep("fixture").unwrap();
    shapes.add("workshop.sorts", &sorts(&app, true));
    shapes.add(
        "workshop.sidebar",
        &app.browse_search(&asked, 0, 10).unwrap().sidebar,
    );
    shapes.add("workshop.related", &RelatedShown::none());
    shapes.add(
        "workshop.related",
        &RelatedShown::of(Related::NeededBy, HARMONY, "Fixture Harmony"),
    );
    app.add_to_list(EXTRAS, true);
    let mut shown = app.browse_details(EXTRAS).unwrap();
    shown.problem = Some(rimstack_core::said!("refused"));
    const PICTURE: &str = "https://images.steamusercontent.com/ugc/1/A/";
    shown.pieces = vec![
        rimstack_core::base::markup::Piece::Html("<b>Text</b>".into()),
        rimstack_core::base::markup::Piece::Image(PICTURE.into()),
        rimstack_core::base::markup::Piece::Image("https://x.example/b.png".into()),
    ];
    shown.created_said = Some(rimstack_core::said!("Created 2 days ago"));
    shown.requires[0].stars = vec![true, true, true, false, false];
    fn details<'a>(shown: &'a Shown, preview_path: &'a str) -> DetailsShown<'a> {
        const SHOTS: &[String] = &[];
        DetailsShown {
            shown,
            preview_path,
            description: shown.description(&Default::default()),
            screenshots: SHOTS,
        }
    }
    let shots = ["/previews/b.img".to_string()];
    let with_shots = DetailsShown {
        shown: &shown,
        preview_path: "/previews/a.img",
        description: shown.description(&std::collections::HashMap::from([(
            PICTURE.to_string(),
            "/previews/c.img".to_string(),
        )])),
        screenshots: &shots,
    };
    shapes.add("workshop.details", &with_shots);
    shapes.add("workshop.details", &details(&shown, "/previews/a.img"));
    shapes.add(
        "workshop.details",
        &details(&app.browse_details(ROYAL).unwrap(), ""),
    );
    shapes.add(
        "workshop.details",
        &details(&app.browse_details(HARMONY).unwrap(), ""),
    );
    shapes.add(
        "workshop.details",
        &details(&app.browse_details("3100000003").unwrap(), ""),
    );
    shapes.add("workshop.list", &ListShown::empty());
    let failed = rimstack_core::base::error::Error::refused(rimstack_core::said!("refused"));
    app.apply_downloads(&[("3100000001".to_string(), Err(failed))]);
    shapes.add("workshop.list", &app.list_shown());
}
