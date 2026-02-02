use gpui::*;

use crate::grid::{
    CloseFile, ForceWrite, NewFile, OpenFile, Quit, SaveFile, SaveFileAs,
    ToggleReadOnly,
};

/// Set up the application menu bar
pub fn setup_menu(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "zsheets".into(),
            items: vec![
                MenuItem::action("About zsheets", About),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New", NewFile),
                MenuItem::separator(),
                MenuItem::action("Open...", OpenFile),
                MenuItem::separator(),
                MenuItem::action("Save", SaveFile),
                MenuItem::action("Save As...", SaveFileAs),
                MenuItem::action("Force Write", ForceWrite),
                MenuItem::separator(),
                MenuItem::action("Close", CloseFile),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", Undo),
                MenuItem::action("Redo", Redo),
                MenuItem::separator(),
                MenuItem::action("Cut", Cut),
                MenuItem::action("Copy", Copy),
                MenuItem::action("Paste", Paste),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Read-Only", ToggleReadOnly),
            ],
        },
    ]);
}

// Menu-specific actions that don't fit elsewhere
actions!(menu, [About, Undo, Redo, Cut, Copy, Paste]);
