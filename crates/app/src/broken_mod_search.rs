//! The search for the one mod that breaks the game, by halves.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type BrokenModSearch = super::BrokenModSearchRust;
    }

    impl cxx_qt::Threading for BrokenModSearch {}

    extern "RustQt" {
        /// The search for the one mod that breaks the game, as JSON; see
        /// `broken_mod::state`.
        #[qinvokable]
        #[cxx_name = "huntState"]
        fn hunt_state(self: &BrokenModSearch) -> QString;

        #[qinvokable]
        #[cxx_name = "huntStart"]
        fn hunt_start(self: &BrokenModSearch) -> QString;

        #[qinvokable]
        #[cxx_name = "huntAnswer"]
        fn hunt_answer(self: &BrokenModSearch, broken: bool) -> QString;

        #[qinvokable]
        #[cxx_name = "huntStop"]
        fn hunt_stop(self: &BrokenModSearch);
    }
}

use crate::backend::now_secs;
use crate::backend::save_report::saves_dir;
use cxx_qt_lib::QString;
use rimstack_core::saves;
use serde_json::json;
use std::path::PathBuf;

#[path = "broken_mod_search/broken_mod.rs"]
mod broken_mod;

#[derive(Default)]
pub struct BrokenModSearchRust {}
