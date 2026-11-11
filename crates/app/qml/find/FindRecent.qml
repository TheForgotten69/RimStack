pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import RimStack.Ui

Popup {
    id: recent
    enter: PopIn {}
    exit: PopOut {}
    x: parent.width - width
    y: parent.height + Theme.gapTiny
    width: Theme.sized(380)
    padding: Theme.gapSmall
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: Rectangle { radius: Theme.cornerLarge; color: Theme.bar; border.width: 1; border.color: Theme.line }
    contentItem: Column {
        Repeater {
            model: Hub.find.recent || []
            Rectangle {
                id: entry
                required property var modelData
                required property int index
                width: recent.availableWidth
                height: Theme.row
                radius: Theme.corner
                color: "transparent"
                Accessible.role: Accessible.Button
                Accessible.name: modelData.text
                Rectangle { anchors.fill: parent; radius: parent.radius; color: Theme.hover; opacity: 0.08 * Number(entryHover.hovered) }
                Text {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.gap
                    anchors.rightMargin: Theme.gap
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                    textFormat: Text.PlainText
                    color: Theme.ink
                    font.family: "monospace"
                    font.pixelSize: Theme.small
                    text: entry.modelData.text
                }
                HoverHandler { id: entryHover; cursorShape: Qt.PointingHandCursor }
                TapHandler {
                    onTapped: {
                        Hub.find.useRecent(entry.index)
                        recent.close()
                    }
                }
            }
        }
    }
}
