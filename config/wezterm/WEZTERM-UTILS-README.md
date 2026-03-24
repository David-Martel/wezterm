# WezTerm Utilities Integration Module

Lazy-loaded integration module providing seamless access to WezTerm filesystem explorer, file watcher, and text editor utilities directly from WezTerm.

## Features

- **Lazy Loading**: Modules only loaded when first utility is invoked (minimal startup overhead)
- **State Persistence**: Remember last-used directories, patterns, and files across restarts
- **Backward Compatible**: .wezterm.lua works even if utilities module fails to load
- **Binary Verification**: Optional checks to verify utilities exist before launching
- **Graceful Degradation**: Toasts notifications if binaries missing instead of errors
- **IPC Support**: Optional IPC for advanced communication (stub implementation)

## Installation

1. **Copy Module Files**:
   ```
   C:\Users\david\.config\wezterm\
   ├── wezterm-utils.lua              # Main entry point
   └── wezterm-utils\
       ├── init.lua                   # Module initialization
       ├── launcher.lua               # Utility launching
       ├── ipc.lua                    # IPC client (stub)
       ├── state.lua                  # State persistence
       ├── events.lua                 # Event handlers
       └── config.lua                 # Configuration schema
   ```

2. **Integration Already Complete**: Your `.wezterm.lua` already has integration code:
   - Lines 19-31: Module loading with graceful fallback
   - Lines 422-478: Keybinding registration

3. **Build Utility Binaries**: (See separate Rust projects)
   - `wezterm-fs-explorer` - Filesystem explorer
   - `wezterm-watch` - File watcher
   - `wedit` - Text editor (Python)

## Configuration

### Default Configuration

```lua
{
  -- Binary paths
  explorer_bin = 'C:\\Users\\david\\.local\\bin\\wezterm-fs-explorer.exe',
  watcher_bin = 'C:\\Users\\david\\.local\\bin\\wezterm-watch.exe',
  editor_bin = 'uv',
  editor_args = { 'run', 'python', '-m', 'wedit' },

  -- IPC (optional, disabled by default)
  ipc_socket = '\\\\.\\pipe\\wezterm-utils-ipc',
  ipc_enabled = false,

  -- State persistence
  state_dir = 'C:\\Users\\david\\.config\\wezterm\\wezterm-utils-state',
  state_enabled = true,

  -- Performance
  lazy_load = true,  -- Only load when first used

  -- Features
  verify_binaries = true,  -- Check binaries exist before launching
}
```

### Custom Configuration

In your `.wezterm.lua`, after the utilities module loads:

```lua
if utils_available then
  local setup_success = utils.setup({
    -- Override default binary paths
    explorer_bin = 'C:\\custom\\path\\explorer.exe',
    watcher_bin = 'C:\\custom\\path\\watcher.exe',

    -- Disable lazy loading (load all modules at startup)
    lazy_load = false,

    -- Enable IPC (when implemented)
    ipc_enabled = true,

    -- Custom state directory
    state_dir = wezterm.home_dir .. '\\.wezterm-utils-state',

    -- Disable binary verification (launch regardless)
    verify_binaries = false,
  })
end
```

## Keybindings

### Default Keybindings (Already Configured)

**Filesystem Explorer:**
- `Alt+E` - Open in split pane (horizontal)
- `Alt+Shift+E` - Open in new tab

**File Watcher:**
- `Alt+W` - Open in split pane (horizontal)
- `Alt+Shift+W` - Open in new tab

**Text Editor:**
- `Ctrl+Alt+E` - Open in split pane
- `Ctrl+Alt+Shift+E` - Open in new tab

### Custom Keybindings

Add your own keybindings in `.wezterm.lua`:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 'f',
    mods = 'CTRL|SHIFT',
    action = utils.explorer_split('C:\\Projects'),
    -- Opens explorer starting in C:\Projects
  })

  table.insert(config.keys, {
    key = 'l',
    mods = 'CTRL|SHIFT',
    action = utils.watcher_split('*.log'),
    -- Watches only .log files
  })
end
```

## API Reference

### Main Module (`wezterm-utils.lua`)

#### `utils.setup(user_config) -> boolean`
Initialize the utilities module with optional configuration.

**Parameters:**
- `user_config` (table, optional): Configuration overrides

**Returns:** `true` on success, `false` on failure

**Example:**
```lua
local success = utils.setup({
  lazy_load = false,
  state_enabled = true,
})
```

---

#### `utils.explorer_split(directory) -> action_callback`
Launch filesystem explorer in horizontal split pane.

**Parameters:**
- `directory` (string, optional): Starting directory (defaults to current working directory)

**Returns:** WezTerm action callback

**Example:**
```lua
{ key = 'e', mods = 'ALT', action = utils.explorer_split() }
{ key = 'e', mods = 'LEADER', action = utils.explorer_split('C:\\Projects') }
```

---

#### `utils.explorer_tab(directory) -> action_callback`
Launch filesystem explorer in new tab.

---

#### `utils.watcher_split(pattern) -> action_callback`
Launch file watcher in horizontal split pane.

**Parameters:**
- `pattern` (string, optional): File pattern to watch (e.g., `"*.rs"`, `"*.log"`)

**Example:**
```lua
{ key = 'w', mods = 'ALT', action = utils.watcher_split('*.log') }
```

---

#### `utils.watcher_tab(pattern) -> action_callback`
Launch file watcher in new tab.

---

#### `utils.editor_split(file_path) -> action_callback`
Launch text editor in horizontal split pane.

**Parameters:**
- `file_path` (string, optional): File to open (defaults to empty buffer)

**Example:**
```lua
{ key = 'e', mods = 'CTRL|ALT', action = utils.editor_split() }
{ key = 'e', mods = 'LEADER', action = utils.editor_split('config.lua') }
```

---

#### `utils.editor_tab(file_path) -> action_callback`
Launch text editor in new tab.

---

#### `utils.shutdown()`
Cleanup and shutdown utilities module (called automatically on WezTerm exit).

---

#### `utils.diagnostics() -> table`
Get diagnostic information about module state.

**Returns:**
```lua
{
  config = { ... },  -- Current configuration
  binaries = {
    explorer = true,  -- Binary exists
    watcher = false,  -- Binary missing
  },
  modules_loaded = { 'launcher', 'state' },  -- Lazy-loaded modules
}
```

---

### Launcher Module (`wezterm-utils.launcher`)

#### `launcher.launch_explorer(window, pane, mode, directory) -> boolean`
Launch filesystem explorer utility.

**Parameters:**
- `window` - WezTerm window object
- `pane` - WezTerm pane object
- `mode` - `'split'` or `'tab'`
- `directory` (optional) - Starting directory

**Returns:** `true` on success

---

#### `launcher.launch_watcher(window, pane, mode, pattern) -> boolean`
Launch file watcher utility.

---

#### `launcher.launch_editor(window, pane, mode, file_path) -> boolean`
Launch text editor utility.

---

#### `launcher.launch_utility(window, pane, mode, binary, args, options) -> boolean`
Generic utility launcher for custom utilities.

**Example:**
```lua
local launcher = require('wezterm-utils.launcher')
launcher.launch_utility(window, pane, 'split', 'C:\\custom-tool.exe', {'--arg1', '--arg2'}, {
  cwd = 'C:\\Projects',
})
```

---

### State Module (`wezterm-utils.state`)

#### `state.init(state_dir) -> boolean`
Initialize state persistence directory.

---

#### `state.load_state(utility_name) -> table|nil`
Load persisted state for a utility.

**Example:**
```lua
local state_data = state.load_state('explorer')
-- { last_directory = "C:\\Projects", mode = "split", last_updated = 1234567890 }
```

---

#### `state.save_state(utility_name, state) -> boolean`
Save state for a utility.

**Example:**
```lua
state.save_state('watcher', {
  last_directory = 'C:\\Logs',
  last_pattern = '*.log',
})
```

---

#### `state.delete_state(utility_name) -> boolean`
Delete state for a utility.

---

#### `state.list_states() -> table`
List all saved states.

---

#### `state.clear_all_states() -> boolean`
Clear all saved states.

---

### Config Module (`wezterm-utils.config`)

#### `config.validate(config) -> boolean, errors`
Validate configuration against schema.

**Returns:**
- `true, nil` if valid
- `false, {errors}` if invalid

---

#### `config.apply_defaults(config) -> table`
Apply default values to configuration.

---

#### `config.merge(base_config, user_config) -> table`
Merge two configurations.

---

#### `config.print_config(config)`
Log configuration to WezTerm logs.

---

#### `config.generate_docs() -> string`
Generate Markdown documentation for configuration schema.

## State Persistence

State is automatically saved to `~/.config/wezterm/wezterm-utils-state/`:

```
wezterm-utils-state/
├── explorer.json         # Last directory used
├── watcher.json          # Last pattern and directory
├── editor.json           # Last file opened
└── launch_history.json   # Recent launches (max 50)
```

**Example state file** (`explorer.json`):
```json
{
  "last_directory": "C:\\Users\\david\\Projects",
  "mode": "split",
  "last_updated": 1234567890,
  "wezterm_version": "20230408-112425-69ae8472"
}
```

State files can be manually edited or deleted to reset preferences.

## Troubleshooting

### Module Not Loading

**Check WezTerm logs** (Ctrl+Shift+L or view logs file):
```
WezTerm utilities module loaded successfully
WezTerm utility keybindings added
```

If you see warnings:
```
WezTerm utilities module not found - utility keybindings will not be available
```

**Solution:** Verify module files exist:
```powershell
dir C:\Users\david\.config\wezterm\wezterm-utils*.lua
dir C:\Users\david\.config\wezterm\wezterm-utils\
```

---

### Binary Not Found

When pressing keybinding, you see a toast notification:
```
WezTerm Utils
Explorer binary not found
```

**Solution 1:** Build the utility binaries (see Rust projects)

**Solution 2:** Update binary paths in configuration:
```lua
utils.setup({
  explorer_bin = 'C:\\path\\to\\your\\wezterm-fs-explorer.exe',
})
```

**Solution 3:** Disable binary verification:
```lua
utils.setup({
  verify_binaries = false,  -- Launch anyway (may fail)
})
```

---

### Keybindings Not Working

**Check for conflicts:**
- Your `.wezterm.lua` may already use Alt+E, Alt+W, etc.
- Remove conflicting keybindings or change utility keybindings

**Verify module loaded:**
```lua
-- Add diagnostic keybinding to check
table.insert(config.keys, {
  key = 'd',
  mods = 'CTRL|SHIFT',
  action = wezterm.action_callback(function(window, pane)
    if utils_available then
      local diag = utils.diagnostics()
      wezterm.log_info('Diagnostics: ' .. wezterm.json_encode(diag))
      window:toast_notification('Diagnostics', wezterm.json_encode(diag), nil, 10000)
    else
      window:toast_notification('Diagnostics', 'Utils not available', nil, 4000)
    end
  end),
})
```

---

### State Not Persisting

**Check state directory exists:**
```powershell
dir C:\Users\david\.config\wezterm\wezterm-utils-state\
```

**Check WezTerm logs:**
```
State directory initialized: C:\Users\david\.config\wezterm\wezterm-utils-state
Saved state for explorer
```

**Solution:** Manually create directory:
```powershell
mkdir C:\Users\david\.config\wezterm\wezterm-utils-state
```

---

### Performance Issues

**Disable lazy loading** to pre-load all modules at startup:
```lua
utils.setup({
  lazy_load = false,
})
```

**Disable state persistence** if not needed:
```lua
utils.setup({
  state_enabled = false,
})
```

**Disable IPC** (already disabled by default):
```lua
utils.setup({
  ipc_enabled = false,
})
```

## Testing

### Manual Testing

1. **Test module loading:**
   ```lua
   -- Add to .wezterm.lua after utils.setup()
   if utils_available then
     wezterm.log_info('Utils available: ' .. tostring(utils ~= nil))
     local diag = utils.diagnostics()
     wezterm.log_info('Diagnostics: ' .. wezterm.json_encode(diag))
   end
   ```

2. **Test keybindings:**
   - Press `Alt+E` - Should show toast if binary missing
   - Press `Ctrl+Shift+L` - View logs for error messages

3. **Test state persistence:**
   - Launch explorer with `Alt+E`
   - Check state file: `C:\Users\david\.config\wezterm\wezterm-utils-state\explorer.json`

4. **Test graceful degradation:**
   - Rename binary temporarily: `wezterm-fs-explorer.exe` → `wezterm-fs-explorer.exe.bak`
   - Press `Alt+E` - Should show toast notification, not crash

### Automated Testing

**Lua unit tests** (using busted or similar):
```lua
-- test_wezterm_utils.lua
local utils = require('wezterm-utils')

describe('wezterm-utils', function()
  it('should load module', function()
    assert.is_not_nil(utils)
  end)

  it('should have setup function', function()
    assert.is_function(utils.setup)
  end)

  it('should initialize with defaults', function()
    local success = utils.setup()
    assert.is_true(success)
  end)

  it('should validate configuration', function()
    local config_module = require('wezterm-utils.config')
    local valid, errors = config_module.validate(utils.config)
    assert.is_true(valid)
    assert.is_nil(errors)
  end)
end)
```

## Advanced Usage

### Custom Utility Integration

Add your own utilities using the module:

```lua
-- In your .wezterm.lua
if utils_available then
  local launcher = require('wezterm-utils.launcher')

  table.insert(config.keys, {
    key = 't',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      launcher.launch_utility(
        window,
        pane,
        'split',
        'C:\\my-custom-tool.exe',
        {'--verbose', '--color'},
        { cwd = 'C:\\Projects' }
      )
    end),
  })
end
```

---

### State Management

Implement custom state logic:

```lua
-- Save custom state
local state = require('wezterm-utils.state')
state.save_state('my-utility', {
  last_used = os.time(),
  preferences = { theme = 'dark', font_size = 14 },
})

-- Load custom state
local my_state = state.load_state('my-utility')
if my_state then
  wezterm.log_info('Last used: ' .. my_state.last_used)
end
```

---

### Event Handling

Use the events module for custom logic:

```lua
local events = require('wezterm-utils.events')

-- Register custom event handler
wezterm.on('user-var-changed', function(window, pane, name, value)
  if name == 'UTILITY_STATE' then
    events.on_utility_launched('custom', 'split', pane:pane_id())
  end
end)
```

## Architecture

### Module Structure

```
wezterm-utils.lua (Main API)
    ├── Lazy loading infrastructure
    ├── Binary verification
    ├── Public API (explorer_split, watcher_tab, etc.)
    └── Setup/shutdown

wezterm-utils/ (Submodules)
    ├── init.lua (Version info)
    ├── launcher.lua (Process launching)
    ├── state.lua (Persistence)
    ├── ipc.lua (IPC client - stub)
    ├── events.lua (Event handlers)
    └── config.lua (Schema validation)
```

### Lazy Loading Flow

```
1. User presses Alt+E
2. Keybinding triggers utils.explorer_split()
3. Action callback checks binary exists
4. Lazy-loads launcher module (if not loaded)
5. launcher.launch_explorer() spawns process
6. State saved to explorer.json
```

### Graceful Degradation

```
.wezterm.lua loads
    ├── Try: require('wezterm-utils')
    ├── Success: utils_available = true
    │   └── Add keybindings
    └── Failure: utils_available = false
        └── Skip keybindings, config still works
```

## License

MIT License - See project root for details.

## Support

For issues, feature requests, or contributions:
- Check WezTerm logs: `Ctrl+Shift+L`
- Review this documentation
- Test with diagnostics: `utils.diagnostics()`