pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Item {
    id: checks
    required property var window
    property int failed: 0
    property int at: 0
    property int waited: 0

    function named(item, name) {
        if (item.objectName === name)
            return item
        for (const child of item.children) {
            const found = named(child, name)
            if (found)
                return found
        }
        return null
    }

    function scrollToEnd() {
        const grid = named(window.contentItem, "resultGrid")
        if (grid)
            grid.contentY = Math.max(0, grid.contentHeight - grid.height)
    }

    function listed(value) {
        return Array.from(value || [])
    }

    function check(what, ok) {
        console.log("check " + (ok ? "ok   " : "FAIL ") + what)
        if (!ok)
            failed += 1
    }

    readonly property var steps: [
        {
            section: Theme.sections.saves,
            what: "Saves lists both saves, with the mods each was saved with",
            ready: () => !Hub.saves.busy && (Hub.saves.saves || []).length > 0,
            ok: () => checks.listed(Hub.saves.saves).length === 2 && checks.listed(Hub.saves.saves).every(save => save.ids.length >= 43)
        },
        {
            section: Theme.sections.saves,
            what: "and says the save with a mod that is not installed breaks the list",
            ready: () => true,
            ok: () => checks.listed(Hub.saves.saves).filter(save => save.verdict.breaks).length === 1
        },
        {
            section: Theme.sections.logs,
            what: "Logs reads the game's log and groups its errors",
            ready: () => !Hub.logs.busy && !!Hub.logs.head,
            ok: () => Hub.logs.head.errors >= 2 && Hub.logs.errors.length === 2
        },
        {
            section: Theme.sections.logs,
            what: "and shows its lines",
            ready: () => (Hub.logs.lines || []).length > 0,
            ok: () => checks.listed(Hub.logs.lines).some(line => line.error)
        },
        {
            section: Theme.sections.textures,
            what: "Textures counts the textures of the mods",
            ready: () => !Hub.textures.busy && !!Hub.textures.report && Hub.textures.report.all > 0,
            ok: () => Hub.textures.report.all >= 33 && Hub.textures.report.rows.length > 0
        },
        {
            section: Theme.sections.downloads,
            what: "Downloads offers the mod the load order needs and nothing installs",
            ready: () => !Hub.downloads.queueBusy && !!Hub.downloads.queue,
            ok: () => (Mods.overview.toGet || []).length >= 1
        },
        {
            section: Theme.sections.downloads,
            what: "a needed mod's problem ignored is gone, and back with the undo",
            ready: () => true,
            ok: () => {
                const count = () => (Mods.overview.toGet || []).length
                const [before, needed] = [count(), Mods.overview.toGet[0]]
                Hub.mods.ignoreProblem(needed.packageId, needed.ignore, true)
                const ignored = count() === before - 1
                Hub.mods.ignoreProblem(needed.packageId, needed.ignore, false)
                return needed.ignore !== "" && ignored && count() === before
            }
        },
        {
            section: Theme.sections.workshop,
            what: "Browse opened with a search lists the mods that match, from the made-up database",
            ready: () => {
                if (!checks.browsed) {
                    checks.browsed = true
                    Hub.workshop.browse("harmony")
                }
                return !Hub.workshop.searching && Hub.workshop.total > 0
            },
            ok: () => Hub.workshop.ready && Hub.workshop.total === 1 && Hub.workshop.problem === ""
        },
        {
            section: Theme.sections.workshop,
            what: "the sidebar counts the matches of the search and lists every game version beside them",
            ready: () => true,
            ok: () => {
                const side = Hub.workshop.sidebar
                return side.counted && side.versions.length === 2 && side.types[0].count === "1" && side.found === "1 result matching your filters" && !side.filtered
            }
        },
        {
            section: Theme.sections.workshop,
            what: "a search of nothing lists every made-up mod, the pages of three loading as the grid is scrolled near its end",
            ready: () => {
                if (!checks.faceted) {
                    checks.faceted = true
                    Hub.workshop.search("")
                }
                checks.scrollToEnd()
                return !Hub.workshop.searching && Hub.workshop.total === 7 && !Hub.workshop.more
            },
            ok: () => checks.ids().length === 7
        },
        {
            section: Theme.sections.workshop,
            what: "ticking Mod and then crossing out 1.5 leaves the two mods for 1.6 only",
            ready: () => {
                if (!checks.crossed) {
                    checks.crossed = true
                    Hub.workshop.toggleTag("Mod", true)
                    Hub.workshop.toggleTag("1.5", false)
                }
                return !Hub.workshop.searching && Hub.workshop.total === 2
            },
            ok: () => {
                const side = Hub.workshop.sidebar
                const old = side.versions[0]
                return side.chips.length === 2 && side.types[0].include && old.id === "1.5" && old.exclude && old.count === "3" && side.found === "2 results matching your filters"
            }
        },
        {
            section: Theme.sections.workshop,
            what: "clearing the filters brings back every result",
            ready: () => {
                if (!checks.cleared) {
                    checks.cleared = true
                    Hub.workshop.clearAll()
                }
                return !Hub.workshop.searching && Hub.workshop.total === 7
            },
            ok: () => Hub.workshop.sidebar.chips.length === 0
        },
        {
            section: Theme.sections.workshop,
            what: "choosing Most subscribed reorders every result by the sweep's subscriber counts",
            ready: () => {
                if (!checks.swept) {
                    checks.swept = true
                    checks.firstIds = checks.ids()
                    Hub.workshop.chooseSort("subscribed")
                }
                return !Hub.workshop.searching && Hub.workshop.sort === "subscribed" && Hub.workshop.total === 7
            },
            ok: () => {
                const reordered = checks.ids().join() !== checks.firstIds.join() && Hub.workshop.sortLabel !== ""
                Hub.workshop.chooseSort("recent")
                return reordered
            }
        },
        {
            section: Theme.sections.workshop,
            what: "a card selected shows its details, with the requirement the database does not know",
            ready: () => {
                if (!checks.picked && !Hub.workshop.searching) {
                    checks.picked = true
                    checks.placed = checks.cardPlace("3100000002")
                    Hub.workshop.select("3100000002")
                }
                return checks.picked && !Hub.workshop.detailsBusy && !!Hub.workshop.details
            },
            ok: () => Hub.workshop.details.id === "3100000002" && Hub.workshop.details.requires.length === 2 && Hub.workshop.details.unknownRequirements.length === 1
        },
        {
            section: Theme.sections.workshop,
            what: "and the grid keeps its columns and the card its place",
            ready: () => true,
            ok: () => checks.placed !== "" && checks.cardPlace("3100000002") === checks.placed
        },
        {
            section: Theme.sections.workshop,
            what: "Add with what it requires fills the download list beside the results",
            ready: () => {
                if (!checks.added) {
                    checks.added = true
                    Hub.workshop.add("3100000002", true)
                }
                return Hub.workshop.list.count > 0
            },
            ok: () => {
                Hub.workshop.select("")
                return Hub.workshop.list.count === 3 && Hub.workshop.list.rows.every(row => row.reason !== "")
            }
        },
        {
            section: Theme.sections.settings,
            what: "Settings keeps a made-up Steam Web API key once Steam accepts it, and shows it kept",
            ready: () => {
                if (!checks.keyTyped) {
                    checks.keyTyped = true
                    Hub.settings.checkKey("made-up-key")
                }
                return !!Hub.settings.steamKey && Hub.settings.steamKey.stored && !Hub.settings.steamKey.checking
            },
            ok: () => Hub.settings.steamKey.available && !Hub.settings.steamKey.refused && !Hub.settings.steamKey.note
        },
        {
            section: Theme.sections.workshop,
            what: "Browse in key mode offers Steam's sorts with one chosen, and the game versions",
            ready: () => {
                if (!checks.keyBrowsed) {
                    checks.keyBrowsed = true
                    Hub.workshop.browse("")
                }
                return !Hub.workshop.searching && Hub.workshop.keyMode && Hub.workshop.total > 1
            },
            ok: () => {
                const sorts = checks.listed(Hub.workshop.sorts)
                checks.firstIds = checks.ids()
                return sorts.length === 6 && sorts.filter(sort => sort.chosen).length === 1 && Hub.workshop.sort === "recent" && !Hub.workshop.sidebar.counted
            }
        },
        {
            section: Theme.sections.workshop,
            what: "choosing Most subscribed puts the results in another order and marks that sort",
            ready: () => {
                if (!checks.keySorted) {
                    checks.keySorted = true
                    Hub.workshop.chooseSort("subscribed")
                }
                return !Hub.workshop.searching && Hub.workshop.total > 1
            },
            ok: () => checks.ids().join() !== checks.firstIds.join() && checks.listed(Hub.workshop.sorts).filter(sort => sort.chosen).map(sort => sort.id).join() === "subscribed"
        },
        {
            section: Theme.sections.workshop,
            what: "the Collections choice shows the collection and nothing else",
            ready: () => {
                if (!checks.keyCollected) {
                    checks.keyCollected = true
                    Hub.workshop.showCollections(true)
                }
                return !Hub.workshop.searching && Hub.workshop.collections
            },
            ok: () => checks.ids().length === 1 && Hub.workshop.rows.data(Hub.workshop.rows.index(0, 0), 258) === "Fixture Collection"
        },
        {
            section: Theme.sections.workshop,
            what: "details in key mode show the votes and a row of screenshots",
            ready: () => {
                if (!checks.keyDetailed) {
                    checks.keyDetailed = true
                    Hub.workshop.showCollections(false)
                    Hub.workshop.select("3100000002")
                }
                return !Hub.workshop.searching && !Hub.workshop.detailsBusy && !!Hub.workshop.details && Hub.workshop.details.screenshots.length > 0
            },
            ok: () => {
                const shown = Hub.workshop.details
                Hub.workshop.select("")
                return shown.id === "3100000002" && !!shown.votesSaid && shown.screenshots.length === 3
            }
        },
        {
            section: Theme.sections.settings,
            what: "Forget the key takes it away again",
            ready: () => {
                if (!checks.keyForgotten) {
                    checks.keyForgotten = true
                    Hub.settings.forgetKey()
                }
                return !Hub.settings.steamKey.stored && !Hub.settings.steamKey.checking
            },
            ok: () => Hub.settings.steamKey.available
        },
        {
            section: Theme.sections.settings,
            what: "Settings shows the folders in use, worked out in Rust",
            ready: () => true,
            ok: () => Hub.settings.report.game.endsWith("game") && Hub.settings.report.localMods.endsWith("Mods") && Hub.settings.report.gameLog.endsWith("Player.log")
        },
        {
            section: Theme.sections.settings,
            what: "a folder typed in Settings is judged after a pause, and a missing one is refused and not kept",
            ready: () => {
                if (!checks.typed) {
                    checks.typed = true
                    Hub.settings.checkFolder("config", "/no/such/folder")
                }
                return !!(Hub.settings.folderChecks || {}).config
            },
            ok: () => !Hub.settings.folderChecks.config.ok && !Hub.settings.report.setConfig
        },
        {
            section: Theme.sections.settings,
            what: "the text size follows a drag one step at a time without keeping the window busy",
            ready: () => {
                if (checks.dragged < checks.sizes.length) {
                    Theme.stored.textScale = checks.sizes[checks.dragged]
                    checks.dragged += 1
                    checks.waited = 0
                }
                return checks.dragged === checks.sizes.length
            },
            ok: () => {
                Theme.stored.textScale = 1
                return true
            }
        },
        {
            section: Theme.sections.settings,
            what: "a theme chosen paints the open window without a restart",
            ready: () => checks.flipped(() => Theme.stored.theme = "light"),
            ok: () => {
                const shown = Qt.colorEqual(checks.window.color, "#f5f5f7")
                Theme.stored.theme = "rim"
                return shown && Qt.colorEqual(checks.window.color, "#1c1916")
            }
        },
        {
            section: Theme.sections.settings,
            what: "high contrast strengthens grey text in the open window",
            ready: () => checks.flipped(() => {
                checks.before = String(Theme.ink3)
                Theme.stored.highContrast = true
            }),
            ok: () => {
                const shown = String(Theme.ink3) !== checks.before
                Theme.stored.highContrast = false
                return shown && String(Theme.ink3) === checks.before
            }
        },
        {
            section: Theme.sections.settings,
            what: "reduced motion stops animations at once",
            ready: () => checks.flipped(() => Theme.stored.reduceMotion = true),
            ok: () => {
                const shown = Theme.quick === 0 && Theme.calm === 0
                Theme.stored.reduceMotion = false
                return shown && Theme.quick > 0
            }
        },
        {
            section: Theme.sections.settings,
            what: "taller mod rows apply at once",
            ready: () => checks.flipped(() => {
                checks.before = String(Theme.row)
                Theme.stored.largeRows = !Theme.stored.largeRows
            }),
            ok: () => {
                const shown = String(Theme.row) !== checks.before
                Theme.stored.largeRows = !Theme.stored.largeRows
                return shown && String(Theme.row) === checks.before
            }
        },
        {
            section: Theme.sections.settings,
            what: "a font chosen reaches every text already shown",
            ready: () => checks.flipped(() => {
                checks.before = Qt.application.font.family
                checks.window.useFont("RimStack Check Sans")
            }),
            ok: () => {
                const left = checks.fonted(checks.window.contentItem, checks.before)
                const moved = checks.fonted(checks.window.contentItem, "RimStack Check Sans")
                checks.window.useFont("")
                return left === 0 && moved > 0 && checks.fonted(checks.window.contentItem, "RimStack Check Sans") === 0
            }
        },
        {
            section: Theme.sections.find,
            what: "Find lists the matches of a word in the mods' files, grouped by mod and file",
            ready: () => {
                if (!checks.searched) {
                    checks.searched = true
                    Hub.find.text = "Fixture"
                    Hub.find.search()
                }
                return !Hub.find.running && Hub.find.summary !== ""
            },
            ok: () => Hub.find.count >= 2
        },
        {
            section: Theme.sections.find,
            what: "a hit chosen shows its file around it, the line marked",
            ready: () => {
                if (!checks.hitPicked) {
                    checks.hitPicked = true
                    Hub.find.select(2)
                }
                return !!Hub.find.preview
            },
            ok: () => Hub.find.preview.lines.length > 1 && Hub.find.preview.lines.some(line => line.marked && line.number === Hub.find.preview.line) && !Hub.find.preview.problem
        },
        {
            section: Theme.sections.find,
            what: "ticking Match case searches again at once, without Search pressed",
            ready: () => {
                if (!checks.recased) {
                    checks.recased = true
                    Hub.find.text = "fixture"
                    Hub.find.search()
                }
                if (checks.uncased === "" && !Hub.find.running && Hub.find.summary !== "") {
                    checks.uncased = Hub.find.summary
                    Hub.find.flip("case")
                }
                return checks.uncased !== "" && !Hub.find.running && Hub.find.summary !== "" && Hub.find.summary !== checks.uncased
            },
            ok: () => {
                const changed = Hub.find.summary !== checks.uncased && Hub.find.options.switches[0].chosen
                checks.uncased = ""
                Hub.find.text = "Fixture"
                Hub.find.flip("case")
                return changed
            }
        },
        {
            section: Theme.sections.find,
            what: "a bad regular expression is said at once, before any search",
            ready: () => checks.flipped(() => {
                Hub.find.text = "(Fixture"
                Hub.find.flip("regex")
            }),
            ok: () => {
                const said = Hub.find.problem !== "" && !Hub.find.running
                Hub.find.text = "Fixture"
                Hub.find.flip("regex")
                return said
            }
        },
        {
            section: Theme.sections.find,
            what: "Stop ends a search of every installed mod",
            ready: () => {
                if (!checks.stopped) {
                    checks.stopped = true
                    Hub.find.choose("installed")
                    Hub.find.fileTypes = "."
                    Hub.find.text = "e"
                    Hub.find.search()
                    Hub.find.stop()
                }
                return !Hub.find.running
            },
            ok: () => !Hub.find.running && Hub.find.summary !== "" && Hub.find.problem === ""
        },
        {
            section: Theme.sections.find,
            what: "Show these mods in the list leaves Mods showing the mods with matches",
            ready: () => {
                if (!checks.shown) {
                    checks.shown = true
                    Hub.find.choose("active")
                    Hub.find.fileTypes = "xml"
                    Hub.find.text = "Fixture"
                    Hub.find.search()
                }
                return !Hub.find.running && Hub.find.count > 0
            },
            ok: () => {
                Mods.filterBy(Hub.find.showInMods())
                Mods.showSection(Theme.sections.mods)
                return Hub.find.showInMods() === "in:search"
            }
        },
        {
            section: Theme.sections.mods,
            what: "and Mods keeps exactly those mods",
            ready: () => Mods.filtered === "in:search",
            ok: () => {
                const kept = Mods.active.matching + Mods.inactive.matching
                Mods.filterBy("")
                return kept === Hub.find.count
            }
        }
    ]
    property bool searched: false
    property bool hitPicked: false
    property bool recased: false
    property string uncased
    property bool stopped: false
    property bool shown: false
    property bool typed: false
    property bool browsed: false
    property bool faceted: false
    property bool crossed: false
    property bool swept: false
    property bool cleared: false
    property bool keyTyped: false
    property bool keyBrowsed: false
    property bool keySorted: false
    property bool keyCollected: false
    property bool keyDetailed: false
    property bool keyForgotten: false
    property var firstIds: []
    function ids() {
        const rows = Hub.workshop.rows
        const found = []
        for (let at = 0; at < 100; at++) {
            const id = rows.data(rows.index(at, 0), 257)
            if (!id)
                break
            found.push(id)
        }
        return found
    }
    property bool picked: false
    property string placed
    function cardPlace(id) {
        const cards = named(window.contentItem, "resultGrid")
        const card = cards ? Array.from(cards.contentItem.children).find(child => !!child.model && child.model.id === id) : null
        if (!card)
            return ""
        const at = card.mapToItem(null, 0, 0)
        return cards.across + " across, at " + Math.round(at.x) + "," + Math.round(at.y) + ", " + Math.round(card.width) + " wide"
    }
    property bool added: false
    property bool changed: false
    property string before
    // Changes a setting on one tick and says it is ready on the next, so bindings settle first.
    function flipped(change) {
        if (changed) {
            changed = false
            return true
        }
        change()
        changed = true
        return false
    }
    function fonted(root, family) {
        let found = root.font !== undefined && root.font.family === family ? 1 : 0
        for (const child of root.children || [])
            found += fonted(child, family)
        return found
    }
    property int dragged: 0
    readonly property var sizes: [0.9, 1, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.4, 1.2, 1, 0.85, 1]

    Timer {
        interval: 200
        repeat: true
        running: true
        onTriggered: {
            if (checks.at === checks.steps.length) {
                stop()
                checks.languages()
                return
            }
            const step = checks.steps[checks.at]
            if (checks.window.section !== step.section) {
                checks.window.section = step.section
                checks.waited = 0
                return
            }
            checks.waited += 1
            if (!step.ready() && checks.waited < 50)
                return
            checks.check(step.what, step.ok())
            checks.at += 1
            checks.waited = 0
        }
    }

    function languages() {
        Hub.settings.useLanguage("Français")
        speaking.start()
    }
    Timer {
        id: speaking
        property int waited: 0
        property bool french: true
        interval: 100
        repeat: true
        onTriggered: {
            waited += 1
            const fromRust = Hub.settings.databases.some(d => d.use.indexOf(Theme.say("Names of mods that are not installed")) >= 0)
            const shown = fromRust && (french ? Theme.say("Save") === "Enregistrer" && !Hub.settings.report.english
                                              : Theme.say("Save") === "Save" && Hub.settings.report.english)
            if (!shown && waited < 30)
                return
            checks.check(french ? "a language chosen is used without a restart" : "and English again", shown)
            waited = 0
            if (french) {
                french = false
                Hub.settings.useLanguage("English")
                return
            }
            stop()
            checks.window.section = Theme.sections.mods
            Qt.exit(checks.failed > 0 ? 1 : 0)
        }
    }
}
