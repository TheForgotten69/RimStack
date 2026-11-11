pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Rectangle {
    id: tick
    property string icon
    property bool on: false
    property color tone: Theme.accent
    property string tip
    signal clicked()
    implicitWidth: Theme.strip
    implicitHeight: Theme.strip
    radius: Theme.corner
    color: Theme.ctl
    activeFocusOnTab: true
    Keys.onReturnPressed: tick.clicked()
    Keys.onSpacePressed: tick.clicked()
    FocusRing { corner: tick.radius }
    Accessible.role: Accessible.CheckBox
    Accessible.name: tip
    Accessible.checkable: true
    Accessible.checked: on
    Accessible.onPressAction: tick.clicked()
    Rectangle { anchors.fill: parent; radius: tick.radius; color: Theme.ctlHi; opacity: Number(ticking.hovered) }
    Rectangle { visible: tick.on; anchors.fill: parent; radius: tick.radius; color: tick.tone }
    Icon { visible: !tick.on; anchors.centerIn: parent; name: tick.icon; color: Theme.ink2 }
    Icon { visible: tick.on; anchors.centerIn: parent; name: tick.icon; color: Theme.accentInk }
    HoverHandler { id: ticking; cursorShape: Qt.PointingHandCursor }
    Tip { text: tick.tip; over: ticking.hovered || tick.activeFocus }
    TapHandler { onTapped: tick.clicked() }
}
