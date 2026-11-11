pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui
import "find"

Item {
    id: screen
    Appear { of: screen }

    readonly property bool searched: Hub.find.running || Hub.find.summary !== ""
    readonly property bool resulting: results.count > 0 || resultFilter.text !== ""

    Binding { target: Hub.find; property: "selectedMod"; value: Mods.selectedId }
    Binding { target: searchBox; property: "text"; value: Hub.find.text }
    Shortcut { sequence: "Ctrl+Shift+F"; enabled: screen.visible; onActivated: searchBox.focusInput() }
    TextMetrics { id: digits; font.family: "monospace"; font.pixelSize: Theme.small; text: "00000" }

    readonly property real previewWidth: Math.min(width * 0.36, Theme.sized(620))
    readonly property real lineWidth: digits.advanceWidth * 16
    readonly property bool wide: width - previewWidth - Theme.sized(250) >= lineWidth + 4 * Theme.gapLarge
    readonly property bool sideShown: wide || Mods.findFilters
    readonly property real sideWidth: Theme.sized(250) * Number(wide)

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.gapSmall

        Text {
            Layout.leftMargin: Theme.gapCard
            Layout.preferredHeight: Theme.header
            verticalAlignment: Text.AlignVCenter
            color: Theme.ink
            font.pixelSize: Theme.heading
            font.weight: Font.DemiBold
            text: Theme.say("Search")
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            spacing: Theme.gap
            Btn { visible: !screen.wide; primary: Mods.findFilters; icon: "funnel"; text: Theme.say("Filters"); onClicked: Mods.findFilters = !Mods.findFilters }
            FilterField {
                id: searchBox
                Layout.fillWidth: true
                Layout.preferredWidth: 0
                icon: "search"
                keys: Theme.say("Ctrl + Shift + F")
                placeholder: Theme.say("Search the text of mods' files")
                wanted: screen.visible
                onTextChanged: Hub.find.text = text
                onAccepted: Hub.find.search()
            }
            Btn { visible: !Hub.find.running; primary: true; icon: "search"; text: Theme.say("Search"); enabled: searchBox.text.trim() !== ""; onClicked: Hub.find.search() }
            Btn { visible: Hub.find.running; icon: "pause"; text: Theme.say("Stop"); onClicked: Hub.find.stop() }
            Btn {
                id: recentButton
                visible: (Hub.find.recent || []).length > 0
                icon: "history"
                menu: true
                text: Theme.say("Recent")
                tip: Theme.say("The searches made before")
                onClicked: recent.open()
                FindRecent { id: recent }
            }
        }
        Text {
            visible: Hub.find.problem !== ""
            Layout.fillWidth: true
            Layout.leftMargin: Theme.gapLarge
            Layout.rightMargin: Theme.gapLarge
            wrapMode: Text.WordWrap
            color: Theme.err
            font.pixelSize: Theme.small
            text: Hub.find.problem
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Theme.gapLarge
            spacing: Theme.gapLarge

            FindSidebar {
                visible: screen.sideShown
                Layout.fillHeight: true
                Layout.fillWidth: !screen.wide
                Layout.preferredWidth: screen.sideWidth
            }

            ColumnLayout {
                visible: screen.wide || !Mods.findFilters
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 0
                Layout.preferredWidth: 0
                spacing: Theme.gapSmall

                RowLayout {
                    visible: screen.searched
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.gapLarge
                    Layout.rightMargin: Theme.gapLarge
                    spacing: Theme.gap
                    Text {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 0
                        elide: Text.ElideRight
                        color: Theme.ink2
                        font.pixelSize: Theme.small
                        font.features: { "tnum": 1 }
                        text: Hub.find.progress + Hub.find.summary
                    }
                    Chip { visible: Hub.find.capped; tone: Theme.warn; text: Theme.say("Results capped"); tip: Theme.say("Only the first matches are listed; narrow the search to see the rest") }
                }
                Rectangle {
                    visible: Hub.find.running
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.gapLarge
                    Layout.rightMargin: Theme.gapLarge
                    implicitHeight: Theme.gapTiny
                    radius: height / 2
                    color: Theme.ctl
                    Rectangle {
                        width: parent.width * Hub.find.done / Math.max(1, Hub.find.total)
                        height: parent.height
                        radius: parent.radius
                        color: Theme.accent
                    }
                }
                Flow {
                    visible: screen.resulting
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.gapLarge
                    Layout.rightMargin: Theme.gapLarge
                    spacing: Theme.gap
                    FilterField {
                        id: resultFilter
                        width: Math.min(parent.width, Theme.sized(260))
                        icon: "funnel"
                        placeholder: Theme.say("Filter these results")
                        onTextChanged: Hub.find.filterResults(text)
                    }
                    Btn {
                        visible: Hub.find.count > 0
                        icon: "list"
                        text: Theme.plural(Hub.find.count, "Show this mod in the list", "Show these {} mods in the list")
                        tip: Theme.say("Goes to Mods and keeps only the mods with matches (in:search)")
                        onClicked: {
                            Mods.filterBy(Hub.find.showInMods())
                            Mods.showSection(Theme.sections.mods)
                        }
                    }
                }

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    ListView {
                        id: results
                        objectName: "findResults"
                        anchors.fill: parent
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        currentIndex: -1
                        model: Hub.find
                        FindKeys {
                            parent: results
                            view: results
                            title: Theme.say("Search")
                        }
                        Wheel {}
                        ScrollBar.vertical: Scroller {}
                        delegate: FindRow {
                            width: results.width - Theme.sized(12)
                            numberWidth: digits.advanceWidth
                        }
                    }
                    EmptyState {
                        visible: !screen.searched
                        anchors.centerIn: parent
                        width: Math.min(parent.width - 2 * Theme.gapPage, Theme.sized(420))
                        icon: "search"
                        title: Theme.say("Search the files of your mods")
                        hint: Theme.say("Finds a word or a pattern in the XML of the mods, grouped by mod and file. Ctrl + Shift + F comes here from anywhere.")
                    }
                    EmptyState {
                        visible: screen.searched && !Hub.find.running && results.count === 0
                        anchors.centerIn: parent
                        width: Math.min(parent.width - 2 * Theme.gapPage, Theme.sized(420))
                        icon: "search"
                        title: Theme.say("No matches")
                        hint: Theme.say("Nothing in these mods' files matches. Try other words, or look in more places.")
                    }
                }
            }

            FindPreview {
                visible: !!Hub.find.preview && (screen.wide || !Mods.findFilters)
                Layout.fillHeight: true
                Layout.preferredWidth: screen.previewWidth
                shown: Hub.find.preview || ({})
                row: Hub.find.current
            }
        }
    }
}
