pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import RimStack.Ui

Btn {
    id: button
    menu: true
    icon: "arrow-up-down"
    text: Hub.workshop.sortLabel
    tip: Theme.say("Sort the results")
    onClicked: choices.visible = !choices.visible
    Popup {
        id: choices
        y: button.height + Theme.gapTiny
        x: button.width - width
        padding: Theme.gapTiny
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
        background: Rectangle { radius: Theme.cornerLarge; color: Theme.bar; border.width: 1; border.color: Theme.line }
        contentItem: Column {
            Repeater {
                model: Hub.workshop.sorts
                Rectangle {
                    id: choice
                    required property var modelData
                    implicitWidth: label.implicitWidth + 3 * Theme.gap + Theme.icon
                    width: Math.max(implicitWidth, parent.width)
                    height: Theme.strip
                    radius: Theme.corner
                    color: "transparent"
                    Accessible.role: Accessible.MenuItem
                    Accessible.name: modelData.label
                    Rectangle { anchors.fill: parent; radius: parent.radius; color: Theme.chosen; opacity: Number(over.hovered) }
                    Icon { x: Theme.gap; anchors.verticalCenter: parent.verticalCenter; visible: choice.modelData.chosen; name: "check"; color: Theme.ink }
                    Text { id: label; x: 2 * Theme.gap + Theme.icon; anchors.verticalCenter: parent.verticalCenter; color: Theme.ink; font.pixelSize: Theme.body; text: choice.modelData.label }
                    HoverHandler { id: over; cursorShape: Qt.PointingHandCursor }
                    TapHandler {
                        onTapped: {
                            choices.close()
                            Hub.workshop.chooseSort(choice.modelData.id)
                        }
                    }
                }
            }
        }
    }
}
