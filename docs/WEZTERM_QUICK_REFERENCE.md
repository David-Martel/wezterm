# WezTerm Advanced Features - Quick Reference Card

## 🚀 One-Page Guide to Everything

---

## 📍 File Locations

```
Main Config:     C:\Users\david\.wezterm.lua
Explorer:        C:\Users\david\wezterm\wezterm-fs-explorer\
Watcher:         C:\Users\david\wezterm\wezterm-watch\
Editor:          C:\codedev\wezterm-editor\
Binaries:        C:\Users\david\.local\bin\
Documentation:   WEZTERM_ADVANCED_FEATURES_SUMMARY.md
```

---

## ⚡ Quick Start

### Build Everything
```powershell
# Explorer
cd C:\Users\david\wezterm\wezterm-fs-explorer
.\build.ps1 -Release -Install

# Watcher
cd C:\Users\david\wezterm\wezterm-watch
$env:RUSTC_WRAPPER = ""
cargo build --release
copy target\release\wezterm-watch.exe ~\.local\bin\

# Editor
cd C:\codedev\wezterm-editor
.\install.ps1

# Launch WezTerm
wezterm
```

---

## ⌨️ Essential Keybindings

### WezTerm Core
```
Ctrl+Shift+P       Command Palette (fuzzy search everything)
Ctrl+Shift+T       Tab Navigator (visual overview)
Ctrl+Shift+N       New Window
Ctrl+Shift+D       Split Pane Horizontal
Ctrl+Shift+Alt+D   Split Pane Vertical
Ctrl+Tab           Next Tab
Ctrl+Shift+Tab     Previous Tab
Alt+1-8            Jump to Tab 1-8
Alt+Enter          Fullscreen
Ctrl+Shift+C       Copy (smart: copies selection or sends Ctrl+C)
Ctrl+Shift+V       Paste
Ctrl+Shift+F       Search
Ctrl+Shift+K       Clear Scrollback
Ctrl++/Ctrl+-      Font Size Increase/Decrease
Ctrl+0             Reset Font Size
Ctrl+Alt+R         Reload Configuration
```

### Launch Features (Custom)
```
Alt+E              Launch Filesystem Explorer
Alt+W              Launch File Watcher
Ctrl+Alt+E         Quick Edit (wedit)
```

### Leader Key (Ctrl+B then...)
```
Leader + |         Split Horizontal
Leader + -         Split Vertical
Leader + D         Detach Domain
Leader + Ctrl+B    Send Ctrl+B to Terminal
```

---

## 📁 Filesystem Explorer (wezterm-fs-explorer)

### Navigation
```
j/k        Move down/up              g/G        Top/bottom
h/l        Parent/enter directory    Ctrl+D/U   Page down/up
/          Search                    Space      Select/multi-select
Enter      Open file/directory       q          Quit
```

### Operations
```
d          Delete (with confirm)     c          Copy
r          Rename                    m          Move
n          New file/directory        .          Toggle hidden
Tab        Toggle preview pane       ?          Help
```

### Usage
```bash
wezterm-fs-explorer                    # Current directory
wezterm-fs-explorer ~/projects         # Specific directory
wezterm-fs-explorer --json .          # JSON output mode
```

---

## 👁️ File Watcher (wezterm-watch)

### Usage
```bash
wezterm-watch .                        # Watch current directory
wezterm-watch . --format pretty        # Pretty colored output
wezterm-watch . --format json          # JSON for parsing
wezterm-watch . --status               # Git status overview
wezterm-watch . --ignore "*.log"       # Custom ignore patterns
```

### Output Formats
- **pretty** - Human-readable with colors
- **json** - Structured for scripts
- **events** - Real-time event stream
- **summary** - Periodic status updates

---

## ✏️ Text Editor (wedit)

### Nano-Compatible Keys
```
Ctrl+O             Save                 Ctrl+W        Search
Ctrl+X             Exit                 Ctrl+K        Cut Line
Ctrl+G             Help                 Ctrl+U        Paste
Ctrl+C             Show Position        Ctrl+L        Toggle Line Numbers
Alt+U              Undo                 Alt+E         Redo
```

### Usage
```bash
wedit myfile.txt                       # Edit file
wedit                                  # New file
wedit --help                           # Show help
```

---

## 🎨 Color Schemes

### Switch Themes (in .wezterm.lua)
```lua
config.color_scheme = 'Campbell'       -- Default Windows
config.color_scheme = 'One Dark'       -- Dark theme
config.color_scheme = 'Gruvbox'        -- Retro
config.color_scheme = 'Nord'           -- Cool blue
config.color_scheme = 'Solarized Dark' -- Classic
```

Browse all: https://wezfurlong.org/wezterm/colorschemes/

---

## 💼 Workspaces (Warp-inspired)

```
Alt+9              Show workspace launcher
Ctrl+Shift+Alt+W   Create new workspace

# In .wezterm.lua
{ key = '1', mods = 'ALT|SHIFT', action = act.SwitchToWorkspace {
  name = 'dev',
  spawn = { cwd = '~/projects' }
}}
```

---

## 🔧 Troubleshooting

### Build Errors
```powershell
# sccache issues
$env:RUSTC_WRAPPER = ""
cargo clean
cargo build --release

# Missing dependencies
cargo update
```

### WezTerm Not Loading Config
```bash
# Check syntax
wezterm show-keys

# View debug overlay
Ctrl+Shift+L
```

### Font Issues
```bash
# List available fonts
wezterm ls-fonts

# Check if Cascadia Mono is installed
wezterm ls-fonts | grep Cascadia
```

---

## 📚 Documentation

### Main Docs
- **WEZTERM_ADVANCED_FEATURES_SUMMARY.md** - Complete overview
- **WEZTERM_CONFIG_GUIDE.md** - Config reference

### Explorer Docs
- **wezterm-fs-explorer/README.md** - Full guide
- **wezterm-fs-explorer/QUICKREF.md** - Quick reference

### Watcher Docs
- **wezterm-watch/README.md** - User guide
- **wezterm-watch/PERFORMANCE.md** - Benchmarks

### Editor Docs
- **wezterm-editor/README.md** - Complete reference
- **wezterm-editor/KEYBINDINGS.md** - All keys

---

## 🎯 Common Workflows

### Workflow 1: Browse and Edit
```
1. Press Alt+E (launch explorer)
2. Navigate with j/k/h/l
3. Press Enter on file
4. File opens in wedit
5. Edit and save with Ctrl+O
6. Exit with Ctrl+X
```

### Workflow 2: Watch and React
```
1. Press Alt+W (launch watcher)
2. Watcher shows real-time changes
3. Git status updates automatically
4. React to changes as they happen
```

### Workflow 3: Multi-Pane Development
```
1. Ctrl+Shift+D (split pane)
2. Alt+E in right pane (explorer)
3. Ctrl+Shift+D in left pane (split again)
4. Alt+W in bottom-left (watcher)
5. Code in top-left pane
```

---

## 🚨 Critical Commands

### Emergency Stop
```
Ctrl+C             Stop current process
Ctrl+Shift+W       Close pane
Ctrl+Alt+W         Force close pane
```

### Reset Everything
```bash
# Remove config and start fresh
mv ~/.wezterm.lua ~/.wezterm.lua.bak
wezterm
```

---

## 📊 Performance Tips

### Optimize Large Directories
```bash
# Disable git integration if slow
wezterm-fs-explorer --no-git

# Limit depth
wezterm-watch . --recursive 3
```

### Reduce Memory Usage
```lua
-- In .wezterm.lua
config.scrollback_lines = 5000  -- Reduce from 9001
```

---

## 🎓 Learning Resources

### WezTerm Official
- Docs: https://wezfurlong.org/wezterm/
- GitHub: https://github.com/wezterm/wezterm
- Color Schemes: https://wezfurlong.org/wezterm/colorschemes/

### This Project
- **Explorer:** See GETTING_STARTED.md
- **Watcher:** See QUICKSTART.md
- **Editor:** See README.md

---

## 🔗 Integration Examples

### Example 1: Quick File Edit
```lua
{ key = 'o', mods = 'CTRL|ALT', action = wezterm.action_callback(function(window, pane)
  local sel = window:get_selection_text_for_pane(pane)
  if sel then
    window:perform_action(act.SpawnCommandInNewTab {
      args = { 'wedit', sel }
    }, pane)
  end
end)}
```

### Example 2: Project Workspace
```lua
{ key = 'p', mods = 'ALT|SHIFT', action = act.SwitchToWorkspace {
  name = 'myproject',
  spawn = {
    cwd = '~/projects/myproject',
    args = { 'bash', '-c', 'wezterm-watch . & bash' }
  }
}}
```

---

## ✅ Quick Verification

```powershell
# Check binaries exist
where wezterm
where wezterm-fs-explorer
where wezterm-watch
where wedit

# Test each feature
wezterm-fs-explorer .
wezterm-watch . --status
wedit test.txt

# Launch WezTerm with config
wezterm
```

---

## 💡 Pro Tips

1. **Use Command Palette** (Ctrl+Shift+P) - Fuzzy search everything
2. **Learn Leader Key** (Ctrl+B) - tmux-style shortcuts
3. **Master Workspaces** - Organize projects efficiently
4. **Customize Colors** - Make it yours
5. **Read the Docs** - Each feature has comprehensive guides

---

## 📞 Need Help?

1. Check documentation in each project folder
2. Review WEZTERM_ADVANCED_FEATURES_SUMMARY.md
3. See WezTerm official docs: wezfurlong.org/wezterm/
4. Build and test incrementally

---

**Print this page for quick reference!** 🖨️

---

*Last Updated: 2025-09-30*
*WezTerm Advanced Features v1.0*