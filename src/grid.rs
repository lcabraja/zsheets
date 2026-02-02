use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::cell::CellInput;
use crate::command_palette::{CommandPalette, HideCommandPalette, ShowCommandPalette, VimCommand};
use crate::file_io;
use crate::file_state::FileState;
use crate::state::{CellPosition, Mode, GRID_COLS, GRID_ROWS};
use crate::Theme;

pub const CELL_WIDTH: f32 = 100.0;
pub const CELL_HEIGHT: f32 = 28.0;
pub const ROW_HEADER_WIDTH: f32 = 50.0;
pub const COLUMN_HEADER_HEIGHT: f32 = 24.0;
pub const HEADER_HEIGHT: f32 = 32.0;
pub const FOOTER_HEIGHT: f32 = 24.0;

// Minimum window size: enough for header + column headers + 1 cell row + footer (height)
// and row header + 1 cell column (width)
pub const MIN_WINDOW_WIDTH: f32 = ROW_HEADER_WIDTH + CELL_WIDTH;
pub const MIN_WINDOW_HEIGHT: f32 = HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + CELL_HEIGHT + FOOTER_HEIGHT;

// Actions for Normal mode
actions!(
    normal_mode,
    [
        MoveUp,
        MoveDown,
        MoveLeft,
        MoveRight,
        EnterEditMode,
    ]
);

// Actions for Edit mode
actions!(
    edit_mode,
    [
        ExitEditMode,
        ExitAndMoveUp,
        ExitAndMoveDown,
        ExitAndMoveLeft,
        ExitAndMoveRight,
    ]
);

// Global actions
actions!(spreadsheet, [Quit]);

// File operation actions
actions!(
    file_ops,
    [
        NewFile,
        OpenFile,
        SaveFile,
        SaveFileAs,
        ForceWrite,
        CloseFile,
        ToggleReadOnly,
        ForceQuit,
    ]
);

/// The main spreadsheet application component
pub struct SpreadsheetApp {
    grid: Entity<SpreadsheetGrid>,
}

impl SpreadsheetApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let grid = cx.new(|cx| SpreadsheetGrid::new(cx));
        Self { grid }
    }
}

impl Render for SpreadsheetApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.base)
            .text_color(theme.text)
            .font_family("Berkeley Mono")
            .child(self.grid.clone())
    }
}

/// The spreadsheet grid component
pub struct SpreadsheetGrid {
    focus_handle: FocusHandle,
    active_input: Entity<CellInput>,
    cells: Vec<Vec<String>>,
    selected: CellPosition,
    scroll_row: usize,
    scroll_col: usize,
    mode: Mode,
    visible_rows: usize,
    visible_cols: usize,
    file_state: FileState,
    command_palette: Entity<CommandPalette>,
    show_command_palette: bool,
}

impl SpreadsheetGrid {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let active_input = cx.new(|cx| CellInput::new(cx));
        let command_palette = cx.new(|cx| CommandPalette::new(cx));

        // Initialize 100x100 grid with empty strings
        let cells = (0..GRID_ROWS)
            .map(|_| (0..GRID_COLS).map(|_| String::new()).collect())
            .collect();

        Self {
            focus_handle,
            active_input,
            cells,
            selected: CellPosition::new(0, 0),
            scroll_row: 0,
            scroll_col: 0,
            mode: Mode::Normal,
            visible_rows: 20,
            visible_cols: 10,
            file_state: FileState::new(),
            command_palette,
            show_command_palette: false,
        }
    }

    fn move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(-1, 0, window, cx);
    }

    fn move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(1, 0, window, cx);
    }

    fn move_left(&mut self, _: &MoveLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(0, -1, window, cx);
    }

    fn move_right(&mut self, _: &MoveRight, window: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(0, 1, window, cx);
    }

    fn move_selection(&mut self, delta_row: isize, delta_col: isize, _window: &mut Window, cx: &mut Context<Self>) {
        // Calculate new position with bounds clamping
        let new_row = (self.selected.row as isize + delta_row)
            .max(0)
            .min((GRID_ROWS - 1) as isize) as usize;
        let new_col = (self.selected.col as isize + delta_col)
            .max(0)
            .min((GRID_COLS - 1) as isize) as usize;

        self.selected = CellPosition::new(new_row, new_col);
        self.ensure_visible();
        cx.notify();
    }

    fn enter_edit_mode(&mut self, _: &EnterEditMode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = Mode::Edit;

        // Load current cell content into the input
        let content = self.cells[self.selected.row][self.selected.col].clone();
        self.active_input.update(cx, |input, cx| {
            input.set_content(content, cx);
        });

        // Focus the input
        let focus_handle = self.active_input.focus_handle(cx);
        focus_handle.focus(window, cx);
        cx.notify();
    }

    fn exit_edit_mode(&mut self, _: &ExitEditMode, window: &mut Window, cx: &mut Context<Self>) {
        self.save_and_exit_edit_mode(window, cx);
    }

    fn exit_and_move_up(&mut self, _: &ExitAndMoveUp, window: &mut Window, cx: &mut Context<Self>) {
        self.save_and_exit_edit_mode(window, cx);
        self.move_selection(-1, 0, window, cx);
    }

    fn exit_and_move_down(&mut self, _: &ExitAndMoveDown, window: &mut Window, cx: &mut Context<Self>) {
        self.save_and_exit_edit_mode(window, cx);
        self.move_selection(1, 0, window, cx);
    }

    fn exit_and_move_left(&mut self, _: &ExitAndMoveLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.save_and_exit_edit_mode(window, cx);
        self.move_selection(0, -1, window, cx);
    }

    fn exit_and_move_right(&mut self, _: &ExitAndMoveRight, window: &mut Window, cx: &mut Context<Self>) {
        self.save_and_exit_edit_mode(window, cx);
        self.move_selection(0, 1, window, cx);
    }

    fn save_and_exit_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Save the content from the input back to the cell
        let content = self.active_input.read(cx).get_content();
        let old_content = &self.cells[self.selected.row][self.selected.col];
        if &content != old_content {
            self.cells[self.selected.row][self.selected.col] = content;
            self.file_state.mark_dirty();
        }

        self.mode = Mode::Normal;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    // File operations
    fn new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        // Reset all cells
        self.cells = (0..GRID_ROWS)
            .map(|_| (0..GRID_COLS).map(|_| String::new()).collect())
            .collect();
        self.selected = CellPosition::new(0, 0);
        self.scroll_row = 0;
        self.scroll_col = 0;
        self.file_state = FileState::new();
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn open_file(&mut self, _: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        self.open_file_dialog(false, window, cx);
    }

    fn open_file_dialog(&mut self, read_only: bool, window: &mut Window, cx: &mut Context<Self>) {
        let path = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .add_filter("All Files", &["*"])
            .pick_file();

        if let Some(path) = path {
            self.load_file(path, read_only, cx);
        }

        self.focus_handle.focus(window, cx);
    }

    fn load_file(&mut self, path: PathBuf, read_only: bool, cx: &mut Context<Self>) {
        match file_io::read_csv(&path) {
            Ok(cells) => {
                self.cells = cells;
                self.selected = CellPosition::new(0, 0);
                self.scroll_row = 0;
                self.scroll_col = 0;
                self.file_state = FileState::new();
                self.file_state.set_path(path);
                self.file_state.set_read_only(read_only);
                cx.notify();
            }
            Err(e) => {
                eprintln!("Failed to open file: {}", e);
            }
        }
    }

    fn save_file(&mut self, _: &SaveFile, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_state.is_read_only {
            eprintln!("File is read-only. Use :w! to force write.");
            return;
        }

        if let Some(path) = self.file_state.current_path.clone() {
            self.save_to_path(&path, cx);
        } else {
            self.save_file_as(&SaveFileAs, window, cx);
        }
    }

    fn save_file_as(&mut self, _: &SaveFileAs, window: &mut Window, cx: &mut Context<Self>) {
        let path = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("spreadsheet.csv")
            .save_file();

        if let Some(path) = path {
            self.save_to_path(&path, cx);
            self.file_state.set_path(path);
        }

        self.focus_handle.focus(window, cx);
    }

    fn force_write(&mut self, _: &ForceWrite, window: &mut Window, cx: &mut Context<Self>) {
        let was_read_only = self.file_state.is_read_only;
        self.file_state.set_read_only(false);

        if let Some(path) = self.file_state.current_path.clone() {
            self.save_to_path(&path, cx);
        } else {
            self.save_file_as(&SaveFileAs, window, cx);
        }

        self.file_state.set_read_only(was_read_only);
    }

    fn save_to_path(&mut self, path: &PathBuf, cx: &mut Context<Self>) {
        match file_io::write_csv(path, &self.cells) {
            Ok(()) => {
                self.file_state.mark_clean();
                self.file_state.set_path(path.clone());
                cx.notify();
            }
            Err(e) => {
                eprintln!("Failed to save file: {}", e);
            }
        }
    }

    fn close_file(&mut self, _: &CloseFile, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_state.is_dirty {
            eprintln!("File has unsaved changes. Use :q! to force quit.");
            return;
        }
        self.new_file(&NewFile, window, cx);
    }

    fn force_quit(&mut self, _: &ForceQuit, _window: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    fn toggle_read_only(&mut self, _: &ToggleReadOnly, _window: &mut Window, cx: &mut Context<Self>) {
        self.file_state.set_read_only(!self.file_state.is_read_only);
        cx.notify();
    }

    // Command palette
    fn show_command_palette(&mut self, _: &ShowCommandPalette, window: &mut Window, cx: &mut Context<Self>) {
        // Exit edit mode if active
        if self.mode == Mode::Edit {
            self.save_and_exit_edit_mode(window, cx);
        }

        self.show_command_palette = true;
        self.command_palette.update(cx, |palette, cx| {
            palette.reset(cx);
        });

        let palette_focus = self.command_palette.focus_handle(cx);
        palette_focus.focus(window, cx);
        cx.notify();
    }

    fn hide_command_palette(&mut self, _: &HideCommandPalette, window: &mut Window, cx: &mut Context<Self>) {
        self.show_command_palette = false;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn handle_command(&mut self, cmd_id: &str, vim_cmd: Option<VimCommand>, window: &mut Window, cx: &mut Context<Self>) {
        // Hide palette first
        self.show_command_palette = false;
        self.focus_handle.focus(window, cx);

        // Handle vim commands
        if let Some(vim_cmd) = vim_cmd {
            match vim_cmd {
                VimCommand::Write => self.save_file(&SaveFile, window, cx),
                VimCommand::WriteTo(path) => {
                    self.save_to_path(&path, cx);
                    self.file_state.set_path(path);
                }
                VimCommand::ForceWrite => self.force_write(&ForceWrite, window, cx),
                VimCommand::WriteQuit => {
                    self.save_file(&SaveFile, window, cx);
                    cx.quit();
                }
                VimCommand::Quit => self.close_file(&CloseFile, window, cx),
                VimCommand::ForceQuit => cx.quit(),
                VimCommand::Edit(path) => self.load_file(path, false, cx),
                VimCommand::View(path) => self.load_file(path, true, cx),
                VimCommand::SaveAs(path) => {
                    self.save_to_path(&path, cx);
                    self.file_state.set_path(path);
                }
                VimCommand::New => self.new_file(&NewFile, window, cx),
            }
            cx.notify();
            return;
        }

        // Handle regular commands
        match cmd_id {
            "new_file" => self.new_file(&NewFile, window, cx),
            "open_file" => self.open_file(&OpenFile, window, cx),
            "save_file" => self.save_file(&SaveFile, window, cx),
            "save_file_as" => self.save_file_as(&SaveFileAs, window, cx),
            "force_write" => self.force_write(&ForceWrite, window, cx),
            "close_file" => self.close_file(&CloseFile, window, cx),
            "quit" => cx.quit(),
            "toggle_read_only" => self.toggle_read_only(&ToggleReadOnly, window, cx),
            _ => {}
        }
        cx.notify();
    }

    fn ensure_visible(&mut self) {
        // Vertical scrolling
        if self.selected.row < self.scroll_row {
            self.scroll_row = self.selected.row;
        } else if self.selected.row >= self.scroll_row + self.visible_rows {
            self.scroll_row = self.selected.row.saturating_sub(self.visible_rows - 1);
        }

        // Horizontal scrolling
        if self.selected.col < self.scroll_col {
            self.scroll_col = self.selected.col;
        } else if self.selected.col >= self.scroll_col + self.visible_cols {
            self.scroll_col = self.selected.col.saturating_sub(self.visible_cols - 1);
        }
    }

    fn on_cell_click(&mut self, row: usize, col: usize, window: &mut Window, cx: &mut Context<Self>) {
        // If clicking on a different cell while in edit mode, save and exit first
        if self.mode == Mode::Edit && (row != self.selected.row || col != self.selected.col) {
            self.save_and_exit_edit_mode(window, cx);
        }

        self.selected = CellPosition::new(row, col);
        self.ensure_visible();
        cx.notify();
    }

    fn on_cell_double_click(&mut self, row: usize, col: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = CellPosition::new(row, col);
        self.ensure_visible();

        // Enter edit mode on double click
        self.mode = Mode::Edit;
        let content = self.cells[row][col].clone();
        self.active_input.update(cx, |input, cx| {
            input.set_content(content, cx);
        });
        let focus_handle = self.active_input.focus_handle(cx);
        focus_handle.focus(window, cx);
        cx.notify();
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let cell_ref = self.selected.to_reference();

        div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(HEADER_HEIGHT))
            .bg(theme.mantle)
            .border_b_1()
            .border_color(theme.surface0)
            .items_center()
            .px(px(8.))
            .gap(px(8.))
            .child(
                // Cell reference label
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(60.))
                    .h(px(24.))
                    .bg(theme.surface0)
                    .rounded(px(4.))
                    .text_size(px(14.))
                    .text_color(theme.subtext1)
                    .child(cell_ref)
            )
            .child(
                // Formula bar / content display
                div()
                    .flex_1()
                    .h(px(24.))
                    .bg(theme.surface0)
                    .rounded(px(4.))
                    .overflow_hidden()
                    .px(px(8.))
                    .items_center()
                    .text_size(px(14.))
                    .child(if self.mode == Mode::Edit {
                        // Show input content in edit mode
                        let content = self.active_input.read(cx).get_content();
                        content
                    } else {
                        // Show cell content in normal mode
                        self.cells[self.selected.row][self.selected.col].clone()
                    })
            )
    }

    fn render_column_headers(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let end_col = (self.scroll_col + self.visible_cols).min(GRID_COLS);

        div()
            .flex()
            .flex_row()
            .h(px(COLUMN_HEADER_HEIGHT))
            .bg(theme.mantle)
            .border_b_1()
            .border_color(theme.surface0)
            .child(
                // Empty corner cell
                div()
                    .w(px(ROW_HEADER_WIDTH))
                    .h_full()
                    .flex_none()
                    .border_r_1()
                    .border_color(theme.surface0)
            )
            .children(
                (self.scroll_col..end_col).map(|col| {
                    let col_letter = CellPosition::new(0, col).to_reference();
                    let col_letter: String = col_letter.chars().take_while(|c| c.is_alphabetic()).collect();
                    let is_selected = col == self.selected.col;

                    div()
                        .w(px(CELL_WIDTH))
                        .h_full()
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_r_1()
                        .border_color(theme.surface0)
                        .text_size(px(12.))
                        .text_color(if is_selected { theme.accent } else { theme.subtext0 })
                        .font_weight(if is_selected { FontWeight::BOLD } else { FontWeight::NORMAL })
                        .child(col_letter)
                })
            )
    }

    fn render_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let end_row = (self.scroll_row + self.visible_rows).min(GRID_ROWS);
        let end_col = (self.scroll_col + self.visible_cols).min(GRID_COLS);

        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .children(
                (self.scroll_row..end_row).map(|row| {
                    let is_row_selected = row == self.selected.row;

                    div()
                        .flex()
                        .flex_row()
                        .h(px(CELL_HEIGHT))
                        .child(
                            // Row header
                            div()
                                .w(px(ROW_HEADER_WIDTH))
                                .h_full()
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(theme.mantle)
                                .border_r_1()
                                .border_b_1()
                                .border_color(theme.surface0)
                                .text_size(px(12.))
                                .text_color(if is_row_selected { theme.accent } else { theme.subtext0 })
                                .font_weight(if is_row_selected { FontWeight::BOLD } else { FontWeight::NORMAL })
                                .child(format!("{}", row + 1))
                        )
                        .children(
                            (self.scroll_col..end_col).map(|col| {
                                let is_selected = row == self.selected.row && col == self.selected.col;
                                let content = self.cells[row][col].clone();

                                if is_selected && self.mode == Mode::Edit {
                                    // Render the active input for selected cell in edit mode
                                    div()
                                        .id(ElementId::Name(format!("cell-edit-{}-{}", row, col).into()))
                                        .w(px(CELL_WIDTH))
                                        .h(px(CELL_HEIGHT))
                                        .flex_none()
                                        .border_2()
                                        .border_color(theme.accent)
                                        .overflow_hidden()
                                        .child(self.active_input.clone())
                                } else {
                                    // Render static cell
                                    let row = row;
                                    let col = col;
                                    div()
                                        .id(ElementId::Name(format!("cell-{}-{}", row, col).into()))
                                        .w(px(CELL_WIDTH))
                                        .h(px(CELL_HEIGHT))
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .px(px(4.))
                                        .border_r_1()
                                        .border_b_1()
                                        .border_color(if is_selected { theme.accent } else { theme.surface0 })
                                        .when(is_selected, |d| d.border_2())
                                        .bg(if is_selected { theme.surface0 } else { theme.base })
                                        .text_size(px(14.))
                                        .overflow_hidden()
                                        .on_mouse_down(MouseButton::Left, {
                                            let entity = cx.entity().clone();
                                            move |event, window, app| {
                                                if event.click_count == 2 {
                                                    entity.update(app, |this, cx| {
                                                        this.on_cell_double_click(row, col, window, cx);
                                                    });
                                                } else {
                                                    entity.update(app, |this, cx| {
                                                        this.on_cell_click(row, col, window, cx);
                                                    });
                                                }
                                            }
                                        })
                                        .child(content)
                                }
                            })
                        )
                })
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let mode_text = match self.mode {
            Mode::Normal => "-- NORMAL --",
            Mode::Edit => "-- EDIT --",
        };

        let file_name = self.file_state.file_name();
        let dirty_indicator = if self.file_state.is_dirty { "[+] " } else { "" };
        let read_only_indicator = if self.file_state.is_read_only { "[RO] " } else { "" };

        div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(FOOTER_HEIGHT))
            .bg(theme.mantle)
            .border_t_1()
            .border_color(theme.surface0)
            .items_center()
            .justify_between()
            .px(px(8.))
            .text_size(px(12.))
            .text_color(theme.subtext0)
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .child(mode_text)
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .child(
                        div()
                            .when(self.file_state.is_read_only, |d| d.text_color(theme.overlay1))
                            .child(read_only_indicator)
                    )
                    .child(
                        div()
                            .when(self.file_state.is_dirty, |d| d.text_color(theme.accent))
                            .child(dirty_indicator)
                    )
                    .child(file_name)
            )
    }
}

impl Render for SpreadsheetGrid {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Calculate visible rows and columns based on window size
        let content_bounds = window.viewport_size();
        let grid_height = f32::from(content_bounds.height) - HEADER_HEIGHT - COLUMN_HEADER_HEIGHT - FOOTER_HEIGHT;
        let grid_width = f32::from(content_bounds.width) - ROW_HEADER_WIDTH;

        self.visible_rows = ((grid_height / CELL_HEIGHT).ceil() as usize).max(1);
        self.visible_cols = ((grid_width / CELL_WIDTH).ceil() as usize).max(1);

        // Ensure selection is still visible after resize
        self.ensure_visible();

        let key_context = if self.show_command_palette {
            "CommandPalette"
        } else if self.mode == Mode::Edit {
            "EditMode"
        } else {
            "NormalMode"
        };

        // Set up command handler for the palette
        let entity = cx.entity().clone();
        self.command_palette.update(cx, |palette, _cx| {
            palette.set_command_handler(move |cmd_id, vim_cmd, window, app| {
                entity.update(app, |grid, cx| {
                    grid.handle_command(cmd_id, vim_cmd, window, cx);
                });
            });
        });

        let show_palette = self.show_command_palette;

        div()
            .flex()
            .flex_col()
            .size_full()
            .key_context(key_context)
            .track_focus(&self.focus_handle)
            // Normal mode actions
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::enter_edit_mode))
            // Edit mode actions
            .on_action(cx.listener(Self::exit_edit_mode))
            .on_action(cx.listener(Self::exit_and_move_up))
            .on_action(cx.listener(Self::exit_and_move_down))
            .on_action(cx.listener(Self::exit_and_move_left))
            .on_action(cx.listener(Self::exit_and_move_right))
            // File actions
            .on_action(cx.listener(Self::new_file))
            .on_action(cx.listener(Self::open_file))
            .on_action(cx.listener(Self::save_file))
            .on_action(cx.listener(Self::save_file_as))
            .on_action(cx.listener(Self::force_write))
            .on_action(cx.listener(Self::close_file))
            .on_action(cx.listener(Self::force_quit))
            .on_action(cx.listener(Self::toggle_read_only))
            // Command palette actions
            .on_action(cx.listener(Self::show_command_palette))
            .on_action(cx.listener(Self::hide_command_palette))
            .child(self.render_header(cx))
            .child(self.render_column_headers(cx))
            .child(self.render_grid(cx))
            .child(self.render_footer(cx))
            // Command palette overlay
            .when(show_palette, |d| {
                d.child(
                    div()
                        .absolute()
                        .size_full()
                        .top_0()
                        .left_0()
                        .flex()
                        .items_start()
                        .justify_center()
                        .pt(px(100.))
                        .bg(rgba(0x00000080))
                        .on_mouse_down(MouseButton::Left, {
                            let entity = cx.entity().clone();
                            move |_, window, app| {
                                entity.update(app, |grid, cx| {
                                    grid.hide_command_palette(&HideCommandPalette, window, cx);
                                });
                            }
                        })
                        .child(
                            div()
                                .on_mouse_down(MouseButton::Left, |_, _, _| {
                                    // Prevent click from bubbling to backdrop
                                })
                                .child(self.command_palette.clone())
                        )
                )
            })
    }
}

impl Focusable for SpreadsheetGrid {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
