//! The text of `DescriptionView.qml`: a Workshop description's blocks drawn with their styles,
//! as Qt's rich text drew the HTML they came from; a link pressed is handed to `link`.

use super::theme::{self, Theme};
use super::widgets::{TextSized, with_alpha};
use gpui::{
    AnyElement, App, ElementId, FontStyle, FontWeight, HighlightStyle, InteractiveText,
    IntoElement, ParentElement, Pixels, StrikethroughStyle, Styled, StyledText, UnderlineStyle,
    Window, div, px,
};
use rimstack_core::base::markup::{Block, Kind, Run};
use std::ops::Range;
use std::rc::Rc;

pub type Link = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// Qt indents a list and a quote by 40 at the text size it was made for.
fn indent(theme: &Theme) -> Pixels {
    theme.sized(40.)
}

fn styled(theme: &Theme, run: &Run) -> HighlightStyle {
    let c = &theme.colours;
    let line = |colour| UnderlineStyle {
        thickness: px(1.),
        color: Some(colour),
        wavy: false,
    };
    HighlightStyle {
        color: run.link.as_ref().map(|_| c.focus),
        font_weight: run.bold.then_some(FontWeight::BOLD),
        font_style: run.italic.then_some(FontStyle::Italic),
        underline: match (run.link.is_some(), run.underline) {
            (true, _) => Some(line(c.focus)),
            (false, true) => Some(line(c.ink)),
            (false, false) => None,
        },
        strikethrough: run.strike.then_some(StrikethroughStyle {
            thickness: px(1.),
            color: Some(c.ink),
        }),
        ..HighlightStyle::default()
    }
}

/// A block's words, styled, its links pressed.
fn words(id: ElementId, theme: &Theme, block: &Block, link: &Link) -> AnyElement {
    let mut text = String::new();
    let mut marks = Vec::new();
    let mut links: Vec<(Range<usize>, String)> = Vec::new();
    for run in &block.runs {
        let start = text.len();
        text += &run.text;
        let range = start..text.len();
        let style = styled(theme, run);
        if style != HighlightStyle::default() {
            marks.push((range.clone(), style));
        }
        if let Some(url) = &run.link {
            links.push((range, url.clone()));
        }
    }
    let shown = StyledText::new(text).with_highlights(marks);
    if links.is_empty() {
        return shown.into_any_element();
    }
    let (ranges, urls): (Vec<_>, Vec<_>) = links.into_iter().unzip();
    let link = link.clone();
    InteractiveText::new(id, shown)
        .on_click(ranges, move |at, window, cx| link(&urls[at], window, cx))
        .into_any_element()
}

/// The blocks, one element each, in the body size.
pub fn blocks(id: &'static str, theme: &Theme, blocks: &[Block], link: &Link) -> Vec<AnyElement> {
    let c = &theme.colours;
    blocks
        .iter()
        .enumerate()
        .map(|(at, block)| {
            let shown = words((id, at).into(), theme, block, link);
            let text = div()
                .w_full()
                .sized_text(theme, theme.body())
                .text_color(c.ink);
            match block.kind {
                Kind::Paragraph => text.child(shown),
                Kind::Heading(level) => {
                    let size = match level {
                        3 => theme.body() * 1.2,
                        _ => theme.body(),
                    };
                    text.sized_text(theme, size)
                        .font_weight(FontWeight::BOLD)
                        .pt(theme::GAP_SMALL)
                        .child(shown)
                }
                Kind::Item { depth, number } => {
                    let mark = match (number, depth) {
                        (Some(n), _) => format!("{n}."),
                        (None, 1) => "•".into(),
                        (None, 2) => "◦".into(),
                        (None, _) => "▪".into(),
                    };
                    text.flex()
                        .pl(indent(theme) * (depth - 1) as f32)
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .justify_end()
                                .w(indent(theme))
                                .pr(theme::GAP)
                                .child(mark),
                        )
                        .child(div().flex_1().min_w_0().child(shown))
                }
                Kind::Quote => text.pl(indent(theme)).pr(indent(theme)).child(shown),
                Kind::Code => text
                    .font_family(theme.mono.clone())
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(shown),
                Kind::Rule => div()
                    .w_full()
                    .py(theme::GAP_SMALL)
                    .child(div().h(px(1.)).bg(with_alpha(c.ink3, 0.6))),
            }
            .into_any_element()
        })
        .collect()
}
