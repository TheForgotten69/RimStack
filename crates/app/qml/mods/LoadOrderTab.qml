// Where the selected mod loads and why: its position, what it requires,
// rules of your own, and what the other rules say.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Column {
    id: side
    width: parent.width
    visible: Mods.tab === 1
    GroupTitle { text: Theme.say("Position") }
    Text {
        width: parent.width
        wrapMode: Text.WordWrap
        color: Theme.ink
        font.pixelSize: Theme.body
        text: Mods.selectedActive ? Theme.fill(Mods.info.category ? "{} of {}, in {}" : "{} of {}", Mods.active.indexOf(Mods.selectedId) + 1, Mods.active.count, Theme.say(Mods.info.category || "")) : Theme.say("Not active")
    }
    NameList { width: parent.width; title: Theme.say("Requires"); lines: Mods.info.requires || "" }
    GroupTitle { visible: Mods.selectedActive; text: Theme.say("Own rules") }
    Repeater {
        model: Mods.selectedActive && Mods.info.yourRules ? Mods.info.yourRules.split("\n") : []
        RowLayout {
            id: own
            required property string modelData
            readonly property var parts: modelData.split("\t")
            width: side.width
            height: Theme.sized(26)
            spacing: Theme.gap
            Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: Theme.fill("Loads {} {}", own.parts[2], own.parts[1]) }
            Btn {
                icon: "x"
                tip: Theme.say("Remove this rule")
                onClicked: own.parts[2] === "after" ? Mods.active.removeRule(Number(own.parts[0]), Mods.selectedId) : Mods.active.removeRule(Mods.selectedId, Number(own.parts[0]))
            }
        }
    }
    Text {
        visible: Mods.selectedActive && Mods.picking !== ""
        width: parent.width
        bottomPadding: Theme.gapSmall
        wrapMode: Text.WordWrap
        color: Theme.moved
        font.pixelSize: Theme.body
        lineHeight: 1.3
        text: Theme.fill("Click the mod this one loads {} (Esc cancels)", Mods.picking)
    }
    Flow {
        visible: Mods.selectedActive
        width: parent.width
        spacing: Theme.gapSmall
        Btn { visible: Mods.picking === ""; icon: "plus"; text: Theme.say("Load after…"); onClicked: Mods.picking = "after" }
        Btn { visible: Mods.picking === ""; icon: "plus"; text: Theme.say("Load before…"); onClicked: Mods.picking = "before" }
        Btn { visible: Mods.picking !== ""; text: Theme.say("Cancel"); onClicked: Mods.picking = "" }
    }
    // What Requires lists just above is counted here, not listed again.
    NameList {
        readonly property var required: (Mods.info.requires || "").split("\n").filter(line => line !== "").map(line => line.replace(/^!/, "").split("\t")[0])
        readonly property var others: (Mods.info.loadsAfter || "").split("\n").filter(line => line !== "" && required.indexOf(line.replace(/^!/, "").split("\t")[0]) < 0)
        readonly property int repeated: (Mods.info.loadsAfter || "").split("\n").filter(line => line !== "").length - others.length
        width: parent.width
        title: Theme.say("Loads after")
        lead: repeated > 0 ? Theme.plural(repeated, "{} required, listed above", "{} required, listed above") : ""
        lines: others.join("\n")
    }
    NameList { width: parent.width; title: Theme.say("Loads before"); lines: Mods.info.loadsBefore || "" }
    GroupTitle { visible: detected.count > 0; text: Theme.say("Detected from files") }
    Repeater {
        id: detected
        model: Mods.info.detected ? Mods.info.detected.split("\n") : []
        Column {
            id: found
            required property string modelData
            readonly property var parts: modelData.split("\t")
            width: side.width
            bottomPadding: Theme.gapSmall
            Text { width: parent.width; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: Theme.fill("After {}", found.parts[0]) }
            Text {
                width: parent.width
                wrapMode: Text.WrapAnywhere
                color: Theme.ink3
                font.pixelSize: Theme.small
                text: found.parts[2] === "hint" ? Theme.fill("{}  (a hint; the game does not need this order)", found.parts[1]) : found.parts[1]
            }
        }
    }
    GroupTitle { visible: !!Mods.info.placement; text: Theme.say("Placement") }
    Text { visible: !!Mods.info.placement; width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink; font.pixelSize: Theme.body; lineHeight: 1.3; text: Mods.info.placement || "" }
}
