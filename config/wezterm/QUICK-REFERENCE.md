# WezTerm Utilities - Quick Reference Card

One-page reference for common operations and troubleshooting.

## Default Keybindings

| Key | Action |
|-----|--------|
| `Alt+E` | Filesystem explorer (split) |
| `Alt+Shift+E` | Filesystem explorer (tab) |
| `Alt+W` | File watcher (split) |
| `Alt+Shift+W` | File watcher (tab) |
| `Ctrl+Alt+E` | Text editor (split) |
| `Ctrl+Alt+Shift+E` | Text editor (tab) |

## Quick Commands

```powershell
# View WezTerm logs
# In WezTerm: Ctrl+Shift+L

# Reload WezTerm config
# In WezTerm: Ctrl+Alt+R

# Check installation
dir "C:\Users\david\.config\wezterm\wezterm-utils*"

# View state files
dir "$env:USERPROFILE\.config\wezterm\wezterm-utils-state\"

# Build binaries
cd C:\Projects\wezterm-fs-explorer && cargo build --release
cd C:\Projects\wezterm-watch && cargo build --release
cd C:\Projects\wedit && uv pip install -e .

# Install binaries
copy target\release\*.exe "$env:USERPROFILE\bin\"
```

## Configuration Snippets

### Basic Setup
```lua
utils.setup()  -- Use all defaults
```

### Custom Paths
```lua
utils.setup({
  explorer_bin = 'D:\\Tools\\explorer.exe',
  watcher_bin = 'D:\\Tools\\watcher.exe',
})
```

### Minimal Overhead
```lua
utils.setup({
  lazy_load = true,
  state_enabled = false,
  verify_binaries = false,
})
```

## Common Issues

| Issue | Quick Fix |
|-------|-----------|
| Module not loading | Check files exist: `dir C:\Users\david\.config\wezterm\wezterm-utils.lua` |
| Keybindings not working | Check logs: Ctrl+Shift+L, look for "initialized successfully" |
| Binary not found | Build: `cargo build --release`, copy to `bin\` |
| State not persisting | Create: `mkdir -Force ~\.config\wezterm\wezterm-utils-state` |

## File Locations

| Path | Purpose |
|------|---------|
| `~\.config\wezterm\wezterm-utils.lua` | Main module |
| `~\.config\wezterm\wezterm-utils\*.lua` | Submodules |
| `~\.config\wezterm\wezterm-utils-state\` | State files |
| `~\bin\wezterm-*.exe` | Utility binaries |
| `~\.wezterm.lua` | Main config (updated) |

## Log Messages

**Success:**
```
WezTerm utilities initialized successfully
```

**Warning (non-critical):**
```
WezTerm utilities module not found - utility keybindings will not be available
Explorer binary not found at: ...
```

**Error (critical):**
```
Failed to load wezterm-utils: ...
State directory initialization failed: ...
```

## Diagnostics

### Add Diagnostic Keybinding
```lua
table.insert(config.keys, {
  key = 'F12',
  mods = 'NONE',
  action = wezterm.action_callback(function(window, pane)
    if utils_available and utils then
      local diag = utils.diagnostics()
      window:toast_notification('Diagnostics', wezterm.json_encode(diag), nil, 15000)
    end
  end),
})
```

Press `F12` to view diagnostics.

## Module API

```lua
-- Explorer
utils.explorer_split(directory)
utils.explorer_tab(directory)

-- Watcher
utils.watcher_split(pattern)
utils.watcher_tab(pattern)

-- Editor
utils.editor_split(file_path)
utils.editor_tab(file_path)

-- System
utils.setup(config)
utils.shutdown()
utils.diagnostics()
```

## State Files

Located in: `~\.config\wezterm\wezterm-utils-state\`

```json
// explorer.json
{
  "last_directory": "C:\\Projects",
  "mode": "split",
  "last_updated": 1234567890
}

// watcher.json
{
  "last_directory": "C:\\Logs",
  "last_pattern": "*.log",
  "mode": "split"
}
```

## Documentation Files

| File | Content |
|------|---------|
| `WEZTERM-UTILS-README.md` | Complete API reference (25 KB) |
| `WEZTERM-UTILS-EXAMPLES.md` | 20+ usage examples (18 KB) |
| `WEZTERM-UTILS-TROUBLESHOOTING.md` | Problem solving (15 KB) |
| `WEZTERM-UTILS-SUMMARY.md` | Installation summary |
| `INSTALLATION-VERIFICATION.md` | Verification steps |
| `QUICK-REFERENCE.md` | This file |

## Verification Checklist

- [ ] Files exist (7 module files + 4 docs)
- [ ] WezTerm starts without errors
- [ ] Logs show "initialized successfully"
- [ ] Alt+E shows toast (even without binary)
- [ ] State directory created
- [ ] (Optional) Binaries installed

## Build & Install Workflow

```powershell
# 1. Build filesystem explorer
cd C:\Projects\wezterm-fs-explorer
cargo build --release

# 2. Build file watcher
cd C:\Projects\wezterm-watch
cargo build --release

# 3. Install Python editor
cd C:\Projects\wedit
uv pip install -e .

# 4. Copy binaries
mkdir -Force "$env:USERPROFILE\bin"
copy C:\Projects\wezterm-fs-explorer\target\release\wezterm-fs-explorer.exe "$env:USERPROFILE\bin\"
copy C:\Projects\wezterm-watch\target\release\wezterm-watch.exe "$env:USERPROFILE\bin\"

# 5. Test in WezTerm
# Press Alt+E - should launch explorer
```

## Performance

| Metric | Value |
|--------|-------|
| Startup overhead (lazy) | <1ms |
| Startup overhead (no lazy) | ~5-10ms |
| Memory usage (base) | ~100 KB |
| Memory usage (full) | ~500 KB |
| Binary verification | ~1ms |
| State save/load | ~2ms |

## Support Resources

1. **WezTerm Logs:** Ctrl+Shift+L
2. **Diagnostics:** F12 (if keybinding added)
3. **Test Suite:** `test-wezterm-utils.lua`
4. **Documentation:** See files above

## Emergency Reset

```powershell
# Backup config
copy "$env:USERPROFILE\.wezterm.lua" "$env:USERPROFILE\.wezterm.lua.bak"

# Remove state
Remove-Item -Recurse -Force "$env:USERPROFILE\.config\wezterm\wezterm-utils-state"

# Restart WezTerm
Stop-Process -Name wezterm -Force
wezterm.exe
```

## Status Indicators

**✅ Working:** Logs show "initialized successfully", keybindings respond
**⚠️ Warning:** Toast shows "binary not found" (graceful degradation working)
**❌ Error:** WezTerm crashes or logs show critical errors

---

**Quick Help:** For detailed help, see README.md, EXAMPLES.md, or TROUBLESHOOTING.md