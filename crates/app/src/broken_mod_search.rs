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

use crate::workspace::now_secs;
use cxx_qt_lib::QString;
use rimstack_core::rimworld::saves;
use std::path::PathBuf;

#[path = "broken_mod_search/broken_mod.rs"]
mod broken_mod;

#[derive(Default)]
pub struct BrokenModSearchRust {}
