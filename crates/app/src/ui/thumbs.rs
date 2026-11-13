//! Previews decoded off the window's thread at the size they are shown, kept in memory up to a
//! size. GPUI's `img` keeps a picture whole: 2,000 mods' previews held that way took 150 MB.

use gpui::{App as Cx, EntityId, RenderImage};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Bytes of decoded pictures kept: about 1,300 tiles at 2x, more than a list shows at once.
const MOST_BYTES: usize = 48 << 20;
/// Decodes running at once.
const DECODING: usize = 4;
/// Pictures waiting to be decoded, the rows in view and three pages ahead of them; past it the
/// oldest asked, scrolled by, are dropped.
const WAITING: usize = 128;

type Size = (u32, u32);
type Key = (Size, String);

#[derive(Default)]
pub struct Thumbs {
    /// Each with when it was last shown.
    shown: HashMap<Key, (Arc<RenderImage>, u64)>,
    /// Least lately shown first, for dropping past `MOST_BYTES`.
    by_use: BTreeMap<u64, Key>,
    uses: u64,
    bytes: usize,
    /// Pictures that could not be read: not tried again until `renew`.
    unreadable: HashSet<Key>,
    /// Decoded from the end: the rows in view are pushed there, those ahead of them put first.
    waiting: Vec<Key>,
    /// Asked and not decoded yet, with the view to draw again once it is, if one shows it.
    asked: HashMap<Key, Option<EntityId>>,
    decoding: usize,
}

pub type Shared = Rc<RefCell<Thumbs>>;

impl Thumbs {
    /// The pictures may have changed on disk: each is read again when next shown.
    pub fn renew(&mut self) {
        self.shown.clear();
        self.by_use.clear();
        self.bytes = 0;
        self.unreadable.clear();
    }

    fn used(&mut self) -> u64 {
        self.uses += 1;
        self.uses
    }

    fn hit(&mut self, key: &Key) -> Option<Arc<RenderImage>> {
        let now = self.uses + 1;
        let (shown, at) = self.shown.get_mut(key)?;
        let was = std::mem::replace(at, now);
        let shown = shown.clone();
        self.uses = now;
        if let Some(key) = self.by_use.remove(&was) {
            self.by_use.insert(now, key);
        }
        Some(shown)
    }

    fn keep(&mut self, key: Key, shown: Arc<RenderImage>) {
        let at = self.used();
        let bytes = tile_bytes(key.0);
        self.by_use.insert(at, key.clone());
        match self.shown.insert(key, (shown, at)) {
            Some((_, was)) => {
                self.by_use.remove(&was);
            }
            None => self.bytes += bytes,
        }
        while self.bytes > MOST_BYTES {
            let Some((_, old)) = self.by_use.pop_first() else {
                break;
            };
            if self.shown.remove(&old).is_some() {
                self.bytes -= tile_bytes(old.0);
            }
        }
    }

    /// `key` asked for a row in view, which `shown_by` draws: ahead of the rest.
    fn ask_shown(&mut self, key: Key, shown_by: EntityId) {
        if let Some(shows) = self.asked.get_mut(&key) {
            *shows = Some(shown_by);
            if let Some(at) = self.waiting.iter().position(|waits| *waits == key) {
                let key = self.waiting.remove(at);
                self.waiting.push(key);
            }
            return;
        }
        self.asked.insert(key.clone(), Some(shown_by));
        self.waiting.push(key);
        if self.waiting.len() > WAITING {
            let dropped = self.waiting.remove(0);
            self.asked.remove(&dropped);
        }
    }

    /// `key` asked for a row about to be scrolled to: after every row in view, and not at all
    /// when as many wait already.
    fn ask_ahead(&mut self, key: Key) -> bool {
        if self.waiting.len() >= WAITING
            || self.shown.contains_key(&key)
            || self.unreadable.contains(&key)
            || self.asked.contains_key(&key)
        {
            return false;
        }
        self.asked.insert(key.clone(), None);
        self.waiting.insert(0, key);
        true
    }
}

#[cfg(all(test, target_os = "linux"))]
impl Thumbs {
    /// Pictures asked for rows in view and not decoded yet.
    pub(crate) fn awaited_in_view(&self) -> usize {
        self.asked.values().filter(|shows| shows.is_some()).count()
    }

    pub(crate) fn decoded(&self) -> usize {
        self.shown.len()
    }
}

fn tile_bytes((width, height): Size) -> usize {
    width as usize * height as usize * 4
}

/// The picture at `path` cut to fill `size` device pixels, once decoded; `shown_by` is drawn
/// again when it is.
pub fn thumb(
    thumbs: &Shared,
    path: &str,
    size: Size,
    shown_by: EntityId,
    cx: &mut Cx,
) -> Option<Arc<RenderImage>> {
    let key = (size, path.to_string());
    let mut held = thumbs.borrow_mut();
    if let Some(shown) = held.hit(&key) {
        return Some(shown);
    }
    if held.unreadable.contains(&key) {
        return None;
    }
    held.ask_shown(key, shown_by);
    drop(held);
    decode_next(thumbs, cx);
    None
}

/// The picture at `path` decoded before its row is in view, so it shows with the row.
pub fn ahead(thumbs: &Shared, path: &str, size: Size, cx: &mut Cx) {
    let asked = thumbs.borrow_mut().ask_ahead((size, path.to_string()));
    if asked {
        decode_next(thumbs, cx);
    }
}

fn decode_next(thumbs: &Shared, cx: &mut Cx) {
    let mut held = thumbs.borrow_mut();
    if held.decoding >= DECODING {
        return;
    }
    let Some(key) = held.waiting.pop() else {
        return;
    };
    held.decoding += 1;
    drop(held);
    let (size, from) = (key.0, PathBuf::from(&key.1));
    let decoding = cx
        .background_executor()
        .spawn(async move { rimstack_core::base::thumbnail::crop(&from, size) });
    let thumbs = thumbs.clone();
    cx.spawn(async move |cx| {
        let made = decoding.await;
        let shown_by = {
            let mut held = thumbs.borrow_mut();
            held.decoding -= 1;
            let shown_by = held.asked.remove(&key).flatten();
            match made.and_then(|pixels| picture(pixels, size)) {
                Some(made) => held.keep(key, Arc::new(made)),
                None => {
                    held.unreadable.insert(key);
                }
            }
            shown_by
        };
        cx.update(|cx| {
            if let Some(view) = shown_by {
                cx.notify(view);
            }
            decode_next(&thumbs, cx);
        });
    })
    .detach();
}

fn picture(pixels: Vec<u8>, (width, height): Size) -> Option<RenderImage> {
    let frame = image::RgbaImage::from_raw(width, height, pixels)?;
    Some(RenderImage::new(smallvec::smallvec![image::Frame::new(
        frame
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_oldest_pictures_go_past_the_most_bytes() {
        let mut thumbs = Thumbs::default();
        let size = (512, 512);
        let fits = MOST_BYTES / tile_bytes(size);
        for at in 0..fits + 3 {
            let made = picture(vec![0; tile_bytes(size)], size).unwrap();
            thumbs.keep((size, at.to_string()), Arc::new(made));
        }
        assert!(thumbs.bytes <= MOST_BYTES);
        assert_eq!(thumbs.shown.len(), fits);
        assert!(!thumbs.shown.contains_key(&(size, "0".to_string())));
        assert!(thumbs.shown.contains_key(&(size, (fits + 2).to_string())));
    }

    #[test]
    fn a_picture_shown_again_is_kept_over_older_ones() {
        let mut thumbs = Thumbs::default();
        let size = (512, 512);
        let fits = MOST_BYTES / tile_bytes(size);
        let key = |at: usize| (size, at.to_string());
        for at in 0..fits {
            let made = picture(vec![0; tile_bytes(size)], size).unwrap();
            thumbs.keep(key(at), Arc::new(made));
        }
        assert!(thumbs.hit(&key(0)).is_some());
        let made = picture(vec![0; tile_bytes(size)], size).unwrap();
        thumbs.keep(key(fits), Arc::new(made));
        assert!(thumbs.shown.contains_key(&key(0)));
        assert!(!thumbs.shown.contains_key(&key(1)));
        assert_eq!(thumbs.by_use.len(), thumbs.shown.len());
    }

    #[test]
    fn rows_ahead_wait_behind_the_rows_in_view_and_gain_a_view_once_shown() {
        let mut cx = gpui::HeadlessAppContext::with_asset_source(
            crate::ui::made_up::text_system(),
            Arc::new(crate::ui::assets::Assets),
        );
        let view = cx.update(|cx| gpui::AppContext::new(cx, |_| ()).entity_id());
        let mut thumbs = Thumbs::default();
        let key = |name: &str| ((4, 4), name.to_string());
        assert!(thumbs.ask_ahead(key("ahead")));
        thumbs.ask_shown(key("in view"), view);
        assert!(!thumbs.ask_ahead(key("in view")));
        assert_eq!(thumbs.waiting.last(), Some(&key("in view")));
        assert_eq!(thumbs.asked[&key("ahead")], None);
        thumbs.ask_shown(key("ahead"), view);
        assert_eq!(thumbs.waiting.last(), Some(&key("ahead")));
        assert_eq!(thumbs.asked[&key("ahead")], Some(view));
    }
}
