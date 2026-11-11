import QtQuick
import RimStack.Ui

QtObject {
    readonly property var doers: ({
        moveSelected: () => Mods.moveSelected(),
        moveAcross: e => Hub.mods.take(e.active, e.index, -1),
        openFolder: e => Qt.openUrlExternally(Theme.fileUrl(e.path)),
        openUrl: e => Qt.openUrlExternally(e.url),
        copyText: e => Theme.copy(e.value),
        applyFix: e => Mods.applyFix(e.modId, e.fix),
        subscribe: e => Hub.downloads.subscribe(e.workshopId),
        copyToLocal: e => Notices.ask(Object.assign({ run: () => Mods.copyToLocal(e.ids) }, e.ask)),
        redownload: e => Notices.ask(Object.assign({ run: () => Hub.downloads.redownload(e.modId) }, e.ask)),
        unsubscribe: e => Mods.unsubscribe(e.modId),
        updateFromGit: e => Hub.downloads.updateFromGit(e.modId),
        putInSub: e => Mods.putInSub(e.modId, e.tag),
        newSub: e => Mods.subNameRequested(e.modId),
        trust: e => Hub.mods.trust(e.ids, e.yes),
        bin: e => Mods.binRequested(e.name, e.path, e.modId),
        toggleSub: e => Hub.mods.toggleSub(e.active, e.row),
        removeSub: e => Hub.mods.removeSub(e.active, e.row),
    })

    function open(row, x, y) {
        const inActive = row.ownModel === Mods.active
        if (!(row.current))
            Mods.select(row.modId, inActive)
        Mods.menuRequested(row, x, y, Hub.mods.rowMenu(inActive, row.modIndex, row.modId))
    }
}
