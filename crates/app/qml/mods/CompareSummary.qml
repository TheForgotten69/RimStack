// Beside a comparison of two mods' textures: where the choices stand,
// and choosing all at once.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Column {
    id: cmpSummary
    width: parent.width
    visible: Mods.comparing !== null
    readonly property var cmp: Mods.compareView
    readonly property var names: cmp && cmp.report.earlier ? [cmp.report.earlier.name, cmp.report.later.name] : ["", ""]
    Text { color: Theme.ink; font.pixelSize: Theme.heading; font.weight: Font.DemiBold; text: Theme.say("Shared textures") }
    Text { width: parent.width; topPadding: Theme.gapHair; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.fill("{} and {}", cmpSummary.names[0], cmpSummary.names[1]) }

    GroupTitle { text: Theme.say("So far") }
    Repeater {
        model: cmpSummary.cmp ? [
            [Theme.fill("{} kept", cmpSummary.names[1]), cmpSummary.cmp.all.length - cmpSummary.cmp.keptEarlier],
            [Theme.fill("{} kept", cmpSummary.names[0]), cmpSummary.cmp.keptEarlier],
            [Theme.say("Chosen by hand"), Theme.fill("{} of {}", cmpSummary.cmp.yours, cmpSummary.cmp.all.length)],
        ] : []
        RowLayout {
            id: tally
            required property var modelData
            width: cmpSummary.width
            height: Theme.sized(24)
            Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: tally.modelData[0] }
            Text { color: Theme.ink2; font.pixelSize: Theme.body; text: tally.modelData[1] }
        }
    }

    GroupTitle { text: Theme.say("All at once") }
    Flow {
        width: parent.width
        spacing: Theme.gapSmall
        Btn { text: Theme.fill("Keep all from {}", cmpSummary.names[1]); onClicked: cmpSummary.cmp.keepAll(false) }
        Btn {
            text: Theme.fill("Keep all from {}", cmpSummary.names[0])
            // Keeping the earlier mod's versions renames files in the other mod.
            tip: cmpSummary.cmp ? Theme.plural(cmpSummary.cmp.all.length, "Sets aside {} file in {} by renaming it; Undo names it back.", "Sets aside {} files in {} by renaming them; Undo names them back.", cmpSummary.names[1]) : ""
            onClicked: {
                const cmp = cmpSummary.cmp
                const folder = Mods.about(cmp.later).path || ""
                const aside = ["#", cmp.all.length, Theme.kept("Set aside {} texture of {}"), Theme.kept("Set aside {} textures of {}"), cmpSummary.names[1]]
                const said = Theme.put(aside)
                cmp.keepAll(true)
                Notices.keep(aside, "unhide " + folder)
                Notices.offer(said, Theme.say("Undo"), () => {
                    Mods.textureTools.unhideIn(folder)
                    if (Mods.compareView)
                        Mods.compareView.reload()
                })
            }
        }
    }

    GroupTitle { text: Theme.say("Default") }
    Text {
        width: parent.width
        wrapMode: Text.WordWrap
        color: Theme.ink
        font.pixelSize: Theme.body
        text: Theme.fill("{} wins: it loads later.", cmpSummary.names[1])
        Tip { text: Theme.say("A choice against the load order sets the losing file aside: it is renamed inside its mod, with .rimstack-hidden added, so the game skips it. Nothing is deleted; choosing the other version renames it back. Steam restores renamed files when it updates a mod.") }
    }
}
