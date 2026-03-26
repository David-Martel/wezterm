-- WezTerm Integration Examples for wezterm-watch
-- Add these snippets to your wezterm.lua configuration

local wezterm = require 'wezterm'
local config = {}

-- ============================================================================
-- EXAMPLE 1: Auto-start File Watcher on Startup
-- ============================================================================

-- Automatically start wezterm-watch in a split pane when WezTerm launches
wezterm.on('gui-startup', function(cmd)
  local tab, pane, window = wezterm.mux.spawn_window(cmd or {})

  -- Get the current working directory
  local cwd = pane:get_current_working_dir()
  if cwd then
    cwd = cwd.file_path
  else
    cwd = wezterm.home_dir
  end

  -- Split horizontally and spawn watcher (30% of screen height)
  local watch_pane = pane:split {
    direction = 'Bottom',
    size = 0.3,
    cwd = cwd,
    args = {
      'wezterm-watch',
      cwd,
      '--format', 'pretty',
    },
  }

  -- Focus back on the main pane
  pane:activate()
end)

-- ============================================================================
-- EXAMPLE 2: Key Bindings for Watch Control
-- ============================================================================

config.keys = {
  -- Ctrl+Shift+W: Start watcher in new tab
  {
    key = 'W',
    mods = 'CTRL|SHIFT',
    action = wezterm.action.SpawnCommandInNewTab {
      args = { 'wezterm-watch', '.', '--format', 'summary' },
    },
  },

  -- Ctrl+Shift+G: Show git status in floating pane
  {
    key = 'G',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      local cwd = pane:get_current_working_dir()
      if cwd then
        cwd = cwd.file_path
      else
        cwd = '.'
      end

      window:perform_action(
        wezterm.action.SpawnCommandInNewWindow {
          args = { 'wezterm-watch', cwd, '--status', '--format', 'pretty' },
        },
        pane
      )
    end),
  },

  -- Ctrl+Shift+E: Start event stream watcher
  {
    key = 'E',
    mods = 'CTRL|SHIFT',
    action = wezterm.action.SpawnCommandInNewTab {
      args = { 'wezterm-watch', '.', '--format', 'events' },
    },
  },

  -- Ctrl+Shift+J: Start JSON watcher for parsing
  {
    key = 'J',
    mods = 'CTRL|SHIFT',
    action = wezterm.action.SpawnCommandInNewTab {
      args = { 'wezterm-watch', '.', '--format', 'json' },
    },
  },
}

-- ============================================================================
-- EXAMPLE 3: Status Bar Integration
-- ============================================================================

-- Update the status bar with git information from wezterm-watch
wezterm.on('update-status', function(window, pane)
  local cwd = pane:get_current_working_dir()
  if not cwd then
    return
  end

  cwd = cwd.file_path

  -- Run wezterm-watch --status to get git info
  local success, stdout, stderr = wezterm.run_child_process({
    'wezterm-watch',
    cwd,
    '--status',
    '--format', 'json',
    '--no-git', -- Remove this line to enable git status in status bar
  })

  if success and stdout and stdout ~= '' then
    local ok, status = pcall(wezterm.json_parse, stdout)
    if ok and status and status.git_branch then
      -- Format: [branch] ↑ahead ↓behind | M:modified S:staged U:untracked
      local status_text = string.format(
        ' [%s] ↑%d ↓%d | M:%d S:%d U:%d ',
        status.git_branch,
        status.git_ahead or 0,
        status.git_behind or 0,
        status.modified_files or 0,
        status.staged_files or 0,
        status.untracked_files or 0
      )

      -- Color code based on status
      local bg_color = 'rgba(0, 0, 0, 0.8)'
      local fg_color = '#ffffff'

      if status.has_conflicts then
        bg_color = 'rgba(200, 0, 0, 0.8)' -- Red for conflicts
      elseif status.modified_files and status.modified_files > 0 then
        bg_color = 'rgba(200, 100, 0, 0.8)' -- Orange for modified
      elseif status.staged_files and status.staged_files > 0 then
        bg_color = 'rgba(0, 150, 0, 0.8)' -- Green for staged
      end

      window:set_right_status(wezterm.format({
        { Background = { Color = bg_color } },
        { Foreground = { Color = fg_color } },
        { Text = status_text },
      }))
      return
    end
  end

  -- Fallback: just show cwd
  window:set_right_status(wezterm.format({
    { Text = ' ' .. cwd .. ' ' },
  }))
end)

-- ============================================================================
-- EXAMPLE 4: Toast Notifications on File Changes
-- ============================================================================

-- Parse JSON events from wezterm-watch and show toast notifications
local function parse_watch_event(json_line)
  local ok, event = pcall(wezterm.json_parse, json_line)
  if not ok then
    return nil
  end
  return event
end

-- Monitor a specific pane running wezterm-watch and show notifications
wezterm.on('user-var-changed', function(window, pane, name, value)
  if name == 'watch-event' then
    local event = parse_watch_event(value)
    if event and event.path then
      -- Show toast notification for file changes
      local icon = '📄'
      if event.event_type == 'created' then
        icon = '✨'
      elseif event.event_type == 'modified' then
        icon = '✏️'
      elseif event.event_type == 'deleted' then
        icon = '🗑️'
      elseif event.event_type == 'renamed' then
        icon = '📝'
      end

      window:toast_notification(
        'wezterm-watch',
        string.format('%s %s: %s', icon, event.event_type, event.path),
        nil,
        4000
      )
    end
  end
end)

-- ============================================================================
-- EXAMPLE 5: Project-Specific Watchers
-- ============================================================================

-- Define project-specific watcher configurations
local projects = {
  rust = {
    path = 'C:\\Users\\david\\projects\\rust-project',
    ignore = { 'target/', 'Cargo.lock' },
    format = 'pretty',
  },
  wezterm = {
    path = 'C:\\Users\\david\\wezterm',
    ignore = { 'target/', 'deps/' },
    format = 'summary',
  },
  docs = {
    path = 'C:\\Users\\david\\documents',
    ignore = { '*.tmp', '.git/' },
    format = 'events',
  },
}

-- Function to start a project watcher
local function start_project_watcher(project_name)
  local project = projects[project_name]
  if not project then
    wezterm.log_error('Unknown project: ' .. project_name)
    return
  end

  local args = {
    'wezterm-watch',
    project.path,
    '--format', project.format,
  }

  -- Add ignore patterns
  if project.ignore then
    for _, pattern in ipairs(project.ignore) do
      table.insert(args, '--ignore')
      table.insert(args, pattern)
    end
  end

  return wezterm.action.SpawnCommandInNewTab { args = args }
end

-- Key bindings for project watchers
table.insert(config.keys, {
  key = '1',
  mods = 'CTRL|ALT',
  action = start_project_watcher('rust'),
})

table.insert(config.keys, {
  key = '2',
  mods = 'CTRL|ALT',
  action = start_project_watcher('wezterm'),
})

table.insert(config.keys, {
  key = '3',
  mods = 'CTRL|ALT',
  action = start_project_watcher('docs'),
})

-- ============================================================================
-- EXAMPLE 6: Custom Event Handler
-- ============================================================================

-- Run custom commands on specific file changes
local function handle_file_event(event)
  if not event or not event.path then
    return
  end

  local path = event.path

  -- Auto-reload WezTerm config on changes
  if path:match('wezterm%.lua$') then
    wezterm.reload_configuration()
    return
  end

  -- Auto-build Rust projects on changes
  if path:match('%.rs$') and event.event_type ~= 'deleted' then
    wezterm.log_info('Rust file changed, triggering build...')
    -- Trigger build in background
    -- Note: This is just an example, actual implementation depends on your needs
  end

  -- Auto-format on save
  if path:match('%.lua$') and event.event_type == 'modified' then
    -- Run stylua or similar formatter
  end
end

-- ============================================================================
-- EXAMPLE 7: Sidebar Watcher (Always Visible)
-- ============================================================================

-- Create a persistent sidebar pane with file watcher
wezterm.on('gui-attached', function(domain)
  local mux = wezterm.mux

  for _, window in ipairs(mux.all_windows()) do
    for _, tab in ipairs(window:tabs()) do
      local panes = tab:panes()

      -- Check if we already have a watcher pane
      local has_watcher = false
      for _, p in ipairs(panes) do
        local foreground = p:get_foreground_process_name()
        if foreground and foreground:match('wezterm%-watch') then
          has_watcher = true
          break
        end
      end

      -- Create watcher sidebar if not present
      if not has_watcher and #panes > 0 then
        local main_pane = panes[1]
        main_pane:split {
          direction = 'Right',
          size = 0.25,
          args = {
            'wezterm-watch',
            '.',
            '--format', 'events',
            '--interval', '200',
          },
        }
      end
    end
  end
end)

-- ============================================================================
-- EXAMPLE 8: Minimal Configuration (Recommended Starting Point)
-- ============================================================================

--[[
-- Minimal wezterm-watch integration
-- Copy this to your wezterm.lua to get started:

local wezterm = require 'wezterm'
local config = {}

-- Key binding to start file watcher
config.keys = {
  {
    key = 'W',
    mods = 'CTRL|SHIFT',
    action = wezterm.action.SpawnCommandInNewTab {
      args = { 'wezterm-watch', '.', '--format', 'pretty' },
    },
  },
}

return config
]]

-- ============================================================================
-- Return Configuration
-- ============================================================================

return config
