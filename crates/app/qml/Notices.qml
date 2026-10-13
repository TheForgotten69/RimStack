// What the app has to say for a moment: a notice bottom right, with
// Undo where the thing just done can be taken back. Also the line the
// status bar shows about whatever the pointer is on.
pragma Singleton
import QtQuick
import RimStack.Ui

QtObject {
    id: notices

    property Backend backend
    // Each is { key, text, tone: "", "ok" or "err", label, run }.
    property var shown: []
    // This run's notices, newest first: { text, tone, time }.
    property var earlier: []
    property int counted: 0
    // Goes up when the journal on disk gets a line.
    property int journalled: 0
    property string hint

    function add(notice) {
        notice.key = ++counted
        // One offer to undo at a time: the newest stands for the last thing done.
        const kept = notice.run ? shown.filter(other => !other.run) : shown
        shown = kept.concat([notice]).slice(-4)
        earlier = [{ text: notice.text, tone: notice.tone || "", time: Date.now() }].concat(earlier).slice(0, 40)
    }
    function say(text, tone) {
        add({ text: text, tone: tone || "" })
    }
    // Done already; `run` takes it back.
    function offer(text, label, run) {
        add({ text: text, tone: "", label: label, run: run })
    }
    // Before something touches files outside RimStack: what will happen
    // and in which folder, to confirm. { title, lines, label, danger, run }.
    property var asking: null
    function ask(question) {
        asking = question
    }

    function dismiss(key) {
        shown = shown.filter(notice => notice.key !== key)
    }
    // A line for the history that outlives this run; `revert` as
    // `Backend.noteChange` describes it.
    // `sentence` as `Theme.put` reads it: kept in English with its parts.
    function keep(sentence, revert) {
        record(JSON.stringify(sentence), revert)
    }
    function record(what, revert) {
        if (!backend || Theme.demo)
            return
        backend.noteChange(what, revert || "")
        journalled++
    }
}
