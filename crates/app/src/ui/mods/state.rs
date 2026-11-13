//! The Mods tab's state, by what it is about.

use super::{chosen, columns, hints, notes};
use crate::jobs::Lane;
use gpui::Pixels;
use rimstack_core::mods::library::ModId;
use std::rc::Rc;

/// What Save, upkeep and the folder chosen last ran into, and Sort's note.
#[derive(Default)]
pub struct Status {
    pub save_problem: String,
    /// Why a mod was not binned or copied to the game's folder.
    pub upkeep_problem: String,
    /// ModsConfig.xml was written outside RimStack since it was read: Save refused to overwrite it.
    pub file_changed: bool,
    /// Why the game folder chosen was not taken.
    pub folder_problem: String,
    pub unhide_problem: String,
    /// The Sort note closed, by its words; and the one whose closing is on its way.
    pub closed_note: String,
    pub note_timer: String,
}

/// The panes' widths, the drawer and what is dragged or placed in them.
pub struct Layout {
    pub inactive_width: Pixels,
    pub details_width: Pixels,
    /// The inactive mods shown over the load order, from the rail.
    pub drawer_open: bool,
    /// The turn of the drawer's last opening, which slides it in.
    pub drawer_turn: usize,
    /// Where a heading or a column's edge was pressed, and the column's width then.
    pub column_drag: Option<(Pixels, Pixels)>,
    /// Where the View and the inactive order buttons were drawn.
    pub menu_buttons: columns::Placed,
}

impl Layout {
    pub fn new(inactive_width: Pixels, details_width: Pixels) -> Self {
        Self {
            inactive_width,
            details_width,
            drawer_open: false,
            drawer_turn: 0,
            column_drag: None,
            menu_buttons: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct Hinting {
    /// The first-run hint shown, 1 to 3; 0 for none.
    pub step: u8,
    pub anchors: hints::Anchors,
}

/// What is read of the chosen mod off the window's thread, and its notes being written.
#[derive(Default)]
pub struct ChosenReads {
    pub read: Lane,
    /// What the Files tab shows of the chosen mod.
    pub files: Option<(ModId, Rc<chosen::FilesRead>)>,
    /// A Git checkout's last commits, for its Latest changes.
    pub commits: Option<(ModId, Rc<Vec<rimstack_core::sources::changes::Note>>)>,
    pub repository: Option<(ModId, String)>,
    pub commits_read: Lane,
    /// The other mod of a rule of your own being chosen.
    pub picking: Option<notes::Picking>,
    pub drafts: notes::Drafts,
    /// "Saved" shows beside the notes for a while after they are kept.
    pub notes_saved: bool,
}

/// What the lists' state counts for the details.
#[derive(Default)]
pub struct Reads {
    pub conflicts: usize,
    /// Inactive mods installed since the game was last started from RimStack.
    pub fresh: usize,
}

/// The arrows beside the problems.
#[derive(Default)]
pub struct ProblemNav {
    /// The row of the problem last shown.
    pub row: Option<usize>,
    pub open: bool,
}
