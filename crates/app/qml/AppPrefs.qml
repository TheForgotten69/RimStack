// What is chosen for the whole app and kept between runs.
import QtCore
import QtQuick

Settings {
    category: "Appearance"
    // One of `Theme.themeNames`.
    property string theme: "rim"
    property bool comfortable: false
    property bool largeRows: false
    property bool reduceMotion: false
    property bool highContrast: false
    property real textScale: 1
    property bool hintsSeen: false
    property string folded: ""
    property int windowWidth: 1440
    property int windowHeight: 860
    property bool workshopInSteam: false
    // A preference added later goes last: read from a file written
    // before it existed, those declared after it come back as defaults.
    property bool ownColours: false
    property bool wrapLog: false
    property bool wrapTrace: true
    // A category's own colour, by its number and a name of `Theme.swatches`: "2=Rose,5=Sky".
    property string categoryColours: ""
}
