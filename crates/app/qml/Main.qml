// The window: section switch on top, one screen at a time, status bar.
// `--demo` shows 2,300 made-up mods; `--selftest` measures and quits.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

ApplicationWindow {
    id: window
    width: 1440
    height: 860
    onWidthChanged: if (shotPath === "" && visible) Theme.stored.windowWidth = width
    onHeightChanged: if (shotPath === "" && visible) Theme.stored.windowHeight = height
    minimumWidth: 1000
    minimumHeight: 560
    visible: true
    title: Theme.say("RimStack")
    color: Theme.win

    // Which section is showing: 0 Mods, 1 Downloads, 2 Textures, 3 Saves,
    // 4 Logs, 5 Settings.
    // `--section=3` opens on that section; for screenshots and checks.
    property int section: {
        const given = Qt.application.arguments.find(a => a.startsWith("--section="))
        return given ? Number(given.substring(10)) : 0
    }

    // The one-page welcome, until Start is pressed once. Not for demo
    // data, and not when taking a picture of a screen.
    // `--first` shows the set-up card whatever the settings say; for screenshots.
    property bool firstRun: Qt.application.arguments.indexOf("--first") !== -1
        || (!Theme.demo && shotPath === "" && Mods.checkMode === "" && appBackend.setting("setupDone") === "")

    // What a careful player would settle before starting the game.
    readonly property var inTheWay: [
        Mods.errors > 0 ? Theme.plural(Mods.errors, "{} mod has a problem the game will trip on", "{} mods have problems the game will trip on") : "",
        Mods.dirty ? Theme.say("The list has unsaved changes; the game would start with the saved one") : "",
    ].filter(line => line !== "")
    // Starts the search by halves; the list as it is now is kept by name
    // first, so it can be had back whatever happens to the search.
    function findBroken() {
        const before = Mods.snapshot("@hunt")
        Notices.keep([Theme.kept("Began finding the broken mod")], before === "" ? "" : "list " + before)
        hunt.start()
    }
    // Looked up every few seconds: the game may be started and closed
    // from Steam as well as from here.
    property bool gameRunning: false
    Timer {
        interval: 3000
        repeat: true
        running: !Theme.demo && window.shotPath === ""
        triggeredOnStart: true
        onTriggered: window.gameRunning = appBackend.gameRunning()
    }
    // The game empties its list of mods when it first starts on a new
    // version: offer the list of the last launch, once per start.
    Timer {
        interval: 2500
        running: !Theme.demo && window.shotPath === "" && Mods.checkMode === "" && !window.firstRun
        onTriggered: {
            const ids = appLists.listResetByGame()
            if (ids !== "")
                Notices.offer(Theme.plural(JSON.parse(ids).length, "The game emptied its list of mods, as it does on a new version. The last launch from RimStack had {} mod.", "The game emptied its list of mods, as it does on a new version. The last launch from RimStack had {} mods."), Theme.say("Use that list"), () => window.useList(JSON.parse(ids)))
        }
    }
    function pressPlay() {
        if (gameRunning)
            return
        if (inTheWay.length > 0)
            topBar.beforePlay.open()
        else
            play()
    }

        Backend { id: appBackend }
        ModEdits { id: appEdits }
        CommunityData { id: appCommunity }
        BrokenModSearch { id: appHunt }
        KeptLists { id: appLists }
        Downloads { id: appDownloads }
        SaveReader { id: appSaves }
        TextureTools { id: appTextures }
        LogReader { id: appLogs }

    readonly property var behindItems: appDownloads.updates ? (JSON.parse(appDownloads.updates).items || []) : []
    readonly property int behind: behindItems.length
    // What the game would load: the count the window shows.
    readonly property int behindActive: behindItems.filter(item => item.active).length
    // The newest launch on record, for the status bar; read again when a
    // launch is recorded or the log has been read.
    property var lastLaunch: null
    function readLastLaunch() {
        if (Theme.demo)
            return
        const report = appLogs.logs ? JSON.parse(appLogs.logs) : ({})
        lastLaunch = JSON.parse(appBackend.lastStart(report.written || 0, !!report.previous, (report.errors || []).length))
        Mods.lastStart = lastLaunch
    }
    // What changed on disk while the window was open is read again
    // without being asked: looked for every two seconds while the window
    // is in front, and when the game stops.
    function look() {
        // `--live` keeps looking while a picture is taken; for checking this.
        if (Theme.demo || (Mods.checkMode !== "" && Mods.checkMode !== "live") || (shotPath !== "" && Qt.application.arguments.indexOf("--live") === -1))
            return
        const inTheWay = Mods.active.rescanning || Mods.active.previewing || Mods.drag.held !== null || Mods.comparing !== null
        const noticed = JSON.parse(appBackend.look(gameRunning, inTheWay))
        if (noticed.mods)
            Mods.active.rescan()
        if (noticed.theme)
            readOwnColours()
        if (noticed.language)
            Theme.words = JSON.parse(appBackend.languageWords(appBackend.setting("language")))
        if (noticed.saves && section === 3)
            appSaves.loadSaves()
        if (noticed.log) {
            appLogs.keepLogs()
            if (section === 4 && logsLoader.item)
                logsLoader.item.load()
            Qt.callLater(readLastLaunch)
        }
        // Never taken unasked: the game empties the file when it first
        // starts on a new version.
        if (noticed.modsConfig) {
            const ids = JSON.parse(appLists.listOnDisk() || "[]")
            Notices.offer(Theme.plural(ids.length, "ModsConfig.xml was written outside RimStack and now lists {} mod; the list here has {}.", "ModsConfig.xml was written outside RimStack and now lists {} mods; the list here has {}.", Mods.active.count), Theme.say("Use the file's"), () => { window.useList(ids); Mods.save() })
        }
    }
    // While the window shows, in front or not: a list beside the game or Steam stays true.
    Timer { interval: Mods.checkMode === "live" ? 500 : 2000; repeat: true; triggeredOnStart: true; running: (window.visibility !== Window.Minimized && window.visibility !== Window.Hidden) || Mods.checkMode === "live"; onTriggered: window.look() }
    onGameRunningChanged: {
        if (gameRunning && !Theme.demo)
            appDownloads.noteGameStarted()
        else
            look()
    }
    // The font chosen in Settings, for every text at once.
    function useFont(name) {
        const from = Qt.application.font.family
        const to = appBackend.useFont(name)
        Theme.refont(window.contentItem, from, to)
        Theme.refont(Overlay.overlay, from, to)
    }

    function readOwnColours() {
        Theme.custom = JSON.parse(appBackend.ownTheme(Theme.name))
    }
    Connections {
        target: Theme
        function onNameChanged() { window.readOwnColours() }
    }
    Component.onCompleted: {
        if (shotPath === "") {
            width = Math.max(minimumWidth, Theme.stored.windowWidth)
            height = Math.max(minimumHeight, Theme.stored.windowHeight)
        } else {
            // `--size=1000x560` takes the picture at that size.
            const given = (Qt.application.arguments.find(a => a.startsWith("--size=")) || "").substring(7).split("x")
            if (given.length === 2) {
                width = Number(given[0])
                height = Number(given[1])
            }
        }
        const kept = JSON.parse(appBackend.settings())
        // The interface size of earlier versions: now part of the text size.
        const size = Number(kept.interfaceScale || 1)
        if (size !== 1 && size > 0) {
            Theme.stored.textScale = Math.round(Theme.stored.textScale * size * 20) / 20
            appBackend.setSetting("interfaceScale", "")
        }
        readOwnColours()
        if (!Theme.demo) {
            appDownloads.restoreUpdates()
            appLogs.keepLogs()
        }
        Theme.words = JSON.parse(appBackend.languageWords(kept.language || ""))
        readLastLaunch()
    }
    Connections {
        target: appLogs
        function onLogsChanged() { window.readLastLaunch() }
    }

    Binding { target: Mods; property: "backend"; value: appBackend }
    Binding { target: Mods; property: "edits"; value: appEdits }
    Binding { target: Mods; property: "communityData"; value: appCommunity }
    Connections {
        target: appCommunity
        function onRulesBusyChanged() {
            if (appCommunity.rulesBusy)
                return
            Mods.active.communityChanged()
            Mods.overview = JSON.parse(Mods.active.overview())
        }
    }
    Binding { target: Mods; property: "hunter"; value: appHunt }
    Binding { target: Mods; property: "keptLists"; value: appLists }
    Binding { target: Mods; property: "downloads"; value: appDownloads }
    Binding { target: Mods; property: "textureTools"; value: appTextures }
    Binding { target: Notices; property: "keptLists"; value: appLists }
    // What a background task says when it ends becomes a notice; the
    // tray shows it while it runs.
    Connections {
        target: appDownloads
        function onTaskNoteChanged() {
            const note = appDownloads.taskNote
            if (note === "" || appDownloads.taskRunning)
                return
            const failed = appDownloads.taskFailed
            Notices.say(Theme.said(note), failed ? "err" : "ok")
            if (!failed)
                Notices.record(note, "")
        }
        function onModsWrittenChanged() {
            Mods.active.rescan()
        }
    }
    // What runs in the background now: { text, cancel }.
    readonly property var tasks: [
        Mods.active.rescanning ? { text: Theme.say("Reading the mod folders…") } : null,
        !Mods.active.filesRead && Mods.active.count > 0 && !Theme.demo ? { text: Theme.say("Reading the active mods' files…") } : null,
        appTextures.texturesBusy ? { text: appTextures.texturesProgress || Theme.say("Counting textures…"), cancel: () => appTextures.cancelTextures() } : null,
        appDownloads.updatesBusy ? { text: Theme.say("Asking Steam for updates…") } : null,
        appCommunity.rulesBusy ? { text: Theme.say("Downloading community data…") } : null,
        appDownloads.taskRunning ? { text: appDownloads.taskNote } : null,
    ].filter(task => task !== null)
    Binding { target: Mods; property: "behind"; value: window.behindActive }
    Binding { target: Mods; property: "hunting"; value: !!hunt.search.active }
    Connections {
        target: Mods
        function onShowUpdates() { window.section = 1 }
        function onFindBrokenRequested() { window.findBroken() }
        function onShowLogs() { window.section = 4 }
        function onRescanned() {
            appSaves.forget()
            appTextures.forget()
            appLogs.forget()
            // What Downloads lists names mods by their place in the last
            // reading of the folders: asked again, not emptied.
            if (!Theme.demo)
                appDownloads.restoreUpdates()
        }
    }

    function showMod(modId) {
        section = 0
        Mods.showMod(modId)
    }

    function useList(ids) {
        Mods.useList(ids)
        section = 0
    }

    // `round`: one try of the search for the broken mod. It is not a
    // launch to record, and it must not push the copies of the save from
    // before the search out of the few that are kept.
    function play(round) {
        // A copy of the newest save, in case this launch damages it.
        const problem = Theme.demo || round ? "" : appSaves.backUpNewestSave()
        if (problem !== "")
            console.warn("save not backed up: " + problem)
        if (!Theme.demo && !round) {
            appBackend.recordLaunch()
            readLastLaunch()
        }
        const how = JSON.parse(appBackend.settings())
        if (how.launchDirectly) {
            const failed = appBackend.startGameDirectly()
            if (failed !== "")
                console.warn("game not started: " + failed)
            return
        }
        const given = (how.launchArguments || "").trim()
        Qt.openUrlExternally(given === "" ? "steam://rungameid/294100" : "steam://run/294100//" + encodeURIComponent(given) + "/")
    }

    // One item holding the whole window, so a picture of it has the background.
    Rectangle {
        id: everything
        anchors.fill: parent
        color: Theme.win

        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            // Under the welcome screen nothing takes the keyboard.
            enabled: !window.firstRun

            TopBar {
                id: topBar
                keptLists: appLists
                Layout.fillWidth: true
                section: window.section
                behind: window.behindActive
                gameRunning: window.gameRunning
                inTheWay: window.inTheWay
                onGo: section => window.section = section
                onSearch: commands.open()
                onPressPlay: window.pressPlay()
                onPlay: window.play()
                onUseList: ids => window.useList(ids)
            }

            HuntBar {
                hunter: appHunt
                id: hunt
                Layout.fillWidth: true
                // The round's list goes on the Mods screen; whoever is reading the log stays there.
                onUseList: ids => Mods.useList(ids, true)
                onSaveAndPlay: if (Mods.save()) window.play(true)
            }

            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: window.section
                ModsScreen {}
                // The other screens are built the first time they are
                // shown, so starting up costs only the Mods screen.
                Loader {
                    active: window.section === 1 || item !== null
                    sourceComponent: DownloadsScreen {
                        downloads: appDownloads
                        backend: appBackend
                        onShowMod: modId => window.showMod(modId)
                    }
                }
                Loader {
                    active: window.section === 2 || item !== null
                    sourceComponent: TexturesScreen { textureTools: appTextures }
                }
                Loader {
                    active: window.section === 3 || item !== null
                    sourceComponent: SavesScreen {
                        keptLists: appLists
                        backend: appBackend
                        saveReader: appSaves
                        onUseList: ids => window.useList(ids)
                    }
                }
                Loader {
                    id: logsLoader
                    active: window.section === 4 || item !== null
                    sourceComponent: LogsScreen {
                        reader: appLogs
                        onUseList: ids => window.useList(ids)
                        onShowMod: modId => window.showMod(modId)
                        onFindBroken: window.findBroken()
                        hunting: !!hunt.search.active
                    }
                }
                Loader {
                    id: settingsLoader
                    active: window.section === 5 || item !== null
                    sourceComponent: SettingsScreen {
                        communityData: appCommunity
                        downloads: appDownloads
                        keptLists: appLists
                        backend: appBackend
                        reader: appLogs
                                                rules: Mods.active
                        overview: Mods.overview
                        onUseList: ids => window.useList(ids)
                        onTrust: (modId, yes) => Mods.trust(modId, yes)
                        onFont: family => window.useFont(family)
                    }
                }
            }

            StatusBar {
                keptLists: appLists
                textureTools: appTextures
                id: statusBar
                Layout.fillWidth: true
                lastLaunch: window.lastLaunch
                tasks: window.tasks
                onUseList: ids => window.useList(ids)
                onKeys: keysSheet.open()
                onLogs: window.section = 4
            }
        }

        Toasts {
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            anchors.leftMargin: Theme.gapCard
            anchors.bottomMargin: 38
            z: 20
        }
    }


    // `--shot=/path/file.png` saves a picture of the window's contents
    // two seconds after start and quits; for checking screens.
    readonly property string shotPath: {
        const given = Qt.application.arguments.find(a => a.startsWith("--shot="))
        return given ? given.substring(7) : ""
    }
    Timer {
        // `--shot-delay=6000` waits longer, for screens that fetch something.
        interval: {
            const given = Qt.application.arguments.find(a => a.startsWith("--shot-delay="))
            return given ? Number(given.substring(13)) : 2000
        }
        running: window.shotPath !== ""
        // With `--open=`, the picture is of that menu's or dialog's
        // contents: the window's own picture leaves those out.
        onTriggered: (window.opened ? window.opened.contentItem : everything).grabToImage(result => {
            result.saveToFile(window.shotPath)
            Qt.quit()
        })
    }

    // A newer RimStack is looked for once the window is up, unless Settings says not to, and said once.
    Timer { interval: 4000; running: !Theme.demo && window.shotPath === "" && Mods.checkMode === ""; onTriggered: if (JSON.parse(appBackend.settings()).selfCheck) appBackend.checkRelease() }
    // Steam's site refused past the last wait: said once, with how far the asking got.
    Connections {
        target: appDownloads
        function onChangeNotesStateChanged() {
            const state = appDownloads.changeNotesState ? JSON.parse(appDownloads.changeNotesState) : ({})
            if (state.refused)
                Notices.say(Theme.plural(state.asked, "Steam's site still refuses: {} mod asked, change notes of {} read", "Steam's site still refuses: {} mods asked, change notes of {} read", state.done), "err")
        }
    }
    Connections {
        target: appBackend
        function onReleaseChanged() {
            const found = appBackend.release ? JSON.parse(appBackend.release) : ({})
            if (found.version)
                Notices.offer(Theme.fill("RimStack {} is published; this is {}", found.version, JSON.parse(appBackend.settings()).version), Theme.say("Open its page"), () => Qt.openUrlExternally(found.page))
        }
    }

    Commands {
        id: commands
        x: Math.round((window.width - width) / 2)
        y: 90
        findMods: text => JSON.parse(Mods.active.findMods(text, 12)).map(m => ({
            name: m.name, group: m.active ? Theme.say("Active mod") : Theme.say("Inactive mod"), run: () => window.showMod(m.modId) }))
        commands: {
            const folders = Theme.demo ? ({}) : JSON.parse(appBackend.settings())
            const open = path => Qt.openUrlExternally(Theme.fileUrl(path))
            const mod = Mods.selectedId >= 0 && Mods.selectedCount === 1 ? Mods.info : null
            const filter = text => { window.section = 0; Mods.filterBy(text) }
            const settings = part => { window.section = 5; Qt.callLater(() => settingsLoader.item.part = part) }
            const logs = view => { window.section = 4; Qt.callLater(() => logsLoader.item.view = view) }
            // Each setting by name, so the search finds it: the page it is on.
            const found = [
                ["RimWorld folder", 0], ["Config and saves folder", 0], ["Workshop mods folder", 0], ["Play starts the game: through Steam or directly", 0], ["Launch arguments", 0], ["Git for Workshop mods", 0],
                [Theme.say("Theme"), 1], [Theme.say("Language"), 1], [Theme.say("Colours"), 1], [Theme.say("Font"), 1], [Theme.say("Text size"), 1], [Theme.say("Mod rows"), 1], [Theme.say("Motion"), 1], [Theme.say("Contrast"), 1], [Theme.say("First-run hints"), 1], [Theme.say("Workshop pages open in"), 1],
                [Theme.say("Community databases"), 2], [Theme.say("Game version mismatch"), 2], [Theme.say("Patch and def hints"), 2], [Theme.say("Mods marked as working"), 2],
                [Theme.say("Verify game files with Steam"), 3], [Theme.say("Set mod settings aside"), 3], [Theme.say("List from before RimStack"), 3], [Theme.say("RimStack's data folder"), 3], [Theme.say("Version and licence"), 3],
            ].map(setting => ({ name: Theme.say(setting[0]), group: Theme.say("Setting"), run: () => settings(setting[1]) }))
            return found.concat([
                { name: Theme.say("Keyboard shortcuts"), group: Theme.say("Help"), keys: "?", run: () => keysSheet.open() },
                { name: Theme.say("Report a problem"), group: Theme.say("Help"), run: () => Qt.openUrlExternally(Theme.issues) },
                { name: Theme.say("Show the first-run hints again"), group: Theme.say("Help"), run: () => { Theme.stored.hintsSeen = false; window.section = 0; Mods.startHints() } },
                { name: Theme.say("Find the broken mod"), group: Theme.say("Game"), run: () => window.findBroken(), when: !Theme.demo && !hunt.search.active },
                { name: Theme.say("All problems"), group: Theme.say("Load order"), run: () => { window.section = 0; Mods.problemsOpen = true }, when: Mods.troubles.length > 0 },
                { name: Theme.say("What RimStack changed"), group: Theme.say("Help"), run: () => statusBar.historyMenu.open(), when: !Theme.demo },
                { name: Theme.say("Sort"), group: Theme.say("Load order"), run: () => { window.section = 0; Mods.previewSort() }, when: !Theme.demo },
                { name: Theme.say("Save"), group: Theme.say("Load order"), keys: Theme.say("Ctrl + S"), run: () => Mods.save(), when: Mods.dirty },
                { name: Theme.say("Undo"), group: Theme.say("Load order"), keys: Theme.say("Ctrl + Z"), run: () => Mods.undo(), when: Mods.active.canUndo },
                { name: Theme.say("Redo"), group: Theme.say("Load order"), keys: Theme.say("Ctrl + Shift + Z"), run: () => Mods.redo(), when: Mods.active.canRedo },
                { name: Theme.say("Play"), group: Theme.say("Game"), run: () => window.pressPlay() },
                { name: Theme.say("Next problem"), group: Theme.say("Load order"), run: () => { window.section = 0; Mods.showNextProblem() }, when: Mods.errors + Mods.active.warnings > 0 },
                { name: Theme.say("Saved lists"), group: Theme.say("Load order"), run: () => topBar.listsMenu.open(), when: !Theme.demo },
                { name: Theme.say("Use the list from before RimStack"), group: Theme.say("Load order"), run: () => { const ids = appLists.originalList(); if (ids !== "") window.useList(JSON.parse(ids)) }, when: !!folders.backup },

                { name: Theme.fill(Mods.selectedActive ? "Deactivate {}" : "Activate {}", mod ? mod.name : ""), group: Theme.say("Selected mod"), run: () => Mods.toggleSelected(), when: !!mod && !Mods.active.previewing },
                { name: Theme.fill("Open folder of {}", (mod ? mod.name : "")), group: Theme.say("Selected mod"), run: () => open(mod.path), when: !!mod && !!mod.path },
                { name: Theme.fill("Workshop page of {}", (mod ? mod.name : "")), group: Theme.say("Selected mod"), run: () => Theme.openWorkshop(mod.folder), when: !!mod && mod.source === "Workshop" },
                { name: Theme.fill("Copy package id of {}", (mod ? mod.name : "")), group: Theme.say("Selected mod"), run: () => Theme.copy(mod.packageId), when: !!mod && !!mod.packageId },
                { name: Theme.say("Clear selection"), group: Theme.say("Selected mod"), keys: Theme.say("Esc"), run: () => Mods.select(-1, false), when: Mods.selectedId >= 0 },

                { name: Theme.say("Use the list on the clipboard"), group: Theme.say("Load order"), run: () => { const ids = JSON.parse(appLists.idsInText(Theme.pasted())); if (ids.length > 0) window.useList(ids) }, when: !Theme.demo },
                { name: Theme.say("Copy the load order as text"), group: Theme.say("Load order"), run: () => Theme.copy(Mods.active.listing()) },
                { name: Theme.say("Mods with errors"), group: Theme.say("Filter"), run: () => filter("is:error"), when: Mods.errors > 0 },
                { name: Theme.say("Mods with warnings"), group: Theme.say("Filter"), run: () => filter("is:warning"), when: Mods.active.warnings > 0 },
                { name: Theme.say("Mods sharing textures"), group: Theme.say("Filter"), run: () => filter("is:conflict"), when: Mods.active.conflicts > 0 },
                { name: Theme.say("Mods with an update"), group: Theme.say("Filter"), run: () => filter("is:updated"), when: window.behind > 0 },
                { name: Theme.say("Mods with code"), group: Theme.say("Filter"), run: () => filter("has:code") },
                { name: Theme.say("Local mods"), group: Theme.say("Filter"), run: () => filter("is:local") },
                { name: Theme.say("Clear filter"), group: Theme.say("Filter"), run: () => filter("") },

                { name: Theme.say("Read the mod folders again"), group: Theme.say("Mods"), run: () => Mods.active.rescan(), when: !Theme.demo },
                { name: Theme.say("Check for updates"), group: Theme.say("Downloads"), run: () => { window.section = 1; appDownloads.checkUpdates() }, when: !Theme.demo },
                { name: Theme.say("Update community rules"), group: Theme.say("Sorting"), run: () => appCommunity.downloadRules(), when: !Theme.demo && !appCommunity.rulesCurrent },
                { name: Theme.say("Read the saves again"), group: Theme.say("Saves"), run: () => { window.section = 3; appSaves.loadSaves() }, when: !Theme.demo },
                { name: Theme.say("Read the log again"), group: Theme.say("Logs"), run: () => { window.section = 4; Qt.callLater(() => logsLoader.item.load()) }, when: !Theme.demo },
                { name: Theme.say("Back up the newest save"), group: Theme.say("Saves"), run: () => appSaves.backUpNewestSave(), when: !Theme.demo },

                { name: Theme.say("Mods"), group: Theme.say("Go to"), run: () => window.section = 0 },
                { name: Theme.say("Downloads"), group: Theme.say("Go to"), run: () => window.section = 1 },
                { name: Theme.say("Textures"), group: Theme.say("Go to"), run: () => window.section = 2 },
                { name: Theme.say("Saves"), group: Theme.say("Go to"), run: () => window.section = 3 },
                { name: Theme.say("Logs: errors"), group: Theme.say("Go to"), run: () => logs(0) },
                { name: Theme.say("Logs: mods"), group: Theme.say("Go to"), run: () => logs(1) },
                { name: Theme.say("Logs: full log"), group: Theme.say("Go to"), run: () => logs(2) },
                { name: Theme.say("Settings: game and folders"), group: Theme.say("Go to"), run: () => settings(0) },
                { name: Theme.say("Settings: appearance"), group: Theme.say("Go to"), run: () => settings(1) },
                { name: Theme.say("Settings: sorting"), group: Theme.say("Go to"), run: () => settings(2) },
                { name: Theme.say("Settings: RimStack"), group: Theme.say("Go to"), run: () => settings(3) },

                { name: Theme.say("Theme: dark"), group: Theme.say("View"), run: () => Theme.stored.theme = "dark", when: Theme.name !== "dark" },
                { name: Theme.say("Theme: OLED"), group: Theme.say("View"), run: () => Theme.stored.theme = "oled", when: Theme.name !== "oled" },
                { name: Theme.say("Theme: light"), group: Theme.say("View"), run: () => Theme.stored.theme = "light", when: Theme.name !== "light" },
                { name: Theme.say("Theme: Rim"), group: Theme.say("View"), run: () => Theme.stored.theme = "rim", when: Theme.name !== "rim" },
                { name: Theme.stored.comfortable ? Theme.say("Compact rows") : Theme.say("Comfortable rows"), group: Theme.say("View"), run: () => Theme.stored.comfortable = !Theme.stored.comfortable },
                { name: Mods.prefs.inactiveHidden ? Theme.say("Show the inactive list") : Theme.say("Hide the inactive list"), group: Theme.say("View"), run: () => Mods.prefs.inactiveHidden = !Mods.prefs.inactiveHidden },
                { name: (Mods.prefs.inactiveByName ? Theme.say("Inactive mods by category") : Theme.say("Inactive mods by name")), group: Theme.say("View"), run: () => Mods.prefs.inactiveByName = !Mods.prefs.inactiveByName },

                { name: Theme.say("RimWorld folder"), group: Theme.say("Open"), run: () => open(folders.game), when: !!folders.game },
                { name: Theme.say("Workshop mods folder"), group: Theme.say("Open"), run: () => open(folders.workshop), when: !!folders.workshop },
                { name: Theme.say("Config and saves folder"), group: Theme.say("Open"), run: () => open(folders.config), when: !!folders.config },
                { name: Theme.say("Game log"), group: Theme.say("Open"), run: () => open(folders.config + "/Player.log"), when: !!folders.config },
                { name: Theme.say("RimStack data folder"), group: Theme.say("Open"), run: () => open(folders.dataFolder), when: !!folders.dataFolder },
                { name: Theme.say("RimWorld store page"), group: Theme.say("Open"), run: () => Qt.openUrlExternally("steam://store/294100") },
            ])
        }
    }
    Shortcut { sequence: "Ctrl+K"; onActivated: commands.open() }
    Repeater {
        model: topBar.sections.length
        Item {
            id: sectionKey
            required property int index
            Shortcut { sequence: "Ctrl+" + (sectionKey.index + 1); onActivated: window.section = sectionKey.index }
        }
    }
    KeysSheet { id: keysSheet }
    Prompt {
        id: question
        readonly property var asked: Notices.asking || ({ title: "", lines: [], label: "", danger: false })
        onClosed: Notices.asking = null
        Connections {
            target: Notices
            function onAskingChanged() {
                if (Notices.asking !== null)
                    question.open()
            }
        }
        title: asked.title
        Repeater {
            model: question.asked.lines
            Text {
                required property string modelData
                width: parent.width
                // A folder is shown whole, broken anywhere; a sentence by words.
                wrapMode: modelData.charAt(0) === "/" ? Text.WrapAnywhere : Text.WordWrap
                color: modelData.charAt(0) === "/" ? Theme.ink : Theme.ink2
                font.pixelSize: modelData.charAt(0) === "/" ? Theme.small : Theme.body
                font.family: modelData.charAt(0) === "/" ? "monospace" : Qt.application.font.family
                text: modelData
            }
        }
        buttons: [
            Btn { text: Theme.say("Cancel"); onClicked: question.close() },
            Btn {
                primary: !question.asked.danger
                danger: !!question.asked.danger
                text: question.asked.label
                onClicked: {
                    const run = question.asked.run
                    question.close()
                    run()
                }
            }
        ]
    }
    // Not while typing: there "?" is a letter like any other.
    Shortcut { sequence: "?"; enabled: !(window.activeFocusItem && window.activeFocusItem.cursorPosition !== undefined); onActivated: keysSheet.open() }
    Shortcut { sequence: "F1"; onActivated: keysSheet.open() }

    Loader {
        parent: everything
        z: 10
        anchors.fill: parent
        active: window.firstRun
        sourceComponent: FirstRun {
            info: JSON.parse(appBackend.settings())
            databases: JSON.parse(appCommunity.databases())
            languages: JSON.parse(appBackend.languages())
            onSettingPicked: (name, value) => appBackend.setSetting(name, value)
            onLanguagePicked: name => {
                appBackend.setSetting("language", name)
                Theme.words = JSON.parse(appBackend.languageWords(name))
                Mods.active.rescan()
            }
            activeCount: Mods.active.count
            inactiveCount: Mods.inactive.count
            problem: Mods.active.problem
                        onStart: (withData, withRimSort) => {
                appBackend.setSetting("setupDone", "yes")
                if (withData && (!Mods.overview.communityMods || !Mods.overview.worksOn))
                    appCommunity.downloadRules()
                if (withRimSort)
                    appBackend.importRimSortRules()
                window.firstRun = false
                Mods.startHints()
            }
        }
    }

    // For pictures of what only shows after something is done:
    // `--notices` two notices, `--hints` the first hint, `--problems` the
    // problems panel, `--several` a few mods selected. Give `--hunt-works`
    // a copy of the game's settings: it saves.
    Timer {
        readonly property var given: Qt.application.arguments
        interval: 1200
        running: window.shotPath !== ""
        onTriggered: {
            if (given.indexOf("--notices") !== -1) {
                Notices.say(Theme.say("Saved"), "ok")
                Notices.offer(Theme.plural(3, "Deactivated {} mod", "Deactivated {} mods"), Theme.say("Undo"), () => {})
            }
            // `--hints` the first hint, `--hints=3` the third.
            const hint = given.find(a => a.startsWith("--hints"))
            if (hint)
                Mods.hintStep = Number(hint.substring(8)) || 1
            if (given.indexOf("--problems") !== -1)
                Mods.problemsOpen = true
            // `--font=Name` changes the font once the window is up.
            const font = given.find(a => a.startsWith("--font="))
            if (font)
                window.useFont(font.substring(7))
            // `--hunt-works`: a round of the search answered "works now"
            // and saved, as it looks afterwards.
            if (given.indexOf("--hunt-works") !== -1 && hunt.search.active) {
                hunt.answer(false)
                Mods.save()
            }
            // `--open=play`, `history`, `keys`, `search`, `ask`, `lists` or
            // `tray` opens that menu or dialog.
            const open = (given.find(a => a.startsWith("--open=")) || "").substring(7)
            window.opened = { play: topBar.beforePlay, history: statusBar.historyMenu, keys: keysSheet, search: commands, lists: topBar.listsMenu, tray: statusBar.trayMenu, ask: question }[open] || null
            if (window.opened && open !== "ask")
                window.opened.open()
            if (open === "ask")
                Notices.ask({ title: Theme.say("Set the mods' settings aside"), lines: [Theme.say("Every Mod_*.xml is moved out of"), "/home/example/.config/unity3d/Ludeon Studios/RimWorld by Ludeon Studios/Config", Theme.say("Mods start from their defaults. Moving the files back restores them.")], label: Theme.say("Set aside"), run: () => {} })
            if (given.indexOf("--several") !== -1) {
                Mods.select(Mods.nthMod(Mods.active, 14), true)
                for (const at of [15, 16, 19])
                    Mods.selectAlso(Mods.nthMod(Mods.active, at), true)
            }
        }
    }

    property var opened: null
    property bool readyMarked: false
    onFrameSwapped: {
        if (!readyMarked) {
            readyMarked = true
            Mods.firstFrame()
        }
    }
}
