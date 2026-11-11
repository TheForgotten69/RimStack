import QtQuick
import RimStack.Ui

Row {
    id: stars
    property var filled: []
    spacing: Theme.gapHair
    Repeater {
        model: stars.filled
        Item {
            required property bool modelData
            width: Theme.icon
            height: Theme.icon
            Icon { name: "star"; color: Theme.line }
            Icon { visible: parent.modelData; name: "star"; color: Theme.warn }
        }
    }
}
