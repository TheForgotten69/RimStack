// A push button with text, an icon, or both. Disabled, it dims and
// ignores clicks.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import RimStack.Ui

Rectangle {
    id: btn
    property alias text: btnLabel.text
    property bool primary: false
    property bool large: false
    property bool danger: false
    property string icon
    // It opens a menu: a chevron after its words.
    property bool menu: false
    property string tip
    // What a screen reader says where the text alone would not tell this
    // button from its neighbours: "Show Goodwill Preview".
    property string spoken
    // A colour of its own, for the one button that is neither plain nor
    // the main action: tinted with it, labelled in it.
    property color tone: "transparent"
    readonly property bool toned: tone.a > 0 && !primary
    // Where room is short: the icon alone, its text under the pointer.
    property bool compact: false
    readonly property bool bare: compact && icon !== ""
    signal clicked()
    implicitWidth: btnContent.implicitWidth + (large ? 36 : (text === "" || bare) && !menu ? 14 : 20)
    implicitHeight: large ? Theme.controlLarge : Theme.control
    radius: Theme.corner
    // Not pressable: the frame fades further than the words, which still say what could be done.
    readonly property real frame: enabled ? 1 : 0.4
    activeFocusOnTab: true
    Keys.onSpacePressed: btn.clicked()
    Keys.onReturnPressed: btn.clicked()
    Keys.onEnterPressed: btn.clicked()
    border.width: primary ? 0 : 1
    border.color: toned ? Qt.rgba(tone.r, tone.g, tone.b, frame) : Qt.rgba(Theme.fieldLine.r, Theme.fieldLine.g, Theme.fieldLine.b, frame)
    FocusRing {}
    Accessible.role: Accessible.Button
    // What does not show is not read out either.
    Accessible.ignored: !visible
    Accessible.name: spoken !== "" ? spoken : text !== "" ? text : tip !== "" ? tip : icon
    Accessible.onPressAction: if (enabled) btn.clicked()
    readonly property color ground: primary ? (btnHover.hovered ? Theme.accentHi : Theme.accent)
        : toned ? Qt.rgba(tone.r, tone.g, tone.b, btnHover.hovered ? 0.3 : 0.18)
        : (btnHover.hovered ? Theme.ctlHi : Theme.ctl)
    color: Qt.rgba(ground.r, ground.g, ground.b, ground.a * frame)
    scale: btnPress.pressed && !Theme.stored.reduceMotion ? 0.97 : 1
    Behavior on color { ColorAnimation { duration: Theme.quick } }
    Behavior on scale { NumberAnimation { duration: Theme.quick / 2 } }
    Row {
        id: btnContent
        anchors.centerIn: parent
        spacing: Theme.gapSmall
        opacity: btn.enabled ? 1 : 0.5
        Icon { visible: btn.icon !== ""; name: btn.icon; size: Theme.icon; color: btnLabel.color; anchors.verticalCenter: parent.verticalCenter }
        Text {
            id: btnLabel
            visible: text !== "" && !btn.bare
            // Red says "this destroys something" only while it can be pressed.
            color: btn.primary ? Theme.onSelected : btn.danger && btn.enabled ? Theme.err : btn.toned ? btn.tone : Theme.ink
            font.pixelSize: btn.large ? Theme.body : Theme.small
            font.weight: btn.large ? Font.DemiBold : btn.primary || btn.toned ? Font.Medium : Font.Normal
        }
        Icon { visible: btn.menu; name: "chevron-down"; size: Theme.sized(12); color: btnLabel.color; opacity: 0.8; anchors.verticalCenter: parent.verticalCenter }
    }
    HoverHandler { id: btnHover }
    // The tip shows for the keyboard too: on a button reached with Tab.
    Tip { text: btn.bare && btn.tip === "" ? btn.text : btn.tip; over: btnHover.hovered || btn.activeFocus }
    TapHandler { id: btnPress; onTapped: btn.clicked() }
}
