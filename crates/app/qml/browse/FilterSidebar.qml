pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Rectangle {
    id: side
    readonly property var facets: Hub.workshop.sidebar
    radius: Theme.cornerLarge
    color: Theme.panel
    border.width: 1
    border.color: Theme.line

    component Heading: Text {
        color: Theme.ink3
        font.pixelSize: Theme.caption
        font.weight: Font.DemiBold
        font.capitalization: Font.AllUppercase
        font.letterSpacing: 1
    }

    Scrolled {
        anchors.fill: parent
        padding: Theme.gapCard
        spacing: Theme.gap
        Heading { text: Theme.say("Categories") }
        Repeater {
            model: side.facets.types
            FacetRow {
                required property var modelData
                width: parent.width
                facet: modelData
                onIncluded: Hub.workshop.toggleTag(modelData.id, true)
                onExcluded: Hub.workshop.toggleTag(modelData.id, false)
            }
        }
        Repeater {
            model: side.facets.versions
            FacetRow {
                required property var modelData
                width: parent.width
                facet: modelData
                onIncluded: Hub.workshop.toggleTag(modelData.id, true)
                onExcluded: Hub.workshop.toggleTag(modelData.id, false)
            }
        }
        Heading { visible: side.facets.dlc.length > 0; topPadding: Theme.gap; text: Theme.say("Required DLC") }
        Repeater {
            model: side.facets.dlc
            FacetRow {
                required property var modelData
                width: parent.width
                facet: modelData
                onIncluded: Hub.workshop.toggleDlc(modelData.id, true)
                onExcluded: Hub.workshop.toggleDlc(modelData.id, false)
            }
        }
        RowLayout {
            width: parent.width
            spacing: Theme.gap
            Toggle { silent: true; on: side.facets.hideUnowned; onFlipped: Hub.workshop.hideUnowned(!side.facets.hideUnowned) }
            Text { Layout.fillWidth: true; Layout.preferredWidth: 0; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.small; text: Theme.say("Hide mods that need an expansion I don't own") }
        }
        RowLayout {
            width: parent.width
            spacing: Theme.gap
            Toggle { silent: true; on: side.facets.showIncompatible; onFlipped: Hub.workshop.showIncompatible(!side.facets.showIncompatible) }
            Column {
                Layout.fillWidth: true
                Layout.preferredWidth: 0
                spacing: Theme.gapHair
                Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.small; text: Theme.say("Show incompatible items") }
                Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink3; font.pixelSize: Theme.caption; text: Theme.say("Items Steam has banned or removed are left out unless this is on.") }
            }
        }
    }
}
