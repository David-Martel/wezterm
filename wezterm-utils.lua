-- ============================================================================
-- WEZTERM-UTILS - Main Entry Point
-- Lazy-loaded integration module for WezTerm companion utilities
-- ============================================================================

local wezterm = require('wezterm')
local M = {}

-- ============================================================================
-- CONFIGURATION WITH DEFAULTS
-- ============================================================================

M.config = {
  explorer_bin = wezterm.home_dir .. '\\bin\\wezterm-fs-explorer.exe',
  watcher_bin = wezterm.home_dir .. '\\bin\\wezterm-watch.exe',
  editor_bin = 'uv',
  editor_args = { 'run', 'python', '-m', 'wedit' },

  ipc_socket = '\\\\.\\pipe\\wezterm-utils-ipc',
  ipc_enabled = false,

  state_dir = wezterm.home_dir .. '\\.config\\wezterm\\wezterm-utils-state',
  state_enabled = true,

  lazy_load = true,
  verify_binaries = true,
}

-- ============================================================================
-- LAZY LOADING INFRASTRUCTURE
-- ============================================================================

_G._wezterm_utils_modules = _G._wezterm_utils_modules or {}

local function lazy_require(module_name)
  if not _G._wezterm_utils_modules[module_name] then
    local success, module = pcall(require, 'wezterm-utils.' .. module_name)
    if success then
      _G._wezterm_utils_modules[module_name] = module
      wezterm.log_info('Lazy-loaded wezterm-utils.' .. module_name)
    else
      wezterm.log_error('Failed to load wezterm-utils.' .. module_name .. ': ' .. tostring(module))
      return nil
    end
  end

  return _G._wezterm_utils_modules[module_name]
end

local function verify_binary(path)
  if not M.config.verify_binaries then
    return true
  end

  local handle = io.open(path, 'r')
  if handle then
    handle:close()
    return true
  end

  return false
end

local function launch_with(launcher_name, ...)
  local launcher = lazy_require('launcher')
  if launcher and launcher[launcher_name] then
    return launcher[launcher_name](...)
  end
  return nil
end

-- ============================================================================
-- PUBLIC API
-- ============================================================================

function M.explorer_split(directory)
  return wezterm.action_callback(function(window, pane)
    if not verify_binary(M.config.explorer_bin) then
      wezterm.log_warn('wezterm-fs-explorer binary not found at: ' .. M.config.explorer_bin)
      window:toast_notification('WezTerm Utils', 'Explorer binary not found', nil, 4000)
      return
    end

    launch_with('launch_explorer', window, pane, 'split', directory)
  end)
end

function M.explorer_tab(directory)
  return wezterm.action_callback(function(window, pane)
    if not verify_binary(M.config.explorer_bin) then
      wezterm.log_warn('wezterm-fs-explorer binary not found at: ' .. M.config.explorer_bin)
      window:toast_notification('WezTerm Utils', 'Explorer binary not found', nil, 4000)
      return
    end

    launch_with('launch_explorer', window, pane, 'tab', directory)
  end)
end

function M.watcher_split()
  return wezterm.action_callback(function(window, pane)
    if not verify_binary(M.config.watcher_bin) then
      wezterm.log_warn('wezterm-watch binary not found at: ' .. M.config.watcher_bin)
      window:toast_notification('WezTerm Utils', 'Watcher binary not found', nil, 4000)
      return
    end

    launch_with('launch_watcher', window, pane, 'split')
  end)
end

function M.watcher_tab()
  return wezterm.action_callback(function(window, pane)
    if not verify_binary(M.config.watcher_bin) then
      wezterm.log_warn('wezterm-watch binary not found at: ' .. M.config.watcher_bin)
      window:toast_notification('WezTerm Utils', 'Watcher binary not found', nil, 4000)
      return
    end

    launch_with('launch_watcher', window, pane, 'tab')
  end)
end

function M.editor_split(file_path)
  return wezterm.action_callback(function(window, pane)
    launch_with('launch_editor', window, pane, 'split', file_path)
  end)
end

function M.editor_tab(file_path)
  return wezterm.action_callback(function(window, pane)
    launch_with('launch_editor', window, pane, 'tab', file_path)
  end)
end

-- ============================================================================
-- SETUP AND DIAGNOSTICS
-- ============================================================================

function M.setup(user_config)
  local config_module = lazy_require('config')

  if config_module then
    local merged = config_module.merge(M.config, user_config or {})
    merged = config_module.apply_defaults(merged)

    local valid, errors = config_module.validate(merged)
    if not valid then
      wezterm.log_error('wezterm-utils config validation failed: ' .. table.concat(errors, '; '))
      return false
    end

    M.config = merged
  elseif user_config then
    for key, value in pairs(user_config) do
      M.config[key] = value
    end
  end

  if M.config.state_enabled then
    local state = lazy_require('state')
    if state then
      state.init(M.config.state_dir)
    end
  end

  wezterm.log_info('WezTerm utilities initialized (lazy_load=' .. tostring(M.config.lazy_load) .. ')')
  return true
end

function M.shutdown()
  _G._wezterm_utils_modules = {}
  wezterm.log_info('WezTerm utilities shutdown complete')
end

function M.diagnostics()
  local diag = {
    config = M.config,
    binaries = {
      explorer = verify_binary(M.config.explorer_bin),
      watcher = verify_binary(M.config.watcher_bin),
    },
    modules_loaded = {},
  }

  for name, _ in pairs(_G._wezterm_utils_modules or {}) do
    table.insert(diag.modules_loaded, name)
  end

  return diag
end

return M
