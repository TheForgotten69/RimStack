import QtQuick
import RimStack.Ui

QtObject {
    id: columns

    required property ModsPrefs prefs
    required property bool loadTimes
    // "" once load times are known.
    required property string loadTimesHow
    required property var orders

    readonly property ModsData rules: Hub.mods
    readonly property var labels: ({ author: "Author", version: "Version", load: "Load", updated: "Updated", downloaded: "Downloaded", activated: "Activated", size: "Size" })
    readonly property var alignedRight: ({ load: true, size: true })
    readonly property var notes: ({ load: loadTimesHow })
    readonly property var chosen: prefs.columns === "" ? [] : prefs.columns.split(",")
    readonly property var shown: chosen.filter(column => column !== "load" || loadTimes)
    // At 100% text, where a column was dragged.
    readonly property var chosenWidths: rules.columnWidths(prefs.columnSizes)
    readonly property var widths: ({
        author: Theme.sized(chosenWidths.author || 0) || Math.max(authorHeading.advanceWidth, authorValue.advanceWidth),
        version: Theme.sized(chosenWidths.version || 0) || Math.max(versionHeading.advanceWidth, versionValue.advanceWidth),
        load: Theme.sized(chosenWidths.load || 0) || Math.max(loadHeading.advanceWidth, loadValue.advanceWidth),
        updated: Theme.sized(chosenWidths.updated || 0) || Math.max(updatedHeading.advanceWidth, agoWidest),
        downloaded: Theme.sized(chosenWidths.downloaded || 0) || Math.max(downloadedHeading.advanceWidth, agoWidest),
        activated: Theme.sized(chosenWidths.activated || 0) || Math.max(activatedHeading.advanceWidth, agoWidest),
        size: Theme.sized(chosenWidths.size || 0) || Math.max(sizeHeading.advanceWidth, sizeValue.advanceWidth) })
    readonly property TextMetrics authorHeading: TextMetrics { font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Author") + " ▾" }
    readonly property TextMetrics authorValue: TextMetrics { font.pixelSize: Theme.body; text: columns.orders.widest.author || "" }
    readonly property TextMetrics versionHeading: TextMetrics { font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Version") + " ▾" }
    readonly property TextMetrics versionValue: TextMetrics { font.pixelSize: Theme.body; text: columns.orders.widest.version || "" }
    readonly property TextMetrics loadHeading: TextMetrics { font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Load") + " ▾" }
    readonly property TextMetrics loadValue: TextMetrics { font.pixelSize: Theme.small; text: columns.orders.widest.load || "" }
    readonly property TextMetrics updatedHeading: TextMetrics { font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Updated") + " ▾" }
    readonly property TextMetrics downloadedHeading: TextMetrics { font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Downloaded") + " ▾" }
    readonly property TextMetrics activatedHeading: TextMetrics { font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Activated") + " ▾" }
    readonly property real agoWidest: Math.max(agoMinutes.advanceWidth, agoHours.advanceWidth, agoDays.advanceWidth, agoMonths.advanceWidth, agoYears.advanceWidth)
    readonly property TextMetrics agoMinutes: TextMetrics { font.pixelSize: Theme.body; text: Theme.plural(59, "{} minute ago", "{} minutes ago") }
    readonly property TextMetrics agoHours: TextMetrics { font.pixelSize: Theme.body; text: Theme.plural(47, "{} hour ago", "{} hours ago") }
    readonly property TextMetrics agoDays: TextMetrics { font.pixelSize: Theme.body; text: Theme.plural(59, "{} day ago", "{} days ago") }
    readonly property TextMetrics agoMonths: TextMetrics { font.pixelSize: Theme.body; text: Theme.plural(11, "{} month ago", "{} months ago") }
    readonly property TextMetrics agoYears: TextMetrics { font.pixelSize: Theme.body; text: Theme.plural(99, "{} year ago", "{} years ago") }
    readonly property TextMetrics sizeHeading: TextMetrics { font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Size") + " ▾" }
    readonly property TextMetrics sizeValue: TextMetrics { font.pixelSize: Theme.small; text: Theme.fill("{} MB", 999) }

    // "" for the load order itself, else as Rust keeps it: "name", "updated down".
    property string order: ""

    function toggle(column) { prefs.columns = rules.columnsToggled(prefs.columns, column) }
    function move(column, to) { prefs.columns = rules.columnsMoved(prefs.columns, column, to) }
    function size(column, width) { prefs.columnSizes = rules.columnSized(prefs.columnSizes, column, width / Theme.grow) }

    readonly property var entries: Object.keys(labels).map(column => ({
        on: chosen.indexOf(column) >= 0,
        label: Theme.say(labels[column]),
        text: (chosen.indexOf(column) >= 0 ? "✓  " : "     ") + Theme.say(labels[column]),
        note: notes[column] || "",
        run: () => toggle(column)
    }))
}
