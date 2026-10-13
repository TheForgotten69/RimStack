// Mods by name, each with a short note; `lines` is what the model's
// `details` gives: "name<tab>note" per line, "!" first for trouble.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Column {
    id: names
    property string title
    property string lines
    // Shown as a button on each line when set; `acted` then gives the
    // line's third part.
    property string action
    signal acted(string what)
    // One grey line before the names, for what is said elsewhere.
    property string lead
    visible: lines !== "" || lead !== ""
    GroupTitle { text: names.title }
    Text { visible: names.lead !== ""; width: names.width; height: Theme.sized(22); verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight; color: Theme.ink3; font.pixelSize: Theme.body; text: names.lead }
    Repeater {
        model: names.lines ? names.lines.split("\n") : []
        RowLayout {
            id: entry
            required property string modelData
            readonly property bool trouble: modelData.charAt(0) === "!"
            readonly property var parts: (trouble ? modelData.substring(1) : modelData).split("\t")
            width: names.width
            height: Theme.sized(names.action !== "" ? 28 : 22)
            spacing: Theme.gap
            Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: entry.parts[0] }
            Text { color: entry.trouble ? Theme.err : Theme.ink3; font.pixelSize: Theme.small; text: (entry.trouble ? "! " : "") + (entry.parts[1] || "") }
            Btn { visible: names.action !== ""; text: names.action; onClicked: names.acted(entry.parts[2] || "") }
        }
    }
}
