//! The one object QML imports: it is handed the `App`, makes one object
//! per part of the window on it, and owns them for as long as it lives.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("rimstack-app/src/settings_data.cxxqt.h");
        type SettingsData = crate::settings_data::qobject::SettingsData;
        include!("rimstack-app/src/mods_data.cxxqt.h");
        type ModsData = crate::mods_data::qobject::ModsData;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(*mut SettingsData, settings, READ, CONSTANT)]
        #[qproperty(*mut ModsData, mods, READ, CONSTANT)]
        type Hub = super::HubRust;
    }

    impl cxx_qt::Initialize for Hub {}
}

use core::pin::Pin;
use cxx::UniquePtr;
use cxx_qt::CxxQtType;
use qobject::{ModsData, SettingsData};

pub struct HubRust {
    settings: *mut SettingsData,
    mods: *mut ModsData,
    owned_settings: UniquePtr<SettingsData>,
    owned_mods: UniquePtr<ModsData>,
}

impl Default for HubRust {
    fn default() -> Self {
        Self {
            settings: std::ptr::null_mut(),
            mods: std::ptr::null_mut(),
            owned_settings: UniquePtr::null(),
            owned_mods: UniquePtr::null(),
        }
    }
}

fn given(flag: &str) -> bool {
    std::env::args().any(|a| a == flag || a.starts_with(&format!("{flag}=")))
}

/// What QML holds of an object the Hub owns: its address, valid for as
/// long as the Hub lives.
pub(crate) fn address<T>(owned: &mut UniquePtr<T>) -> *mut T
where
    T: cxx::memory::UniquePtrTarget,
{
    owned.as_mut().map_or(std::ptr::null_mut(), |pinned| {
        // SAFETY: the address is only handed to QML; the object stays
        // pinned inside the `UniquePtr` the Hub keeps.
        unsafe { Pin::get_unchecked_mut(pinned) as *mut T }
    })
}

impl cxx_qt::Initialize for qobject::Hub {
    fn initialize(mut self: Pin<&mut Self>) {
        let app = crate::workspace::shared();
        let mut settings = crate::settings_data::qobject::make_settings_data();
        if let Some(settings) = settings.as_mut() {
            settings.connect(app.clone());
        }
        // Made-up mods are shown and nothing is written for them; while
        // checked, no snapshot or history is kept.
        let demo = given("--demo") || given("--selftest");
        let mut mods = crate::mods_data::qobject::make_mods_data();
        if let Some(mods) = mods.as_mut() {
            mods.connect(app.clone(), demo, !given("--check"));
        }
        let mut hub = self.as_mut().rust_mut();
        hub.settings = address(&mut settings);
        hub.owned_settings = settings;
        hub.mods = address(&mut mods);
        hub.owned_mods = mods;
    }
}
