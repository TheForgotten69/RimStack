// Nothing selected: the state of the whole load order, as cards that
// each lead somewhere.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Column {
    id: summary
    Appear { of: summary }
    width: parent.width
    spacing: Theme.gapCard
    // Counted again each time this panel shows, and when the lists change.
    readonly property var facts: visible && Mods.backend && !Theme.demo && Mods.active.count >= 0 && Mods.tick >= 0 && Notices.journalled >= 0 ? JSON.parse(Mods.backend.summary()) : ({})
    readonly property int changed: visible && Mods.backend && !Theme.demo ? JSON.parse(Mods.downloads.sinceLaunch()).items.length : 0
    readonly property var errors: Mods.troubles.filter(trouble => trouble.error)
    readonly property var slowest: Mods.overview.slowest || []

    component Line: RowLayout {
        id: line
        property string text
        property string action
        property bool primary: false
        signal acted()
        width: summary.width
        spacing: Theme.gap
        Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.ink; font.pixelSize: Theme.body; text: line.text }
        Btn { visible: line.action !== ""; primary: line.primary; text: line.action; onClicked: line.acted() }
    }

    Column {
        width: parent.width
        Text { color: Theme.ink; font.pixelSize: Theme.heading; font.weight: Font.DemiBold; text: Theme.demo ? Theme.say("Demo data") : Theme.say("Load order") }
        Flow {
            width: parent.width
            topPadding: Theme.gapSmall
            spacing: Theme.gapTiny
            Chip { visible: !Theme.demo; text: Theme.fill("RimWorld {}", (Mods.overview.gameVersion || "")) }
            Chip { text: Theme.fill("{} active", Theme.count(Mods.active.count)) }
            Chip { text: Theme.fill("{} inactive", Theme.count(Mods.inactive.count)) }
            Chip { visible: Mods.fresh.length > 0; tone: Theme.ok; pressable: true; text: Theme.fill("{} new", Mods.fresh.length); tip: Theme.say("Activate the mods installed since the game was last started from RimStack"); onClicked: Mods.activateFresh() }
        }
    }

    Card {
        id: verdict
        visible: !Theme.demo
        // Green is kept for a list with nothing to say about it.
        readonly property bool playable: summary.errors.length === 0 && !Mods.active.dirty
        readonly property int versionOnly: Mods.overview.versionOnly || 0
        title: !playable ? Theme.say("Before you play") : Mods.active.warnings === 0 ? Theme.say("Ready to play") : Theme.plural(Mods.active.warnings, "Playable, {} warning", "Playable, {} warnings")
        tone: summary.errors.length > 0 ? Theme.err : Mods.active.dirty ? Theme.warn : Mods.active.warnings > 0 ? Theme.ink : Theme.ok
        note: summary.errors.length > 0 ? summary.errors.length : ""
        spacing: Theme.gap
        Repeater {
            model: summary.errors.slice(0, 4)
            ProblemLine {
                required property var modelData
                width: summary.width
                name: modelData.name
                words: modelData.words
                fix: modelData.fix
                modId: modelData.modId
            }
        }
        Line {
            visible: Mods.troubles.length > 0
            text: summary.errors.length > 4 ? Theme.plural(summary.errors.length - 4, "{} more problem", "{} more problems") : Mods.active.warnings === 0 ? "" : verdict.versionOnly === Mods.active.warnings ? Theme.say("All for the game version") : Theme.plural(Mods.active.warnings, "{} mod with a warning", "{} mods with a warning")
            action: Theme.say("All problems")
            onActed: Mods.problemsOpen = true
        }
        // How the last attempt went, in one quiet line.
        RowLayout {
            id: launched
            readonly property var launch: Mods.lastStart || ({})
            readonly property var logged: launch.errors
            visible: !!Mods.lastStart
            width: summary.width
            spacing: Theme.gap
            Text {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: Theme.ink2
                font.pixelSize: Theme.body
                text: [Theme.fill("Last started {}", Theme.moment(launched.launch.time || 0)),
                    launched.launch.loadSeconds ? Theme.fill("loaded in {} min {} s", Math.floor(launched.launch.loadSeconds / 60), Math.round(launched.launch.loadSeconds % 60)) : "",
                    launched.logged === null || launched.logged === undefined ? "" : launched.logged === 0 ? Theme.say("no errors") : Theme.plural(launched.logged, "{} different error", "{} different errors"),
                ].filter(part => part !== "").join(", ")
            }
            Btn { visible: launched.logged > 0; text: Theme.say("Open log"); onClicked: Mods.showLogs() }
        }
        Line { visible: Mods.active.dirty; text: Theme.fill("Not saved: {}", Mods.active.unsaved); action: Theme.say("Save"); primary: true; onActed: Mods.save() }
        Line { visible: summary.errors.length === 0 && !Mods.active.dirty && Mods.troubles.length === 0; text: Theme.say("No problems, and the list is saved.") }
    }

    Card {
        visible: Mods.active.conflicts + Mods.behind + (summary.facts.replacements || 0) + summary.changed > 0
        title: Theme.say("To review")
        key: "overview-review"
        spacing: Theme.gap
        Line {
            visible: Mods.active.conflicts > 0
            text: Theme.fill("{} mods share textures with another", Mods.active.conflicts)
            action: Theme.say("Show")
            onActed: Mods.filterBy("is:conflict")
        }
        Line {
            visible: summary.changed > 0
            text: Theme.plural(summary.changed, "{} mod changed since the game was last started", "{} mods changed since the game was last started")
            action: Theme.say("Show")
            onActed: Mods.showUpdates()
        }
        Line {
            visible: Mods.behind > 0
            text: Theme.plural(Mods.behind, "{} update available", "{} updates available")
            action: Theme.say("Show")
            onActed: Mods.showUpdates()
        }
        Line {
            visible: (summary.facts.replacements || 0) > 0
            text: Theme.plural((summary.facts.replacements || 0), "{} mod has a replacement", "{} mods have a replacement")
            action: Theme.say("Show")
            onActed: Mods.showUpdates()
        }
    }

    Card {
        visible: (summary.slowest.length > 0)
        title: Theme.say("Slowest to load")
        key: "overview-slowest"
        Repeater {
            model: summary.slowest
            Item {
                id: slow
                required property var modelData
                width: summary.width
                height: Theme.control
                Rectangle { width: parent.width * slow.modelData.part; height: parent.height; radius: Theme.corner; color: Theme.warn; opacity: 0.14 }
                Text { x: 6; width: parent.width - 70; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: slow.modelData.name }
                Text { anchors.right: parent.right; anchors.rightMargin: Theme.gapSmall; anchors.verticalCenter: parent.verticalCenter; color: Theme.ink2; font.pixelSize: Theme.small; font.features: { "tnum": 1 }; text: slow.modelData.took }
                HoverHandler { cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: Mods.showMod(slow.modelData.modId) }
            }
        }
    }

    Card {
        visible: (!Theme.demo)
        id: keptCard
        title: Theme.say("Kept")
        key: "overview-inventory"
        startsFolded: true
        note: folded ? [summary.facts.saves === undefined ? "" : Theme.plural(summary.facts.saves, "{} save", "{} saves"),
            Theme.plural(summary.facts.snapshots || 0, "{} snapshot", "{} snapshots"),
            Mods.overview.communityMods ? Theme.fill("rules for {} mods", Theme.count(Mods.overview.communityMods)) : "",
        ].filter(part => part !== "").join(", ") : ""
        Repeater {
            model: [
                [Theme.say("Saves"), summary.facts.saves === undefined ? "" : (summary.facts.folders > 1 ? Theme.fill("{} in {} RimSaves folders", summary.facts.saves, summary.facts.folders) : summary.facts.saves + "")],
                [Theme.say("Snapshots"), (summary.facts.snapshots || 0) + ""],
                [Theme.say("Own rules"), (summary.facts.rules || 0) + ""],
                [Theme.say("Community rules"), Mods.overview.communityMods ? Theme.fill("{} mods", Theme.count(Mods.overview.communityMods)) : Theme.say("Not downloaded")],
            ].filter(pair => pair[1] !== "")
            Item {
                id: about
                required property var modelData
                width: summary.width
                height: Theme.sized(22)
                Text { id: kept; width: Theme.sized(126); color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.say(about.modelData[0]) }
                Text { x: kept.width; width: parent.width - kept.width; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: about.modelData[1] }
            }
        }
        Text {
            visible: Mods.communityData.rulesProblem !== ""
            width: parent.width
            wrapMode: Text.WordWrap
            color: Theme.err
            font.pixelSize: Theme.body
            text: Theme.fill("Community rules not updated: {}", Mods.communityData.rulesProblem)
        }
    }
}
