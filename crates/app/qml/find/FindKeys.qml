import QtQuick
import RimStack.Ui

Item {
    id: keys
    property ListView view: null
    property string title

    anchors.fill: parent
    activeFocusOnTab: view.count > 0
    Accessible.role: Accessible.List
    Accessible.name: title
    Keys.onUpPressed: { Hub.find.select(Hub.find.step(Hub.find.current, -1)); view.positionViewAtIndex(Hub.find.current, ListView.Contain) }
    Keys.onDownPressed: { Hub.find.select(Hub.find.step(Hub.find.current, 1)); view.positionViewAtIndex(Hub.find.current, ListView.Contain) }
    Keys.onSpacePressed: { Hub.find.select(Hub.find.fold(Hub.find.current, 0)); view.positionViewAtIndex(Hub.find.current, ListView.Contain) }
    Keys.onLeftPressed: { Hub.find.select(Hub.find.fold(Hub.find.current, -1)); view.positionViewAtIndex(Hub.find.current, ListView.Contain) }
    Keys.onRightPressed: { Hub.find.select(Hub.find.fold(Hub.find.current, 1)); view.positionViewAtIndex(Hub.find.current, ListView.Contain) }
    Keys.onReturnPressed: Hub.find.current >= 0 && Qt.openUrlExternally(Theme.fileUrl(Hub.find.openFile(Hub.find.current)))
    Keys.onEnterPressed: Hub.find.current >= 0 && Qt.openUrlExternally(Theme.fileUrl(Hub.find.openFile(Hub.find.current)))
    Shortcut { sequences: [StandardKey.Copy]; enabled: keys.activeFocus && Hub.find.current >= 0; onActivated: Theme.copy(Hub.find.copyLine(Hub.find.current)) }
    TapHandler { onTapped: keys.forceActiveFocus() }
    Rectangle { visible: keys.activeFocus; anchors.fill: parent; anchors.margins: Theme.gapLine; z: 2; radius: Theme.corner; color: "transparent"; border.width: 2; border.color: Theme.focus }
}
