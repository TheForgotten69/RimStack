pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Item {
    id: row
    required property int index
    required property bool isMod
    required property bool isFile
    required property string name
    required property string file
    required property int count
    required property int files
    required property bool collapsed
    required property int number
    required property bool isMatch
    required property bool isGap
    required property var parts
    property real numberWidth
    readonly property bool current: index === Hub.find.current
    readonly property bool isLine: !isMod && !isFile
    readonly property real indent: Theme.gapPage + Theme.icon + Theme.gapSmall

    height: stack.implicitHeight

    Column {
        id: stack
        width: row.width
        Loader { width: stack.width; active: row.isMod; sourceComponent: modLine }
        Loader { width: stack.width; active: row.isFile; sourceComponent: fileLine }
        Loader { width: stack.width; active: row.isLine; sourceComponent: lineRow }
    }

    Component {
        id: modLine
        Item {
            implicitHeight: Theme.row + Theme.gapSmall
            Accessible.role: Accessible.ListItem
            Accessible.name: row.name
            Rectangle {
                anchors.fill: parent
                anchors.leftMargin: Theme.gapSmall
                anchors.rightMargin: Theme.gapSmall
                anchors.topMargin: Theme.gapTiny
                radius: Theme.corner
                color: Theme.panel
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.gapLarge
                anchors.rightMargin: Theme.gapLarge
                anchors.topMargin: Theme.gapTiny
                spacing: Theme.gapSmall
                Icon { name: "chevron-down"; rotation: -90 * Number(row.collapsed) }
                Text {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 0
                    elide: Text.ElideRight
                    textFormat: Text.PlainText
                    color: Theme.ink
                    font.pixelSize: Theme.body
                    font.weight: Font.DemiBold
                    text: row.name
                }
                Text {
                    color: Theme.ink2
                    font.pixelSize: Theme.small
                    text: Theme.plural(row.count, "{} match", "{} matches") + ", " + Theme.plural(row.files, "{} file", "{} files")
                }
            }
            HoverHandler { cursorShape: Qt.PointingHandCursor }
            TapHandler { onTapped: Hub.find.toggle(row.index) }
        }
    }

    Component {
        id: fileLine
        Item {
            implicitHeight: Theme.row
            Accessible.role: Accessible.ListItem
            Accessible.name: row.file
            Rectangle {
                anchors.fill: parent
                anchors.leftMargin: Theme.gapSmall
                anchors.rightMargin: Theme.gapSmall
                radius: Theme.corner
                color: Theme.accent
                opacity: 0.22 * Number(row.current)
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.gapWide
                anchors.rightMargin: Theme.gapLarge
                spacing: Theme.gapSmall
                Item {
                    implicitWidth: Theme.icon + Theme.gapSmall
                    implicitHeight: Theme.row
                    Icon { anchors.centerIn: parent; name: "chevron-down"; rotation: -90 * Number(row.collapsed) }
                    TapHandler { gesturePolicy: TapHandler.ReleaseWithinBounds; onTapped: Hub.find.toggle(row.index) }
                }
                Text {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 0
                    elide: Text.ElideLeft
                    textFormat: Text.PlainText
                    color: Theme.ink
                    font.family: "monospace"
                    font.pixelSize: Theme.small
                    text: row.file
                }
                Text {
                    color: Theme.ink3
                    font.pixelSize: Theme.small
                    text: Theme.plural(row.count, "{} match", "{} matches")
                }
            }
            HoverHandler { cursorShape: Qt.PointingHandCursor }
            TapHandler { onTapped: Hub.find.select(row.index) }
        }
    }

    Component {
        id: lineRow
        Item {
            implicitHeight: code.implicitHeight + Theme.gapHair
            Accessible.role: Accessible.ListItem
            Accessible.name: String(row.number)
            Rectangle {
                anchors.fill: parent
                anchors.leftMargin: Theme.gapSmall
                anchors.rightMargin: Theme.gapSmall
                radius: Theme.corner
                color: Theme.accent
                opacity: 0.22 * Number(row.current)
            }
            Text {
                id: number
                visible: !row.isGap
                x: row.indent
                width: row.numberWidth
                horizontalAlignment: Text.AlignRight
                color: Theme.ink3
                font.family: "monospace"
                font.pixelSize: Theme.small
                text: row.number
            }
            Item {
                x: number.x + number.width + Theme.gap
                width: parent.width - x - Theme.gapLarge
                height: code.implicitHeight
                clip: true
                Text {
                    visible: row.isGap
                    color: Theme.ink3
                    font.family: "monospace"
                    font.pixelSize: Theme.small
                    text: "…"
                }
                Row {
                    id: code
                    opacity: 0.6 + 0.4 * Number(row.isMatch)
                    Repeater {
                        model: row.parts
                        Item {
                            id: part
                            required property var modelData
                            width: piece.implicitWidth
                            height: piece.implicitHeight
                            Rectangle {
                                visible: part.modelData.marked
                                anchors.fill: parent
                                radius: Theme.gapHair
                                color: Theme.accent
                                opacity: 0.35
                            }
                            Text {
                                id: piece
                                textFormat: Text.PlainText
                                color: Theme.ink
                                font.family: "monospace"
                                font.pixelSize: Theme.small
                                font.bold: part.modelData.marked
                                text: part.modelData.text
                            }
                        }
                    }
                }
            }
            HoverHandler { cursorShape: Qt.PointingHandCursor }
            TapHandler { onTapped: Hub.find.select(row.index) }
        }
    }
}
