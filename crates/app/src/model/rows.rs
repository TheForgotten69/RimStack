//! What each row of the list shows: the roles QML reads and the value
//! of each for a heading or a mod.

use super::*;

const IS_HEADER: i32 = 257;
const NAME: i32 = 258;
const AUTHOR: i32 = 259;
const VERSION: i32 = 260;
const NUMBER: i32 = 261;
const COUNT: i32 = 262;
const COLLAPSED: i32 = 263;
const CATEGORY: i32 = 264;
const MOD_INDEX: i32 = 265;
const MOD_ID: i32 = 266;
const MOVED_FROM: i32 = 267;
const TROUBLE: i32 = 268;
const TAGS: i32 = 269;
const TROUBLE_TEXT: i32 = 270;
const LOCAL: i32 = 271;
const PICTURE: i32 = 272;
const IS_SUB: i32 = 273;
const LOAD_TIME: i32 = 274;
const AGAIN: i32 = 275;
const IGNORED: i32 = 276;
const SOURCE: i32 = 277;
const LOAD_PART: i32 = 278;
const FRESH: i32 = 279;
const WHY: i32 = 280;
const QUIET: i32 = 281;
const GROUP_SIZE: i32 = 282;
const TWIN: i32 = 283;

const ROLES: [(i32, &str); 27] = [
    (TWIN, "twin"),
    (GROUP_SIZE, "groupSize"),
    (QUIET, "troubleQuiet"),
    (WHY, "whyMoved"),
    (FRESH, "fresh"),
    (SOURCE, "source"),
    (LOAD_PART, "loadPart"),
    (IGNORED, "versionIgnored"),
    (AGAIN, "again"),
    (LOAD_TIME, "loadTime"),
    (IS_SUB, "isSub"),
    (IS_HEADER, "isHeader"),
    (NAME, "name"),
    (AUTHOR, "author"),
    (VERSION, "version"),
    (NUMBER, "number"),
    (COUNT, "count"),
    (COLLAPSED, "collapsed"),
    (CATEGORY, "category"),
    (MOD_INDEX, "modIndex"),
    (MOD_ID, "modId"),
    (MOVED_FROM, "movedFrom"),
    (TROUBLE, "trouble"),
    (TAGS, "tags"),
    (TROUBLE_TEXT, "troubleText"),
    (LOCAL, "local"),
    (PICTURE, "picture"),
];

impl qobject::ModListModel {
    pub(crate) fn row_count(&self, _parent: &QModelIndex) -> i32 {
        self.list.rows().len() as i32
    }

    pub(crate) fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut names = QHash::<QHashPair_i32_QByteArray>::default();
        for (role, name) in ROLES {
            names.insert(role, QByteArray::from(name));
        }
        names
    }

    pub(crate) fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(row) = self.list.rows().get(index.row() as usize) else {
            return QVariant::default();
        };
        match *row {
            Row::Sub {
                first,
                count,
                collapsed,
            } => {
                let of = &self.list.mods()[first];
                match role {
                    IS_HEADER | IS_SUB => QVariant::from(&true),
                    // The group has mods further up as well: a rule keeps them apart.
                    AGAIN => {
                        let mods = self.list.mods();
                        let earlier = &mods[..first];
                        let same = |m: &&Mod| m.category == of.category && m.sub == of.sub;
                        QVariant::from(&earlier.iter().any(|m| same(&m)))
                    }
                    IGNORED | FRESH | QUIET | TWIN => QVariant::from(&false),
                    // Every mod of the group, wherever in the category it is.
                    GROUP_SIZE => QVariant::from(&(self.list.in_sub_of(first).len() as i32)),
                    NAME => text(&of.sub),
                    AUTHOR | VERSION | TAGS | TROUBLE_TEXT | PICTURE | LOAD_TIME | WHY => text(""),
                    LOCAL => QVariant::from(&false),
                    COLLAPSED => QVariant::from(&collapsed),
                    COUNT => QVariant::from(&(count as i32)),
                    CATEGORY => QVariant::from(&(of.category as i32)),
                    MOD_INDEX | MOD_ID => QVariant::from(&-1),
                    NUMBER | MOVED_FROM | TROUBLE | SOURCE => QVariant::from(&0),
                    LOAD_PART => QVariant::from(&0.0),
                    _ => QVariant::default(),
                }
            }
            Row::Header {
                category,
                count,
                collapsed,
            } => match role {
                IS_HEADER => QVariant::from(&true),
                IS_SUB => QVariant::from(&false),
                AGAIN => {
                    let earlier = &self.list.rows()[..index.row() as usize];
                    let same =
                        |r: &Row| matches!(r, Row::Header { category: c, .. } if *c == category);
                    QVariant::from(&earlier.iter().any(same))
                }
                IGNORED | FRESH | QUIET | TWIN => QVariant::from(&false),
                NAME => text(category.label()),
                AUTHOR | VERSION | TAGS | TROUBLE_TEXT | PICTURE | LOAD_TIME | WHY => text(""),
                LOCAL => QVariant::from(&false),
                NUMBER => QVariant::from(&0),
                COUNT => QVariant::from(&(count as i32)),
                COLLAPSED => QVariant::from(&collapsed),
                CATEGORY => QVariant::from(&(category as i32)),
                MOD_INDEX | MOD_ID => QVariant::from(&-1),
                MOVED_FROM | TROUBLE | SOURCE | GROUP_SIZE => QVariant::from(&0),
                LOAD_PART => QVariant::from(&0.0),
                _ => QVariant::default(),
            },
            Row::Mod { index } => {
                let m = &self.list.mods()[index];
                match role {
                    IS_HEADER | COLLAPSED | AGAIN => QVariant::from(&false),
                    IGNORED => {
                        QVariant::from(&self.version_ignored.get(index).copied().unwrap_or(false))
                    }
                    FRESH => QVariant::from(&self.is_new(m)),
                    // Another mod of this list goes by the same name.
                    TWIN => QVariant::from(&self.twins.get(index).copied().unwrap_or(false)),
                    WHY => text(self.why_moved(index)),
                    NAME => text(&m.name),
                    AUTHOR => text(&m.author),
                    VERSION => text(&m.version),
                    LOAD_TIME => {
                        // Under a millisecond is nothing worth a number.
                        let took = m.load_micros.filter(|micros| *micros >= 1000);
                        let ms = took.map(|micros| f64::from(micros) / 1000.0);
                        text(&ms.map(rimstack_core::startup::seconds).unwrap_or_default())
                    }
                    LOAD_PART => {
                        let part = match (m.load_micros, self.slowest) {
                            (Some(own), Some(most)) if most > 0 => f64::from(own) / f64::from(most),
                            _ => 0.0,
                        };
                        QVariant::from(&part)
                    }
                    // 0 Workshop, 1 local, 2 a Git checkout, 3 the game's own.
                    SOURCE => {
                        let loaded = crate::app().library();
                        let installed = match (&*loaded, m.installed) {
                            (Ok((library, _)), Some(i)) => library.installed.get(i),
                            _ => None,
                        };
                        use rimstack_core::paths::Source;
                        let source = installed.map_or(0, |i| match i.source {
                            Source::Official => 3,
                            Source::Local if i.git => 2,
                            Source::Local => 1,
                            Source::Workshop => 0,
                        });
                        QVariant::from(&source)
                    }
                    NUMBER => QVariant::from(&(index as i32 + 1)),
                    COUNT => QVariant::from(&0),
                    CATEGORY => QVariant::from(&(m.category as i32)),
                    MOD_INDEX => QVariant::from(&(index as i32)),
                    MOD_ID => QVariant::from(&m.installed.map_or(-1, |i| i as i32)),
                    GROUP_SIZE => QVariant::from(&0),
                    TAGS => text(&m.tags.join(", ")),
                    TROUBLE => {
                        let own = self
                            .troubles
                            .get(index)
                            .map(Vec::as_slice)
                            .unwrap_or_default();
                        let worst = own.iter().map(|t| if t.error { 2 } else { 1 }).max();
                        QVariant::from(&worst.unwrap_or(0))
                    }
                    // Its only trouble is the game version it lists.
                    QUIET => {
                        let own = self
                            .troubles
                            .get(index)
                            .map(Vec::as_slice)
                            .unwrap_or_default();
                        let version = |t: &Trouble| t.fix == rimstack_core::words::Fix::Trust;
                        QVariant::from(&(!own.is_empty() && own.iter().all(version)))
                    }
                    TROUBLE_TEXT => {
                        let own = self
                            .troubles
                            .get(index)
                            .map(Vec::as_slice)
                            .unwrap_or_default();
                        let first = own.iter().find(|t| t.error).or(own.first());
                        text(first.map_or("", |t| t.short.as_str()))
                    }
                    LOCAL => QVariant::from(&m.local),
                    PICTURE => {
                        let loaded = crate::app().library();
                        let installed = match (&*loaded, m.installed) {
                            (Ok((library, _)), Some(i)) => library.installed.get(i),
                            _ => None,
                        };
                        let picture = installed.and_then(|i| i.preview.as_ref());
                        text(&picture.map_or(String::new(), |p| p.to_string_lossy().into_owned()))
                    }
                    MOVED_FROM => {
                        let from = self.preview.as_ref().and_then(|p| p.moved_from.get(index));
                        QVariant::from(&from.map_or(0, |&n| n as i32))
                    }
                    _ => QVariant::default(),
                }
            }
        }
    }
}
