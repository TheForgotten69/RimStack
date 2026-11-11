import QtQuick

QtObject {
    // { model, modIndex, modId, ids, name } of the mod held, `ids` all it takes
    // along; null when none is.
    property var held: null
    // { model, modIndex } of the place under the pointer.
    property var target: null
    property point point
    signal dropped(var from, var to)

    function finish() {
        const from = held
        const to = target
        held = null
        target = null
        if (from && to)
            dropped(from, to)
    }
}
