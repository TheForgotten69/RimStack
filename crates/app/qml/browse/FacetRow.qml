pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

RowLayout {
    id: row
    required property var facet
    signal included()
    signal excluded()
    spacing: Theme.gapSmall
    Text {
        Layout.fillWidth: true
        Layout.preferredWidth: 0
        elide: Text.ElideRight
        color: Theme.ink
        opacity: 1 - 0.5 * Number(row.facet.exclude)
        font.pixelSize: Theme.body
        font.strikeout: row.facet.exclude
        text: row.facet.label
    }
    Text { color: Theme.ink3; font.pixelSize: Theme.small; text: row.facet.count || "" }
    Tick { icon: "plus"; on: row.facet.include; tip: Theme.fill("Only show {}", row.facet.label); onClicked: row.included() }
    Tick { icon: "minus"; on: row.facet.exclude; tone: Theme.err; tip: Theme.fill("Hide {}", row.facet.label); onClicked: row.excluded() }
}
