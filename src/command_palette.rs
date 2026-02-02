use std::ops::Range;
use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::Theme;

actions!(
    command_palette,
    [
        ShowCommandPalette,
        HideCommandPalette,
        SelectNext,
        SelectPrevious,
        Confirm,
    ]
);

/// A command that can be executed from the palette
#[derive(Clone, Debug)]
pub struct Command {
    pub id: &'static str,
    pub name: &'static str,
    pub shortcut: Option<&'static str>,
    pub vim_alias: Option<&'static str>,
}

impl Command {
    pub const fn new(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            shortcut: None,
            vim_alias: None,
        }
    }

    pub const fn with_shortcut(mut self, shortcut: &'static str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub const fn with_vim(mut self, alias: &'static str) -> Self {
        self.vim_alias = Some(alias);
        self
    }
}

/// Result of parsing a vim command
#[derive(Clone, Debug)]
pub enum VimCommand {
    /// :w - save current file
    Write,
    /// :w <path> - save to path
    WriteTo(PathBuf),
    /// :w! - force write (ignores read-only)
    ForceWrite,
    /// :wq - write and quit
    WriteQuit,
    /// :q - quit
    Quit,
    /// :q! - force quit (discard changes)
    ForceQuit,
    /// :e <path> - open file for editing
    Edit(PathBuf),
    /// :view <path> or :vi <path> - open file read-only
    View(PathBuf),
    /// :saveas <path> - save as
    SaveAs(PathBuf),
    /// :new - new file
    New,
}

impl VimCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if !input.starts_with(':') {
            return None;
        }

        let input = &input[1..]; // Remove leading ':'
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0];
        let arg = parts.get(1).map(|s| s.trim());

        match cmd {
            "w" if arg.is_none() => Some(VimCommand::Write),
            "w" if arg.is_some() => Some(VimCommand::WriteTo(PathBuf::from(arg.unwrap()))),
            "w!" => Some(VimCommand::ForceWrite),
            "wq" => Some(VimCommand::WriteQuit),
            "q" => Some(VimCommand::Quit),
            "q!" => Some(VimCommand::ForceQuit),
            "e" | "edit" if arg.is_some() => Some(VimCommand::Edit(PathBuf::from(arg.unwrap()))),
            "vi" | "view" if arg.is_some() => Some(VimCommand::View(PathBuf::from(arg.unwrap()))),
            "saveas" if arg.is_some() => Some(VimCommand::SaveAs(PathBuf::from(arg.unwrap()))),
            "new" => Some(VimCommand::New),
            _ => None,
        }
    }
}

/// All available commands
pub const COMMANDS: &[Command] = &[
    // File commands
    Command::new("new_file", "New File")
        .with_shortcut("⌘N")
        .with_vim(":new"),
    Command::new("open_file", "Open File...")
        .with_shortcut("⌘O")
        .with_vim(":e"),
    Command::new("save_file", "Save")
        .with_shortcut("⌘S")
        .with_vim(":w"),
    Command::new("save_file_as", "Save As...")
        .with_shortcut("⇧⌘S")
        .with_vim(":saveas"),
    Command::new("force_write", "Force Write")
        .with_vim(":w!"),
    Command::new("close_file", "Close")
        .with_shortcut("⌘W")
        .with_vim(":q"),
    Command::new("quit", "Quit")
        .with_shortcut("⌘Q")
        .with_vim(":q!"),
    // Edit commands
    Command::new("undo", "Undo").with_shortcut("⌘Z"),
    Command::new("redo", "Redo").with_shortcut("⇧⌘Z"),
    Command::new("cut", "Cut").with_shortcut("⌘X"),
    Command::new("copy", "Copy").with_shortcut("⌘C"),
    Command::new("paste", "Paste").with_shortcut("⌘V"),
    // View commands
    Command::new("toggle_read_only", "Toggle Read-Only")
        .with_vim(":view"),
];

pub struct CommandPalette {
    focus_handle: FocusHandle,
    input: String,
    cursor_pos: usize,
    selected_index: usize,
    filtered_commands: Vec<usize>,
    vim_command: Option<VimCommand>,
    on_command: Option<Box<dyn Fn(&str, Option<VimCommand>, &mut Window, &mut App) + 'static>>,
}

impl CommandPalette {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut palette = Self {
            focus_handle: cx.focus_handle(),
            input: String::new(),
            cursor_pos: 0,
            selected_index: 0,
            filtered_commands: Vec::new(),
            vim_command: None,
            on_command: None,
        };
        palette.update_filter();
        palette
    }

    pub fn set_command_handler<F>(&mut self, handler: F)
    where
        F: Fn(&str, Option<VimCommand>, &mut Window, &mut App) + 'static,
    {
        self.on_command = Some(Box::new(handler));
    }

    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.input.clear();
        self.cursor_pos = 0;
        self.selected_index = 0;
        self.vim_command = None;
        self.update_filter();
        cx.notify();
    }

    fn update_filter(&mut self) {
        let query = self.input.to_lowercase();

        // Check if it's a vim command
        self.vim_command = VimCommand::parse(&self.input);

        self.filtered_commands = COMMANDS
            .iter()
            .enumerate()
            .filter(|(_, cmd)| {
                if query.is_empty() {
                    return true;
                }
                // Match against name
                if cmd.name.to_lowercase().contains(&query) {
                    return true;
                }
                // Match against vim alias
                if let Some(alias) = cmd.vim_alias {
                    if query.starts_with(':') && alias.contains(&query) {
                        return true;
                    }
                }
                false
            })
            .map(|(idx, _)| idx)
            .collect();

        // Reset selection if out of bounds
        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = 0;
        }
    }

    fn select_next(&mut self, _: &SelectNext, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.filtered_commands.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_commands.len();
            cx.notify();
        }
    }

    fn select_previous(&mut self, _: &SelectPrevious, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.filtered_commands.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_commands.len() - 1;
            } else {
                self.selected_index -= 1;
            }
            cx.notify();
        }
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        // If there's a vim command, execute it directly
        if let Some(vim_cmd) = self.vim_command.take() {
            if let Some(handler) = &self.on_command {
                handler("vim_command", Some(vim_cmd), window, cx);
            }
            return;
        }

        // Otherwise execute the selected command
        if let Some(&cmd_idx) = self.filtered_commands.get(self.selected_index) {
            let cmd_id = COMMANDS[cmd_idx].id;
            if let Some(handler) = &self.on_command {
                handler(cmd_id, None, window, cx);
            }
        }
    }

    fn on_input_changed(&mut self, cx: &mut Context<Self>) {
        self.update_filter();
        cx.notify();
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .key_context("CommandPalette")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::confirm))
            .flex()
            .flex_col()
            .w(px(400.))
            .max_h(px(300.))
            .bg(theme.mantle)
            .border_1()
            .border_color(theme.surface1)
            .rounded(px(8.))
            .shadow_lg()
            .overflow_hidden()
            .child(self.render_input(cx))
            .child(self.render_results(cx))
    }
}

impl CommandPalette {
    fn render_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let input = self.input.clone();

        div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .h(px(40.))
            .px(px(12.))
            .border_b_1()
            .border_color(theme.surface0)
            .child(
                div()
                    .text_color(theme.subtext0)
                    .text_size(px(16.))
                    .mr(px(8.))
                    .child(">")
            )
            .child(
                div()
                    .id("palette-input")
                    .flex_1()
                    .text_size(px(14.))
                    .text_color(theme.text)
                    .child(CommandPaletteInput {
                        palette: cx.entity().clone(),
                        content: input,
                    })
            )
    }

    fn render_results(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .children(
                self.filtered_commands.iter().enumerate().map(|(idx, &cmd_idx)| {
                    let cmd = &COMMANDS[cmd_idx];
                    let is_selected = idx == self.selected_index;

                    div()
                        .id(ElementId::Name(format!("cmd-{}", cmd.id).into()))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .h(px(32.))
                        .px(px(12.))
                        .when(is_selected, |d| d.bg(theme.surface0))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, {
                            let entity = cx.entity().clone();
                            let selected_idx = idx;
                            move |_, window, app| {
                                entity.update(app, |palette, cx| {
                                    palette.selected_index = selected_idx;
                                    cx.notify();
                                });
                                // Dispatch the confirm action
                                window.dispatch_action(Box::new(Confirm), app);
                            }
                        })
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .text_color(theme.text)
                                        .child(cmd.name)
                                )
                                .when_some(cmd.vim_alias, |d, alias| {
                                    d.child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.subtext0)
                                            .child(alias)
                                    )
                                })
                        )
                        .when_some(cmd.shortcut, |d, shortcut| {
                            d.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.subtext0)
                                    .child(shortcut)
                            )
                        })
                })
            )
    }
}

impl Focusable for CommandPalette {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Input element for the command palette
pub struct CommandPaletteInput {
    palette: Entity<CommandPalette>,
    content: String,
}

impl IntoElement for CommandPaletteInput {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for CommandPaletteInput {
    type RequestLayoutState = ();
    type PrepaintState = ShapedLine;

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
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());

        let display_text: SharedString = if self.content.is_empty() {
            "Type a command...".into()
        } else {
            self.content.clone().into()
        };

        let text_color: Hsla = if self.content.is_empty() {
            cx.global::<Theme>().subtext0.into()
        } else {
            style.color
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        window.text_system().shape_line(
            display_text,
            font_size,
            &[run],
            None,
        )
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
        let focus_handle = self.palette.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.palette.clone()),
            cx,
        );

        prepaint.paint(bounds.origin, window.line_height(), gpui::TextAlign::Left, None, window, cx)
            .unwrap();

        // Draw cursor
        if focus_handle.is_focused(window) {
            let theme = cx.global::<Theme>();
            let cursor_x = if self.content.is_empty() {
                px(0.)
            } else {
                let cursor_pos = self.palette.read(cx).cursor_pos;
                prepaint.x_for_index(cursor_pos)
            };

            let cursor_bounds = Bounds::new(
                point(bounds.left() + cursor_x, bounds.top()),
                size(px(2.), bounds.size.height),
            );
            window.paint_quad(fill(cursor_bounds, theme.accent));
        }
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
}

impl EntityInputHandler for CommandPalette {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.input[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let pos = self.offset_to_utf16(self.cursor_pos);
        Some(UTF16Selection {
            range: pos..pos,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .unwrap_or(self.cursor_pos..self.cursor_pos);

        self.input = self.input[..range.start].to_owned() + new_text + &self.input[range.end..];
        self.cursor_pos = range.start + new_text.len();
        self.on_input_changed(cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range_utf16, new_text, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.offset_to_utf16(self.cursor_pos))
    }
}

impl CommandPalette {
    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.input.chars() {
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

        for ch in self.input.chars() {
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
}
