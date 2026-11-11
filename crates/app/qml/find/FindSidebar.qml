pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Rectangle {
    id: side
    readonly property var all: Hub.find.options || ({})
    readonly property var listed: Hub.find.mods || ({})
    readonly property var types: all.types || ({})
    radius: Theme.cornerLarge
    color: Theme.panel
    border.width: 1
    border.color: Theme.line

    component Heading: Text {
        width: parent.width
        topPadding: Theme.gap
        color: Theme.ink3
        font.pixelSize: Theme.caption
        font.weight: Font.DemiBold
        font.capitalization: Font.AllUppercase
        font.letterSpacing: 1
        elide: Text.ElideRight
    }
    component Checks: Repeater {
        delegate: FindPick {
            required property var modelData
            width: parent.width
            text: modelData.label
            chosen: modelData.chosen
            onClicked: Hub.find.flip(modelData.id)
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.gapCard
        spacing: Theme.gapSmall

        Scrolled {
            id: options
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(options.contentHeight, side.height * 0.6)
            Heading { text: Theme.say("Where") }
            Repeater {
                model: side.all.where
                FindPick {
                    required property var modelData
                    width: parent.width
                    radio: true
                    text: modelData.label
                    chosen: modelData.chosen
                    enabled: modelData.available
                    onClicked: Hub.find.choose(modelData.id)
                }
            }
            Heading { text: Theme.say("Match") }
            Checks { model: side.all.switches }
            Text { width: parent.width; topPadding: Theme.gapSmall; bottomPadding: Theme.gapTiny; color: Theme.ink2; font.pixelSize: Theme.small; text: Theme.say("File types") }
            FilterField {
                id: typesBox
                width: parent.width
                placeholder: Theme.say("xml, cs; json  or  .")
                onTextChanged: { Hub.find.fileTypes = text; offers.shut = false }
                holdEscape: offers.visible
                onAccepted: text = Hub.find.completeType(offers.pick)
                Keys.enabled: offers.visible
                Keys.onDownPressed: offers.at = Hub.find.stepType(offers.at, 1)
                Keys.onUpPressed: offers.at = Hub.find.stepType(offers.at, -1)
                Keys.onTabPressed: text = Hub.find.completeType(offers.at)
                onTapped: offers.shut = false
                onEditingChanged: offers.shut = false
                onEscaped: offers.shut = true
                Popup {
                    id: offers
                    property bool shut: false
                    property int at: 0
                    readonly property var offered: Hub.find.typeSuggestions || []
                    readonly property int pick: visible * (at + 1) - 1
                    visible: typesBox.editing && !shut
                    onOfferedChanged: at = 0
                    onClosed: shut = true
                    closePolicy: Popup.CloseOnPressOutsideParent
                    y: typesBox.height + Theme.gapTiny
                    width: typesBox.width
                    padding: Theme.gapTiny
                    background: Rectangle { radius: Theme.cornerLarge; color: Theme.bar; border.width: 1; border.color: Theme.line }
                    contentItem: ListView {
                        implicitHeight: Math.max(Math.min(contentHeight, 6 * Theme.strip), none.implicitHeight)
                        Text {
                            id: none
                            visible: offers.offered.length === 0
                            width: parent.width
                            padding: Theme.gap
                            wrapMode: Text.WordWrap
                            color: Theme.ink3
                            font.pixelSize: Theme.small
                            text: Theme.say("None of these mods has such files; it is still searched")
                        }
                        clip: true
                        model: offers.offered
                        currentIndex: offers.at
                        highlight: Rectangle { radius: Theme.corner; color: Theme.chip }
                        delegate: Item {
                            id: offer
                            required property var modelData
                            required property int index
                            width: ListView.view.width
                            height: Theme.strip
                            Accessible.role: Accessible.Button
                            Accessible.name: offer.modelData.label
                            Text { id: offerKey; anchors.left: parent.left; anchors.leftMargin: Theme.gap; anchors.verticalCenter: parent.verticalCenter; color: Theme.ink; font.pixelSize: Theme.body; text: offer.modelData.label }
                            Text { anchors.left: offerKey.right; anchors.leftMargin: Theme.gap; anchors.right: parent.right; anchors.rightMargin: Theme.gap; anchors.verticalCenter: parent.verticalCenter; horizontalAlignment: Text.AlignRight; elide: Text.ElideRight; color: Theme.ink2; font.pixelSize: Theme.small; text: offer.modelData.note }
                            HoverHandler { onHoveredChanged: offers.at = offer.index }
                            TapHandler { onTapped: typesBox.text = Hub.find.completeType(offer.index) }
                        }
                    }
                }
            }
            Binding { target: typesBox; property: "text"; value: Hub.find.fileTypes }
            Text {
                visible: !!side.types.problem
                width: parent.width
                topPadding: Theme.gapTiny
                wrapMode: Text.WordWrap
                color: Theme.err
                font.pixelSize: Theme.small
                text: side.types.problem || ""
            }
            Text {
                visible: !side.types.problem
                width: parent.width
                topPadding: Theme.gapTiny
                wrapMode: Text.WordWrap
                color: Theme.ink3
                font.pixelSize: Theme.caption
                text: Theme.say("Separate with commas, semicolons or spaces. A dot is every text file.")
            }
            Heading { text: Theme.say("Leave out folders named") }
            Checks { model: side.all.leave }
        }
        Text {
            Layout.fillWidth: true
            Layout.topMargin: Theme.gapSmall
            elide: Text.ElideRight
            color: Theme.ink3
            font.pixelSize: Theme.caption
            font.weight: Font.DemiBold
            font.capitalization: Font.AllUppercase
            font.letterSpacing: 1
            text: Theme.say("Mods with matches")
        }
        FindPick {
            Layout.fillWidth: true
            Layout.rightMargin: Theme.sized(12)
            radio: true
            text: side.listed.allLabel
            count: side.listed.allCounted
            chosen: side.listed.allChosen
            onClicked: Hub.find.pickMod(-1)
        }
        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: side.listed.rows
            Wheel {}
            ScrollBar.vertical: Scroller {}
            delegate: FindPick {
                required property var modelData
                width: list.width - Theme.sized(12)
                radio: true
                text: modelData.name
                count: modelData.counted
                chosen: modelData.chosen
                onClicked: Hub.find.pickMod(modelData.index)
            }
        }
    }
}
