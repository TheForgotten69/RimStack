pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Rectangle {
    id: pane
    readonly property alias list: inactiveView
    readonly property alias orderMenu: orderMenu
    property bool drawer: false
    color: Theme.panel
    Shortcut { sequences: [StandardKey.Find]; enabled: pane.drawer; onActivated: mirror.focusInput() }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        // Measured against the pane: the head's own width would push past the pane.
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
            Text { color: Theme.ink; font.pixelSize: Theme.heading; font.weight: Font.DemiBold; text: Theme.say("Inactive") }
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
                menu: true
                text: Mods.orders.inactiveSaid
                spoken: Theme.say("Order the inactive mods")
                tip: spoken
                onClicked: orderMenu.visible = !orderMenu.visible
                Popup {
                    id: orderMenu
                    enter: PopIn {}
                    exit: PopOut {}
                    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
                    x: parent.width - width
                    y: parent.height + Theme.gapSmall
                    padding: Theme.gapMedium
                    background: Rectangle { radius: Theme.cornerLarge; color: Theme.bar; border.width: 1; border.color: Theme.line }
                    contentItem: Column {
                        spacing: Theme.gapSmall
                        Repeater {
                            model: Mods.orders.inactive
                            Btn {
                                required property var modelData
                                width: Math.max(implicitWidth, parent.width)
                                icon: modelData.icon
                                text: modelData.label
                                onClicked: Mods.prefs.inactiveOrder = modelData.then
                            }
                        }
                    }
                }
            }
            Btn { visible: !pane.drawer; text: Theme.say("Hide"); tip: Theme.say("Folds the list into a bar at the left edge; its button opens the list over the load order"); onClicked: Mods.prefs.inactiveHidden = true }
            Btn { visible: pane.drawer; text: Theme.say("Pin"); tip: Theme.say("Beside the load order, always open"); onClicked: Mods.prefs.inactiveHidden = false }
            }
        }
        ModsFilter {
            id: mirror
            visible: pane.drawer
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            Layout.bottomMargin: Theme.gap
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
