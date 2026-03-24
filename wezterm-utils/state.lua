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

  local mkdir_cmd
  if wezterm.target_triple == 'x86_64-pc-windows-msvc' then
    mkdir_cmd = 'if not exist "' .. state_dir .. '" mkdir "' .. state_dir .. '"'
  else
    mkdir_cmd = 'mkdir -p "' .. state_dir .. '"'
  end

  local result = os.execute(mkdir_cmd)

  if result == 0 or result == true then
    M.initialized = true
    wezterm.log_info('State directory initialized: ' .. state_dir)
    return true
  end

  wezterm.log_error('Failed to create state directory: ' .. state_dir)
  return false
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
