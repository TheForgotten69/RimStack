//! Textures: what the Textures screen lists, converting and auditing them,
//! and choosing between two mods' versions of one.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, textures)]
        #[qproperty(bool, textures_busy, cxx_name = "texturesBusy")]
        #[qproperty(QString, textures_progress, cxx_name = "texturesProgress")]
        type TextureTools = super::TextureToolsRust;
    }

    impl cxx_qt::Threading for TextureTools {}

    extern "RustQt" {
        /// Count every installed mod's textures and what stands beside
        /// them. Fills `textures` with a JSON array of the mods that have any.
        #[qinvokable]
        #[cxx_name = "loadTextures"]
        fn load_textures(self: Pin<&mut TextureTools>);

        /// Make DDS files for the mods with these library ids (a JSON
        /// array), or delete the ones RimStack made. `texturesProgress`
        /// says how far it is; the list is counted again at the end.
        #[qinvokable]
        #[cxx_name = "convertTextures"]
        fn convert_textures(self: Pin<&mut TextureTools>, mod_ids: &QString);

        #[qinvokable]
        #[cxx_name = "removeTextures"]
        fn remove_textures(self: Pin<&mut TextureTools>, mod_ids: &QString);

        /// A conversion of textures stops after the mod it is on.
        #[qinvokable]
        #[cxx_name = "cancelTextures"]
        fn cancel_textures(self: &TextureTools);

        #[qinvokable]
        #[cxx_name = "skipTextures"]
        fn skip_textures(self: Pin<&mut TextureTools>, mod_id: i32, skip: bool);

        /// The DDS files of the installed mods the game would refuse, as a
        /// JSON array of { mod, file, why }.
        #[qinvokable]
        #[cxx_name = "ddsAudit"]
        fn dds_audit(self: &TextureTools) -> QString;

        /// Renames these DDS files (a JSON array of paths) out of the
        /// game's way. Returns how many could not be.
        #[qinvokable]
        #[cxx_name = "setDdsAside"]
        fn set_dds_aside(self: &TextureTools, files: &QString) -> i32;

        /// The textures two active mods share, as a JSON object. Give the
        /// mod that loads earlier first (both by library id).
        #[qinvokable]
        fn compare(self: &TextureTools, earlier: i32, later: i32) -> QString;

        /// Choose which of two mods' versions of a texture the game uses,
        /// given both files. Keeping the earlier mod's hides the later
        /// mod's file by renaming it; keeping the later one's puts every
        /// file back. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "keepTexture"]
        fn keep_texture(
            self: &TextureTools,
            earlier_file: &QString,
            later_file: &QString,
            keep_earlier: bool,
        ) -> QString;

        #[qinvokable]
        #[cxx_name = "hiddenIn"]
        fn hidden_in(self: &TextureTools, mod_id: i32) -> i32;

        /// Put every hidden texture of a mod back. Returns what went
        /// wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "unhideAll"]
        fn unhide_all(self: &TextureTools, mod_id: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "unhideIn"]
        fn unhide_in(self: &TextureTools, folder: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "textureDifference"]
        fn texture_difference(self: &TextureTools, earlier: &QString, later: &QString) -> QString;

        /// Drop what was worked out from the mod folders as they were read
        /// before; the screen asks again when it is next shown.
        #[qinvokable]
        fn forget(self: Pin<&mut TextureTools>);
    }
}

use crate::backend::{hidden_file, ids_from, loaded_folders};
use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rimstack_core::mods::hidden::Hidden;
use rimstack_core::mods::library::{Library, ModId};
use rimstack_core::{mods::conflicts, mods::textures, say_n};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "texture_tools/texture_report.rs"]
mod texture_report;
#[path = "texture_tools/texture_work.rs"]
mod texture_work;
pub(crate) use texture_report::manifest_file;
use texture_report::*;

/// Set while a conversion should stop after the mod it is on.
static STOP_TEXTURES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Default)]
pub struct TextureToolsRust {
    textures: QString,
    textures_busy: bool,
    textures_progress: QString,
}

impl qobject::TextureTools {
    fn cancel_textures(&self) {
        STOP_TEXTURES.store(true, Ordering::Relaxed);
    }

    /// Brings back the textures RimStack set aside in this mod folder.
    fn unhide_in(&self, folder: &QString) -> QString {
        let file = hidden_file(crate::app());
        let mut hidden = Hidden::read(&file);
        let path = PathBuf::from(folder.to_string());
        let done = hidden.unhide_all(&path).and_then(|_| hidden.write(&file));
        crate::app().textures_changed();
        QString::from(&done.err().map(String::from).unwrap_or_default())
    }

    fn forget(mut self: Pin<&mut Self>) {
        self.as_mut().set_textures(QString::default());
    }
}
