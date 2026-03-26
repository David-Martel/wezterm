-- codex_ui/panels.lua
-- Toggle-based companion panels for Explorer, Watcher, Editor.
-- Panels open as split panes alongside the active TUI session.
-- Toggle via Alt+1/2/3 or command bar icons.

local M = {}

function M.new(wezterm, act, shared, utility_bins, utility_paths, utils_available, utils)
  local api = {}

  local path_exists = shared.path_exists
  local mux = wezterm.mux
  wezterm.GLOBAL.codex_ui_panel_state = wezterm.GLOBAL.codex_ui_panel_state or {}
  wezterm.GLOBAL.codex_ui_panel_restore_done = wezterm.GLOBAL.codex_ui_panel_restore_done or {}
  wezterm.GLOBAL.codex_ui_panel_preferences = wezterm.GLOBAL.codex_ui_panel_preferences or {}
  local save_panel_preferences = nil

  local function normalize_preferences(preferences)
    return {
      explorer = preferences and preferences.explorer == true or false,
      watcher = preferences and preferences.watcher == true or false,
      editor = preferences and preferences.editor == true or false,
    }
  end

  local function replace_preferences(preferences)
    wezterm.GLOBAL.codex_ui_panel_preferences = normalize_preferences(preferences)
    return wezterm.GLOBAL.codex_ui_panel_preferences
  end

  replace_preferences(wezterm.GLOBAL.codex_ui_panel_preferences)

  local function preferences_state()
    return wezterm.GLOBAL.codex_ui_panel_preferences
  end

  local function persist_preferences()
    if save_panel_preferences then
      return save_panel_preferences(preferences_state())
    end
    return true
  end

  local function set_panel_intent(panel_name, is_open)
    local state = preferences_state()
    state[panel_name] = is_open == true
    persist_preferences()
  end

  local function current_cwd(pane)
    local cwd_raw = shared.cwd_to_path(pane:get_current_working_dir())
    return cwd_raw or wezterm.home_dir
  end

  local function panel_state_for_window(window)
    local window_id = window:window_id()
    local state = wezterm.GLOBAL.codex_ui_panel_state
    state[window_id] = state[window_id] or {}
    return state[window_id]
  end

  local function tracked_panel_pane(window, panel_name)
    local pane_id = panel_state_for_window(window)[panel_name]
    if not pane_id then
      return nil
    end

    local pane = mux and mux.get_pane and mux.get_pane(pane_id) or nil
    if pane then
      return pane
    end

    panel_state_for_window(window)[panel_name] = nil
    return nil
  end

  local function clear_tracked_panel(window, panel_name)
    panel_state_for_window(window)[panel_name] = nil
  end

  local function mark_window_restored(window)
    wezterm.GLOBAL.codex_ui_panel_restore_done[window:window_id()] = true
  end

  local function window_restored(window)
    return wezterm.GLOBAL.codex_ui_panel_restore_done[window:window_id()] == true
  end

  local function close_tracked_panel(window, panel_name)
    local tracked = tracked_panel_pane(window, panel_name)
    if not tracked then
      return false
    end

    local pane_id = tracked:pane_id()
    local activated = pcall(function()
      tracked:activate()
    end)
    if not activated then
      clear_tracked_panel(window, panel_name)
      return false
    end

    local active_pane = window:active_pane()
    if not active_pane or active_pane:pane_id() ~= pane_id then
      clear_tracked_panel(window, panel_name)
      return false
    end

    clear_tracked_panel(window, panel_name)
    window:perform_action(act.CloseCurrentPane({ confirm = false }), active_pane)
    return true
  end

  local function utility_config()
    if utils_available and utils and utils.config then
      return utils.config
    end
    return {
      editor_args = { 'run', 'python', '-m', 'wedit' },
      editor_bin = 'uv',
      explorer_bin = utility_paths.explorer_bin,
      ipc_enabled = false,
      ipc_socket = '\\\\.\\pipe\\wezterm-utils-ipc',
      watcher_bin = utility_paths.watcher_bin,
    }
  end

  -- Panel definitions: direction, default size, how to launch
  local panel_defs = {
    explorer = {
      direction = 'Right',
      size = 0.35,
      label = 'Explorer',
      launch = function(pane)
        local cwd = current_cwd(pane)
        local cfg = utility_config()
        local explorer_bin = cfg.explorer_bin or utility_paths.explorer_bin
        if utility_bins.explorer and path_exists(explorer_bin) then
          local args = { explorer_bin }
          if cfg.ipc_enabled and cfg.ipc_socket then
            table.insert(args, '--ipc-socket')
            table.insert(args, cfg.ipc_socket)
          end
          table.insert(args, cwd)
          return {
            direction = 'Right',
            size = 0.35,
            cwd = cwd,
            args = args,
          }
        else
          -- Placeholder: show build instructions
          return {
            direction = 'Right',
            size = 0.35,
            cwd = cwd,
            args = { 'pwsh.exe', '-NoLogo', '-NoProfile', '-Command',
              "Write-Host 'wezterm-fs-explorer not installed' -ForegroundColor Yellow; " ..
              "Write-Host 'Expected: " .. explorer_bin:gsub("'", "''") .. "'; " ..
              "Write-Host 'Build: cd ~/wezterm && cargo build --release -p wezterm-fs-explorer'; " ..
              "Write-Host ''; " ..
              "Write-Host 'Press any key to close...' -ForegroundColor DarkGray; " ..
              "$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')"
            },
          }
        end
      end,
    },
    watcher = {
      direction = 'Bottom',
      size = 0.26,
      label = 'Watcher',
      launch = function(pane)
        local cwd = current_cwd(pane)
        local cfg = utility_config()
        local watcher_bin = cfg.watcher_bin or utility_paths.watcher_bin
        if utility_bins.watcher and path_exists(watcher_bin) then
          return {
            direction = 'Bottom',
            size = 0.26,
            cwd = cwd,
            args = { watcher_bin, cwd },
          }
        else
          return {
            direction = 'Bottom',
            size = 0.26,
            cwd = cwd,
            args = { 'pwsh.exe', '-NoLogo', '-NoProfile', '-Command',
              "Write-Host 'wezterm-watch not installed' -ForegroundColor Yellow; " ..
              "Write-Host 'Expected: " .. watcher_bin:gsub("'", "''") .. "'; " ..
              "Write-Host 'Build: cd ~/wezterm && cargo build --release -p wezterm-watch'; " ..
              "Write-Host ''; " ..
              "Write-Host 'Press any key to close...' -ForegroundColor DarkGray; " ..
              "$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')"
            },
          }
        end
      end,
    },
    editor = {
      direction = 'Right',
      size = 0.35,
      label = 'Editor',
      launch = function(pane)
        local cwd = current_cwd(pane)
        local cfg = utility_config()
        if utils_available and utils then
          local args = { cfg.editor_bin or 'uv' }
          for _, arg in ipairs(cfg.editor_args or { 'run', 'python', '-m', 'wedit' }) do
            table.insert(args, arg)
          end
          return {
            direction = 'Right',
            size = 0.35,
            cwd = cwd,
            args = args,
          }
        else
          return {
            direction = 'Right',
            size = 0.35,
            cwd = cwd,
            args = { 'pwsh.exe', '-NoLogo', '-NoProfile', '-Command',
              "Write-Host 'Editor (wedit) not available' -ForegroundColor Yellow; " ..
              "Write-Host 'Requires: uv run python -m wedit'; " ..
              "Write-Host ''; " ..
              "Write-Host 'Press any key to close...' -ForegroundColor DarkGray; " ..
              "$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')"
            },
          }
        end
      end,
    },
  }

  local function open_panel(window, pane, panel_name)
    local def = panel_defs[panel_name]
    local opts = def.launch(pane)
    local new_pane = pane:split({
      direction = opts.direction,
      size = opts.size,
      cwd = opts.cwd,
      args = opts.args,
      set_environment_variables = opts.env,
    })
    panel_state_for_window(window)[panel_name] = new_pane:pane_id()
    return new_pane
  end

  function api.configure_persistence(preferences, save_callback)
    replace_preferences(preferences)
    save_panel_preferences = save_callback
  end

  -- Toggle a panel: close a tracked pane if it exists, otherwise open a new one.
  function api.toggle(window, pane, panel_name)
    local def = panel_defs[panel_name]
    if not def then
      window:toast_notification('WezTerm', 'Unknown panel: ' .. tostring(panel_name), nil, 2000)
      return
    end

    if close_tracked_panel(window, panel_name) then
      set_panel_intent(panel_name, false)
      mark_window_restored(window)
      window:toast_notification('WezTerm', def.label .. ' panel closed', nil, 1500)
      return
    end

    open_panel(window, pane, panel_name)
    set_panel_intent(panel_name, true)
    mark_window_restored(window)
  end

  function api.restore(window, pane)
    if not window or not pane or window_restored(window) then
      return
    end

    mark_window_restored(window)
    local desired = preferences_state()
    for _, panel_name in ipairs({ 'explorer', 'watcher', 'editor' }) do
      if desired[panel_name] and not tracked_panel_pane(window, panel_name) then
        local ok, _ = pcall(open_panel, window, pane, panel_name)
        if not ok then
          clear_tracked_panel(window, panel_name)
        end
      end
    end
  end

  -- Register event handlers for panel toggles
  function api.register_events()
    wezterm.on('toggle-explorer', function(window, pane)
      local ok, err = pcall(api.toggle, window, pane, 'explorer')
      if not ok then
        window:toast_notification('WezTerm', 'Explorer error: ' .. tostring(err), nil, 3000)
      end
    end)
    wezterm.on('toggle-watcher', function(window, pane)
      local ok, err = pcall(api.toggle, window, pane, 'watcher')
      if not ok then
        window:toast_notification('WezTerm', 'Watcher error: ' .. tostring(err), nil, 3000)
      end
    end)
    wezterm.on('toggle-editor', function(window, pane)
      local ok, err = pcall(api.toggle, window, pane, 'editor')
      if not ok then
        window:toast_notification('WezTerm', 'Editor error: ' .. tostring(err), nil, 3000)
      end
    end)
    -- NOTE: window-config-reloaded handler removed — registering it causes
    -- a cairo stack overflow crash on Windows during the first render frame.
    -- Panel restore is now triggered only via explicit user action (Alt+1/2/3)
    -- or can be wired to gui-startup if persistence is needed.
  end

  return api
end

return M
