-- Preference file persistence helpers for the local WezTerm config.
--
-- WezTerm docs cross references:
-- - Config files and loading: C:/Users/david/wezterm/docs/config/files.md
-- - Dynamic config overrides: C:/Users/david/wezterm/docs/config/lua/window/get_config_overrides.md

local M = {}

function M.load(path, path_exists, wezterm)
  if not path_exists(path) then
    return {}
  end

  local ok, result = pcall(dofile, path)
  if ok and type(result) == 'table' then
    return result
  end

  if wezterm and wezterm.log_warn then
    wezterm.log_warn('Failed to load UI preferences from ' .. tostring(path))
  end
  return {}
end

function M.save(path, preferences, ensure_directory, serialize_lua_value, wezterm)
  local parent = path:match('^(.*)[/\\][^/\\]+$')
  ensure_directory(parent)

  local handle, err = io.open(path, 'w')
  if not handle then
    if wezterm and wezterm.log_error then
      wezterm.log_error('Unable to save UI preferences: ' .. tostring(err))
    end
    return false
  end

  handle:write('return ')
  handle:write(serialize_lua_value(preferences))
  handle:write('\n')
  handle:close()
  return true
end

return M
