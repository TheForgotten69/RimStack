pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Item {
    id: checks
    required property var window
    property double began: 0
    property double first: 0
    property double last: Date.now()
    property double longest: 0
    property double stopped: 0
    property bool picking: false

    function report(note) {
        console.log("find " + note + " mods=" + Hub.find.count + " capped=" + Hub.find.capped + " longest_gap_ms=" + Math.round(checks.longest) + " | " + Hub.find.summary)
    }

    Component.onCompleted: {
        Theme.stored.reduceMotion = true
        checks.window.section = Theme.sections.find
        const text = Hub.argument("--find-text")
        if (text === "") {
            idle.start()
            return
        }
        if (Hub.given("--find-regex"))
            Hub.find.flip("regex")
        Hub.find.choose(Hub.argument("--find-where") || "installed")
        Hub.find.fileTypes = Hub.given("--find-all") ? "." : "xml"
        Hub.find.text = text
        begin.start()
    }

    Timer {
        id: idle
        interval: 4000
        onTriggered: { console.log("find idle"); Qt.exit(0) }
    }
    Timer {
        id: begin
        interval: 1500
        onTriggered: {
            checks.began = Date.now()
            checks.last = checks.began
            Hub.find.search()
            watch.start()
        }
    }
    Timer {
        id: watch
        interval: 10
        repeat: true
        onTriggered: {
            const now = Date.now()
            if (now - checks.last - interval > 30)
                console.log("find gap " + (now - checks.last - interval) + " at " + (now - checks.began) + " mods=" + Hub.find.count)
            checks.longest = Math.max(checks.longest, now - checks.last - interval)
            checks.last = now
            if (checks.first === 0 && Hub.find.count > 0) {
                checks.first = now - checks.began
                console.log("find first_results_ms=" + checks.first)
            }
            const stopAt = Hub.argument("--find-stop")
            if (stopAt !== "" && checks.stopped === 0 && Hub.find.running && now - checks.began >= Number(stopAt)) {
                checks.stopped = now
                Hub.find.stop()
            }
            if (Hub.find.running)
                return
            stop()
            if (checks.stopped > 0)
                console.log("find stop_to_idle_ms=" + (now - checks.stopped))
            console.log("find done_ms=" + (now - checks.began))
            checks.report("end")
            settle.start()
        }
    }
    Timer {
        id: settle
        interval: 1500
        onTriggered: {
            checks.report("settled")
            if (Hub.argument("--picture-to") === "" || checks.picking) {
                Qt.exit(0)
                return
            }
            checks.picking = true
            checks.window.contentItem.children[0].grabToImage(result => {
                result.saveToFile(Hub.argument("--picture-to"))
                Qt.exit(0)
            })
        }
    }
}
