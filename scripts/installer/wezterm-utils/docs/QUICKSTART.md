# WezTerm Utilities Quick Start Guide

Get up and running with WezTerm Utilities in 5 minutes!

## Installation (2 minutes)

### Step 1: Run Installer
```powershell
cd T:\projects\wezterm-utilities-installer
.\install.ps1
```

Expected output:
```
╔══════════════════════════════════════════════════════════╗
║       WezTerm Utilities Installer v1.0.0                ║
╚══════════════════════════════════════════════════════════╝

Creating backup...
  → Backed up existing configuration
  ✓ Backup created

Installing binaries...
  ✓ Installed wezterm-fs-explorer.exe
  ✓ Installed wezterm-watch.exe

Installing Lua modules...
  ✓ Lua modules installed

✓ Installation Successful!
```

### Step 2: Validate Installation
```powershell
.\validate-deployment.ps1
```

Should show:
```
✓ ALL CHECKS PASSED! Deployment is ready for use.
```

### Step 3: Restart WezTerm
Close and reopen WezTerm to load the new utilities.

## First Use (3 minutes)

### Try the Filesystem Explorer

1. **Launch**: Press `Alt+E` in WezTerm
2. **Navigate**: Use arrow keys or `j`/`k` to move
3. **Open**: Press `Enter` to open a file or directory
4. **Search**: Press `/` to search for files
5. **Quit**: Press `q` to exit

**Example session:**
```
Press Alt+E
  → Explorer opens showing current directory
Arrow Down (or 'j')
  → Move to next file
Enter
  → Open file/directory
Backspace
  → Go back to parent directory
/
  → Enter search mode
Type "readme"
  → Find files matching "readme"
q
  → Exit explorer
```

### Try the File Watcher

Open a new terminal and run:
```powershell
# Watch current directory
wezterm-watch .
```

In another terminal, create a file:
```powershell
echo "test" > test.txt
```

You'll see in the watcher terminal:
```
[CREATED] test.txt
```

Press `Ctrl+C` to stop watching.

## Common Tasks

### Task 1: Browse Project Files
```powershell
# Open explorer in specific directory
wezterm-fs-explorer C:\projects\myapp

# Or set keybinding in .wezterm.lua to always start in projects
```

### Task 2: Watch for Code Changes
```powershell
# Watch Rust files and run tests on change
wezterm-watch . --pattern "*.rs" --exec "cargo test"

# Watch multiple patterns
wezterm-watch . --pattern "*.rs" --pattern "*.toml" --exec "cargo check"
```

### Task 3: Quick File Preview
1. Press `Alt+E` to open explorer
2. Navigate to a file
3. Press `p` to preview
4. Press `Esc` to close preview

### Task 4: Batch Operations
1. Press `Alt+E`
2. Use `Space` to select multiple files
3. Press `d` to delete, `c` to copy, or `m` to move
4. Confirm operation

## Integration with .wezterm.lua

Add these common configurations to your `.wezterm.lua`:

```lua
local wezterm = require 'wezterm'
local config = wezterm.config_builder()

-- Load WezTerm utilities
local utils = require('wezterm-utils')
utils.setup(config)

-- Custom keybindings
config.keys = {
    -- Explorer in current directory
    {
        key = 'e',
        mods = 'ALT',
        action = wezterm.action.SpawnCommandInNewTab {
            args = { 'wezterm-fs-explorer', '.' }
        }
    },
    -- Explorer in home directory
    {
        key = 'h',
        mods = 'ALT|SHIFT',
        action = wezterm.action.SpawnCommandInNewTab {
            args = { 'wezterm-fs-explorer', wezterm.home_dir }
        }
    },
    -- Watch current directory
    {
        key = 'w',
        mods = 'ALT',
        action = wezterm.action.SpawnCommandInNewTab {
            args = { 'wezterm-watch', '.' }
        }
    },
}

return config
```

## Performance Tips

### Tip 1: Use Patterns for Large Directories
Instead of watching everything:
```powershell
# Bad: watches all files
wezterm-watch C:\large-project

# Good: watches only relevant files
wezterm-watch C:\large-project --pattern "*.rs" --pattern "*.toml"
```

### Tip 2: Ignore Build Directories
```powershell
wezterm-watch . --ignore "target" --ignore "node_modules" --ignore ".git"
```

### Tip 3: Adjust Debounce for Rapid Changes
```powershell
# Longer debounce for file save bursts
wezterm-watch . --debounce 1000 --exec "cargo check"
```

## Troubleshooting

### Problem: Explorer doesn't launch with Alt+E
**Solution**:
```powershell
# Check if utilities are in PATH
where.exe wezterm-fs-explorer

# Test direct launch
wezterm-fs-explorer

# Check WezTerm config
cat $env:USERPROFILE\.wezterm.lua
```

### Problem: Watcher not detecting changes
**Solution**:
```powershell
# Test with verbose mode
wezterm-watch . --verbose

# Check permissions
wezterm-watch . --pattern "*.txt"
```

### Problem: "Command not found"
**Solution**:
```powershell
# Add to PATH
$env:PATH += ";$env:USERPROFILE\.local\bin"

# Or add permanently
[Environment]::SetEnvironmentVariable(
    "PATH",
    "$env:PATH;$env:USERPROFILE\.local\bin",
    "User"
)
```

## Next Steps

1. **Read full documentation**: `docs/README.md`
2. **Customize configuration**: `~/.config/wezterm/wezterm-utils-config.json`
3. **Set up keybindings**: Edit `.wezterm.lua`
4. **Explore advanced features**: See `docs/ADVANCED.md`

## Quick Command Reference

| Command | Description |
|---------|-------------|
| `wezterm-fs-explorer [dir]` | Launch filesystem explorer |
| `wezterm-watch <dir>` | Watch directory for changes |
| `wezterm-watch --help` | Show all options |
| `.\validate-deployment.ps1` | Verify installation |
| `.\rollback.ps1 --ListBackups` | List available backups |

## Getting Help

- **Documentation**: `docs/` directory
- **Validation**: `.\validate-deployment.ps1 --Verbose`
- **Logs**: `$env:USERPROFILE\.config\wezterm\*.log`
- **Rollback**: `.\rollback.ps1 --Latest`

---

**You're ready to go!** Press `Alt+E` and start exploring. 🚀