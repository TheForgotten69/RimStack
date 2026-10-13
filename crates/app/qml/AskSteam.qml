// The button that asks Steam's site for change notes. While it asks it
// says how far it is, and counts down when Steam asked to wait.
pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Btn {
    id: ask
    required property var backend
    property string idle
    readonly property bool busy: !!backend && backend.changeNotesBusy
    readonly property var far: backend && backend.changeNotesState ? JSON.parse(backend.changeNotesState) : ({})
    property int waits: 0
    enabled: !!backend && !busy
    icon: "download"
    text: !busy ? idle
        : waits > 0 ? Theme.fill("Steam asks to wait, {} s", waits)
        : far.asked > 1 ? Theme.fill("Asking Steam, {} of {}", Math.min(far.done + 1, far.asked), far.asked)
        : Theme.say("Asking Steam…")
    Timer {
        interval: 1000
        repeat: true
        triggeredOnStart: true
        running: ask.busy && ask.far.resume > 0
        onTriggered: ask.waits = Math.max(0, ask.far.resume - Math.floor(Date.now() / 1000))
        onRunningChanged: if (!running) ask.waits = 0
    }
}
