// A value picked along a track, by dragging or with the arrow keys.
pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Item {
    id: slide
    property real from: 0
    property real to: 1
    property real step: 0.05
    property real value: 0
    // What it sets, and its value as it is read out.
    property string title
    property string reading
    signal moved(real value)
    // For a value that moves the slider itself, as a text size does the
    // page under the pointer: it is given on release, and until then the
    // knob alone follows the drag.
    property bool settles: false
    property real held: -1
    readonly property real shown: held >= 0 ? held : value
    readonly property real part: (shown - from) / (to - from)
    function at(x) {
        const raw = from + Math.max(0, Math.min(1, x / track.width)) * (to - from)
        return Math.round(raw / step) * step
    }
    function put(x) {
        const next = at(x)
        if (Math.abs(next - value) > 1e-6)
            slide.moved(next)
    }
    implicitWidth: Theme.sized(180)
    implicitHeight: Theme.control
    activeFocusOnTab: true
    Accessible.role: Accessible.Slider
    Accessible.name: title
    Accessible.description: reading
    Keys.onLeftPressed: slide.moved(Math.max(from, value - step))
    Keys.onRightPressed: slide.moved(Math.min(to, value + step))
    Rectangle {
        id: track
        x: 8
        width: parent.width - 16
        height: 4
        radius: Math.min(width, height) / 2
        anchors.verticalCenter: parent.verticalCenter
        color: Theme.ctl
        Rectangle { width: parent.width * slide.part; height: parent.height; radius: Math.min(width, height) / 2; color: Theme.accent }
    }
    Rectangle {
        x: track.x + track.width * slide.part - 8
        anchors.verticalCenter: parent.verticalCenter
        width: 16
        height: 16
        radius: Math.min(width, height) / 2
        color: Theme.ink
        FocusRing { of: slide; corner: 8 }
    }
    HoverHandler { cursorShape: Qt.PointingHandCursor }
    // A click also takes the keys, so the arrows then move it a step at a time.
    TapHandler { onTapped: point => { slide.forceActiveFocus(); slide.put(point.position.x - track.x) } }
    DragHandler {
        target: null
        onCentroidChanged: {
            if (!active)
                return
            if (slide.settles)
                slide.held = slide.at(centroid.position.x - track.x)
            else
                slide.put(centroid.position.x - track.x)
        }
        onActiveChanged: {
            if (active || slide.held < 0)
                return
            const next = slide.held
            slide.held = -1
            if (Math.abs(next - slide.value) > 1e-6)
                slide.moved(next)
        }
    }
}
