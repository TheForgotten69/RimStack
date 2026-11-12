//! The text a field holds, as `TextInput` kept it: one line, a selection, and the text an
//! input method is still composing.

use super::theme::Theme;
use gpui::{
    App as Cx, Bounds, ClipboardItem, Context, ElementInputHandler, EntityInputHandler,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, KeyBinding,
    ParentElement, Pixels, Point, Render, ShapedLine, SharedString, Styled, TextRun,
    UTF16Selection, Window, actions, canvas, div, fill, point, px, size,
};
use std::ops::Range;

actions!(
    text_field,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Paste,
        Copy,
        Cut,
        Escape
    ]
);

const CONTEXT: &str = "TextField";

/// A key held with Command on macOS and Ctrl elsewhere, as GPUI names it.
#[macro_export]
macro_rules! command {
    ($key:literal) => {
        if cfg!(target_os = "macos") {
            concat!("cmd-", $key)
        } else {
            concat!("ctrl-", $key)
        }
    };
}

pub fn bind_keys(cx: &mut Cx) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("left", Left, Some(CONTEXT)),
        KeyBinding::new("right", Right, Some(CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(CONTEXT)),
        KeyBinding::new(crate::command!("a"), SelectAll, Some(CONTEXT)),
        KeyBinding::new("home", Home, Some(CONTEXT)),
        KeyBinding::new("end", End, Some(CONTEXT)),
        KeyBinding::new(crate::command!("v"), Paste, Some(CONTEXT)),
        KeyBinding::new(crate::command!("c"), Copy, Some(CONTEXT)),
        KeyBinding::new(crate::command!("x"), Cut, Some(CONTEXT)),
        KeyBinding::new("escape", Escape, Some(CONTEXT)),
    ]);
}

/// Told after each change of the text.
pub struct Edited;

pub struct TextField {
    focus: FocusHandle,
    text: String,
    selected: Range<usize>,
    reversed: bool,
    marked: Option<Range<usize>>,
    shaped: Option<(ShapedLine, Bounds<Pixels>)>,
    size: Pixels,
}

impl EventEmitter<Edited> for TextField {}

impl Focusable for TextField {
    fn focus_handle(&self, _: &Cx) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextField {
    pub fn new(size: Pixels, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            text: String::new(),
            selected: 0..0,
            reversed: false,
            marked: None,
            shaped: None,
            size,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.text = text.to_string();
        self.selected = self.text.len()..self.text.len();
        self.marked = None;
        cx.emit(Edited);
        cx.notify();
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }

    fn cursor(&self) -> usize {
        if self.reversed {
            self.selected.start
        } else {
            self.selected.end
        }
    }

    fn move_to(&mut self, at: usize, cx: &mut Context<Self>) {
        self.selected = at..at;
        self.reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, at: usize, cx: &mut Context<Self>) {
        match self.reversed {
            true => self.selected.start = at,
            false => self.selected.end = at,
        }
        if self.selected.end < self.selected.start {
            self.reversed = !self.reversed;
            self.selected = self.selected.end..self.selected.start;
        }
        cx.notify();
    }

    fn previous(&self, at: usize) -> usize {
        self.text[..at]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    fn next(&self, at: usize) -> usize {
        self.text[at..]
            .chars()
            .next()
            .map_or(at, |c| at + c.len_utf8())
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let at = if self.selected.is_empty() {
            self.previous(self.cursor())
        } else {
            self.selected.start
        };
        self.move_to(at, cx);
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let at = if self.selected.is_empty() {
            self.next(self.cursor())
        } else {
            self.selected.end
        };
        self.move_to(at, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous(self.cursor()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next(self.cursor()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected = 0..self.text.len();
        self.reversed = false;
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.text.len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.select_to(self.previous(self.cursor()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.select_to(self.next(self.cursor()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace('\n', " "), window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.text[self.selected.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.copy(&Copy, window, cx);
        self.replace_text_in_range(None, "", window, cx);
    }

    fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        self.set_text("", cx);
    }

    fn to_utf16(&self, offset: usize) -> usize {
        self.text[..offset.min(self.text.len())]
            .encode_utf16()
            .count()
    }

    fn from_utf16(&self, offset: usize) -> usize {
        let mut counted = 0;
        for (at, c) in self.text.char_indices() {
            if counted >= offset {
                return at;
            }
            counted += c.len_utf16();
        }
        self.text.len()
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.to_utf16(range.start)..self.to_utf16(range.end)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.from_utf16(range.start)..self.from_utf16(range.end)
    }

    fn shape(&self, window: &Window, theme: &Theme) -> ShapedLine {
        let run = TextRun {
            len: self.text.len(),
            font: gpui::font(theme.family.clone()),
            color: theme.colours.ink,
            background_color: None,
            underline: self.marked.as_ref().map(|_| gpui::UnderlineStyle {
                thickness: px(1.),
                color: Some(theme.colours.ink),
                wavy: false,
            }),
            strikethrough: None,
        };
        window.text_system().shape_line(
            SharedString::from(self.text.clone()),
            self.size,
            &[run],
            None,
        )
    }
}

impl EntityInputHandler for TextField {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range);
        adjusted.replace(self.range_to_utf16(&range));
        Some(self.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected),
            reversed: self.reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked = None;
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked.clone())
            .unwrap_or(self.selected.clone());
        self.text = format!(
            "{}{new}{}",
            &self.text[..range.start],
            &self.text[range.end..]
        );
        let at = range.start + new.len();
        self.selected = at..at;
        self.reversed = false;
        self.marked = None;
        cx.emit(Edited);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked.clone())
            .unwrap_or(self.selected.clone());
        self.text = format!(
            "{}{new}{}",
            &self.text[..range.start],
            &self.text[range.end..]
        );
        self.marked = (!new.is_empty()).then(|| range.start..range.start + new.len());
        self.selected = selected
            .as_ref()
            .map(|inside| range.start + inside.start..range.start + inside.end)
            .unwrap_or(range.start + new.len()..range.start + new.len());
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let (shaped, _) = self.shaped.as_ref()?;
        let range = self.range_from_utf16(&range);
        Some(Bounds::from_corners(
            point(
                bounds.left() + shaped.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + shaped.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        at: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let (shaped, bounds) = self.shaped.as_ref()?;
        let index = shaped
            .index_for_x(at.x - bounds.left())
            .unwrap_or(self.text.len());
        Some(self.to_utf16(index))
    }
}

impl Render for TextField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let field = cx.entity();
        let focus = self.focus.clone();
        let shaped = self.shape(window, &theme);
        let (selected, cursor, focused) = (
            self.selected.clone(),
            self.cursor(),
            focus.is_focused(window),
        );
        let (ink, accent) = (theme.colours.ink, theme.colours.accent);
        let line = theme.line(self.size);
        div()
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::escape))
            .w_full()
            .h(line)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, cx| {
                        let top = bounds.top() + (bounds.size.height - line) / 2.;
                        if !selected.is_empty() {
                            let from = bounds.left() + shaped.x_for_index(selected.start);
                            let to = bounds.left() + shaped.x_for_index(selected.end);
                            window.paint_quad(fill(
                                Bounds::from_corners(point(from, top), point(to, top + line)),
                                accent,
                            ));
                        }
                        let _ = shaped.paint(
                            point(bounds.left(), top),
                            line,
                            gpui::TextAlign::Left,
                            None,
                            window,
                            cx,
                        );
                        if focused && selected.is_empty() {
                            let x = bounds.left() + shaped.x_for_index(cursor);
                            window.paint_quad(fill(
                                Bounds::new(point(x, top), size(px(1.), line)),
                                ink,
                            ));
                        }
                        let kept = field.clone();
                        kept.update(cx, |field, _| field.shaped = Some((shaped, bounds)));
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, field.clone()),
                            cx,
                        );
                    },
                )
                .size_full(),
            )
    }
}
