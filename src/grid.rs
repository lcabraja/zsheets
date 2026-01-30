use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::cell::CellInput;
use crate::state::{CellPosition, Mode, GRID_COLS, GRID_ROWS};
use crate::Theme;

pub const CELL_WIDTH: f32 = 100.0;
pub const CELL_HEIGHT: f32 = 28.0;
pub const ROW_HEADER_WIDTH: f32 = 50.0;
pub const COLUMN_HEADER_HEIGHT: f32 = 24.0;

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
}

impl SpreadsheetGrid {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let active_input = cx.new(|cx| CellInput::new(cx));

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
        self.cells[self.selected.row][self.selected.col] = content;

        self.mode = Mode::Normal;
        self.focus_handle.focus(window, cx);
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
            .h(px(32.))
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
                                        .text_ellipsis()
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

        div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(24.))
            .bg(theme.mantle)
            .border_t_1()
            .border_color(theme.surface0)
            .items_center()
            .px(px(8.))
            .text_size(px(12.))
            .text_color(theme.subtext0)
            .font_weight(FontWeight::BOLD)
            .child(mode_text)
    }
}

impl Render for SpreadsheetGrid {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Calculate visible rows and columns based on window size
        let content_bounds = window.viewport_size();
        let grid_height = f32::from(content_bounds.height) - 32.0 - COLUMN_HEADER_HEIGHT - 24.0; // header + col headers + footer
        let grid_width = f32::from(content_bounds.width) - ROW_HEADER_WIDTH;

        self.visible_rows = ((grid_height / CELL_HEIGHT).floor() as usize).max(1);
        self.visible_cols = ((grid_width / CELL_WIDTH).floor() as usize).max(1);

        // Ensure selection is still visible after resize
        self.ensure_visible();

        let key_context = if self.mode == Mode::Edit { "EditMode" } else { "NormalMode" };

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
            .child(self.render_header(cx))
            .child(self.render_column_headers(cx))
            .child(self.render_grid(cx))
            .child(self.render_footer(cx))
    }
}

impl Focusable for SpreadsheetGrid {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
