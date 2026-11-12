//! A made-up install for the tests that need a game: `tools/fixture-game.sh` and
//! `tools/fixture-downloads.sh`, all in a scratch folder.

use crate::App;
use std::time::Duration;

pub(crate) fn made_up() -> (tempfile::TempDir, App) {
    use rimstack_core::base::program;
    use rimstack_core::rimworld::paths::Overrides;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let tools = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let fx = root.join("fx");
    let run = |script: &str, args: &[&std::path::Path]| {
        let mut command = program::command(tools.join(script));
        command.args(args);
        let done = program::run(&mut command, Duration::from_secs(60)).unwrap();
        assert!(
            done.status.success(),
            "{script}: {}",
            String::from_utf8_lossy(&done.stderr)
        );
    };
    run("fixture-game.sh", &[&fx]);
    run("fixture-downloads.sh", &[&fx, &root.join("data/RimStack")]);
    let given = Overrides {
        game: Some(fx.join("game")),
        config: Some(fx.join("config")),
        workshop: Some(fx.join("steam/workshop/content/294100")),
    };
    let app = App::new(
        root.join("home"),
        &root.join("settings"),
        &root.join("data"),
        given,
    );
    (tmp, app)
}
