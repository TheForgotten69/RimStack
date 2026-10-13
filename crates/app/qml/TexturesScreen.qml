// The Textures screen ("Faster loading"): which mods have PNG textures
// without a DDS beside them, and converting those so the game loads
// them faster.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RimStack.Ui

Item {
    id: screen
    Appear { of: screen }

    required property Backend backend

    readonly property var all: backend.textures ? JSON.parse(backend.textures) : []
    property bool activeOnly: true
    readonly property var shown: all.filter(m => !activeOnly || m.active)
    // The state most rows are in: not repeated on each of them.
    readonly property string usual: {
        const counts = {}
        for (const m of shown)
            counts[m.state] = (counts[m.state] || 0) + 1
        const most = Object.keys(counts).sort((a, b) => counts[b] - counts[a])[0] || ""
        return counts[most] > shown.length / 2 ? most : ""
    }
    // What "convert remaining" and "rebuild" would work on.
    readonly property var remaining: shown.filter(m => !m.skipped && m.notConverted > 0)
    readonly property var stale: shown.filter(m => !m.skipped && m.outOfDate > 0)
    readonly property var made: shown.filter(m => m.converted + m.outOfDate > 0)
    property int selectedId: -1
    readonly property var mod: all.find(m => m.id === selectedId) || null
    property bool confirmRemove: false

    onVisibleChanged: if (visible && all.length === 0) backend.loadTextures()
    Component.onCompleted: if (visible) backend.loadTextures()
    // A re-read of the mods empties this report: the open screen asks for it again.
    Connections {
        target: screen.backend
        function onTexturesChanged() { if (screen.visible && screen.backend.textures === "") screen.backend.loadTextures() }
    }

    function sum(list, key) {
        return list.reduce((n, m) => n + m[key], 0)
    }

    function megabytes(n) {
        if (n <= 0)
            return ""
        if (n < 0.1)
            return Theme.fill("{} KB", Math.max(1, Math.round(n * 1024)))
        return n < 10 ? Theme.fill("{} MB", n.toFixed(1)) : Theme.fill("{} MB", Theme.count(Math.round(n)))
    }

    function ids(list) {
        return JSON.stringify(list.map(m => m.id))
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.margins: Theme.gapLarge
                Layout.bottomMargin: Theme.gap
                implicitHeight: summary.implicitHeight + 16
                color: "transparent"
                readonly property int total: screen.sum(screen.shown, "textures")
                readonly property int have: total - screen.sum(screen.shown, "notConverted")
                ColumnLayout {
                    id: summary
                    x: 2
                    y: 8
                    width: parent.width - 4
                    spacing: Theme.gap
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gap
                        Flow {
                            Layout.fillWidth: true
                            spacing: Theme.gap
                            Row {
                                spacing: Theme.gap
                                Text { id: have; color: Theme.ink; font.pixelSize: Theme.heading; font.weight: Font.DemiBold; font.features: { "tnum": 1 }; text: Theme.fill("{} of {}", Theme.count(summary.parent.have), Theme.count(summary.parent.total)) }
                                Text { anchors.baseline: have.baseline; color: Theme.ink2; font.pixelSize: Theme.body; text: Theme.say("textures have a DDS"); Tip { text: Theme.meaning("DDS") } }
                            }
                            Chip {
                                maxWidth: parent.width
                                readonly property real png: screen.sum(screen.shown, "pngMb")
                                readonly property real dds: screen.sum(screen.shown, "ddsMb")
                                visible: png > 0
                                text: screen.megabytes(png) + " PNG  →  " + (screen.megabytes(dds) || Theme.say("0 MB")) + " DDS" + (dds > png * 1.2 ? ", " + Theme.fill("{}× the size on disk, loads faster", (dds / png).toFixed(1)) : "")
                            }
                            Chip {
                                readonly property var waiting: screen.shown.filter(m => m.waits)
                                visible: waiting.length > 0
                                pressable: true
                                tone: Theme.warn
                                text: Theme.plural(waiting.length, "{} mod to convert", "{} mods to convert")
                                tip: Theme.say("Shows the first of them")
                                onClicked: { screen.selectedId = waiting[0].id; list.positionViewAtBeginning() }
                            }
                            Chip { visible: screen.sum(screen.stale, "outOfDate") > 0; tone: Theme.warn; icon: "triangle-alert"; text: Theme.fill("{} out of date", Theme.count(screen.sum(screen.stale, "outOfDate"))) }
                        }
                        Btn {
                            Layout.alignment: Qt.AlignTop
                            text: Theme.say("Check all mods")
                            tip: Theme.say("Reads the header of every DDS file of the installed mods: not a DDS, cut short, a format the game has no reader for, or compressed with a size that is not a multiple of four.")
                            onClicked: ddsAudit.found = JSON.parse(screen.backend.ddsAudit())
                        }
                        Choice { Layout.alignment: Qt.AlignTop; title: Theme.say("Mods"); options: ["Active mods", "All mods"]; current: screen.activeOnly ? 0 : 1; onPicked: index => screen.activeOnly = index === 0 }
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 6
                        radius: Math.min(width, height) / 2
                        color: Theme.chip
                        Rectangle { width: summary.parent.total > 0 ? parent.width * summary.parent.have / summary.parent.total : 0; Behavior on width { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } } height: parent.height; radius: Math.min(width, height) / 2; color: summary.parent.have === summary.parent.total ? Theme.ok : Theme.accent }
                    }
                    // What deletes stands at the far end, away from the main action.
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gapWide
                        Flow {
                            Layout.fillWidth: true
                            spacing: Theme.gap
                            Text {
                                visible: text !== ""
                                width: Math.min(implicitWidth, parent.width)
                                height: Theme.control
                                verticalAlignment: Text.AlignVCenter
                                elide: Text.ElideRight
                                color: Theme.ink2
                                font.pixelSize: Theme.small
                                text: screen.backend.texturesBusy ? (screen.backend.texturesProgress || Theme.say("Counting textures…")) : screen.backend.texturesProgress
                            }
                            Btn { visible: screen.backend.texturesBusy; text: Theme.say("Cancel"); tip: Theme.say("Stops after the mod it is on; what is done stays done"); onClicked: screen.backend.cancelTextures() }
                            Btn {
                                primary: screen.remaining.length > 0
                                enabled: !screen.backend.texturesBusy && screen.remaining.length > 0
                                text: Theme.say("Convert remaining")
                                tip: Theme.say("Writes a .dds beside each .png that has none, inside each mod's own folder. The .png files are not changed.")
                                onClicked: screen.backend.convertTextures(screen.ids(screen.remaining))
                            }
                            Btn {
                                enabled: !screen.backend.texturesBusy && screen.stale.length > 0
                                text: Theme.say("Rebuild out of date")
                                tip: Theme.say("Converts again where the .png changed after its .dds was made.")
                                onClicked: screen.backend.convertTextures(screen.ids(screen.stale))
                            }
                        }
                        Btn {
                            Layout.alignment: Qt.AlignTop
                            enabled: !screen.backend.texturesBusy && screen.made.length > 0
                            danger: true
                            text: Theme.say("Remove all converted…")
                            tip: Theme.say("Deletes the .dds files RimStack made, inside the mods' folders. Files it did not make are not touched.")
                            onClicked: screen.confirmRemove = true
                        }
                    }
                }
            }

            Rectangle {
                visible: screen.confirmRemove
                Layout.fillWidth: true
                Layout.leftMargin: Theme.gapLarge
                Layout.rightMargin: Theme.gapLarge
                Layout.bottomMargin: Theme.gap
                implicitHeight: Theme.sized(40)
                radius: Theme.corner
                color: Theme.dangerBg
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.gapLarge
                    anchors.rightMargin: Theme.gap
                    spacing: Theme.gap
                    Text {
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        color: Theme.ink
                        font.pixelSize: Theme.body
                        text: Theme.fill("Delete the {} DDS files RimStack made in {} mods? The PNG files stay.", Theme.count(screen.sum(screen.made, "converted") + screen.sum(screen.made, "outOfDate")), screen.made.length)
                    }
                    Btn { text: Theme.say("Cancel"); onClicked: screen.confirmRemove = false }
                    Btn {
                        danger: true
                        text: Theme.say("Delete them")
                        onClicked: {
                            screen.confirmRemove = false
                            screen.backend.removeTextures(screen.ids(screen.made))
                        }
                    }
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.sized(24)
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.gapWide
                    anchors.rightMargin: Theme.gapWide
                    spacing: Theme.gapLarge
                    Text { Layout.preferredWidth: Theme.sized(560); Layout.minimumWidth: Theme.sized(80); color: Theme.ink3; font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Mod") }
                    Text { Layout.preferredWidth: Theme.sized(80); horizontalAlignment: Text.AlignRight; color: Theme.ink3; font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("Textures") }
                    Text { Layout.preferredWidth: Theme.sized(90); horizontalAlignment: Text.AlignRight; color: Theme.ink3; font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("PNG") }
                    Text { Layout.preferredWidth: Theme.sized(90); horizontalAlignment: Text.AlignRight; color: Theme.ink3; font.pixelSize: Theme.caption; font.weight: Font.Medium; text: Theme.say("DDS") }
                    Item { Layout.fillWidth: true }
                }
                Rectangle { anchors.bottom: parent.bottom; x: 6; width: parent.width - 12; height: 1; color: Theme.line }
            }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                topMargin: Theme.gapTiny
                clip: true
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds
                Wheel {}
                ScrollBar.vertical: ScrollBar {}
                model: screen.shown
                RowKeys {
                    parent: list
                    view: list
                    title: Theme.say("Mods with textures")
                    count: list.count
                    current: screen.shown.findIndex(m => m.id === screen.selectedId)
                    onChosen: index => screen.selectedId = screen.shown[index].id
                }
                delegate: Item {
                    id: row
                    required property var modelData
                    readonly property bool current: modelData.id === screen.selectedId
                    readonly property color quiet: current ? Theme.onSelectedDim : Theme.ink2
                    readonly property bool settled: modelData.notConverted === 0 && modelData.outOfDate === 0
                    Accessible.role: Accessible.ListItem
                    Accessible.name: Theme.fill("{}, {} textures, {}", modelData.name, Theme.count(modelData.textures), Theme.say(modelData.state))
                    width: list.width
                    height: Theme.row
                    opacity: settled && !current ? 0.6 : 1
                    Rectangle {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.gapSmall
                        anchors.rightMargin: Theme.gapSmall
                        radius: Theme.corner
                        color: row.current ? Theme.accent : Theme.hover
                        opacity: row.current ? 1 : rowHover.hovered ? 0.04 : 0
                    }
                    HoverHandler { id: rowHover }
                    TapHandler { onTapped: screen.selectedId = row.current ? -1 : row.modelData.id }
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.gapWide
                        anchors.rightMargin: Theme.gapWide
                        spacing: Theme.gapLarge
                        RowLayout {
                            // A layout inside a layout takes spare room unless told not to; the heading's cell does not.
                            Layout.fillWidth: false
                            Layout.preferredWidth: Theme.sized(560)
                            Layout.minimumWidth: Theme.sized(80)
                            spacing: Theme.gap
                            Text { Layout.maximumWidth: Theme.sized(560) - 190; elide: Text.ElideRight; color: row.current ? Theme.onSelected : Theme.ink; font.pixelSize: Theme.body; text: row.modelData.name }
                            Chip {
                                // Said only where it is not what every other row says.
                                visible: row.modelData.state !== screen.usual
                                maxWidth: Theme.sized(190)
                                Layout.alignment: Qt.AlignVCenter
                                onAccent: row.current
                                tone: row.modelData.state === "Converted" ? Theme.ok : row.modelData.state === "Out of date" ? Theme.warn : Theme.ink2
                                icon: row.modelData.state === "Converted" ? "check" : row.modelData.state === "Out of date" ? "triangle-alert" : ""
                                text: Theme.say(row.modelData.state)
                            }
                            Item { Layout.fillWidth: true }
                        }
                        Text { Layout.preferredWidth: Theme.sized(80); horizontalAlignment: Text.AlignRight; color: row.quiet; font.pixelSize: Theme.body; text: Theme.count(row.modelData.textures) }
                        Text { Layout.preferredWidth: Theme.sized(90); horizontalAlignment: Text.AlignRight; color: row.quiet; font.pixelSize: Theme.body; text: screen.megabytes(row.modelData.pngMb) }
                        Text { Layout.preferredWidth: Theme.sized(90); horizontalAlignment: Text.AlignRight; color: row.quiet; font.pixelSize: Theme.body; text: screen.megabytes(row.modelData.ddsMb) }
                        Item { Layout.fillWidth: true }
                    }
                }

                Column {
                    visible: screen.shown.length === 0
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 80, 460)
                    spacing: Theme.gapSmall
                    Text { width: parent.width; horizontalAlignment: Text.AlignHCenter; color: Theme.ink; font.pixelSize: Theme.title; font.weight: Font.DemiBold; text: screen.backend.texturesBusy ? Theme.say("Counting textures…") : Theme.say("No textures") }
                    Text { visible: !screen.backend.texturesBusy; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.body; lineHeight: 1.3; text: screen.all.length === 0 ? "" : Theme.say("None in active mods") }
                }
            }
        }

        Rectangle { visible: details.visible; Layout.fillHeight: true; implicitWidth: 1; color: Theme.edge }

        Rectangle {
            id: details
            visible: screen.mod !== null || ddsAudit.found !== null
            Layout.preferredWidth: Math.min(Theme.sized(360), screen.width * 0.3)
            Layout.fillHeight: true
            color: Theme.panel

            Column {
                id: side
                x: 14
                y: 14
                width: parent.width - 28

                Column {
                    width: parent.width
                    visible: screen.mod !== null
                    Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink; font.pixelSize: Theme.heading; font.weight: Font.DemiBold; text: screen.mod ? screen.mod.name : "" }
                    Text { width: parent.width; topPadding: Theme.gapHair; wrapMode: Text.WordWrap; color: Theme.ink2; font.pixelSize: Theme.body; text: screen.mod ? Theme.fill(screen.mod.active ? "{} textures, {}" : "{} textures, {}, not active", Theme.count(screen.mod.textures), Theme.say(screen.mod.state).toLowerCase()) : "" }

                    GroupTitle { text: Theme.say("Converted files") }
                    Repeater {
                        model: screen.mod ? [
                            [Theme.say("Created by RimStack"), [Theme.plural(screen.mod.converted + screen.mod.outOfDate, "{} file", "{} files"), screen.mod.madeMb > 0 ? screen.megabytes(screen.mod.madeMb) : ""].filter(part => part !== "").join(", "), false],
                            [Theme.say("Not made by RimStack"), Theme.plural(screen.mod.shipped, "{} file", "{} files"), false],
                            [Theme.say("Source PNG changed since"), Theme.plural(screen.mod.outOfDate, "{} file", "{} files"), screen.mod.outOfDate > 0],
                            [Theme.say("No DDS yet"), Theme.fill("{} files", Theme.count(screen.mod.notConverted)), false],
                        ] : []
                        RowLayout {
                            id: fact
                            required property var modelData
                            width: side.width
                            height: Theme.sized(24)
                            Text { Layout.fillWidth: true; color: fact.modelData[2] ? Theme.warn : Theme.ink; font.pixelSize: Theme.body; text: fact.modelData[0] }
                            Text { color: Theme.ink2; font.pixelSize: Theme.body; text: fact.modelData[1] }
                        }
                    }
                    Flow {
                        width: parent.width
                        topPadding: Theme.gap
                        spacing: Theme.gapSmall
                        Btn {
                            enabled: !screen.backend.texturesBusy && !!screen.mod && screen.mod.notConverted + screen.mod.outOfDate > 0
                            text: !!screen.mod && screen.mod.outOfDate > 0 ? Theme.say("Rebuild") : Theme.say("Convert")
                            onClicked: screen.backend.convertTextures(JSON.stringify([screen.mod.id]))
                        }
                        Btn {
                            enabled: !screen.backend.texturesBusy && !!screen.mod && screen.mod.converted + screen.mod.outOfDate > 0
                            text: Theme.say("Remove converted")
                            tip: Theme.say("Deletes only the .dds files RimStack made and that have not changed since.")
                            onClicked: screen.backend.removeTextures(JSON.stringify([screen.mod.id]))
                        }
                        Btn {
                            enabled: !screen.backend.texturesBusy
                            text: !!screen.mod && screen.mod.skipped ? Theme.say("Stop skipping") : Theme.say("Skip this mod")
                            onClicked: screen.backend.skipTextures(screen.mod.id, !screen.mod.skipped)
                        }
                    }
                }

                GroupTitle { visible: ddsAudit.found !== null; width: parent.width; wrapMode: Text.WordWrap; text: Theme.say("All mods: DDS files the game would refuse") }
                Column {
                    id: ddsAudit
                    property var found: null
                    visible: found !== null
                    width: parent.width
                    spacing: Theme.gapSmall
                    Row {
                        spacing: Theme.gap
                        Btn { text: Theme.say("Close"); onClicked: ddsAudit.found = null }
                        Btn {
                            visible: !!ddsAudit.found && ddsAudit.found.length > 0
                            text: Theme.say("Set all aside…")
                            tip: Theme.say("Renames each to .dds.rimstack-bad, so the game loads the PNG beside it.")
                            onClicked: Notices.ask({
                                title: Theme.fill("Set {} DDS files aside", ddsAudit.found.length),
                                lines: [Theme.say("Each is renamed to .dds.rimstack-bad inside its own mod's folder, so the game loads the PNG beside it. The first of them:"), ddsAudit.found[0].file],
                                label: Theme.say("Set aside"),
                                run: () => { screen.backend.setDdsAside(JSON.stringify(ddsAudit.found.map(bad => bad.file))); ddsAudit.found = JSON.parse(screen.backend.ddsAudit()) } })
                        }
                    }
                    Text {
                        visible: ddsAudit.found !== null
                        color: Theme.ink
                        font.pixelSize: Theme.body
                        text: ddsAudit.found === null ? "" : ddsAudit.found.length === 0 ? Theme.say("None") : Theme.fill("{} files", Theme.count(ddsAudit.found.length))
                    }
                    Repeater {
                        model: ddsAudit.found === null ? [] : ddsAudit.found.slice(0, 60)
                        Column {
                            id: refused
                            required property var modelData
                            width: ddsAudit.width
                            Text { width: parent.width; elide: Text.ElideLeft; color: Theme.ink; font.pixelSize: Theme.small; text: refused.modelData.mod + " — " + refused.modelData.file.split("/").pop() }
                            Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.ink3; font.pixelSize: Theme.small; text: refused.modelData.why }
                        }
                    }
                }

                GroupTitle { text: Theme.say("Quality") }
                Text {
                    width: parent.width
                    wrapMode: Text.WordWrap
                    color: Theme.ink
                    font.pixelSize: Theme.body
                    lineHeight: 1.3
                    text: Theme.say("BC7 with transparency, DXT1 without; with mipmaps")
                    Tip { text: Theme.say("A .dds file is written beside each .png and the game loads it instead: quicker, and less memory. The .png is not changed. Steam may delete the .dds files when it updates a mod; they then show as not converted.") }
                }
            }
        }
    }
}
