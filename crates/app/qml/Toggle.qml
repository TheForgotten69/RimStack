// A setting that is on or off: a switch, with its state in words beside it.
import QtQuick
import RimStack.Ui

Row {
    id: toggle
    property bool on: false
    // What is switched, for a screen reader.
    property string title
    signal flipped(bool on)

    spacing: Theme.gap
    Rectangle {
        id: track
        anchors.verticalCenter: parent.verticalCenter
        width: Theme.sized(38)
        height: Theme.segment
        radius: height / 2
        color: toggle.on ? Theme.accent : Theme.ctl
        border.width: toggle.on ? 0 : 1
        border.color: Theme.fieldLine
        activeFocusOnTab: true
        Accessible.role: Accessible.CheckBox
        Accessible.name: toggle.title
        Accessible.checkable: true
        Accessible.checked: toggle.on
        Accessible.onToggleAction: toggle.flipped(!toggle.on)
        Accessible.onPressAction: toggle.flipped(!toggle.on)
        Keys.onSpacePressed: toggle.flipped(!toggle.on)
        Keys.onReturnPressed: toggle.flipped(!toggle.on)
        Keys.onEnterPressed: toggle.flipped(!toggle.on)
        Behavior on color { ColorAnimation { duration: Theme.quick } }
        FocusRing { corner: track.radius }
        Rectangle {
            x: toggle.on ? parent.width - width - 3 : 3
            anchors.verticalCenter: parent.verticalCenter
            width: parent.height - 6
            height: width
            radius: width / 2
            color: toggle.on ? Theme.accentInk : Theme.ink2
            Behavior on x { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
        }
        HoverHandler { cursorShape: Qt.PointingHandCursor }
        TapHandler { onTapped: toggle.flipped(!toggle.on) }
    }
    Text { anchors.verticalCenter: parent.verticalCenter; color: Theme.ink2; font.pixelSize: Theme.small; text: toggle.on ? Theme.say("On") : Theme.say("Off") }
}
