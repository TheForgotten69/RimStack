// The saved lists: keep the load order under a name, and go back to one.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Popup {
    enter: PopIn {}
    exit: PopOut {}
    id: menu

    required property Backend backend
    signal useList(var ids)
    // The load order now goes by this name; "" when it has none.
    signal named(string name)

    property var lists: []
    // The load order as it was before each Sort or change of list.
    property var snapshots: []
    property string problem

    function reload() {
        lists = JSON.parse(backend.savedLists())
        snapshots = JSON.parse(backend.snapshots())
    }

    width: Theme.sized(380)
    padding: Theme.gapLarge
    onAboutToShow: {
        problem = ""
        reload()
    }
    background: Rectangle { radius: Theme.cornerLarge; color: Theme.bar; border.width: 1; border.color: Theme.line }

    contentItem: Column {
        spacing: Theme.gapSmall

        Text { color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Saved lists") }
        Text {
            visible: menu.lists.length === 0
            width: parent.width
            color: Theme.ink2
            font.pixelSize: Theme.body
            text: Theme.say("None")
        }
        Repeater {
            model: menu.lists
            RowLayout {
                id: saved
                required property var modelData
                width: menu.availableWidth
                height: Theme.sized(28)
                spacing: Theme.gapSmall
                Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: saved.modelData.name }
                Text { color: Theme.ink3; font.pixelSize: Theme.small; text: Theme.plural(saved.modelData.mods, "{} mod", "{} mods") }
                Btn {
                    // Named on the list in use; an icon on the others, where it is pressed less.
                    compact: Mods.prefs.listName !== saved.modelData.name
                    icon: "save"
                    text: Theme.say("Update")
                    tip: Theme.fill("Keeps the load order as it is now under {}, in place of what it held", saved.modelData.name)
                    onClicked: {
                        menu.problem = menu.backend.saveListAs(saved.modelData.name)
                        if (menu.problem === "")
                            menu.named(saved.modelData.name)
                        menu.reload()
                    }
                }
                Btn {
                    text: Theme.say("Use")
                    onClicked: {
                        const ids = menu.backend.savedList(saved.modelData.name)
                        if (ids !== "") {
                            menu.useList(JSON.parse(ids))
                            menu.named(saved.modelData.name)
                        }
                        menu.close()
                    }
                }
                Btn {
                    icon: "trash-2"
                    tip: Theme.fill("Delete {}", saved.modelData.name)
                    onClicked: {
                        menu.backend.deleteList(saved.modelData.name)
                        menu.reload()
                    }
                }
            }
        }

        Rectangle { width: parent.width; height: 1; color: Theme.line }
        RowLayout {
            width: parent.width
            spacing: Theme.gapSmall
            FilterField {
                id: name
                Layout.fillWidth: true
                placeholder: Theme.say("Name for the current list")
                onAccepted: keep.clicked()
            }
            Btn {
                id: keep
                enabled: name.text.trim() !== ""
                icon: "save"
                text: Theme.say("Keep")
                onClicked: {
                    menu.problem = menu.backend.saveListAs(name.text)
                    if (menu.problem === "") {
                        menu.named(name.text.trim())
                        name.text = ""
                    }
                    menu.reload()
                }
            }
        }
        Text { visible: menu.problem !== ""; width: parent.width; wrapMode: Text.WordWrap; color: Theme.err; font.pixelSize: Theme.body; text: menu.problem }

        Text { visible: menu.snapshots.length > 0; topPadding: Theme.gapSmall; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Earlier") }
        Repeater {
            // The five newest fit; all ten are in RimStack's data folder.
            model: menu.snapshots.slice(0, 5)
            RowLayout {
                id: earlier
                required property var modelData
                width: menu.availableWidth
                height: Theme.sized(28)
                spacing: Theme.gapSmall
                Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.listName(earlier.modelData.name) }
                Btn {
                    text: Theme.say("Use")
                    onClicked: {
                        const ids = menu.backend.snapshotList(earlier.modelData.name)
                        if (ids !== "") {
                            menu.useList(JSON.parse(ids))
                            menu.named("")
                        }
                        menu.close()
                    }
                }
            }
        }

        Text { topPadding: Theme.gapSmall; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Clipboard") }
        Flow {
            width: parent.width
            spacing: Theme.gapSmall
            Btn { icon: "copy"; text: Theme.say("Copy the load order"); tip: Theme.say("The active mods as text, one a line"); onClicked: { Theme.copy(Mods.active.listing()); menu.close() } }
            Btn {
                text: Theme.say("Use the copied list")
                tip: Theme.say("A list of mods copied as text or as links becomes the load order. Save writes it.")
                onClicked: {
                    const ids = JSON.parse(menu.backend.idsInText(Theme.pasted()))
                    if (ids.length > 0) {
                        menu.useList(ids)
                        menu.named("")
                        menu.close()
                    } else {
                        menu.problem = Theme.say("No mod found on the clipboard")
                    }
                }
            }
        }
    }
}
