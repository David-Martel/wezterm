-- ============================================================================
-- WEZTERM CONFIGURATION
-- Palette-first UX: Ctrl+Space opens the command palette, all settings
-- are exposed there via codex_ui modules (schemes, chrome, palette).
--
-- Architecture:
--   1. Bootstrap, module requires, platform/env helpers
--   2. LLM/agent integration (feature flags, launch menu, agents)
--   3. Visual config: schemes (codex_ui.schemes), fonts, GPU, window
--   4. Events: command palette (codex_ui.palette), chrome, status
--   5. Keybindings (flat), mouse, hyperlinks, cursor, performance
--   6. Optional embedded-dev extension merge
-- ============================================================================

local wezterm = require('wezterm')
local act = wezterm.action
local config = {}

if wezterm.config_builder then
  config = wezterm.config_builder()
end

config.check_for_updates = false

package.path = table.concat({
  wezterm.config_dir .. '/?.lua',
  wezterm.config_dir .. '/?/init.lua',
  package.path,
}, ';')

local shared = require('codex_ui.shared')
local prefs_io = require('codex_ui.prefs')
local schemes = require('codex_ui.schemes')
local chrome = require('codex_ui.chrome').new(wezterm, schemes, shared)
-- panels module loaded after utils init (needs utility_bins, utility_paths)
local panels = nil

local path_exists = shared.path_exists
local deep_copy = shared.deep_copy
local deep_merge = shared.deep_merge

-- ============================================================================
-- UTILITIES INTEGRATION
-- ============================================================================

-- Try to load wezterm-utils module for integrated tools
local utils_available = false
local utils = nil
local utility_bins = {
  explorer = false,
  watcher = false,
  editor = true,
}
local utility_paths = {
  explorer_bin = wezterm.home_dir .. '\\bin\\wezterm-fs-explorer.exe',
  watcher_bin = wezterm.home_dir .. '\\bin\\wezterm-watch.exe',
  -- State dir MUST be outside ~/.config/wezterm/ to avoid triggering
  -- WezTerm's config file watcher on every state write (reload loop).
  state_dir = wezterm.home_dir .. '\\.local\\state\\wezterm-utils',
}

local status, result = pcall(function()
  utils = require('wezterm-utils')
  return utils
end)

if status and result then
  utils_available = true

  -- Initialize utilities with setup
  local setup_success = utils.setup({
    explorer_bin = utility_paths.explorer_bin,
    watcher_bin = utility_paths.watcher_bin,
    state_dir = utility_paths.state_dir,
  })

  if setup_success then
    utility_bins.explorer = path_exists(utils.config.explorer_bin)
    utility_bins.watcher = path_exists(utils.config.watcher_bin)
    wezterm.log_info('WezTerm utilities initialized successfully')
  else
    wezterm.log_warn('WezTerm utilities setup failed')
    utils_available = false
  end
else
  wezterm.log_warn('WezTerm utilities module not found - utility keybindings will not be available')
end

-- Initialize panels module (needs utility_bins and paths from above)
panels = require('codex_ui.panels').new(
  wezterm, act, shared, utility_bins, utility_paths, utils_available, utils
)
panels.register_events()

-- ============================================================================
-- PLATFORM DETECTION
-- ============================================================================

local is_windows = wezterm.target_triple == 'x86_64-pc-windows-msvc'
local is_linux = wezterm.target_triple:find('linux') ~= nil
local is_mac = wezterm.target_triple:find('darwin') ~= nil

-- ============================================================================
-- ENVIRONMENT / PATH OPTIMIZATION (RUST-FIRST TOOLING)
-- ============================================================================

local rust_tool_paths = {
  wezterm.home_dir .. '\\bin',
  wezterm.home_dir .. '\\.local\\wezterm',
  wezterm.home_dir .. '\\.local\\bin',
  wezterm.home_dir .. '\\.cargo\\bin',
}

config.set_environment_variables = config.set_environment_variables or {}
config.set_environment_variables.PATH = shared.build_windows_path(rust_tool_paths, os.getenv('PATH'))

local rg_config = wezterm.home_dir .. '\\.ripgreprc'
if path_exists(rg_config) then
  config.set_environment_variables.RIPGREP_CONFIG_PATH = rg_config
end

local ui_preferences_path = wezterm.home_dir .. '/.config/wezterm/ui-preferences.lua'
local function save_ui_preferences(preferences)
  return prefs_io.save(
    ui_preferences_path,
    preferences,
    function(path)
      return shared.ensure_directory(path, is_windows)
    end,
    shared.serialize_lua_value,
    wezterm
  )
end

local ui_preferences = prefs_io.load(ui_preferences_path, path_exists, wezterm)
local panel_preferences_path = utility_paths.state_dir .. '\\panel-preferences.lua'

local function normalize_panel_preferences(preferences)
  local normalized = {
    explorer = false,
    watcher = false,
    editor = false,
  }

  for key in pairs(normalized) do
    normalized[key] = preferences and preferences[key] == true or false
  end

  return normalized
end

local function save_panel_preferences(preferences)
  return prefs_io.save(
    panel_preferences_path,
    normalize_panel_preferences(preferences),
    function(path)
      return shared.ensure_directory(path, is_windows)
    end,
    shared.serialize_lua_value,
    wezterm
  )
end

local panel_preferences = normalize_panel_preferences(
  prefs_io.load(panel_preferences_path, path_exists, wezterm)
)

local function pref(key, fallback)
  local value = ui_preferences[key]
  if value ~= nil then
    return value
  end
  return fallback
end

panels.configure_persistence(panel_preferences, function(next_preferences)
  panel_preferences = normalize_panel_preferences(next_preferences)
  return save_panel_preferences(panel_preferences)
end)

-- ============================================================================
-- LLM/AGENT WORKFLOWS (FEATURE FLAGS)
-- ============================================================================

local llm_features = {
  enabled = true,
  status_badge = true,
  startup_workspaces = false,
}

local llm_paths = {
  project_root = wezterm.home_dir .. '\\wezterm',
  docker_compose_dir = wezterm.home_dir .. '\\llm-docker',
}

local llm_defaults = {
  frontier_agent = 'claude',
  local_agent = 'ollama',
  ollama_model = 'llama3.1',
}

local build_font_from_preferences
local apply_runtime_preferences
local clear_runtime_overrides
local restart_notice_for_updates

local function checked_cmd(executable, run_command)
  if is_windows then
    local check = 'where ' .. executable .. ' >nul 2>nul'
    local msg = 'echo Missing tool: ' .. executable .. ' ^& echo Install it and retry. ^& timeout /t 3 >nul'
    return { 'cmd.exe', '/c', check .. ' && ' .. run_command .. ' || (' .. msg .. ')' }
  end

  local check = 'command -v ' .. executable .. ' >/dev/null 2>&1'
  local msg = 'echo "Missing tool: ' .. executable .. ' (install then retry)"; sleep 3'
  return { 'bash', '-lc', check .. ' && ' .. run_command .. ' || { ' .. msg .. '; }' }
end

local llm_agents = {
  gemini = {
    label = 'LLM: Gemini CLI',
    args = checked_cmd('gemini', 'gemini'),
    env = { LLM_PROVIDER = 'gemini' },
  },
  codex = {
    label = 'LLM: Codex CLI',
    args = checked_cmd('codex', 'codex'),
    env = { LLM_PROVIDER = 'codex' },
  },
  claude = {
    label = 'LLM: Claude CLI',
    args = checked_cmd('claude', 'claude'),
    env = { LLM_PROVIDER = 'claude' },
  },
  copilot = {
    label = 'LLM: GitHub Copilot CLI',
    args = checked_cmd('gh', 'gh copilot chat'),
    env = { LLM_PROVIDER = 'copilot' },
  },
  ollama = {
    label = 'LLM: Ollama Chat',
    args = checked_cmd('ollama', 'ollama run ' .. llm_defaults.ollama_model),
    env = { LLM_PROVIDER = 'ollama', LLM_MODEL = llm_defaults.ollama_model },
  },
  lmstudio = {
    label = 'LLM: LM Studio Server',
    args = checked_cmd('lms', 'lms server start'),
    env = { LLM_PROVIDER = 'lmstudio' },
  },
  docker_llm = {
    label = 'LLM: Docker Compose Stack',
    args = checked_cmd('docker', 'docker compose up'),
    cwd = llm_paths.docker_compose_dir,
    env = { LLM_PROVIDER = 'docker' },
  },
}

-- Config context for palette module
local config_context = {
  llm_agents = llm_agents,
  apply_ui = function(window, updates, message)
    local next_prefs = {}
    for k, v in pairs(ui_preferences) do next_prefs[k] = v end
    for k, v in pairs(updates) do next_prefs[k] = v end
    ui_preferences = next_prefs
    save_ui_preferences(next_prefs)
    apply_runtime_preferences(window, next_prefs)
    local restart_notice = restart_notice_for_updates(updates)
    local toast_message = message
    if restart_notice then
      if toast_message and toast_message ~= '' then
        toast_message = toast_message .. ' ' .. restart_notice
      else
        toast_message = restart_notice
      end
    end
    if toast_message then window:toast_notification('WezTerm', toast_message, nil, 3000) end
  end,
}
local palette = require('codex_ui.palette').new(wezterm, act, schemes, ui_preferences)

-- ============================================================================
-- VISUAL APPEARANCE
-- ============================================================================

local ui = schemes.ui
config.color_schemes = schemes.custom_schemes
config.color_scheme = pref('color_scheme', 'Codex PowerShell')

local base_window_frame = {
  font = wezterm.font({ family = 'Segoe UI Semibold', weight = 'DemiBold' }),
  font_size = 12.5,
  -- Match terminal bg for seamless appearance (no color seam at top)
  active_titlebar_bg = ui.bg,
  inactive_titlebar_bg = ui.bg,
  active_titlebar_fg = ui.fg,
  inactive_titlebar_fg = ui.muted,
  button_fg = ui.muted,
  button_bg = ui.bg,
  button_hover_fg = ui.fg,
  button_hover_bg = ui.surface_active,
}

local base_window_padding = {
  left = 8,
  right = 8,
  top = 4,
  bottom = 4,
}

local base_inactive_pane_hsb = {
  saturation = 0.92,
  brightness = 0.82,
}

local managed_runtime_override_keys = {
  'cell_width',
  'color_scheme',
  'font',
  'font_size',
  'line_height',
  'tab_max_width',
  'use_fancy_tab_bar',
  'window_background_opacity',
  'window_frame',
}

local runtime_restart_required_keys = {
  backdrop = true,
  custom_status_bar = true,
  custom_tab_titles = true,
  front_end = true,
  gpu_vendor = true,
}

config.bold_brightens_ansi_colors = true

-- ============================================================================
-- FONT CONFIGURATION - DIRECTWRITE (WINDOWS) / FREETYPE (LINUX)
-- ============================================================================

build_font_from_preferences = function(preferences)
  local family = preferences.font_family or pref('font_family', 'Cascadia Mono')
  local weight = preferences.font_weight or pref('font_weight', 'Regular')
  return wezterm.font_with_fallback({
    { family = family, weight = weight },
    { family = 'Cascadia Code', weight = 'Regular' },
    { family = 'Segoe UI Symbol' },
    { family = 'Consolas' },
  })
end

local function build_runtime_overrides(preferences)
  return {
    color_scheme = preferences.color_scheme,
    font = build_font_from_preferences(preferences),
    font_size = preferences.font_size,
    line_height = preferences.line_height,
    cell_width = preferences.cell_width,
    window_background_opacity = preferences.window_background_opacity,
    use_fancy_tab_bar = preferences.use_fancy_tab_bar,
    tab_max_width = preferences.tab_max_width,
    window_frame = deep_merge(base_window_frame, {
      font_size = preferences.window_frame_font_size or pref('window_frame_font_size', 12.5),
    }),
  }
end

apply_runtime_preferences = function(window, preferences)
  local existing = window:get_config_overrides() or {}
  local next_overrides = {}
  for key, value in pairs(existing) do
    next_overrides[key] = value
  end
  for _, key in ipairs(managed_runtime_override_keys) do
    next_overrides[key] = nil
  end
  for key, value in pairs(build_runtime_overrides(preferences or {})) do
    if value ~= nil then
      next_overrides[key] = value
    end
  end
  window:set_config_overrides(next_overrides)
end

clear_runtime_overrides = function(window)
  local existing = window:get_config_overrides() or {}
  for _, key in ipairs(managed_runtime_override_keys) do
    existing[key] = nil
  end
  window:set_config_overrides(existing)
end

restart_notice_for_updates = function(updates)
  local flagged = {}
  for key in pairs(updates or {}) do
    if runtime_restart_required_keys[key] then
      table.insert(flagged, key)
    end
  end
  if #flagged == 0 then
    return nil
  end
  table.sort(flagged)
  return 'Restart required for: ' .. table.concat(flagged, ', ')
end

config.font = build_font_from_preferences(ui_preferences)
config.font_size = pref('font_size', 12.5)
config.line_height = pref('line_height', 1.0)
config.cell_width = pref('cell_width', 1.0)
config.command_palette_font_size = 12.5
config.command_palette_bg_color = ui.surface
config.command_palette_fg_color = ui.fg
config.command_palette_rows = 14
config.use_cap_height_to_scale_fallback_fonts = true

-- Enable ligatures for modern coding fonts (matches WT behavior with Cascadia Code)
config.harfbuzz_features = { 'calt=1', 'clig=1', 'liga=1' }

-- Renderer selection: WebGpu by default with automatic OpenGL fallback
local configured_front_end = pref('front_end', 'WebGpu')
local gpu_vendor = pref('gpu_vendor', 'auto')

local available_gpus = nil
if wezterm.gui and wezterm.gui.enumerate_gpus then
  local ok, gpus = pcall(wezterm.gui.enumerate_gpus)
  if ok and type(gpus) == 'table' then
    available_gpus = gpus
  end
end

local function gpu_text(value)
  return tostring(value or ''):lower()
end

local function gpu_matches_vendor(gpu, vendor)
  local name = gpu_text(gpu.name)
  local driver = gpu_text(gpu.driver)
  if vendor == 'nvidia' then
    return name:find('nvidia') or driver:find('nvidia')
  end
  if vendor == 'intel' then
    return name:find('intel') or name:find('arc') or driver:find('intel')
  end
  return false
end

local function gpu_score(gpu, requested_vendor_name)
  local backend = gpu_text(gpu.backend)
  local device_type = gpu_text(gpu.device_type)
  local name = gpu_text(gpu.name)
  local score = 0

  if backend == 'gl' or device_type == 'cpu' then
    return -1000
  end

  if name:find('iddcx') or name:find('luminon') or name:find('basic render') then
    return -800
  end

  -- Prefer Dx12 over Vulkan for best Windows/DirectWrite integration
  if backend == 'dx12' then
    score = score + 50
  elseif backend == 'vulkan' then
    score = score + 20
  end

  if device_type == 'discretegpu' then
    score = score + 90
  elseif device_type == 'integratedgpu' then
    score = score + 55
  else
    score = score + 25
  end

  if gpu_matches_vendor(gpu, 'nvidia') then
    score = score + 40
  elseif gpu_matches_vendor(gpu, 'intel') then
    score = score + 25
  end

  if requested_vendor_name == 'nvidia' then
    if gpu_matches_vendor(gpu, 'nvidia') then
      score = score + 120
    else
      score = score - 60
    end
  elseif requested_vendor_name == 'intel' then
    if gpu_matches_vendor(gpu, 'intel') then
      score = score + 120
    else
      score = score - 60
    end
  end

  return score
end

local function pick_webgpu_adapter(gpus, requested_vendor_name)
  local best_gpu = nil
  local best_score = -10000

  for _, gpu in ipairs(gpus or {}) do
    local score = gpu_score(gpu, requested_vendor_name)
    if score > best_score then
      best_score = score
      best_gpu = gpu
    end
  end

  return best_gpu
end

local function power_preference_for_adapter(adapter, requested_vendor_name)
  if requested_vendor_name == 'intel' or gpu_matches_vendor(adapter or {}, 'intel') then
    return 'LowPower'
  end
  return 'HighPerformance'
end

if configured_front_end == 'WebGpu' then
  local preferred_adapter = pick_webgpu_adapter(available_gpus, gpu_vendor)
  if preferred_adapter then
    config.front_end = 'WebGpu'
    config.webgpu_power_preference = power_preference_for_adapter(preferred_adapter, gpu_vendor)
    config.webgpu_preferred_adapter = preferred_adapter
    wezterm.log_info(string.format(
      'WebGpu: selected %s [%s %s] power=%s',
      tostring(preferred_adapter.name),
      tostring(preferred_adapter.backend),
      tostring(preferred_adapter.device_type),
      config.webgpu_power_preference
    ))
  else
    -- No suitable WebGpu adapter found; fall back to OpenGL with FreeType
    config.front_end = 'OpenGL'
    config.freetype_load_target = 'HorizontalLcd'
    config.freetype_render_target = 'HorizontalLcd'
    wezterm.log_warn('WebGpu: no suitable adapter, falling back to OpenGL + FreeType')
  end
elseif configured_front_end == 'OpenGL' then
  config.front_end = 'OpenGL'
  config.freetype_load_target = 'HorizontalLcd'
  config.freetype_render_target = 'HorizontalLcd'
else
  config.front_end = configured_front_end
end

-- ============================================================================
-- WINDOW CONFIGURATION
-- ============================================================================

config.initial_cols = 120
config.initial_rows = 30

-- Keep a visible draggable title/tab area instead of a border-only window.
config.window_decorations = 'INTEGRATED_BUTTONS|RESIZE'
local win_opacity = pref('window_background_opacity', 0.98)
config.window_background_opacity = win_opacity
-- Match text bg to window bg to avoid visible cell grid boundary in padding area
config.text_background_opacity = 1.0
config.window_padding = deep_copy(base_window_padding)

-- System backdrop (None, Acrylic, Mica, Tabbed)
local backdrop = pref('backdrop', 'None')
if is_windows and backdrop ~= 'None' then
  config.win32_system_backdrop = backdrop
end

-- Always show the tab strip so the tabbed UI remains visible.
config.hide_tab_bar_if_only_one_tab = false
config.use_fancy_tab_bar = pref('use_fancy_tab_bar', true)
config.tab_bar_at_bottom = false
config.tab_max_width = pref('tab_max_width', 42)
config.show_tab_index_in_tab_bar = false
config.show_new_tab_button_in_tab_bar = true
config.enable_scroll_bar = true
config.switch_to_last_active_tab_when_closing_tab = true
config.adjust_window_size_when_changing_font_size = true

-- Window title / tab header font and colors
config.window_frame = deep_merge(base_window_frame, { font_size = pref('window_frame_font_size', 12.5) })

-- ============================================================================
-- SCROLLBACK AND BUFFER
-- ============================================================================

-- Windows cmd.exe default buffer is 9001 lines
config.scrollback_lines = 9001

-- ============================================================================
-- SHELL CONFIGURATION
-- ============================================================================

-- Default shell based on platform
if is_windows then
  -- Match Windows Terminal defaults
  config.default_prog = { 'pwsh.exe' }
  config.default_cwd = wezterm.home_dir

  local launch_menu = {}
  local function add_menu(label, args, cwd)
    table.insert(launch_menu, {
      label = label,
      args = args,
      cwd = cwd,
    })
  end

  add_menu('PowerShell', { 'pwsh.exe' }, wezterm.home_dir)
  add_menu('Windows PowerShell', { 'powershell.exe' }, wezterm.home_dir)
  add_menu('Command Prompt', { 'cmd.exe' }, wezterm.home_dir)
  add_menu('Ubuntu', { 'wsl.exe', '--distribution', 'Ubuntu', '--cd', '~' })
  add_menu('Ubuntu 22.04', { 'wsl.exe', '--distribution', 'Ubuntu-22.04', '--cd', '~' })

  local git_bash = 'C:\\Program Files\\Git\\bin\\bash.exe'
  if path_exists(git_bash) then
    add_menu('Git Bash', { git_bash, '-l' }, wezterm.home_dir)
  end

  local msys2 = 'C:\\codedev\\msys64\\msys2.exe'
  if path_exists(msys2) then
    add_menu('MSYS2', { msys2 }, wezterm.home_dir)
  end

  local vs_dev_cmd = 'C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\Common7\\Tools\\VsDevCmd.bat'
  if path_exists(vs_dev_cmd) then
    add_menu('Developer Command Prompt for VS 2022', { 'cmd.exe', '/k', vs_dev_cmd })
  end

  local vs_dev_shell = 'C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\Common7\\Tools\\Launch-VsDevShell.ps1'
  if path_exists(vs_dev_shell) then
    add_menu('Developer PowerShell for VS 2022', {
      'pwsh.exe',
      '-NoLogo',
      '-NoProfile',
      '-Command',
      '& "' .. vs_dev_shell .. '" -Arch amd64 -HostArch amd64',
    })
  end

  -- Add LLM agents to launch menu (accessible via + dropdown)
  if pref('llm_agents', true) then
    for name, agent in pairs(llm_agents) do
      table.insert(launch_menu, {
        label = agent.label or name,
        args = agent.args,
        cwd = agent.cwd,
        set_environment_variables = agent.env,
      })
    end
  end

  config.launch_menu = launch_menu
elseif is_linux then
  config.default_prog = { 'bash', '-l' }
end

wezterm.on('reset-ui-defaults', function(window, pane)
  ui_preferences = {}
  save_ui_preferences(ui_preferences)
  clear_runtime_overrides(window)
  window:toast_notification('WezTerm', 'Reset runtime UI overrides. Restart if renderer, backdrop, or chrome flags changed.', nil, 3500)
end)

wezterm.on('show-render-diagnostics', function(window, pane)
  local dims = window:get_dimensions()
  local effective = window:effective_config()
  local message = string.format(
    'renderer=%s dpi=%s font=%.1f line=%.2f cell=%.2f',
    tostring(effective.front_end or 'unknown'),
    tostring(dims.dpi),
    tonumber(effective.font_size or 0),
    tonumber(effective.line_height or 0),
    tonumber(effective.cell_width or 0)
  )
  window:toast_notification('WezTerm', message, nil, 4500)
end)

wezterm.on('augment-command-palette', function(window, pane)
  return palette.build_palette(config_context)
end)

wezterm.on('show-profiles', function(window, pane)
  local choices = {}
  for _, profile in ipairs(schemes.appearance_profiles) do
    table.insert(choices, { id = profile.id, label = profile.label })
  end
  window:perform_action(act.InputSelector({
    title = 'Appearance Profiles', fuzzy = true, choices = choices,
    action = wezterm.action_callback(function(inner_window, _, id)
      for _, profile in ipairs(schemes.appearance_profiles) do
        if profile.id == id then
          config_context.apply_ui(inner_window, profile.overrides, profile.label .. ' applied')
          return
        end
      end
    end),
  }), pane)
end)

wezterm.on('split-auto', function(window, pane)
  local dims = pane:get_dimensions()
  local w = dims.pixel_width or dims.cols or 0
  local h = dims.pixel_height or dims.rows or 0
  if w >= h and w > 0 and h > 0 then
    window:perform_action(act.SplitHorizontal({ domain = 'CurrentPaneDomain' }), pane)
  else
    window:perform_action(act.SplitVertical({ domain = 'CurrentPaneDomain' }), pane)
  end
end)

-- ============================================================================
-- KEYBINDINGS (flattened, palette-first)
-- ============================================================================

local function copy_or_send_ctrl_c(window, pane)
  local selection = window:get_selection_text_for_pane(pane)
  if selection and selection ~= '' then
    window:perform_action(act.CopyTo('Clipboard'), pane)
  else
    window:perform_action(act.SendKey({ key = 'c', mods = 'CTRL' }), pane)
  end
end

-- Prevent Alt keys from being sent to the application so our bindings work
config.send_composed_key_when_left_alt_is_pressed = false
config.send_composed_key_when_right_alt_is_pressed = false

config.keys = {
  -- Tier 1: Instant access
  { key = 'Space', mods = 'CTRL', action = act.ActivateCommandPalette },
  { key = 'p', mods = 'CTRL|SHIFT', action = act.ActivateCommandPalette },
  -- Panel toggles
  { key = '1', mods = 'ALT', action = act.EmitEvent('toggle-explorer') },
  { key = '2', mods = 'ALT', action = act.EmitEvent('toggle-watcher') },
  { key = '3', mods = 'ALT', action = act.EmitEvent('toggle-editor') },

  -- Tier 2: WT-standard
  { key = 'c', mods = 'CTRL', action = wezterm.action_callback(copy_or_send_ctrl_c) },
  { key = 'v', mods = 'CTRL', action = act.PasteFrom('Clipboard') },
  { key = 't', mods = 'CTRL|SHIFT', action = act.SpawnTab('CurrentPaneDomain') },
  { key = 'w', mods = 'CTRL|SHIFT', action = act.CloseCurrentTab({ confirm = true }) },
  { key = 'Tab', mods = 'CTRL', action = act.ActivateTabRelative(1) },
  { key = 'Tab', mods = 'CTRL|SHIFT', action = act.ActivateTabRelative(-1) },
  { key = 'f', mods = 'CTRL|SHIFT', action = act.Search({ CaseSensitiveString = '' }) },
  { key = 'd', mods = 'ALT|SHIFT', action = wezterm.action_callback(function(window, pane)
    window:perform_action(act.EmitEvent('split-auto'), pane)
  end) },
  { key = 'PageUp', mods = 'SHIFT', action = act.ScrollByPage(-1) },
  { key = 'PageDown', mods = 'SHIFT', action = act.ScrollByPage(1) },
}

-- ============================================================================
-- MOUSE BINDINGS
-- ============================================================================

config.mouse_bindings = {
  -- Right click pastes from clipboard
  {
    event = { Down = { streak = 1, button = 'Right' } },
    mods = 'NONE',
    action = act.PasteFrom('Clipboard'),
  },

  -- Ctrl+Click opens hyperlinks
  {
    event = { Up = { streak = 1, button = 'Left' } },
    mods = 'CTRL',
    action = act.OpenLinkAtMouseCursor,
  },

  -- Disable Ctrl+left-click drag (conflicts with selection)
  {
    event = { Drag = { streak = 1, button = 'Left' } },
    mods = 'CTRL',
    action = act.Nop,
  },
}

-- ============================================================================
-- HYPERLINK RULES
-- ============================================================================

-- Detect URLs, file paths, and common patterns
config.hyperlink_rules = {
  -- Standard URLs
  {
    regex = '\\b\\w+://[\\w.-]+\\.[a-z]{2,15}\\S*\\b',
    format = '$0',
  },

  -- Email addresses
  {
    regex = [[\b\w+@[\w-]+(\.[\w-]+)+\b]],
    format = 'mailto:$0',
  },

  -- File paths (Windows drives)
  {
    regex = [[[A-Z]:\\(?:[^\s:*?"<>|]+\\)*[^\s:*?"<>|]+]],
    format = 'file:///$0',
  },
}

-- ============================================================================
-- VISUAL ENHANCEMENTS (Warp-inspired)
-- ============================================================================

-- Cursor style
config.default_cursor_style = 'SteadyBar'
config.cursor_blink_rate = 500
config.cursor_blink_ease_in = 'Constant'
config.cursor_blink_ease_out = 'Constant'

-- Visual bell (instead of audible)
config.audible_bell = 'Disabled'
config.visual_bell = {
  fade_in_function = 'EaseIn',
  fade_in_duration_ms = 50,
  fade_out_function = 'EaseOut',
  fade_out_duration_ms = 50,
}

-- Inactive pane dimming
config.inactive_pane_hsb = {
  saturation = base_inactive_pane_hsb.saturation,
  brightness = base_inactive_pane_hsb.brightness,
}

-- ============================================================================
-- PERFORMANCE OPTIMIZATIONS
-- ============================================================================

local configured_max_fps = pref('max_fps', 165)
if configured_max_fps < 100 then configured_max_fps = 100 end
config.max_fps = configured_max_fps
config.animation_fps = math.min(configured_max_fps, 120)
config.alternate_buffer_wheel_scroll_speed = 3
if is_linux then config.enable_wayland = true end

-- ============================================================================
-- STATUS BAR (Warp-inspired)
-- ============================================================================

if llm_features.enabled and llm_features.status_badge then
  wezterm.on('user-var-changed', function(window, pane, name, value)
    if not name:match('^LLM_') then
      return
    end

    local state = wezterm.GLOBAL.llm_status or {}
    local key = name:sub(5):lower()
    state[key] = value
    wezterm.GLOBAL.llm_status = state
  end)
end

local enable_custom_status = pref('custom_status_bar', true)

if enable_custom_status then
  wezterm.on('update-right-status', function(window, pane)
    chrome.update_right_status(window, pane)
  end)
end

-- ============================================================================
-- TAB TITLE (Show process and directory)
-- ============================================================================

local enable_custom_tab_title = pref('custom_tab_titles', true)

if enable_custom_tab_title then
  wezterm.on('format-tab-title', function(tab, tabs, panes, cfg, hover, max_width)
    return chrome.format_tab_title(tab, tabs, panes, cfg, hover, max_width)
  end)
end

-- ============================================================================
-- LLM WORKSPACE STARTUP (OPTIONAL)
-- ============================================================================

if llm_features.enabled and llm_features.startup_workspaces then
  local mux = wezterm.mux

  wezterm.on('gui-startup', function(cmd)
    local args = cmd and cmd.args or nil

    local function spawn_workspace(name, cwd, agent_name, use_args)
      local spawn_args = use_args and args or nil
      local tab, pane, window = mux.spawn_window({
        workspace = name,
        cwd = cwd,
        args = spawn_args,
      })

      local agent = llm_agents[agent_name]
      if agent then
        pane:split({
          direction = 'Right',
          size = 0.45,
          args = agent.args,
          cwd = agent.cwd or cwd,
          set_environment_variables = agent.env,
        })
      end

      pane:split({
        direction = 'Bottom',
        size = 0.35,
        cwd = cwd,
      })

      return window
    end

    spawn_workspace('Frontier', wezterm.home_dir, llm_defaults.frontier_agent, true)
    spawn_workspace('Local', llm_paths.project_root, llm_defaults.local_agent, false)

    mux.set_active_workspace('Frontier')
  end)
end

-- ============================================================================
-- EMBEDDED DEVELOPMENT INTEGRATION
-- ============================================================================

-- Attempt to load embedded development configuration if it exists
local embedded_config_path = wezterm.config_dir .. '/embedded-dev-config.lua'
local embedded_config_exists = false

-- Check if file exists
local f = io.open(embedded_config_path, 'r')
if f ~= nil then
  io.close(f)
  embedded_config_exists = true
end

if embedded_config_exists then
  local status, embedded_config = pcall(require, 'embedded-dev-config')
  if status and embedded_config then
    wezterm.log_info('Loading embedded development configuration...')

    -- Merge embedded development keys (if any)
    if embedded_config.keys then
      for _, key in ipairs(embedded_config.keys) do
        table.insert(config.keys, key)
      end
    end

    -- Merge launch menu items
    if embedded_config.launch_menu then
      config.launch_menu = config.launch_menu or {}
      for _, item in ipairs(embedded_config.launch_menu) do
        table.insert(config.launch_menu, item)
      end
    end
  end
end

-- ============================================================================
-- RETURN CONFIGURATION
-- ============================================================================

return config
