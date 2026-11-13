//! Previews decoded off the window's thread at the size they are shown, kept in memory up to a
//! size. GPUI's `img` keeps a picture whole: 2,000 mods' previews held that way took 150 MB.

use gpui::{App as Cx, EntityId, RenderImage};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Bytes of decoded pictures kept: about 1,300 tiles at 2x, more than a list shows at once.
const MOST_BYTES: usize = 48 << 20;
/// Decodes running at once.
const DECODING: usize = 4;
/// Pictures waiting to be decoded; past it the oldest asked, scrolled by, are dropped.
const WAITING: usize = 64;

type Size = (u32, u32);
type Key = (Size, String);

#[derive(Default)]
pub struct Thumbs {
    shown: HashMap<Key, Arc<RenderImage>>,
    /// Oldest first, for dropping past `MOST_BYTES`.
    order: VecDeque<Key>,
    bytes: usize,
    /// Pictures that could not be read: not tried again until `renew`.
    unreadable: HashSet<Key>,
    /// Newest asked last: the rows in view are decoded first.
    waiting: Vec<(Key, EntityId)>,
    asked: HashSet<Key>,
    decoding: usize,
}

pub type Shared = Rc<RefCell<Thumbs>>;

impl Thumbs {
    /// The pictures may have changed on disk: each is read again when next shown.
    pub fn renew(&mut self) {
        self.shown.clear();
        self.order.clear();
        self.bytes = 0;
        self.unreadable.clear();
    }

    fn keep(&mut self, key: Key, shown: Arc<RenderImage>) {
        self.bytes += tile_bytes(key.0);
        self.order.push_back(key.clone());
        self.shown.insert(key, shown);
        while self.bytes > MOST_BYTES {
            let Some(old) = self.order.pop_front() else {
                break;
            };
            if self.shown.remove(&old).is_some() {
                self.bytes -= tile_bytes(old.0);
            }
        }
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
    if let Some(shown) = held.shown.get(&key) {
        return Some(shown.clone());
    }
    if held.unreadable.contains(&key) || !held.asked.insert(key.clone()) {
        return None;
    }
    held.waiting.push((key, shown_by));
    if held.waiting.len() > WAITING {
        let (dropped, _) = held.waiting.remove(0);
        held.asked.remove(&dropped);
    }
    drop(held);
    decode_next(thumbs, cx);
    None
}

fn decode_next(thumbs: &Shared, cx: &mut Cx) {
    let mut held = thumbs.borrow_mut();
    if held.decoding >= DECODING {
        return;
    }
    let Some((key, shown_by)) = held.waiting.pop() else {
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
        {
            let mut held = thumbs.borrow_mut();
            held.decoding -= 1;
            held.asked.remove(&key);
            match made.and_then(|pixels| picture(pixels, size)) {
                Some(made) => held.keep(key, Arc::new(made)),
                None => {
                    held.unreadable.insert(key);
                }
            }
        }
        cx.update(|cx| {
            cx.notify(shown_by);
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
}
