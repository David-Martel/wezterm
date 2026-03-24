-- ============================================================================
-- WEZTERM-UTILS STATE MODULE
-- Handles state persistence across WezTerm restarts
-- ============================================================================

local wezterm = require('wezterm')
local M = {}

M.state_dir = nil
M.initialized = false

function M.init(state_dir)
  if M.initialized then
    return true
  end

  M.state_dir = state_dir

  -- Check if directory exists WITHOUT creating it during config load.
  -- Creating dirs inside ~/.config/wezterm/ triggers WezTerm's file watcher,
  -- causing a config reload loop (config load -> mkdir -> watcher fires -> reload).
  -- Defer directory creation to first write operation instead.
  local handle = io.open(state_dir .. '/.state_check', 'r')
  if handle then
    handle:close()
    M.initialized = true
    return true
  end

  -- Directory might exist but .state_check doesn't — try opening the dir itself
  -- by checking if a known file or any file exists
  local dir_handle = io.open(state_dir, 'r')
  if dir_handle then
    dir_handle:close()
    M.initialized = true
    return true
  end

  -- Directory doesn't exist yet. Mark as initialized but defer mkdir
  -- to the first save_state() call to avoid triggering reload loops.
  M.initialized = true
  M._needs_mkdir = true
  wezterm.log_info('State directory will be created on first write: ' .. state_dir)
  return true
end

local function get_state_file_path(utility_name)
  if not M.state_dir then
    return nil
  end

  return M.state_dir .. '\\' .. utility_name .. '.json'
end

function M.load_state(utility_name)
  if not M.initialized then
    wezterm.log_warn('State module not initialized')
    return nil
  end

  local file_path = get_state_file_path(utility_name)
  if not file_path then
    return nil
  end

  local handle = io.open(file_path, 'r')
  if not handle then
    wezterm.log_info('No state file found for ' .. utility_name)
    return nil
  end

  local content = handle:read('*all')
  handle:close()

  if not content or content == '' then
    return nil
  end

  local success, state = pcall(function()
    return wezterm.json_parse(content)
  end)

  if success and state then
    wezterm.log_info('Loaded state for ' .. utility_name)
    return state
  end

  wezterm.log_error('Failed to parse state file for ' .. utility_name .. ': ' .. tostring(state))
  return nil
end

function M.save_state(utility_name, state)
  if not M.initialized then
    wezterm.log_warn('State module not initialized')
    return false
  end

  -- Deferred directory creation (avoids triggering config reload during load)
  if M._needs_mkdir and M.state_dir then
    local mkdir_cmd
    if wezterm.target_triple == 'x86_64-pc-windows-msvc' then
      mkdir_cmd = 'if not exist "' .. M.state_dir .. '" mkdir "' .. M.state_dir .. '"'
    else
      mkdir_cmd = 'mkdir -p "' .. M.state_dir .. '"'
    end
    os.execute(mkdir_cmd)
    M._needs_mkdir = nil
  end

  local file_path = get_state_file_path(utility_name)
  if not file_path then
    return false
  end

  state = state or {}
  state.last_updated = os.time()
  state.wezterm_version = wezterm.version

  local success, json = pcall(function()
    return wezterm.json_encode(state)
  end)

  if not success then
    wezterm.log_error('Failed to encode state for ' .. utility_name .. ': ' .. tostring(json))
    return false
  end

  local handle = io.open(file_path, 'w')
  if not handle then
    wezterm.log_error('Failed to open state file for writing: ' .. file_path)
    return false
  end

  handle:write(json)
  handle:close()

  wezterm.log_info('Saved state for ' .. utility_name)
  return true
end

function M.delete_state(utility_name)
  if not M.initialized then
    return false
  end

  local file_path = get_state_file_path(utility_name)
  if not file_path then
    return false
  end

  local success = os.remove(file_path)
  if success then
    wezterm.log_info('Deleted state for ' .. utility_name)
    return true
  end

  wezterm.log_warn('Failed to delete state file: ' .. file_path)
  return false
end

function M.list_states()
  if not M.initialized then
    return {}
  end

  local states = {}
  local known_utilities = { 'explorer', 'watcher', 'editor' }

  for _, util_name in ipairs(known_utilities) do
    local state = M.load_state(util_name)
    if state then
      states[util_name] = state
    end
  end

  return states
end

function M.clear_all_states()
  if not M.initialized then
    return false
  end

  local count = 0
  local known_utilities = { 'explorer', 'watcher', 'editor' }

  for _, util_name in ipairs(known_utilities) do
    if M.delete_state(util_name) then
      count = count + 1
    end
  end

  wezterm.log_info('Cleared ' .. count .. ' state files')
  return true
end

return M
