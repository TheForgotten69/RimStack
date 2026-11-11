pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Popup {
    enter: PopIn {}
    exit: PopOut {}
    id: listsMenu

    signal useList(var ids)
    // The load order now goes by this name; "" when it has none.
    signal named(string name)

    readonly property var lists: Hub.lists.saved || []
    readonly property var snapshots: Hub.lists.snapshots || []
    property string problem

    width: Theme.sized(380)
    padding: Theme.gapLarge
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
    onAboutToShow: { problem = ""; Hub.lists.clipboardProblem = "" }
    Component.onCompleted: {
        Hub.lists.found.connect(listsMenu.useList)
        Hub.lists.found.connect(listsMenu.close)
        Hub.lists.renamed.connect(listsMenu.named)
        Hub.lists.shared.connect(Theme.copy)
        Hub.lists.told.connect(Notices.say)
    }
    background: Rectangle { radius: Theme.cornerLarge; color: Theme.bar; border.width: 1; border.color: Theme.line }

    contentItem: Column {
        spacing: Theme.gapSmall

        RowLayout {
            width: parent.width
            spacing: Theme.gapSmall
            Text { Layout.fillWidth: true; Layout.preferredWidth: 0; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Saved lists") }
            Btn {
                id: newList
                icon: "plus"
                text: Theme.say("New")
                tip: Theme.say("Start a load order from the game alone or with its expansions; one Undo brings the old one back")
                onClicked: freshMenu.openAt(newList, 0, newList.height, [
                    { text: Theme.say("Only the game"), run: Hub.lists.startWithGame },
                    { text: Theme.say("The game and its expansions"), run: Hub.lists.startWithExpansions }
                ])
            }
        }
        PopMenu { id: freshMenu }
        Text {
            visible: listsMenu.lists.length === 0
            width: parent.width
            color: Theme.ink2
            font.pixelSize: Theme.body
            text: Theme.say("None")
        }
        Repeater {
            model: listsMenu.lists
            RowLayout {
                id: saved
                required property var modelData
                width: listsMenu.availableWidth
                height: Theme.sized(28)
                spacing: Theme.gapSmall
                Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: saved.modelData.name }
                Text { color: Theme.ink3; font.pixelSize: Theme.small; text: Theme.plural(saved.modelData.mods, "{} mod", "{} mods") }
                Btn {
                    compact: Mods.prefs.listName !== saved.modelData.name
                    icon: "save"
                    text: Theme.say("Update")
                    tip: Theme.fill("Saves the current load order as {}, replacing what it held", saved.modelData.name)
                    onClicked: {
                        listsMenu.problem = Hub.lists.keep(saved.modelData.name)
                        if (listsMenu.problem === "")
                            listsMenu.named(saved.modelData.name)
                    }
                }
                Btn {
                    text: Theme.say("Use")
                    onClicked: {
                        const ids = Hub.lists.named(saved.modelData.name)
                        if (ids) {
                            listsMenu.useList(ids)
                            listsMenu.named(saved.modelData.name)
                        }
                        listsMenu.close()
                    }
                }
                Btn {
                    icon: "trash-2"
                    tip: Theme.fill("Delete {}", saved.modelData.name)
                    onClicked: Hub.lists.remove(saved.modelData.name)
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
                    listsMenu.problem = Hub.lists.keep(name.text)
                    if (listsMenu.problem === "") {
                        listsMenu.named(name.text.trim())
                        name.text = ""
                    }
                }
            }
        }
        Text { visible: listsMenu.problem !== ""; width: parent.width; wrapMode: Text.WordWrap; color: Theme.err; font.pixelSize: Theme.body; text: listsMenu.problem }

        Text { visible: listsMenu.snapshots.length > 0; topPadding: Theme.gapSmall; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Earlier") }
        Repeater {
            model: listsMenu.snapshots
            RowLayout {
                id: earlier
                required property var modelData
                width: listsMenu.availableWidth
                height: Theme.sized(28)
                spacing: Theme.gapSmall
                Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink2; font.pixelSize: Theme.body; text: earlier.modelData.title }
                Btn {
                    text: Theme.say("Use")
                    onClicked: {
                        const ids = Hub.lists.snapshotNamed(earlier.modelData.name)
                        if (ids) {
                            listsMenu.useList(ids)
                            listsMenu.named("")
                        }
                        listsMenu.close()
                    }
                }
            }
        }

        Text { topPadding: Theme.gapSmall; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Clipboard and Rentry.co") }
        Flow {
            width: parent.width
            spacing: Theme.gapSmall
            Btn { icon: "copy"; text: Theme.say("Copy the load order"); tip: Theme.say("The active mods as text, one a line"); onClicked: { Theme.copy(Hub.mods.listing()); listsMenu.close() } }
            Btn {
                enabled: !Hub.lists.busy
                text: Theme.say("Use the copied list")
                tip: Theme.say("A list of mods copied as text or as links, or a link to a Rentry.co page, becomes the load order. Save writes it.")
                onClicked: Hub.lists.useCopied(Theme.pasted())
            }
            Btn {
                enabled: !Hub.lists.busy
                icon: "share-2"
                text: Theme.say("Share on Rentry.co")
                tip: Theme.say("Publishes the load order as a page at Rentry.co and copies its link")
                onClicked: Notices.ask({
                    title: Theme.say("Share on Rentry.co"),
                    lines: [Theme.say("The load order is published at a public link on Rentry.co that anyone who has it can open. RimStack does not keep the code that edits the page; the message that follows shows it.")],
                    label: Theme.say("Publish"), run: () => Hub.lists.share(Hub.mods.listing()) })
            }
        }
        Text { visible: Hub.lists.busy; width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.say("Waiting for Rentry.co…") }
        Text { visible: Hub.lists.clipboardProblem !== ""; width: parent.width; wrapMode: Text.WordWrap; color: Theme.err; font.pixelSize: Theme.body; text: Hub.lists.clipboardProblem }
    }
}
