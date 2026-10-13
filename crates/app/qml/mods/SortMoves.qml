// The sort on show: every mod it moves, under the reason for the move.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Column {
    id: moves
    width: parent.width
    visible: Mods.active.previewing && Mods.comparing === null
    spacing: Theme.gapHair
    // "name<tab>from<tab>to<tab>library id<tab>reason's group<tab>reason"
    // per moved mod, in the new order.
    readonly property var lines: visible ? String(Mods.active.sortMoves()).split("\n").filter(line => line !== "").map(line => line.split("\t")) : []
    readonly property var groups: {
        const found = []
        for (const parts of lines) {
            let group = found.find(g => g.name === parts[4])
            if (!group)
                found.push(group = { name: parts[4], moves: [] })
            group.moves.push(parts)
        }
        return found.sort((a, b) => b.moves.length - a.moves.length)
    }

    Text { color: Theme.ink; font.pixelSize: Theme.heading; font.weight: Font.DemiBold; text: Theme.say("Sort preview") }
    Text { color: Theme.ink2; font.pixelSize: Theme.body; bottomPadding: Theme.gapMedium; text: Theme.plural(moves.lines.length, "{} mod moves", "{} mods move") }
    Repeater {
        model: moves.groups
        Column {
            id: group
            required property var modelData
            width: moves.width
            spacing: Theme.gapHair
            bottomPadding: Theme.gapMedium
            Row {
                spacing: Theme.gapSmall
                bottomPadding: Theme.gapHair
                Text { color: Theme.ink2; font.pixelSize: Theme.small; font.weight: Font.DemiBold; text: Theme.say(group.modelData.name) }
                Text { color: Theme.ink3; font.pixelSize: Theme.small; text: group.modelData.moves.length }
            }
            Repeater {
                model: group.modelData.moves
                Rectangle {
                    id: move
                    required property var modelData
                    readonly property bool up: Number(modelData[2]) < Number(modelData[1])
                    readonly property bool chosen: Number(modelData[3]) === Mods.selectedId
                    width: moves.width
                    height: Math.max(Theme.sized(40), said.implicitHeight + 10)
                    radius: Theme.corner
                    color: chosen ? Theme.seg : hovering.hovered ? Theme.chip : "transparent"
                    Accessible.role: Accessible.Button
                    Accessible.name: modelData[0] + ", " + modelData[5]
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.gapSmall
                        anchors.rightMargin: Theme.gapSmall
                        spacing: Theme.gap
                        Icon { name: move.up ? "arrow-up" : "arrow-down"; size: 13; color: Theme.moved }
                        Column {
                            id: said
                            Layout.fillWidth: true
                            Text { width: parent.width; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: move.modelData[0] }
                            Text { width: parent.width; wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight; color: Theme.ink2; font.pixelSize: Theme.caption; text: move.modelData[5] }
                        }
                        Text { color: Theme.ink2; font.pixelSize: Theme.small; font.features: { "tnum": 1 }; text: move.modelData[1] + " → " + move.modelData[2] }
                    }
                    HoverHandler { id: hovering; cursorShape: Qt.PointingHandCursor }
                    TapHandler {
                        onTapped: {
                            Mods.select(Number(move.modelData[3]), true)
                            Mods.reveal(Mods.active, Mods.active.rowOf(Number(move.modelData[2]) - 1), true)
                        }
                    }
                    Tip { text: move.modelData[5]; over: hovering.hovered }
                }
            }
        }
    }
    Item { width: 1; height: 14 }
}
