# GPUI Spreadsheet

A 100x100 cell spreadsheet with vim-style modal editing, built with GPUI.

## IMPORTANT: Check TODO.md on Load

**Before starting any work, check `TODO.md` for known issues and pending tasks.**

## After Every Code Change

After making any successful code edit, you MUST:
1. Run `cargo build` to verify compilation
2. If build succeeds, run `./build.sh` to build release and deploy to ~/Applications/zsheets.app

## Build & Run

```bash
cargo build
cargo run
```

## Working with GPUI

### Project Setup

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed", package = "gpui" }

[target.'cfg(target_os = "macos")'.dependencies]
core-text = "=21.0.0"  # Pin to avoid version conflicts
objc2 = "0.6"
objc2-app-kit = { version = "0.3", features = ["NSColor", "NSColorSpace"] }
```

### Core Concepts

**Entities** - Stateful components managed by GPUI:
```rust
struct MyComponent {
    focus_handle: FocusHandle,
    // state...
}

impl MyComponent {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}
```

**Render trait** - Components implement this to define their UI:
```rust
impl Render for MyComponent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("Hello")
    }
}
```

**Focusable trait** - For keyboard input:
```rust
impl Focusable for MyComponent {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

### Actions & Keybindings

Define actions:
```rust
actions!(my_module, [MyAction, AnotherAction]);
```

Bind keys (in main):
```rust
cx.bind_keys([
    KeyBinding::new("up", MoveUp, Some("MyContext")),
    KeyBinding::new("cmd-q", Quit, None),  // None = global
]);
```

Handle actions in render:
```rust
div()
    .key_context("MyContext")
    .track_focus(&self.focus_handle)
    .on_action(cx.listener(Self::handle_my_action))
```

Action handler signature:
```rust
fn handle_my_action(&mut self, _: &MyAction, window: &mut Window, cx: &mut Context<Self>) {
    // handle action
    cx.notify();  // trigger re-render
}
```

### Focus Management

```rust
// Focus an element
focus_handle.focus(window, cx);

// Check if focused
if focus_handle.is_focused(window) { ... }
```

### Styling (Tailwind-like)

```rust
div()
    .flex()
    .flex_col()
    .size_full()
    .bg(theme.base)
    .text_color(theme.text)
    .px(px(8.))
    .gap(px(4.))
    .border_1()
    .border_color(theme.surface0)
    .rounded(px(4.))
    .overflow_hidden()
```

### Conditional Styling

Import the trait:
```rust
use gpui::prelude::FluentBuilder;
```

Use `.when()`:
```rust
div()
    .when(is_selected, |d| d.bg(theme.accent))
```

### Mouse Events

```rust
div()
    .id("my-element")  // Required for mouse events
    .on_mouse_down(MouseButton::Left, |event, window, cx| {
        if event.click_count == 2 {
            // double click
        }
    })
```

### Text Input (EntityInputHandler)

For custom text input, implement `EntityInputHandler`:
```rust
impl EntityInputHandler for MyInput {
    fn selected_text_range(...) -> Option<UTF16Selection> { ... }
    fn replace_text_in_range(...) { ... }
    // etc.
}
```

Register input handler in Element::paint:
```rust
window.handle_input(
    &focus_handle,
    ElementInputHandler::new(bounds, self.input.clone()),
    cx,
);
```

### Custom Elements

For complex rendering (cursors, selections), implement `Element`:
```rust
impl Element for MyElement {
    type RequestLayoutState = ();
    type PrepaintState = MyPrepaintState;

    fn request_layout(...) -> (LayoutId, ()) { ... }
    fn prepaint(...) -> MyPrepaintState { ... }
    fn paint(...) { ... }
}
```

### Global State

```rust
struct Theme { ... }
impl Global for Theme {}

// Set in main
app.set_global(Theme::get_dark());

// Access anywhere
let theme = cx.global::<Theme>();
```

### Async Operations

```rust
cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
    cx.background_executor().timer(Duration::from_millis(100)).await;
    this.update(cx, |this, cx| {
        // update state
        cx.notify();
    }).ok();
}).detach();
```

### Common Patterns

**Virtualization** - Only render visible items:
```rust
let visible_range = scroll_offset..(scroll_offset + visible_count);
div().children(
    visible_range.map(|i| render_item(i))
)
```

**Entity updates**:
```rust
self.child_entity.update(cx, |child, cx| {
    child.set_value(new_value);
    cx.notify();
});
```

**Reading entity state**:
```rust
let value = self.child_entity.read(cx).get_value();
```

### Debugging

- Use `cx.notify()` to trigger re-renders after state changes
- Check key contexts match between bindings and `key_context()`
- Ensure `track_focus()` is called for keyboard input to work
- Use `.id()` on elements that need mouse events or state

## Feature TODO

### Core Editing
- [ ] Cell range selection (shift+click, shift+arrow, drag)
- [ ] Copy/paste/cut for cells and ranges
- [ ] Undo/redo history
- [ ] Delete cell contents in normal mode
- [ ] Fill down/right

### Formulas
- [ ] Formula engine (`=SUM(A1:A10)`, `=A1+B2`, etc.)
- [ ] Cell references and dependency graph
- [ ] Auto-recalculation on edit
- [ ] Common functions (SUM, AVG, MIN, MAX, COUNT, IF, VLOOKUP)
- [ ] Relative vs absolute references (`$A$1`)
- [ ] Circular reference detection

### Formatting
- [ ] Bold, italic, underline per cell
- [ ] Text alignment (left/center/right)
- [ ] Number formatting (currency, percentage, dates, decimals)
- [ ] Cell background colors
- [ ] Font size per cell
- [ ] Cell borders (custom per-edge)
- [ ] Conditional formatting
- [ ] Merge cells

### Data Management
- [ ] Sort by column
- [ ] Filter rows by value or condition
- [ ] Find and replace
- [ ] Multiple sheets/tabs
- [ ] Named ranges
- [ ] Freeze rows/columns (split panes)
- [ ] Hide/show rows and columns

### Navigation
- [ ] Goto cell (ctrl+g or `:goto A50`)
- [ ] Page up/down, ctrl+home/end
- [ ] Vim motions: `gg`, `G`, `0`, `$`, `w`, `b`
- [ ] Visual mode for range selection (vim `v`)

### Import/Export
- [ ] TSV import/export
- [ ] Excel (.xlsx) read/write
- [ ] JSON export
- [ ] Print / export to PDF

### Polish
- [ ] Status bar showing SUM/AVG/COUNT of selection
- [ ] Column/row insert and delete
- [ ] Drag and drop cells
- [ ] Auto-fill series (1,2,3... or Mon,Tue,Wed...)
- [ ] Cell comments/notes
- [ ] Wrap text toggle per cell
