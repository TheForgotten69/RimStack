// The state of the Mods screen and everything that changes it: the two
// lists, the selection, a drag, a sort on show. Views read it and call it;
// it knows nothing of them and tells them what to do through signals.
pragma Singleton
pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Item {
    id: mods

    // ---- what the views are told ----
    // A change is about to reset both lists, and has: a list keeps its place.
    signal changing()
    signal changed()
    signal reveal(var model, int row, bool centre)
    signal focusList(var model)
    signal filterRequested(string text)
    // What the one filter over both lists holds now.
    property string filtered: ""
    signal filterFocusRequested()
    // Open a menu of { text, icon, off, danger, run } at this place in an item.
    signal menuRequested(Item item, real x, real y, var entries)
    signal firstFrameShown()

    readonly property ModListModel active: activeModel
    readonly property ModListModel inactive: inactiveModel
    readonly property ModsPrefs prefs: ModsPrefs {}

    readonly property int activeCount: activeModel.count
    readonly property int inactiveCount: inactiveModel.count
    signal rescanned()
    // Goes up with every change to the lists or the mods on disk: what is worked out from them is asked for again.
    property int tick: 0
    // "after" or "before" while choosing the other mod for a rule of your
    // own about the selected mod; empty otherwise.
    property string picking

    function pick(otherId) {
        if (picking === "after")
            activeModel.addRule(otherId, selectedId)
        else
            activeModel.addRule(selectedId, otherId)
        picking = ""
    }

    readonly property bool dirty: activeModel.dirty
    readonly property int errors: activeModel.errors
    // For the selected mod: the library ids of the mods it shares
    // textures with, each "+" if the selected mod wins or "-" if it loses.
    readonly property var conflictMarks: {
        const marks = {}
        for (const mark of (info.conflictMarks || "").split(" ").filter(m => m)) {
            const [id, how] = mark.split(":")
            marks[id] = how
        }
        return marks
    }
    readonly property ModListModel loadOrder: activeModel
    property string saveProblem

    function undo() {
        keepingScroll(() => { activeModel.undo(); inactiveModel.sync() })
        refresh()
    }

    function redo() {
        keepingScroll(() => { activeModel.redo(); inactiveModel.sync() })
        refresh()
    }

    function discardChanges() {
        keepingScroll(() => { activeModel.discardChanges(); inactiveModel.sync() })
        done(Theme.say("Back to the saved list"))
        refresh()
    }

    function save() {
        if (activeModel.previewing)
            endSort(true)
        const changes = JSON.parse(activeModel.unsavedKept())
        const before = backend && !Theme.demo && checkMode === "" ? backend.snapshotSaved(keptName("@saving")) : ""
        saveProblem = activeModel.save()
        if (saveProblem === "" && checkMode === "") {
            Notices.keep([Theme.kept("Saved the load order: {}"), changes], before === "" ? "" : "list " + before)
        }
        return saveProblem === ""
    }

    function idAt(model, row) {
        return model.data(model.index(row, 0), 266)
    }

    function step(model, by) {
        const position = selectedId >= 0 ? model.indexOf(selectedId) : -1
        let row = position >= 0 ? model.rowOf(position) : (by > 0 ? -1 : model.rows())
        for (row += by; row >= 0 && row < model.rows(); row += by) {
            const id = idAt(model, row)
            if (id >= 0) {
                select(id, model === activeModel)
                reveal(model, row, false)
                return
            }
        }
    }

    // Enter: move the selected mod to the other list, and select the mod
    // that was next to it so the keyboard stays in this list.
    function activateSelected(model) {
        const position = selectedId >= 0 ? model.indexOf(selectedId) : -1
        if (position < 0 || activeModel.previewing)
            return
        const row = model.rowOf(position)
        let next = -1
        for (const candidate of [row + 1, row - 1, row + 2, row - 2]) {
            if (next < 0 && candidate >= 0 && candidate < model.rows())
                next = idAt(model, candidate)
        }
        moveAcross(model, position, selectedId)
        if (next >= 0)
            select(next, model === activeModel)
    }

    property Backend backend

    function compareWith(otherId) {
        const mine = activeModel.indexOf(selectedId)
        const theirs = activeModel.indexOf(otherId)
        if (mine < 0 || theirs < 0)
            return
        comparing = { earlier: mine < theirs ? selectedId : otherId, later: mine < theirs ? otherId : selectedId }
    }

    signal showLogs()
    // When the game was last started and how that went, as far as known:
    // { time, errors }. The same wherever it is said.
    property var lastStart: null
    // Every problem of every active mod, each with what solves it.
    readonly property var troubles: String(overview.troubleLines || "").split("\n").filter(line => line !== "").map(line => {
        const part = line.split("\t")
        return { modId: Number(part[0]), name: part[1], error: part[2] === "1", group: part[3], words: part[4], fix: part[5], fixLabel: part[6] }
    })
    // The needed mods that are installed and only switched off, by package id.
    readonly property var needed: [...new Set(troubles.filter(t => t.fix.startsWith("activate ")).map(t => t.fix.substring(9)))]
    readonly property int fixable: needed.length
    property bool problemsOpen: false
    signal problemsRequested()
    onProblemsRequested: problemsOpen = true

    // Where the last run was left: put back once the lists are filled.
    property bool leftOffRestored: false
    function restoreLeftOff() {
        // Not for a picture of a screen, unless `--resume` asks for it.
        const picture = Qt.application.arguments.some(a => a.startsWith("--shot=")) && Qt.application.arguments.indexOf("--resume") === -1
        if (leftOffRestored || Theme.demo || checkMode !== "" || picture || activeModel.count === 0)
            return
        leftOffRestored = true
        // Nothing is unsaved at the start, so "is:changed" is not put back.
        const kept = prefs.lastFilter.split(" ").filter(term => term !== "" && term !== "is:changed").join(" ")
        if (kept !== "")
            filterBy(kept)
        const key = prefs.lastMod
        const here = key === "" ? -1 : activeModel.packageMod(key)
        const there = key === "" || here >= 0 ? -1 : inactiveModel.packageMod(key)
        if (here >= 0 || there >= 0) {
            showMod(here >= 0 ? here : there)
            tab = prefs.lastTab
        }
        leftOffScroll(prefs.lastScroll)
    }
    signal leftOffScroll(real y)
    onTabChanged: if (leftOffRestored) prefs.lastTab = tab

    // Which of the three first-run hints shows; 0 for none.
    property int hintStep: 0
    function startHints() {
        if (!Theme.stored.hintsSeen && !Theme.demo && checkMode === "")
            hintStep = 1
    }
    function endHints() {
        hintStep = 0
        Theme.stored.hintsSeen = true
    }

    function applyFix(modId, code) {
        const what = code.split(" ")[0]
        const about = code.substring(what.length + 1)
        if (what === "activate" && !activeModel.previewing) {
            const needed = inactiveModel.packageMod(about)
            if (needed >= 0)
                moveAcross(inactiveModel, inactiveModel.indexOf(needed), needed)
        } else if (what === "deactivate" && !activeModel.previewing) {
            moveAcross(activeModel, Number(about), -1)
        } else if (what === "subscribe") {
            Theme.openWorkshop(about)
        } else if (what === "search") {
            Qt.openUrlExternally("https://steamcommunity.com/workshop/browse/?appid=294100&searchtext=" + encodeURIComponent(about))
        } else if (what === "trust") {
            trust(modId, true)
        }
    }

    function fixAll() {
        if (activeModel.previewing)
            return
        activeModel.beginBatch()
        keepingScroll(() => {
            for (const key of needed) {
                const id = inactiveModel.packageMod(key)
                if (id >= 0)
                    activeModel.takeFrom(inactiveModel, inactiveModel.indexOf(id))
            }
        })
        activeModel.endBatch()
        done(Theme.say("Activated the needed mods"))
        refresh()
    }

    function winAll(otherId) {
        activeModel.addRule(otherId, selectedId)
        const mine = activeModel.indexOf(selectedId)
        const theirs = activeModel.indexOf(otherId)
        if (mine >= 0 && theirs > mine)
            keepingScroll(() => activeModel.moveMod(mine, theirs))
        refresh()
    }

    // New update results: the is:updated filter must hear of them.
    onBehindChanged: if (!Theme.demo) activeModel.recheck()

    // No warning about the game version while the game stays on `version`;
    // "" brings the warnings back.
    function setVersionAside(version) {
        const before = JSON.parse(backend.settings()).versionAsideFor || ""
        backend.setSetting("versionAsideFor", version)
        keepingScroll(() => activeModel.recheck())
        inactiveModel.recheck()
        refresh()
        if (version !== "")
            Notices.offer(Theme.fill("Game version warnings are ignored for RimWorld {}", version), Theme.say("Undo"), () => setVersionAside(before))
    }

    // The selected mods that are warned about for the game version they list.
    readonly property var versionWarned: selectedCount < 2 ? [] : selectedIds.filter(id => troubles.some(t => t.modId === id && t.fix === "trust"))
    function trustSelected(yes) {
        const ids = yes ? versionWarned : selectedIds
        keepingScroll(() => {
            for (const id of ids)
                activeModel.setVersionOk(id, yes)
        })
        inactiveModel.recheck()
        refresh()
        overview = activeModel.overview()
    }

    function trust(modId, yes) {
        keepingScroll(() => activeModel.setVersionOk(modId, yes))
        inactiveModel.recheck()
        refresh()
        // Settings lists the mods marked.
        overview = activeModel.overview()
    }

    function putInSub(modId, name) {
        keepingScroll(() => {
            activeModel.setSub(modId, name)
            activeModel.sync()
            inactiveModel.sync()
        })
        refresh()
    }
    // The sub-category goes; its mods stay in the category.
    function removeSub(model, row) {
        keepingScroll(() => {
            model.removeSub(row)
            activeModel.sync()
            inactiveModel.sync()
        })
        refresh()
    }
    signal subNameRequested(int modId)
    // "tag" or "sub": ask for the word to give every selected mod.
    signal bulkRequested(string what)
    readonly property var selectedIds: selectedId < 0 ? [] : [selectedId].concat(Object.keys(also).map(Number))

    function tagSelected(tag) {
        const model = selectedActive ? activeModel : inactiveModel
        const ids = selectedIds
        keepingScroll(() => {
            for (const id of ids) {
                const about = activeModel.details(id)
                const tags = (about.tags || "").split(",").map(t => t.trim()).filter(t => t !== "")
                if (tags.indexOf(tag) < 0)
                    model.setNote(id, tags.concat([tag]).join(", "), about.note || "")
            }
        })
        Notices.say(Theme.fill("Tagged {} mods: {}", ids.length, tag), "ok")
    }

    function subSelected(name) {
        const ids = selectedIds
        keepingScroll(() => {
            for (const id of ids)
                activeModel.setSub(id, name)
            activeModel.sync()
            inactiveModel.sync()
        })
    }

    // Each selected mod to the top or bottom of its category's stretch,
    // keeping their order among themselves. One step for Undo.
    function selectedToEdge(top) {
        const ids = selectedIds.slice().sort((a, b) => activeModel.indexOf(a) - activeModel.indexOf(b))
        if (top)
            ids.reverse()
        activeModel.beginBatch()
        keepingScroll(() => {
            for (const id of ids)
                activeModel.moveToEdge(activeModel.indexOf(id), top)
        })
        activeModel.endBatch()
        done(Theme.fill("Moved {} mods", ids.length))
    }

    // T and N on a row: type the selected mod's tags or its note.
    signal editRequested(string what)

    // How many mods Sort would move now; counted a moment after the
    // list last changed, not on every change.
    property int sortPending: 0
    property Timer sortCount: Timer {
        interval: 400
        onTriggered: mods.sortPending = mods.active.filesRead && !mods.active.previewing ? mods.active.sortPending() : 0
    }
    onChanged: { tick++; sortCount.restart(); countFresh(); overview = activeModel.overview() }
    property Connections sortWatch: Connections {
        target: mods.active
        function onFilesReadChanged() { mods.sortCount.restart() }
        function onPreviewingChanged() { mods.sortCount.restart() }
    }
    signal binRequested(string name, string folder)

    // Why the last copy or removal of a mod did not work; empty when it did.
    property string upkeepProblem
    function copyToLocal(folder) {
        upkeepProblem = backend.copyToLocal(folder)
        if (upkeepProblem === "")
            activeModel.rescan()
    }
    function binMod(folder) {
        upkeepProblem = backend.binMod(folder)
        if (upkeepProblem === "")
            activeModel.rescan()
    }

    // The columns a load-order row can show after the name.
    readonly property var columnNames: ({ author: "Author", version: "Version", load: "Load" })
    readonly property var columnWidths: ({ author: Theme.sized(chosenWidths.author || 142), version: Theme.sized(chosenWidths.version || 84), load: Theme.sized(chosenWidths.load || 56) })
    // A column with nothing in it is not drawn: no load times are known
    // until the game has been started from here on its present version.
    readonly property var shownColumns: chosenColumns.filter(column => column !== "load" || overview.loadTimes === true)
    // The column the load order is shown by, "" for the load order itself.
    property string shownBy: ""
    property bool shownDescending: false
    function showBy(column) {
        if (column === "" || (shownBy === column && shownDescending))
            shownBy = ""
        else if (shownBy === column)
            shownDescending = true
        else {
            shownBy = column
            shownDescending = false
        }
        if (shownBy === "")
            shownDescending = false
        keepingScroll(() => activeModel.showBy(shownBy, shownDescending))
    }
    // The columns chosen.
    readonly property var chosenColumns: prefs.columns === "" ? [] : prefs.columns.split(",")
    function toggleColumn(column) {
        const now = chosenColumns.slice()
        const at = now.indexOf(column)
        if (at >= 0)
            now.splice(at, 1)
        else
            now.push(column)
        prefs.columns = now.join(",")
    }
    function moveColumn(column, to) {
        const now = chosenColumns.filter(c => c !== column)
        now.splice(Math.max(0, Math.min(now.length, to)), 0, column)
        prefs.columns = now.join(",")
    }
    // The colours a category's mark can take, at this place in an item.
    function categoryMenu(item, x, y, category) {
        const chosen = Theme.chosenSwatches[category] || Theme.swatches[category][0]
        const entries = [{ title: Theme.categoryNames[category] }]
        for (const swatch of Theme.swatches)
            entries.push({ text: Theme.say(swatch[0]) + (swatch[0] === chosen ? "  ✓" : ""), colour: Theme.swatch(swatch[0]), run: () => Theme.paintCategory(category, swatch[0]) })
        entries.push({ line: true })
        entries.push({ text: Theme.say("Its own colour"), icon: "undo-2", off: !Theme.chosenSwatches[category], run: () => Theme.paintCategory(category, "") })
        menuRequested(item, x, y, entries)
    }

    // The widths chosen for the columns, at 100% text: "author=180,load=70".
    readonly property var chosenWidths: {
        const chosen = {}
        for (const pair of prefs.columnSizes.split(",")) {
            const [column, width] = pair.split("=")
            if (Number(width) > 0)
                chosen[column] = Number(width)
        }
        return chosen
    }
    function sizeColumn(column, width) {
        const now = Object.assign({}, chosenWidths)
        now[column] = Math.round(Math.max(48, Math.min(420, width / Theme.grow)))
        prefs.columnSizes = Object.keys(now).map(key => key + "=" + now[key]).join(",")
    }

    function columnMenu() {
        return Object.keys(columnNames).map(column => ({
            text: (shownColumns.indexOf(column) >= 0 ? "✓  " : "     ") + Theme.say(columnNames[column]),
            run: () => toggleColumn(column)
        }))
    }

    function fileUnder(modId, label) {
        keepingScroll(() => {
            activeModel.setCategory(modId, label)
            activeModel.sync()
            inactiveModel.sync()
        })
        refresh()
    }

    function categoryChoices(modId) {
        const chosen = activeModel.details(modId).categoryChosen || ""
        const labels = ["Loaders", "Libraries", "Content", "Patches", "Textures", "Interface", "Performance & tweaks", "Loads last"]
        const entries = [{ text: Theme.say("Automatic"), icon: chosen === "" ? "check" : "", run: () => fileUnder(modId, "") }, { line: true }]
        for (const label of labels)
            entries.push({ text: label, icon: chosen === label ? "check" : "", run: () => fileUnder(modId, label) })
        return entries
    }

    readonly property RowMenu menus: RowMenu {}
    function rowMenu(row, x, y) {
        menus.open(row, x, y)
    }

    // Asks first, naming the folder Steam will delete.
    function unsubscribe(name, workshopId, folder) {
        Notices.ask({
            title: Theme.fill("Unsubscribe from {}", name),
            lines: [Theme.say("Steam, which must be running, unsubscribes and then deletes the mod's folder:"), folder],
            label: Theme.say("Unsubscribe"), danger: true, run: () => backend.askSteam("unsubscribe", JSON.stringify([workshopId])) })
    }

    function openFolder(modId) {
        const path = activeModel.details(modId).path
        if (path)
            Qt.openUrlExternally(Theme.fileUrl(path))
    }

    // The mod that was just moved by hand and how many places, so its
    // row can slide in from where it was; cleared a moment later.
    property var slid: ({ modId: -1, by: 0 })
    // True for a moment after Sort shows its order: moved rows slide in.
    property bool sorting: false
    property Timer slideEnd: Timer { interval: 400; onTriggered: { mods.slid = ({ modId: -1, by: 0 }); mods.sorting = false } }
    function slide(modId, by) {
        slid = ({ modId: modId, by: by })
        slideEnd.restart()
    }

    // Alt + Up and Alt + Down: the selected mod one place up or down.
    function nudge(model, by) {
        const at = selectedId >= 0 ? model.indexOf(selectedId) : -1
        const to = at + by
        if (at < 0 || to < 0 || to >= model.count || activeModel.previewing || (model === activeModel && shownBy !== ""))
            return
        slide(selectedId, -by)
        keepingScroll(() => model.moveMod(at, to))
        reveal(model, model.rowOf(to), false)
        refresh()
    }

    // The inactive mods installed since the game was last started from here.
    property var fresh: []
    function countFresh() {
        fresh = Theme.demo ? [] : JSON.parse(inactiveModel.freshIds())
    }
    onRescanned: { tick++; countFresh() }
    function activateFresh() {
        const ids = fresh.slice().sort((a, b) => inactiveModel.indexOf(a) - inactiveModel.indexOf(b))
        activeModel.beginBatch()
        keepingScroll(() => {
            for (const id of ids) {
                const index = inactiveModel.indexOf(id)
                if (index >= 0)
                    activeModel.takeFrom(inactiveModel, index)
            }
        })
        activeModel.endBatch()
        done(Theme.fill("Activated {} new mods", ids.length))
        countFresh()
        refresh()
    }

    function keepNote(tags, note) {
        keepingScroll(() => (selectedActive ? activeModel : inactiveModel).setNote(selectedId, tags, note))
        refresh()
    }

    function showMod(modId) {
        const at = activeModel.indexOf(modId)
        select(modId, at >= 0)
        if (at >= 0)
            reveal(activeModel, activeModel.rowOf(at), true)
        else if (inactiveModel.indexOf(modId) >= 0) {
            prefs.inactiveHidden = false
            reveal(inactiveModel, inactiveModel.rowOf(inactiveModel.indexOf(modId)), true)
        }
    }

    function filterBy(text) {
        filterRequested(text)
    }

    // `quiet` for a list that is one step of something larger: no
    // snapshot, no line in the history, no notice.
    function useList(ids, quiet) {
        const before = quiet ? "" : snapshot("@list")
        keepingScroll(() => { activeModel.useList(JSON.stringify(ids)); inactiveModel.sync() })
        if (!quiet) {
            Notices.keep(["#", ids.length, Theme.kept("Another list taken: {} mod"), Theme.kept("Another list taken: {} mods")], before === "" ? "" : "list " + before)
            done(Theme.plural(ids.length, "Using a list of {} mod", "Using a list of {} mods"))
        }
        refresh()
    }

    // The selected mod, by its id in the library; -1 for none.
    property int selectedId: -1
    property bool selectedActive: false
    property var info: ({})
    property var overview: ({})

    function select(modId, inActive) {
        selectedId = modId
        selectedActive = inActive
        also = ({})
        info = modId >= 0 ? activeModel.details(modId) : ({})
        if (leftOffRestored)
            prefs.lastMod = modId >= 0 ? String(info.packageId || "").toLowerCase() : ""
    }

    // More mods selected along with `selectedId`, all in the same list:
    // library id -> true. Ctrl + click adds or removes one; Shift + click
    // takes every mod from the selected one to the one clicked.
    // Which tab of the details panel shows: 0 About, 1 Load order, 2 Files.
    property int tab: 0

    property var also: ({})
    readonly property int selectedCount: selectedId < 0 ? 0 : 1 + Object.keys(also).length

    function selectAlso(modId, inActive) {
        if (selectedId < 0 || inActive !== selectedActive)
            return select(modId, inActive)
        if (modId === selectedId)
            return
        const next = Object.assign({}, also)
        if (next[modId])
            delete next[modId]
        else
            next[modId] = true
        also = next
    }

    function selectThrough(modId, inActive) {
        if (selectedId < 0 || inActive !== selectedActive)
            return select(modId, inActive)
        const model = inActive ? activeModel : inactiveModel
        const from = model.rowOf(model.indexOf(selectedId))
        const to = model.rowOf(model.indexOf(modId))
        if (from < 0 || to < 0)
            return
        const next = {}
        for (let row = Math.min(from, to); row <= Math.max(from, to); row++) {
            const id = idAt(model, row)
            if (id >= 0 && id !== selectedId)
                next[id] = true
        }
        also = next
    }

    function moveSelected() {
        const from = selectedActive ? activeModel : inactiveModel
        const ids = [selectedId].concat(Object.keys(also).map(Number))
        ids.sort((a, b) => from.indexOf(a) - from.indexOf(b))
        // One step for Undo, however many mods.
        activeModel.beginBatch()
        keepingScroll(() => {
            for (const id of ids) {
                const index = from.indexOf(id)
                if (index >= 0)
                    (from === activeModel ? inactiveModel : activeModel).takeFrom(from, index)
            }
        })
        activeModel.endBatch()
        done(from === activeModel ? Theme.plural(ids.length, "Deactivated {} mod", "Deactivated {} mods") : Theme.plural(ids.length, "Activated {} mod", "Activated {} mods"))
        select(-1, false)
    }

    // Reads the selected mod's details again; the others selected with it stay.
    function refresh() {
        const others = also
        select(selectedId, selectedActive)
        also = others
    }

        // How many active Workshop mods are behind their Workshop version, once
    // Downloads has asked Steam; and the request to go and see them.
    property int behind: 0
    signal showUpdates()
    // The search for the broken mod: asked for from Problems and before
    // Play, and whether one is under way.
    signal findBrokenRequested()
    property bool hunting: false

    property int problemRow: -1
    property int problemPlace: 0
    property int problemMod: -1
    readonly property int problemCount: errors + activeModel.warnings >= 0 && overview ? activeModel.problemCount() : 0
    function showNextProblem() {
        showProblem(true)
    }
    function showProblem(forward) {
        problemRow = activeModel.nextProblem(problemRow, forward)
        problemPlace = activeModel.problemPlace(problemRow)
        if (problemRow < 0)
            return
        reveal(activeModel, problemRow, false)
        select(activeModel.data(activeModel.index(problemRow, 0), 266), true)
        problemMod = selectedId
    }

        function previewSort() {
        if (activeModel.previewing || !activeModel.filesRead)
            return
        sorting = true
        slideEnd.restart()
        keepingScroll(() => activeModel.previewSort())
        // Once the list has laid its new rows out: before that it has no place to go to.
        Qt.callLater(() => {
            const row = activeModel.firstMoved()
            if (row >= 0)
                reveal(activeModel, row, true)
        })
        refresh()
    }

    // Keeps the load order as it is now among the snapshots (Lists menu),
    // before something rearranges all of it.
    // A kind from `Theme.keptKinds` and the time, to the second: also the file's name.
    function keptName(kind) {
        return kind + " " + Theme.moment(Date.now() / 1000).replace(":", ".") + "." + String(new Date().getSeconds()).padStart(2, "0")
    }
    function snapshot(kind) {
        if (!backend || Theme.demo || checkMode !== "")
            return ""
        const name = keptName(kind)
        backend.snapshot(name)
        return name
    }
    // Something was done that Undo takes back: say so, with the button.
    function done(text) {
        if (checkMode === "")
            Notices.offer(text, Theme.say("Undo"), () => undo())
    }

    function endSort(keep) {
        const applies = keep && activeModel.previewing
        const before = applies ? snapshot("@sorting") : ""
        const moves = applies ? String(activeModel.sortMoves()).split("\n").filter(line => line !== "").length : 0
        keepingScroll(() => keep ? activeModel.applySort() : activeModel.discardSort())
        if (applies) {
            const sorted = ["#", moves, Theme.kept("Sorted: {} mod moved"), Theme.kept("Sorted: {} mods moved")]
            Notices.keep(sorted, before === "" ? "" : "list " + before)
            done(Theme.put(sorted))
        }
        refresh()
    }

    // A change resets both lists; the lists put themselves back where
    // they were.
    function keepingScroll(change) {
        changing()
        change()
        changed()
    }

    function moveAcross(fromModel, modIndex, modId) {
        const to = fromModel === activeModel ? inactiveModel : activeModel
        const name = modId >= 0 ? (activeModel.details(modId).name || "") : ""
        keepingScroll(() => to.takeFrom(fromModel, modIndex))
        if (modId >= 0 && modId === selectedId)
            selectedActive = to === activeModel
        done(Theme.fill(to === activeModel ? "Activated {}" : "Deactivated {}", name))
        refresh()
    }

    function toggleSelected() {
        const from = selectedActive ? activeModel : inactiveModel
        const index = from.indexOf(selectedId)
        if (index >= 0)
            moveAcross(from, index, selectedId)
    }

    property var dragged: null
    property var dropTarget: null

    function finishDrag() {
        const from = dragged
        const to = dropTarget
        dragged = null
        dropTarget = null
        if (!from || !to)
            return
        if (from.model === to.model) {
            // Dropped below the last row: it becomes the last mod.
            const place = Math.min(to.modIndex, to.model.count - 1)
            if (from.modIndex !== place) {
                slide(from.modId, from.modIndex - place)
                keepingScroll(() => to.model.moveMod(from.modIndex, place))
            }
        } else {
            keepingScroll(() => to.model.takeFromAt(from.model, from.modIndex, to.modIndex))
            if (from.modId >= 0 && from.modId === selectedId)
                selectedActive = to.model === activeModel
        }
        refresh()
    }

    property point dragPoint


    // The two mods being compared, { earlier, later } by library id; null
    // when the lists are showing.
    property var comparing: null
    property var compareView: null

    ModListModel {
        id: inactiveModel
        demoCount: 300
        byName: prefs.inactiveByName
        onByNameChanged: reshape()
        Component.onCompleted: Theme.demo ? reload() : loadInstalled(false)
    }
    ModListModel {
        id: activeModel
                demoCount: 2000
        headings: prefs.headings
        onHeadingsChanged: reshape()
        Component.onCompleted: Theme.demo ? reload() : loadInstalled(true)
        onStatusChanged: mods.overview = overview()
        onCountChanged: Qt.callLater(mods.restoreLeftOff)
        onRulesBusyChanged: mods.overview = overview()
        onConflictsVersionChanged: mods.refresh()
        onRulesVersionChanged: mods.refresh()
        onLibraryVersionChanged: {
            // The mod folders were read again: ids from before are stale.
            mods.select(-1, false)
            inactiveModel.loadInstalled(false)
            mods.rescanned()
        }
    }

    function nthMod(model, position) {
        const row = model.rowOf(position)
        return row < 0 ? -1 : idAt(model, row)
    }
    readonly property string checkMode: {
        const given = Qt.application.arguments.find(a => a.startsWith("--check"))
        return given ? (given.substring(8) || "all") : ""
    }

    function firstFrame() {
        activeModel.markReady()
        firstFrameShown()
    }
}
