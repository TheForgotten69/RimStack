pragma ComponentBehavior: Bound
import QtQuick
import QtTest
import RimStack.Ui

Item {
    id: checks
    required property var window
    property int failed: 0
    property int at: 0
    property int waited: 0
    property bool acted: false
    property int row: -1
    property ListView list: null

    TestEvent { id: events }

    function check(what, ok) {
        console.log("check " + (ok ? "ok   " : "FAIL ") + what)
        if (!ok)
            failed += 1
    }

    function listIn(root) {
        for (const child of root.children || []) {
            if (child.objectName === "findResults")
                return child
            const found = listIn(child)
            if (found)
                return found
        }
        return null
    }

    function fileOf(index) {
        for (let i = index; i >= 0; i--)
            if (list.itemAtIndex(i)?.isFile)
                return list.itemAtIndex(i).file
        return ""
    }

    function matchRow() {
        for (let i = 0; i < 40; i++) {
            const item = list.itemAtIndex(i)
            const next = list.itemAtIndex(Hub.find.step(i, 1))
            if (item?.isMatch && next && !next.isFile && !next.isMod)
                return i
        }
        return -1
    }

    function onShow(index) {
        const item = list.itemAtIndex(index)
        return !!item && item.y >= list.contentY - 1 && item.y + item.height <= list.contentY + list.height + 1
    }

    function press(key, modifier, times) {
        for (let i = 0; i < times; i++)
            events.keyClick(key, modifier, -1)
    }

    function previewing(index) {
        const item = list.itemAtIndex(index)
        const shown = Hub.find.preview
        return !!shown && !!item && Hub.find.current === index && shown.line === item.number
            && shown.path.endsWith(fileOf(index)) && Array.from(shown.lines).some(line => line.marked && line.number === shown.line)
    }

    function folded(wanted) {
        const item = list.itemAtIndex(Hub.find.current)
        return !!item && item.isFile && item.collapsed === wanted
    }

    function searchFor(text) {
        Hub.find.text = text
        Hub.find.search()
    }

    readonly property var steps: [
        {
            what: "a search of the made-up mods lists matches",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    Hub.find.choose("active")
                    Hub.find.fileTypes = "xml"
                    Hub.find.text = "Fixture"
                    Hub.find.search()
                }
                checks.list = checks.listIn(checks.window.contentItem)
                return !Hub.find.running && Hub.find.count > 0 && !!checks.list
            },
            ok: () => {
                checks.row = checks.matchRow()
                return checks.row > 1
            }
        },
        {
            what: "a tap on a matching line chooses it and the preview shows its file around that line",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    const item = checks.list.itemAtIndex(checks.row)
                    events.mouseClick(item, item.width / 2, item.height / 2, Qt.LeftButton, Qt.NoModifier, -1)
                }
                return checks.previewing(checks.row)
            },
            ok: () => true
        },
        {
            what: "Down chooses the next line and the preview follows",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    checks.press(Qt.Key_Down, Qt.NoModifier, 1)
                }
                return checks.previewing(Hub.find.step(checks.row, 1))
            },
            ok: () => Hub.find.current === Hub.find.step(checks.row, 1)
        },
        {
            what: "Up goes back to the line tapped",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    checks.press(Qt.Key_Up, Qt.NoModifier, 1)
                }
                return checks.previewing(checks.row)
            },
            ok: () => true
        },
        {
            what: "Down again and again keeps the line chosen in view",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    checks.press(Qt.Key_Down, Qt.NoModifier, 60)
                    return false
                }
                return checks.waited > 1
            },
            ok: () => Hub.find.current > checks.row + 20 && checks.onShow(Hub.find.current)
        },
        {
            what: "Left folds the file of the line chosen and chooses the file",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    Hub.find.select(checks.row)
                    checks.press(Qt.Key_Left, Qt.NoModifier, 1)
                }
                return checks.folded(true)
            },
            ok: () => Hub.find.current < checks.row && !!Hub.find.preview
        },
        {
            what: "Right opens it and Space folds it again",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    checks.press(Qt.Key_Right, Qt.NoModifier, 1)
                    return false
                }
                return checks.folded(false) && checks.acted
            },
            ok: () => {
                events.keyClick(Qt.Key_Space, Qt.NoModifier, -1)
                return true
            }
        },
        {
            what: "and that left it folded",
            ready: () => checks.folded(true),
            ok: () => true
        },
        {
            what: "every file of the mods is searched, version folders included",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    Hub.find.choose("installed")
                    checks.searchFor("FixtureOldVersion")
                }
                return !Hub.find.running
            },
            ok: () => Hub.find.count === 1
        },
        {
            what: "typed file types choose the files and a bad entry is said",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    Hub.find.fileTypes = "cs; json"
                    checks.searchFor("FixtureProbe")
                }
                return !Hub.find.running
            },
            ok: () => {
                const found = Hub.find.count === 1
                Hub.find.fileTypes = "xml, a/b"
                const said = !!Hub.find.options.types.problem
                Hub.find.fileTypes = "xml"
                return found && said && !Hub.find.options.types.problem
            }
        },
        {
            what: "a mod picked in the list on the side shows only its results and all mods shows them again",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    Hub.find.choose("active")
                    checks.searchFor("Fixture")
                    checks.waited = -20
                }
                return !Hub.find.running && Hub.find.mods.rows.length > 1
            },
            ok: () => {
                const before = checks.list.count
                const rows = Hub.find.mods.rows
                Hub.find.pickMod(rows[1].index)
                const picked = checks.list.count < before && Hub.find.mods.rows[1].chosen && !Hub.find.mods.allChosen
                Hub.find.pickMod(-1)
                return picked && checks.list.count === before && Hub.find.mods.allChosen
            }
        },
        {
            what: "searching inside the file marks the lines, next goes to a match and wrap lines is kept",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    Hub.find.select(checks.matchRow())
                    Hub.find.insideText = "Fixture"
                    Hub.find.wrap = true
                }
                return Hub.find.insideFound > 0 && Array.from(Hub.find.preview?.lines || []).some(line => line.found)
            },
            ok: () => Hub.find.insideSaid !== "" && Hub.find.wrap
        },
        {
            what: "the next match is centred and marked as the one the box is at",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    Hub.find.insideStep(1)
                }
                return Array.from(Hub.find.preview?.lines || []).some(line => line.here)
            },
            ok: () => {
                const here = Array.from(Hub.find.preview.lines).find(line => line.here)
                return here.number === Hub.find.preview.line
            }
        },
        {
            what: "Ctrl+C copies the line chosen",
            ready: () => {
                if (!checks.acted) {
                    checks.acted = true
                    Hub.find.select(checks.row)
                    checks.press(Qt.Key_C, Qt.ControlModifier, 1)
                }
                return checks.waited > 1
            },
            ok: () => {
                const target = Qt.createQmlObject("import QtQuick; TextEdit {}", checks)
                target.paste()
                const same = target.text === Hub.find.copyLine(checks.row) && target.text !== ""
                target.destroy()
                return same
            }
        }
    ]

    Timer {
        interval: 200
        repeat: true
        running: true
        onTriggered: {
            if (checks.at === checks.steps.length) {
                stop()
                console.log("check " + (checks.failed > 0 ? "FAIL" : "ok   ") + " the Find taps and keys")
                Qt.exit(checks.failed > 0 ? 1 : 0)
                return
            }
            if (checks.window.section !== Theme.sections.find) {
                checks.window.section = Theme.sections.find
                return
            }
            const step = checks.steps[checks.at]
            checks.waited += 1
            if (!step.ready() && checks.waited < 25)
                return
            checks.check(step.what, step.ok())
            checks.at += 1
            checks.waited = 0
            checks.acted = false
        }
    }
}
