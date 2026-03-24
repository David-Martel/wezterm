-- Shared helpers for the local WezTerm config.
--
-- WezTerm docs cross references:
-- - Config file loading: C:/Users/david/wezterm/docs/config/files.md
-- - Font selection and fallback: C:/Users/david/wezterm/docs/config/fonts.md
-- - Dynamic overrides: C:/Users/david/wezterm/docs/config/lua/window/set_config_overrides.md

local M = {}

function M.path_exists(path)
  if not path or path == '' then
    return false
  end
  local handle = io.open(path, 'r')
  if handle then
    handle:close()
    return true
  end
  return false
end

function M.split_windows_path(path)
  local parts = {}
  for entry in string.gmatch(path or '', '([^;]+)') do
    if entry and entry ~= '' then
      table.insert(parts, entry)
    end
  end
  return parts
end

function M.normalize_windows_path(path)
  return (path or ''):gsub('/', '\\'):lower()
end

function M.build_windows_path(prepend, current_path)
  local current = M.split_windows_path(current_path)
  local ordered = {}
  for _, path in ipairs(prepend or {}) do
    table.insert(ordered, path)
  end
  for _, path in ipairs(current) do
    table.insert(ordered, path)
  end

  local seen = {}
  local deduped = {}
  for _, path in ipairs(ordered) do
    local key = M.normalize_windows_path(path)
    if key ~= '' and not seen[key] then
      table.insert(deduped, path)
      seen[key] = true
    end
  end

  return table.concat(deduped, ';')
end

function M.cwd_to_path(cwd)
  if not cwd then
    return nil
  end

  if type(cwd) == 'userdata' or type(cwd) == 'table' then
    if cwd.file_path and cwd.file_path ~= '' then
      return cwd.file_path
    end
    return tostring(cwd)
  end

  local path = tostring(cwd)
  path = path:gsub('^file:///', '')
  path = path:gsub('^file://', '')
  return path
end

function M.pattern_escape(value)
  return (value:gsub('([%(%)%.%%%+%-%*%?%[%^%$])', '%%%1'))
end

function M.shorten_home(path, home_dir)
  if not path then
    return ''
  end
  local normalized = path:gsub('/', '\\')
  local home = M.pattern_escape(home_dir or '')
  return normalized:gsub('^' .. home, '~')
end

function M.deep_copy(value)
  if type(value) ~= 'table' then
    return value
  end

  local result = {}
  for key, inner in pairs(value) do
    result[key] = M.deep_copy(inner)
  end
  return result
end

function M.deep_merge(base, extra)
  local result = M.deep_copy(base or {})
  for key, value in pairs(extra or {}) do
    if type(value) == 'table' and type(result[key]) == 'table' then
      result[key] = M.deep_merge(result[key], value)
    else
      result[key] = M.deep_copy(value)
    end
  end
  return result
end

function M.deep_equal(a, b)
  if type(a) ~= type(b) then
    return false
  end

  if type(a) ~= 'table' then
    return a == b
  end

  for key, value in pairs(a) do
    if not M.deep_equal(value, b[key]) then
      return false
    end
  end

  for key, value in pairs(b) do
    if not M.deep_equal(value, a[key]) then
      return false
    end
  end

  return true
end

function M.sorted_keys(tbl)
  local keys = {}
  for key in pairs(tbl or {}) do
    table.insert(keys, key)
  end
  table.sort(keys, function(a, b)
    return tostring(a) < tostring(b)
  end)
  return keys
end

function M.serialize_lua_value(value, indent)
  indent = indent or 0
  local pad = string.rep('  ', indent)
  local next_pad = string.rep('  ', indent + 1)
  local kind = type(value)

  if kind == 'table' then
    local lines = { '{' }
    for _, key in ipairs(M.sorted_keys(value)) do
      local encoded_key
      if type(key) == 'number' then
        encoded_key = '[' .. tostring(key) .. ']'
      else
        encoded_key = '[' .. string.format('%q', tostring(key)) .. ']'
      end
      table.insert(lines, next_pad .. encoded_key .. ' = ' .. M.serialize_lua_value(value[key], indent + 1) .. ',')
    end
    table.insert(lines, pad .. '}')
    return table.concat(lines, '\n')
  end

  if kind == 'string' then
    return string.format('%q', value)
  end

  if kind == 'number' or kind == 'boolean' then
    return tostring(value)
  end

  if kind == 'nil' then
    return 'nil'
  end

  return string.format('%q', tostring(value))
end

function M.ensure_directory(path, is_windows)
  if not path or path == '' then
    return
  end

  if is_windows then
    os.execute('mkdir "' .. path:gsub('/', '\\') .. '" >nul 2>nul')
  else
    os.execute('mkdir -p "' .. path .. '"')
  end
end

function M.trim(value)
  return (tostring(value or '')):match('^%s*(.-)%s*$')
end

function M.normalize_hex_color(value)
  if type(value) ~= 'string' then
    return nil
  end
  local trimmed = M.trim(value)
  if trimmed:match('^#%x%x%x%x%x%x$') or trimmed:match('^#%x%x%x%x%x%x%x%x$') then
    return trimmed:lower()
  end
  return nil
end

return M
