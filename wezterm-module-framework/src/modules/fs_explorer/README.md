# FsExplorerPane Implementation

## Overview

The `FsExplorerPane` is a complete implementation of a terminal-based filesystem explorer for WezTerm, following the `TermWizTerminalPane` pattern. It provides an interactive file browser with vim-style keybindings that integrates seamlessly into WezTerm's pane system.

## Architecture

### Core Components

1. **FsExplorerPane** (`pane.rs`)
   - Implements the `Pane` trait from `mux::pane`
   - Uses `wezterm_term::Terminal` for rendering
   - Manages state through `FsExplorerState`
   - Handles keyboard/mouse input via crossbeam channels

2. **FsExplorerState** (internal to `pane.rs`)
   - Manages current directory navigation
   - Tracks file entries (name, type, size)
   - Handles selection state and scrolling
   - Provides directory loading and error handling

3. **FsExplorerModule** (`mod.rs`)
   - Implements the `Module` trait
   - Provides lifecycle management (init, start, stop)
   - Declares required capabilities (FILESYSTEM_READ, UI_CREATE_PANE)
   - Enables Lua API registration

## Key Features

### Navigation
- **j/k or ↓/↑**: Move cursor down/up
- **Enter**: Open directory or file
- **h or ←**: Go to parent directory
- **g**: Go to top of list
- **G (Shift+g)**: Go to bottom of list
- **Ctrl+d**: Page down
- **Ctrl+u**: Page up

### Operations
- **.**: Toggle hidden files visibility
- **q or Escape**: Quit the file explorer

### Display
- **File icons**: 📁 for directories, 📄 for files
- **File sizes**: Formatted (B, KB, MB, GB)
- **Directory path**: Shown in title bar
- **Status line**: Shows item count and keybindings
- **Selection highlight**: Reverse video for current selection
- **Error messages**: Displayed in red when directory read fails

## Implementation Details

### Pane Trait Implementation

The `FsExplorerPane` implements all required `Pane` trait methods:

```rust
// Core identification and state
fn pane_id(&self) -> PaneId
fn domain_id(&self) -> DomainId
fn is_dead(&self) -> bool
fn kill(&self)

// Display and rendering
fn get_title(&self) -> String
fn get_dimensions(&self) -> RenderableDimensions
fn get_cursor_position(&self) -> StableCursorPosition
fn get_lines(&self, Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>)
fn palette(&self) -> ColorPalette

// Input handling
fn key_down(&self, key: KeyCode, mods: KeyModifiers) -> Result<()>
fn key_up(&self, key: KeyCode, mods: KeyModifiers) -> Result<()>
fn mouse_event(&self, event: MouseEvent) -> Result<()>

// Terminal operations
fn resize(&self, size: TerminalSize) -> Result<()>
fn reader(&self) -> Result<Option<Box<dyn Read + Send>>>
fn writer(&self) -> MappedMutexGuard<'_, dyn Write>

// Configuration
fn set_config(&self, Arc<dyn TerminalConfiguration>)
fn get_config(&self) -> Option<Arc<dyn TerminalConfiguration>>
```

### Terminal Rendering

The pane uses `termwiz` actions to render content:

1. **Clear screen** at start of each render
2. **Title bar** with current directory path (blue, bold)
3. **Separator line** (horizontal rule)
4. **Error messages** (if any, shown in red)
5. **File list** with:
   - Selection indicator (reverse video)
   - File/directory icons
   - File names (left-aligned, 50 chars)
   - File sizes (right-aligned, 10 chars)
6. **Status bar** (blue background) with:
   - Item count
   - Keybinding hints

### State Management

```rust
struct FsExplorerState {
    current_dir: PathBuf,          // Current directory path
    entries: Vec<FileEntry>,       // List of files/directories
    selected_index: usize,         // Current selection
    scroll_offset: usize,          // Viewport scroll position
    show_hidden: bool,             // Show hidden files flag
    error_message: Option<String>, // Last error message
}
```

### Input Processing

Input events are sent through a crossbeam channel:

```rust
pub enum FsExplorerInput {
    Key(KeyCode, KeyModifiers),
    Mouse(MouseEvent),
    Resize { rows: usize, cols: usize },
}
```

The pane handles input in `key_down()` which:
1. Locks the state
2. Processes the key command
3. Updates state (selection, directory, etc.)
4. Updates scroll offset to keep selection visible
5. Re-renders the terminal

### Microsoft Rust Guidelines Compliance

The implementation follows Microsoft's Pragmatic Rust Guidelines:

- **M-CONCISE-NAMES**: No weasel words (Service, Manager, Factory)
- **M-PANIC-IS-STOP**: Uses `Result<()>` for recoverable errors
- **M-PUBLIC-DEBUG**: All public types implement `Debug` (via derives)
- **M-THROUGHPUT**: Efficient rendering with minimal allocations
- **M-UNSAFE**: No `unsafe` code used

## Usage Example

```rust
use wezterm_module_framework::modules::fs_explorer::{allocate_fs_explorer_pane, FsExplorerInput};
use std::path::PathBuf;

// Create a filesystem explorer pane
let (input_rx, pane) = allocate_fs_explorer_pane(
    domain_id,
    TerminalSize::new(80, 24, 800, 600),
    PathBuf::from("/home/user"),
    None, // Use default terminal config
)?;

// The pane is ready to be added to the mux
mux.add_pane(&pane)?;

// Process input events (usually done in a background thread)
while let Ok(event) = input_rx.recv() {
    match event {
        FsExplorerInput::Key(key, mods) => {
            // Already handled by pane.key_down()
        }
        _ => {}
    }
}
```

## Integration with Module Framework

The `FsExplorerModule` integrates with WezTerm's module system:

```rust
let mut module = FsExplorerModule::new(Some(PathBuf::from("/home/user")));

// Initialize and start the module
module.init(&context).await?;
module.start(&context).await?;

// Create panes as needed
let (input_rx, pane) = module.create_pane(domain_id, size, None, None)?;
```

## Testing

The implementation includes unit tests:

```bash
cargo test -p wezterm-module-framework --lib modules::fs_explorer
```

Tests cover:
- File size formatting
- State initialization
- Module lifecycle
- Capability declarations

## Future Enhancements

Potential improvements (not yet implemented):

1. **Git integration**: Show git status indicators
2. **File operations**: Copy, move, delete files
3. **Search/filter**: Fuzzy search for files
4. **Preview pane**: Show file contents
5. **Bookmarks**: Quick navigation to favorite directories
6. **Clipboard integration**: Copy file paths
7. **Custom icons**: Based on file extensions (from wezterm-fs-explorer)
8. **Mouse support**: Click to select, double-click to open
9. **Multi-select**: Visual mode for batch operations
10. **Sorting options**: By name, size, date

## Dependencies

- `mux`: Core multiplexer types (Pane, Domain)
- `wezterm_term`: Terminal emulation (Terminal, KeyCode)
- `termwiz`: Terminal UI primitives (Change, Action, Color)
- `crossbeam`: Channel communication
- `parking_lot`: Efficient Mutex
- `filedescriptor`: Pipe for terminal I/O
- `config`: WezTerm configuration
- `async_trait`: Async trait support

## File Structure

```
wezterm-module-framework/src/modules/fs_explorer/
├── mod.rs           # Module definition and FsExplorerModule
├── pane.rs          # FsExplorerPane implementation
└── README.md        # This file
```

## Performance Considerations

- **Lazy rendering**: Only renders visible viewport
- **Efficient sorting**: Single sort after directory load
- **Minimal allocations**: Reuses terminal buffer
- **Lock scope**: Minimal critical sections with Mutex
- **Channel buffering**: Uses unbounded channels to avoid blocking

## Error Handling

The implementation handles errors gracefully:

- **Directory read failures**: Displayed as error messages
- **Permission errors**: Shown but don't crash the pane
- **Invalid paths**: Caught and reported
- **Channel errors**: Mark pane as dead and return error

## Platform Support

The implementation is cross-platform and works on:
- **Windows**: Native path handling
- **Linux**: Full support
- **macOS**: Full support
- **BSD**: Should work (untested)

## Contributing

When extending this module:

1. Follow the Microsoft Rust Guidelines
2. Add unit tests for new functionality
3. Update this README with new features
4. Ensure no clippy warnings
5. Test on multiple platforms
6. Document public APIs
