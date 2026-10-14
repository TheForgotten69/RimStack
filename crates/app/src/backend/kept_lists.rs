//! Load orders kept aside: snapshots, named lists, launches and the
//! save copied before one.

use super::*;

impl qobject::Backend {
    pub(super) fn record_launch(&self) {
        let list = crate::app().session().current().to_vec();
        if let Err(problem) = launches::record(&launches_file(crate::app()), &list, now_secs()) {
            rimstack_core::problem!("{problem}");
        }
        crate::app().mark_all_seen();
        crate::downloads::since_launch::record_versions(crate::app());
    }

    pub(super) fn game_running(&self) -> bool {
        let paths = crate::app().paths();
        let game = paths.game.as_deref();
        game.is_some_and(|dir| {
            rimstack_core::sources::maintenance::game_running(std::path::Path::new("/proc"), dir)
        })
    }

    pub(super) fn start_game_directly(&self) -> QString {
        let paths = crate::app().paths();
        let Some(game) = paths.game.as_ref() else {
            return QString::from(&say!("The game's folder is not known"));
        };
        let arguments = crate::app()
            .settings()
            .get("launchArguments")
            .unwrap_or_default()
            .to_string();
        match rimstack_core::sources::maintenance::start_game(game, &arguments) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem.to_string()),
        }
    }
}
