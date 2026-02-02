mod assets;
mod cell;
mod command_palette;
mod file_io;
mod file_state;
mod grid;
mod menu;
mod state;
mod theme;

use gpui::*;

use assets::Assets;
use cell::*;
use command_palette::*;
use grid::*;
use theme::Theme;

fn main() {
    Application::new()
        .with_assets(Assets)
        .run(|cx| {
            // Initialize theme
            Theme::init(cx);

            // Set up menu bar
            menu::setup_menu(cx);

            // Register keybindings
            cx.bind_keys([
                // Normal mode navigation
                KeyBinding::new("up", MoveUp, Some("NormalMode")),
                KeyBinding::new("down", MoveDown, Some("NormalMode")),
                KeyBinding::new("left", MoveLeft, Some("NormalMode")),
                KeyBinding::new("right", MoveRight, Some("NormalMode")),
                KeyBinding::new("k", MoveUp, Some("NormalMode")),
                KeyBinding::new("j", MoveDown, Some("NormalMode")),
                KeyBinding::new("h", MoveLeft, Some("NormalMode")),
                KeyBinding::new("l", MoveRight, Some("NormalMode")),
                KeyBinding::new("i", EnterEditMode, Some("NormalMode")),

                // Edit mode
                KeyBinding::new("escape", ExitEditMode, Some("EditMode")),
                KeyBinding::new("backspace", Backspace, Some("CellInput")),
                KeyBinding::new("delete", Delete, Some("CellInput")),

                // Text editing in CellInput
                KeyBinding::new("left", Left, Some("CellInput")),
                KeyBinding::new("right", Right, Some("CellInput")),
                KeyBinding::new("shift-left", SelectLeft, Some("CellInput")),
                KeyBinding::new("shift-right", SelectRight, Some("CellInput")),
                KeyBinding::new("cmd-a", SelectAll, Some("CellInput")),
                KeyBinding::new("home", Home, Some("CellInput")),
                KeyBinding::new("end", End, Some("CellInput")),
                KeyBinding::new("cmd-left", Home, Some("CellInput")),
                KeyBinding::new("cmd-right", End, Some("CellInput")),
                KeyBinding::new("alt-left", WordLeft, Some("CellInput")),
                KeyBinding::new("alt-right", WordRight, Some("CellInput")),
                KeyBinding::new("alt-shift-left", SelectWordLeft, Some("CellInput")),
                KeyBinding::new("alt-shift-right", SelectWordRight, Some("CellInput")),
                KeyBinding::new("cmd-backspace", DeleteToStart, Some("CellInput")),
                KeyBinding::new("alt-backspace", DeleteWordBackward, Some("CellInput")),
                KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some("CellInput")),
                KeyBinding::new("cmd-v", Paste, Some("CellInput")),
                KeyBinding::new("cmd-c", Copy, Some("CellInput")),
                KeyBinding::new("cmd-x", Cut, Some("CellInput")),

                // Command palette
                KeyBinding::new("cmd-k", ShowCommandPalette, Some("NormalMode")),
                KeyBinding::new("shift-;", ShowCommandPalette, Some("NormalMode")), // : key
                KeyBinding::new("escape", HideCommandPalette, Some("CommandPalette")),
                KeyBinding::new("up", SelectPrevious, Some("CommandPalette")),
                KeyBinding::new("down", SelectNext, Some("CommandPalette")),
                KeyBinding::new("enter", Confirm, Some("CommandPalette")),

                // File operations
                KeyBinding::new("cmd-n", NewFile, Some("NormalMode")),
                KeyBinding::new("cmd-o", OpenFile, Some("NormalMode")),
                KeyBinding::new("cmd-s", SaveFile, Some("NormalMode")),
                KeyBinding::new("cmd-shift-s", SaveFileAs, Some("NormalMode")),
                KeyBinding::new("cmd-w", CloseFile, Some("NormalMode")),

                // Global
                KeyBinding::new("cmd-q", Quit, None),
            ]);

            // Register quit action
            cx.on_action::<Quit>(|_, cx| {
                cx.quit();
            });

            // Create the main window
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1000.), px(700.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("zsheets".into()),
                    appears_transparent: false,
                    ..Default::default()
                }),
                window_min_size: Some(size(px(MIN_WINDOW_WIDTH), px(MIN_WINDOW_HEIGHT))),
                ..Default::default()
            };

            cx.open_window(window_options, |_window, cx| {
                cx.new(|cx| SpreadsheetApp::new(cx))
            })
            .unwrap();
        });
}
