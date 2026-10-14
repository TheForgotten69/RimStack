// The inactive mods: how they are ordered, a filter, and the list.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Rectangle {
    id: pane
    readonly property alias list: inactiveView
    // Opened over the load order from the rail, not standing beside it.
    property bool drawer: false
    color: Theme.panel
    Component.onCompleted: if (drawer) mirror.focusInput()
    Shortcut { sequences: [StandardKey.Find]; enabled: pane.drawer; onActivated: mirror.focusInput() }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        // The buttons take a line of their own when the title's does not hold them.
        // Measured against the pane: on one line the head's own width is pushed
        // past the pane by its contents, and would keep them on one line.
        GridLayout {
            id: head
            readonly property bool stacked: named.implicitWidth + acts.implicitWidth + columnSpacing > pane.width - Layout.leftMargin - Layout.rightMargin
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapCard
            Layout.rightMargin: Theme.gapLarge
            columns: stacked ? 1 : 2
            columnSpacing: Theme.gap
            rowSpacing: 0
            RowLayout {
            id: named
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.header
            spacing: Theme.gap
            Text { color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Inactive") }
            Text { color: Theme.ink3; font.pixelSize: Theme.body; text: Theme.count(Mods.inactive.count) }
            Chip {
                visible: Mods.fresh.length > 0
                pressable: true
                tone: Theme.ok
                text: Theme.fill("{} new", Mods.fresh.length)
                tip: Theme.say("Installed since the game was last started from RimStack. Click to show only these.")
                onClicked: Mods.filterBy(Mods.filtered === "is:new" ? "" : "is:new")
            }
            Item { Layout.fillWidth: true }
            }
            RowLayout {
            id: acts
            Layout.alignment: Qt.AlignRight
            Layout.bottomMargin: head.stacked ? Theme.gap : 0
            spacing: Theme.gap
            Btn {
                icon: "arrow-up-down"
                text: Mods.prefs.inactiveByName ? Theme.say("Name") : Theme.say("Category")
                spoken: Mods.prefs.inactiveByName ? Theme.say("Order the inactive mods by category") : Theme.say("Order the inactive mods by name")
                tip: spoken
                onClicked: Mods.prefs.inactiveByName = !Mods.prefs.inactiveByName
            }
            Btn { visible: !pane.drawer; text: Theme.say("Hide"); tip: Theme.say("To a rail at the left edge; its button opens the list over the load order"); onClicked: Mods.prefs.inactiveHidden = true }
            Btn { visible: pane.drawer; text: Theme.say("Pin"); tip: Theme.say("Beside the load order, always open"); onClicked: Mods.prefs.inactiveHidden = false }
            }
        }
        // The filter is the one over the load order; opened over it, this box shows the same text.
        FilterField {
            id: mirror
            visible: pane.drawer
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            Layout.bottomMargin: Theme.gap
            icon: "funnel"
            keys: Theme.say("Ctrl + F")
            chips: true
            placeholder: Theme.say("Filter active and inactive mods")
            found: Mods.filtered === "" ? "" : Theme.fill("{} active, {} inactive", Theme.count(Mods.active.matching), Theme.count(Mods.inactive.matching))
            Component.onCompleted: text = Mods.filtered
            onTextChanged: if (text !== Mods.filtered) Mods.filterBy(text)
            Connections {
                target: Mods
                function onFilteredChanged() { if (mirror.text !== Mods.filtered) mirror.text = Mods.filtered }
            }
        }
        RowLayout {
            visible: Mods.fresh.length > 0 && Mods.filtered === "is:new"
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            Layout.bottomMargin: Theme.gap
            spacing: Theme.gap
            Btn { primary: true; enabled: !Hub.mods.previewing; text: Theme.fill("Activate all {}", Mods.fresh.length); tip: Theme.say("Each goes where its rules put it in the load order. One Undo takes them all back."); onClicked: { Mods.activateFresh(); Mods.filterBy("") } }
            Item { Layout.fillWidth: true }
        }
            ModList {
                id: inactiveView
                Layout.fillWidth: true
                Layout.fillHeight: true
                model: Mods.inactive
                Accessible.name: Theme.say("Inactive mods")
            }
    }
}
