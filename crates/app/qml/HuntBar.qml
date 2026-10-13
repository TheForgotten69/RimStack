// The search for the one mod that breaks the game, round by round: a
// bar under the top bar for as long as it lasts. Each round's list is
// only put on the Mods screen; the game gets it when Save is pressed.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Rectangle {
    id: bar
    required property Backend backend
    // `Backend.huntState`, parsed.
    property var search: ({ active: false })
    signal useList(var ids)
    signal saveAndPlay()

    function show(state) {
        search = JSON.parse(state)
        if (search.active && !search.found && search.stuck.length === 0)
            useList(search.list)
    }
    function start() {
        show(backend.huntStart())
    }
    function answer(broken) {
        show(backend.huntAnswer(broken))
    }
    // Back to the list the search began with, less `without` if given.
    function end(without) {
        const ids = search.original.filter(id => id !== without)
        backend.huntStop()
        search = ({ active: false })
        useList(ids)
    }
    Component.onCompleted: if (!Theme.demo) search = JSON.parse(backend.huntState())

    // A round is under way: neither found nor stuck.
    readonly property bool asking: !!search.active && !search.found && (search.stuck || []).length === 0
    visible: !!search.active
    implicitHeight: visible ? Math.max(44, words.implicitHeight + 18) : 0
    color: Theme.banner
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.gapCard
        anchors.rightMargin: Theme.gapLarge
        spacing: Theme.gap
        Icon { name: "search"; size: 15; color: Theme.moved }
        Text {
            id: words
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            textFormat: Text.StyledText
            color: Theme.ink
            font.pixelSize: Theme.body
            text: !bar.search.active ? ""
                : bar.search.found ? Theme.fill("{} is the mod the game breaks with.", "<b>" + Theme.plain(bar.search.found) + "</b>")
                : (bar.search.stuck || []).length > 0 ? Theme.fill("One of these, which need each other: {}", "<b>" + Theme.plain(bar.search.stuck.join(", ")) + "</b>")
                : "<b>" + Theme.say("Finding the broken mod.") + "</b> " + Theme.plural(bar.search.rounds, "About {} round left, {} suspects.", "About {} rounds left, {} suspects.", bar.search.suspects)
                    + " " + Theme.say("Save, play, then say how it went.")
        }
        Icon {
            visible: bar.asking
            name: "info"
            size: Theme.icon
            color: Theme.ink2
            HoverHandler { id: how }
            Tip { over: how.hovered; text: Theme.say("Half of the suspects are switched off in the list now. Do not load or save a colony during a round; the main menu or a new test colony shows whether it breaks.") }
        }
        Btn { visible: bar.asking; primary: Mods.dirty; icon: "play"; text: Theme.say("Save and play"); tip: Theme.say("Writes this round's list to ModsConfig.xml and starts the game. The newest save was copied to RimStack's data folder, save-before-search, when the search began."); onClicked: bar.saveAndPlay() }
        Btn { visible: bar.asking; text: Theme.say("Still broken"); onClicked: bar.answer(true) }
        Btn { visible: bar.asking; text: Theme.say("Works now"); onClicked: bar.answer(false) }
        Btn { visible: !!bar.search.found; primary: true; text: Theme.say("My list without it"); tip: Theme.say("The list the search began with, less this mod. Nothing is written before Save."); onClicked: bar.end(bar.search.foundId) }
        Btn { text: bar.search.found ? Theme.say("My list as it was") : Theme.say("Stop"); tip: Theme.say("The list the search began with. Nothing is written before Save."); onClicked: bar.end("") }
    }
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.edge }
}
