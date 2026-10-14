//! What RimStack knows of the game's own code, read once from the game's
//! `Assembly-CSharp.dll`: which of its types draw, and for each of its
//! methods when the game runs it.
//!
//! "When" comes from the game's code alone. Unity, the engine under the
//! game, calls a handful of methods by name on the objects it keeps
//! alive: `Update` every frame, `OnGUI` to draw the interface, `Start`
//! once. Everything else runs because one of those calls it, and `call_graph`
//! follows the calls.

use crate::rimworld::call_graph::{Graph, NEVER};
use crate::rimworld::dotnet::{Assembly, Base, Layout, Type};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// The type Unity calls by name: every object it keeps alive extends it.
const UNITY_OBJECT: &str = "UnityEngine.MonoBehaviour";
/// The methods Unity calls on such an object, by when it does.
const EVERY_FRAME: [&str; 3] = ["Update", "LateUpdate", "FixedUpdate"];
const TO_DRAW: [&str; 1] = ["OnGUI"];
const AT_START: [&str; 3] = ["Awake", "Start", "OnEnable"];

/// A type draws when at least this share of its methods take a `Rect`.
const DRAWING_SHARE: f32 = 0.2;

fn draws_itself(t: &Type) -> bool {
    let drawing = t.methods.iter().filter(|m| m.takes_rect).count();
    drawing > 0 && drawing as f32 >= t.methods.len() as f32 * DRAWING_SHARE
}

/// Which of these types draw: by their own methods, or by extending one
/// of them or one of `known` (types of another assembly that draw).
///
/// "Draws" is taken from how the game is drawn: with Unity's
/// immediate-mode GUI, where a method that draws is handed the rectangle
/// to draw in.
pub(crate) fn drawing(types: &[Type], known: &HashSet<String>) -> HashSet<String> {
    let mut found: HashSet<String> = types
        .iter()
        .filter(|t| draws_itself(t))
        .map(|t| t.name.clone())
        .collect();
    // Down the family trees, until a pass adds nothing.
    loop {
        let more: Vec<String> = types
            .iter()
            .filter(|t| !found.contains(&t.name))
            .filter(|t| {
                let parent = t.extends.as_ref();
                parent.is_some_and(|p| found.contains(p) || known.contains(p))
            })
            .map(|t| t.name.clone())
            .collect();
        if more.is_empty() {
            return found;
        }
        found.extend(more);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum When {
    Playing,
    /// Only while the interface is drawn or used, or it is a method of
    /// a type that draws.
    Interface,
    Loading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Runs {
    /// `None` when nothing the engine calls leads to it: it runs from
    /// code the game finds by name (a def's worker, a debug action).
    pub when: Option<When>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Known {
    extends: Option<String>,
    /// Its methods by name; overloads count as one.
    methods: HashMap<String, Runs>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Game {
    draws: HashSet<String>,
    types: HashMap<String, Known>,
}

fn entry_points(layout: &Layout, names: &[&str]) -> Vec<usize> {
    let unity = |owner: usize| {
        let mut chain =
            std::iter::once(owner).chain(crate::rimworld::call_graph::ancestors(layout, owner));
        chain.any(|t| matches!(&layout.types[t].extends, Base::Other(n) if n == UNITY_OBJECT))
    };
    let called = |(_, m): &(usize, &crate::rimworld::dotnet::MethodDef)| {
        names.contains(&m.name) && unity(m.owner)
    };
    let all = layout.methods.iter().enumerate();
    all.filter(called).map(|(at, _)| at).collect()
}

impl Game {
    pub fn read(assembly_csharp: &[u8]) -> Option<Self> {
        let assembly = Assembly::read(assembly_csharp)?;
        let layout = assembly.layout();
        let graph = Graph::of(&assembly, &layout);
        let steps = |names: &[&str]| graph.steps_from(&entry_points(&layout, names));
        let (playing, drawing_it, loading) =
            (steps(&EVERY_FRAME), steps(&TO_DRAW), steps(&AT_START));
        let draws = drawing(&assembly.types(), &HashSet::new());
        let when = |method: usize| {
            // A type that draws is interface whatever opens it: the game
            // opens windows from its every-frame code too.
            if draws.contains(&layout.types[layout.methods[method].owner].name) {
                return Some(When::Interface);
            }
            if playing[method] != NEVER {
                return Some(When::Playing);
            }
            // Reached from both: the nearer one.
            match (drawing_it[method], loading[method]) {
                (NEVER, NEVER) => None,
                (draw, load) if draw <= load => Some(When::Interface),
                _ => Some(When::Loading),
            }
        };
        let mut types: HashMap<String, Known> = HashMap::with_capacity(layout.types.len());
        for t in &layout.types {
            let known = types.entry(t.name.clone()).or_default();
            if let Base::Own(parent) = t.extends {
                known.extends = Some(layout.types[parent].name.clone());
            }
            for at in t.methods.clone() {
                let runs = Runs { when: when(at) };
                let name = layout.methods[at].name;
                match known.methods.get_mut(name) {
                    Some(same_name) => same_name.merge(runs),
                    None => drop(known.methods.insert(name.to_string(), runs)),
                }
            }
        }
        Some(Self { draws, types })
    }

    /// The game's code is `Assembly-CSharp.dll`. From 1.7 it is in the
    /// game's folder itself; up to 1.6, under Unity, in the `Managed`
    /// folder of the game's data: `RimWorldLinux_Data`,
    /// `RimWorldWin64_Data`, or inside the application on a Mac.
    pub fn find(game_dir: &Path) -> Option<Self> {
        let inside = std::fs::read_dir(game_dir)
            .ok()?
            .flatten()
            .map(|e| e.path());
        let mut below: Vec<PathBuf> = inside
            .filter(|p| p.is_dir())
            .flat_map(|p| [p.join("Managed"), p.join("Contents/Resources/Data/Managed")])
            .collect();
        below.sort();
        let places: Vec<PathBuf> = std::iter::once(game_dir.to_path_buf())
            .chain(below)
            .collect();
        let dll = places
            .iter()
            .map(|p| p.join("Assembly-CSharp.dll"))
            .find(|f| f.is_file())?;
        Self::read(&std::fs::read(dll).ok()?)
    }

    pub(crate) fn drawing_types(&self) -> &HashSet<String> {
        &self.draws
    }

    pub fn has(&self, type_name: &str) -> bool {
        self.types.contains_key(type_name)
    }

    /// How the game runs a method of one of its types: the type's own,
    /// or the one it has from a type above it. `None` if the game has no
    /// such method.
    pub fn method(&self, type_name: &str, method: &str) -> Option<Runs> {
        let mut at = self.types.get(type_name)?;
        for _ in 0..64 {
            if let Some(runs) = at.methods.get(method) {
                return Some(*runs);
            }
            at = self.types.get(at.extends.as_deref()?)?;
        }
        None
    }
}

impl Runs {
    /// Two methods of one name count as one: it plays if either does.
    fn merge(&mut self, other: Runs) {
        self.when = match (self.when, other.when) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }
}

#[cfg(test)]
impl Game {
    /// A game of these types, whose methods run when `runs` says, as
    /// (type, method, when); the others never.
    pub(crate) fn of(types: &[Type], runs: &[(&str, &str, When)]) -> Self {
        let when = |t: &Type, method: &str| {
            let said = |r: &&(&str, &str, When)| r.0 == t.name && r.1 == method;
            runs.iter().find(said).map(|r| r.2)
        };
        let known = |t: &Type| Known {
            extends: t.extends.clone(),
            methods: t
                .methods
                .iter()
                .map(|m| {
                    let runs = Runs {
                        when: when(t, &m.name),
                    };
                    (m.name.clone(), runs)
                })
                .collect(),
        };
        Self {
            draws: drawing(types, &HashSet::new()),
            types: types.iter().map(|t| (t.name.clone(), known(t))).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rimworld::dotnet::build::{op, Builder, NEW, VIRTUAL};

    const CALL: u16 = 0x28;
    const METHOD: usize = 0x06;

    /// A game whose root object is called by Unity: every frame it
    /// ticks, to draw it fills a window, at the start it loads; all
    /// three write to the log.
    fn game() -> Game {
        let mut b = Builder::new();
        let unity = b.type_ref("UnityEngine", "MonoBehaviour");
        let root = b.type_def("Verse", "Root", unity);
        let calls = |a: u32, b: u32| [op(CALL, METHOD, a), op(CALL, METHOD, b)].concat();
        b.method_with("Update", 0, &calls(4, 8));
        b.method_with("OnGUI", 0, &calls(6, 8));
        b.method_with("Start", 0, &calls(7, 8));
        b.type_def_under("Verse", "Root_Play", root);
        let thing = b.type_def("Verse", "Thing", 0);
        b.method_with("Tick", VIRTUAL | NEW, &[0x2a]);
        b.method_with("Update", 0, &[0x2a]);
        b.type_def("Verse", "Game", 0);
        b.method_with("FillWindow", 0, &[0x2a]);
        b.method_with("LoadDefs", 0, &[0x2a]);
        b.method_with("Log", 0, &[0x2a]);
        b.method_with("FillWindow", 0, &[0x2a]);
        b.method_with("DebugAction", 0, &[0x2a]);
        b.type_def_under("Verse", "Pawn", thing);
        Game::read(&b.finish()).unwrap()
    }

    fn when(game: &Game, type_name: &str, method: &str) -> Option<When> {
        game.method(type_name, method).unwrap().when
    }

    #[test]
    fn a_method_runs_when_what_unity_calls_leads_to_it() {
        let game = game();
        assert_eq!(when(&game, "Verse.Thing", "Tick"), Some(When::Playing));
        assert_eq!(
            when(&game, "Verse.Game", "FillWindow"),
            Some(When::Interface)
        );
        assert_eq!(when(&game, "Verse.Game", "LoadDefs"), Some(When::Loading));
        assert_eq!(when(&game, "Verse.Game", "DebugAction"), None);
        // What every frame leads to plays, whatever else leads to it.
        assert_eq!(when(&game, "Verse.Game", "Log"), Some(When::Playing));
    }

    #[test]
    fn a_method_of_a_type_that_draws_is_interface_whatever_calls_it() {
        let mut b = Builder::new();
        let unity = b.type_ref("UnityEngine", "MonoBehaviour");
        let rect = b.type_ref("UnityEngine", "Rect");
        b.type_def("Verse", "Root", unity);
        b.method_with("Update", 0, &op(CALL, METHOD, 3));
        b.type_def("Verse", "Window", 0);
        // Takes a `Rect`: the type draws.
        b.method(
            "DoWindowContents",
            &[0x20, 1, 0x01, 0x11, (rect << 2 | 1) as u8],
        );
        b.method_with("PreOpen", 0, &[0x2a]);
        let game = Game::read(&b.finish()).unwrap();
        assert_eq!(
            when(&game, "Verse.Window", "PreOpen"),
            Some(When::Interface)
        );
        assert_eq!(when(&game, "Verse.Root", "Update"), Some(When::Playing));
    }

    #[test]
    fn unity_calls_by_name_only_the_objects_it_keeps() {
        // `Thing.Update` has the name, but `Thing` is no object of Unity's.
        assert_eq!(when(&game(), "Verse.Thing", "Update"), None);
    }

    #[test]
    fn a_method_is_found_on_the_type_or_above_it() {
        let game = game();
        assert_eq!(when(&game, "Verse.Pawn", "Tick"), Some(When::Playing));
        assert_eq!(
            when(&game, "Verse.Root_Play", "OnGUI"),
            Some(When::Interface)
        );
        assert!(game.method("Verse.Pawn", "Fly").is_none());
        assert!(game.method("Not.There", "Tick").is_none());
        assert!(game.has("Verse.Pawn") && !game.has("Not.There"));
    }

    #[test]
    fn a_file_that_is_not_the_game_gives_nothing() {
        assert!(Game::read(b"not an assembly").is_none());
    }

    /// 1.7 left Unity, and the game's methods are still dated from
    /// `Update`, `OnGUI` and `Start`: this checks the installed game keeps
    /// those names.
    #[test]
    #[ignore = "needs RimWorld installed"]
    fn the_installed_game_s_methods_are_dated_from_its_entry_points() {
        use crate::rimworld::paths::{detect, Overrides};
        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        let paths = detect(&home, &Overrides::from_env());
        let dir = paths.game.expect("the game is installed");
        println!("game {:?} at {}", paths.game_version, dir.display());
        let started = std::time::Instant::now();
        let game = Game::find(&dir).expect("Assembly-CSharp.dll read");
        println!("read in {:?}", started.elapsed());
        let probes = [
            ("Verse.TickManager", "DoSingleTick"),
            ("Verse.Pawn", "Tick"),
            ("Verse.Thing", "Tick"),
            ("Verse.Root", "Update"),
            ("Verse.Root", "OnGUI"),
            ("Verse.UIRoot", "UIRootOnGUI"),
            ("Verse.WindowStack", "WindowStackOnGUI"),
            ("RimWorld.MainTabWindow_Work", "DoWindowContents"),
            ("Verse.Game", "LoadGame"),
        ];
        let mut dated = 0;
        for (ty, method) in probes {
            let runs = game.method(ty, method);
            println!("{ty}.{method}: {runs:?}");
            dated += usize::from(runs.is_some_and(|r| r.when.is_some()));
        }
        assert!(
            dated >= probes.len() / 2,
            "{dated} of {} known game methods have a time",
            probes.len()
        );
    }
}
