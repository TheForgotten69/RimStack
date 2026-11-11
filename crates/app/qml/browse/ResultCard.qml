pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Rectangle {
    id: card
    required property var model
    readonly property bool current: (Hub.workshop.details || ({})).id === model.id
    component Mark: Rectangle {
        property string icon
        property color tone
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: Theme.gapSmall
        width: Theme.control
        height: width
        radius: width / 2
        color: tone
        Icon { anchors.centerIn: parent; name: parent.icon; color: Theme.accentInk }
    }
    implicitHeight: body.implicitHeight + 2 * Theme.gapMedium
    radius: Theme.cornerLarge
    color: Theme.panel
    border.width: 1
    border.color: Theme.line
    activeFocusOnTab: true
    Keys.onReturnPressed: Hub.workshop.select(card.model.id)
    Keys.onSpacePressed: Hub.workshop.select(card.model.id)
    FocusRing { corner: card.radius }
    Accessible.role: Accessible.Button
    Accessible.name: model.title
    Accessible.description: model.badge
    Accessible.onPressAction: Hub.workshop.select(card.model.id)

    FontMetrics { id: titleFont; font.pixelSize: Theme.body; font.weight: Font.DemiBold }

    Rectangle { anchors.fill: parent; radius: card.radius; color: Theme.hover; opacity: Number(pointed.hovered) * 0.05 }
    Rectangle { visible: card.current; anchors.fill: parent; radius: card.radius; color: "transparent"; border.width: 2; border.color: Theme.focus }
    HoverHandler { id: pointed; cursorShape: Qt.PointingHandCursor }
    TapHandler { onTapped: Hub.workshop.select(card.model.id) }

    Column {
        id: body
        x: Theme.gapMedium
        y: Theme.gapMedium
        width: parent.width - 2 * Theme.gapMedium
        spacing: Theme.gapSmall
        Rectangle {
            width: parent.width
            height: width * 9 / 16
            radius: Theme.corner
            color: Theme.ctl
            clip: true
            Text { anchors.centerIn: parent; color: Theme.ink3; font.pixelSize: Theme.display; font.weight: Font.DemiBold; text: card.model.title.charAt(0).toUpperCase() }
            Loader {
                anchors.fill: parent
                active: card.model.preview !== ""
                sourceComponent: Image {
                    asynchronous: true
                    fillMode: Image.PreserveAspectCrop
                    sourceSize.width: width
                    source: Theme.fileUrl(card.model.preview)
                }
            }
            Mark { visible: card.model.installed; tone: Theme.ok; icon: "check" }
            Mark { visible: card.model.onList && !card.model.installed; tone: Theme.moved; icon: "list" }
        }
        Text { width: parent.width; height: titleFont.lineSpacing; elide: Text.ElideRight; color: Theme.ink; font: titleFont.font; text: card.model.title }
        Text { opacity: Number(card.model.author !== ""); width: parent.width; height: titleFont.lineSpacing; elide: Text.ElideRight; color: Theme.ink3; font.pixelSize: Theme.small; text: Theme.fill("by {}", card.model.author) }
        Stars { visible: card.model.stars.length > 0; filled: card.model.stars }
        Item {
            width: parent.width
            height: Theme.control
            Btn { visible: card.model.canAdd; icon: "plus"; text: Theme.say("Add"); onClicked: Hub.workshop.add(card.model.id, false) }
        }
    }
}
