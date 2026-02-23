# WezTerm Configuration Guide

## Overview

This WezTerm configuration is optimized for:
- **Windows cmd.exe compatibility** - Campbell color scheme, familiar keybindings
- **Warp-inspired features** - Modern terminal UI, workflows, command palette
- **Embedded systems development** - Serial console, debugging, cross-compilation
- **Cross-platform support** - Windows, WSL, Linux compatibility

## Installation

1. The configuration file is located at: `C:\Users\david\.wezterm.lua`
2. WezTerm will automatically load this configuration on startup
3. To reload after changes: Press `Ctrl+Alt+R` or restart WezTerm

## Visual Appearance

### Campbell Color Scheme
The configuration uses Windows cmd.exe's modern Campbell color scheme:
- **Background**: `#0C0C0C` (almost black)
- **Foreground**: `#CCCCCC` (light gray)
- **16 ANSI colors** optimized for modern displays

### Fonts
- **Primary**: Cascadia Mono (Windows Terminal default)
- **Fallback**: Consolas (legacy Windows Console)
- **Size**: 12pt (Windows Terminal standard)
- **Ligatures**: Enabled for modern coding fonts

### Window Settings
- **Initial size**: 120 columns × 30 rows (cmd.exe default)
- **Scrollback**: 9001 lines (cmd.exe buffer size)
- **Transparency**: Acrylic backdrop on Windows
- **Padding**: 8px on all sides

## Keybinding Reference

### Window and Tab Management

| Keybinding | Action |
|------------|--------|
| `Ctrl+Shift+N` | New window |
| `Ctrl+Shift+W` | Close current pane (with confirmation) |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+Shift+PageUp/Down` | Move tab left/right |
| `Alt+1` through `Alt+8` | Switch to specific tab |
| `Alt+9` | Show workspace launcher |

### Pane Splitting (Warp-inspired blocks)

| Keybinding | Action |
|------------|--------|
| `Ctrl+Shift+D` | Split horizontally (side by side) |
| `Ctrl+Shift+Alt+D` | Split vertically (top and bottom) |
| `Ctrl+Shift+Arrow` | Navigate between panes |
| `Ctrl+Shift+Alt+Arrow` | Resize current pane |
| `Ctrl+Alt+W` | Close current pane |
| `Ctrl+Shift+R` | Rotate panes clockwise |
| `Ctrl+Shift+Shift+R` | Rotate panes counter-clockwise |

### Copy/Paste (Windows-style)

| Keybinding | Action |
|------------|--------|
| `Ctrl+Shift+C` | Copy to clipboard (or send Ctrl+C if no selection) |
| `Ctrl+Shift+V` | Paste from clipboard |
| `Ctrl+Shift+X` | Enter copy mode (Vim-style navigation) |
| `Ctrl+Shift+Space` | Quick select mode |
| `Right Click` | Paste from clipboard |

### Search and Navigation

| Keybinding | Action |
|------------|--------|
| `Ctrl+Shift+F` | Search current pane |
| `Shift+PageUp/Down` | Scroll by page |
| `Shift+Home/End` | Scroll to top/bottom |
| `Ctrl+Shift+K` | Clear scrollback and viewport |

### Font Size Control

| Keybinding | Action |
|------------|--------|
| `Ctrl++` or `Ctrl+=` | Increase font size |
| `Ctrl+-` | Decrease font size |
| `Ctrl+0` | Reset font size |

### Command Palette (Warp-inspired)

| Keybinding | Action |
|------------|--------|
| `Ctrl+Shift+P` | Show launcher (fuzzy finder for everything) |
| `Ctrl+Shift+T` | Show tab navigator |
| `Ctrl+Shift+Alt+W` | Create new workspace |

### Miscellaneous

| Keybinding | Action |
|------------|--------|
| `Alt+Enter` | Toggle fullscreen |
| `Ctrl+Alt+R` | Reload configuration |
| `Ctrl+Shift+L` | Show debug overlay |
| `Ctrl+Click` | Open hyperlink at cursor |

### Leader Key (tmux-style)

Press `Ctrl+B` to activate the leader key (active for 1 second):

| Keybinding | Action |
|------------|--------|
| `Leader` then `\|` | Split horizontally |
| `Leader` then `-` | Split vertically |
| `Leader` then `D` | Detach domain |
| `Leader` then `Ctrl+B` | Send Ctrl+B to terminal |

## Warp-Inspired Features

### 1. Command Palette (`Ctrl+Shift+P`)
Fuzzy finder for:
- **Tabs**: Switch between open tabs
- **Launch Menu**: Start new shells (PowerShell, cmd, WSL)
- **Key Assignments**: Search available keybindings
- **Workspaces**: Switch between projects
- **Commands**: Execute WezTerm actions

### 2. Workspaces
Organize related terminal sessions:
- Create: `Ctrl+Shift+Alt+W` (creates with random name)
- Switch: `Alt+9` (shows workspace launcher)
- Status bar shows current workspace name

Example workflow:
1. Press `Alt+9` to open workspace launcher
2. Type "dev" to filter or create new workspace
3. Each workspace maintains separate tabs/panes

### 3. Tab Navigator (`Ctrl+Shift+T`)
Visual overview of all open tabs with preview:
- Thumbnails of each tab's content
- Process names and working directories
- Quick switching with arrow keys or mouse

### 4. Block-Based Panes
Split terminal into multiple panes like Warp blocks:
- Horizontal split: `Ctrl+Shift+D`
- Vertical split: `Ctrl+Shift+Alt+D`
- Each pane runs independently
- Navigate with `Ctrl+Shift+Arrow`

### 5. Quick Select (`Ctrl+Shift+Space`)
Smart selection mode:
- Automatically detects URLs, file paths, hashes
- Use arrow keys or type to filter
- Press Enter to copy selection

### 6. Smart Copy (`Ctrl+Shift+C`)
Context-aware copy:
- If text selected: copies to clipboard
- If no selection: sends Ctrl+C signal to terminal

## Shell Integration

### Available Shells (via Launch Menu)

Press `Ctrl+Shift+P` and type "launch" to access:

1. **PowerShell 7** - Modern PowerShell (default)
2. **PowerShell 5** - Built-in Windows PowerShell
3. **Command Prompt** - Classic cmd.exe
4. **WSL Ubuntu** - Windows Subsystem for Linux
5. **WSL Bash** - Direct bash shell in WSL
6. **Git Bash** - MinGW bash from Git for Windows

### Default Shell

The configuration automatically selects:
- **Windows**: PowerShell 7 (`pwsh.exe`)
- **Linux**: Bash (`bash -l`)
- **macOS**: Bash or Zsh (system default)

## Embedded Development Integration

If `embedded-dev-config.lua` exists in `C:\Users\david\.config\wezterm\`, additional embedded development keybindings are automatically loaded:

### Serial Console (Alt+F1-F4)
- `Alt+F1`: Quick serial console launcher
- `Alt+F2`: Multiple serial monitors (split view)
- `Alt+F3`: Serial console with logging
- `Alt+F4`: Screen serial session (detachable)

### Build System (Alt+F5-F8)
- `Alt+F5`: Quick build
- `Alt+F6`: Clean build
- `Alt+F7`: Flash to device
- `Alt+F8`: Build and flash

### Debugging (Alt+F9-F12)
- `Alt+F9`: Start OpenOCD
- `Alt+F10`: Connect GDB
- `Alt+F11`: J-Link session
- `Alt+F12`: Full development dashboard

See `EMBEDDED_DEV_KEYBINDINGS.md` for complete embedded development documentation.

## Status Bar

The right side of the status bar shows:
- **Workspace**: Current workspace name (if not default)
- **Directory**: Current working directory (with ~ for home)
- **Date/Time**: Current date and time

## Tab Titles

Tab titles automatically show:
- **Tab number**: Position in tab bar (1-indexed)
- **Process name**: Currently running program
- **Active indicator**: `*` for the active tab

## Hyperlink Detection

The configuration automatically detects and makes clickable:
- **URLs**: `http://`, `https://`, `ftp://`, etc.
- **Email addresses**: `user@example.com`
- **File paths**: Windows (`C:\path\to\file`) and Unix (`/path/to/file`)
- **Issue references**: `T123` (customize regex for your issue tracker)

Click with `Ctrl` held to open links.

## Customization

### Changing Colors

Edit the `config.colors` table in `.wezterm.lua`:

```lua
config.colors = {
  foreground = '#FFFFFF',  -- Text color
  background = '#000000',  -- Background color
  -- ... more colors
}
```

Or use a built-in color scheme:

```lua
config.color_scheme = 'One Dark'
```

Browse available schemes: https://wezfurlong.org/wezterm/colorschemes/

### Changing Fonts

```lua
config.font = wezterm.font_with_fallback({
  'JetBrains Mono',
  'Fira Code',
  'Cascadia Mono',
})
config.font_size = 14.0
```

### Disabling Ligatures

```lua
config.harfbuzz_features = { 'calt=0', 'clig=0', 'liga=0' }
```

### Changing Default Shell

```lua
-- For PowerShell 5
config.default_prog = { 'powershell.exe', '-NoLogo' }

-- For cmd.exe
config.default_prog = { 'cmd.exe' }

-- For WSL
config.default_prog = { 'wsl.exe' }
```

### Adding Custom Keybindings

```lua
table.insert(config.keys, {
  key = 'k',
  mods = 'CTRL|ALT',
  action = act.SendString('ls -la\n'),
})
```

### Creating Custom Launch Menu Items

```lua
table.insert(config.launch_menu, {
  label = 'My Custom Shell',
  args = { 'C:\\path\\to\\shell.exe', '--arg' },
})
```

## Performance Tips

1. **GPU Acceleration**: The config uses WebGPU with high-performance preference
2. **Frame Rate**: Limited to 60fps for optimal performance
3. **Scrollback**: 9001 lines is a good balance (increase if needed)
4. **Font Rendering**: Uses FreeType with normal rendering for best quality

## Troubleshooting

### Configuration not loading
1. Check for syntax errors: `wezterm --version`
2. Look at debug overlay: Press `Ctrl+Shift+L`
3. Check WezTerm logs in debug overlay

### Keybindings not working
1. Some keybindings may conflict with Windows or other apps
2. Check which application has focus
3. Try the leader key (`Ctrl+B`) for alternative bindings

### Colors look wrong
1. Ensure terminal is using 24-bit color (TrueColor)
2. Check if theme is overridden by shell (e.g., Oh My Posh)
3. Try a different color scheme

### Font not found
1. Install Cascadia Mono or Cascadia Code from Microsoft
2. WezTerm will fallback to Consolas if Cascadia is unavailable
3. Check installed fonts: `wezterm ls-fonts`

### WSL not working
1. Ensure WSL is installed: `wsl --status`
2. Check WSL distribution name: `wsl --list`
3. Update launch menu with correct distribution name

### Embedded development config not loading
1. Ensure file exists: `C:\Users\david\.config\wezterm\embedded-dev-config.lua`
2. Check for syntax errors in embedded config
3. Look for "Loading embedded development configuration" in debug overlay

## Advanced Configuration

### Conditional Configuration by Platform

```lua
if wezterm.target_triple == 'x86_64-pc-windows-msvc' then
  -- Windows-specific config
elseif wezterm.target_triple:find('linux') then
  -- Linux-specific config
end
```

### Environment Variables

```lua
config.set_environment_variables = {
  PROMPT = '$E]0;$P$G$E\\$P$G',
  TERM = 'wezterm',
}
```

### Custom Events

```lua
wezterm.on('my-custom-event', function(window, pane)
  window:toast_notification('WezTerm', 'Custom event triggered!', nil, 4000)
end)

-- Trigger with keybinding
config.keys = {
  {
    key = 'e',
    mods = 'CTRL|SHIFT',
    action = wezterm.action.EmitEvent('my-custom-event'),
  },
}
```

## Resources

- **WezTerm Documentation**: https://wezfurlong.org/wezterm/
- **Color Schemes**: https://wezfurlong.org/wezterm/colorschemes/
- **Configuration Examples**: https://github.com/wez/wezterm/tree/main/docs/examples
- **Issue Tracker**: https://github.com/wezterm/wezterm/issues
- **Matrix Chat**: https://matrix.to/#/#wezterm:matrix.org

## Quick Reference Card

Print or save this for quick reference:

```
┌─────────────────────────────────────────────────────────┐
│ WezTerm Quick Reference                                 │
├─────────────────────────────────────────────────────────┤
│ Tabs              │  Panes           │  Copy/Paste      │
│ ──────────────────┼──────────────────┼──────────────────│
│ Ctrl+Shift+N  New │  Ctrl+Shift+D    │  Ctrl+Shift+C    │
│ Ctrl+Tab      Nxt │  Split H         │  Copy            │
│ Ctrl+Shift+Tab Prv│  Ctrl+Shift+Alt+D│  Ctrl+Shift+V    │
│ Alt+1..8  Direct  │  Split V         │  Paste           │
│                   │  Ctrl+Shift+Arrow│  Ctrl+Shift+X    │
│ Font Size         │  Navigate        │  Copy Mode       │
│ ──────────────────┼──────────────────┼──────────────────│
│ Ctrl++   Increase │  Search          │  Leader Key      │
│ Ctrl+-   Decrease │  ──────────────  │  ──────────────  │
│ Ctrl+0   Reset    │  Ctrl+Shift+F    │  Ctrl+B then ... │
│                   │  Search          │  | Split H       │
│ Launcher          │  Ctrl+Shift+P    │  - Split V       │
│ ──────────────────┼──────────────────┼──────────────────│
│ Ctrl+Shift+P      │  Alt+Enter       │  Ctrl+Alt+R      │
│ Command Palette   │  Fullscreen      │  Reload Config   │
└─────────────────────────────────────────────────────────┘
```

## Changelog

### Version 1.0 (Current)
- Initial configuration with cmd.exe Campbell theme
- Warp-inspired features (launcher, workspaces, blocks)
- Embedded development integration
- Comprehensive keybindings for Windows
- Cross-platform shell support
- Status bar with workspace and directory info
- Leader key (tmux-style) support
- Smart hyperlink detection
- Right-click paste support

## License

This configuration is provided as-is for personal use. Modify and distribute freely.

## Support

For WezTerm issues, please visit: https://github.com/wezterm/wezterm/issues

For configuration help, review this guide or the embedded development documentation.