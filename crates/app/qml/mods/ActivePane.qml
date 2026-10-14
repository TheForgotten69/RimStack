// The load order: what needs a look, Undo, Redo, Sort and Save, the sort
// on show, a filter with its suggestions, the column titles and the list.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Item {
    id: activePane
    // Too narrow for everything in its header: what leaves it is in the menu at its end.
    readonly property bool tight: width < Theme.sized(640)
    readonly property bool stepping: Mods.problemPlace > 0 && Mods.selectedId === Mods.problemMod
    function moreEntries() {
        const entries = []
        if (Mods.problemCount > 0) {
            entries.push({ text: Theme.say("Next problem"), icon: "chevron-down", run: () => Mods.showProblem(true) })
            entries.push({ text: Theme.say("Previous problem"), icon: "chevron-up", run: () => Mods.showProblem(false) })
            entries.push({ text: Theme.say("All problems"), icon: "triangle-alert", run: () => Mods.problemsOpen = true })
        }
        if (warningsShown > 0)
            entries.push({ text: Theme.plural(warningsShown, "{} warning", "{} warnings"), icon: "funnel", run: () => toggleTerm("is:warning") })
        if (Mods.active.conflicts > 0)
            entries.push({ text: Theme.fill("{} share textures", Mods.active.conflicts), icon: "image", run: () => toggleTerm("is:conflict") })
        if (Mods.behind > 0)
            entries.push({ text: Theme.plural(Mods.behind, "{} update", "{} updates"), icon: "download", run: () => Mods.showUpdates() })
        if (entries.length > 0)
            entries.push({ line: true })
        entries.push({ text: Theme.say("Undo (Ctrl + Z)"), icon: "undo-2", off: !Mods.active.canUndo || Mods.active.previewing, run: () => Mods.undo() })
        entries.push({ text: Theme.say("Redo (Ctrl + Shift + Z)"), icon: "redo-2", off: !Mods.active.canRedo || Mods.active.previewing, run: () => Mods.redo() })
        return entries
    }
    // The mods whose one trouble is the game version they list.
    readonly property int versionOnly: Mods.overview.versionOnly || 0
    readonly property bool versionLine: versionOnly >= 3 && !Mods.active.previewing && !Theme.demo
    // The mods warned about for something the line above the list does not say.
    readonly property int otherWarnings: Mods.overview.otherWarnings || 0
    readonly property int warningsShown: versionLine ? otherWarnings : Mods.active.warnings
    readonly property alias list: view

    function toggleTerm(term) {
        const now = filter.terms.indexOf(term) >= 0
        const others = filter.text.split(" ").filter(word => word !== "" && word !== term)
        filter.text = (now ? others : others.concat([term])).join(" ")
    }

    // "is:changed" narrows to what is not saved yet: with nothing unsaved it would empty both lists.
    Connections {
        target: Mods.active
        function onDirtyChanged() {
            if (!Mods.active.dirty && filter.terms.indexOf("is:changed") >= 0)
                activePane.toggleTerm("is:changed")
        }
    }
    Connections {
        target: Mods
        function onFilterRequested(text) { filter.text = text }
        function onFilterFocusRequested() { filter.focusInput() }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        visible: Mods.active.problem === ""
        RowLayout {
            z: 6
            Layout.leftMargin: Theme.gapCard
            Layout.rightMargin: Theme.gapLarge
            Layout.preferredHeight: Theme.header
            spacing: Theme.gap
            Text { color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Active") }
            Text {
                color: Theme.ink3
                font.pixelSize: Theme.body
                text: Theme.count(Mods.active.count)
            }
            component Count: Chip {
                property string term
                pressable: true
                // Lit while the list is narrowed to it.
                border.width: filter.terms.indexOf(term) >= 0 || activeFocus ? 2 : 0
                border.color: activeFocus ? Theme.focus : tone
                onClicked: activePane.toggleTerm(term)
            }
            Count { visible: Mods.active.errors > 0; term: "is:error"; tone: Theme.err; icon: "circle-alert"; text: Theme.plural(Mods.active.errors, "{} error", "{} errors"); tip: Theme.say("Mods the game cannot load as they are. Click to show only these.") }
            Count { id: warningsChip; visible: activePane.warningsShown > 0 && !activePane.tight; term: "is:warning"; tone: Theme.warn; icon: "triangle-alert"; text: activePane.versionLine ? Theme.plural(activePane.warningsShown, "{} more warning", "{} more warnings") : Theme.plural(activePane.warningsShown, "{} warning", "{} warnings"); tip: Theme.say("Mods with a warning: the game runs, something about them may not. Click to show only these.") }
            Count { visible: Mods.active.conflicts > 0 && !activePane.tight; term: "is:conflict"; tone: Theme.warn; icon: "image"; text: Theme.fill("{} share textures", Mods.active.conflicts); tip: Theme.say("Mods that provide the same texture as another active mod. Click to show only these.") }
            Row {
                visible: Mods.problemCount > 0 && !activePane.tight
                spacing: Theme.gapLine
                Text {
                    // With no error in the list they are warnings, the word the details and the panel use; not said twice beside the chip.
                    readonly property bool onlyWarnings: Mods.active.errors === 0
                    visible: activePane.stepping || !(onlyWarnings && activePane.warningsShown > 0 && activePane.warningsShown === Mods.problemCount)
                    anchors.verticalCenter: parent.verticalCenter
                    rightPadding: Theme.gapSmall
                    color: Theme.ink2
                    font.pixelSize: Theme.small
                    text: activePane.stepping ? (onlyWarnings ? Theme.fill("Warning {} of {}", Mods.problemPlace, Theme.count(Mods.problemCount)) : Theme.fill("Problem {} of {}", Mods.problemPlace, Theme.count(Mods.problemCount)))
                        : onlyWarnings ? Theme.plural(Mods.problemCount, "{} warning", "{} warnings") : Theme.plural(Mods.problemCount, "{} problem", "{} problems")
                    Hint { step: 1; text: Theme.say("The arrows go from one mod with a problem to the next. Its details say what is wrong and offer the fix.") }
                }
                Btn { icon: "chevron-up"; tip: Theme.say("Previous problem"); onClicked: Mods.showProblem(false) }
                Btn { icon: "chevron-down"; tip: Theme.say("Next problem"); onClicked: Mods.showProblem(true) }
            }
            Item { Layout.fillWidth: true }
            Btn {
                visible: Mods.behind > 0 && !activePane.tight
                text: Theme.plural(Mods.behind, "{} update", "{} updates")
                tip: Theme.plural(Mods.behind, "{} active Workshop mod is older than its Workshop version", "{} active Workshop mods are older than their Workshop version")
                onClicked: Mods.showUpdates()
            }
            Row {
                visible: !Theme.demo && !activePane.tight
                spacing: Theme.gapLine
                Btn { enabled: Mods.active.canUndo && !Mods.active.previewing; icon: "undo-2"; tip: Theme.say("Undo (Ctrl + Z)"); onClicked: Mods.undo() }
                Btn { enabled: Mods.active.canRedo && !Mods.active.previewing; icon: "redo-2"; tip: Theme.say("Redo (Ctrl + Shift + Z)"); onClicked: Mods.redo() }
            }
            // Sort follows what the mods' files say, so it waits until they are read.
            Btn {
                id: sortButton
                // Always named, and in the colour Sort marks what it moves with.
                tone: Theme.moved
                visible: !Theme.demo; enabled: !Mods.active.previewing && Mods.active.filesRead; icon: "arrow-up-down"; text: !Mods.active.filesRead ? Theme.say("Reading files…") : Mods.sortPending > 0 ? Theme.say("Sort") + "  " + Mods.sortPending : Theme.say("Sort"); tip: Theme.say("Shows the order the rules ask for, to keep or discard. Nothing is written before Save."); onClicked: Mods.previewSort()
                Hint { step: 2; atRight: true; text: Theme.say("Sort shows the order the mods' rules ask for and what would move. Apply moves the mods; Discard leaves the list as it was.") }
            }
            // The hint stands beside the button, not inside it: a button that cannot be pressed would switch the hint off with it.
            Item {
                visible: !Theme.demo
                implicitWidth: saveButton.implicitWidth
                implicitHeight: saveButton.implicitHeight
                Btn { id: saveButton; enabled: Mods.active.dirty; primary: Mods.active.dirty; icon: "save"; text: Theme.say("Save"); tip: Theme.say("Writes the load order to ModsConfig.xml (Ctrl + S). The file as it was is copied beside it first."); onClicked: Mods.save() }
                    Hint { step: 3; atRight: true; text: Theme.say("Nothing reaches the game before Save. Until then every change can be undone or discarded.") }
            }
            Btn {
                id: more
                visible: activePane.tight && !Theme.demo
                icon: "ellipsis"
                menu: true
                tone: Mods.active.errors > 0 ? Theme.err : Mods.problemCount > 0 ? Theme.warn : "transparent"
                text: Mods.problemCount > 0 ? Theme.count(Mods.problemCount) : ""
                tip: Theme.say("Problems, Undo, Redo")
                spoken: Theme.say("Problems, Undo, Redo")
                onClicked: Mods.menuRequested(more, 0, more.height + 2, activePane.moreEntries())
            }
        }
        Rectangle {
            visible: Mods.active.dirty && !Mods.active.previewing && !Theme.demo
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            Layout.bottomMargin: Theme.gap
            implicitHeight: Theme.sized(34)
            radius: Theme.corner
            color: Qt.rgba(Theme.warn.r, Theme.warn.g, Theme.warn.b, 0.12)
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.gapLarge
                anchors.rightMargin: Theme.gapTiny
                spacing: Theme.gap
                Rectangle { implicitWidth: 7; implicitHeight: 7; radius: Math.min(width, height) / 2; color: Theme.warn }
                Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: Theme.fill("Not saved: {}", Mods.active.unsaved) }
                Btn { text: Theme.say("Show"); tip: Theme.say("Only the mods moved or activated since the last save"); onClicked: activePane.toggleTerm("is:changed") }
                Btn { text: Theme.say("Discard"); tip: Theme.say("Back to the saved list. Undo brings the changes back."); onClicked: Mods.discardChanges() }
                Btn { primary: true; text: Theme.say("Save"); onClicked: Mods.save() }
            }
        }
        // One line for the warning most mods share after a game update.
        Rectangle {
            visible: activePane.versionLine
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            Layout.bottomMargin: Theme.gap
            implicitHeight: banner.implicitHeight + 10
            radius: Theme.corner
            color: Qt.rgba(Theme.warn.r, Theme.warn.g, Theme.warn.b, 0.12)
            // Its sentence is never cut: the buttons go under it when both do not fit on a line.
            GridLayout {
                id: banner
                readonly property bool stacked: Theme.icon + said.implicitWidth + does.implicitWidth + 2 * columnSpacing > width
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Theme.gapLarge
                anchors.rightMargin: Theme.gapTiny
                columns: stacked ? 1 : 2
                columnSpacing: Theme.gap
                rowSpacing: Theme.gapSmall
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.gap
                    Icon { Layout.alignment: Qt.AlignTop; name: "triangle-alert"; size: Theme.icon; color: Theme.warn }
                    Text { id: said; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.ink; font.pixelSize: Theme.body; text: Theme.fill("{} mods do not list RimWorld {}", activePane.versionOnly, (Mods.overview.gameVersion || "")) }
                }
                RowLayout {
                    id: does
                    Layout.alignment: Qt.AlignRight
                    spacing: Theme.gap
                    Btn { text: Theme.say("Show"); tip: Theme.say("Only the mods with a warning"); onClicked: activePane.toggleTerm("is:warning") }
                    Btn { text: Theme.fill("Ignore for {}", (Mods.overview.gameVersion || "")); tip: Theme.say("No warning about the game version while RimWorld stays on this version. Settings, Sorting has the switch."); onClicked: Mods.setVersionAside(Mods.overview.gameVersion || "") }
                }
            }
        }
        Text {
            visible: Mods.saveProblem !== "" || Mods.upkeepProblem !== ""
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapCard
            Layout.rightMargin: Theme.gapCard
            Layout.bottomMargin: Theme.gap
            wrapMode: Text.WordWrap
            color: Theme.err
            font.pixelSize: Theme.body
            text: Mods.saveProblem !== "" ? Theme.fill("The list was not saved. {}", Mods.saveProblem) : Mods.upkeepProblem
        }
        Rectangle {
            visible: Mods.active.sortNote !== ""
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            Layout.bottomMargin: Theme.gap
            implicitHeight: sortBar.implicitHeight + 14
            radius: Theme.corner
            color: Theme.banner
            RowLayout {
                id: sortBar
                anchors.fill: parent
                anchors.leftMargin: Theme.gapLarge
                anchors.rightMargin: Theme.gap
                spacing: Theme.gap
                Text {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    textFormat: Text.StyledText
                    color: Theme.ink
                    font.pixelSize: Theme.body
                    lineHeight: 1.25
                    // The first sentence is the news; the rest is detail.
                    text: {
                        const stop = Mods.active.sortNote.indexOf(". ")
                        const head = stop < 0 ? Mods.active.sortNote : Mods.active.sortNote.substring(0, stop + 1)
                        const rest = stop < 0 ? "" : Mods.active.sortNote.substring(stop + 1)
                        const plain = s => s.replace(/&/g, "&amp;").replace(/</g, "&lt;")
                        return "<b>" + plain(head) + "</b>" + (Mods.active.previewing ? " <font color=\"" + Theme.ink2 + "\">" + Theme.say("Nothing has changed yet.") + "</font>" : "") + plain(rest)
                    }
                }
                Btn { visible: Mods.active.previewing; primary: true; text: Theme.say("Apply"); onClicked: Mods.endSort(true) }
                Btn { visible: Mods.active.previewing; text: Theme.say("Discard"); onClicked: Mods.endSort(false) }
                Btn { visible: !Mods.active.previewing; text: Theme.say("OK"); onClicked: Mods.active.sortNote = "" }
                // Nothing to decide: the note leaves by itself.
                Timer { interval: 8000; running: Mods.active.sortNote !== "" && !Mods.active.previewing; onTriggered: Mods.active.sortNote = "" }
            }
        }
        FilterField {
            id: filter
            // Above the list, for the suggestions that hang below it.
            z: 5
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            Layout.bottomMargin: Theme.gap
            icon: "funnel"
            keys: Theme.say("Ctrl + F")
            chips: true
            placeholder: Theme.say("Filter active and inactive mods")
            found: Mods.filtered === "" ? "" : Theme.fill("{} active, {} inactive", Theme.count(Mods.active.matching), Theme.count(Mods.inactive.matching))
            onTextChanged: {
                Mods.active.setFilter(text)
                Mods.inactive.setFilter(text)
                Mods.filtered = text
                if (Mods.leftOffRestored)
                    Mods.prefs.lastFilter = text
            }

            Rectangle {
                id: suggestions
                // Stays up while the pointer is on it, so a click lands.
                visible: offers.count > 0 && (filter.editing || suggestHover.hovered)
                z: 5
                y: parent.height + 4
                width: Math.min(parent.width, 420)
                height: suggestList.height + 12
                radius: Theme.cornerLarge
                color: Theme.bar
                border.width: 1
                border.color: Theme.line
                HoverHandler { id: suggestHover }
                Column {
                    id: suggestList
                    x: 6
                    y: 6
                    width: parent.width - 12
                    Repeater {
                        id: offers
                        model: [
                            ["is:error", Theme.plural(Mods.active.errors, "{} mod", "{} mods")],
                            ["is:warning", Theme.plural(Mods.active.warnings, "{} mod", "{} mods")],
                            ["is:conflict", Theme.fill("{} mods sharing textures", Mods.active.conflicts)],
                            ["is:changed", Theme.say("moved or activated since the last save")],
                            ["is:new", Theme.say("installed since the game was last started")],
                            ["is:updated", Mods.behind > 0 ? Theme.fill("{} with an update", Mods.behind) : Theme.say("after checking for updates")],
                            ["is:local", Theme.say("not from the Workshop")],
                            ["is:notinsave", Theme.say("active, and not in the newest save")],
                            ["is:recent", Theme.say("updated by Steam in the last week")],
                            ["is:duplicate", Theme.say("installed more than once")],
                            ["has:code", Theme.say("ships compiled code")],
                            ["patches:", Theme.say("patches this method of the game")],
                            ["tag:", Theme.say("has this tag")],
                            ["author:", Theme.say("by this author")],
                        ].filter(offer => offer[0].startsWith(filter.partial) && offer[0] !== filter.partial && filter.terms.indexOf(offer[0]) < 0)
                        Rectangle {
                            id: offer
                            required property var modelData
                            width: suggestList.width
                            height: Theme.sized(26)
                            radius: Theme.corner
                            color: offerHover.hovered ? Theme.chip : "transparent"
                            Accessible.role: Accessible.Button
                            Accessible.name: Theme.fill("Filter by {}", modelData[0])
                            Text { x: 8; anchors.verticalCenter: parent.verticalCenter; color: Theme.ink; font.family: "monospace"; font.pixelSize: Theme.small; text: offer.modelData[0] }
                            Text { x: 110; anchors.verticalCenter: parent.verticalCenter; color: Theme.ink2; font.pixelSize: Theme.small; text: offer.modelData[1] }
                            HoverHandler { id: offerHover }
                            TapHandler {
                                onTapped: {
                                    filter.take(offer.modelData[0])
                                    filter.focusInput()
                                }
                            }
                        }
                    }
                }
            }
        }
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.control
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.gapWide
                anchors.rightMargin: Theme.gapWide
                spacing: Theme.gapLarge
                component Heading: Text {
                    id: heading
                    property string column
                    color: Mods.columns.shownBy === column && column !== "" ? Theme.ink : Theme.ink3
                    font.pixelSize: Theme.caption
                    font.weight: Font.Medium
                    activeFocusOnTab: true
                    font.underline: activeFocus
                    // Pressable over the heading row's whole height, not only its words'.
                    Layout.fillHeight: true
                    verticalAlignment: Text.AlignVCenter
                    Accessible.role: Accessible.ColumnHeader
                    Accessible.name: text
                    Keys.onReturnPressed: Mods.columns.showBy(heading.column)
                    Keys.onSpacePressed: Mods.columns.showBy(heading.column)
                    Keys.onMenuPressed: Mods.menuRequested(heading, 0, heading.height + 2, Mods.columns.menu())
                    // Ctrl + Alt with an arrow moves the column, as a drag does.
                    Keys.onPressed: event => {
                        const by = event.key === Qt.Key_Left ? -1 : event.key === Qt.Key_Right ? 1 : 0
                        const held = (event.modifiers & Qt.ControlModifier) && (event.modifiers & Qt.AltModifier)
                        const at = Mods.columns.shown.indexOf(heading.column)
                        if (by === 0 || !held || at < 0)
                            return
                        Mods.columns.move(heading.column, at + by)
                        event.accepted = true
                    }
                    HoverHandler { cursorShape: Qt.PointingHandCursor }
                    Tip { text: heading.column === "" ? Theme.say("The load order itself") : Theme.say("Click to show the list by this column; again to turn it round. Right-click to choose the columns. Drag it, or Ctrl + Alt + Left or Right, to move it.") }
                    TapHandler { onTapped: Mods.columns.showBy(heading.column) }
                    TapHandler { acceptedButtons: Qt.RightButton; onTapped: point => Mods.menuRequested(heading, point.position.x, heading.height + 2, Mods.columns.menu()) }
                }
                Heading { Layout.preferredWidth: Theme.sized(28); horizontalAlignment: Text.AlignRight; column: ""; text: "#" }
                Heading { Layout.fillWidth: true; column: "name"; text: Theme.say("Name") + (Mods.columns.shownBy === "name" ? (Mods.columns.descending ? " ▾" : " ▴") : "") }
                Repeater {
                    model: view.fitting
                    Heading {
                        id: extra
                        required property string modelData
                        required property int index
                        column: modelData
                        Layout.preferredWidth: view.columnWidth(modelData)
                        horizontalAlignment: modelData === "load" ? Text.AlignRight : Text.AlignLeft
                        text: Theme.say(Mods.columns.labels[modelData]) + (Mods.columns.shownBy === modelData ? (Mods.columns.descending ? " ▾" : " ▴") : "")
                        // Its left edge, dragged, makes it wider or narrower; the name takes the rest.
                        Item {
                            id: edge
                            x: -Theme.gapLarge / 2 - 4
                            width: 9
                            height: parent.height
                            Rectangle { anchors.horizontalCenter: parent.horizontalCenter; y: 2; width: sizing.active || edgeHover.hovered ? 2 : 1; height: parent.height - 4; color: sizing.active || edgeHover.hovered ? Theme.focus : Theme.fieldLine }
                            HoverHandler { id: edgeHover; cursorShape: Qt.SplitHCursor }
                            Tip { text: Theme.say("Drag to size the column"); over: edgeHover.hovered && !sizing.active }
                            DragHandler {
                                id: sizing
                                property real from: 0
                                target: null
                                yAxis.enabled: false
                                dragThreshold: 1
                                onActiveChanged: if (active) from = extra.width
                                onActiveTranslationChanged: if (active) Mods.columns.size(extra.modelData, from - activeTranslation.x)
                            }
                        }
                        // Dragged sideways past a neighbour, it takes its place.
                        DragHandler {
                            target: null
                            yAxis.enabled: false
                            onActiveChanged: {
                                if (active)
                                    return
                                const moved = centroid.position.x - centroid.pressPosition.x
                                if (Math.abs(moved) > extra.width / 2)
                                    Mods.columns.move(extra.modelData, extra.index + (moved > 0 ? 1 : -1))
                            }
                        }
                    }
                }
                Btn {
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    Layout.leftMargin: Math.max(0, (view.wide ? Theme.sized(124) : Theme.sized(44)) - implicitWidth)
                    menu: true
                    text: Theme.say("View")
                    spoken: Theme.say("View options")
                    onClicked: viewMenu.open()
                    Popup {
                        enter: PopIn {}
                        exit: PopOut {}
                        id: viewMenu
                        x: parent.width - width
                        y: parent.height + 6
                        width: Theme.sized(250)
                        padding: Theme.gapMedium
                        background: Rectangle { radius: Theme.cornerLarge; color: Theme.bar; border.width: 1; border.color: Theme.line }
                        contentItem: Column {
                            spacing: Theme.gapSmall
                            Text { topPadding: Theme.gapTiny; color: Theme.ink2; font.pixelSize: Theme.small; font.weight: Font.DemiBold; text: Theme.say("Load order") }
                            Btn { width: parent.width; icon: Mods.prefs.headings !== 0 ? "check" : ""; text: Theme.say("Under category headings"); onClicked: Mods.keepingScroll(() => Mods.prefs.headings = 1) }
                            Btn { width: parent.width; icon: Mods.prefs.headings === 0 ? "check" : ""; text: Theme.say("One numbered list"); onClicked: Mods.keepingScroll(() => Mods.prefs.headings = 0) }
                            Text { topPadding: Theme.gapTiny; color: Theme.ink2; font.pixelSize: Theme.small; font.weight: Font.DemiBold; text: Theme.say("Columns") }
                            Repeater {
                                model: Object.keys(Mods.columns.labels)
                                Btn {
                                    required property string modelData
                                    width: parent.width
                                    icon: Mods.columns.shown.indexOf(modelData) >= 0 ? "check" : ""
                                    text: Theme.say(Mods.columns.labels[modelData])
                                    onClicked: Mods.columns.toggle(modelData)
                                }
                            }
                            Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink3; font.pixelSize: Theme.caption; text: Theme.say("A heading orders the list when clicked and moves when dragged.") }
                        }
                    }
                }
            }
            Rectangle { anchors.bottom: parent.bottom; x: 6; width: parent.width - 12; height: 1; color: Theme.line }
        }
        ModList {
            id: view
            columns: true
            Layout.fillWidth: true
            Layout.fillHeight: true
            topMargin: Theme.gapTiny
            bottomMargin: Theme.gapTiny
            model: Mods.active
            Accessible.name: Theme.say("Load order")
            onMovementEnded: if (Mods.leftOffRestored) Mods.prefs.lastScroll = contentY
            Connections {
                target: Mods
                function onLeftOffScroll(y) {
                    if (Mods.selectedId < 0)
                        view.contentY = Math.max(-view.topMargin, Math.min(y, view.contentHeight - view.height))
                }
            }
        }
    }

    Column {
        visible: Mods.active.problem !== ""
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 520)
        spacing: Theme.gapSmall
        Text { width: parent.width; horizontalAlignment: Text.AlignHCenter; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Mods could not be read") }
        Text { width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.body; lineHeight: 1.3; text: Mods.active.problem }
    }
}
