pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Item {
    id: checks
    required property var window
    property int failed: 0
    property int at: 0
    property int waited: 0

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
            section: 3,
            what: "Saves lists both saves, with the mods each was saved with",
            ready: () => !Hub.saves.busy && (Hub.saves.saves || []).length > 0,
            ok: () => checks.listed(Hub.saves.saves).length === 2 && checks.listed(Hub.saves.saves).every(save => save.ids.length >= 43)
        },
        {
            section: 3,
            what: "and says the save with a mod that is not installed breaks the list",
            ready: () => true,
            ok: () => checks.listed(Hub.saves.saves).filter(save => save.verdict.breaks).length === 1
        },
        {
            section: 4,
            what: "Logs reads the game's log and groups its errors",
            ready: () => !Hub.logs.busy && !!Hub.logs.head,
            ok: () => Hub.logs.head.errors >= 2 && Hub.logs.errors.length === 2
        },
        {
            section: 4,
            what: "and shows its lines",
            ready: () => (Hub.logs.lines || []).length > 0,
            ok: () => checks.listed(Hub.logs.lines).some(line => line.errorStarts)
        },
        {
            section: 2,
            what: "Textures counts the textures of the mods",
            ready: () => !Hub.textures.busy && !!Hub.textures.report && Hub.textures.report.all > 0,
            ok: () => Hub.textures.report.all >= 33 && Hub.textures.report.rows.length > 0
        },
        {
            section: 1,
            what: "Downloads offers the mod the load order needs and nothing installs",
            ready: () => !Hub.downloads.queueBusy && !!Hub.downloads.queue,
            ok: () => (Mods.overview.toGet || []).length >= 1
        },
        {
            section: 5,
            what: "Settings shows the folders in use, worked out in Rust",
            ready: () => true,
            ok: () => Hub.settings.report.game.endsWith("game") && Hub.settings.report.localMods.endsWith("Mods") && Hub.settings.report.gameLog.endsWith("Player.log")
        }
    ]

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
            const shown = french ? Theme.say("Save") === "Enregistrer" && !Hub.settings.report.english
                                 : Theme.say("Save") === "Save" && Hub.settings.report.english
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
            checks.window.section = 0
            Qt.exit(checks.failed > 0 ? 1 : 0)
        }
    }
}
