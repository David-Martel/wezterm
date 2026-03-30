-- codex_ui/chrome.lua
-- WT-style tab bar formatting and command bar rendering.
--
-- PERFORMANCE DESIGN:
--   - Status bar IPC calls (get_current_working_dir, active_workspace) are
--     throttled to once per second. Between polls, the cached formatted string
--     is reused with zero IPC overhead.
--   - Tab titles cache per (tab_id, process_name, hover, is_active).
--   - TUI name lookup uses a hash set for O(1) matching.
--   - Panel state read from wezterm.GLOBAL (zero IPC, shared memory).
--   - All event handlers use pcall with fallback to prevent error cascades.

local M = {}

-- TUI process names that get a green "active" badge (hash set for O(1))
local TUI_SET = {}
for _, name in ipairs({
  "codex", "claude", "gemini", "aider", "fzf",
  "nvim", "vim", "less", "yazi", "lazygit",
  "gitui", "btop", "htop", "k9s",
}) do
  TUI_SET[name] = true
end

-- Process name normalization map
local PROCESS_ALIASES = {
  pwsh        = "PowerShell",
  powershell  = "PowerShell",
  cmd         = "cmd",
}

-- Unicode glyphs for UI elements
local INDICATOR_ACTIVE   = "\u{25CF}"  -- filled circle
local INDICATOR_INACTIVE = "\u{25CB}"  -- hollow circle
local SEP = "  "

-- Panel toggle icons (left status bar)
local PANEL_ICONS = {
  explorer = { icon = "\u{1F4C1}", label = "E", key = "Alt+1" },  -- folder
  watcher  = { icon = "\u{1F441}", label = "W", key = "Alt+2" },  -- eye
  editor   = { icon = "\u{270F}",  label = "Ed", key = "Alt+3" }, -- pencil
}
local PANEL_ORDER = { "explorer", "watcher", "editor" }

local function is_tui_process(name)
  local lower = name:lower()
  if TUI_SET[lower] then return true end
  for tui_name in pairs(TUI_SET) do
    if lower:find(tui_name, 1, true) then return true end
  end
  return false
end

local function process_label(raw_name)
  if not raw_name or raw_name == "" then return "shell" end
  local base = raw_name:match("[/\\]([^/\\]+)$") or raw_name
  local stripped = base:match("^(.+)%.[Ee][Xx][Ee]$") or base
  local lower = stripped:lower()
  return PROCESS_ALIASES[lower] or stripped
end

local function truncate(s, max_len)
  if #s <= max_len then return s end
  return s:sub(1, max_len - 1) .. "\u{2026}"
end

local function format_tab_title(ui, tab, hover, max_width)
  local is_active = tab.is_active
  local bg, fg
  if is_active then
    bg, fg = ui.surface_active, ui.fg
  elseif hover then
    bg, fg = ui.surface_alt, ui.fg
  else
    bg, fg = ui.surface, ui.muted
  end

  local proc_raw = tab.active_pane and tab.active_pane.foreground_process_name or ""
  local label = process_label(proc_raw)
  local tui = is_tui_process(label)
  local indicator = is_active and INDICATOR_ACTIVE or INDICATOR_INACTIVE

  local badge_reserve = (is_active and tui) and 3 or 0
  local available = max_width - 4 - badge_reserve
  if available < 1 then available = 1 end
  local display_label = truncate(label, available)

  local cells = {
    { Background = { Color = bg } },
    { Foreground = { Color = fg } },
    { Text = " " },
  }
  local n = 3

  if is_active then
    n = n + 1; cells[n] = { Foreground = { Color = ui.accent } }
    n = n + 1; cells[n] = { Text = indicator }
    n = n + 1; cells[n] = { Foreground = { Color = fg } }
  else
    n = n + 1; cells[n] = { Text = indicator }
  end

  n = n + 1; cells[n] = { Text = " " }
  n = n + 1; cells[n] = { Attribute = { Intensity = is_active and "Bold" or "Normal" } }
  n = n + 1; cells[n] = { Text = display_label }

  if is_active and tui then
    n = n + 1; cells[n] = { Attribute = { Intensity = "Normal" } }
    n = n + 1; cells[n] = { Foreground = { Color = "#4ade80" } }
    n = n + 1; cells[n] = { Text = " \u{25C6}" }
    n = n + 1; cells[n] = { Foreground = { Color = fg } }
  end

  n = n + 1; cells[n] = { Text = " " }
  n = n + 1; cells[n] = { Attribute = { Intensity = "Normal" } }
  return cells
end

-- ============================================================================
-- CONSTRUCTOR
-- ============================================================================

function M.new(wezterm, schemes, shared)
  local ui = schemes.ui
  local api = {}

  -- Status bar: throttled IPC + change-detection cache
  local status = {
    last_poll = 0,        -- os.clock() of last IPC poll
    poll_interval = 1.0,  -- seconds between IPC calls
    -- Cached values from last poll
    cwd_short = "",
    workspace = "",
    llm_agent = "",
    clock     = "",
    -- Panel state snapshot (from GLOBAL, no IPC cost)
    panel_state = {},
    -- Pre-formatted results
    left_formatted  = nil,
    right_formatted = nil,
    -- Track what was last set to avoid redundant calls
    last_left  = nil,
    last_right = nil,
  }

  -- Tab title cache: tab_id -> { key, result }
  local tab_cache = {}

  -- Read panel state from GLOBAL (shared memory, zero IPC)
  local function window_state_key(window)
    -- share-data objects behind wezterm.GLOBAL only accept string object keys.
    return tostring(window:window_id())
  end

  local function read_panel_state(window)
    local global_state = wezterm.GLOBAL.codex_ui_panel_state
    if not global_state then return {} end
    return global_state[window_state_key(window)] or {}
  end

  -- Build left status bar: panel toggle indicators
  local function build_left_status(window)
    local pstate = read_panel_state(window)
    local cells = {}
    local n = 0

    -- Background for the panel toggle zone
    n = n + 1; cells[n] = { Background = { Color = ui.surface } }
    n = n + 1; cells[n] = { Text = " " }

    for _, panel_name in ipairs(PANEL_ORDER) do
      local info = PANEL_ICONS[panel_name]
      local is_open = pstate[panel_name] ~= nil

      if is_open then
        -- Highlighted: inverted colors (accent bg, dark fg)
        n = n + 1; cells[n] = { Background = { Color = ui.accent } }
        n = n + 1; cells[n] = { Foreground = { Color = ui.bg } }
        n = n + 1; cells[n] = { Attribute = { Intensity = "Bold" } }
        n = n + 1; cells[n] = { Text = " " .. info.label .. " " }
        n = n + 1; cells[n] = { Attribute = { Intensity = "Normal" } }
      else
        -- Normal: subtle, clickable appearance
        n = n + 1; cells[n] = { Background = { Color = ui.surface } }
        n = n + 1; cells[n] = { Foreground = { Color = ui.muted } }
        n = n + 1; cells[n] = { Text = " " .. info.label .. " " }
      end

      -- Separator between icons
      n = n + 1; cells[n] = { Background = { Color = ui.surface } }
    end

    n = n + 1; cells[n] = { Foreground = { Color = ui.border } }
    n = n + 1; cells[n] = { Text = " \u{2502} " }  -- thin vertical line separator

    return cells
  end

  function api.format_tab_title(tab, tabs, panes, config, hover, max_width)
    local tab_id = tab.tab_id
    local proc_raw = tab.active_pane and tab.active_pane.foreground_process_name or ""
    local cache_key = proc_raw .. "|" .. tostring(hover) .. "|" .. tostring(tab.is_active) .. "|" .. tostring(max_width)

    local cached = tab_cache[tab_id]
    if cached and cached.key == cache_key then
      return cached.result
    end

    local ok, result = pcall(format_tab_title, ui, tab, hover, max_width)
    if not ok then
      result = { { Text = " " .. (tab.active_pane.title or "shell") .. " " } }
    end

    tab_cache[tab_id] = { key = cache_key, result = result }
    return result
  end

  function api.update_right_status(window, pane)
    local ok, err = pcall(function()
      local now = os.clock()
      local elapsed = now - status.last_poll

      -- Fast path: if we polled recently and have a cached result, reuse it.
      -- This avoids IPC calls on 99%+ of frames (at 120fps, polls once/sec).
      if elapsed < status.poll_interval and status.right_formatted then
        -- Only check the clock (cheap) for minute rollover
        local clock = wezterm.strftime("%H:%M")
        if clock == status.clock then
          -- Absolute fast path: nothing could have changed
          if status.last_right == status.right_formatted
             and status.last_left == status.left_formatted then
            return  -- Already set, skip all calls
          end
          if status.last_right ~= status.right_formatted then
            window:set_right_status(status.right_formatted)
            status.last_right = status.right_formatted
          end
          if status.last_left ~= status.left_formatted then
            window:set_left_status(status.left_formatted)
            status.last_left = status.left_formatted
          end
          return
        end
        -- Clock changed: need to rebuild format but skip IPC
        status.clock = clock
      else
        -- Slow path: poll IPC for fresh data (once per second)
        status.last_poll = now

        local cwd_obj  = pane:get_current_working_dir()
        local cwd_raw  = cwd_obj and (cwd_obj.file_path or tostring(cwd_obj)) or ""
        local cwd_path = shared.cwd_to_path(cwd_raw)
        status.cwd_short = shared.shorten_home(cwd_path, wezterm.home_dir) or ""

        status.workspace = window:active_workspace() or ""

        local llm_agent = nil
        if wezterm.GLOBAL and wezterm.GLOBAL.llm_status then
          llm_agent = wezterm.GLOBAL.llm_status.agent
        end
        status.llm_agent = llm_agent or ""

        status.clock = wezterm.strftime("%H:%M")
      end

      -- Rebuild left status (panel toggles) -- reads GLOBAL, no IPC
      local left_cells = build_left_status(window)
      status.left_formatted = wezterm.format(left_cells)

      -- Rebuild right status from cached values
      local cells = {}
      local function segment(bg, fg, text)
        cells[#cells + 1] = { Background = { Color = bg } }
        cells[#cells + 1] = { Foreground = { Color = fg } }
        cells[#cells + 1] = { Text = SEP .. text .. SEP }
      end

      local ws = status.workspace
      if ws ~= "default" and ws ~= "" then
        segment(ui.surface, ui.muted, ws)
      end

      if status.cwd_short ~= "" then
        segment(ui.surface_alt, ui.fg, status.cwd_short)
      end

      if status.llm_agent ~= "" then
        segment("#0d3320", "#4ade80", status.llm_agent)
      end

      -- GPU indicator (shows once on first poll from GLOBAL)
      local gpu_name = wezterm.GLOBAL.codex_ui_gpu_name
      if gpu_name and gpu_name ~= "" then
        segment(ui.surface, ui.muted, gpu_name)
      end

      segment(ui.accent_strong, ui.bg, status.clock)

      status.right_formatted = wezterm.format(cells)

      -- Apply both left and right status
      if status.last_left ~= status.left_formatted then
        window:set_left_status(status.left_formatted)
        status.last_left = status.left_formatted
      end

      if status.last_right ~= status.right_formatted then
        window:set_right_status(status.right_formatted)
        status.last_right = status.right_formatted
      end
    end)

    if not ok then
      wezterm.log_warn("chrome: update_right_status error: " .. tostring(err))
    end
  end

  function api.is_tui_process(name)
    return is_tui_process(name)
  end

  return api
end

return M
