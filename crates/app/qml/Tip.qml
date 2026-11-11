pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Item {
    id: tip
    property string text
    // Set by an item that already knows whether the pointer is on it;
    // left alone, the tip watches for itself.
    property bool over: own.hovered
    anchors.fill: parent

    onOverChanged: {
        const wanted = over && text !== ""
        if (wanted)
            Notices.hint = text
        else if (Notices.hint === text)
            Notices.hint = ""
        if (wanted || Notices.tipped === tip) {
            Notices.tipShown = false
            Notices.tipped = wanted ? tip : null
        }
    }

    // A Loader can take the tip away under the pointer: it lets go first.
    Component.onDestruction: over = false

    HoverHandler { id: own }
}
