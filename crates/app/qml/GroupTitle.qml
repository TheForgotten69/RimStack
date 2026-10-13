// A small heading inside a side panel. A term the glossary knows gets a
// dotted line and says what it means under the pointer.
pragma ComponentBehavior: Bound
import QtQuick
import RimStack.Ui

Text {
    id: title
    readonly property string meaning: Theme.meaning(text)
    topPadding: Theme.gapWide
    bottomPadding: Theme.gapSmall
    color: Theme.ink2
    font.pixelSize: Theme.small
    font.weight: Font.DemiBold
    // A word with a meaning is reached with Tab, which shows it as the pointer does.
    activeFocusOnTab: meaning !== ""
    font.underline: activeFocus
    Accessible.role: Accessible.Heading
    Accessible.name: text
    Accessible.description: meaning
    HoverHandler { id: pointed }
    Dots { visible: title.meaning !== ""; y: parent.height - 5; width: title.contentWidth; tone: title.color }
    Tip { text: title.meaning; over: pointed.hovered || title.activeFocus }
}
