// A mod's menu: only what applies to it, in groups. An entry that cannot
// be used is left out, not greyed.
import QtQuick
import RimStack.Ui

QtObject {
    function open(row, x, y) {
        const inActive = row.ownModel === Mods.active
        if (!(row.current))
            Mods.select(row.modId, inActive)
        const about = Mods.about(row.modId)
        const many = Mods.selectedCount > 1
        const workshop = about.source === "Workshop"
        const page = "https://steamcommunity.com/sharedfiles/filedetails/?id=" + about.folder
        const groups = []
        groups.push([
            { text: many ? (inActive ? Theme.plural(Mods.selectedCount, "Deactivate {} mod", "Deactivate {} mods") : Theme.plural(Mods.selectedCount, "Activate {} mod", "Activate {} mods")) : inActive ? Theme.say("Deactivate") : Theme.say("Activate"), icon: inActive ? "minus" : "plus", off: Mods.active.previewing,
              run: () => many ? Mods.moveSelected() : Mods.moveAcross(row.ownModel, row.ownModel.indexOf(row.modId), row.modId) },
        ])
        const open = []
        if (about.path)
            open.push({ text: Theme.say("Open folder"), icon: "folder-open", run: () => Qt.openUrlExternally(Theme.fileUrl(about.path)) })
        if (about.url)
            open.push({ text: Theme.say("Mod's own page"), icon: "link", run: () => Qt.openUrlExternally(about.url) })
        open.push({ text: Theme.say("Copy name"), icon: "copy", run: () => Theme.copy(about.name || "") })
        if (about.packageId)
            open.push({ text: Theme.say("Copy package id"), icon: "copy", run: () => Theme.copy(about.packageId) })
        groups.push(open)
        if (workshop) {
            const steam = [
                { title: Theme.say("Steam Workshop") },
                { text: Theme.say("Page in Steam"), icon: "external-link", run: () => Qt.openUrlExternally("steam://url/CommunityFilePage/" + about.folder) },
                { text: Theme.say("Page in the browser"), icon: "external-link", run: () => Qt.openUrlExternally(page) },
                { text: Theme.say("Copy link"), icon: "copy", run: () => Theme.copy(page) },
                { text: Theme.say("Copy to the game's Mods folder…"), icon: "copy", run: () => Notices.ask({
                    title: Theme.fill("Copy {} to the game's Mods folder", (about.name || "")),
                    lines: [Theme.say("From"), about.path, Theme.say("Into"), (Hub.settings.report.game || "") + "/Mods", Theme.say("The Workshop copy stays where it is; Steam keeps updating that one, not this.")],
                    label: Theme.say("Copy"), run: () => Mods.copyToLocal(about.path) }) },
            ]
            if (Hub.settings.report.steamcmd)
                steam.push({ text: Theme.say("Download again with SteamCMD…"), icon: "download", run: () => Notices.ask({
                    title: Theme.fill("Download {} again", (about.name || "")),
                    lines: [Theme.say("SteamCMD downloads the mod anew, and the fresh copy replaces this folder:"), about.path],
                    label: Theme.say("Download"), run: () => Mods.downloads.redownload(about.folder, about.path) }) })
            if (!inActive)
                steam.push({ text: Theme.say("Unsubscribe…"), icon: "minus", danger: true, run: () => Mods.unsubscribe(about.name || "", about.folder, about.path) })
            groups.push(steam)
        }
        if (about.repository)
            groups.push([
                { title: Theme.say("Git") },
                { text: Theme.say("Repository"), icon: "link", run: () => Qt.openUrlExternally(about.repository) },
                { text: Theme.say("Update"), icon: "download", run: () => Mods.downloads.updateFromGit(about.path) },
            ])
        // Its own other tags first, then the groups its neighbours are in.
        const filing = [{ title: Theme.say("Group under a tag") }]
        const own = (about.tags || "").split(",").map(tag => tag.trim()).filter(tag => tag !== "")
        const beside = row.ownModel.subsBeside(row.modId).split("\n").filter(name => name !== "")
        const offered = [...new Set(own.concat(beside))].filter(name => name !== about.sub)
        for (const tag of offered)
            filing.push({ text: Theme.fill("Group under {}", tag), run: () => Mods.putInSub(row.modId, tag) })
        filing.push({ text: Theme.say("A new tag…"), icon: "plus", run: () => Mods.subNameRequested(row.modId) })
        if (about.sub)
            filing.push({ text: Theme.fill("Remove the tag {}", about.sub), run: () => Mods.putInSub(row.modId, "") })
        groups.push(filing)
        const last = []
        if (many && Mods.versionWarned.length > 0)
            last.push({ text: Theme.plural(Mods.versionWarned.length, "Ignore the game version mismatch of {} mod", "Ignore the game version mismatch of {} mods"), icon: "check", run: () => Mods.trustSelected(true) })
        else if (many && about.versionOk)
            last.push({ text: Theme.plural(Mods.selectedCount, "Warn about the game version of {} mod again", "Warn about the game version of {} mods again"), run: () => Mods.trustSelected(false) })
        else if (about.versionOk)
            last.push({ text: Theme.say("Warn about its game version again"), run: () => Mods.trust(row.modId, false) })
        else if ((about.problems || []).some(p => p.fix.action === "trust"))
            last.push({ text: Theme.say("Ignore the game version mismatch"), icon: "check", run: () => Mods.trust(row.modId, true) })
        if (about.source !== "Official" && about.path && !inActive)
            last.push({ text: Theme.say("Move to the bin…"), icon: "trash-2", danger: true, run: () => Mods.binRequested(about.name || "", about.path) })
        groups.push(last)
        const entries = []
        for (const group of groups.filter(group => group.length > 0)) {
            if (entries.length > 0)
                entries.push({ line: true })
            entries.push(...group)
        }
        Mods.menuRequested(row, x, y, entries)
    }
}
