local M = {}

local function append_messages(target, values)
  if type(values) == 'string' then
    table.insert(target, values)
    return
  end

  if type(values) ~= 'table' then
    return
  end

  for _, value in ipairs(values) do
    table.insert(target, tostring(value))
  end
end

local function normalize_path(path)
  local normalized = tostring(path or ''):gsub('/', '\\')
  normalized = normalized:gsub('\\+', '\\')
  return normalized:lower()
end

local function is_within_path(parent, candidate)
  local normalized_parent = normalize_path(parent)
  local normalized_candidate = normalize_path(candidate)

  if normalized_parent == '' or normalized_candidate == '' then
    return false
  end

  if normalized_parent == normalized_candidate then
    return true
  end

  return normalized_candidate:sub(1, #normalized_parent + 1) == normalized_parent .. '\\'
end

local function directory_exists(path)
  if not path or path == '' then
    return false
  end

  local ok, _, code = os.rename(path, path)
  return ok or code == 13
end

local function dirname(path)
  local normalized = tostring(path or ''):gsub('/', '\\')
  return normalized:match('^(.*)\\[^\\]+$')
end

local function join_path(base, child)
  local normalized_base = tostring(base or ''):gsub('/', '\\'):gsub('\\+$', '')
  local normalized_child = tostring(child or ''):gsub('/', '\\'):gsub('^\\+', '')

  if normalized_base == '' then
    return normalized_child
  end

  if normalized_child == '' then
    return normalized_base
  end

  return normalized_base .. '\\' .. normalized_child
end

local function add_module_watch_path(wezterm, module_name)
  local resolved_path = package.searchpath(module_name, package.path)
  if resolved_path and type(wezterm.add_to_config_reload_watch_list) == 'function' then
    wezterm.add_to_config_reload_watch_list(resolved_path)
  end
  return resolved_path
end

local function validate_module_resolution(wezterm, module_name, context, errors, warnings)
  local resolved_path = add_module_watch_path(wezterm, module_name)
  if not resolved_path then
    table.insert(
      errors,
      string.format("required module '%s' was not found on package.path", module_name)
    )
    return
  end

  if context.config_dir and not is_within_path(context.config_dir, resolved_path) then
    table.insert(
      warnings,
      string.format(
        "module '%s' resolves outside wezterm.config_dir: %s",
        module_name,
        resolved_path
      )
    )
  end
end

function M.register(wezterm, shared, opts)
  opts = opts or {}

  if type(wezterm.add_config_validator) ~= 'function' then
    wezterm.log_warn(
      'config validator API unavailable in current wezterm binary; skipping validator registration'
    )
    return false
  end

  local diag_flags = opts.diag_flags or {}
  local required_modules = opts.required_modules or {}
  local optional_modules = opts.optional_modules or {}
  local utility_bins = opts.utility_bins or {}
  local utility_paths = opts.utility_paths or {}

  local function collect_reload_sensitive_roots(context)
    local roots = {}
    local seen = {}

    local function add_root(path)
      if not path or path == '' or not directory_exists(path) then
        return
      end

      local key = normalize_path(path)
      if not seen[key] then
        seen[key] = true
        table.insert(roots, path)
      end
    end

    local config_dir = dirname(context.config_file)
    local home_config_file = context.home_dir and join_path(context.home_dir, '.wezterm.lua') or nil

    if normalize_path(context.config_file) ~= normalize_path(home_config_file) then
      add_root(config_dir or context.config_dir)
    end

    if context.home_dir and directory_exists(join_path(context.home_dir, '.config\\wezterm')) then
      add_root(join_path(context.home_dir, '.config\\wezterm'))
    end

    for _, module_name in ipairs(required_modules) do
      local resolved_path = package.searchpath(module_name, package.path)
      add_root(dirname(resolved_path))
    end

    return roots
  end

  wezterm.add_config_validator('repo-module-resolution', function(_, context)
    local errors = {}
    local warnings = {}

    if not shared.path_exists(context.config_file) then
      table.insert(errors, 'wezterm.config_file does not exist: ' .. tostring(context.config_file))
    end

    if not directory_exists(context.config_dir) then
      table.insert(errors, 'wezterm.config_dir does not exist: ' .. tostring(context.config_dir))
    end

    if not tostring(package.path or ''):find(tostring(context.config_dir), 1, true) then
      table.insert(
        errors,
        'package.path does not include wezterm.config_dir; repo modules may not resolve correctly'
      )
    end

    for _, module_name in ipairs(required_modules) do
      validate_module_resolution(wezterm, module_name, context, errors, warnings)
    end

    for _, module_name in ipairs(optional_modules) do
      local resolved_path = add_module_watch_path(wezterm, module_name)
      if not resolved_path and module_name ~= 'embedded-dev-config' then
        table.insert(
          warnings,
          string.format("optional module '%s' is unavailable on package.path", module_name)
        )
      end
    end

    return {
      errors = errors,
      warnings = warnings,
    }
  end)

  wezterm.add_config_validator('repo-runtime-paths', function(_, context)
    local errors = {}
    local warnings = {}

    if utility_paths.state_dir then
      for _, protected_root in ipairs(collect_reload_sensitive_roots(context)) do
        if is_within_path(protected_root, utility_paths.state_dir) then
          table.insert(
            errors,
            string.format(
              'utility state_dir must stay outside config/reload roots to avoid reload loops: %s (under %s)',
              utility_paths.state_dir,
              protected_root
            )
          )
          break
        end
      end
    end

    if utility_paths.explorer_bin and utility_bins.explorer == false
      and not shared.path_exists(utility_paths.explorer_bin)
    then
      table.insert(
        warnings,
        string.format('optional explorer binary missing: %s', utility_paths.explorer_bin)
      )
    end

    if utility_paths.watcher_bin and utility_bins.watcher == false
      and not shared.path_exists(utility_paths.watcher_bin)
    then
      table.insert(
        warnings,
        string.format('optional watcher binary missing: %s', utility_paths.watcher_bin)
      )
    end

    local config_ok, utils_config = pcall(require, 'wezterm-utils.config')
    if config_ok and utils_config and utils_config.validate then
      local valid, validation_errors = utils_config.validate({
        explorer_bin = utility_paths.explorer_bin,
        watcher_bin = utility_paths.watcher_bin,
        state_dir = utility_paths.state_dir,
      })

      if not valid then
        append_messages(errors, validation_errors)
      end
    elseif not diag_flags.disable_panels then
      table.insert(
        warnings,
        'wezterm-utils.config validation was skipped because the module was unavailable'
      )
    end

    return {
      errors = errors,
      warnings = warnings,
    }
  end)

  return true
end

return M
