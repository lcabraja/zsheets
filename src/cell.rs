// Cell input component for editing spreadsheet cells
// Based on the TextInput from gpui-todos

use std::ops::Range;
use std::time::Duration;
use std::time::Instant;

use gpui::*;
use unicode_segmentation::*;

use crate::Theme;

const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(600);
const CURSOR_FADE_DURATION: Duration = Duration::from_millis(400);
const CURSOR_ANIMATION_STEP: Duration = Duration::from_millis(16); // ~60fps

/// Ease-in-out cubic function for smooth animation
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

actions!(
    cell_input,
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
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        DeleteToStart,
        DeleteWordBackward,
    ]
);

pub struct CellInput {
    pub focus_handle: FocusHandle,
    pub content: SharedString,
    pub selected_range: Range<usize>,
    pub selection_reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub last_layout: Option<ShapedLine>,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub is_selecting: bool,
    pub cursor_opacity: f32,
    pub cursor_fading_in: bool,
    pub blink_epoch: usize,
    pub fade_start: Option<Instant>,
    pub scroll_offset: Pixels,
}

impl CellInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            cursor_opacity: 1.0,
            cursor_fading_in: true,
            blink_epoch: 0,
            fade_start: None,
            scroll_offset: px(0.),
        }
    }

    /// Set the content of the cell input (used when entering edit mode)
    pub fn set_content(&mut self, text: String, cx: &mut Context<Self>) {
        let len = text.len();
        self.content = text.into();
        self.selected_range = len..len; // Cursor at end
        self.selection_reversed = false;
        self.marked_range = None;
        self.scroll_offset = px(0.);
        self.reset_cursor_blink(cx);
        cx.notify();
    }

    /// Get the content of the cell input (used when exiting edit mode)
    pub fn get_content(&self) -> String {
        self.content.to_string()
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }

    fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.next_word_boundary(self.cursor_offset()), cx);
    }

    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }

    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_word_boundary(self.cursor_offset()), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete_to_start(&mut self, _: &DeleteToStart, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(0, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete_word_backward(&mut self, _: &DeleteWordBackward, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_word_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;

        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            // Preserve newlines for multiline cell support
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                (&self.content[self.selected_range.clone()]).to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                (&self.content[self.selected_range.clone()]).to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn reset_cursor_blink(&mut self, cx: &mut Context<Self>) {
        self.cursor_opacity = 1.0;
        self.cursor_fading_in = true;
        self.fade_start = None;
        self.blink_epoch += 1;
        let epoch = self.blink_epoch;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            // Initial delay before first blink
            cx.background_executor().timer(CURSOR_BLINK_INTERVAL).await;

            loop {
                // Start fade animation
                let fading_in = this
                    .update(cx, |this, cx| {
                        if this.blink_epoch != epoch {
                            return None;
                        }
                        this.cursor_fading_in = !this.cursor_fading_in;
                        this.fade_start = Some(Instant::now());
                        cx.notify();
                        Some(this.cursor_fading_in)
                    })
                    .ok()
                    .flatten();

                let Some(fading_in) = fading_in else {
                    break;
                };

                // Animate the fade
                let fade_steps = (CURSOR_FADE_DURATION.as_millis() / CURSOR_ANIMATION_STEP.as_millis()) as usize;
                for _ in 0..fade_steps {
                    cx.background_executor().timer(CURSOR_ANIMATION_STEP).await;
                    let should_continue = this
                        .update(cx, |this, cx| {
                            if this.blink_epoch != epoch {
                                return false;
                            }
                            if let Some(start) = this.fade_start {
                                let elapsed = start.elapsed().as_secs_f32();
                                let progress = (elapsed / CURSOR_FADE_DURATION.as_secs_f32()).min(1.0);
                                let eased = ease_in_out_cubic(progress);
                                this.cursor_opacity = if fading_in { eased } else { 1.0 - eased };
                                cx.notify();
                            }
                            true
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        return;
                    }
                }

                // Ensure we reach the final state
                let should_continue = this
                    .update(cx, |this, cx| {
                        if this.blink_epoch != epoch {
                            return false;
                        }
                        this.cursor_opacity = if fading_in { 1.0 } else { 0.0 };
                        this.fade_start = None;
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !should_continue {
                    break;
                }

                // Wait before next blink cycle
                let remaining = CURSOR_BLINK_INTERVAL.saturating_sub(CURSOR_FADE_DURATION);
                if !remaining.is_zero() {
                    cx.background_executor().timer(remaining).await;
                }
            }
        })
        .detach();
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.reset_cursor_blink(cx);
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        // Account for scroll offset when calculating position
        line.closest_index_for_x(position.x - bounds.left() + self.scroll_offset)
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    fn previous_word_boundary(&self, offset: usize) -> usize {
        let mut prev_offset = offset;
        let mut found_word = false;

        for (idx, grapheme) in self.content.grapheme_indices(true).rev() {
            if idx >= offset {
                continue;
            }
            let is_word_char = grapheme.chars().next().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
            if is_word_char {
                found_word = true;
                prev_offset = idx;
            } else if found_word {
                // We've hit a non-word char after finding word chars
                break;
            } else {
                prev_offset = idx;
            }
        }

        if found_word { prev_offset } else { 0 }
    }

    fn next_word_boundary(&self, offset: usize) -> usize {
        let mut in_word = false;

        for (idx, grapheme) in self.content.grapheme_indices(true) {
            if idx <= offset {
                continue;
            }
            let is_word_char = grapheme.chars().next().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
            if is_word_char {
                in_word = true;
            } else if in_word {
                // We've hit a non-word char after being in a word
                return idx;
            }
        }

        self.content.len()
    }
}

impl EntityInputHandler for CellInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        self.reset_cursor_blink(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.marked_range = Some(range.start..range.start + new_text.len());
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;

        assert_eq!(last_layout.text, self.content);
        let utf8_index = last_layout.index_for_x(point.x - line_point.x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

/// Element for rendering the cell input text with cursor
pub struct CellInputElement {
    pub input: Entity<CellInput>,
}

pub struct CellInputPrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<(Bounds<Pixels>, Rgba)>,
    cursor_opacity: f32,
    selection: Option<PaintQuad>,
    scroll_offset: Pixels,
    vertical_offset: Pixels,
}

impl IntoElement for CellInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for CellInputElement {
    type RequestLayoutState = ();
    type PrepaintState = CellInputPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();
        let theme = cx.global::<Theme>();
        let mut scroll_offset = input.scroll_offset;

        let (display_text, text_color) = if content.is_empty() {
            ("".into(), style.color)
        } else {
            (content.clone(), style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run.clone()
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else if display_text.is_empty() {
            vec![]
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());

        // Calculate vertical offset to center on x-height rather than cap-height
        let font_id = window.text_system().resolve_font(&style.font());
        let cap_height = window.text_system().cap_height(font_id, font_size);
        let x_height = window.text_system().x_height(font_id, font_size);
        let vertical_offset = (cap_height - x_height) / 2.0;

        let line = if display_text.is_empty() {
            window.text_system().shape_line(" ".into(), font_size, &[TextRun {
                len: 1,
                font: style.font(),
                color: Hsla::transparent_black().into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }], None)
        } else {
            window.text_system().shape_line(display_text, font_size, &runs, None)
        };

        let cursor_pos = if content.is_empty() {
            px(0.)
        } else {
            line.x_for_index(cursor)
        };
        let cursor_opacity = input.cursor_opacity;

        // Calculate visible width (bounds width minus some padding for the cursor)
        let visible_width = bounds.size.width - px(2.);

        // Adjust scroll offset to keep cursor visible
        if cursor_pos - scroll_offset > visible_width {
            scroll_offset = cursor_pos - visible_width;
        }
        if cursor_pos < scroll_offset {
            scroll_offset = cursor_pos;
        }
        if scroll_offset < px(0.) {
            scroll_offset = px(0.);
        }

        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some((
                    Bounds::new(
                        point(bounds.left() + cursor_pos - scroll_offset, bounds.top()),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    theme.accent,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start) - scroll_offset,
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end) - scroll_offset,
                            bounds.bottom(),
                        ),
                    ),
                    rgba(0x3311ff30),
                )),
                None,
            )
        };

        CellInputPrepaintState {
            line: Some(line),
            cursor,
            cursor_opacity,
            selection,
            scroll_offset,
            vertical_offset,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        let scroll_offset = prepaint.scroll_offset;
        let vertical_offset = prepaint.vertical_offset;

        // Paint text with scroll offset applied, using calculated x-height centering offset
        let text_origin = point(bounds.origin.x - scroll_offset, bounds.origin.y + vertical_offset);
        line.paint(text_origin, window.line_height(), gpui::TextAlign::Left, None, window, cx)
            .unwrap();

        if focus_handle.is_focused(window) {
            if let Some((cursor_bounds, cursor_color)) = prepaint.cursor.take() {
                let opacity = prepaint.cursor_opacity;
                if opacity > 0.0 {
                    let hsla: Hsla = cursor_color.into();
                    let color_with_opacity = Hsla {
                        h: hsla.h,
                        s: hsla.s,
                        l: hsla.l,
                        a: opacity,
                    };
                    window.paint_quad(fill(cursor_bounds, color_with_opacity));
                }
            }
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
            input.scroll_offset = scroll_offset;
        });
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
}

impl Render for CellInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div()
            .flex()
            .key_context("CellInput")
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_to_start))
            .on_action(cx.listener(Self::delete_word_backward))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .bg(theme.surface0)
            .size_full()
            .overflow_hidden()
            .line_height(px(20.))
            .text_size(px(14.))
            .child(
                div()
                    .h(px(20.))
                    .w_full()
                    .overflow_hidden()
                    .px(px(4.))
                    .child(CellInputElement {
                        input: cx.entity().clone(),
                    }),
            )
    }
}

impl Focusable for CellInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
