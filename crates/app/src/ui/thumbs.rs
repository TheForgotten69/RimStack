//! Previews as small as their tiles, decoded off the window's thread. GPUI's `img` keeps a
//! picture whole; a list of 2,000 mods' previews held that way took 150 MB.

use gpui::{App as Cx, EntityId, RenderImage};
use image::imageops::FilterType;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

/// Pictures kept; a list scrolled through the rest decodes them again.
const KEPT: usize = 300;

#[derive(Default)]
pub struct Thumbs {
    shown: HashMap<(String, u32, u32), Arc<RenderImage>>,
    order: VecDeque<(String, u32, u32)>,
    asked: HashSet<(String, u32, u32)>,
}

pub type Shared = Rc<RefCell<Thumbs>>;

/// The picture at `path` cut to fill `width` by `height` device pixels, once it is decoded;
/// `shown_by` is drawn again when it is.
pub fn thumb(
    thumbs: &Shared,
    path: &str,
    (width, height): (u32, u32),
    shown_by: EntityId,
    cx: &mut Cx,
) -> Option<Arc<RenderImage>> {
    let key = (path.to_string(), width, height);
    if let Some(shown) = thumbs.borrow().shown.get(&key) {
        return Some(shown.clone());
    }
    if !thumbs.borrow_mut().asked.insert(key.clone()) {
        return None;
    }
    let (from, size) = (path.to_string(), (width, height));
    let decoding = cx
        .background_executor()
        .spawn(async move { decode(&from, size) });
    let thumbs = thumbs.clone();
    cx.spawn(async move |cx| {
        let made = decoding.await;
        let mut held = thumbs.borrow_mut();
        held.asked.remove(&key);
        if let Some(made) = made {
            held.shown.insert(key.clone(), Arc::new(made));
            held.order.push_back(key);
            while held.order.len() > KEPT {
                if let Some(old) = held.order.pop_front() {
                    held.shown.remove(&old);
                }
            }
        }
        drop(held);
        cx.update(|cx| cx.notify(shown_by));
    })
    .detach();
    None
}

fn decode(path: &str, (width, height): (u32, u32)) -> Option<RenderImage> {
    let read = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let mut shown = read
        .decode()
        .ok()?
        .resize_to_fill(width, height, FilterType::Triangle)
        .to_rgba8();
    // GPUI draws BGRA.
    for pixel in shown.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Some(RenderImage::new(smallvec::smallvec![image::Frame::new(
        shown
    )]))
}
