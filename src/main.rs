mod assets;
mod cell;
mod grid;
mod state;
mod theme;

use gpui::*;

use assets::Assets;
use cell::*;
use grid::*;
use theme::Theme;

fn main() {
    Application::new()
        .with_assets(Assets)
        .run(|cx| {
            // Initialize theme
            Theme::init(cx);

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
                ..Default::default()
            };

            cx.open_window(window_options, |_window, cx| {
                cx.new(|cx| SpreadsheetApp::new(cx))
            })
            .unwrap();
        });
}
