//! Asking the web: every request ends by itself.

use std::time::Duration;

/// An agent whose requests, from connecting to the last byte read, take
/// at most this many seconds.
pub(crate) fn agent(limit_secs: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(limit_secs)))
        .build()
        .into()
}

pub(crate) fn failed(e: ureq::Error) -> crate::error::Error {
    crate::error::Error::network(crate::say!("the download failed: {}", e))
}

pub(crate) fn steam_failed(e: impl std::fmt::Display) -> crate::error::Error {
    crate::error::Error::network(crate::say!("Steam could not be asked: {}", e))
}
