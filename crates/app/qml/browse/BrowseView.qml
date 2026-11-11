pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Item {
    id: view
    readonly property var related: Hub.workshop.related
    readonly property bool opened: !!Hub.workshop.details
    readonly property bool found: Hub.workshop.ready && Hub.workshop.problem === ""

    component Panel: Rectangle {
        radius: Theme.cornerLarge
        color: Theme.panel
        border.width: 1
        border.color: Theme.line
    }

    readonly property bool wide: width >= Theme.sized(1100)
    readonly property bool sideShown: wide || Mods.browseFilters
    readonly property real sideWidth: Theme.sized(250) * Number(wide)
    readonly property real cardLeast: Theme.sized(369)
    readonly property real columnWidth: Math.min(Theme.sized(360), width * 0.4)
    readonly property bool docked: width - 2 * Theme.gapWide - sideWidth - columnWidth - 2 * Theme.gapLarge >= cardLeast
    readonly property bool covered: !docked && (opened || Mods.browseList) && (wide || !Mods.browseFilters)
    readonly property var facets: Hub.workshop.sidebar

    onOpenedChanged: details.enlarged = ""

    Component.onCompleted: {
        Hub.workshop.open()
    }

    Binding { target: field; property: "text"; value: Hub.workshop.query }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.gapWide
        spacing: Theme.gapLarge

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.gap
            Btn { visible: !view.wide; primary: Mods.browseFilters; icon: "funnel"; text: Theme.say("Filters"); onClicked: Mods.browseFilters = !Mods.browseFilters }
            FilterField {
                id: field
                Layout.fillWidth: true
                Layout.preferredWidth: 0
                implicitHeight: Theme.sized(30)
                icon: "search"
                placeholder: Theme.say("Search, or w: id  t: tag  d: expansion  m: needs a mod  a: author  or a link")
                onAccepted: Hub.workshop.search(field.text)
                Tip { text: Theme.say("Words match titles, authors and package ids. w:<id> one item, t:<tag>, d:<expansion app id>, m:<id> mods that need that mod, a:<author>. A pasted Workshop or collection link works too.") }
            }
            Btn { primary: true; text: Theme.say("Search"); onClicked: Hub.workshop.search(field.text) }
            Btn {
                primary: Mods.browseList || view.opened
                icon: "list"
                text: Theme.fill("Download list ({})", Hub.workshop.list.count)
                tip: Theme.say("Show or hide the download list.")
                onClicked: {
                    Hub.workshop.select("")
                    Mods.browseList = view.opened || !Mods.browseList
                }
            }
        }

        Rectangle {
            visible: view.related.active
            Layout.fillWidth: true
            implicitHeight: bannerRow.implicitHeight + 2 * Theme.gapSmall
            radius: Theme.corner
            color: Theme.banner
            RowLayout {
                id: bannerRow
                anchors.fill: parent
                anchors.margins: Theme.gapSmall
                spacing: Theme.gap
                Text { Layout.fillWidth: true; Layout.preferredWidth: 0; elide: Text.ElideRight; color: Theme.ink; font.pixelSize: Theme.body; text: view.related.said || "" }
                Btn { text: Theme.say("Back to results"); onClicked: Hub.workshop.back() }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.gapLarge

            FilterSidebar {
                visible: view.sideShown
                Layout.fillHeight: true
                Layout.fillWidth: !view.wide
                Layout.preferredWidth: view.sideWidth
            }

            ColumnLayout {
                id: results
                visible: view.wide || !Mods.browseFilters
                opacity: Number(!view.covered)
                enabled: !view.covered
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 0
                spacing: Theme.gap
                Text { visible: Hub.workshop.notice !== ""; Layout.fillWidth: true; Layout.preferredWidth: 0; wrapMode: Text.WordWrap; color: Theme.warn; font.pixelSize: Theme.small; text: Hub.workshop.notice }
                RowLayout {
                    visible: view.found
                    Layout.fillWidth: true
                    spacing: Theme.gap
                    Text { Layout.fillWidth: true; Layout.preferredWidth: 0; elide: Text.ElideRight; color: Theme.ink2; font.pixelSize: Theme.small; text: view.facets.found }
                    Text { visible: Hub.workshop.searching; color: Theme.ink3; font.pixelSize: Theme.small; text: Theme.say("Searching…") }
                    SortMenu {}
                }
                Flow {
                    visible: view.found
                    Layout.fillWidth: true
                    spacing: Theme.gapSmall
                    Repeater {
                        model: view.facets.chips
                        Chip {
                            required property var modelData
                            pressable: true
                            tone: Theme[modelData.tone]
                            icon: "x"
                            text: modelData.label
                            tip: Theme.say("Remove this filter")
                            onClicked: Hub.workshop.clearFilter(modelData.id)
                        }
                    }
                    Chip { visible: view.facets.searched; pressable: true; icon: "x"; text: Theme.say("Clear search"); onClicked: { field.text = ""; Hub.workshop.search("") } }
                    Chip { visible: view.facets.filtered; pressable: true; icon: "x"; text: Theme.say("Clear filters"); onClicked: Hub.workshop.clearAll() }
                    Chip { visible: Hub.workshop.keyMode; pressable: true; onAccent: !Hub.workshop.collections; text: Theme.say("Items"); onClicked: Hub.workshop.showCollections(false) }
                    Chip { visible: Hub.workshop.keyMode; pressable: true; onAccent: Hub.workshop.collections; text: Theme.say("Collections"); onClicked: Hub.workshop.showCollections(true) }
                }
                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    GridView {
                        id: grid
                        objectName: "resultGrid"
                        anchors.fill: parent
                        visible: view.found && Hub.workshop.total > 0
                        model: Hub.workshop.rows
                        clip: true
                        reuseItems: true
                        cacheBuffer: Theme.sized(600)
                        boundsBehavior: Flickable.StopAtBounds
                        readonly property real gap: Theme.gapLarge
                        readonly property real usable: width - bar.width
                        readonly property int across: Math.max(1, Math.floor((usable + gap) / (view.cardLeast + gap)))
                        readonly property real each: (usable + gap) / across - gap
                        cellWidth: each + gap
                        cellHeight: probe.height + gap
                        readonly property bool hungry: contentY + 2 * height >= contentHeight && !Hub.workshop.searching
                        onHungryChanged: Hub.workshop.wantMore(hungry)
                        Wheel {}
                        ScrollBar.vertical: Scroller { id: bar }
                        ResultCard {
                            id: probe
                            visible: false
                            width: grid.each
                            model: ({ id: "", title: "", author: "", preview: "", badge: "", stars: [], installed: false, onList: false, canAdd: false })
                        }
                        delegate: ResultCard { width: grid.each }
                    }
                    Column {
                        visible: !Hub.workshop.ready
                        anchors.centerIn: parent
                        width: Math.min(parent.width, Theme.sized(420))
                        spacing: Theme.gapLarge
                        EmptyState {
                            width: parent.width
                            icon: "download"
                            title: Theme.say("The Workshop database is not downloaded")
                            hint: Theme.say("Browsing searches a list of the Workshop's mods that RimStack keeps on this computer. It comes from the community database, downloaded once and refreshed from Settings.")
                        }
                        Btn {
                            anchors.horizontalCenter: parent.horizontalCenter
                            primary: true
                            enabled: !Hub.settings.communityBusy
                            icon: "download"
                            text: Theme.say("Download the Workshop database")
                            onClicked: Hub.settings.downloadCommunity()
                        }
                        Text { visible: Hub.settings.communityBusy; width: parent.width; horizontalAlignment: Text.AlignHCenter; color: Theme.ink3; font.pixelSize: Theme.small; text: Theme.say("Downloading…") }
                    }
                    Column {
                        visible: Hub.workshop.ready && Hub.workshop.problem !== ""
                        anchors.centerIn: parent
                        width: Math.min(parent.width, Theme.sized(420))
                        spacing: Theme.gapLarge
                        EmptyState { width: parent.width; icon: "circle-alert"; title: Theme.say("The search did not work"); hint: Hub.workshop.problem }
                        Btn { anchors.horizontalCenter: parent.horizontalCenter; text: Theme.say("Try again"); onClicked: Hub.workshop.search(field.text) }
                    }
                    EmptyState {
                        visible: view.found && !Hub.workshop.searching && Hub.workshop.total === 0
                        anchors.centerIn: parent
                        width: Math.min(parent.width, Theme.sized(420))
                        icon: "search"
                        title: Theme.say("No mods match")
                        hint: Theme.say("Try fewer words or clear some filters.")
                    }
                }
            }

            Item {
                id: slot
                Layout.fillHeight: true
                Layout.preferredWidth: view.columnWidth * Number(view.docked)
                Panel {
                    visible: view.docked || view.covered
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    anchors.right: parent.right
                    width: slot.width + Number(!view.docked) * (results.width + Theme.gapLarge)
                    BrowseDetails {
                        id: details
                        visible: view.opened
                        anchors.fill: parent
                        anchors.margins: Theme.gapCard
                    }
                    ColumnLayout {
                        visible: !view.opened
                        anchors.fill: parent
                        anchors.margins: Theme.gapCard
                        spacing: Theme.gap
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.gap
                            Text { color: Theme.ink2; font.pixelSize: Theme.small; font.weight: Font.DemiBold; text: Theme.say("Download list") }
                            Text { Layout.fillWidth: true; Layout.preferredWidth: 0; elide: Text.ElideRight; color: Theme.ink3; font.pixelSize: Theme.small; text: Hub.workshop.list.sizeSaid || "" }
                        }
                        ListPane { Layout.fillWidth: true; Layout.fillHeight: true }
                    }
                }
            }
        }
    }

    Rectangle {
        visible: details.enlarged !== ""
        anchors.fill: parent
        z: 10
        color: Theme.scrim
        Image {
            anchors.fill: parent
            anchors.margins: Theme.gapWide
            asynchronous: true
            fillMode: Image.PreserveAspectFit
            sourceSize.width: Theme.sized(1600)
            source: Theme.fileUrl(details.enlarged)
        }
        TapHandler { onTapped: details.enlarged = "" }
    }
}
