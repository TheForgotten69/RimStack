use super::view::DetailsShown;
use super::*;

impl qobject::WorkshopData {
    pub(super) fn reopen_details(self: Pin<&mut Self>) {
        let id = self.selected.clone();
        if !id.is_empty() {
            self.open_details(id);
        }
    }

    pub(super) fn open_details(self: Pin<&mut Self>, id: String) {
        let app = self.app();
        self.opening.run(
            "workshop details",
            self.qt_thread(),
            move |_| app.browse_details(&id),
            |mut me, shown| {
                me.as_mut().set_details_busy(false);
                me.show_details(shown);
            },
        );
    }

    pub(super) fn show_details(mut self: Pin<&mut Self>, shown: Option<Shown>) {
        let url = shown.as_ref().map(|s| s.preview_url.clone());
        let shots = shown.as_ref().map(|s| s.screenshot_urls.clone());
        let pictures = shown.as_ref().map(Shown::picture_urls);
        let me = self.as_mut().rust_mut().get_mut();
        me.shown = shown;
        if let Some(url) = url.filter(|url| *url != me.detail_preview.0) {
            me.detail_preview = (url.clone(), String::new());
            self.as_mut().fetch_detail_preview(url);
        }
        let unfetched =
            |urls: &Vec<String>| !urls.iter().all(|u| self.detail_images.contains_key(u));
        if let Some(urls) = pictures.filter(unfetched) {
            self.as_mut().fetch_images(urls);
        }
        if let Some(urls) = shots.filter(|urls| *urls != self.detail_shots.0) {
            self.as_mut().rust_mut().get_mut().detail_shots = (urls.clone(), Vec::new());
            self.as_mut().fetch_screenshots(urls);
        }
        self.render_details();
    }

    pub(super) fn fetch_detail_preview(self: Pin<&mut Self>, url: String) {
        let app = self.app();
        let asked = url.clone();
        self.detail_previewing.run(
            "workshop preview",
            self.qt_thread(),
            move |_| app.browse_preview(&asked),
            move |mut me, found| match found {
                Ok(Some(path)) if me.detail_preview.0 == url => {
                    me.as_mut().rust_mut().get_mut().detail_preview.1 =
                        path.to_string_lossy().into_owned();
                    me.render_details();
                }
                Ok(_) => {}
                Err(problem) => rimstack_core::problem!("{problem}"),
            },
        );
    }

    pub(super) fn fetch_images(self: Pin<&mut Self>, urls: Vec<String>) {
        let app = self.app();
        let asked = urls.clone();
        self.imaging.run(
            "workshop description pictures",
            self.qt_thread(),
            move |_| app.browse_images(&asked),
            move |mut me, taken| {
                let current = me.shown.as_ref().map(Shown::picture_urls);
                if current.as_ref() == Some(&urls) {
                    let kept = &mut me.as_mut().rust_mut().get_mut().detail_images;
                    for (url, path) in taken {
                        if let Some(path) = path {
                            kept.insert(url, path.to_string_lossy().into_owned());
                        }
                    }
                    me.render_details();
                }
            },
        );
    }

    pub(super) fn fetch_screenshots(self: Pin<&mut Self>, urls: Vec<String>) {
        if urls.is_empty() {
            self.shooting.cancel();
            return;
        }
        let app = self.app();
        let asked = urls.clone();
        self.shooting.run(
            "workshop screenshots",
            self.qt_thread(),
            move |turn| {
                let mut paths = Vec::new();
                for url in asked.iter().take(SCREENSHOTS) {
                    if !turn.current() {
                        break;
                    }
                    match app.browse_preview(url) {
                        Ok(found) => paths.extend(found),
                        Err(problem) => rimstack_core::problem!("{problem}"),
                    }
                }
                paths
            },
            move |mut me, paths| {
                if me.detail_shots.0 == urls {
                    me.as_mut().rust_mut().get_mut().detail_shots.1 = paths
                        .iter()
                        .map(|path| path.to_string_lossy().into_owned())
                        .collect();
                    me.render_details();
                }
            },
        );
    }

    pub(super) fn render_details(mut self: Pin<&mut Self>) {
        let value = match &self.shown {
            Some(shown) => {
                let (url, path) = &self.detail_preview;
                let preview_path = if *url == shown.preview_url { path } else { "" };
                let (urls, files) = &self.detail_shots;
                let screenshots = if *urls == shown.screenshot_urls {
                    files.as_slice()
                } else {
                    &[]
                };
                to_qml(&DetailsShown {
                    shown,
                    preview_path,
                    description: shown.description(&self.detail_images),
                    screenshots,
                })
            }
            None => QVariant::default(),
        };
        self.as_mut().set_details(value);
    }
}
