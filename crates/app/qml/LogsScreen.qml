// The Logs screen: the game's last two logs and the launches on record on
// the left; in the middle the chosen log as its errors (grouped and
// counted, with the mod each most likely comes from), as the mods those
// errors point at, or line by line; and one error's details on the right.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Item {
    id: screen
    Appear { of: screen }

    required property LogReader reader
    signal showMod(int modId)
    signal useList(var ids)
    signal findBroken()
    // The search for the broken mod is under way.
    property bool hunting: false

    readonly property var report: reader.logs ? JSON.parse(reader.logs) : ({})
    // The game keeps two logs: this launch's and the one before.
    property bool previous: false
    // A recorded launch on show in place of a log: its place in `launches`, or -1.
    // `--recorded=0` opens on the newest; for screenshots.
    property int recorded: {
        const given = Qt.application.arguments.find(a => a.startsWith("--recorded="))
        return given ? Number(given.substring(11)) : -1
    }
    property string filterText
    // The launches recorded here, newest first.
    readonly property var launches: report.launches || []
    // The game's last start is not one of them: a round of the search
    // for the broken mod, or a start from Steam.
    readonly property bool outside: onLast && !!report.written && report.written > (launches.length > 0 ? launches[0].time : 0) + 120
    // A run of errors alike but for a name shows as its first, until opened.
    property var openRuns: ({})
    // What the filter holds for the errors that point at no mod.
    readonly property string noMod: reader.noModFilter()
    readonly property var errors: report.errors ? JSON.parse(reader.errorsListed(filterText, JSON.stringify(Object.keys(openRuns).filter(run => openRuns[run]).map(Number)))) : []
    function toggleRun(run) {
        const now = Object.assign({}, openRuns)
        now[run] = !now[run]
        openRuns = now
    }
    property int selected: 0
    readonly property var error: errors[selected] || null
    // What the middle shows: 0 the errors, 1 the mods they point at, 2 the log.
    // `--view=2` opens on that one; for screenshots.
    property int view: {
        const given = Qt.application.arguments.find(a => a.startsWith("--view="))
        return given ? Number(given.substring(7)) : 0
    }
    // The mods the errors point at, most errors first:
    // { name, modId, kinds, times }.
    readonly property var culprits: report.culprits || []
    // The log's lines that hold the filter text: { number, text }.
    readonly property var lines: {
        const all = report.text || []
        const found = []
        for (let i = 0; i < all.length; i++) {
            if (filterText === "" || all[i].toLowerCase().indexOf(filterText) !== -1)
                found.push({ number: i + 1, text: all[i] })
        }
        return found
    }
    // The line each kind of error first appears on.
    readonly property var errorLines: {
        const at = {}
        for (const e of report.errors || [])
            at[e.line] = true
        return at
    }

    function showLine(number) {
        filter.text = ""
        view = 2
        const at = Math.max(0, Math.min(number - 1, lines.length - 1))
        logView.positionViewAtIndex(at, ListView.Center)
        logView.marked = number
    }

    function load() {
        if (kept !== "")
            reader.loadKeptLog(kept)
        else
            reader.loadLogs(previous)
    }
    onVisibleChanged: if (visible) load()
    Component.onCompleted: if (visible) load()
    // A re-read of the mods empties this report: the open screen asks for it again.
    Connections {
        target: screen.reader
        function onLogsChanged() { if (screen.visible && screen.reader.logs === "") screen.load() }
    }
    // A kept log on show, by its path, in place of the game's last or previous one.
    property string kept: ""
    readonly property bool onLast: !previous && kept === ""
    readonly property bool onBefore: previous && kept === ""
    // One load at a time: a log chosen while another is read is read next.
    property bool pending: false
    function show(before, keptLog) {
        previous = before
        kept = keptLog
        recorded = -1
        selected = 0
        pending = true
        loadPending()
    }
    function loadPending() {
        if (!pending || reader.logsBusy)
            return
        pending = false
        load()
    }
    Connections {
        target: screen.reader
        function onLogsBusyChanged() { screen.loadPending() }
    }

    function when(seconds) {
        return Theme.moment(seconds)
    }

    function outcome(count) {
        if (count === null || count === undefined)
            return Theme.say("Not read")
        return count === 0 ? Theme.say("Clean") : Theme.plural(count, "{} error", "{} errors")
    }

    function helpText() {
        const e = screen.error
        const from = e.mod ? "Likely mod: " + e.mod + "\n" : ""
        const game = report.version ? "RimWorld " + report.version + "\n" : ""
        return e.head + "\n" + e.trace.join("\n") + "\n\n" + game + from + "Logged " + e.count + " times, first at line " + e.line + " of the log."
    }


    component Entry: Item {
        id: entry
        property string title
        property string state
        property string stateTip
        property bool bad: false
        property bool current: false
        property bool choosable: true
        signal chosen()
        // The state drops under the title where the two do not fit side by side.
        readonly property bool stacked: entryChip.visible && entryTitle.implicitWidth + entryChip.implicitWidth + 36 + 14 + Theme.gap > width
        width: parent ? parent.width : 0
        implicitHeight: Theme.sized(34) + (stacked ? Theme.tag + Theme.gapTiny : 0) + (entryTitle.lineCount - 1) * entryTitle.font.pixelSize * 1.3
        height: implicitHeight
        Accessible.ignored: !visible
        Accessible.role: Accessible.ListItem
        Accessible.name: title + (state !== "" ? ", " + state : "")
        Accessible.onPressAction: if (choosable) entry.chosen()
        Rectangle {
            anchors.fill: parent
            anchors.leftMargin: Theme.gapSmall
            anchors.rightMargin: Theme.gapSmall
            radius: Theme.corner
            color: entry.current ? Theme.accent : Theme.hover
            opacity: entry.current ? 1 : entryHover.hovered && entry.choosable ? 0.04 : 0
        }
        HoverHandler { id: entryHover }
        TapHandler { enabled: entry.choosable; onTapped: entry.chosen() }
        // The line of the timeline, and this launch's dot on it: green for
        // a clean one, red for errors, grey where the log was not read.
        Rectangle { x: 21; width: 1; height: parent.height; color: entry.current ? Theme.onSelectedDim : Theme.line }
        Rectangle {
            x: 17
            anchors.verticalCenter: parent.verticalCenter
            width: 9
            height: 9
            radius: Math.min(width, height) / 2
            color: entry.current ? Theme.onSelected : entry.bad ? Theme.err : entry.state === screen.outcome(0) ? Theme.ok : Theme.ink3
            border.width: 2
            border.color: entry.current ? Theme.accent : Theme.panel
        }
        GridLayout {
            anchors.fill: parent
            anchors.leftMargin: 36
            anchors.rightMargin: Theme.gapCard
            columns: entry.stacked ? 1 : 2
            columnSpacing: Theme.gap
            rowSpacing: Theme.gapTiny
            Text { id: entryTitle; Layout.fillWidth: true; Layout.alignment: entry.stacked ? Qt.AlignBottom : Qt.AlignVCenter; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight; color: entry.current ? Theme.onSelected : entry.choosable ? Theme.ink : Theme.ink2; font.pixelSize: Theme.body; text: entry.title }
            Chip { id: entryChip; Layout.alignment: entry.stacked ? Qt.AlignTop : Qt.AlignVCenter; visible: entry.state !== ""; maxWidth: entry.width - 50; onAccent: entry.current; tone: entry.bad ? Theme.err : entry.state === screen.outcome(0) ? Theme.ok : Theme.ink3; text: entry.state; tip: entry.stateTip }
        }
    }

    PopMenu { id: lineMenu }

    LaunchRecord {
        visible: screen.recorded >= 0
        z: 5
        x: launchPane.width
        width: parent.width - x
        height: parent.height
        launch: screen.launches[screen.recorded] || null
        outcome: launch ? screen.outcome(launch.errors) : ""
        onUseList: ids => screen.useList(ids)
        onClosed: screen.recorded = -1
    }
    RowLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            id: launchPane
            Layout.preferredWidth: Math.min(Theme.sized(250), screen.width * 0.27)
            Layout.fillHeight: true
            color: Theme.panel
            // The last launch, the one before where it shows, then those on record: one list for the arrows.
            RowKeys {
                readonly property int fixed: beforeEntry.visible ? 2 : 1
                title: Theme.say("Launches")
                count: fixed + screen.launches.length
                current: screen.recorded >= 0 ? fixed + screen.recorded
                    : screen.kept !== "" ? fixed + Math.max(0, screen.launches.findIndex(launch => launch.log === screen.kept))
                    : screen.onBefore && beforeEntry.visible ? 1 : 0
                onChosen: index => {
                    const launch = screen.launches[index - fixed]
                    if (index >= fixed)
                        recordedList.positionViewAtIndex(index - fixed, ListView.Contain)
                    if (index < fixed)
                        screen.show(index === 1, "")
                    else if (launch.log)
                        screen.show(false, launch.log)
                    else
                        screen.recorded = index - fixed
                }
            }
            ColumnLayout {
                anchors.fill: parent
                spacing: 0
                Text { Layout.leftMargin: Theme.gapCard; Layout.preferredHeight: Theme.header; verticalAlignment: Text.AlignVCenter; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: Theme.say("Launches") }
                Entry {
                    Layout.fillWidth: true
                    title: screen.outside ? Theme.say("Last launch, not from here") : Theme.say("Last launch")
                    state: screen.onLast ? screen.outcome((screen.report.errors || []).length) : ""
                    stateTip: Theme.say("Different errors in the log; one that repeats counts once") + (screen.outside ? ". " + Theme.say("The game was started without Play: in a round of the search for the broken mod, or from Steam.") : "")
                    bad: screen.onLast && (screen.report.errors || []).length > 0
                    current: screen.onLast && screen.recorded < 0
                    onChosen: screen.show(false, "")
                }
                Entry {
                    id: beforeEntry
                    Layout.fillWidth: true
                    visible: screen.kept !== "" || screen.previous || !!screen.report.hasOther
                    title: Theme.say("Launch before")
                    state: screen.onBefore ? screen.outcome((screen.report.errors || []).length) : ""
                    stateTip: Theme.say("Different errors in the log; one that repeats counts once")
                    bad: screen.onBefore && (screen.report.errors || []).length > 0
                    current: screen.onBefore && screen.recorded < 0
                    onChosen: screen.show(true, "")
                }
                GroupTitle {
                    Layout.leftMargin: Theme.gapWide
                    visible: screen.launches.length > 0
                    text: Theme.say("Recorded")
                    Tip { text: Theme.say("The game keeps the logs of its last two launches. RimStack records how each launch it starts went.") }
                }
                ListView {
                    id: recordedList
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.topMargin: Theme.gapTiny
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    Wheel {}
                    model: screen.launches
                    delegate: Entry {
                        required property var modelData
                        required property int index
                        current: screen.recorded === index || (screen.kept !== "" && screen.kept === modelData.log && screen.recorded < 0)
                        onChosen: modelData.log ? screen.show(false, modelData.log) : screen.recorded = index
                        title: screen.when(modelData.time)
                        state: screen.outcome(modelData.errors)
                        bad: modelData.errors > 0
                    }
                }
            }
            Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.edge }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 0
            spacing: 0

            GridLayout {
                id: head
                // The button keeps its icon alone where its text would push the others out.
                readonly property bool tight: width < Theme.sized(560)
                // The buttons take a line of their own where they would stand over the date.
                readonly property bool wrapped: width < written.implicitWidth + controls.implicitWidth + Theme.gapWide
                columns: wrapped ? 1 : 2
                Layout.fillWidth: true
                Layout.leftMargin: Theme.gapCard
                Layout.rightMargin: Theme.gapLarge
                Layout.topMargin: wrapped ? Theme.gapSmall : 0
                Layout.bottomMargin: wrapped ? Theme.gapSmall : 0
                Layout.minimumHeight: Theme.header
                columnSpacing: Theme.gap
                rowSpacing: Theme.gapSmall
                Column {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    Text { id: written; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: screen.report.written ? screen.when(screen.report.written) : screen.previous ? Theme.say("Launch before") : Theme.say("Last launch") }
                    Text { width: parent.width; elide: Text.ElideRight; color: Theme.ink3; font.pixelSize: Theme.small; text: screen.report.lines ? [screen.report.version ? "RimWorld " + screen.report.version : "", Theme.fill("{} lines", Theme.count(screen.report.lines))].filter(s => s).join(", ") : "" }
                }
                RowLayout {
                id: controls
                spacing: Theme.gap
                Btn { compact: true; icon: "external-link"; text: Theme.say("Open file"); tip: screen.report.path || ""; enabled: !!screen.report.path; onClicked: Qt.openUrlExternally(Theme.fileUrl(screen.report.path)) }
                Btn { compact: true; icon: "copy"; text: Theme.say("Copy path"); enabled: !!screen.report.path; onClicked: Theme.copy(screen.report.path) }
                Btn {
                    visible: !Theme.demo && !screen.hunting
                    icon: "search"
                    compact: head.tight
                    text: Theme.say("Find the broken mod")
                    tip: Theme.say("Find the broken mod") + ". " + Theme.say("Switches half of the mods off, you play and say whether it still breaks, until one mod is left. Your list is kept and comes back at the end.")
                    onClicked: screen.findBroken()
                }
                Choice { title: Theme.say("View"); options: ["Errors", "Mods", "Log"]; current: screen.view; onPicked: (index) => screen.view = index }
                }
            }

            Rectangle {
                id: since
                visible: !!screen.report.sinceClean && screen.onLast && screen.view === 0
                readonly property var info: screen.report.sinceClean || ({ removed: [], added: [], time: 0, ids: [] })
                readonly property bool same: info.removed.length === 0 && info.added.length === 0 && !info.reordered
                Layout.fillWidth: true
                Layout.leftMargin: Theme.gapLarge
                Layout.rightMargin: Theme.gapLarge
                Layout.bottomMargin: Theme.gap
                implicitHeight: sinceText.implicitHeight + 20
                radius: Theme.cornerLarge
                color: Theme.panel
                RowLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.gapMedium
                    anchors.leftMargin: Theme.gapLarge
                    spacing: Theme.gap
                    Text {
                        id: sinceText
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        textFormat: Text.StyledText
                        color: Theme.ink
                        font.pixelSize: Theme.body
                        lineHeight: 1.35
                        text: {
                            const plain = s => s.replace(/&/g, "&amp;").replace(/</g, "&lt;")
                            const dim = s => "<font color=\"" + Theme.ink2 + "\">" + s + "</font>"
                            const parts = []
                            if (since.info.added.length > 0)
                                parts.push(dim(Theme.say("Added")) + " " + plain(since.info.added.join(", ")))
                            if (since.info.removed.length > 0)
                                parts.push(dim(Theme.say("Removed")) + " " + plain(since.info.removed.join(", ")))
                            if (since.info.reordered)
                                parts.push(dim(Theme.say("Order changed")))
                            return "<b>" + Theme.say("Since the last clean launch") + "</b> " + dim("(" + screen.when(since.info.time) + ")") + "<br>"
                                + (since.same ? dim(Theme.say("Same load order")) : parts.join(" &nbsp; "))
                        }
                    }
                    Btn { visible: !since.same; text: Theme.say("Use"); tip: Theme.say("Makes the load order of the last clean launch the list on the Mods screen. Save writes it."); onClicked: screen.useList(since.info.ids) }
                }
            }

            RowLayout {
                visible: screen.view !== 1
                Layout.fillWidth: true
                Layout.leftMargin: Theme.gapLarge
                Layout.rightMargin: Theme.gapLarge
                Layout.bottomMargin: Theme.gap
                spacing: Theme.gap
                FilterField {
                    id: filter
                    Layout.fillWidth: true
                    placeholder: screen.view === 2 ? Theme.say("Search the log") : Theme.say("Filter by text or mod")
                    onTextChanged: { screen.filterText = text.trim().toLowerCase(); screen.selected = 0 }
                }
                Choice {
                    visible: screen.view === 2
                    title: Theme.say("Long lines")
                    options: ["Cut", "Wrap"]
                    current: Theme.stored.wrapLog ? 1 : 0
                    onPicked: index => Theme.stored.wrapLog = index === 1
                }
            }

            ListView {
                id: culpritList
                visible: screen.view === 1
                Layout.fillWidth: true
                Layout.fillHeight: true
                topMargin: Theme.gapTiny
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                Wheel {}
                ScrollBar.vertical: ScrollBar {}
                model: screen.view === 1 ? screen.culprits : []
                property int at: 0
                function open(culprit) {
                    screen.view = 0
                    filter.text = culprit.name || screen.noMod
                }
                RowKeys {
                    id: culpritKeys
                    parent: culpritList
                    view: culpritList
                    title: Theme.say("Mods the errors point at")
                    count: culpritList.count
                    current: culpritList.at
                    onChosen: index => culpritList.at = index
                    onEntered: culpritList.open(screen.culprits[culpritList.at])
                }
                delegate: Item {
                    id: culpritRow
                    required property var modelData
                    required property int index
                    Accessible.role: Accessible.ListItem
                    Accessible.name: Theme.plural(modelData.kinds, "{} error, {}", "{} errors, {}", modelData.name || Theme.say("No mod recognised"))
                    width: culpritList.width
                    height: Theme.sized(34)
                    readonly property bool current: culpritList.at === index
                    Rectangle { anchors.fill: parent; anchors.leftMargin: Theme.gapSmall; anchors.rightMargin: Theme.gapSmall; radius: Theme.corner; color: culpritRow.current ? Theme.accent : Theme.hover; opacity: culpritRow.current ? 1 : culpritHover.hovered ? 0.04 : 0 }
                    HoverHandler { id: culpritHover; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: culpritList.at = culpritRow.index; onDoubleTapped: culpritList.open(culpritRow.modelData) }
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.gapWide
                        anchors.rightMargin: Theme.gapWide
                        spacing: Theme.gapLarge
                        Text { Layout.fillWidth: true; elide: Text.ElideRight; color: culpritRow.current ? Theme.onSelected : culpritRow.modelData.name ? Theme.ink : Theme.ink2; font.pixelSize: Theme.body; text: culpritRow.modelData.name || Theme.say("No mod recognised") }
                        Text { color: culpritRow.current ? Theme.onSelectedDim : Theme.ink2; font.pixelSize: Theme.body; text: Theme.plural(culpritRow.modelData.kinds, "{} error", "{} errors") + ", " + Theme.plural(culpritRow.modelData.times, "logged {} time", "logged {} times") }
                    }
                }
            }

            ListView {
                id: logView
                property int marked: 0
                visible: screen.view === 2
                Layout.fillWidth: true
                Layout.fillHeight: true
                topMargin: Theme.gapTiny
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                Wheel {}
                ScrollBar.vertical: ScrollBar {}
                model: screen.view === 2 ? screen.lines : []
                RowKeys {
                    parent: logView
                    view: logView
                    title: Theme.say("Log")
                    count: logView.count
                    current: Math.max(0, screen.lines.findIndex(l => l.number === logView.marked))
                    onChosen: index => logView.marked = screen.lines[index].number
                }
                delegate: Item {
                    id: logLine
                    required property var modelData
                    readonly property bool bad: screen.errorLines[modelData.number] === true
                    width: logView.width
                    height: Math.max(Theme.sized(20), lineText.implicitHeight + 4)
                    Rectangle { visible: logLine.modelData.number === logView.marked; anchors.fill: parent; color: Theme.accent; opacity: 0.25 }
                    Text { x: 8; width: Theme.sized(52); horizontalAlignment: Text.AlignRight; anchors.verticalCenter: parent.verticalCenter; color: Theme.ink3; font.family: "monospace"; font.pixelSize: Theme.small; text: logLine.modelData.number }
                    Text {
                        id: lineText
                        x: 72
                        width: parent.width - 90
                        anchors.verticalCenter: parent.verticalCenter
                        elide: Theme.stored.wrapLog ? Text.ElideNone : Text.ElideRight
                        wrapMode: Theme.stored.wrapLog ? Text.WrapAnywhere : Text.NoWrap
                        textFormat: Text.PlainText
                        color: logLine.bad ? Theme.err : Theme.ink
                        font.family: "monospace"
                        font.pixelSize: Theme.small
                        text: logLine.modelData.text
                    }
                    TapHandler { onTapped: logView.marked = logLine.modelData.number }
                    TapHandler { acceptedButtons: Qt.RightButton; onTapped: (point) => lineMenu.openAt(logLine, point.position.x, point.position.y, [{ text: Theme.say("Copy line"), icon: "copy", run: () => Theme.copy(logLine.modelData.text) }]) }
                }
                Text {
                    visible: logView.count === 0 && !screen.reader.logsBusy
                    anchors.centerIn: parent
                    color: Theme.ink2
                    font.pixelSize: Theme.body
                    text: screen.filterText !== "" ? Theme.say("No line matches") : Theme.say("No log")
                }
            }
            Text {
                visible: screen.view === 2 && (screen.report.lines || 0) > (screen.report.text || []).length
                Layout.leftMargin: Theme.gapWide
                Layout.bottomMargin: Theme.gapSmall
                color: Theme.ink3
                font.pixelSize: Theme.small
                text: Theme.fill("First {} of {} lines", Theme.count((screen.report.text || []).length), Theme.count(screen.report.lines || 0))
            }

            ListView {
                id: list
                visible: screen.view === 0
                Layout.fillWidth: true
                Layout.fillHeight: true
                topMargin: Theme.gapTiny
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                Wheel {}
                ScrollBar.vertical: ScrollBar {}
                model: screen.errors
                spacing: Theme.gapHair
                RowKeys {
                    parent: list
                    view: list
                    title: Theme.say("Errors")
                    count: list.count
                    current: screen.selected
                    onChosen: index => screen.selected = index
                }
                delegate: Item {
                    id: row
                    required property int index
                    required property var modelData
                    readonly property bool current: index === screen.selected
                    // The kind of error is the part before the first colon.
                    readonly property int cut: modelData.head.indexOf(":")
                    readonly property bool kinded: cut > 0 && cut <= 90
                    width: list.width
                    height: Math.max(Theme.sized(40), rowText.implicitHeight + Theme.sized(18))
                    Accessible.role: Accessible.ListItem
                    Accessible.name: modelData.head
                    Rectangle {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.gapLarge
                        anchors.rightMargin: Theme.gapCard
                        radius: Theme.corner
                        color: row.current ? Theme.accent : Theme.hover
                        opacity: row.current ? 1 : rowHover.hovered ? 0.05 : 0
                    }
                    Rectangle { visible: !row.current; anchors.bottom: parent.bottom; anchors.bottomMargin: -1; x: 22; width: parent.width - 48; height: 1; color: Theme.line; opacity: 0.6 }
                    HoverHandler { id: rowHover }
                    TapHandler { onTapped: screen.selected = row.index }
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.gapPage
                        anchors.rightMargin: 26
                        spacing: Theme.gapLarge
                        Rectangle {
                            // Once is the rule: the count shows from twice.
                            visible: row.modelData.count > 1
                            implicitWidth: Math.max(34, times.implicitWidth + 14)
                            implicitHeight: Theme.control
                            radius: Theme.corner
                            color: row.current ? Theme.selectedChip : Qt.rgba(Theme.err.r, Theme.err.g, Theme.err.b, 0.16)
                            Text { id: times; anchors.centerIn: parent; color: row.current ? Theme.onSelected : Theme.err; font.pixelSize: Theme.small; font.weight: Font.DemiBold; font.features: { "tnum": 1 }; text: "×" + Theme.count(row.modelData.count) }
                            Tip { text: Theme.say("How many times the log shows it") }
                        }
                        Column {
                            id: rowText
                            Layout.fillWidth: true
                            Layout.minimumWidth: 0
                            spacing: Theme.gapThin
                            Text { width: parent.width; elide: Text.ElideRight; color: row.current ? Theme.onSelected : Theme.ink; font.pixelSize: Theme.body; font.weight: Font.Medium; text: row.kinded ? row.modelData.head.substring(0, row.cut) : row.modelData.head }
                            Text { visible: row.kinded; width: parent.width; elide: Text.ElideRight; color: row.current ? Theme.onSelectedDim : Theme.ink2; font.pixelSize: Theme.small; text: row.modelData.head.substring(row.cut + 1).trim() }
                        }
                        Chip {
                            visible: row.modelData.alike > 1 && row.modelData.run === row.modelData.at && screen.filterText === ""
                            onAccent: row.current
                            pressable: true
                            icon: screen.openRuns[row.modelData.run] ? "chevron-down" : "chevron-right"
                            text: Theme.fill("{} more alike", (row.modelData.alike - 1))
                            tip: Theme.say("The errors after this one that differ from it only in their last word")
                            onClicked: screen.toggleRun(row.modelData.run)
                        }
                        Chip {
                            visible: !!row.modelData.mod
                            maxWidth: Math.min(Theme.sized(300), row.width * 0.36)
                            onAccent: row.current
                            pressable: row.modelData.modId >= 0
                            tone: Theme.warn
                            icon: "package"
                            text: row.modelData.mod || ""
                            tip: row.modelData.modId >= 0 ? Theme.say("The mod the error points at. Click to go to it.") : Theme.say("The mod the error points at")
                            onClicked: screen.showMod(row.modelData.modId)
                        }
                    }
                }

                Column {
                    visible: screen.errors.length === 0
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 80, 480)
                    spacing: Theme.gapSmall
                    Text {
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        color: screen.report.problem ? Theme.err : Theme.ink
                        font.pixelSize: Theme.title
                        font.weight: Font.DemiBold
                        text: screen.reader.logsBusy ? Theme.say("Reading the log…") : screen.report.problem ? Theme.say("Log could not be read") : screen.filterText !== "" ? Theme.say("No error matches") : screen.report.lines ? Theme.say("No errors") : Theme.say("No log")
                    }
                    Text {
                        visible: !screen.reader.logsBusy
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        color: Theme.ink2
                        font.pixelSize: Theme.body
                        lineHeight: 1.3
                        text: screen.report.problem || (screen.filterText !== "" ? ""
                            : screen.report.lines ? Theme.fill("{} lines", Theme.count(screen.report.lines))
                            : Theme.say("Written when the game starts"))
                    }
                }
            }
        }

        Rectangle { Layout.fillHeight: true; implicitWidth: 1; color: Theme.edge }

        Rectangle {
            Layout.preferredWidth: Math.min(Theme.sized(360), screen.width * 0.32)
            Layout.fillHeight: true
            color: Theme.panel

            Flickable {
                anchors.fill: parent
                contentHeight: (modSide.visible ? modSide.implicitHeight : side.implicitHeight) + 28
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                Wheel {}
                ScrollBar.vertical: ScrollBar {}

                // Under Mods: the chosen mod and what the log holds against it.
                Column {
                    id: modSide
                    readonly property var mod: screen.view === 1 ? screen.culprits[culpritList.at] || null : null
                    readonly property var theirs: mod ? (screen.report.errors || []).filter(e => (e.mod || "") === mod.name) : []
                    x: 14
                    y: 14
                    width: parent.width - 28
                    visible: mod !== null
                    spacing: Theme.gap
                    Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink; font.pixelSize: Theme.heading; font.weight: Font.DemiBold; text: modSide.mod ? modSide.mod.name || Theme.say("No mod recognised") : "" }
                    Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.body; text: modSide.mod ? Theme.plural(modSide.mod.kinds, "{} error", "{} errors") + ", " + Theme.plural(modSide.mod.times, "logged {} time", "logged {} times") : "" }
                    Flow {
                        width: parent.width
                        spacing: Theme.gapSmall
                        Btn { text: Theme.say("Show its errors"); tip: Theme.say("Errors, narrowed to this mod"); onClicked: culpritList.open(modSide.mod) }
                        Btn { visible: !!modSide.mod && modSide.mod.modId >= 0; text: Theme.say("Go to mod"); onClicked: screen.showMod(modSide.mod.modId) }
                    }
                    GroupTitle { text: Theme.say("Errors") }
                    Repeater {
                        model: modSide.theirs
                        Column {
                            id: theirError
                            required property var modelData
                            width: modSide.width
                            Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink; font.pixelSize: Theme.body; text: theirError.modelData.head }
                            Text { visible: theirError.modelData.count > 1; color: Theme.ink3; font.pixelSize: Theme.small; text: Theme.plural(theirError.modelData.count, "logged {} time", "logged {} times") }
                        }
                    }
                }
                Column {
                    id: side
                    x: 14
                    y: 14
                    width: parent.width - 28
                    visible: screen.error !== null && screen.view !== 1

                    CopyText {
                        width: parent.width
                        font.pixelSize: Theme.heading
                        font.weight: Font.DemiBold
                        text: screen.error ? (screen.error.head.indexOf(":") > 0 && screen.error.head.indexOf(":") < 90 ? screen.error.head.substring(0, screen.error.head.indexOf(":")) : screen.error.head) : ""
                    }
                    Text {
                        width: parent.width
                        topPadding: Theme.gapHair
                        wrapMode: Text.WordWrap
                        color: Theme.ink2
                        font.pixelSize: Theme.body
                        text: screen.error ? Theme.plural(screen.error.count, "{} time, first at line {}", "{} times, first at line {}", Theme.count(screen.error.line)) : ""
                    }

                    Item { width: 1; height: 10 }
                    Row {
                        spacing: Theme.gapSmall
                        Btn { text: Theme.say("Show in log"); onClicked: screen.showLine(screen.error.line) }
                        Btn { icon: "copy"; text: Theme.say("Copy report"); tip: Theme.say("The error, its stack trace, the game version and the likely mod, as text."); onClicked: Theme.copy(screen.helpText()) }
                    }

                    GroupTitle { text: Theme.say("Mods") }
                    Text {
                        visible: !!screen.error && screen.error.mods.length === 0
                        color: Theme.ink2
                        font.pixelSize: Theme.body
                        text: Theme.say("None recognised")
                        Tip { text: Theme.say("No active mod's code is in the stack trace, and the error names none.") }
                    }
                    Repeater {
                        model: screen.error ? screen.error.mods : []
                        RowLayout {
                            id: culprit
                            required property var modelData
                            width: side.width
                            height: Theme.sized(31)
                            spacing: Theme.gap
                            Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: culprit.modelData.name }
                            Btn { text: Theme.say("Go to mod"); onClicked: screen.showMod(culprit.modelData.modId) }
                        }
                    }

                    GroupTitle { text: Theme.say("Logged") }
                    CopyText { width: parent.width; text: screen.error ? screen.error.head : "" }

                    Item {
                        visible: !!screen.error && screen.error.trace.length > 0
                        readonly property bool stacked: traceTitle.implicitWidth + traceLines.implicitWidth + Theme.gap > width
                        width: parent.width
                        height: traceTitle.height + (stacked ? traceLines.height + Theme.gapTiny : 0)
                        GroupTitle { id: traceTitle; text: Theme.say("Stack trace") }
                        Choice {
                            id: traceLines
                            anchors.right: parent.stacked ? undefined : parent.right
                            anchors.bottom: parent.bottom
                            anchors.bottomMargin: Theme.gapTiny
                            title: Theme.say("Long lines")
                            options: ["Cut", "Wrap"]
                            current: Theme.stored.wrapTrace ? 1 : 0
                            onPicked: index => Theme.stored.wrapTrace = index === 1
                        }
                    }
                    Rectangle {
                        visible: !!screen.error && screen.error.trace.length > 0
                        width: parent.width
                        height: trace.implicitHeight + 20
                        radius: Theme.cornerLarge
                        color: Theme.well
                        clip: true
                        CopyText {
                            id: trace
                            x: 10
                            y: 10
                            width: parent.width - 20
                            // A line breaks only after a separator: a zero-width space follows each.
                            wrapMode: Theme.stored.wrapTrace ? TextEdit.Wrap : TextEdit.NoWrap
                            color: Theme.ink2
                            font.family: "monospace"
                            font.pixelSize: Theme.small
                            text: screen.error ? screen.error.trace.map(l => l.replace(/\s+$/, "").replace(/([.\/(,])/g, "$1\u200b")).join("\n") : ""
                        }
                    }
                }
            }
        }
    }
}
