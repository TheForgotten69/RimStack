//! Load orders kept aside: snapshots, named lists, launches and the
//! save copied before one.

use super::*;

impl qobject::Backend {
    pub(super) fn import_rimsort_rules(&self) -> QString {
        let said = match rimsort_rules() {
            Err(problem) => problem.into(),
            Ok(pairs) => {
                let mut yours = crate::app().user_rules().writing();
                let added = yours.import(&pairs);
                match yours.write(&crate::app().user_rules_file()) {
                    Err(problem) => problem.into(),
                    Ok(()) if pairs.is_empty() => say!("RimSort has no rules of your own."),
                    Ok(()) => say!(
                        "{} of RimSort's {} rules were new and are now yours here.",
                        added,
                        pairs.len()
                    ),
                }
            }
        };
        QString::from(&said)
    }

    pub(super) fn record_launch(&self) {
        let list = crate::app().session().current().to_vec();
        if let Err(problem) = launches::record(&launches_file(crate::app()), &list, now_secs()) {
            rimstack_core::problem!("{problem}");
        }
        crate::app().mark_all_seen();
        crate::downloads::since_launch::record_versions();
    }

    pub(super) fn game_running(&self) -> bool {
        let paths = crate::app().paths();
        let game = paths.game.as_deref();
        game.is_some_and(|dir| {
            rimstack_core::upkeep::game_running(std::path::Path::new("/proc"), dir)
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
        match rimstack_core::upkeep::start_game(game, &arguments) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem.to_string()),
        }
    }
}
