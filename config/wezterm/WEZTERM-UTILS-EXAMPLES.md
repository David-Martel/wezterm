# WezTerm Utilities - Usage Examples

Practical examples for integrating and using the WezTerm utilities module.

## Basic Usage

### Example 1: Default Setup

Minimal configuration - just load the module:

```lua
-- In .wezterm.lua (already configured)
local utils_available, utils = pcall(require, 'wezterm-utils')

if utils_available then
  utils.setup()  -- Use all defaults
end
```

**Result:**
- Keybindings active: Alt+E, Alt+W, etc.
- State saved to `~/.config/wezterm/wezterm-utils-state/`
- Lazy loading enabled (minimal overhead)

---

### Example 2: Custom Binary Paths

Override default binary locations:

```lua
if utils_available then
  utils.setup({
    explorer_bin = 'D:\\Tools\\my-explorer.exe',
    watcher_bin = 'D:\\Tools\\my-watcher.exe',
    editor_bin = 'code',  -- Use VS Code instead of wedit
    editor_args = { '--new-window' },
  })
end
```

---

### Example 3: Disable Features

Minimal configuration for performance:

```lua
if utils_available then
  utils.setup({
    lazy_load = false,       -- Pre-load all modules
    state_enabled = false,   -- Don't persist state
    verify_binaries = false, -- Skip binary checks
    ipc_enabled = false,     -- Disable IPC
  })
end
```

---

## Custom Keybindings

### Example 4: Leader-Based Shortcuts

Use WezTerm's leader key (Ctrl+B) for utilities:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 'e',
    mods = 'LEADER',
    action = utils.explorer_split(),
  })

  table.insert(config.keys, {
    key = 'w',
    mods = 'LEADER',
    action = utils.watcher_split(),
  })

  table.insert(config.keys, {
    key = 'v',
    mods = 'LEADER',
    action = utils.editor_split(),
  })
end
```

**Usage:**
- `Ctrl+B`, then `e` - Open explorer
- `Ctrl+B`, then `w` - Open watcher
- `Ctrl+B`, then `v` - Open editor

---

### Example 5: Project-Specific Shortcuts

Quick access to specific project directories:

```lua
if utils_available and utils then
  -- Open explorer in specific projects
  table.insert(config.keys, {
    key = '1',
    mods = 'CTRL|SHIFT',
    action = utils.explorer_split('C:\\Projects\\rust-project'),
  })

  table.insert(config.keys, {
    key = '2',
    mods = 'CTRL|SHIFT',
    action = utils.explorer_split('C:\\Projects\\python-project'),
  })

  table.insert(config.keys, {
    key = '3',
    mods = 'CTRL|SHIFT',
    action = utils.explorer_split('C:\\Projects\\web-project'),
  })
end
```

---

### Example 6: Pattern-Based Watcher

Quick shortcuts for watching specific file types:

```lua
if utils_available and utils then
  -- Watch Rust files
  table.insert(config.keys, {
    key = 'r',
    mods = 'ALT|CTRL',
    action = utils.watcher_split('*.rs'),
  })

  -- Watch log files
  table.insert(config.keys, {
    key = 'l',
    mods = 'ALT|CTRL',
    action = utils.watcher_split('*.log'),
  })

  -- Watch Python files
  table.insert(config.keys, {
    key = 'p',
    mods = 'ALT|CTRL',
    action = utils.watcher_split('*.py'),
  })
end
```

---

### Example 7: Multi-Action Keybindings

Combine utilities with WezTerm actions:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 'd',
    mods = 'CTRL|ALT',
    action = wezterm.action_callback(function(window, pane)
      -- Split pane vertically
      window:perform_action(
        act.SplitVertical({ domain = 'CurrentPaneDomain' }),
        pane
      )

      -- Get new pane (bottom pane after split)
      local tab = window:active_tab()
      local new_pane = tab:active_pane()

      -- Launch explorer in new pane
      local launcher = require('wezterm-utils.launcher')
      launcher.launch_explorer(window, new_pane, 'split', nil)
    end),
  })
end
```

---

## Advanced Integration

### Example 8: Dynamic Directory Selection

Prompt user for directory before launching explorer:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 'e',
    mods = 'CTRL|SHIFT|ALT',
    action = wezterm.action_callback(function(window, pane)
      -- Use WezTerm's input prompt
      window:perform_action(
        act.PromptInputLine({
          description = 'Enter directory path:',
          action = wezterm.action_callback(function(window, pane, line)
            if line and line ~= '' then
              local launcher = require('wezterm-utils.launcher')
              launcher.launch_explorer(window, pane, 'split', line)
            end
          end),
        }),
        pane
      )
    end),
  })
end
```

---

### Example 9: Workspace-Specific Utilities

Launch different utilities based on active workspace:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 'u',
    mods = 'LEADER',
    action = wezterm.action_callback(function(window, pane)
      local workspace = window:active_workspace()
      local launcher = require('wezterm-utils.launcher')

      if workspace == 'rust-dev' then
        -- Watch Rust files in Rust workspace
        launcher.launch_watcher(window, pane, 'split', '*.rs')
      elseif workspace == 'web-dev' then
        -- Watch JS/TS files in web workspace
        launcher.launch_watcher(window, pane, 'split', '*.{js,ts,tsx}')
      else
        -- Default: open explorer
        launcher.launch_explorer(window, pane, 'split', nil)
      end
    end),
  })
end
```

---

### Example 10: State-Aware Actions

Use saved state to reopen last-used utility:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 'r',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      local state = require('wezterm-utils.state')
      local launcher = require('wezterm-utils.launcher')

      -- Load last explorer state
      local explorer_state = state.load_state('explorer')

      if explorer_state and explorer_state.last_directory then
        -- Reopen explorer in last directory
        launcher.launch_explorer(
          window,
          pane,
          explorer_state.mode or 'split',
          explorer_state.last_directory
        )
        window:toast_notification(
          'Reopened',
          'Directory: ' .. explorer_state.last_directory,
          nil,
          3000
        )
      else
        window:toast_notification('No State', 'No previous directory found', nil, 3000)
      end
    end),
  })
end
```

---

### Example 11: Custom Utility Integration

Integrate your own custom utility:

```lua
if utils_available then
  local launcher = require('wezterm-utils.launcher')

  -- Custom git log viewer
  table.insert(config.keys, {
    key = 'g',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      launcher.launch_utility(
        window,
        pane,
        'split',
        'C:\\Tools\\git-log-viewer.exe',
        { '--pretty', '--graph' },
        { cwd = pane:get_current_working_dir().file_path }
      )
    end),
  })

  -- Custom database browser
  table.insert(config.keys, {
    key = 'b',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      launcher.launch_utility(
        window,
        pane,
        'tab',
        'uv',
        { 'run', 'python', '-m', 'db_browser' },
        {}
      )
    end),
  })
end
```

---

### Example 12: Diagnostics and Debugging

Add diagnostic keybinding to troubleshoot issues:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 'd',
    mods = 'CTRL|ALT|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      local diag = utils.diagnostics()

      -- Format diagnostics for display
      local lines = {
        'WezTerm Utilities Diagnostics',
        '',
        'Configuration:',
        '  explorer_bin: ' .. diag.config.explorer_bin,
        '  watcher_bin: ' .. diag.config.watcher_bin,
        '  state_enabled: ' .. tostring(diag.config.state_enabled),
        '  lazy_load: ' .. tostring(diag.config.lazy_load),
        '',
        'Binaries:',
        '  explorer: ' .. (diag.binaries.explorer and 'FOUND' or 'MISSING'),
        '  watcher: ' .. (diag.binaries.watcher and 'FOUND' or 'MISSING'),
        '',
        'Modules Loaded:',
      }

      for _, module in ipairs(diag.modules_loaded) do
        table.insert(lines, '  - ' .. module)
      end

      local message = table.concat(lines, '\n')

      -- Display in notification
      window:toast_notification('Diagnostics', message, nil, 15000)

      -- Also log to WezTerm logs
      wezterm.log_info(message)
    end),
  })
end
```

---

## Launch Menu Integration

### Example 13: Add Utilities to Launch Menu

Make utilities accessible via WezTerm's launcher (Ctrl+Shift+P):

```lua
if utils_available then
  config.launch_menu = config.launch_menu or {}

  -- Add separator
  table.insert(config.launch_menu, {
    label = '─── WezTerm Utilities ───',
    args = { 'cmd.exe', '/c', 'echo WezTerm Utilities' },
  })

  -- Add explorer launcher
  table.insert(config.launch_menu, {
    label = 'Filesystem Explorer',
    args = { config.explorer_bin },
  })

  -- Add watcher launcher
  table.insert(config.launch_menu, {
    label = 'File Watcher',
    args = { config.watcher_bin },
  })

  -- Add editor launcher
  table.insert(config.launch_menu, {
    label = 'Text Editor',
    args = { 'uv', 'run', 'python', '-m', 'wedit' },
  })
end
```

---

## State Management

### Example 14: Clear State on Demand

Add keybinding to reset all saved state:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 'x',
    mods = 'CTRL|ALT|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      local state = require('wezterm-utils.state')

      -- Confirm before clearing
      window:perform_action(
        act.PromptInputLine({
          description = 'Clear all utility state? (yes/no)',
          action = wezterm.action_callback(function(window, pane, line)
            if line and line:lower() == 'yes' then
              state.clear_all_states()
              window:toast_notification('State Cleared', 'All utility state deleted', nil, 3000)
            end
          end),
        }),
        pane
      )
    end),
  })
end
```

---

### Example 15: View Saved State

Display current saved state:

```lua
if utils_available and utils then
  table.insert(config.keys, {
    key = 's',
    mods = 'CTRL|ALT|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      local state = require('wezterm-utils.state')
      local all_states = state.list_states()

      local lines = { 'Saved Utility State:', '' }

      for utility_name, utility_state in pairs(all_states) do
        table.insert(lines, utility_name .. ':')
        for key, value in pairs(utility_state) do
          if key ~= 'last_updated' and key ~= 'wezterm_version' then
            table.insert(lines, '  ' .. key .. ': ' .. tostring(value))
          end
        end
        table.insert(lines, '')
      end

      local message = table.concat(lines, '\n')
      window:toast_notification('Saved State', message, nil, 10000)
      wezterm.log_info(message)
    end),
  })
end
```

---

## Configuration Patterns

### Example 16: Development vs. Production Config

Use different configurations based on environment:

```lua
if utils_available then
  local is_development = os.getenv('WEZTERM_DEV') == '1'

  local config_opts = {
    lazy_load = not is_development,  -- Pre-load in dev
    verify_binaries = not is_development,  -- Skip checks in dev
    state_enabled = true,
  }

  if is_development then
    -- Use local development builds
    config_opts.explorer_bin = 'C:\\Projects\\wezterm-utils\\target\\debug\\wezterm-fs-explorer.exe'
    config_opts.watcher_bin = 'C:\\Projects\\wezterm-utils\\target\\debug\\wezterm-watch.exe'
  end

  utils.setup(config_opts)
end
```

**Usage:**
```powershell
# Enable development mode
$env:WEZTERM_DEV = "1"
wezterm.exe
```

---

### Example 17: Per-User Configuration

Load user-specific configuration:

```lua
if utils_available then
  local user_config_path = wezterm.home_dir .. '\\.wezterm-utils-config.lua'

  -- Try to load user config
  local user_config = {}
  local f = io.open(user_config_path, 'r')
  if f then
    io.close(f)
    local success, loaded_config = pcall(dofile, user_config_path)
    if success then
      user_config = loaded_config
    end
  end

  utils.setup(user_config)
end
```

**Create user config** (`~/.wezterm-utils-config.lua`):
```lua
return {
  explorer_bin = 'D:\\MyTools\\explorer.exe',
  watcher_bin = 'D:\\MyTools\\watcher.exe',
  state_enabled = false,
}
```

---

## Integration with Existing Config

### Example 18: Conditional Loading Based on OS

Only load utilities on Windows:

```lua
local is_windows = wezterm.target_triple == 'x86_64-pc-windows-msvc'

if is_windows then
  local utils_available, utils = pcall(require, 'wezterm-utils')

  if utils_available then
    utils.setup()

    -- Add Windows-specific keybindings
    if utils then
      table.insert(config.keys, {
        key = 'e',
        mods = 'ALT',
        action = utils.explorer_split(),
      })
    end
  end
else
  wezterm.log_info('WezTerm utilities not loaded (non-Windows platform)')
end
```

---

### Example 19: Graceful Fallback

Provide alternative actions if utilities not available:

```lua
local utils_available, utils = pcall(require, 'wezterm-utils')

if utils_available and utils then
  utils.setup()

  -- Use utilities
  table.insert(config.keys, {
    key = 'e',
    mods = 'ALT',
    action = utils.explorer_split(),
  })
else
  -- Fallback: open Windows Explorer
  table.insert(config.keys, {
    key = 'e',
    mods = 'ALT',
    action = wezterm.action_callback(function(window, pane)
      local cwd = pane:get_current_working_dir()
      if cwd then
        os.execute('explorer "' .. cwd.file_path .. '"')
      end
    end),
  })
end
```

---

## Performance Optimization

### Example 20: Preload Specific Modules

Preload only frequently-used modules:

```lua
if utils_available and utils then
  utils.setup({
    lazy_load = true,  -- Enable lazy loading
  })

  -- Preload launcher module (most commonly used)
  require('wezterm-utils.launcher')

  -- Don't preload IPC or events (rarely used)
end
```

---

## Testing Examples

### Example 21: Test Utility Launch

Test if utilities launch correctly:

```lua
-- test-utilities.lua
local wezterm = require('wezterm')
local utils = require('wezterm-utils')

utils.setup({
  verify_binaries = false,  -- Skip verification for testing
})

local launcher = require('wezterm-utils.launcher')

-- Test explorer
print('Testing explorer launch...')
local success = launcher.launch_explorer(window, pane, 'split', 'C:\\')
print('Explorer launch: ' .. (success and 'SUCCESS' or 'FAILED'))

-- Test watcher
print('Testing watcher launch...')
success = launcher.launch_watcher(window, pane, 'split', '*.log')
print('Watcher launch: ' .. (success and 'SUCCESS' or 'FAILED'))
```

---

## Summary

These examples demonstrate:

1. **Basic Setup** - Default and custom configurations
2. **Keybindings** - Various keybinding patterns and combinations
3. **Advanced Integration** - Dynamic directories, workspaces, state management
4. **Custom Utilities** - Integrating your own tools
5. **Diagnostics** - Troubleshooting and debugging
6. **Configuration** - Development/production, per-user configs
7. **Performance** - Optimization strategies

Choose the patterns that best fit your workflow and customize as needed!