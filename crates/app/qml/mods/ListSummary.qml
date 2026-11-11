pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Column {
    id: summary
    Appear { of: summary }
    width: parent.width
    spacing: Theme.gapCard
    readonly property var facts: Hub.mods.summary || ({})
    readonly property int changed: (Hub.downloads.since || {}).count || 0
    readonly property int errorCount: Mods.overview.errorCount || 0
    readonly property var firstErrors: Mods.overview.firstErrors || []
    readonly property int moreErrors: Mods.overview.moreErrors || 0
    readonly property var slowest: Mods.overview.slowest || []
    readonly property real keptWidth: Math.max(keptSaves.advanceWidth, keptSnapshots.advanceWidth, keptOwn.advanceWidth, keptCommunity.advanceWidth) + Theme.gap
    TextMetrics { id: keptSaves; font.pixelSize: Theme.body; text: Theme.say("Saves") }
    TextMetrics { id: keptSnapshots; font: keptSaves.font; text: Theme.say("Snapshots") }
    TextMetrics { id: keptOwn; font: keptSaves.font; text: Theme.say("Own rules") }
    TextMetrics { id: keptCommunity; font: keptSaves.font; text: Theme.say("Community rules") }

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
        readonly property bool playable: summary.errorCount === 0 && !Hub.mods.dirty
        readonly property int versionOnly: Mods.overview.versionOnly || 0
        title: !playable ? Theme.say("Before you play") : Hub.mods.warnings === 0 ? Theme.say("Ready to play") : Theme.plural(Hub.mods.warnings, "Playable, {} warning", "Playable, {} warnings")
        tone: summary.errorCount > 0 ? Theme.err : Hub.mods.dirty ? Theme.warn : Hub.mods.warnings > 0 ? Theme.ink : Theme.ok
        note: summary.errorCount > 0 ? summary.errorCount : ""
        spacing: Theme.gap
        Repeater {
            model: summary.firstErrors
            ProblemLine {
                required property var modelData
                width: summary.width
                name: modelData.name
                words: modelData.words
                fix: modelData.fix
                ignore: modelData.ignore
                packageId: modelData.packageId
                modId: modelData.modId
            }
        }
        Line {
            visible: Mods.troubles.length > 0
            text: summary.moreErrors > 0 ? Theme.plural(summary.moreErrors, "{} more problem", "{} more problems") : Hub.mods.warnings === 0 ? "" : verdict.versionOnly === Hub.mods.warnings ? Theme.say("All for the game version") : Theme.plural(Hub.mods.warnings, "{} mod with a warning", "{} mods with a warning")
            action: Theme.say("All problems")
            onActed: Mods.problemsOpen = true
        }
        RowLayout {
            id: launched
            readonly property var launch: Hub.game.lastStart || ({})
            readonly property var logged: launch.errors
            visible: !!Hub.game.lastStart
            width: summary.width
            spacing: Theme.gap
            Text {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: Theme.ink2
                font.pixelSize: Theme.body
                text: launched.launch.line || ""
            }
            Btn { visible: launched.logged > 0; text: Theme.say("Open log"); onClicked: Mods.showSection(Theme.sections.logs) }
        }
        Line { visible: Hub.mods.dirty; text: Theme.fill("Not saved: {}", Hub.mods.unsaved); action: Theme.say("Save"); primary: true; onActed: Mods.save() }
        Line { visible: summary.errorCount === 0 && !Hub.mods.dirty && Mods.troubles.length === 0; text: Theme.say("No problems, and the list is saved.") }
    }

    Card {
        visible: Hub.mods.conflicts + Hub.downloads.behindActive + (summary.facts.replacements || 0) + summary.changed > 0
        title: Theme.say("To review")
        key: "overview-review"
        spacing: Theme.gap
        Line {
            visible: Hub.mods.conflicts > 0
            text: Theme.plural(Hub.mods.conflicts, "{} mod shares textures with another", "{} mods share textures with another")
            action: Theme.say("Show")
            onActed: Mods.filterBy("is:conflict")
        }
        Line {
            visible: summary.changed > 0
            text: Theme.plural(summary.changed, "{} mod changed since the game was last started", "{} mods changed since the game was last started")
            action: Theme.say("Show")
            onActed: Mods.showSection(Theme.sections.downloads)
        }
        Line {
            visible: Hub.downloads.behindActive > 0
            text: Theme.plural(Hub.downloads.behindActive, "{} update available", "{} updates available")
            action: Theme.say("Show")
            onActed: Mods.showSection(Theme.sections.downloads)
        }
        Line {
            visible: (summary.facts.replacements || 0) > 0
            text: Theme.plural((summary.facts.replacements || 0), "{} mod has a replacement", "{} mods have a replacement")
            action: Theme.say("Show")
            onActed: Mods.showSection(Theme.sections.downloads)
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
                Text { x: Theme.gapSmall; width: parent.width - 3 * Theme.gapSmall - took.width; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: slow.modelData.name }
                Text { id: took; anchors.right: parent.right; anchors.rightMargin: Theme.gapSmall; anchors.verticalCenter: parent.verticalCenter; color: Theme.ink2; font.pixelSize: Theme.small; font.features: { "tnum": 1 }; text: slow.modelData.took }
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
        note: folded ? summary.facts.keptNote || "" : ""
        Repeater {
            model: summary.facts.kept || []
            Item {
                id: about
                required property var modelData
                width: summary.width
                height: kept.implicitHeight + Theme.gapTiny
                Text { id: kept; width: summary.keptWidth; color: Theme.ink2; font.pixelSize: Theme.body; text: about.modelData.label }
                Text { x: kept.width; width: parent.width - kept.width; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: about.modelData.value }
            }
        }
        Text {
            visible: Hub.settings.communityProblem !== ""
            width: parent.width
            wrapMode: Text.WordWrap
            color: Theme.err
            font.pixelSize: Theme.body
            text: Theme.fill("Community rules not updated: {}", Hub.settings.communityProblem)
        }
    }
}
