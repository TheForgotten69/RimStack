pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Column {
    id: side
    Appear { of: side }
    width: parent.width

    readonly property var summary: Hub.mods.selectionSummary || ({ place: "", rows: [], troubled: "", notInView: 0 })
    readonly property int most: 12
    property bool open: false
    onSummaryChanged: if (summary.rows.length <= most) open = false

    Text { color: Theme.ink; font.pixelSize: Theme.heading; font.weight: Font.DemiBold; text: Theme.plural(Mods.selectedCount, "{} mod selected", "{} mods selected") }
    Text {
        topPadding: Theme.gapHair
        color: Theme.ink2
        font.pixelSize: Theme.body
        text: side.summary.place
    }

    GroupTitle { text: Theme.say("Selected") }
    Repeater {
        model: side.open ? side.summary.rows : side.summary.rows.slice(0, side.most)
        RowLayout {
            id: one
            required property var modelData
            width: side.width
            height: Theme.sized(24)
            spacing: Theme.gap
            Rectangle { implicitWidth: 7; implicitHeight: 7; radius: 3.5; color: Theme.categoryColours[one.modelData.category] || Theme.ink3 }
            Text { Layout.fillWidth: true; elide: Text.ElideMiddle; color: Theme.ink; font.pixelSize: Theme.body; text: one.modelData.name }
            Icon { visible: one.modelData.trouble > 0; name: one.modelData.trouble === 2 ? "circle-alert" : "triangle-alert"; size: Theme.sized(12); color: one.modelData.trouble === 2 ? Theme.err : Theme.ink3 }
            Text { visible: text !== ""; color: Theme.ink3; font.pixelSize: Theme.small; font.features: { "tnum": 1 }; text: one.modelData.load }
        }
    }
    Item { visible: more.visible; width: 1; height: Theme.gapSmall }
    Btn {
        id: more
        visible: side.summary.rows.length > side.most
        text: side.open ? Theme.fill("Show {}", side.most) : Theme.fill("{} more", (side.summary.rows.length - side.most))
        onClicked: side.open = !side.open
    }

    GroupTitle { visible: facts.visible; text: Theme.say("Together") }
    Grid {
        id: facts
        visible: side.summary.troubled.length > 0 || side.summary.notInView > 0
        width: parent.width
        columns: 2
        columnSpacing: Theme.gapLarge
        rowSpacing: Theme.gapTiny
        Text { visible: side.summary.troubled.length > 0; color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.say("With a problem") }
        Text { visible: side.summary.troubled.length > 0; color: Theme.ink; font.pixelSize: Theme.body; text: side.summary.troubled }
        Text { visible: side.summary.notInView > 0; color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.say("Not in view") }
        Text { visible: side.summary.notInView > 0; color: Theme.ink; font.pixelSize: Theme.body; text: side.summary.notInView }
    }
}
