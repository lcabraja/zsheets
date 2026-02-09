use std::collections::HashSet;
use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::cell::CellInput;
use crate::command_palette::{CommandPalette, HideCommandPalette, ShowCommandPalette, VimCommand};
use crate::file_io;
use crate::file_state::FileState;
use crate::metadata::SpreadsheetMetadata;
use crate::state::{CellPosition, Mode, GRID_COLS, GRID_ROWS};
use crate::Theme;

pub const DEFAULT_CELL_WIDTH: f32 = 100.0;
pub const DEFAULT_CELL_HEIGHT: f32 = 28.0;
pub const MIN_CELL_WIDTH: f32 = 30.0;
pub const MIN_CELL_HEIGHT: f32 = 20.0;
pub const RESIZE_HANDLE_WIDTH: f32 = 5.0;
pub const ROW_HEADER_WIDTH: f32 = 50.0;
pub const COLUMN_HEADER_HEIGHT: f32 = 24.0;
pub const HEADER_HEIGHT: f32 = 32.0;
pub const FOOTER_HEIGHT: f32 = 24.0;

// Minimum window size: enough for header + column headers + 1 cell row + footer (height)
// and row header + 1 cell column (width)
pub const MIN_WINDOW_WIDTH: f32 = ROW_HEADER_WIDTH + DEFAULT_CELL_WIDTH;
pub const MIN_WINDOW_HEIGHT: f32 = HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + DEFAULT_CELL_HEIGHT + FOOTER_HEIGHT;

/// Target for resize operation
#[derive(Clone, Copy, Debug)]
pub enum ResizeTarget {
    Column(usize),
    Row(usize),
}

/// State for active resize operation
#[derive(Clone, Copy, Debug)]
pub struct ResizeState {
    pub target: ResizeTarget,
    pub start_mouse_pos: f32,
    pub original_size: f32,
}

/// Auto-fit watch mode configuration
#[derive(Clone, Debug, Default)]
pub enum AutoFitWatch {
    #[default]
    None,
    All,
    Columns(HashSet<usize>),
    Rows(HashSet<usize>),
}

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
actions!(spreadsheet, [Quit, ToggleKeepCursorInView]);

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
    grid_height: f32,
    grid_width: f32,
    file_state: FileState,
    command_palette: Entity<CommandPalette>,
    show_command_palette: bool,
    // Scroll pixel offsets for smooth scrolling
    scroll_offset_x: f32,
    scroll_offset_y: f32,
    // When true, scrolling moves the cursor to stay in view
    // When false, cursor stays put; arrow keys snap viewport back to cursor
    keep_cursor_in_view: bool,
    // Resizing support
    column_widths: Vec<f32>,
    row_heights: Vec<f32>,
    resize_state: Option<ResizeState>,
    autofit_watch: AutoFitWatch,
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
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            keep_cursor_in_view: false,
            mode: Mode::Normal,
            visible_rows: 20,
            visible_cols: 10,
            grid_height: 0.0,
            grid_width: 0.0,
            file_state: FileState::new(),
            command_palette,
            show_command_palette: false,
            column_widths: vec![DEFAULT_CELL_WIDTH; GRID_COLS],
            row_heights: vec![DEFAULT_CELL_HEIGHT; GRID_ROWS],
            resize_state: None,
            autofit_watch: AutoFitWatch::None,
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
        let content_changed = &content != old_content;
        if content_changed {
            self.cells[self.selected.row][self.selected.col] = content;
            self.file_state.mark_dirty();
            // Check if auto-fit watch mode should resize this cell
            let row = self.selected.row;
            let col = self.selected.col;
            self.check_autofit_watch(row, col, cx);
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
        self.scroll_offset_x = 0.0;
        self.scroll_offset_y = 0.0;
        // Reset dimensions to defaults
        self.column_widths = vec![DEFAULT_CELL_WIDTH; GRID_COLS];
        self.row_heights = vec![DEFAULT_CELL_HEIGHT; GRID_ROWS];
        self.autofit_watch = AutoFitWatch::None;
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
                self.scroll_offset_x = 0.0;
                self.scroll_offset_y = 0.0;

                // Load metadata (column widths, row heights)
                match SpreadsheetMetadata::load(&path) {
                    Ok(metadata) => {
                        self.column_widths = metadata.get_column_widths();
                        self.row_heights = metadata.get_row_heights();
                    }
                    Err(_) => {
                        // Reset to defaults if metadata can't be loaded
                        self.column_widths = vec![DEFAULT_CELL_WIDTH; GRID_COLS];
                        self.row_heights = vec![DEFAULT_CELL_HEIGHT; GRID_ROWS];
                    }
                }

                self.file_state = FileState::new();
                self.file_state.set_path(path);
                self.file_state.set_read_only(read_only);
                self.autofit_watch = AutoFitWatch::None;
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
                // Save metadata (column widths, row heights)
                let metadata = SpreadsheetMetadata {
                    column_widths: Some(self.column_widths.clone()),
                    row_heights: Some(self.row_heights.clone()),
                };
                if let Err(e) = metadata.save(path) {
                    eprintln!("Warning: Failed to save metadata: {}", e);
                }

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

    fn toggle_keep_cursor_in_view(&mut self, _: &ToggleKeepCursorInView, _window: &mut Window, cx: &mut Context<Self>) {
        self.keep_cursor_in_view = !self.keep_cursor_in_view;
        crate::menu::setup_menu_with_state(cx, self.keep_cursor_in_view);
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
                // Auto-fit commands
                VimCommand::AutoFitAll => self.auto_fit_all(cx),
                VimCommand::AutoFitColumn => self.auto_fit_column(self.selected.col, cx),
                VimCommand::AutoFitRow => self.auto_fit_row(self.selected.row, cx),
                VimCommand::AutoFitWatch => self.toggle_autofit_watch_all(cx),
                VimCommand::AutoFitColumnWatch => self.toggle_autofit_watch_column(self.selected.col, cx),
                VimCommand::AutoFitRowWatch => self.toggle_autofit_watch_row(self.selected.row, cx),
                VimCommand::ResetAllSizes => self.reset_all_sizes(cx),
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
            // Auto-fit commands
            "autofit_all" => self.auto_fit_all(cx),
            "autofit_column" => self.auto_fit_column(self.selected.col, cx),
            "autofit_row" => self.auto_fit_row(self.selected.row, cx),
            "autofit_watch" => self.toggle_autofit_watch_all(cx),
            "reset_sizes" => self.reset_all_sizes(cx),
            _ => {}
        }
        cx.notify();
    }

    fn ensure_visible(&mut self) {
        // Vertical: cursor above viewport or partially hidden at top
        if self.selected.row < self.scroll_row
            || (self.selected.row == self.scroll_row && self.scroll_offset_y > 0.0)
        {
            self.scroll_row = self.selected.row;
            self.scroll_offset_y = 0.0;
        } else {
            // Check if cursor row is partially clipped at the bottom
            let last_full_row = self.last_fully_visible_row();
            if self.selected.row > last_full_row {
                // Scroll down so cursor row is fully visible at the bottom
                self.scroll_to_show_row_at_bottom(self.selected.row);
            }
        }

        // Horizontal: cursor left of viewport or partially hidden at left
        if self.selected.col < self.scroll_col
            || (self.selected.col == self.scroll_col && self.scroll_offset_x > 0.0)
        {
            self.scroll_col = self.selected.col;
            self.scroll_offset_x = 0.0;
        } else {
            // Check if cursor col is partially clipped at the right
            let last_full_col = self.last_fully_visible_col();
            if self.selected.col > last_full_col {
                // Scroll right so cursor col is fully visible at the right
                self.scroll_to_show_col_at_right(self.selected.col);
            }
        }
    }

    /// Find the last row index that is fully visible in the viewport
    fn last_fully_visible_row(&self) -> usize {
        let grid_height = self.grid_height;
        let mut total = 0.0;
        for (i, row) in (self.scroll_row..GRID_ROWS).enumerate() {
            let h = self.row_heights[row];
            let visible_h = if i == 0 { h - self.scroll_offset_y } else { h };
            total += visible_h;
            if total > grid_height {
                // This row is partially clipped; the previous row is the last fully visible
                return if row > self.scroll_row { row - 1 } else { self.scroll_row };
            }
        }
        (GRID_ROWS - 1).min(self.scroll_row + self.visible_rows - 1)
    }

    /// Find the last column index that is fully visible in the viewport
    fn last_fully_visible_col(&self) -> usize {
        let grid_width = self.grid_width;
        let mut total = 0.0;
        for (i, col) in (self.scroll_col..GRID_COLS).enumerate() {
            let w = self.column_widths[col];
            let visible_w = if i == 0 { w - self.scroll_offset_x } else { w };
            total += visible_w;
            if total > grid_width {
                return if col > self.scroll_col { col - 1 } else { self.scroll_col };
            }
        }
        (GRID_COLS - 1).min(self.scroll_col + self.visible_cols - 1)
    }

    /// Scroll viewport by just enough pixels to fully reveal `target_row` at the bottom
    fn scroll_to_show_row_at_bottom(&mut self, target_row: usize) {
        // Compute how far the bottom edge of target_row extends past the viewport
        let mut total = 0.0;
        for (i, row) in (self.scroll_row..=target_row).enumerate() {
            let h = self.row_heights[row];
            let visible_h = if i == 0 { h - self.scroll_offset_y } else { h };
            total += visible_h;
        }
        let overflow = total - self.grid_height;
        if overflow > 0.0 {
            self.apply_smooth_scroll(0.0, overflow);
        }
    }

    /// Scroll viewport by just enough pixels to fully reveal `target_col` at the right
    fn scroll_to_show_col_at_right(&mut self, target_col: usize) {
        let mut total = 0.0;
        for (i, col) in (self.scroll_col..=target_col).enumerate() {
            let w = self.column_widths[col];
            let visible_w = if i == 0 { w - self.scroll_offset_x } else { w };
            total += visible_w;
        }
        let overflow = total - self.grid_width;
        if overflow > 0.0 {
            self.apply_smooth_scroll(overflow, 0.0);
        }
    }

    /// Calculate number of visible rows from scroll position that fit in given height
    fn calculate_visible_rows(&self, available_height: f32) -> usize {
        let mut total_height = 0.0;
        let mut count = 0;
        for row in self.scroll_row..GRID_ROWS {
            let row_h = self.row_heights[row];
            // First row is partially hidden by scroll_offset_y
            let visible_h = if count == 0 { row_h - self.scroll_offset_y } else { row_h };
            total_height += visible_h;
            count += 1;
            if total_height >= available_height {
                break;
            }
        }
        count.max(1)
    }

    /// Calculate number of visible columns from scroll position that fit in given width
    fn calculate_visible_cols(&self, available_width: f32) -> usize {
        let mut total_width = 0.0;
        let mut count = 0;
        for col in self.scroll_col..GRID_COLS {
            let col_w = self.column_widths[col];
            // First column is partially hidden by scroll_offset_x
            let visible_w = if count == 0 { col_w - self.scroll_offset_x } else { col_w };
            total_width += visible_w;
            count += 1;
            if total_width >= available_width {
                break;
            }
        }
        count.max(1)
    }

    // === Resize handle detection helpers ===

    /// Get the X position where a column ends (relative to grid area, after row header)
    fn column_end_x(&self, col: usize) -> f32 {
        let sum: f32 = self.column_widths[self.scroll_col..=col].iter().sum();
        sum - self.scroll_offset_x
    }

    /// Get the Y position where a row ends (relative to grid area, after column header)
    fn row_end_y(&self, row: usize) -> f32 {
        let sum: f32 = self.row_heights[self.scroll_row..=row].iter().sum();
        sum - self.scroll_offset_y
    }

    /// Find if x position is near a column resize border, returns the column index whose right edge is near
    fn column_resize_target(&self, x: f32) -> Option<usize> {
        let end_col = (self.scroll_col + self.visible_cols).min(GRID_COLS);
        for col in self.scroll_col..end_col {
            let col_end = self.column_end_x(col);
            if (x - col_end).abs() <= RESIZE_HANDLE_WIDTH {
                return Some(col);
            }
        }
        None
    }

    /// Find if y position is near a row resize border, returns the row index whose bottom edge is near
    fn row_resize_target(&self, y: f32) -> Option<usize> {
        let end_row = (self.scroll_row + self.visible_rows).min(GRID_ROWS);
        for row in self.scroll_row..end_row {
            let row_end = self.row_end_y(row);
            if (y - row_end).abs() <= RESIZE_HANDLE_WIDTH {
                return Some(row);
            }
        }
        None
    }

    // === Resize operations ===

    /// Start a column resize operation
    fn start_column_resize(&mut self, col: usize, mouse_x: f32, _cx: &mut Context<Self>) {
        self.resize_state = Some(ResizeState {
            target: ResizeTarget::Column(col),
            start_mouse_pos: mouse_x,
            original_size: self.column_widths[col],
        });
    }

    /// Start a row resize operation
    fn start_row_resize(&mut self, row: usize, mouse_y: f32, _cx: &mut Context<Self>) {
        self.resize_state = Some(ResizeState {
            target: ResizeTarget::Row(row),
            start_mouse_pos: mouse_y,
            original_size: self.row_heights[row],
        });
    }

    /// Update size during resize drag
    fn update_resize(&mut self, current_pos: f32, cx: &mut Context<Self>) {
        if let Some(state) = &self.resize_state {
            let delta = current_pos - state.start_mouse_pos;
            let new_size = (state.original_size + delta).max(MIN_CELL_WIDTH);

            match state.target {
                ResizeTarget::Column(col) => {
                    self.column_widths[col] = new_size.max(MIN_CELL_WIDTH);
                }
                ResizeTarget::Row(row) => {
                    self.row_heights[row] = new_size.max(MIN_CELL_HEIGHT);
                }
            }
            cx.notify();
        }
    }

    /// End resize operation
    fn end_resize(&mut self, cx: &mut Context<Self>) {
        self.resize_state = None;
        self.file_state.mark_dirty();
        cx.notify();
    }

    /// Handle column header mouse down - start resize or double-click auto-fit
    fn on_column_header_mouse_down(&mut self, event: &MouseDownEvent, header_x: f32, cx: &mut Context<Self>) {
        // x position relative to column header area (after row header)
        let x = f32::from(event.position.x) - ROW_HEADER_WIDTH - header_x;

        if let Some(col) = self.column_resize_target(x) {
            if event.click_count == 2 {
                // Double-click: auto-fit column
                self.auto_fit_column(col, cx);
            } else {
                // Single click: start resize
                self.start_column_resize(col, f32::from(event.position.x), cx);
            }
        }
    }

    /// Handle row header mouse down - start resize or double-click auto-fit
    fn on_row_header_mouse_down(&mut self, event: &MouseDownEvent, header_y: f32, cx: &mut Context<Self>) {
        // y position relative to row area (after column header)
        let y = f32::from(event.position.y) - COLUMN_HEADER_HEIGHT - HEADER_HEIGHT - header_y;

        if let Some(row) = self.row_resize_target(y) {
            if event.click_count == 2 {
                // Double-click: auto-fit row
                self.auto_fit_row(row, cx);
            } else {
                // Single click: start resize
                self.start_row_resize(row, f32::from(event.position.y), cx);
            }
        }
    }

    // === Auto-fit methods (implemented in Phase 5) ===

    /// Auto-fit a column width to its content
    fn auto_fit_column(&mut self, col: usize, cx: &mut Context<Self>) {
        // Find the maximum content width in this column
        let mut max_width = DEFAULT_CELL_WIDTH;
        for row in 0..GRID_ROWS {
            let content = &self.cells[row][col];
            if !content.is_empty() {
                // Estimate width: approximately 8 pixels per character + padding
                let estimated_width = content.len() as f32 * 8.0 + 16.0;
                max_width = max_width.max(estimated_width);
            }
        }
        self.column_widths[col] = max_width.max(DEFAULT_CELL_WIDTH);
        self.file_state.mark_dirty();
        cx.notify();
    }

    /// Auto-fit a row height to its content
    fn auto_fit_row(&mut self, row: usize, cx: &mut Context<Self>) {
        // For now, use default height. Multiline support will improve this.
        let mut max_height = DEFAULT_CELL_HEIGHT;
        for col in 0..GRID_COLS {
            let content = &self.cells[row][col];
            if !content.is_empty() {
                // Count newlines to determine height
                let line_count = content.lines().count().max(1);
                let estimated_height = line_count as f32 * 20.0 + 8.0;
                max_height = max_height.max(estimated_height);
            }
        }
        self.row_heights[row] = max_height.max(DEFAULT_CELL_HEIGHT);
        self.file_state.mark_dirty();
        cx.notify();
    }

    /// Auto-fit all columns and rows
    fn auto_fit_all(&mut self, cx: &mut Context<Self>) {
        for col in 0..GRID_COLS {
            let mut max_width = DEFAULT_CELL_WIDTH;
            for row in 0..GRID_ROWS {
                let content = &self.cells[row][col];
                if !content.is_empty() {
                    let estimated_width = content.len() as f32 * 8.0 + 16.0;
                    max_width = max_width.max(estimated_width);
                }
            }
            self.column_widths[col] = max_width.max(DEFAULT_CELL_WIDTH);
        }
        for row in 0..GRID_ROWS {
            let mut max_height = DEFAULT_CELL_HEIGHT;
            for col in 0..GRID_COLS {
                let content = &self.cells[row][col];
                if !content.is_empty() {
                    let line_count = content.lines().count().max(1);
                    let estimated_height = line_count as f32 * 20.0 + 8.0;
                    max_height = max_height.max(estimated_height);
                }
            }
            self.row_heights[row] = max_height.max(DEFAULT_CELL_HEIGHT);
        }
        self.file_state.mark_dirty();
        cx.notify();
    }

    /// Reset all column widths and row heights to defaults
    fn reset_all_sizes(&mut self, cx: &mut Context<Self>) {
        self.column_widths = vec![DEFAULT_CELL_WIDTH; GRID_COLS];
        self.row_heights = vec![DEFAULT_CELL_HEIGHT; GRID_ROWS];
        self.file_state.mark_dirty();
        cx.notify();
    }

    // === Watch mode methods ===

    /// Toggle auto-fit watch mode for all cells
    fn toggle_autofit_watch_all(&mut self, cx: &mut Context<Self>) {
        self.autofit_watch = match &self.autofit_watch {
            AutoFitWatch::All => AutoFitWatch::None,
            _ => AutoFitWatch::All,
        };
        cx.notify();
    }

    /// Toggle auto-fit watch for a specific column
    fn toggle_autofit_watch_column(&mut self, col: usize, cx: &mut Context<Self>) {
        match &mut self.autofit_watch {
            AutoFitWatch::Columns(cols) => {
                if cols.contains(&col) {
                    cols.remove(&col);
                    if cols.is_empty() {
                        self.autofit_watch = AutoFitWatch::None;
                    }
                } else {
                    cols.insert(col);
                }
            }
            AutoFitWatch::None => {
                let mut cols = HashSet::new();
                cols.insert(col);
                self.autofit_watch = AutoFitWatch::Columns(cols);
            }
            _ => {
                // If All or Rows mode, switch to just this column
                let mut cols = HashSet::new();
                cols.insert(col);
                self.autofit_watch = AutoFitWatch::Columns(cols);
            }
        }
        cx.notify();
    }

    /// Toggle auto-fit watch for a specific row
    fn toggle_autofit_watch_row(&mut self, row: usize, cx: &mut Context<Self>) {
        match &mut self.autofit_watch {
            AutoFitWatch::Rows(rows) => {
                if rows.contains(&row) {
                    rows.remove(&row);
                    if rows.is_empty() {
                        self.autofit_watch = AutoFitWatch::None;
                    }
                } else {
                    rows.insert(row);
                }
            }
            AutoFitWatch::None => {
                let mut rows = HashSet::new();
                rows.insert(row);
                self.autofit_watch = AutoFitWatch::Rows(rows);
            }
            _ => {
                // If All or Columns mode, switch to just this row
                let mut rows = HashSet::new();
                rows.insert(row);
                self.autofit_watch = AutoFitWatch::Rows(rows);
            }
        }
        cx.notify();
    }

    /// Check if auto-fit should be applied for a cell, and apply it
    fn check_autofit_watch(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        match &self.autofit_watch {
            AutoFitWatch::None => {}
            AutoFitWatch::All => {
                self.auto_fit_column(col, cx);
                self.auto_fit_row(row, cx);
            }
            AutoFitWatch::Columns(cols) => {
                if cols.contains(&col) {
                    self.auto_fit_column(col, cx);
                }
            }
            AutoFitWatch::Rows(rows) => {
                if rows.contains(&row) {
                    self.auto_fit_row(row, cx);
                }
            }
        }
    }

    // === Scroll wheel / trackpad ===

    fn handle_scroll_wheel(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match event.delta {
            ScrollDelta::Lines(delta) => {
                // Mouse wheel: jump by whole cells
                self.scroll_offset_x = 0.0;
                self.scroll_offset_y = 0.0;

                let row_delta = -delta.y.round() as isize;
                let col_delta = -delta.x.round() as isize;

                self.scroll_row = (self.scroll_row as isize + row_delta)
                    .max(0)
                    .min((GRID_ROWS - 1) as isize) as usize;
                self.scroll_col = (self.scroll_col as isize + col_delta)
                    .max(0)
                    .min((GRID_COLS - 1) as isize) as usize;
            }
            ScrollDelta::Pixels(delta) => {
                // Trackpad: smooth pixel scrolling
                self.apply_smooth_scroll(f32::from(-delta.x), f32::from(-delta.y));
            }
        }

        if self.keep_cursor_in_view {
            self.clamp_cursor_to_viewport();
        }

        cx.notify();
    }

    fn apply_smooth_scroll(&mut self, dx: f32, dy: f32) {
        // Accumulate vertical offset
        self.scroll_offset_y += dy;

        // Carry over to next/previous rows
        while self.scroll_offset_y >= self.row_heights[self.scroll_row]
            && self.scroll_row < GRID_ROWS - 1
        {
            self.scroll_offset_y -= self.row_heights[self.scroll_row];
            self.scroll_row += 1;
        }
        while self.scroll_offset_y < 0.0 && self.scroll_row > 0 {
            self.scroll_row -= 1;
            self.scroll_offset_y += self.row_heights[self.scroll_row];
        }

        // Accumulate horizontal offset
        self.scroll_offset_x += dx;

        // Carry over to next/previous columns
        while self.scroll_offset_x >= self.column_widths[self.scroll_col]
            && self.scroll_col < GRID_COLS - 1
        {
            self.scroll_offset_x -= self.column_widths[self.scroll_col];
            self.scroll_col += 1;
        }
        while self.scroll_offset_x < 0.0 && self.scroll_col > 0 {
            self.scroll_col -= 1;
            self.scroll_offset_x += self.column_widths[self.scroll_col];
        }

        self.clamp_scroll_position();
    }

    fn clamp_scroll_position(&mut self) {
        // Clamp at top/left edges
        if self.scroll_row == 0 && self.scroll_offset_y < 0.0 {
            self.scroll_offset_y = 0.0;
        }
        if self.scroll_col == 0 && self.scroll_offset_x < 0.0 {
            self.scroll_offset_x = 0.0;
        }
        // Clamp at bottom/right edges
        if self.scroll_row >= GRID_ROWS - 1 {
            self.scroll_row = GRID_ROWS - 1;
            if self.scroll_offset_y > 0.0 {
                self.scroll_offset_y = 0.0;
            }
        }
        if self.scroll_col >= GRID_COLS - 1 {
            self.scroll_col = GRID_COLS - 1;
            if self.scroll_offset_x > 0.0 {
                self.scroll_offset_x = 0.0;
            }
        }
    }

    /// Move the cursor into the fully visible viewport (used when keep_cursor_in_view is enabled)
    fn clamp_cursor_to_viewport(&mut self) {
        // First fully visible row: if pixel offset hides part of scroll_row, skip it
        let first_full_row = if self.scroll_offset_y > 0.0 {
            (self.scroll_row + 1).min(GRID_ROWS - 1)
        } else {
            self.scroll_row
        };
        let last_full_row = self.last_fully_visible_row();

        if self.selected.row < first_full_row {
            self.selected.row = first_full_row;
        } else if self.selected.row > last_full_row {
            self.selected.row = last_full_row;
        }

        let first_full_col = if self.scroll_offset_x > 0.0 {
            (self.scroll_col + 1).min(GRID_COLS - 1)
        } else {
            self.scroll_col
        };
        let last_full_col = self.last_fully_visible_col();

        if self.selected.col < first_full_col {
            self.selected.col = first_full_col;
        } else if self.selected.col > last_full_col {
            self.selected.col = last_full_col;
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
        let entity = cx.entity().clone();
        let end_col = (self.scroll_col + self.visible_cols).min(GRID_COLS);
        let column_widths = self.column_widths.clone();
        let selected_col = self.selected.col;
        let offset_x = self.scroll_offset_x;

        div()
            .id("column-headers")
            .flex()
            .flex_row()
            .h(px(COLUMN_HEADER_HEIGHT))
            .bg(theme.mantle)
            .border_b_1()
            .border_color(theme.surface0)
            .on_mouse_down(MouseButton::Left, {
                let entity = entity.clone();
                move |event, _window, app| {
                    entity.update(app, |grid, cx| {
                        grid.on_column_header_mouse_down(event, 0.0, cx);
                    });
                }
            })
            .on_mouse_move({
                let entity = entity.clone();
                move |event, _window, app| {
                    entity.update(app, |grid, cx| {
                        if grid.resize_state.is_some() {
                            grid.update_resize(f32::from(event.position.x), cx);
                        }
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let entity = entity.clone();
                move |_event, _window, app| {
                    entity.update(app, |grid, cx| {
                        if grid.resize_state.is_some() {
                            grid.end_resize(cx);
                        }
                    });
                }
            })
            .child(
                // Empty corner cell
                div()
                    .w(px(ROW_HEADER_WIDTH))
                    .h_full()
                    .flex_none()
                    .border_r_1()
                    .border_color(theme.surface0)
            )
            .child(
                // Clipped container for column headers with horizontal scroll offset
                div()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .h_full()
                            .ml(px(-offset_x))
                            .children(
                                (self.scroll_col..end_col).map(move |col| {
                                    let col_letter = CellPosition::new(0, col).to_reference();
                                    let col_letter: String = col_letter.chars().take_while(|c| c.is_alphabetic()).collect();
                                    let is_selected = col == selected_col;
                                    let col_width = column_widths[col];

                                    div()
                                        .w(px(col_width))
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
                    )
            )
    }

    fn render_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let entity = cx.entity().clone();
        let end_row = (self.scroll_row + self.visible_rows).min(GRID_ROWS);
        let end_col = (self.scroll_col + self.visible_cols).min(GRID_COLS);
        let column_widths = self.column_widths.clone();
        let row_heights = self.row_heights.clone();
        let cells = self.cells.clone();
        let selected = self.selected;
        let mode = self.mode;
        let active_input = self.active_input.clone();
        let scroll_col = self.scroll_col;
        let offset_x = self.scroll_offset_x;
        let offset_y = self.scroll_offset_y;

        div()
            .id("grid-area")
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .on_mouse_move({
                let entity = entity.clone();
                move |event, _window, app| {
                    entity.update(app, |grid, cx| {
                        if grid.resize_state.is_some() {
                            match grid.resize_state.as_ref().unwrap().target {
                                ResizeTarget::Column(_) => {
                                    grid.update_resize(f32::from(event.position.x), cx);
                                }
                                ResizeTarget::Row(_) => {
                                    grid.update_resize(f32::from(event.position.y), cx);
                                }
                            }
                        }
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let entity = entity.clone();
                move |_event, _window, app| {
                    entity.update(app, |grid, cx| {
                        if grid.resize_state.is_some() {
                            grid.end_resize(cx);
                        }
                    });
                }
            })
            .child(
                // Inner container with vertical scroll offset
                div()
                    .flex()
                    .flex_col()
                    .mt(px(-offset_y))
                    .children(
                        (self.scroll_row..end_row).map(move |row| {
                            let is_row_selected = row == selected.row;
                            let row_height = row_heights[row];
                            let column_widths = column_widths.clone();
                            let cells = cells.clone();
                            let entity = entity.clone();
                            let active_input = active_input.clone();

                            div()
                                .flex()
                                .flex_row()
                                .h(px(row_height))
                                .child({
                                    // Row header with resize handling
                                    let entity = entity.clone();
                                    div()
                                        .id(ElementId::Name(format!("row-header-{}", row).into()))
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
                                        .on_mouse_down(MouseButton::Left, {
                                            move |event, _window, app| {
                                                entity.update(app, |grid, cx| {
                                                    grid.on_row_header_mouse_down(event, 0.0, cx);
                                                });
                                            }
                                        })
                                        .child(format!("{}", row + 1))
                                })
                                .child(
                                    // Clipped container for cells with horizontal scroll offset
                                    div()
                                        .flex_1()
                                        .h_full()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .h_full()
                                                .ml(px(-offset_x))
                                                .children(
                                                    (scroll_col..end_col).map(move |col| {
                                                        let is_selected = row == selected.row && col == selected.col;
                                                        let content = cells[row][col].clone();
                                                        let col_width = column_widths[col];
                                                        let entity = entity.clone();

                                                        if is_selected && mode == Mode::Edit {
                                                            // Render the active input for selected cell in edit mode
                                                            div()
                                                                .id(ElementId::Name(format!("cell-edit-{}-{}", row, col).into()))
                                                                .w(px(col_width))
                                                                .h(px(row_height))
                                                                .flex_none()
                                                                .border_2()
                                                                .border_color(theme.accent)
                                                                .overflow_hidden()
                                                                .child(active_input.clone())
                                                        } else {
                                                            // Render static cell with multiline support
                                                            let has_newlines = content.contains('\n');
                                                            div()
                                                                .id(ElementId::Name(format!("cell-{}-{}", row, col).into()))
                                                                .w(px(col_width))
                                                                .h(px(row_height))
                                                                .flex_none()
                                                                .flex()
                                                                .flex_col()
                                                                .when(!has_newlines, |d| d.items_center().justify_center())
                                                                .when(has_newlines, |d| d.items_start().pt(px(2.)))
                                                                .px(px(4.))
                                                                .border_r_1()
                                                                .border_b_1()
                                                                .border_color(if is_selected { theme.accent } else { theme.surface0 })
                                                                .when(is_selected, |d| d.border_2())
                                                                .bg(if is_selected { theme.surface0 } else { theme.base })
                                                                .text_size(px(14.))
                                                                .overflow_hidden()
                                                                .on_mouse_down(MouseButton::Left, {
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
                                                                .when(!has_newlines, |d| d.child(content.clone()))
                                                                .when(has_newlines, |d| {
                                                                    d.children(content.lines().map(|line| {
                                                                        div()
                                                                            .w_full()
                                                                            .line_height(px(18.))
                                                                            .child(line.to_string())
                                                                    }))
                                                                })
                                                        }
                                                    })
                                                )
                                        )
                                )
                        })
                    )
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
        self.grid_height = f32::from(content_bounds.height) - HEADER_HEIGHT - COLUMN_HEADER_HEIGHT - FOOTER_HEIGHT;
        self.grid_width = f32::from(content_bounds.width) - ROW_HEADER_WIDTH;

        // Calculate visible rows by summing row heights from scroll position
        self.visible_rows = self.calculate_visible_rows(self.grid_height);
        self.visible_cols = self.calculate_visible_cols(self.grid_width);

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
            .id("spreadsheet-root")
            .flex()
            .flex_col()
            .size_full()
            .key_context(key_context)
            .track_focus(&self.focus_handle)
            .on_scroll_wheel(cx.listener(Self::handle_scroll_wheel))
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
            .on_action(cx.listener(Self::toggle_keep_cursor_in_view))
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
