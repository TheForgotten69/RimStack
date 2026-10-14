//! An installed mod as one reading of the mod folders knows it. The
//! folders are read again whenever they change, and the same place in
//! the new reading may hold another mod: an id from before then finds
//! nothing, rather than the wrong mod.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::atomic::{AtomicU32, Ordering};

/// The window holds an id as one non-negative 32-bit number: the place
/// in the low 16 bits, the reading in the 15 above. A reading of more
/// mods than this names the rest with -1, none.
pub const MOST_MODS: usize = 1 << 16;
/// Readings told apart before the count starts again: far more than
/// one run of the program reads.
const READINGS: u32 = 1 << 15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModId {
    index: usize,
    reading: u32,
}

impl ModId {
    /// The mod at `index` in `Library::installed` of the reading
    /// `reading`. Only a library being built, and tests, make one.
    pub fn new(reading: u32, index: usize) -> Self {
        Self {
            index,
            reading: reading % READINGS,
        }
    }

    pub(crate) fn index(self) -> usize {
        self.index
    }

    pub(crate) fn reading(self) -> u32 {
        self.reading
    }

    /// As the window holds it.
    pub fn to_qml(self) -> i32 {
        match self.index < MOST_MODS {
            true => ((self.reading << 16) as i32) | self.index as i32,
            false => -1,
        }
    }

    /// From the window; a negative number is no mod.
    pub fn from_qml(held: i32) -> Option<Self> {
        let held = u32::try_from(held).ok()?;
        Some(Self::new(held >> 16, (held & 0xffff) as usize))
    }
}

/// The reading a new library is: each is another.
pub(crate) fn next_reading() -> u32 {
    static READ: AtomicU32 = AtomicU32::new(1);
    READ.fetch_add(1, Ordering::Relaxed) % READINGS
}

impl Serialize for ModId {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        to.serialize_i32(self.to_qml())
    }
}

impl<'de> Deserialize<'de> for ModId {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let held = i32::deserialize(from)?;
        Self::from_qml(held).ok_or_else(|| serde::de::Error::custom("no mod"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_goes_to_the_window_and_back_unchanged() {
        for id in [
            ModId::new(0, 0),
            ModId::new(7, 2_300),
            ModId::new(READINGS - 1, MOST_MODS - 1),
        ] {
            assert!(id.to_qml() >= 0);
            assert_eq!(ModId::from_qml(id.to_qml()), Some(id));
            let said = serde_json::to_string(&id).unwrap();
            assert_eq!(serde_json::from_str::<ModId>(&said).unwrap(), id);
        }
        assert_eq!(ModId::from_qml(-1), None);
        assert!(serde_json::from_str::<ModId>("-1").is_err());
    }

    #[test]
    fn the_same_place_in_two_readings_is_two_ids() {
        let (first, second) = (next_reading(), next_reading());
        assert_ne!(ModId::new(first, 3), ModId::new(second, 3));
        assert_ne!(
            ModId::new(first, 3).to_qml(),
            ModId::new(second, 3).to_qml()
        );
    }

    #[test]
    fn a_place_past_the_most_mods_is_no_mod_to_the_window() {
        assert_eq!(ModId::new(1, MOST_MODS).to_qml(), -1);
    }
}
