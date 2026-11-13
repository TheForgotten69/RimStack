//! Previews read from a pack of copies at the size they are shown (`base::picture_pack`), in
//! the frame their row appears. The pack is filled off the window's thread with every picture
//! the view may show, so a scroll asks for nothing.

use gpui::{App as Cx, EntityId, RenderImage};
use rimstack_core::base::picture_pack::{Found, Pack};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Pictures kept decoded; the rest are read from the pack again.
const KEPT: usize = 300;

type Size = (u32, u32);

/// Every picture a view may show, read off the window's thread.
pub type Wanted = Arc<dyn Fn() -> Vec<PathBuf> + Send + Sync>;

pub struct Thumbs {
    set: &'static str,
    wanted: Wanted,
    packs: HashMap<Size, Arc<Pack>>,
    /// Sizes whose pack was filled, or is being filled, for what is wanted now.
    asked: HashSet<Size>,
    shown: HashMap<Size, HashMap<String, Arc<RenderImage>>>,
    order: VecDeque<(Size, String)>,
}

pub type Shared = Rc<RefCell<Thumbs>>;

pub fn shared(set: &'static str, wanted: Wanted) -> Shared {
    Rc::new(RefCell::new(Thumbs {
        set,
        wanted,
        packs: HashMap::new(),
        asked: HashSet::new(),
        shown: HashMap::new(),
        order: VecDeque::new(),
    }))
}

impl Thumbs {
    /// The pictures may have changed: each size's pack is filled again at its next use, and
    /// shown from the one there meanwhile.
    pub fn renew(&mut self) {
        self.asked.clear();
        self.shown.clear();
        self.order.clear();
    }

    pub fn want(&mut self, wanted: Wanted) {
        self.wanted = wanted;
        self.renew();
    }

    fn keep(&mut self, size: Size, path: &str, shown: Arc<RenderImage>) {
        self.shown
            .entry(size)
            .or_default()
            .insert(path.to_string(), shown);
        self.order.push_back((size, path.to_string()));
        while self.order.len() > KEPT {
            if let Some((size, old)) = self.order.pop_front()
                && let Some(of_size) = self.shown.get_mut(&size)
            {
                of_size.remove(&old);
            }
        }
    }
}

/// The picture at `path` cut to fill `size` device pixels, once the pack holds it; `shown_by`
/// is drawn again when the pack is filled.
pub fn thumb(
    thumbs: &Shared,
    path: &str,
    size: Size,
    shown_by: EntityId,
    cx: &mut Cx,
) -> Option<Arc<RenderImage>> {
    let (kept, pack, asked) = {
        let held = thumbs.borrow();
        let kept = held.shown.get(&size).and_then(|of| of.get(path)).cloned();
        (
            kept,
            held.packs.get(&size).cloned(),
            held.asked.contains(&size),
        )
    };
    if !asked {
        fill(thumbs, size, shown_by, cx);
    }
    if kept.is_some() {
        return kept;
    }
    let Found::Pixels(pixels) = pack?.found(path) else {
        return None;
    };
    let shown = Arc::new(picture(pixels, size)?);
    thumbs.borrow_mut().keep(size, path, shown.clone());
    Some(shown)
}

fn fill(thumbs: &Shared, size: Size, shown_by: EntityId, cx: &mut Cx) {
    let (set, wanted) = {
        let mut held = thumbs.borrow_mut();
        held.asked.insert(size);
        (held.set, held.wanted.clone())
    };
    let app = crate::ui::app(cx);
    let waiting =
        crate::jobs::off_thread("previews", move || app.fill_pictures(set, size, &wanted()));
    let thumbs = thumbs.clone();
    cx.spawn(async move |cx| {
        match waiting.await {
            Ok(Ok(pack)) => {
                let mut held = thumbs.borrow_mut();
                held.packs.insert(size, Arc::new(pack));
                held.shown.remove(&size);
            }
            Ok(Err(problem)) => rimstack_core::problem!("{problem}"),
            Err(_) => return,
        }
        cx.update(|cx| cx.notify(shown_by));
    })
    .detach();
}

fn picture(pixels: Vec<u8>, (width, height): Size) -> Option<RenderImage> {
    let frame = image::RgbaImage::from_raw(width, height, pixels)?;
    Some(RenderImage::new(smallvec::smallvec![image::Frame::new(
        frame
    )]))
}
