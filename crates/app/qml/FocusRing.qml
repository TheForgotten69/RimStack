// The mark of keyboard focus: a ring outside the control with a gap
// between, so it shows on any fill.
import QtQuick
import RimStack.Ui

Rectangle {
    property Item of: parent
    property real corner: Theme.corner
    // Round a control on a selected row: the focus colour is too close to the accent there (1.0 to 2.4:1).
    property bool onAccent: false
    visible: of.activeFocus
    anchors.fill: parent
    anchors.margins: -4
    radius: corner + 4
    color: "transparent"
    border.width: 2
    border.color: onAccent ? Theme.onSelected : Theme.focus
}
