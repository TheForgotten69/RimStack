// A tooltip: placed inside an item, it shows `text` once the pointer has
// rested on that item for a moment.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import RimStack.Ui

Item {
    id: tip
    property string text
    // Set by an item that already knows whether the pointer is on it;
    // left alone, the tip watches for itself.
    property bool over: own.hovered
    anchors.fill: parent

    // Shown only once the pointer has rested: a flag the timer sets,
    // since a tip tied to "the timer is not running" shows for a moment
    // before the timer has started.
    property bool shown: false
    onOverChanged: {
        // The status bar says it at once, for those who do not wait.
        if (over && text !== "")
            Notices.hint = text
        else if (Notices.hint === text)
            Notices.hint = ""
        shown = false
        if (over && text !== "")
            rest.restart()
        else
            rest.stop()
    }

    HoverHandler { id: own }
    Timer {
        id: rest
        interval: 600
        onTriggered: tip.shown = tip.over
    }
    Loader {
        active: tip.shown && tip.text !== ""
        sourceComponent: ToolTip {
            parent: tip
            visible: true
            delay: 0
            timeout: 15000
            padding: Theme.gap
            contentWidth: Math.min(words.implicitWidth, 340)
            contentItem: Text { id: words; wrapMode: Text.WordWrap; color: Theme.ink; font.pixelSize: Theme.small; lineHeight: 1.25; text: tip.text }
            background: Rectangle { radius: Theme.corner; color: Theme.bar; border.width: 1; border.color: Theme.line }
        }
    }
}
