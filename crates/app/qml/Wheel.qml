// Inside a list or any other scrolling view: the mouse wheel moves it
// the same distance for every notch, as the system's own lists do, and
// eases to where it is going.
//
// A wheel's turn is read from its angle alone (120 to a notch): some
// wheels send a notch in several small parts, and those add up to the
// same distance. A touchpad says how far the fingers moved, and the view
// moves that far.
//
// Touchpads are accepted too because that is the only way to be given a
// wheel's events on Wayland: every event there carries a distance in
// pixels, and a handler that accepts mice alone is passed none of them.
// The two are told apart by the phase only a touchpad's gesture has.
pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

WheelHandler {
    id: wheel
    // Pixels per notch at rest: the system's lines per notch, at 40 px a
    // line, times the speed chosen in Settings.
    readonly property real step: Qt.styleHints.wheelScrollLines * 40
    // A handler declared in a scrolling view belongs to its content.
    readonly property Flickable view: (parent instanceof Flickable ? parent : parent?.parent) as Flickable
    property real goal: 0

    target: null
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
    onWheel: (event) => {
        if (!view)
            return
        const first = view.originY - view.topMargin
        const end = Math.max(first, view.originY + view.contentHeight + view.bottomMargin - view.height)
        const within = (y) => Math.max(first, Math.min(end, y))
        if (Theme.wheelLog)
            console.log("wheel angle=" + event.angleDelta.y + " pixel=" + event.pixelDelta.y + " phase=" + event.phase)
        if (event.phase !== Qt.NoScrollPhase) {
            ease.stop()
            view.contentY = within(view.contentY - event.pixelDelta.y)
            return
        }
        if (event.angleDelta.y === 0)
            return
        // Carry on from where it was going, unless something else moved it.
        const from = ease.running ? goal : view.contentY
        goal = within(from - event.angleDelta.y / 120 * step)
        ease.from = view.contentY
        ease.to = goal
        ease.restart()
    }

    property NumberAnimation ease: NumberAnimation {
        target: wheel.view
        property: "contentY"
        duration: Theme.stored.reduceMotion ? 0 : 200
        easing.type: Easing.OutCubic
    }
}
