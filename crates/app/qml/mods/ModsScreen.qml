// The Mods screen: inactive mods, the load order, and details of the
// selected mod, with Sort as a preview to keep or discard. Nothing is
// written to disk until Save. This file only lays the parts out; what
// they show and do is in `Mods`.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Item {
    id: screen
    Appear { of: screen }

    // Inactive hidden: a rail at the left edge opens it over the load order.
    // `--drawer` opens it at once; for screenshots.
    property bool drawerOpen: Qt.application.arguments.indexOf("--drawer") !== -1
    Connections {
        target: Mods.prefs
        function onInactiveHiddenChanged() { screen.drawerOpen = false }
    }

    Rectangle {
        id: rail
        z: 2
        visible: Mods.prefs.inactiveHidden && Mods.active.problem === "" && Mods.comparing === null
        width: visible ? Theme.sized(44) : 0
        height: parent.height - bulk.height
        color: Theme.panel
        Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.edge }
        Btn {
                id: railButton
                anchors.horizontalCenter: parent.horizontalCenter
                y: 9
                primary: !screen.drawerOpen
                icon: screen.drawerOpen ? "x" : "plus"
                tip: screen.drawerOpen ? Theme.say("Close the inactive mods") : Theme.say("Inactive mods")
                onClicked: screen.drawerOpen = !screen.drawerOpen
            }
        // Read from the bottom up, as on the spine of a book.
        Text {
            rotation: -90
            x: (rail.width - width) / 2
            y: railButton.y + railButton.height + Theme.gapLarge + (width - height) / 2
            color: Theme.ink3
            font.pixelSize: Theme.small
            text: Theme.fill("Inactive {}", Theme.count(Mods.inactive.count))
        }
        // A mod dragged out of the load order is dropped here.
        DropArea {
            anchors.fill: parent
            onEntered: Mods.dropTarget = { model: Mods.inactive, modIndex: Mods.inactive.count, item: rail }
            onExited: if (Mods.dropTarget && Mods.dropTarget.item === rail) Mods.dropTarget = null
            Rectangle { visible: parent.containsDrag; anchors.fill: parent; anchors.margins: Theme.gapThin; radius: Theme.corner; color: "transparent"; border.width: 2; border.color: Theme.focus }
        }
    }

    SplitView {
        id: panes
        anchors.fill: parent
        anchors.leftMargin: rail.width
        anchors.bottomMargin: bulk.height
        orientation: Qt.Horizontal
        handle: Rectangle {
            id: grip
            // Left of the load order it sizes the inactive mods; right of it, the details.
            readonly property bool first: x < activePane.x
            function move(by) {
                const pane = first ? inactivePane : detailsPane
                const wide = Math.max(pane.SplitView.minimumWidth, pane.width + (first ? by : -by))
                pane.SplitView.preferredWidth = wide
                if (first)
                    Mods.prefs.inactive = wide
                else
                    Mods.prefs.details = wide
            }
            // A gutter of its own width, so that nothing moves when the pointer
            // comes over it; the line in its middle is what lights up.
            readonly property bool held: SplitHandle.pressed || SplitHandle.hovered || activeFocus
            implicitWidth: 7
            color: "transparent"
            activeFocusOnTab: true
            Accessible.role: Accessible.Separator
            Accessible.name: first ? Theme.say("Width of the inactive mods") : Theme.say("Width of the details")
            Keys.onLeftPressed: move(-24)
            Keys.onRightPressed: move(24)
            // Grabbed four pixels beyond each side as well, but not over a list's own scroll bar on its left.
            containmentMask: Item { x: -2; width: 15; height: grip.height }
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                width: grip.held ? 3 : 1
                height: parent.height
                color: grip.held ? Theme.focus : Theme.edge
            }
            // Takes no button, so the drag still reaches the divider: it only shapes the pointer over all that can be grabbed.
            MouseArea { x: -2; width: 15; height: parent.height; acceptedButtons: Qt.NoButton; cursorShape: Qt.SizeHorCursor }
            // A double click gives the pane its first width back.
            TapHandler { onDoubleTapped: grip.move(grip.first ? 290 - inactivePane.width : detailsPane.width - 360) }
        }

        InactivePane {
            id: inactivePane
            // Under the comparison a click must not reach the list.
            enabled: Mods.comparing === null
            SplitView.preferredWidth: Mods.prefs.inactive
            SplitView.minimumWidth: 200
            onWidthChanged: if (panes.resizing) Mods.prefs.inactive = width
            visible: Mods.active.problem === "" && !Mods.prefs.inactiveHidden
        }
        ActivePane {
            id: activePane
            enabled: Mods.comparing === null
            SplitView.fillWidth: true
            SplitView.minimumWidth: 380
        }
        DetailsPane {
            id: detailsPane
            SplitView.preferredWidth: Mods.prefs.details
            SplitView.minimumWidth: 260
            onWidthChanged: if (panes.resizing) Mods.prefs.details = width
            visible: Mods.active.problem === ""
        }
    }

    // A click beside the open drawer closes it.
    MouseArea {
        visible: drawerLoader.active
        z: 1
        x: rail.width
        width: parent.width - rail.width
        height: parent.height - bulk.height
        onClicked: screen.drawerOpen = false
        Rectangle { anchors.fill: parent; color: Theme.scrim }
    }
    Loader {
        id: drawerLoader
        active: screen.drawerOpen && rail.visible
        x: rail.width
        width: Math.min(Math.max(Theme.sized(260), Mods.prefs.inactive), parent.width - rail.width - 60)
        height: parent.height - bulk.height
        z: 1
        sourceComponent: InactivePane {
            drawer: true
            transform: Translate { id: drawerShift }
            NumberAnimation { target: drawerShift; property: "x"; from: -drawerLoader.width; to: 0; duration: Theme.calm; easing.type: Easing.OutCubic; running: true }
            Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.edge }
            Rectangle { anchors.left: parent.right; width: 12; height: parent.height; gradient: Gradient { orientation: Gradient.Horizontal; GradientStop { position: 0; color: Qt.rgba(0, 0, 0, 0.3) } GradientStop { position: 1; color: "transparent" } } }
        }
    }

    Loader {
    id: compareLoader
    onItemChanged: Mods.compareView = item
    width: parent.width - Mods.prefs.details - 1
    height: parent.height - bulk.height
    active: Mods.comparing !== null
    sourceComponent: CompareView {
        backend: Mods.backend
                    earlier: Mods.comparing.earlier
        later: Mods.comparing.later
        earlierPlace: Mods.active.indexOf(Mods.comparing.earlier) + 1
        laterPlace: Mods.active.indexOf(Mods.comparing.later) + 1
        Component.onCompleted: {
            reload()
            forceActiveFocus()
        }
        onDone: {
            Mods.comparing = null
            Mods.focusList(Mods.active)
        }
        // What the game loads changed: count the conflicts again.
        onChanged: Mods.active.recheck()
    }
}

Rectangle {
    id: ghost
    visible: Mods.dragged !== null
    width: Theme.sized(300)
    height: Theme.sized(26)
    radius: Theme.corner
    color: Theme.accent
    opacity: 0.9
    readonly property point at: screen.mapFromItem(null, Mods.dragPoint.x, Mods.dragPoint.y)
    x: at.x + 10
    y: at.y - height / 2
    Drag.active: visible
    Drag.hotSpot.x: -10
    Drag.hotSpot.y: height / 2
    Text {
        anchors.verticalCenter: parent.verticalCenter
        x: 10
        width: parent.width - 20
        elide: Text.ElideRight
        color: "white"
        font.pixelSize: Theme.body
        text: Mods.dragged ? Mods.dragged.name : ""
    }
}

    BulkBar {
        id: bulk
        anchors.bottom: parent.bottom
        width: parent.width
        // The bar takes the list's last rows: the mod just selected stays in view.
        onHeightChanged: {
            if (height !== Theme.header || Mods.selectedId < 0)
                return
            const model = Mods.selectedActive ? Mods.active : Mods.inactive
            Mods.reveal(model, model.rowOf(model.indexOf(Mods.selectedId)), false)
        }
    }

    PopMenu { id: rowMenu }
    Prompt {
        id: subPrompt
        property int modId: -1
        function create() {
            if (subName.text.trim() === "")
                return
            Mods.putInSub(subPrompt.modId, subName.text.trim())
            close()
        }
        title: Theme.say("Group under a new tag")
        onOpened: { subName.text = ""; subName.focusInput() }
        FilterField { id: subName; width: parent.width; placeholder: Theme.say("Name"); onAccepted: subPrompt.create() }
        buttons: [
            Btn { text: Theme.say("Cancel"); onClicked: subPrompt.close() },
            Btn { primary: true; text: Theme.say("Create"); enabled: subName.text.trim() !== ""; onClicked: subPrompt.create() }
        ]
    }
    Prompt {
        id: bulkPrompt
        property string what
        function apply() {
            const word = bulkWord.text.trim()
            if (word === "")
                return
            if (what === "tag")
                Mods.tagSelected(word)
            else
                Mods.subSelected(word)
            close()
        }
        title: what === "tag" ? Theme.plural(Mods.selectedCount, "Tag for {} mod", "Tag for {} mods") : Theme.plural(Mods.selectedCount, "Tag to group under, for {} mod", "Tag to group under, for {} mods")
        onOpened: { bulkWord.text = ""; bulkWord.focusInput() }
        FilterField { id: bulkWord; width: parent.width; placeholder: Theme.say("Name"); onAccepted: bulkPrompt.apply() }
        buttons: [
            Btn { text: Theme.say("Cancel"); onClicked: bulkPrompt.close() },
            Btn { primary: true; text: Theme.say("Apply"); enabled: bulkWord.text.trim() !== ""; onClicked: bulkPrompt.apply() }
        ]
    }
    Prompt {
        id: binPrompt
        property string name
        property string folder
        title: Theme.fill("Move to the bin: {}", binPrompt.name)
        Text { width: parent.width; wrapMode: Text.WrapAnywhere; color: Theme.ink2; font.pixelSize: Theme.small; text: binPrompt.folder }
        Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.say("Steam downloads a Workshop mod again while you stay subscribed to it.") }
        buttons: [
            Btn { text: Theme.say("Cancel"); onClicked: binPrompt.close() },
            Btn { danger: true; text: Theme.say("Move to the bin"); onClicked: { Mods.binMod(binPrompt.folder); binPrompt.close() } }
        ]
    }
    Connections {
        target: Mods
        function onSubNameRequested(modId) { subPrompt.modId = modId; subPrompt.open() }
        function onBulkRequested(what) { bulkPrompt.what = what; bulkPrompt.open() }
        function onBinRequested(name, folder) { binPrompt.name = name; binPrompt.folder = folder; binPrompt.open() }
    }
    Connections {
        target: Mods
        function onMenuRequested(item, x, y, entries) { rowMenu.openAt(item, x, y, entries) }
    }

    Shortcut { sequences: [StandardKey.Find]; onActivated: Mods.filterFocusRequested() }
    Shortcut { sequences: [StandardKey.Undo]; onActivated: Mods.undo() }
    Shortcut { sequences: [StandardKey.Redo]; onActivated: Mods.redo() }
    Shortcut { sequences: [StandardKey.Save]; onActivated: if (Mods.active.dirty) Mods.save() }
    Shortcut { sequence: "Escape"; enabled: Mods.comparing === null && !subPrompt.opened; onActivated: screen.drawerOpen ? screen.drawerOpen = false : Mods.picking !== "" ? Mods.picking = "" : Mods.select(-1, false) }

// `--sorting` shows the sort preview and `--comparing` the first pair of
// mods that share a texture, once the mods' files are read; for
// screenshots.
Timer {
    readonly property bool sorting: Qt.application.arguments.indexOf("--sorting") !== -1
    readonly property bool comparing: Qt.application.arguments.indexOf("--comparing") !== -1
    interval: 400
    repeat: true
    running: sorting || comparing
    onTriggered: {
        if (!Mods.active.filesRead)
            return
        stop()
        if (sorting)
            Mods.previewSort()
        for (let at = 0; comparing && at < Mods.active.count && Mods.comparing === null; at++) {
            Mods.select(Mods.nthMod(Mods.active, at), true)
            const other = (Mods.info.losesTo || Mods.info.winsOver || "").split("\n")[0].split("\t")[2]
            if (other)
                Mods.compareWith(Number(other))
        }
    }
}

Timer {
    interval: 2500
    running: Qt.application.arguments.indexOf("--probe-move") !== -1
    onTriggered: {
        const a = activePane.list, i = inactivePane.list
        a.contentY = 900; i.contentY = 1200
        console.log("PROBE before active=" + a.contentY + " inactive=" + i.contentY)
        const row = 45
        const id = Mods.idAt(Mods.active, row)
        Mods.moveAcross(Mods.active, Mods.active.indexOf(id), id)
        console.log("PROBE just after active=" + a.contentY + " inactive=" + i.contentY)
        later.start()
    }
}
Timer { id: later; interval: 800; onTriggered: { console.log("PROBE later active=" + activePane.list.contentY + " inactive=" + inactivePane.list.contentY); const id = Mods.idAt(Mods.inactive, 60); inactivePane.list.contentY = 1200; Mods.moveAcross(Mods.inactive, Mods.inactive.indexOf(id), id); later2.start() } }
Timer { id: later2; interval: 800; onTriggered: console.log("PROBE activate later active=" + activePane.list.contentY + " inactive=" + inactivePane.list.contentY) }
// `--pick=40` selects the mod on that row of the load order, `--tab=1`
// a tab of its details, `--notes` fetches its change notes; for screenshots.
Timer {
    readonly property string given: Qt.application.arguments.find(a => a.startsWith("--pick=")) || ""
    interval: 1500
    running: given !== ""
    onTriggered: {
        const tab = Qt.application.arguments.find(a => a.startsWith("--tab="))
        Mods.select(Mods.idAt(Mods.active, Number(given.substring(7))), true)
        Mods.reveal(Mods.active, Number(given.substring(7)), true)
        Mods.tab = tab ? Number(tab.substring(6)) : 0
        // `--notes` asks for the picked mod's change notes, as its Fetch button does.
        if (Qt.application.arguments.indexOf("--notes") !== -1 && Mods.info.source === "Workshop")
            Mods.backend.fetchChangeNotes(JSON.stringify([[Mods.info.folder, 0]]))
    }
}

    // `visible` is true from the click; `opened` only once the menu has faded in.
    readonly property alias menuOpen: rowMenu.visible

    Loader {
        active: (Mods.checkMode !== "" && Mods.checkMode !== "pointer") || Theme.selfTest
        sourceComponent: ModsChecks { view: activePane.list }
    }
    // Opened by address, so Qt's test module is needed only for this.
    Loader {
        source: Mods.checkMode === "pointer" ? "qrc:/qt/qml/RimStack/Ui/qml/checks/PointerChecks.qml" : ""
        onLoaded: {
            item.screen = screen
            item.activeList = activePane.list
            item.inactiveList = inactivePane.list
            item.details = detailsPane
        }
    }
}
