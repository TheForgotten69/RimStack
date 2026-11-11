pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Item {
    id: pick
    property string text
    property string count
    property bool chosen: false
    property bool radio: false
    signal clicked()

    implicitHeight: Theme.control
    opacity: 0.4 + 0.6 * Number(enabled)
    activeFocusOnTab: true
    Accessible.role: Accessible.CheckBox
    Accessible.name: text
    Accessible.checkable: true
    Accessible.checked: chosen
    Accessible.onToggleAction: pick.clicked()
    Keys.onSpacePressed: pick.clicked()
    Keys.onReturnPressed: pick.clicked()
    FocusRing {}
    Rectangle { anchors.fill: parent; radius: Theme.corner; color: Theme.hover; opacity: 0.06 * Number(pointed.hovered && pick.enabled) }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.gapSmall
        anchors.rightMargin: Theme.gapSmall
        spacing: Theme.gap
        Rectangle {
            Layout.preferredWidth: Theme.icon
            Layout.preferredHeight: Theme.icon
            radius: Math.min(width / 2, Theme.corner + (width / 2 - Theme.corner) * Number(pick.radio))
            color: Theme.ctl
            border.width: Theme.outline
            border.color: Theme.fieldLine
            Rectangle { visible: pick.chosen; anchors.fill: parent; radius: parent.radius; color: Theme.accent }
            Icon { visible: pick.chosen && !pick.radio; anchors.centerIn: parent; name: "check"; size: Theme.sized(11); color: Theme.accentInk }
            Rectangle { visible: pick.chosen && pick.radio; anchors.centerIn: parent; width: parent.width / 3; height: width; radius: width / 2; color: Theme.accentInk }
        }
        Text {
            Layout.fillWidth: true
            Layout.preferredWidth: 0
            elide: Text.ElideRight
            color: Theme.ink
            font.pixelSize: Theme.small
            text: pick.text
        }
        Text { visible: pick.count !== ""; color: Theme.ink3; font.pixelSize: Theme.small; font.features: { "tnum": 1 }; text: pick.count }
    }
    HoverHandler { id: pointed; cursorShape: Qt.PointingHandCursor }
    TapHandler { enabled: pick.enabled; gesturePolicy: TapHandler.ReleaseWithinBounds; onTapped: pick.clicked() }
}
