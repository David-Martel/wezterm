-- WezTerm Filesystem Explorer Integration Example
-- Copy relevant sections to your ~/.wezterm.lua

local wezterm = require 'wezterm'
local act = wezterm.action

-- Configuration table
local config = {}

-- Use config builder for newer versions
if wezterm.config_builder then
  config = wezterm.config_builder()
end

--------------------------------------------------------------------------------
-- HELPER FUNCTIONS
--------------------------------------------------------------------------------

-- Run explorer and capture output
local function run_explorer(path, json_mode)
  local args = { 'wezterm-fs-explorer' }
  if json_mode then
    table.insert(args, '--json')
  end
  if path then
    table.insert(args, path)
  end

  local success, stdout, stderr = wezterm.run_child_process(args)
  return success, stdout, stderr
end

-- Get current working directory from pane
local function get_cwd(pane)
  local cwd_uri = pane:get_current_working_dir()
  if cwd_uri then
    return cwd_uri.file_path
  end
  return os.getenv("HOME") or os.getenv("USERPROFILE")
end

-- Parse JSON safely
local function parse_json(json_string)
  local ok, result = pcall(wezterm.json_parse, json_string)
  if ok then
    return result
  end
  return nil
end

--------------------------------------------------------------------------------
-- INTEGRATION FUNCTIONS
--------------------------------------------------------------------------------

-- 1. File Picker: Select and open files in editor
local function file_picker(window, pane)
  local cwd = get_cwd(pane)
  local success, stdout = run_explorer(cwd, false)

  if success and stdout ~= "" then
    for file_path in stdout:gmatch("[^\r\n]+") do
      if file_path ~= "" then
        -- Open in default editor
        local editor = os.getenv("EDITOR") or "nvim"
        pane:send_text(editor .. " '" .. file_path .. "'\r")
        return
      end
    end
  else
    wezterm.log_error("File picker failed")
  end
end

-- 2. Directory Navigator: Quick CD to selected directory
local function directory_navigator(window, pane)
  local cwd = get_cwd(pane)
  local success, stdout = run_explorer(cwd, true)

  if success and stdout ~= "" then
    local paths = parse_json(stdout)
    if paths and #paths > 0 then
      local selected = paths[1]
      -- Navigate to directory
      pane:send_text("cd '" .. selected .. "'\r")
      pane:send_text("clear\r")
    end
  end
end

-- 3. Split Pane Explorer: Open explorer in side pane
local function split_pane_explorer(window, pane)
  local tab = window:mux_window():active_tab()
  local cwd = get_cwd(pane)

  -- Create split pane
  local new_pane = tab:split_pane({
    direction = 'Right',
    size = { Percent = 40 },
    cwd = cwd,
  })

  -- Launch explorer
  new_pane:send_text('wezterm-fs-explorer\n')
end

-- 4. Multi-File Selector: Select multiple files for batch operations
local function multi_file_selector(window, pane)
  local cwd = get_cwd(pane)
  local success, stdout = run_explorer(cwd, true)

  if success and stdout ~= "" then
    local paths = parse_json(stdout)
    if paths and #paths > 0 then
      -- Build file list
      local files = {}
      for _, path in ipairs(paths) do
        table.insert(files, "'" .. path .. "'")
      end

      -- Example: Open all in tabs
      local editor = os.getenv("EDITOR") or "nvim"
      pane:send_text(editor .. " -p " .. table.concat(files, " ") .. "\r")
    end
  end
end

-- 5. Project File Finder: Find files in project root
local function project_file_finder(window, pane)
  local cwd = get_cwd(pane)

  -- Find project root (look for .git, Cargo.toml, package.json, etc.)
  local project_markers = { ".git", "Cargo.toml", "package.json", "go.mod", "pyproject.toml" }
  local project_root = cwd

  -- Simple upward search for project root
  local current = cwd
  while current and current ~= "/" and current ~= "C:\\" do
    for _, marker in ipairs(project_markers) do
      local marker_path = current .. "/" .. marker
      -- Note: This is a simplified check, real implementation would use filesystem
      local f = io.open(marker_path, "r")
      if f then
        f:close()
        project_root = current
        break
      end
    end

    -- Go up one directory
    local parent = current:match("(.+)/[^/]+$") or current:match("(.+)\\[^\\]+$")
    if parent == current then
      break
    end
    current = parent
  end

  -- Launch explorer in project root
  local success, stdout = run_explorer(project_root, false)
  if success and stdout ~= "" then
    for file_path in stdout:gmatch("[^\r\n]+") do
      local editor = os.getenv("EDITOR") or "nvim"
      pane:send_text(editor .. " '" .. file_path .. "'\r")
      return
    end
  end
end

-- 6. Recent Directories: Navigate to recent directories
local recent_dirs = {}

local function add_recent_dir(dir)
  -- Remove if already exists
  for i, d in ipairs(recent_dirs) do
    if d == dir then
      table.remove(recent_dirs, i)
      break
    end
  end

  -- Add to front
  table.insert(recent_dirs, 1, dir)

  -- Keep only last 10
  while #recent_dirs > 10 do
    table.remove(recent_dirs)
  end
end

local function recent_directory_picker(window, pane)
  if #recent_dirs == 0 then
    wezterm.log_info("No recent directories")
    return
  end

  local choices = {}
  for i, dir in ipairs(recent_dirs) do
    table.insert(choices, {
      id = tostring(i),
      label = dir,
    })
  end

  window:perform_action(
    act.InputSelector {
      action = wezterm.action_callback(function(inner_window, inner_pane, id, label)
        if id then
          local idx = tonumber(id)
          local dir = recent_dirs[idx]
          inner_pane:send_text("cd '" .. dir .. "'\r")
        end
      end),
      title = "Recent Directories",
      choices = choices,
      fuzzy = true,
    },
    pane
  )
end

-- Hook to track directory changes
wezterm.on('update-status', function(window, pane)
  local cwd = get_cwd(pane)
  if cwd then
    add_recent_dir(cwd)
  end
end)

--------------------------------------------------------------------------------
-- KEY BINDINGS
--------------------------------------------------------------------------------

config.keys = {
  -- Ctrl+Shift+E: File picker
  {
    key = 'e',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(file_picker),
  },

  -- Ctrl+Shift+D: Directory navigator
  {
    key = 'd',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(directory_navigator),
  },

  -- Ctrl+Shift+N: Split pane explorer
  {
    key = 'n',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(split_pane_explorer),
  },

  -- Ctrl+Shift+M: Multi-file selector
  {
    key = 'm',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(multi_file_selector),
  },

  -- Ctrl+Shift+F: Project file finder
  {
    key = 'f',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(project_file_finder),
  },

  -- Ctrl+Shift+R: Recent directories
  {
    key = 'r',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(recent_directory_picker),
  },

  -- Ctrl+Shift+O: Open explorer in new tab
  {
    key = 'o',
    mods = 'CTRL|SHIFT',
    action = act.SpawnCommandInNewTab {
      args = { 'wezterm-fs-explorer' },
    },
  },
}

--------------------------------------------------------------------------------
-- VISUAL CONFIGURATION
--------------------------------------------------------------------------------

-- Font with Nerd Font support (required for icons)
config.font = wezterm.font_with_fallback {
  'JetBrains Mono Nerd Font',
  'FiraCode Nerd Font',
  'Hack Nerd Font',
  'Cascadia Code',
}
config.font_size = 12.0

-- Color scheme
config.color_scheme = 'Tokyo Night'

-- Tab bar
config.use_fancy_tab_bar = true
config.hide_tab_bar_if_only_one_tab = false

-- Window
config.window_padding = {
  left = 5,
  right = 5,
  top = 5,
  bottom = 5,
}

--------------------------------------------------------------------------------
-- STATUS BAR (shows keybindings hint)
--------------------------------------------------------------------------------

wezterm.on('update-status', function(window, pane)
  local keybindings = {
    "Ctrl+Shift+E: Files",
    "Ctrl+Shift+D: Dirs",
    "Ctrl+Shift+F: Project",
  }

  window:set_right_status(wezterm.format {
    { Foreground = { Color = '#808080' } },
    { Text = table.concat(keybindings, " | ") },
  })
end)

--------------------------------------------------------------------------------
-- RETURN CONFIG
--------------------------------------------------------------------------------

return config
