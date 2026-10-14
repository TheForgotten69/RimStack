// The Mods screen's choices of view, kept between runs.
import QtCore
import QtQuick

Settings {
    category: "Mods"
    property real inactive: 290
    property real details: 360
    property bool inactiveHidden: false
    // The saved list the load order was last taken from or kept as.
    property string listName: ""
    // Where the last run was left: the selected mod by folded package
    // id, the tab of its details, the load order's filter and scroll.
    property string lastMod: ""
    property int lastTab: 0
    property string lastFilter: ""
    property real lastScroll: 0
    // The load order's columns after the name, in order, by key.
    property string columns: "author,version,load"
    property bool inactiveByName: false
    // The load order under category headings: 1 always, 0 never (one
    // numbered list), -1 when the order is close to category order.
    property int headings: -1
    // The columns' widths where they were dragged; see `Mods.columns.chosenWidths`.
    property string columnSizes: ""
}
