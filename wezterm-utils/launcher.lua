-- ============================================================================
-- WEZTERM-UTILS LAUNCHER MODULE
-- Handles launching utility processes in panes/tabs
-- ============================================================================

local wezterm = require('wezterm')
local act = wezterm.action
local M = {}

local function get_config()
  local utils = require('wezterm-utils')
  return utils.config
end

local function get_cwd(pane, fallback)
  local cwd_uri = pane:get_current_working_dir()
  if cwd_uri then
    return cwd_uri.file_path
  end
  return fallback or wezterm.home_dir
end

local function build_args(binary, extra_args)
  local args = { binary }

  if extra_args then
    for _, arg in ipairs(extra_args) do
      table.insert(args, arg)
    end
  end

  return args
end

local function launch_in_split(window, pane, args, cwd)
  window:perform_action(act.SplitHorizontal({
    args = args,
    cwd = cwd,
  }), pane)
end

local function launch_in_tab(window, pane, args, cwd)
  window:perform_action(act.SpawnCommandInNewTab({
    args = args,
    cwd = cwd,
  }), pane)
end

local function launch(window, pane, mode, args, cwd, utility_name, state_data)
  if mode == 'split' then
    launch_in_split(window, pane, args, cwd)
  elseif mode == 'tab' then
    launch_in_tab(window, pane, args, cwd)
  else
    wezterm.log_error('Invalid launch mode for ' .. utility_name .. ': ' .. tostring(mode))
    return false
  end

  local config = get_config()
  if config.state_enabled and state_data then
    local state = require('wezterm-utils.state')
    state.save_state(utility_name, state_data)
  end

  wezterm.log_info('Launched ' .. utility_name .. ' in ' .. mode .. ' mode')
  return true
end

function M.launch_explorer(window, pane, mode, directory)
  local config = get_config()
  local cwd = directory or get_cwd(pane)
  local extra_args = {}

  if config.ipc_enabled then
    table.insert(extra_args, '--ipc-socket')
    table.insert(extra_args, config.ipc_socket)
  end

  if cwd and cwd ~= '' then
    table.insert(extra_args, cwd)
  end

  return launch(
    window,
    pane,
    mode,
    build_args(config.explorer_bin, extra_args),
    cwd,
    'explorer',
    { last_directory = cwd, mode = mode }
  )
end

function M.launch_watcher(window, pane, mode)
  local config = get_config()
  local cwd = get_cwd(pane)
  local extra_args = {}

  if cwd and cwd ~= '' then
    table.insert(extra_args, cwd)
  end

  return launch(
    window,
    pane,
    mode,
    build_args(config.watcher_bin, extra_args),
    cwd,
    'watcher',
    { last_directory = cwd, mode = mode }
  )
end

function M.launch_editor(window, pane, mode, file_path)
  local config = get_config()
  local cwd = get_cwd(pane)
  local args = { config.editor_bin }

  for _, arg in ipairs(config.editor_args) do
    table.insert(args, arg)
  end

  if file_path then
    table.insert(args, file_path)
  end

  return launch(
    window,
    pane,
    mode,
    args,
    cwd,
    'editor',
    { last_file = file_path, mode = mode }
  )
end

function M.launch_utility(window, pane, mode, binary, args, options)
  options = options or {}
  local cwd = options.cwd or get_cwd(pane)
  return launch(window, pane, mode, build_args(binary, args), cwd, 'custom-utility', nil)
end

return M
