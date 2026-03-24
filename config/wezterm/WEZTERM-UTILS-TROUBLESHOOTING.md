# WezTerm Utilities - Troubleshooting Guide

Comprehensive troubleshooting guide for resolving common issues with the WezTerm utilities module.

## Quick Diagnostics

### Run Built-in Diagnostics

Add this keybinding to your `.wezterm.lua`:

```lua
table.insert(config.keys, {
  key = 'd',
  mods = 'CTRL|ALT|SHIFT',
  action = wezterm.action_callback(function(window, pane)
    if utils_available and utils then
      local diag = utils.diagnostics()
      window:toast_notification(
        'Diagnostics',
        wezterm.json_encode(diag, { indent = true }),
        nil,
        15000
      )
      wezterm.log_info('Diagnostics: ' .. wezterm.json_encode(diag))
    else
      window:toast_notification('Error', 'Utils not available', nil, 4000)
    end
  end),
})
```

Press `Ctrl+Alt+Shift+D` to run diagnostics.

---

## Common Issues

### Issue 1: Module Not Loading

**Symptoms:**
- WezTerm logs show: `WezTerm utilities module not found`
- Keybindings do nothing
- No toast notifications

**Diagnosis:**
```lua
-- Add to .wezterm.lua
wezterm.log_info('Checking for wezterm-utils module...')
local utils_available, utils = pcall(require, 'wezterm-utils')
wezterm.log_info('Utils available: ' .. tostring(utils_available))
if not utils_available then
  wezterm.log_error('Failed to load: ' .. tostring(utils))
end
```

**Solutions:**

1. **Verify file exists:**
   ```powershell
   dir "C:\Users\david\.config\wezterm\wezterm-utils.lua"
   dir "C:\Users\david\.config\wezterm\wezterm-utils\"
   ```

2. **Check Lua syntax errors:**
   ```powershell
   # Test Lua syntax (if you have lua.exe)
   lua -e "require('wezterm-utils')"
   ```

3. **Check WezTerm config directory:**
   ```lua
   wezterm.log_info('Config dir: ' .. wezterm.config_dir)
   ```

4. **Reload WezTerm config:**
   - Press `Ctrl+Alt+R` (Reload configuration)
   - Or restart WezTerm

---

### Issue 2: Keybindings Not Working

**Symptoms:**
- Pressing Alt+E does nothing
- No toast notification
- Other WezTerm keybindings work

**Diagnosis:**
```lua
-- Check if utils_available is true
wezterm.log_info('utils_available: ' .. tostring(utils_available))
wezterm.log_info('utils is nil: ' .. tostring(utils == nil))
```

**Solutions:**

1. **Check for keybinding conflicts:**
   ```lua
   -- Search your .wezterm.lua for existing Alt+E binding
   -- Comment out conflicting bindings
   ```

2. **Verify keybindings added:**
   ```lua
   -- After utils keybinding section
   wezterm.log_info('Total keybindings: ' .. #config.keys)
   ```

3. **Test with different key:**
   ```lua
   table.insert(config.keys, {
     key = 'F1',  -- Use F1 instead
     mods = 'NONE',
     action = utils.explorer_split(),
   })
   ```

4. **Check if utils_available is false:**
   ```lua
   if not utils_available then
     wezterm.log_error('Utils not available - keybindings skipped')
   end
   ```

---

### Issue 3: Binary Not Found

**Symptoms:**
- Toast notification: "Explorer binary not found"
- Keybinding pressed, but nothing launches
- WezTerm logs show: "binary not found at: C:\..."

**Diagnosis:**
```lua
-- Check binary exists
local f = io.open('C:\\Users\\david\\.local\\bin\\wezterm-fs-explorer.exe', 'r')
if f then
  io.close(f)
  wezterm.log_info('Binary found')
else
  wezterm.log_error('Binary NOT found')
end
```

**Solutions:**

1. **Build the binaries** (if not built yet):
   ```powershell
   cd "C:\Projects\wezterm-fs-explorer"
   cargo build --release

   # Copy to .local\bin
   mkdir -Force "$env:USERPROFILE\.local\bin"
   copy target\release\wezterm-fs-explorer.exe "$env:USERPROFILE\.local\bin\"
   ```

2. **Update binary paths in config:**
   ```lua
   utils.setup({
     explorer_bin = 'D:\\MyTools\\wezterm-fs-explorer.exe',
     watcher_bin = 'D:\\MyTools\\wezterm-watch.exe',
   })
   ```

3. **Disable binary verification** (temporary workaround):
   ```lua
   utils.setup({
     verify_binaries = false,  -- Launch anyway
   })
   ```

4. **Use absolute paths:**
   ```lua
   utils.setup({
     explorer_bin = 'C:\\Users\\david\\.local\\bin\\wezterm-fs-explorer.exe',
   })
   ```

---

### Issue 4: State Not Persisting

**Symptoms:**
- Last directory not remembered
- State directory not created
- State files missing

**Diagnosis:**
```lua
local state = require('wezterm-utils.state')
wezterm.log_info('State dir: ' .. (state.state_dir or 'nil'))
wezterm.log_info('State initialized: ' .. tostring(state.initialized))

-- Try to load state
local explorer_state = state.load_state('explorer')
wezterm.log_info('Explorer state: ' .. wezterm.json_encode(explorer_state or {}))
```

**Solutions:**

1. **Manually create state directory:**
   ```powershell
   mkdir -Force "$env:USERPROFILE\.config\wezterm\wezterm-utils-state"
   ```

2. **Check directory permissions:**
   ```powershell
   # Verify you can write to directory
   echo "test" > "$env:USERPROFILE\.config\wezterm\wezterm-utils-state\test.txt"
   type "$env:USERPROFILE\.config\wezterm\wezterm-utils-state\test.txt"
   del "$env:USERPROFILE\.config\wezterm\wezterm-utils-state\test.txt"
   ```

3. **Enable state in config:**
   ```lua
   utils.setup({
     state_enabled = true,
   })
   ```

4. **Check WezTerm logs for state errors:**
   ```
   State directory initialized: ...
   Saved state for explorer
   Failed to open state file...  # Error indicator
   ```

---

### Issue 5: Utilities Launch But Crash Immediately

**Symptoms:**
- Pane opens then immediately closes
- Binary exits with error
- No visible UI

**Diagnosis:**
```powershell
# Run binary manually to see error
C:\Users\david\.local\bin\wezterm-fs-explorer.exe

# Check for missing DLLs (Windows)
dumpbin /dependents C:\Users\david\.local\bin\wezterm-fs-explorer.exe

# Run with verbose logging
C:\Users\david\.local\bin\wezterm-fs-explorer.exe --verbose --debug
```

**Solutions:**

1. **Check binary dependencies:**
   ```powershell
   # Install Visual C++ Redistributable if needed
   # https://aka.ms/vs/17/release/vc_redist.x64.exe
   ```

2. **Test binary standalone:**
   ```powershell
   # Run in regular terminal
   wezterm-fs-explorer.exe
   ```

3. **Check for argument errors:**
   ```lua
   -- Remove IPC arguments if binary doesn't support them
   utils.setup({
     ipc_enabled = false,
   })
   ```

4. **Run with debugger:**
   ```powershell
   # Rust binaries
   cargo run --bin wezterm-fs-explorer -- --directory C:\
   ```

---

### Issue 6: Lazy Loading Not Working

**Symptoms:**
- High startup time
- All modules loaded immediately
- Logs show all modules loaded at startup

**Diagnosis:**
```lua
wezterm.log_info('Lazy load enabled: ' .. tostring(M.config.lazy_load))

-- Check module cache
for module_name, _ in pairs(_G._wezterm_utils_modules or {}) do
  wezterm.log_info('Module cached: ' .. module_name)
end
```

**Solutions:**

1. **Verify lazy_load setting:**
   ```lua
   utils.setup({
     lazy_load = true,  -- Ensure enabled
   })
   ```

2. **Don't manually require modules at startup:**
   ```lua
   -- BAD: Defeats lazy loading
   local launcher = require('wezterm-utils.launcher')

   -- GOOD: Let utils handle lazy loading
   utils.explorer_split()  -- Loads launcher on first use
   ```

3. **Check module cache clearing:**
   ```lua
   -- Clear cache manually
   _G._wezterm_utils_modules = {}
   ```

---

### Issue 7: IPC Not Working

**Symptoms:**
- IPC errors in logs
- Utilities can't communicate with WezTerm
- Named pipe connection failures

**Note:** IPC is currently a stub implementation.

**Solutions:**

1. **Disable IPC** (recommended):
   ```lua
   utils.setup({
     ipc_enabled = false,  -- Default
   })
   ```

2. **Verify IPC not required:**
   - Utilities should work without IPC
   - IPC only needed for advanced features

3. **Wait for IPC implementation:**
   - Current implementation is a placeholder
   - Future versions will support Windows named pipes

---

### Issue 8: Configuration Validation Errors

**Symptoms:**
- WezTerm logs show validation errors
- Invalid configuration warnings
- Setup fails

**Diagnosis:**
```lua
local config_module = require('wezterm-utils.config')
local valid, errors = config_module.validate(utils.config)

wezterm.log_info('Config valid: ' .. tostring(valid))
if errors then
  for _, error in ipairs(errors) do
    wezterm.log_error('Config error: ' .. error)
  end
end
```

**Solutions:**

1. **Check configuration types:**
   ```lua
   utils.setup({
     lazy_load = true,  -- boolean, not "true"
     state_enabled = true,  -- boolean
     verify_binaries = true,  -- boolean
   })
   ```

2. **Use correct table format:**
   ```lua
   utils.setup({
     editor_args = { 'run', 'python', '-m', 'wedit' },  -- table
   })
   ```

3. **Print configuration:**
   ```lua
   local config_module = require('wezterm-utils.config')
   config_module.print_config(utils.config)
   ```

---

## Advanced Troubleshooting

### Debug Logging

Add verbose logging to diagnose issues:

```lua
-- In wezterm-utils.lua or modules
wezterm.log_info('DEBUG: explorer_split called')
wezterm.log_info('DEBUG: binary path: ' .. M.config.explorer_bin)
wezterm.log_info('DEBUG: verify_binaries: ' .. tostring(M.config.verify_binaries))
```

---

### Test Individual Modules

Test modules in isolation:

```lua
-- Test state module
local state = require('wezterm-utils.state')
state.init('C:\\temp\\test-state')
state.save_state('test', { foo = 'bar' })
local loaded = state.load_state('test')
wezterm.log_info('Loaded state: ' .. wezterm.json_encode(loaded))

-- Test launcher module
local launcher = require('wezterm-utils.launcher')
-- launcher.launch_explorer(window, pane, 'split', 'C:\\')
```

---

### Trace Module Loading

Add loading traces:

```lua
-- In wezterm-utils.lua
local function lazy_require(module_name)
  wezterm.log_info('TRACE: lazy_require called for ' .. module_name)

  if not _G._wezterm_utils_modules[module_name] then
    wezterm.log_info('TRACE: Loading module ' .. module_name)
    local success, module = pcall(require, 'wezterm-utils.' .. module_name)

    if success then
      wezterm.log_info('TRACE: Module loaded successfully')
      _G._wezterm_utils_modules[module_name] = module
    else
      wezterm.log_error('TRACE: Module load failed: ' .. tostring(module))
      return nil
    end
  else
    wezterm.log_info('TRACE: Module already cached')
  end

  return _G._wezterm_utils_modules[module_name]
end
```

---

### Check WezTerm Version

Ensure compatible WezTerm version:

```lua
wezterm.log_info('WezTerm version: ' .. wezterm.version)
wezterm.log_info('Target triple: ' .. wezterm.target_triple)
```

**Minimum requirements:**
- WezTerm 20230408 or later
- Windows 10/11 (for Windows named pipes)

---

### Test Action Callbacks

Verify action callbacks work:

```lua
table.insert(config.keys, {
  key = 'F2',
  mods = 'NONE',
  action = wezterm.action_callback(function(window, pane)
    wezterm.log_info('TEST: Action callback fired')
    window:toast_notification('Test', 'Action callback works!', nil, 3000)
  end),
})
```

---

## Error Messages and Solutions

### Error: "Failed to load wezterm-utils: module not found"

**Cause:** Module file doesn't exist or Lua can't find it.

**Solution:**
```powershell
# Check files exist
dir "C:\Users\david\.config\wezterm\wezterm-utils.lua"
dir "C:\Users\david\.config\wezterm\wezterm-utils\"
```

---

### Error: "attempt to index nil value (field 'utils')"

**Cause:** `utils` variable is nil (module load failed).

**Solution:**
```lua
-- Always check utils_available
if utils_available and utils then
  -- Safe to use utils here
end
```

---

### Error: "bad argument #1 to 'insert' (table expected, got nil)"

**Cause:** Trying to add keybindings when utils is nil.

**Solution:**
```lua
-- Check utils exists before adding keybindings
if utils_available and utils then
  for _, key in ipairs(utility_keys) do
    table.insert(config.keys, {
      key = key.key,
      mods = key.mods,
      action = key.action,
    })
  end
end
```

---

### Error: "State directory initialized: ... failed"

**Cause:** Can't create state directory (permissions issue).

**Solution:**
```powershell
# Manually create with correct permissions
mkdir -Force "$env:USERPROFILE\.config\wezterm\wezterm-utils-state"
icacls "$env:USERPROFILE\.config\wezterm\wezterm-utils-state" /grant "$env:USERNAME:(OI)(CI)F"
```

---

### Warning: "WezTerm utilities module not found - utility keybindings will not be available"

**Cause:** Normal warning when module not installed (graceful degradation).

**Solution:**
- If you want utilities: Install module files
- If you don't: Ignore warning (config still works)

---

## Performance Issues

### High Startup Time

**Diagnosis:**
```lua
local start_time = os.clock()
local utils_available, utils = pcall(require, 'wezterm-utils')
local end_time = os.clock()
wezterm.log_info('Utils load time: ' .. (end_time - start_time) .. 's')
```

**Solutions:**
1. Enable lazy loading (default)
2. Disable state persistence if not needed
3. Disable IPC (already default)
4. Don't preload modules manually

---

### High Memory Usage

**Diagnosis:**
```powershell
# Check WezTerm process memory
Get-Process wezterm | Select-Object WS
```

**Solutions:**
1. Clear module cache periodically
2. Disable unused features
3. Limit state history

---

## Getting Help

If issues persist:

1. **Check WezTerm logs:**
   - Press `Ctrl+Shift+L` in WezTerm
   - Or check logs file: `%USERPROFILE%\.local\share\wezterm\wezterm.log`

2. **Run diagnostics:**
   - Use built-in diagnostics keybinding (Ctrl+Alt+Shift+D)

3. **Test in isolation:**
   - Create minimal `.wezterm.lua` with only utilities module

4. **Report issue:**
   - Include WezTerm version (`wezterm --version`)
   - Include configuration snippet
   - Include error messages from logs
   - Include diagnostics output

---

## Useful Commands

### Reset Everything

```powershell
# Backup current config
copy "$env:USERPROFILE\.wezterm.lua" "$env:USERPROFILE\.wezterm.lua.bak"

# Delete state
Remove-Item -Recurse -Force "$env:USERPROFILE\.config\wezterm\wezterm-utils-state"

# Restart WezTerm
Stop-Process -Name wezterm -Force
wezterm.exe
```

---

### Verify Installation

```powershell
# Check all files exist
$files = @(
  "$env:USERPROFILE\.config\wezterm\wezterm-utils.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\init.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\launcher.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\state.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\ipc.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\events.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\config.lua"
)

foreach ($file in $files) {
  if (Test-Path $file) {
    Write-Host "OK: $file" -ForegroundColor Green
  } else {
    Write-Host "MISSING: $file" -ForegroundColor Red
  }
}
```

---

## Summary

Most common issues:
1. Module files missing → Install files
2. Keybindings not working → Check utils_available
3. Binaries not found → Build or update paths
4. State not persisting → Create directory manually
5. Utilities crash → Test binaries standalone

Remember: The module is designed for graceful degradation. If it fails to load, WezTerm config still works without utilities.