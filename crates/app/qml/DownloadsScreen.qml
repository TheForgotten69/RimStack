// The Downloads screen: which Workshop mods have a newer version than the
// one on disk, and looking up Workshop links before subscribing. Steam
// does the downloading itself; RimStack shows what is behind and opens
// the right page.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Item {
    id: screen
    Appear { of: screen }

    required property Backend backend
    signal showMod(int modId)

    readonly property var report: backend.updates ? JSON.parse(backend.updates) : ({})
    readonly property var updates: report.items || []
    readonly property var found: backend.queue ? JSON.parse(backend.queue) : ({})
    readonly property var queue: found.items || []
    readonly property var repositories: found.repositories || []
    // Read again whenever a task ends: installing SteamCMD is one.
    readonly property bool steamcmd: (backend.taskNote, JSON.parse(backend.settings()).steamcmd)
    // The mods the load order needs and does not have active.
    readonly property var needed: Mods.troubles.filter(t => /^(activate|subscribe|search) /.test(t.fix))
    readonly property var info: visible ? JSON.parse(backend.settings()) : ({})
    // The mods that changed since the game was last seen starting;
    // read again when the screen shows and when the mod folders were.
    readonly property var since: visible && !Theme.demo ? (backend.updates, Mods.active.libraryVersion, JSON.parse(backend.sinceLaunch())) : ({ time: 0, items: [] })
    // Per Workshop id, the entries Steam's site gave: { time, text }.
    readonly property var notes: backend.changeNotes ? JSON.parse(backend.changeNotes) : ({})

    function ago(seconds) {
        const minutes = Math.floor((Date.now() / 1000 - seconds) / 60)
        if (minutes < 1)
            return Theme.say("just now")
        if (minutes < 60)
            return (minutes === 1 ? Theme.fill("{} minute ago", minutes) : Theme.fill("{} minutes ago", minutes))
        if (minutes < 60 * 48)
            return Theme.fill("{} hours ago", Math.floor(minutes / 60))
        if (minutes < 60 * 24 * 60)
            return Theme.fill("{} days ago", Math.floor(minutes / 1440))
        return Theme.fill("{} months ago", Math.floor(minutes / 43200))
    }

    property bool sinceOpen: false
    // "" for all the changed mods, "active" or "inactive" for those alone.
    property string sinceOnly: ""
    // By the day of the change, newest first; within a day active before inactive.
    readonly property var changedMods: since.items.slice().sort((a, b) => day(b.now).localeCompare(day(a.now)) || Number(b.active) - Number(a.active) || b.now - a.now)
    readonly property var changedShown: sinceOnly === "" ? changedMods : changedMods.filter(item => item.active === (sinceOnly === "active"))
    function day(seconds) {
        return seconds ? Theme.day(seconds) : ""
    }

    function when(seconds) {
        return Theme.moment(seconds)
    }

    function megabytes(n) {
        return !n ? "" : n < 10 ? Theme.fill("{} MB", n.toFixed(1)) : Theme.fill("{} MB", Theme.count(Math.round(n)))
    }

    readonly property var activeUpdates: updates.filter(item => item.active)
    readonly property var inactiveUpdates: updates.filter(item => !item.active)
    property bool inactiveOpen: false
    // The Workshop ids Steam was asked to download, until they are no longer behind.
    property var asked: []
    readonly property var checkouts: report.checkouts || []
    readonly property var checkoutsBehind: checkouts.filter(c => c.behind > 0 || !!c.problem || !c.checked)
    property bool checkoutsOpen: false
    // The checkout being brought up to its remote, by its folder.
    property string updating: ""
    Connections {
        target: screen.backend
        function onTaskNoteChanged() {
            if (screen.backend.taskNote.indexOf("…") === -1)
                screen.updating = ""
        }
    }

    // The Workshop ids of the mods whose suggested replacement is set aside.
    property var ignoredReplacements: JSON.parse(backend.settings()).replacementsIgnored || []
    property int replacementsRead: 0
    function ignoreReplacements(ids) {
        backend.setSetting("replacementsIgnored", ids.join(","))
        ignoredReplacements = ids
        replacementsRead += 1
        Mods.tick++
    }

    component UpdateRow: RowLayout {
        id: row
        required property var modelData
        width: page.width
        height: Theme.sized(30)
        spacing: Theme.gapLarge
        Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: row.modelData.name }
        Chip { visible: screen.asked.indexOf(row.modelData.id) !== -1; tone: Theme.ok; icon: "check"; text: Theme.say("Asked of Steam") }
        Chip { tone: Theme.moved; text: screen.day(row.modelData.have) + " → " + screen.day(row.modelData.now); tip: Theme.fill("Published {}", screen.ago(row.modelData.now)) }
        Btn { text: Theme.say("Show"); spoken: Theme.fill("Show {}", row.modelData.name); onClicked: screen.showMod(row.modelData.modId) }
        Btn { text: Theme.say("What changed"); spoken: Theme.fill("What changed in {}", row.modelData.name); tip: Theme.say("The mod's change notes on the Workshop."); onClicked: Theme.openChangeNotes(row.modelData.id) }
    }

    component Cell: Text {
        elide: Text.ElideRight
        color: Theme.ink2
        font.pixelSize: Theme.body
    }

    Flickable {
        id: scroller
        anchors.fill: parent
        contentHeight: page.implicitHeight + 32
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        Wheel {}
        ScrollBar.vertical: ScrollBar {}

        Column {
            id: page
            x: Math.max(16, (parent.width - width) / 2)
            y: 16
            width: Math.min(Theme.sized(900), screen.width - 32)
            spacing: Theme.gapPage

            RowLayout {
                width: parent.width
                spacing: Theme.gap
                FilterField {
                    id: pasted
                    Layout.fillWidth: true
                    implicitHeight: Theme.sized(30)
                    placeholder: Theme.say("Paste Workshop links, collection links, mod ids or Git links")
                    onAccepted: add.clicked()
                }
                Btn {
                    id: add
                    enabled: !screen.backend.queueBusy && pasted.text.trim() !== ""
                    primary: pasted.text.trim() !== ""
                    text: screen.backend.queueBusy ? Theme.say("Asking Steam…") : Theme.say("Look up")
                    onClicked: screen.backend.lookUp(pasted.text)
                }
                        Btn {
                primary: !screen.report.checked
                enabled: !screen.backend.updatesBusy
                icon: "refresh-cw"
                text: Theme.say("Check for updates")
                tip: screen.info.gitForWorkshop ? Theme.say("Asks Steam for the current version of each installed Workshop mod (only their Workshop ids are sent), and Git for new commits on each checkout in the game's Mods folder and in the Workshop folder.")
                    : Theme.say("Asks Steam for the current version of each installed Workshop mod (only their Workshop ids are sent), and Git for new commits on each checkout in the game's Mods folder.")
                onClicked: screen.backend.checkUpdates()
            }
            }

            Card {
                title: Theme.say("Updates")
                note: screen.backend.updatesBusy ? Theme.say("Asking Steam…")
                    : !screen.report.checked ? Theme.say("Not checked")
                    : Theme.fill("{} for active mods, checked {}", screen.activeUpdates.length, screen.ago(screen.report.checked))
                actions: [
                    Btn {
                        visible: screen.updates.length > 0
                        primary: screen.activeUpdates.length > 0
                        enabled: screen.backend.taskNote.indexOf("…") === -1
                        icon: "download"
                        text: Theme.say("Update in Steam")
                        tip: Theme.say("Steam, which must be running, downloads the newer version of each now. Left alone it does so the next time it starts.")
                        onClicked: {
                            const ids = screen.updates.map(item => item.id)
                            screen.asked = ids
                            screen.backend.askSteam("download", JSON.stringify(ids))
                        }
                    }
                ]
                Rectangle {
                    visible: screen.backend.updatesBusy
                    width: parent.width
                    height: 3
                    radius: Math.min(width, height) / 2
                    color: Theme.chip
                    clip: true
                    Rectangle {
                        width: parent.width / 4
                        height: parent.height
                        radius: Math.min(width, height) / 2
                        color: Theme.accent
                        NumberAnimation on x { running: screen.backend.updatesBusy && !Theme.stored.reduceMotion; from: -page.width / 4; to: page.width; duration: 1200; loops: Animation.Infinite }
                    }
                }
                Text { visible: !!screen.report.problem; width: parent.width; wrapMode: Text.WordWrap; color: Theme.err; font.pixelSize: Theme.body; text: Theme.fill("Steam not reached: {}", (screen.report.problem || "")) }
                Text { visible: !screen.report.problem && screen.updates.length === 0 && !!screen.report.checked; color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.say("All Workshop mods are current") }
                Repeater {
                    model: screen.activeUpdates
                    UpdateRow {}
                }
                Row {
                    visible: screen.inactiveUpdates.length > 0
                    topPadding: screen.activeUpdates.length > 0 ? 8 : 0
                    spacing: Theme.gap
                    Text { color: Theme.ink2; font.pixelSize: Theme.small; font.weight: Font.DemiBold; text: Theme.say("Inactive mods") }
                    Text { color: Theme.ink3; font.pixelSize: Theme.small; text: screen.inactiveUpdates.length }
                }
                Repeater {
                    model: screen.inactiveOpen ? screen.inactiveUpdates : screen.inactiveUpdates.slice(0, 3)
                    UpdateRow {}
                }
                Btn {
                    visible: screen.inactiveUpdates.length > 3
                    text: screen.inactiveOpen ? Theme.fill("Show {}", 3) : Theme.fill("{} more", (screen.inactiveUpdates.length - 3))
                    onClicked: screen.inactiveOpen = !screen.inactiveOpen
                }
            }

            Card {
                visible: screen.checkouts.length > 0
                title: Theme.say("From Git")
                note: screen.checkoutsBehind.length > 0 ? Theme.fill("{} behind", screen.checkoutsBehind.length) : Theme.say("all up to date")
                Repeater {
                    model: screen.checkoutsOpen ? screen.checkouts : screen.checkoutsBehind
                    RowLayout {
                        id: checkout
                        required property var modelData
                        readonly property bool updating: screen.updating === modelData.path
                        width: page.width
                        height: Theme.sized(30)
                        spacing: Theme.gapLarge
                        Icon { name: "git-branch"; size: Theme.icon; color: Theme.ink3 }
                        Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: checkout.modelData.name }
                        Chip {
                            maxWidth: Theme.sized(340)
                            tone: checkout.modelData.problem || !checkout.modelData.checked ? Theme.ink2 : checkout.modelData.behind > 0 ? Theme.moved : Theme.ok
                            text: checkout.modelData.problem ? Theme.fill("Not checked: {}", checkout.modelData.problem)
                                : !checkout.modelData.checked ? Theme.say("Not checked")
                                : checkout.modelData.behind > 0 ? Theme.plural(checkout.modelData.behind, "{} commit behind", "{} commits behind") : Theme.say("Up to date")
                        }
                        Btn { text: Theme.say("Show"); spoken: Theme.fill("Show {}", checkout.modelData.name); onClicked: screen.showMod(checkout.modelData.modId) }
                        Btn {
                            visible: !checkout.modelData.workshop && checkout.modelData.behind > 0
                            enabled: screen.updating === ""
                            primary: !checkout.updating
                            text: checkout.updating ? Theme.say("Updating…") : Theme.say("Update")
                            spoken: Theme.fill("Update {}", checkout.modelData.name)
                            tip: Theme.say("Moves forward to the remote; stops if the folder has changes of its own.")
                            onClicked: {
                                screen.updating = checkout.modelData.path
                                screen.backend.updateFromGit(checkout.modelData.path)
                            }
                        }
                    }
                }
                RowLayout {
                    readonly property int settled: screen.checkouts.length - screen.checkoutsBehind.length
                    visible: settled > 0
                    width: page.width
                    height: Theme.sized(30)
                    spacing: Theme.gapLarge
                    Text { Layout.fillWidth: true; color: Theme.ink2; font.pixelSize: Theme.body; text: screen.checkoutsOpen ? "" : Theme.fill("{} not behind", parent.settled) }
                    Btn { text: screen.checkoutsOpen ? Theme.say("Only those behind") : Theme.say("Show all"); onClicked: screen.checkoutsOpen = !screen.checkoutsOpen }
                }
            }

            Card {
                visible: screen.since.items.length > 0
                title: Theme.say("Changed since the game last started")
                note: Theme.fill("{}, since {}", screen.since.items.length, screen.when(screen.since.time))
                key: "downloads-changed"
                actions: [
                    Choice {
                        readonly property int active: screen.since.items.filter(item => item.active).length
                        visible: active > 0 && active < screen.since.items.length
                        title: Theme.say("Changed mods shown")
                        options: [Theme.say("All"), Theme.fill("Active {}", active), Theme.fill("Inactive {}", screen.since.items.length - active)]
                        current: ["", "active", "inactive"].indexOf(screen.sinceOnly)
                        onPicked: index => screen.sinceOnly = ["", "active", "inactive"][index]
                    },
                    AskSteam {
                    readonly property var asked: screen.since.items.filter(item => item.kind === "workshop").map(item => [item.id, item.was])
                    visible: asked.length > 0
                    backend: screen.backend
                    idle: Theme.say("Fetch change notes")
                    tip: Theme.say("Reads each mod's change notes from its Workshop page. Only the Workshop ids are sent.")
                    onClicked: screen.backend.fetchChangeNotes(JSON.stringify(asked))
                    }
                ]
                spacing: Theme.gapLarge
                // One Tab stop for the rows: the arrows move, Enter shows the mod, N opens its change notes.
                Column {
                id: changedRows
                property int at: 0
                readonly property var shown: screen.sinceOpen ? screen.changedShown : screen.changedShown.slice(0, 12)
                function go(to) {
                    at = Math.max(0, Math.min(shown.length - 1, to))
                    const row = rows.itemAt(at)
                    if (!row)
                        return
                    const top = row.mapToItem(scroller.contentItem, 0, 0).y
                    if (top < scroller.contentY)
                        scroller.contentY = Math.max(0, top - Theme.gapLarge)
                    else if (top + row.height > scroller.contentY + scroller.height)
                        scroller.contentY = top + row.height + Theme.gapLarge - scroller.height
                }
                width: page.width
                spacing: Theme.gapLarge
                activeFocusOnTab: shown.length > 0
                Accessible.role: Accessible.List
                Accessible.name: Theme.say("Changed since the game last started")
                Keys.onUpPressed: go(at - 1)
                Keys.onDownPressed: go(at + 1)
                Keys.onReturnPressed: if (shown[at]) screen.showMod(shown[at].modId)
                Keys.onEnterPressed: if (shown[at]) screen.showMod(shown[at].modId)
                Keys.onPressed: event => {
                    if (event.key !== Qt.Key_N || !shown[at] || shown[at].kind !== "workshop")
                        return
                    Theme.openChangeNotes(shown[at].id)
                    event.accepted = true
                }
                Repeater {
                    id: rows
                    model: changedRows.shown
                    Column {
                        id: changed
                        readonly property bool reached: changedRows.activeFocus && changedRows.at === index
                        required property var modelData
                        required property int index
                        readonly property string day: screen.day(modelData.now)
                        readonly property bool opensDay: index === 0 || screen.day(screen.changedShown[index - 1].now) !== day
                        readonly property var said: modelData.kind === "git" ? modelData.notes : (screen.notes[modelData.id] || [])
                        width: page.width
                        spacing: Theme.gapTiny
                        Text {
                            visible: changed.opensDay
                            topPadding: changed.index === 0 ? 0 : Theme.gap
                            bottomPadding: Theme.gapThin
                            color: Theme.ink2
                            font.pixelSize: Theme.small
                            font.weight: Font.DemiBold
                            text: changed.day + "  " + Theme.plural(screen.changedShown.filter(item => screen.day(item.now) === changed.day).length, "{} mod", "{} mods")
                        }
                        RowLayout {
                            width: parent.width
                            spacing: Theme.gap
                            Icon { name: changed.modelData.kind === "git" ? "git-branch" : "cloud"; size: 14; color: changed.reached ? Theme.focus : Theme.ink3 }
                            Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; font.weight: Font.Medium; font.underline: changed.reached; text: changed.modelData.name; Accessible.role: Accessible.ListItem; Accessible.name: text }
                            Chip { visible: !changed.modelData.active; text: Theme.say("inactive") }
                            Chip { visible: changed.modelData.kind === "git"; tone: Theme.moved; text: Theme.plural(changed.modelData.notes.length, "{} commit", "{} commits") }
                            Text {
                                visible: changed.modelData.kind !== "git"
                                color: Theme.ink3
                                font.pixelSize: Theme.small
                                text: screen.day(changed.modelData.was) === changed.day ? Theme.clock(changed.modelData.was) + " → " + Theme.clock(changed.modelData.now)
                                    : screen.day(changed.modelData.was) + " → " + (changed.day === screen.day(Date.now() / 1000) ? Theme.say("today") : changed.day)
                            }
                            Btn { activeFocusOnTab: false; text: Theme.say("Show"); spoken: Theme.fill("Show {}", changed.modelData.name); tip: Theme.say("Enter"); onClicked: screen.showMod(changed.modelData.modId) }
                            Btn { activeFocusOnTab: false; visible: changed.modelData.kind === "workshop"; icon: "external-link"; spoken: Theme.fill("Change notes of {}", changed.modelData.name); tip: Theme.say("The mod's change notes on the Workshop."); onClicked: Theme.openChangeNotes(changed.modelData.id) }
                        }
                        Repeater {
                            model: changed.said
                            RowLayout {
                                id: entry
                                required property var modelData
                                width: changed.width
                                spacing: Theme.gap
                                Text { Layout.preferredWidth: Theme.sized(22); color: Theme.ink3; font.pixelSize: Theme.small; text: "" }
                                Text { Layout.preferredWidth: Theme.sized(74); Layout.alignment: Qt.AlignTop; color: Theme.ink3; font.pixelSize: Theme.small; text: entry.modelData.time ? screen.day(entry.modelData.time) : "" }
                                Text {
                                    Layout.fillWidth: true
                                    wrapMode: Text.WordWrap
                                    maximumLineCount: 12
                                    elide: Text.ElideRight
                                    textFormat: Text.PlainText
                                    color: entry.modelData.time ? Theme.ink2 : Theme.err
                                    font.pixelSize: Theme.small
                                    lineHeight: 1.25
                                    text: entry.modelData.text !== "" ? entry.modelData.text : Theme.say("No note written")
                                }
                            }
                        }
                    }
                }
                }
                Btn {
                    visible: screen.changedShown.length > 12
                    text: screen.sinceOpen ? Theme.fill("Show {}", 12) : Theme.fill("{} more", (screen.changedShown.length - 12))
                    onClicked: screen.sinceOpen = !screen.sinceOpen
                }
            }

            Card {
                visible: screen.queue.length + screen.repositories.length > 0 || !!(screen.found.problem || screen.found.note)
                title: Theme.say("Look up")
                note: screen.queue.length + screen.repositories.length > 0 ? screen.queue.length + screen.repositories.length : ""
                actions: [
                    Btn {
                        readonly property var wanted: screen.queue.filter(item => !!item.title && !item.installed).map(item => item.id)
                        visible: wanted.length > 0
                        icon: "plus"
                        text: Theme.say("Subscribe in Steam")
                        tip: Theme.say("Steam, which must be running, subscribes to each and downloads it; it keeps them up to date afterwards.")
                        onClicked: screen.backend.askSteam("subscribe", JSON.stringify(wanted))
                    },
                    Btn {
                        readonly property var wanted: screen.queue.filter(item => !!item.title && !item.installed).map(item => item.id)
                        visible: wanted.length > 0 && screen.steamcmd
                        icon: "download"
                        text: Theme.say("Download with SteamCMD…")
                        tip: Theme.fill("Into the game's own Mods folder, {}/Mods, without subscribing: Steam will not update these.", (screen.info.game || ""))
                        onClicked: Notices.ask({
                            title: Theme.plural(wanted.length, "Download {} mod with SteamCMD", "Download {} mods with SteamCMD"),
                            lines: [Theme.say("Each is downloaded and copied into the game's own Mods folder:"), (screen.info.game || "") + "/Mods", Theme.say("Steam does not update mods installed this way.")],
                            label: Theme.say("Download"), run: () => screen.backend.downloadWithSteamcmd(JSON.stringify(wanted)) })
                    }
                ]
                Text { visible: screen.queue.length === 0 && !!(screen.found.problem || screen.found.note); width: parent.width; wrapMode: Text.WordWrap; color: screen.found.problem ? Theme.err : Theme.ink2; font.pixelSize: Theme.body; text: screen.found.problem || screen.found.note || "" }
                Repeater {
                    model: screen.repositories
                    RowLayout {
                        id: repository
                        required property var modelData
                        width: page.width
                        height: Theme.sized(32)
                        spacing: Theme.gapLarge
                        Icon { name: "git-branch"; size: 14; color: Theme.ink3 }
                        Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: repository.modelData.name }
                        Chip { visible: repository.modelData.installed; tone: Theme.ok; icon: "check"; text: Theme.say("Installed") }
                        Btn { icon: "external-link"; text: Theme.say("Repository page"); onClicked: Qt.openUrlExternally(repository.modelData.link) }
                        Btn {
                            visible: !repository.modelData.installed
                            enabled: !!screen.info.git && !!repository.modelData.into
                            icon: "download"
                            text: Theme.say("Clone…")
                            tip: screen.info.git ? Theme.say("git clone into the game's own Mods folder; Check for updates then asks its repository") : Theme.say("Git is not installed")
                            onClicked: Notices.ask({
                                title: Theme.fill("Clone {}", repository.modelData.name),
                                lines: [Theme.say("git clone, from"), repository.modelData.link, Theme.say("Into"), repository.modelData.into],
                                label: Theme.say("Clone"), run: () => screen.backend.cloneMod(repository.modelData.link) })
                        }
                    }
                }
                Repeater {
                    model: screen.queue
                    RowLayout {
                        id: item
                        required property var modelData
                        width: page.width
                        height: Theme.sized(32)
                        spacing: Theme.gapLarge
                        Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: item.modelData.title || item.modelData.id }
                        Cell { Layout.preferredWidth: Theme.sized(100); text: screen.day(item.modelData.updated) }
                        Cell { Layout.preferredWidth: Theme.sized(70); horizontalAlignment: Text.AlignRight; text: screen.megabytes(item.modelData.megabytes) }
                        Chip { visible: !item.modelData.title; tone: Theme.err; icon: "circle-alert"; text: Theme.say("Not on the Workshop") }
                        Chip { visible: !!item.modelData.title && item.modelData.installed; tone: Theme.ok; icon: "check"; text: Theme.say("Installed") }
                        Btn { visible: !!item.modelData.title && !item.modelData.installed; icon: "external-link"; text: Theme.say("Workshop page"); onClicked: Theme.openWorkshop(item.modelData.id) }
                    }
                }
            }

            Card {
                visible: screen.needed.length > 0
                title: Theme.say("Needed by the load order")
                note: screen.needed.length
                tone: Theme.err
                spacing: Theme.gap
                Repeater {
                    model: screen.needed
                    ProblemLine {
                        required property var modelData
                        width: page.width
                        name: modelData.name
                        words: modelData.words
                        fix: modelData.fix
                        fixLabel: modelData.fixLabel
                        modId: modelData.modId
                    }
                }
            }

            Card {
                visible: replaced.count > 0 || screen.ignoredReplacements.length > 0
                title: Theme.say("Replacements suggested")
                note: Theme.fill("{}, from the Use This Instead list", replaced.count)
                key: "downloads-replaced"
                actions: [
                    Chip {
                        visible: screen.ignoredReplacements.length > 0
                        pressable: true
                        icon: "eye-off"
                        text: Theme.fill("{} ignored", screen.ignoredReplacements.length)
                        tip: Theme.say("Suggest them again")
                        onClicked: screen.ignoreReplacements([])
                    }
                ]
                Repeater {
                    id: replaced
                    model: screen.visible && screen.replacementsRead >= 0 && Mods.tick >= 0 ? JSON.parse(screen.backend.replacements()) : []
                    RowLayout {
                        id: old
                        required property var modelData
                        width: page.width
                        height: Theme.sized(46)
                        spacing: Theme.gapLarge
                        Column {
                            Layout.fillWidth: true
                            Layout.minimumWidth: 0
                            spacing: Theme.gapHair
                            RowLayout {
                                width: parent.width
                                spacing: Theme.gap
                                Text { Layout.maximumWidth: parent.width - (wasOff.visible ? wasOff.width + Theme.gap : 0); elide: Text.ElideRight; color: Theme.ink2; font.pixelSize: Theme.body; text: old.modelData.name }
                                Chip { id: wasOff; visible: !old.modelData.active; text: Theme.say("inactive") }
                                Item { Layout.fillWidth: true }
                            }
                            RowLayout {
                                width: parent.width
                                spacing: Theme.gap
                                Icon { name: "corner-down-right"; size: Theme.icon; color: Theme.ink3 }
                                Text { Layout.maximumWidth: parent.width - Theme.icon - Theme.gap - (hasNew.visible ? hasNew.width + Theme.gap : 0); elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; font.weight: Font.Medium; text: old.modelData.newName }
                                Chip { id: hasNew; visible: old.modelData.haveNew; tone: Theme.ok; icon: "check"; text: Theme.say("installed") }
                                Item { Layout.fillWidth: true }
                            }
                        }
                        Btn { text: Theme.say("Show old mod"); spoken: Theme.fill("Show {}", old.modelData.name); tip: Theme.say("The installed mod the list would replace"); onClicked: screen.showMod(old.modelData.modId) }
                        Btn { visible: !old.modelData.haveNew; icon: "external-link"; text: Theme.say("Replacement"); tip: Theme.say("Workshop page of the replacement"); onClicked: Theme.openWorkshop(old.modelData.newId) }
                        Btn { icon: "eye-off"; text: Theme.say("Ignore"); spoken: Theme.fill("Ignore the replacement of {}", old.modelData.name); tip: Theme.say("No replacement is suggested for this mod again"); onClicked: screen.ignoreReplacements(screen.ignoredReplacements.concat([old.modelData.id])) }
                        Btn {
                            danger: true
                            text: Theme.say("Unsubscribe…")
                            spoken: Theme.fill("Unsubscribe from {}", old.modelData.name)
                            tip: Theme.say("Lets the replaced mod go: Steam unsubscribes and deletes its folder")
                            onClicked: { const about = Mods.active.details(old.modelData.modId); Mods.unsubscribe(old.modelData.name, about.folder, about.path) }
                        }
                    }
                }
            }
            RowLayout {
                width: parent.width
                spacing: Theme.gap
                Text { color: Theme.ink3; font.pixelSize: Theme.small; text: Theme.say("SteamCMD, for mods without subscribing") ; Tip { text: Theme.say("Valve's own downloader, fetched from Valve into RimStack's data folder. It downloads Workshop mods without a Steam account, into the game's Mods folder. It runs with a home folder of its own, so it cannot touch Steam's settings.") } }
                Chip { visible: screen.steamcmd; tone: Theme.ok; icon: "check"; text: Theme.say("Installed") }
                Btn {
                    visible: !screen.steamcmd
                    text: Theme.say("Install…")
                    onClicked: Notices.ask({
                        title: Theme.say("Install SteamCMD"),
                        lines: [Theme.say("Valve's own downloader is fetched from Valve into"), (screen.info.dataFolder || "") + "/steamcmd", Theme.say("It runs with a home folder of its own there, apart from Steam's settings.")],
                        label: Theme.say("Install"), run: () => screen.backend.installSteamcmd() })
                }
                Item { Layout.fillWidth: true }
            }
        }
    }
}
