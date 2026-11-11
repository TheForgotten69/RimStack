pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

ColumnLayout {
    id: pane
    readonly property var list: Hub.workshop.list
    readonly property var info: Hub.settings.report
    spacing: Theme.gap

    Text {
        visible: pane.list.count === 0
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        color: Theme.ink3
        font.pixelSize: Theme.body
        text: Theme.say("Nothing on the list yet. Add mods from the results.")
    }
    Scrolled {
        visible: pane.list.count > 0
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.preferredHeight: Math.min(contentHeight, Theme.sized(240))
        spacing: Theme.gapTiny
        Repeater {
            model: pane.list.rows
            RowLayout {
                id: row
                required property var modelData
                width: parent.width
                spacing: Theme.gap
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 0
                    spacing: 0
                    Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: row.modelData.title || row.modelData.id }
                    Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink3; font.pixelSize: Theme.small; text: row.modelData.reason }
                    Text { visible: !!row.modelData.failed; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.err; font.pixelSize: Theme.small; text: row.modelData.failed || "" }
                }
                Text { visible: !!row.modelData.sizeSaid; color: Theme.ink3; font.pixelSize: Theme.small; text: row.modelData.sizeSaid || "" }
                Btn { icon: "x"; compact: true; tip: Theme.say("Take off the list"); onClicked: Hub.workshop.remove(row.modelData.id) }
            }
        }
    }
    Text { visible: !!pane.list.action.stuck; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.ink3; font.pixelSize: Theme.small; text: pane.list.action.stuck || "" }
    Flow {
        Layout.fillWidth: true
        spacing: Theme.gap
        Btn {
            visible: pane.list.action.download
            primary: true
            enabled: pane.list.action.enabled
            icon: "download"
            text: pane.list.action.label
            tip: Theme.fill("Into the game's own Mods folder, {}, without subscribing: Steam will not update these.", (pane.info.localMods || ""))
            onClicked: Notices.ask({
                title: Theme.plural(pane.list.count, "Download {} mod with SteamCMD", "Download {} mods with SteamCMD"),
                lines: [Theme.say("Each is downloaded and copied into the game's own Mods folder:"), { path: pane.info.localMods || "" }, Theme.say("Steam does not update mods installed this way.")],
                label: Theme.say("Download"), run: Hub.downloads.downloadWanted })
        }
        Btn {
            visible: pane.list.action.subscribe
            primary: !pane.list.action.download && !pane.list.action.installSteamcmd
            enabled: pane.list.action.subscribeEnabled
            icon: "plus"
            text: Theme.say("Subscribe in Steam")
            tip: Theme.say("Steam must be running. It subscribes to each and downloads it, then keeps them up to date.")
            onClicked: Hub.downloads.subscribeWanted()
        }
        Btn {
            visible: pane.list.action.installSteamcmd
            primary: true
            enabled: pane.list.action.enabled
            icon: "download"
            text: pane.list.action.label
            onClicked: Notices.ask({
                title: Theme.say("Install SteamCMD"),
                lines: [Theme.say("Valve's own downloader is fetched from Valve into"), { path: pane.info.steamcmdFolder || "" }, Theme.say("It runs with a home folder of its own there, apart from Steam's settings.")],
                label: Theme.say("Install"), run: Hub.downloads.installSteamcmd })
        }
        Btn { visible: pane.list.failures; icon: "refresh-cw"; text: Theme.say("Retry"); tip: Theme.say("Forgets what went wrong, so the next download tries these again."); onClicked: Hub.workshop.retry() }
        Btn { visible: pane.list.count > 0; text: Theme.say("Empty the list"); onClicked: Hub.workshop.empty() }
    }
    Item { visible: pane.list.count === 0; Layout.fillHeight: true }
}
