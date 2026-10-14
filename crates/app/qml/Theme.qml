// Colours, a few preferences and small helpers shared by every screen.
// The colours follow the theme chosen in Settings.
pragma Singleton
import QtQuick
import RimStack.Ui

QtObject {
    id: theme

    readonly property AppPrefs stored: AppPrefs {}

    readonly property string name: stored.theme
    readonly property bool light: palette.bright === true
    // The colours `theme.json` gives for the chosen theme, by the names
    // below; each stands over the built-in one.
    readonly property var custom: Hub.settings.ownColours || ({})
    readonly property Binding themeName: Binding { target: Hub.settings; property: "theme"; value: theme.name }
    readonly property var colourNames: ["win", "panel", "bar", "ctl", "ctlHi", "line", "edge", "well", "seg", "chip", "hover", "ink", "ink2", "ink3", "brand", "fieldLine", "accent", "accentHi", "focus", "accentInk", "err", "warn", "ok", "moved", "banner", "dangerBg", "scrim"]

    // Type: five sizes, the lists in `body`.
    // Scaled by the text size chosen in Settings, at once.
    readonly property real scale: Math.max(0.85, Math.min(1.6, stored.textScale))
    readonly property int caption: Math.round(11 * scale)
    readonly property int small: Math.round(12 * scale)
    readonly property int body: Math.round(13 * scale)
    readonly property int title: Math.round(15 * scale)
    readonly property int heading: Math.round(17 * scale)
    readonly property int display: Math.round(26 * scale)
    // What holds text grows with it, and never shrinks under its design size.
    readonly property real grow: Math.max(1, scale)
    readonly property int row: Math.round((stored.largeRows ? 32 : 26) * grow)
    readonly property int control: Math.round(24 * grow)
    readonly property int controlLarge: Math.round(28 * grow)
    readonly property int field: Math.round(28 * grow)
    readonly property int segment: Math.round(24 * grow)
    readonly property int tag: Math.round(18 * grow)
    readonly property int strip: Math.round(26 * grow)
    readonly property int header: Math.round(42 * grow)
    readonly property int topBar: Math.round(52 * grow)
    readonly property int icon: Math.round(14 * grow)
    function sized(pixels) {
        return Math.round(pixels * grow)
    }
    // Corners: a control, a panel or dialog.
    readonly property int corner: 6
    readonly property int cornerLarge: 9
    // Distances between neighbours, and between the cards of a panel
    // and of a page.
    readonly property int gapLine: 1
    readonly property int gapHair: 2
    readonly property int gapThin: 3
    readonly property int gapTiny: 4
    readonly property int gapSmall: 6
    readonly property int gap: 8
    readonly property int gapMedium: 10
    readonly property int gapLarge: 12
    readonly property int gapWide: 16
    readonly property int gapCard: 14
    readonly property int gapPage: 20
    // How long a change of state takes to show, in milliseconds.
    readonly property int quick: stored.reduceMotion ? 0 : 140
    // A pane sliding in, a page fading in: longer, to be followed by the eye.
    readonly property int calm: stored.reduceMotion ? 0 : 220
    readonly property bool contrast: stored.highContrast
    // Text and marks on a selected row or the accent colour.
    readonly property color onSelected: "#ffffff"
    readonly property color onSelectedDim: "#ebffffff"
    // Behind a chip or a count on a selected row: darker than the accent,
    // so white on it keeps 7:1 in every theme (white at 20% gave 3.6:1).
    readonly property color selectedChip: "#38000000"

    readonly property string issues: "https://github.com/TheForgotten69/RimStack/issues"

    // The chosen language: each English text of the interface with what
    // stands for it. A text it does not have is shown in English.
    readonly property var words: Hub.settings.words || ({})
    function say(text) {
        return words[text] || text
    }
    // A sentence with places: each `{}` takes the next part, in order.
    function fill(text, ...parts) {
        let at = 0
        return say(text).replace(/\{\}/g, () => parts[at++])
    }
    // Marks a text kept in English as it is written down, and put in the
    // chosen language where it is shown: see `put`.
    function kept(text) {
        return text
    }
    // A sentence kept as data, in words: [english, part…], a part being a
    // sentence itself; ["#", n, one, many, part…] for a count; [",", …] for a list.
    function put(sentence) {
        if (!Array.isArray(sentence))
            return String(sentence)
        const parts = sentence.slice(1).map(put)
        if (sentence[0] === ",")
            return parts.join(", ")
        return sentence[0] === "#" ? plural(sentence[1], sentence[2], sentence[3], ...parts.slice(3)) : fill(sentence[0], ...parts)
    }
    // What a record says: a kept sentence, or the plain text an older record holds.
    function said(record) {
        if (record.charAt(0) !== "[")
            return say(record)
        try {
            return put(JSON.parse(record))
        } catch (problem) {
            return record
        }
    }
    // `fill` with the count first, in the form the language uses for it:
    // the language file's "@plural" lists the counts that take the singular.
    // After a slash, the last digits that take a third form outside the
    // teens ("1 / 2 3 4" in Polish), written before the plural's with a bar.
    function plural(n, one, many, ...parts) {
        const rule = (words["@plural"] || "1").split("/").map(side => side.trim().split(" "))
        let forms = say(many).split("|")
        if (rule[0].indexOf(String(n)) !== -1)
            forms = [say(one)]
        else if (forms.length > 1 && !(rule[1] && rule[1].indexOf(String(n % 10)) !== -1 && (n % 100 < 11 || n % 100 > 19)))
            forms = [forms[1]]
        let at = 0
        const all = [count(n), ...parts]
        return forms[0].replace(/\{\}/g, () => all[at++])
    }

    // Each theme's colours by name; `high` stands over them when Contrast
    // is High: the two greys of text move toward the ink, lines toward the text.
    readonly property var palettes: ({
        dark: {
            win: "#191b23", panel: "#1f212b", bar: "#272a36", ctl: "#353949", ctlHi: "#454a5e", line: "#323646", edge: "#101117",
            well: "#13151b", seg: "#16181f", chip: "#14ffffff", hover: "#ffffff", ink: "#f3f1ec", ink2: "#b3b2ba", ink3: "#9b9ba6",
            brand: "#d9b88a", fieldLine: "#6c728b", accent: "#2468c8", accentHi: "#2f75d6", focus: "#4c96f5", accentInk: "#ffffff",
            err: "#ff6961", warn: "#f5a93b", ok: "#5fcf80", moved: "#6cb0ff", banner: "#1b2a44", dangerBg: "#3a2222", scrim: "#66000000",
            high: { ink2: "#e4e3ea", ink3: "#c9c8d2", line: "#7c8199", fieldLine: "#b9bdd0" },
        },
        oled: {
            win: "#000000", panel: "#0b0b0c", bar: "#111113", ctl: "#202023", ctlHi: "#303034", line: "#242427", edge: "#000000",
            well: "#060607", seg: "#0d0d0e", chip: "#1cffffff", hover: "#ffffff", ink: "#f3f1ec", ink2: "#a8a8ae", ink3: "#8f8f96",
            brand: "#d9b88a", fieldLine: "#62677c", accent: "#1f63c2", accentHi: "#2a70d4", focus: "#4c96f5", accentInk: "#ffffff",
            err: "#ff6961", warn: "#f5a93b", ok: "#5fcf80", moved: "#6cb0ff", banner: "#0d1d33", dangerBg: "#2a1414", scrim: "#80000000",
            high: { ink2: "#e4e3ea", ink3: "#c9c8d2", line: "#74788c", fieldLine: "#b9bdd0" },
        },
        light: {
            bright: true,
            win: "#f5f5f7", panel: "#ececef", bar: "#e3e3e7", ctl: "#d5d5da", ctlHi: "#ffffff", line: "#cdcdd3", edge: "#c3c3c9",
            well: "#ffffff", seg: "#dadade", chip: "#12000000", hover: "#000000", ink: "#1d1d1f", ink2: "#55555c", ink3: "#5c5c64",
            brand: "#7a4d22", fieldLine: "#85858f", accent: "#1f63c2", accentHi: "#2a70d4", focus: "#1f63c2", accentInk: "#ffffff",
            err: "#ab241e", warn: "#824b00", ok: "#14682a", moved: "#1558b0", banner: "#d7e5fb", dangerBg: "#f7dcda", scrim: "#40000000",
            high: { ink2: "#26262a", ink3: "#3a3a40", line: "#6d6d77", fieldLine: "#4a4a52" },
        },
        // After RimWorld's own interface: warm dark earth, parchment text, and
        // the blue of its panels, more saturated, for what is chosen or pressed.
        rim: {
            win: "#1c1916", panel: "#23201c", bar: "#2b2722", ctl: "#3b352d", ctlHi: "#4b4439", line: "#3a342c", edge: "#110f0d",
            well: "#151311", seg: "#181512", chip: "#14ffffff", hover: "#ffffff", ink: "#f2ebdd", ink2: "#bcb3a3", ink3: "#a0988a",
            brand: "#e2bb7c", fieldLine: "#7d7465", accent: "#2870ab", accentHi: "#2b76b3", focus: "#7db4dc", accentInk: "#ffffff",
            err: "#ff7b6b", warn: "#f2b04a", ok: "#8fce7a", moved: "#8fc3ea", banner: "#22303a", dangerBg: "#3d2320", scrim: "#66000000",
            high: { ink2: "#e6dfd0", ink3: "#cfc7b8", line: "#8a8172", fieldLine: "#c2b9a8" },
        },
    })
    readonly property var themeNames: ["rim", "dark", "oled", "light"]
    readonly property var themeLabels: ["Rim", "Dark", "OLED", "Light"]
    readonly property var palette: palettes[name] || palettes.rim
    readonly property var raised: contrast ? palette.high : ({})
    // The user's own colours for this theme, from `theme.json`.
    readonly property var own: stored.ownColours ? custom : ({})
    readonly property bool ownInUse: stored.ownColours && Object.keys(custom).length > 0
    function colour(key) {
        return raised[key] || own[key] || palette[key]
    }

    readonly property color win: colour("win")
    readonly property color panel: colour("panel")
    readonly property color bar: colour("bar")
    readonly property color ctl: colour("ctl")
    readonly property color ctlHi: colour("ctlHi")
    readonly property color line: colour("line")
    readonly property color edge: colour("edge")
    readonly property color well: colour("well")
    // Behind a segmented control, and behind a chip or a hovered item.
    readonly property color seg: colour("seg")
    readonly property color chip: colour("chip")
    // Laid over a row under the pointer, at a low opacity.
    readonly property color hover: colour("hover")
    readonly property color ink: colour("ink")
    readonly property color ink2: colour("ink2")
    readonly property color ink3: colour("ink3")
    // The warm colour of the icon's planet: for the app's name, never for
    // something to press.
    readonly property color brand: colour("brand")
    // Round a box to type in, a button or a segmented control: seen
    // against the window without its fill.
    readonly property color fieldLine: colour("fieldLine")
    readonly property color accent: colour("accent")
    readonly property color accentHi: colour("accentHi")
    readonly property color focus: colour("focus")
    // Text and marks on the accent colour, in every theme.
    readonly property color accentInk: colour("accentInk")
    readonly property color err: colour("err")
    readonly property color warn: colour("warn")
    readonly property color ok: colour("ok")
    readonly property color moved: colour("moved")
    // Behind a notice, and behind a question before something is destroyed.
    readonly property color banner: colour("banner")
    readonly property color dangerBg: colour("dangerBg")
    // Over what a dialog or a drawer stands in front of.
    readonly property color scrim: colour("scrim")

    // By category, in load order: loaders, game, libraries, content,
    // patches, textures, interface, performance, loads last.
    readonly property var categoryIcons: ["plug", "gamepad-2", "library", "package", "wrench", "image", "layout-dashboard", "clock", "arrow-down-to-line"]
    // The colours a category can be given, each for dark themes and for
    // Light; the first nine are the categories' own, in their order.
    readonly property var swatches: [
        ["Violet", "#9a8fc2", "#6d5fa8"], ["Slate", "#8d99a6", "#5f6b78"], ["Blue", "#6fa3c7", "#3f7da6"],
        ["Green", "#86b06b", "#4f8a36"], ["Amber", "#d2a94e", "#9a7a1c"], ["Coral", "#c98277", "#a8574b"],
        ["Teal", "#7fb8b0", "#3f8a80"], ["Sand", "#c9a36a", "#8a6a2a"], ["Grey", "#a3a3ad", "#6f6f78"],
        ["Rose", "#d58fb3", "#a44f7c"], ["Lime", "#b5c95a", "#6f8420"], ["Sky", "#7cc4e8", "#2f86b3"], ["Orange", "#e09a5c", "#a8621e"],
    ]
    function swatch(name) {
        const found = swatches.find(s => s[0] === name)
        return found ? found[light ? 2 : 1] : ""
    }
    // Per category number, the name of the swatch chosen for it.
    readonly property var chosenSwatches: {
        const chosen = {}
        for (const pair of stored.categoryColours.split(",")) {
            const [category, name] = pair.split("=")
            if (name)
                chosen[category] = name
        }
        return chosen
    }
    readonly property var categoryColours: categoryIcons.map((icon, category) => swatch(chosenSwatches[category] || swatches[category][0]))
    // "" gives the category its own colour back.
    function paintCategory(category, name) {
        const now = Object.assign({}, chosenSwatches)
        if (name === "" || name === swatches[category][0])
            delete now[category]
        else
            now[category] = name
        stored.categoryColours = Object.keys(now).map(key => key + "=" + now[key]).join(",")
    }

    readonly property var categoryNames: ["Loaders", "Game and DLC", "Libraries", "Content", "Patches", "Textures", "Interface", "Performance & tweaks", "Loads last"]

    // `--wheel-log` prints what each turn of the mouse wheel sends.
    readonly property bool wheelLog: Qt.application.arguments.indexOf("--wheel-log") !== -1
    readonly property bool selfTest: Qt.application.arguments.indexOf("--selftest") !== -1
    readonly property bool demo: selfTest || Qt.application.arguments.indexOf("--demo") !== -1

    // Words of the trade, each in a sentence; shown where a heading uses one.
    readonly property var terms: ({
        "Load order": "The order in which the game loads the active mods. A mod that changes another has to load after it.",
        "Own rules": "Rules you added: this mod loads after, or before, that one. Sort follows them before anything else.",
        "Community rules": "Load-order rules kept by players (RimSort's Community Rules Database), for mods whose own files do not say enough.",
        "Detected from files": "What a mod's files point at in another mod. A hint for Sort, not something the game requires.",
        "Texture conflicts": "Two active mods provide a picture under the same name. The game uses the one from the mod that loads later.",
        "Patched methods": "The methods of the game this mod's code changes, through the Harmony library. Two mods patching one method may clash.",
        "Group": "A mod's first tag. Inside a category, mods with the same first tag are kept side by side under its name.",
        "Textures set aside": "Files RimStack renamed inside this mod so the game skips them. Nothing is deleted.",
        "Folders the game loads from": "A mod can ship a folder per game version or per other mod; the game reads only those that apply.",
        "Requires": "Mods this one needs active, as its About.xml says.",
        "Filed under": "The category Sort keeps this mod in. Categories load in a fixed order: loaders, the game, libraries, content, patches, textures.",
        "DDS": "A texture format the game loads faster than PNG. RimStack writes DDS copies beside the PNG files.",
        "To review": "Not problems: things worth a look when you have a moment.",
    })
    function meaning(term) {
        // A heading arrives in the chosen language; the terms are kept in English.
        const english = Object.keys(terms).find(key => key === term || say(key) === term)
        return english ? say(terms[english]) : ""
    }

    // The cards folded away, by key.
    readonly property var folded: stored.folded === "" ? [] : stored.folded.split(",")
    function fold(key) {
        const now = folded.filter(k => k !== key)
        if (now.length === folded.length)
            now.push(key)
        stored.folded = now.join(",")
    }

    // Qt gives a new application font to texts made afterwards only:
    // those on screen, and in menus not open, are given it here. A text
    // with a family of its own (monospace) keeps it.
    function refont(root, from, to) {
        if (!root || from === to)
            return
        if (root.font !== undefined && root.font.family === from)
            root.font.family = to
        for (const child of root.children || [])
            refont(child, from, to)
        for (const kept of root.resources || []) {
            if (kept.contentItem)
                refont(kept.contentItem, from, to)
        }
    }

    // Text as StyledText shows it letter for letter.
    function plain(text) {
        return String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;")
    }

    // A date, or a date and time, in figures: the same in every language.
    function day(seconds) {
        const d = new Date(seconds * 1000)
        return d.getFullYear() + "-" + String(d.getMonth() + 1).padStart(2, "0") + "-" + String(d.getDate()).padStart(2, "0")
    }
    function moment(seconds) {
        const d = new Date(seconds * 1000)
        return day(seconds) + " " + String(d.getHours()).padStart(2, "0") + ":" + String(d.getMinutes()).padStart(2, "0")
    }

    function clock(seconds) {
        const d = new Date(seconds * 1000)
        return String(d.getHours()).padStart(2, "0") + ":" + String(d.getMinutes()).padStart(2, "0")
    }

    // How long ago a moment (seconds since 1970) was.
    function ago(seconds) {
        const minutes = Math.floor((Date.now() / 1000 - seconds) / 60)
        if (minutes < 1)
            return say("just now")
        if (minutes < 60)
            return plural(minutes, "{} minute ago", "{} minutes ago")
        const hours = Math.floor(minutes / 60)
        if (hours < 48)
            return plural(hours, "{} hour ago", "{} hours ago")
        const days = Math.floor(hours / 24)
        if (days < 60)
            return plural(days, "{} day ago", "{} days ago")
        if (days < 730)
            return plural(Math.floor(days / 30), "{} month ago", "{} months ago")
        return plural(Math.floor(days / 365), "{} year ago", "{} years ago")
    }

    function count(n) {
        return Number(n).toLocaleString(Qt.locale(), "f", 0)
    }

    // On Windows a path has backslashes and starts with its drive: file:///C:/…
    function fileUrl(path) {
        const parts = String(path).replace(/\\/g, "/").split("/")
        return "file://" + (parts[0] === "" ? "" : "/") + parts.map(part => encodeURIComponent(part).replace(/%3A/g, ":")).join("/")
    }

    property TextEdit clip: TextEdit { visible: false }
    function copy(text) {
        clip.text = text
        clip.selectAll()
        clip.copy()
        clip.text = ""
    }

    function pasted() {
        clip.text = ""
        clip.paste()
        const text = clip.text
        clip.text = ""
        return text
    }

    function openWorkshop(id) {
        Qt.openUrlExternally(stored.workshopInSteam ? "steam://url/CommunityFilePage/" + id
            : "https://steamcommunity.com/sharedfiles/filedetails/?id=" + id)
    }

    function openChangeNotes(id) {
        const page = "https://steamcommunity.com/sharedfiles/filedetails/changelog/" + id
        Qt.openUrlExternally(stored.workshopInSteam ? "steam://openurl/" + page : page)
    }
}
