-- ============================================================================
-- WEZTERM-UTILS CONFIG MODULE
-- Configuration schema and validation
-- ============================================================================

local wezterm = require('wezterm')
local M = {}

M.schema = {
  explorer_bin = {
    type = 'string',
    default = wezterm.home_dir .. '\\bin\\wezterm-fs-explorer.exe',
    description = 'Path to filesystem explorer binary',
  },
  watcher_bin = {
    type = 'string',
    default = wezterm.home_dir .. '\\bin\\wezterm-watch.exe',
    description = 'Path to file watcher binary',
  },
  editor_bin = {
    type = 'string',
    default = 'uv',
    description = 'Path to editor binary (or launcher like uv)',
  },
  editor_args = {
    type = 'table',
    default = { 'run', 'python', '-m', 'wedit' },
    description = 'Arguments to pass to editor binary',
  },
  ipc_socket = {
    type = 'string',
    default = '\\\\.\\pipe\\wezterm-utils-ipc',
    description = 'Windows named pipe path for IPC',
  },
  ipc_enabled = {
    type = 'boolean',
    default = false,
    description = 'Enable IPC communication',
  },
  state_dir = {
    type = 'string',
    default = wezterm.home_dir .. '\\.config\\wezterm\\wezterm-utils-state',
    description = 'Directory for state persistence',
  },
  state_enabled = {
    type = 'boolean',
    default = true,
    description = 'Enable state persistence',
  },
  lazy_load = {
    type = 'boolean',
    default = true,
    description = 'Lazy-load modules on first use',
  },
  verify_binaries = {
    type = 'boolean',
    default = true,
    description = 'Verify binaries exist before launching',
  },
}

local function validate_type(value, expected_type)
  local actual_type = type(value)

  if expected_type == 'table' then
    return actual_type == 'table'
  elseif expected_type == 'string' then
    return actual_type == 'string'
  elseif expected_type == 'boolean' then
    return actual_type == 'boolean'
  elseif expected_type == 'number' then
    return actual_type == 'number'
  end

  return false
end

function M.validate(config)
  local errors = {}

  for key, schema_entry in pairs(M.schema) do
    local value = config[key]

    if value ~= nil and not validate_type(value, schema_entry.type) then
      table.insert(errors, string.format(
        "Invalid type for '%s': expected %s, got %s",
        key,
        schema_entry.type,
        type(value)
      ))
    end
  end

  if #errors > 0 then
    return false, errors
  end

  return true, nil
end

function M.apply_defaults(config)
  config = config or {}

  for key, schema_entry in pairs(M.schema) do
    if config[key] == nil then
      config[key] = schema_entry.default
    end
  end

  return config
end

function M.merge(base_config, user_config)
  base_config = base_config or {}
  user_config = user_config or {}

  local merged = {}

  for key, value in pairs(base_config) do
    merged[key] = value
  end

  for key, value in pairs(user_config) do
    merged[key] = value
  end

  return merged
end

function M.print_config(config)
  wezterm.log_info('WezTerm Utilities Configuration:')
  for key, value in pairs(config) do
    local value_str = type(value) == 'table' and wezterm.json_encode(value) or tostring(value)
    wezterm.log_info('  ' .. key .. ' = ' .. value_str)
  end
end

function M.generate_docs()
  local docs = {}

  table.insert(docs, '# WezTerm Utilities Configuration')
  table.insert(docs, '')

  for key, schema_entry in pairs(M.schema) do
    table.insert(docs, '## `' .. key .. '`')
    table.insert(docs, '')
    table.insert(docs, '- **Type**: `' .. schema_entry.type .. '`')
    table.insert(docs, '- **Default**: `' .. tostring(schema_entry.default) .. '`')
    table.insert(docs, '- **Description**: ' .. schema_entry.description)
    table.insert(docs, '')
  end

  return table.concat(docs, '\n')
end

return M
