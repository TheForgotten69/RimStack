// One problem of a mod in a sentence, ending in the action that solves it.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

RowLayout {
    id: line
    property bool error: true
    // Shown before the sentence when the line is not about the selected mod.
    property string name
    property string words
    // What `Mods.applyFix` is given, and what the button says.
    property string fix
    property string fixLabel
    property int modId: -1
    spacing: Theme.gap
    Icon { Layout.alignment: Qt.AlignTop; Layout.topMargin: Theme.gapHair; name: line.error ? "circle-alert" : "triangle-alert"; size: 14; color: line.error ? Theme.err : Theme.warn }
    Text {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        textFormat: Text.StyledText
        color: Theme.ink
        font.pixelSize: Theme.body
        lineHeight: 1.25
        text: (line.name !== "" ? "<b>" + Theme.plain(line.name) + "</b>  " : "") + Theme.plain(line.words)
        HoverHandler { enabled: line.name !== "" && line.modId >= 0; cursorShape: Qt.PointingHandCursor }
        TapHandler { enabled: line.name !== "" && line.modId >= 0; onTapped: Mods.showMod(line.modId) }
    }
    Btn {
        Layout.alignment: Qt.AlignTop
        visible: line.fix !== "" && !Mods.active.previewing
        text: Theme.say(line.fixLabel)
        onClicked: Mods.applyFix(line.modId, line.fix)
    }
}
