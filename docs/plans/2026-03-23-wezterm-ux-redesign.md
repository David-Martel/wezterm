# WezTerm UX/UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the 3250-line monolithic `.wezterm.lua` into a modular, Windows Terminal-inspired configuration with WebGpu rendering, mouse-first chrome, and a flat command palette.

**Architecture:** Phase 1 fixes text rendering by switching to WebGpu/DirectWrite and killing env vars. Phase 2 rewrites the chrome (tab bar, command bar, keybindings) as focused Lua modules. Phase 3 adds the panel toggle system. Phase 4 is Rust-side fork work (separate plan, coordinated via agent-bus).

**Tech Stack:** Lua (WezTerm config API), WebGpu/DirectWrite (rendering), Rust (Phase 4 fork crates)

**Spec:** `docs/superpowers/specs/2026-03-23-wezterm-ux-redesign-design.md`

---

## File Map

### Phase 1: Config Changes Only
| Action | File | Responsibility |
|--------|------|----------------|
| Create | `~/.config/wezterm/ui-preferences.lua` | All user settings (rendering, appearance, features) |
| Modify | `~/.wezterm.lua` | Replace env var gates with config file reads, switch renderer |

### Phase 2: Chrome Overhaul
| Action | File | Responsibility |
|--------|------|----------------|
| Create | `~/.config/wezterm/codex_ui/chrome.lua` | Tab bar formatting + command bar rendering |
| Create | `~/.config/wezterm/codex_ui/palette.lua` | Categorized command palette entries (absorbs dispatch.lua) |
| Create | `~/.config/wezterm/codex_ui/schemes.lua` | Color schemes + palette presets (extracted from .wezterm.lua) |
| Rewrite | `~/.wezterm.lua` | Slim bootstrap (~300 lines) loading modules |
| Keep | `~/.config/wezterm/codex_ui/shared.lua` | Utilities (trimmed) |
| Keep | `~/.config/wezterm/codex_ui/prefs.lua` | Persistence helpers |
| Keep | `~/.config/wezterm/wezterm-utils-state/` | Runtime state directory (unmodified) |
| Delete | `~/.config/wezterm/codex_ui/dispatch.lua` | Merged into palette.lua |

### Dependencies
- Task 4 (palette.lua) depends on Task 3 (schemes.lua) -- palette imports schemes
- Task 5 (chrome.lua) depends on Task 3 (schemes.lua) -- chrome imports schemes
- Tasks 6a/6b/6c depend on Tasks 3, 4, 5 -- bootstrap wires all modules

### Phase 3: Panel System (follow-up plan)
| Action | File | Responsibility |
|--------|------|----------------|
| Create | `~/.config/wezterm/codex_ui/panels.lua` | Companion panel toggle + state persistence |
| Modify | `~/.config/wezterm/codex_ui/chrome.lua` | Add panel toggle state indicators to command bar |
| Modify | `~/.wezterm.lua` | Wire panel toggle keybindings and events |

---

## Phase 1: Rendering + Config Cleanup

### Task 1: Create ui-preferences.lua with rendering defaults

**Files:**
- Create: `~/.config/wezterm/ui-preferences.lua`

- [ ] **Step 1: Create the preferences file with all settings from spec Section 4.1**

```lua
-- ~/.config/wezterm/ui-preferences.lua
-- All WezTerm settings. Hot-reloadable. Replaces env vars.
-- Edit this file to change rendering, appearance, and feature flags.
return {
  -- Rendering
  front_end = 'WebGpu',
  gpu_vendor = 'auto',
  max_fps = 165,
  backdrop = 'Acrylic',

  -- Appearance
  color_scheme = 'Codex PowerShell',
  font_family = 'Cascadia Mono',
  font_weight = 'Regular',
  font_size = 12.5,
  line_height = 1.0,
  cell_width = 1.0,
  window_background_opacity = 0.98,

  -- Tab bar
  use_fancy_tab_bar = true,
  tab_max_width = 42,

  -- Features
  custom_status_bar = true,
  custom_tab_titles = true,
  llm_agents = true,
  llm_prompt_capture = true,
  embedded_dev = true,
}
```

- [ ] **Step 2: Verify the file loads correctly**

Run: `wezterm cli list` (or restart WezTerm)
Expected: WezTerm starts without errors. Check debug log for "Failed to load UI preferences" -- should NOT appear.

- [ ] **Step 3: Commit**

```bash
git add ~/.config/wezterm/ui-preferences.lua
git commit -m "feat(wezterm): create ui-preferences.lua with rendering defaults

Replaces 8 env vars (WEZTERM_USE_WEBGPU, WEZTERM_GPU_VENDOR,
WEZTERM_MAX_FPS, WEZTERM_USE_ACRYLIC, WEZTERM_UI_SELFTEST,
WEZTERM_UI_SELFTEST_APPLY, WEZTERM_DISABLE_CUSTOM_STATUS,
WEZTERM_DISABLE_CUSTOM_TAB_TITLE) with config file entries."
```

---

### Task 2: Switch .wezterm.lua to WebGpu with config-driven settings

**Files:**
- Modify: `~/.wezterm.lua:536-650` (renderer section)
- Modify: `~/.wezterm.lua:131-144` (preferences loading)

This task modifies the existing config to read from ui-preferences.lua instead of env vars, and switches the default renderer to WebGpu.

- [ ] **Step 1: Update the preferences loading to merge with ui-preferences.lua**

In `~/.wezterm.lua`, find the preferences loading block (around line 131-144). Replace the `ui_preferences_path` and loading with a version that also reads the new rendering/feature keys:

Replace lines 131-144:
```lua
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
```

With:
```lua
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

-- Config-driven settings (replaces all env vars)
local function pref(key, fallback)
  local value = ui_preferences[key]
  if value ~= nil then
    return value
  end
  return fallback
end
```

- [ ] **Step 2: Replace the renderer section (lines ~536-673) with config-driven WebGpu**

Remove the entire env-var-gated WebGpu/OpenGL block and replace with:

```lua
-- ============================================================================
-- RENDERING - WebGpu + DirectWrite (config-driven, no env vars)
-- ============================================================================

local requested_front_end = pref('front_end', 'WebGpu')
local requested_gpu_vendor = tostring(pref('gpu_vendor', 'auto')):lower()

local available_gpus = nil
if wezterm.gui and wezterm.gui.enumerate_gpus then
  local ok, gpus = pcall(wezterm.gui.enumerate_gpus)
  if ok and type(gpus) == 'table' then
    available_gpus = gpus
  end
end

-- GPU scoring (kept from original, un-gated)
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
  if backend == 'gl' or device_type == 'cpu' then return -1000 end
  if name:find('iddcx') or name:find('luminon') or name:find('basic render') then return -800 end
  if device_type == 'discretegpu' then score = score + 90
  elseif device_type == 'integratedgpu' then score = score + 55
  else score = score + 25 end
  if gpu_matches_vendor(gpu, 'nvidia') then score = score + 40
  elseif gpu_matches_vendor(gpu, 'intel') then score = score + 25 end
  if requested_vendor_name == 'nvidia' then
    score = score + (gpu_matches_vendor(gpu, 'nvidia') and 120 or -60)
  elseif requested_vendor_name == 'intel' then
    score = score + (gpu_matches_vendor(gpu, 'intel') and 120 or -60)
  end
  return score
end

local function pick_best_adapter(gpus, vendor)
  local best_gpu, best_score = nil, -10000
  for _, gpu in ipairs(gpus or {}) do
    local score = gpu_score(gpu, vendor)
    if score > best_score then
      best_score = score
      best_gpu = gpu
    end
  end
  return best_gpu
end

-- Apply rendering config
if requested_front_end == 'WebGpu' and available_gpus then
  local adapter = pick_best_adapter(available_gpus, requested_gpu_vendor)
  if adapter then
    config.front_end = 'WebGpu'
    config.webgpu_preferred_adapter = adapter
    config.webgpu_power_preference = gpu_matches_vendor(adapter, 'intel') and 'LowPower' or 'HighPerformance'
    wezterm.log_info('WebGpu: using ' .. tostring(adapter.name))
  else
    config.front_end = 'OpenGL'
    config.freetype_load_target = 'HorizontalLcd'
    config.freetype_render_target = 'HorizontalLcd'
    wezterm.log_warn('WebGpu: no suitable adapter found, falling back to OpenGL')
  end
elseif requested_front_end == 'WebGpu' then
  -- No GPU enumeration available, try WebGpu anyway
  config.front_end = 'WebGpu'
  config.webgpu_power_preference = 'HighPerformance'
else
  config.front_end = 'OpenGL'
  config.freetype_load_target = 'HorizontalLcd'
  config.freetype_render_target = 'HorizontalLcd'
end
```

- [ ] **Step 3: Replace the font configuration section (lines ~506-531) with config-driven fonts**

Replace with:
```lua
-- ============================================================================
-- FONT CONFIGURATION - config-driven
-- ============================================================================

local font_family = pref('font_family', 'Cascadia Mono')
local font_weight = pref('font_weight', 'Regular')

config.font = wezterm.font_with_fallback({
  { family = font_family, weight = font_weight },
  { family = 'Cascadia Code', weight = 'Regular' },
  { family = 'Segoe UI Symbol' },
  { family = 'Consolas' },
})
config.font_size = pref('font_size', 12.5)
config.line_height = pref('line_height', 1.0)
config.cell_width = pref('cell_width', 1.0)
config.harfbuzz_features = { 'calt=1', 'clig=1', 'liga=1' }
config.use_cap_height_to_scale_fallback_fonts = true
```

- [ ] **Step 4: Replace performance/FPS section (lines ~2993-3007) with config-driven values**

Replace with:
```lua
-- ============================================================================
-- PERFORMANCE - config-driven
-- ============================================================================

local configured_max_fps = pref('max_fps', 165)
if configured_max_fps < 100 then configured_max_fps = 100 end
config.max_fps = configured_max_fps
config.animation_fps = math.min(configured_max_fps, 120)

if is_linux then
  config.enable_wayland = true
end
```

- [ ] **Step 5: Replace Acrylic section (lines ~689-698) with config-driven backdrop**

Replace the env-var-gated acrylic block with:
```lua
local backdrop = pref('backdrop', 'Auto')
if is_windows and backdrop ~= 'None' and backdrop ~= 'Auto' then
  config.win32_system_backdrop = backdrop
end
```

- [ ] **Step 6: Remove all remaining env var reads**

Search for `os.getenv('WEZTERM_` in the file and remove:
- `WEZTERM_UI_SELFTEST` / `WEZTERM_UI_SELFTEST_APPLY` blocks (lines ~1327-1328 and ~2609-2648) -- delete the entire selftest blocks
- `WEZTERM_DISABLE_CUSTOM_STATUS` (line ~3026-3028) -- replace with `pref('custom_status_bar', true)`
- `WEZTERM_DISABLE_CUSTOM_TAB_TITLE` (line ~3098-3100) -- replace with `pref('custom_tab_titles', true)`

- [ ] **Step 7: Remove the `font_rasterizer = 'Harfbuzz'` line (line ~527)**

Delete: `config.font_rasterizer = 'Harfbuzz'`

- [ ] **Step 8: Remove the `webgpu_env` / `use_webgpu` / `requested_gpu_vendor` env var blocks (lines ~539-546)**

These are replaced by the `pref()` calls in Step 2.

- [ ] **Step 9: Test the rendering**

Restart WezTerm. Verify:
1. Title bar shows WezTerm (no crash on startup)
2. Text looks noticeably different (DirectWrite vs FreeType)
3. GPU adapter is logged: check WezTerm debug overlay or log for "WebGpu: using ..."
4. Acrylic backdrop is visible (slight transparency)
5. No `os.getenv('WEZTERM_` calls remain in the file

Run: `grep -c "os.getenv('WEZTERM_" ~/.wezterm.lua`
Expected: 0

- [ ] **Step 10: Test OpenGL fallback path**

Temporarily edit `ui-preferences.lua` to set `front_end = 'OpenGL'`. Restart WezTerm. Verify it starts with FreeType rendering (text will look different/worse). Restore `front_end = 'WebGpu'` and restart to confirm WebGpu works again.

- [ ] **Step 11: Commit**

```bash
git add ~/.wezterm.lua
git commit -m "feat(wezterm): switch to WebGpu/DirectWrite, kill all env vars

- front_end = WebGpu by default (was OpenGL, gated by env var)
- GPU adapter auto-selected (NVIDIA > Intel > Integrated)
- Font weight Regular (was DemiLight), added Segoe UI Symbol fallback
- Removed font_rasterizer = Harfbuzz (let DirectWrite handle it)
- Removed all freetype_load/render_target overrides
- All 8 WEZTERM_* env vars replaced with ui-preferences.lua reads
- Removed UI selftest blocks entirely
- Acrylic backdrop from config, not env var"
```

---

## Phase 2: Chrome Overhaul

### Task 3: Create codex_ui/schemes.lua (extract color schemes)

**Files:**
- Create: `~/.config/wezterm/codex_ui/schemes.lua`

- [ ] **Step 1: Create schemes.lua with all color schemes and palette presets**

Extract from `.wezterm.lua` lines 290-937 (the `ui` table, `custom_schemes`, `curated_scheme_names`, `palette_theme_presets`, `palette_slot_catalog`) into a standalone module:

```lua
-- Color schemes and palette presets for the WezTerm config.
local M = {}

M.ui = {
  bg = '#0b1220',
  surface = '#111827',
  surface_alt = '#172033',
  surface_active = '#1f2a40',
  border = '#2a3a57',
  fg = '#f5f7fb',
  muted = '#b6c2d1',
  accent = '#7dd3fc',
  accent_strong = '#38bdf8',
  selection_bg = '#1d4ed8',
  selection_fg = '#f8fafc',
}

M.custom_schemes = {
  ['Codex PowerShell'] = {
    foreground = M.ui.fg,
    background = M.ui.bg,
    cursor_bg = M.ui.accent,
    cursor_fg = M.ui.bg,
    cursor_border = M.ui.accent,
    selection_bg = M.ui.selection_bg,
    selection_fg = M.ui.selection_fg,
    scrollbar_thumb = '#314158',
    split = M.ui.border,
    compose_cursor = '#f59e0b',
    ansi = { '#1b2333', '#f87171', '#4ade80', '#facc15', '#60a5fa', '#c084fc', '#22d3ee', '#cbd5e1' },
    brights = { '#334155', '#fca5a5', '#86efac', '#fde047', '#93c5fd', '#d8b4fe', '#67e8f9', '#ffffff' },
    tab_bar = {
      background = M.ui.bg,
      active_tab = { bg_color = M.ui.surface_active, fg_color = M.ui.fg, intensity = 'Bold' },
      inactive_tab = { bg_color = M.ui.surface, fg_color = M.ui.muted },
      inactive_tab_hover = { bg_color = M.ui.surface_alt, fg_color = M.ui.fg, italic = false },
      new_tab = { bg_color = M.ui.surface, fg_color = M.ui.muted },
      new_tab_hover = { bg_color = M.ui.surface_alt, fg_color = M.ui.fg, italic = false },
    },
  },
  ['Codex Ember'] = {
    foreground = '#fff7ed', background = '#17120f',
    cursor_bg = '#fb923c', cursor_fg = '#17120f', cursor_border = '#fb923c',
    selection_bg = '#9a3412', selection_fg = '#fff7ed',
    ansi = { '#221814', '#f87171', '#86efac', '#fbbf24', '#fb923c', '#f0abfc', '#2dd4bf', '#e7d8cf' },
    brights = { '#4b2f26', '#fca5a5', '#bbf7d0', '#fde68a', '#fdba74', '#f5d0fe', '#99f6e4', '#fff7ed' },
  },
  ['Codex Graphite'] = {
    foreground = '#f8fafc', background = '#111318',
    cursor_bg = '#a5b4fc', cursor_fg = '#111318', cursor_border = '#a5b4fc',
    selection_bg = '#334155', selection_fg = '#f8fafc',
    ansi = { '#1f2937', '#f87171', '#34d399', '#facc15', '#818cf8', '#c084fc', '#22d3ee', '#cbd5e1' },
    brights = { '#475569', '#fca5a5', '#6ee7b7', '#fde047', '#a5b4fc', '#d8b4fe', '#67e8f9', '#ffffff' },
  },
}

M.curated_scheme_names = {
  'Codex PowerShell', 'Codex Ember', 'Codex Graphite',
  'OneHalfDark', 'Builtin Solarized Dark', 'Dracula', 'Gruvbox Dark', 'Campbell',
}

M.palette_presets = {
  studio = {
    background = M.ui.bg, foreground = M.ui.fg,
    tab_bar_bg = M.ui.bg, active_tab_bg = M.ui.surface_active, active_tab_fg = M.ui.fg,
    inactive_tab_bg = M.ui.surface, inactive_tab_fg = M.ui.muted,
    selection_bg = M.ui.selection_bg, selection_fg = M.ui.selection_fg,
    accent_cursor = M.ui.accent, border = M.ui.border,
  },
  ember = {
    background = '#17120f', foreground = '#fff7ed',
    tab_bar_bg = '#120f0d', active_tab_bg = '#7c2d12', active_tab_fg = '#fff7ed',
    inactive_tab_bg = '#2a211b', inactive_tab_fg = '#fed7aa',
    selection_bg = '#9a3412', selection_fg = '#fff7ed',
    accent_cursor = '#fb923c', border = '#7c2d12',
  },
  graphite = {
    background = '#111318', foreground = '#f8fafc',
    tab_bar_bg = '#0f1217', active_tab_bg = '#334155', active_tab_fg = '#ffffff',
    inactive_tab_bg = '#1f2937', inactive_tab_fg = '#cbd5e1',
    selection_bg = '#334155', selection_fg = '#f8fafc',
    accent_cursor = '#a5b4fc', border = '#475569',
  },
  high_contrast = {
    background = '#05070a', foreground = '#ffffff',
    tab_bar_bg = '#000000', active_tab_bg = '#0f172a', active_tab_fg = '#ffffff',
    inactive_tab_bg = '#111827', inactive_tab_fg = '#dbeafe',
    selection_bg = '#1d4ed8', selection_fg = '#ffffff',
    accent_cursor = '#38bdf8', border = '#60a5fa',
  },
}

M.font_family_presets = {
  'Cascadia Mono', 'Cascadia Code', 'Consolas', 'JetBrains Mono',
  'Aptos Mono', 'FiraCode Nerd Font Mono', 'Iosevka Term', 'Source Code Pro',
}

M.appearance_profiles = {
  { id = 'powershell-studio', label = 'PowerShell Studio',
    overrides = { color_scheme = 'Codex PowerShell', font_family = 'Cascadia Mono', font_size = 12.5 } },
  { id = 'presentation', label = 'Presentation',
    overrides = { color_scheme = 'Codex Ember', font_family = 'Cascadia Code', font_size = 14.5 } },
  { id = 'compact', label = 'Compact Focus',
    overrides = { color_scheme = 'Codex Graphite', font_family = 'Consolas', font_size = 11.5, use_fancy_tab_bar = false } },
}

return M
```

- [ ] **Step 2: Verify module loads**

Add a temporary line to `.wezterm.lua` after the `require('codex_ui.shared')` line:
```lua
local schemes = require('codex_ui.schemes')
wezterm.log_info('Loaded ' .. tostring(#schemes.curated_scheme_names) .. ' curated schemes')
```

Restart WezTerm, check log for "Loaded 8 curated schemes".

- [ ] **Step 3: Commit**

```bash
git add ~/.config/wezterm/codex_ui/schemes.lua
git commit -m "feat(wezterm): extract color schemes into codex_ui/schemes.lua"
```

---

### Task 4: Create codex_ui/palette.lua (command palette with categories)

**Files:**
- Create: `~/.config/wezterm/codex_ui/palette.lua`

- [ ] **Step 1: Create palette.lua with categorized command palette entries**

This module absorbs `dispatch.lua`'s helper functions and adds the categorized palette from spec Section 3.3:

```lua
-- Categorized command palette and dispatch helpers for WezTerm.
-- Absorbs and replaces dispatch.lua.
local M = {}

function M.new(wezterm, act, schemes, prefs)
  local api = {}
  local ui = schemes.ui

  -- Dispatch helper (from dispatch.lua)
  function api.show_selector(window, pane, title, choices, on_select, description)
    window:perform_action(
      act.InputSelector({
        title = title,
        fuzzy = true,
        description = description or 'Select an item and press Enter.',
        choices = choices,
        action = wezterm.action_callback(function(inner_window, inner_pane, id, label)
          if id then on_select(inner_window, inner_pane, id, label) end
        end),
      }),
      pane
    )
  end

  function api.command_palette_entry(brief, action, doc, icon)
    return { brief = brief, action = action, doc = doc, icon = icon }
  end

  -- Tag prefixes for visual categorization in the palette
  local function tagged(tag, label)
    return '[' .. tag .. '] ' .. label
  end

  -- Build the full categorized palette
  function api.build_palette(config_context)
    local entries = {}
    local function add(tag, brief, action, doc)
      table.insert(entries, api.command_palette_entry(tagged(tag, brief), action, doc))
    end

    -- LLM agents
    local llm_agents = config_context.llm_agents or {}
    for name, agent in pairs(llm_agents) do
      add('LLM', 'Launch ' .. (agent.label or name), act.SpawnCommandInNewTab({
        args = agent.args, cwd = agent.cwd,
        set_environment_variables = agent.env,
      }))
    end

    -- UI settings
    add('UI', 'Settings: Color Scheme', act.InputSelector({
      title = 'Color Scheme', fuzzy = true,
      choices = (function()
        local choices = {}
        for _, name in ipairs(schemes.curated_scheme_names) do
          table.insert(choices, { id = name, label = name })
        end
        return choices
      end)(),
      action = wezterm.action_callback(function(window, _, id)
        if id and config_context.apply_ui then
          config_context.apply_ui(window, { color_scheme = id }, 'Color scheme: ' .. id)
        end
      end),
    }), 'Switch the active color theme')

    add('UI', 'Settings: Font Size', act.InputSelector({
      title = 'Font Size', fuzzy = true,
      choices = (function()
        local sizes = { 10.5, 11.0, 11.5, 12.0, 12.5, 13.0, 14.0, 15.0, 16.0 }
        local choices = {}
        for _, s in ipairs(sizes) do
          table.insert(choices, { id = tostring(s), label = string.format('%.1f pt', s) })
        end
        return choices
      end)(),
      action = wezterm.action_callback(function(window, _, id)
        local size = tonumber(id)
        if size and config_context.apply_ui then
          config_context.apply_ui(window, { font_size = size }, string.format('Font size: %.1f pt', size))
        end
      end),
    }), 'Change font size')

    add('UI', 'Settings: Appearance Profiles', act.EmitEvent('show-profiles'), 'Switch between PowerShell Studio, Presentation, and Compact profiles')
    add('UI', 'Settings: Reset Defaults', act.EmitEvent('reset-ui-defaults'), 'Restore all settings to defaults')

    -- Tools
    add('TOOL', 'Open Explorer Panel', act.EmitEvent('toggle-explorer'), 'Toggle filesystem explorer')
    add('TOOL', 'Open Watcher Panel', act.EmitEvent('toggle-watcher'), 'Toggle file watcher')
    add('TOOL', 'Open Editor Panel', act.EmitEvent('toggle-editor'), 'Toggle inline editor')
    add('TOOL', 'Show Render Diagnostics', act.EmitEvent('show-render-diagnostics'), 'Display renderer, GPU, font info')

    -- Shells
    add('SHELL', 'New PowerShell Tab', act.SpawnCommandInNewTab({ args = { 'pwsh.exe' } }))
    add('SHELL', 'New Command Prompt Tab', act.SpawnCommandInNewTab({ args = { 'cmd.exe' } }))
    add('SHELL', 'New WSL Ubuntu Tab', act.SpawnCommandInNewTab({ args = { 'wsl.exe', '--distribution', 'Ubuntu', '--cd', '~' } }))

    -- Navigation
    add('NAV', 'Open Launcher', act.ShowLauncherArgs({
      flags = 'FUZZY|LAUNCH_MENU_ITEMS|COMMANDS|WORKSPACES|TABS',
      title = 'Launcher',
    }))
    add('NAV', 'Search Scrollback', act.Search({ CaseSensitiveString = '' }))
    add('NAV', 'Split Pane Auto', act.EmitEvent('split-auto'))

    return entries
  end

  return api
end

return M
```

- [ ] **Step 2: Verify module loads**

Add a temporary line to `.wezterm.lua`:
```lua
local palette_mod = require('codex_ui.palette')
assert(type(palette_mod.new) == 'function', 'palette.new must be a function')
wezterm.log_info('palette.lua loaded successfully')
```
Restart WezTerm, check log for "palette.lua loaded successfully". Remove temp lines.

- [ ] **Step 3: Commit**

```bash
git add ~/.config/wezterm/codex_ui/palette.lua
git commit -m "feat(wezterm): create codex_ui/palette.lua with categorized command palette"
```

---

### Task 5: Create codex_ui/chrome.lua (tab bar + command bar)

**Files:**
- Create: `~/.config/wezterm/codex_ui/chrome.lua`

- [ ] **Step 1: Create chrome.lua with tab title formatter and command bar**

```lua
-- Tab bar and command bar rendering for the WezTerm config.
-- Implements WT-style tabs with process names and activity indicators,
-- plus a command bar with panel toggles, cwd, LLM badge, and clock.
local M = {}

function M.new(wezterm, schemes, shared)
  local api = {}
  local ui = schemes.ui

  local function process_label(process_name)
    if not process_name or process_name == '' then return 'shell' end
    local label = process_name:match('([^/\\]+)$') or process_name
    label = label:gsub('%.exe$', '')
    if label == 'pwsh' or label == 'powershell' then return 'PowerShell'
    elseif label == 'cmd' then return 'cmd' end
    return label
  end

  local function is_tui_process(name)
    local lower = tostring(name or ''):lower()
    for _, m in ipairs({ 'codex', 'claude', 'gemini', 'aider', 'fzf', 'nvim', 'vim', 'less', 'yazi', 'lazygit', 'gitui', 'btop', 'htop', 'k9s' }) do
      if lower:find(m, 1, true) then return true end
    end
    return false
  end

  -- Format tab title: process name, no index numbers, activity indicator
  function api.format_tab_title(tab, tabs, panes, config, hover, max_width)
    local process = process_label(tab.active_pane.foreground_process_name)
    local is_active = tab.is_active
    local tui_active = is_tui_process(process) and is_active

    local label = process
    if #label > (max_width - 4) then
      label = label:sub(1, max_width - 6) .. '..'
    end

    local bg = is_active and ui.surface_active or (hover and ui.surface_alt or ui.surface)
    local fg = is_active and ui.fg or ui.muted

    local elements = {
      { Background = { Color = bg } },
      { Foreground = { Color = fg } },
      { Attribute = { Intensity = is_active and 'Bold' or 'Normal' } },
    }

    -- Activity indicator
    if is_active then
      table.insert(elements, { Foreground = { Color = ui.accent } })
      table.insert(elements, { Text = ' \u{25CF} ' })
      table.insert(elements, { Foreground = { Color = fg } })
    else
      table.insert(elements, { Foreground = { Color = ui.muted } })
      table.insert(elements, { Text = ' \u{25CB} ' })
      table.insert(elements, { Foreground = { Color = fg } })
    end

    table.insert(elements, { Text = label })

    -- TUI badge
    if tui_active then
      table.insert(elements, { Foreground = { Color = '#4ade80' } })
      table.insert(elements, { Text = ' active' })
    end

    table.insert(elements, { Text = ' ' })
    return elements
  end

  -- Command bar (right status)
  function api.update_right_status(window, pane)
    local cwd_raw = shared.cwd_to_path(pane:get_current_working_dir())
    local cwd = cwd_raw and shared.shorten_home(cwd_raw, wezterm.home_dir) or nil

    local workspace = window:active_workspace()
    if workspace == 'default' then workspace = nil else workspace = 'WS ' .. workspace end

    local date = wezterm.strftime('%H:%M')
    local process = process_label(pane:get_foreground_process_name() or '')

    -- LLM badge
    local llm_badge = nil
    local state = wezterm.GLOBAL.llm_status or {}
    if state.agent and state.agent ~= '' then
      llm_badge = state.agent
    end

    local cells = {}
    local function push(bg, fg, text)
      if not text or text == '' then return end
      table.insert(cells, { Background = { Color = bg } })
      table.insert(cells, { Foreground = { Color = fg } })
      table.insert(cells, { Text = ' ' .. text .. ' ' })
    end

    push(ui.surface, ui.muted, workspace)
    push(ui.surface_alt, ui.fg, cwd)
    if llm_badge then
      push('#0d3320', '#4ade80', llm_badge)
    end
    push(ui.accent_strong, ui.bg, date)

    window:set_right_status(wezterm.format(cells))
  end

  api.is_tui_process = is_tui_process

  return api
end

return M
```

- [ ] **Step 2: Verify module loads**

Add a temporary line to `.wezterm.lua`:
```lua
local chrome_mod = require('codex_ui.chrome')
assert(type(chrome_mod.new) == 'function', 'chrome.new must be a function')
wezterm.log_info('chrome.lua loaded successfully')
```
Restart WezTerm, check log for "chrome.lua loaded successfully". Remove temp lines.

- [ ] **Step 3: Commit**

```bash
git add ~/.config/wezterm/codex_ui/chrome.lua
git commit -m "feat(wezterm): create codex_ui/chrome.lua with WT-style tab bar and command bar"
```

---

### Task 6a: Wire new modules into .wezterm.lua (incremental integration)

**Files:**
- Modify: `~/.wezterm.lua`

Wire chrome, palette, and schemes modules into the existing config alongside the old code. This lets us verify the new modules work before deleting the old code.

- [ ] **Step 1: Back up the current config**

```bash
cp ~/.wezterm.lua ~/.wezterm.lua.backup-$(date +%Y%m%d)
```

- [ ] **Step 2: Add module requires and config_context after the existing module loads (after line ~57)**

```lua
-- New modules (Phase 2)
local schemes = require('codex_ui.schemes')
local chrome = require('codex_ui.chrome').new(wezterm, schemes, shared)
local palette_mod = require('codex_ui.palette')

-- Config context for palette (defines the interface palette.lua expects)
local config_context = {
  llm_agents = llm_agents, -- the existing llm_agents table (line ~192)
  apply_ui = function(window, updates, message)
    -- Simplified: persist and apply config overrides
    local next_prefs = {}
    for k, v in pairs(ui_preferences) do next_prefs[k] = v end
    for k, v in pairs(updates) do next_prefs[k] = v end
    ui_preferences = next_prefs
    save_ui_preferences(next_prefs)
    -- Rebuild font if family/weight changed
    local overrides = window:get_config_overrides() or {}
    if updates.color_scheme then overrides.color_scheme = updates.color_scheme end
    if updates.font_size then overrides.font_size = updates.font_size end
    if updates.font_family then
      overrides.font = wezterm.font_with_fallback({
        { family = updates.font_family, weight = updates.font_weight or 'Regular' },
        { family = 'Cascadia Code', weight = 'Regular' },
        { family = 'Segoe UI Symbol' },
        { family = 'Consolas' },
      })
    end
    window:set_config_overrides(overrides)
    if message then
      window:toast_notification('WezTerm', message, nil, 2500)
    end
  end,
}
local palette = palette_mod.new(wezterm, act, schemes, ui_preferences)
```

- [ ] **Step 3: Replace the format-tab-title handler with chrome module delegation**

Find the `wezterm.on('format-tab-title', ...)` block (around line 3132-3161) and replace with:

```lua
wezterm.on('format-tab-title', function(tab, tabs, panes, cfg, hover, max_width)
  return chrome.format_tab_title(tab, tabs, panes, cfg, hover, max_width)
end)
```

- [ ] **Step 4: Replace the update-right-status handler with chrome module delegation**

Find the `wezterm.on('update-right-status', ...)` block (around line 3040-3091) and replace with:

```lua
wezterm.on('update-right-status', function(window, pane)
  chrome.update_right_status(window, pane)
end)
```

- [ ] **Step 5: Replace augment-command-palette with palette module**

Find the `wezterm.on('augment-command-palette', ...)` block (around line 2651-2723) and replace with:

```lua
wezterm.on('augment-command-palette', function(window, pane)
  return palette.build_palette(config_context)
end)
```

- [ ] **Step 6: Add show-profiles event handler**

```lua
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
```

- [ ] **Step 7: Test the delegated handlers**

Restart WezTerm. Verify:
1. Tab titles show process names with circle indicators (new chrome)
2. Right status bar shows cwd + clock (new chrome)
3. `Ctrl+Shift+P` command palette shows `[LLM]`, `[UI]`, `[TOOL]`, `[SHELL]`, `[NAV]` tagged entries
4. Old menus still work (they haven't been deleted yet)

- [ ] **Step 8: Commit**

```bash
git add ~/.wezterm.lua
git commit -m "feat(wezterm): wire chrome + palette modules into existing config

Delegates format-tab-title, update-right-status, and augment-command-palette
to new codex_ui modules. Old menu code still present for rollback safety."
```

---

### Task 6b: Delete old menu system and flatten keybindings

**Files:**
- Modify: `~/.wezterm.lua`

Now that the new modules are wired and verified, delete all the old InputSelector menu code, flatten keybindings, and clean up.

- [ ] **Step 1: Delete all show_*_selector and show_*_menu functions**

Remove these function definitions and all their supporting code (roughly lines 844-2574):
- `show_color_scheme_selector`, `show_font_family_selector`, `show_font_weight_selector`
- `show_font_size_selector`, `show_line_height_selector`, `show_cell_width_selector`
- `show_font_rendering_selector`, `show_typography_menu`
- `show_palette_preset_selector`, `show_palette_slot_selector`, `show_palette_lab`
- `show_opacity_selector`, `show_tab_style_selector`, `show_tab_width_selector`
- `show_titlebar_font_selector`, `show_tabs_frame_menu`
- `show_panel_launch_selector`, `show_panel_size_selector`
- `show_profile_selector`, `show_settings_hub`, `show_app_menu`
- `open_wireframes_panel`, `open_smart_workbench`, `open_readonly_panel`
- `launch_context_dock`, `launch_ui_controls_dock`, `launch_explorer_panel`
- `launch_watcher_panel`, `launch_editor_panel`, `launch_daemon_status_panel`
- `launch_placeholder_surface`, `recommend_workbench`, `selection_snapshot`
- All `register_window_event_handlers` calls for old events
- All `appearance_profiles`, `curated_scheme_names`, `font_family_presets` definitions (moved to schemes.lua)
- All `palette_slot_catalog`, `palette_theme_presets`, `base_ui_palette` (moved to schemes.lua)
- All `normalize_ui_preferences`, `managed_ui_differences`, `sync_window_ui`, `desired_ui_overrides`, `runtime_tui_ui_overrides`
- All `managed_ui_keys`, `merge_preferences`, `current_stored_preference`, `current_panel_size_preference`
- All `build_color_overrides`, `build_window_frame_from_preferences`, `palette_from_preferences`
- The UI selftest blocks
- The `custom_schemes` definition (moved to schemes.lua)
- The `ui` color table definition (moved to schemes.lua)

- [ ] **Step 2: Replace keybindings with the flat 3-tier system**

Replace the entire `config.keys = { ... }` block and all LLM/utility keybinding sections with:

```lua
-- ============================================================================
-- KEYBINDINGS - 3-tier system (WT-standard)
-- ============================================================================

local function split_auto(window, pane)
  local dims = pane:get_dimensions()
  local w = dims.pixel_width or dims.cols or 0
  local h = dims.pixel_height or dims.rows or 0
  if w >= h and w > 0 and h > 0 then
    window:perform_action(act.SplitHorizontal({ domain = 'CurrentPaneDomain' }), pane)
  else
    window:perform_action(act.SplitVertical({ domain = 'CurrentPaneDomain' }), pane)
  end
end

wezterm.on('split-auto', function(window, pane) split_auto(window, pane) end)

local function copy_or_send_ctrl_c(window, pane)
  local selection = window:get_selection_text_for_pane(pane)
  if selection and selection ~= '' then
    window:perform_action(act.CopyTo('Clipboard'), pane)
  else
    window:perform_action(act.SendKey({ key = 'c', mods = 'CTRL' }), pane)
  end
end

config.keys = {
  -- Tier 1: Instant access
  { key = 'Space', mods = 'CTRL', action = act.ActivateCommandPalette },
  { key = 'p', mods = 'CTRL|SHIFT', action = act.ActivateCommandPalette },
  -- Alt+1/2/3 panel toggles wired in Phase 3

  -- Tier 2: WT-standard
  { key = 'c', mods = 'CTRL', action = wezterm.action_callback(copy_or_send_ctrl_c) },
  { key = 'v', mods = 'CTRL', action = act.PasteFrom('Clipboard') },
  { key = 't', mods = 'CTRL|SHIFT', action = act.SpawnTab('CurrentPaneDomain') },
  { key = 'w', mods = 'CTRL|SHIFT', action = act.CloseCurrentTab({ confirm = true }) },
  { key = 'Tab', mods = 'CTRL', action = act.ActivateTabRelative(1) },
  { key = 'Tab', mods = 'CTRL|SHIFT', action = act.ActivateTabRelative(-1) },
  { key = 'f', mods = 'CTRL|SHIFT', action = act.Search({ CaseSensitiveString = '' }) },
  { key = 'd', mods = 'ALT|SHIFT', action = wezterm.action_callback(split_auto) },
  { key = 'PageUp', mods = 'SHIFT', action = act.ScrollByPage(-1) },
  { key = 'PageDown', mods = 'SHIFT', action = act.ScrollByPage(1) },
}
```

- [ ] **Step 3: Clean up the mouse bindings section**

Ensure mouse bindings match spec Section 3.5:
```lua
config.mouse_bindings = {
  { event = { Down = { streak = 1, button = 'Right' } }, mods = 'NONE', action = act.PasteFrom('Clipboard') },
  { event = { Up = { streak = 1, button = 'Left' } }, mods = 'CTRL', action = act.OpenLinkAtMouseCursor },
  { event = { Drag = { streak = 1, button = 'Left' } }, mods = 'CTRL', action = act.Nop },
}
```

- [ ] **Step 4: Apply color schemes from schemes module**

Replace the inline `custom_schemes` and `color_scheme` assignment with:
```lua
config.color_schemes = schemes.custom_schemes
config.color_scheme = pref('color_scheme', 'Codex PowerShell')
```

- [ ] **Step 5: Remove the dispatch.lua require (line ~41)**

Delete: `local dispatch = require('codex_ui.dispatch').new(wezterm, act)`
And all variables that came from it (`show_selector`, `show_dispatch_menu`, `show_launcher`, `register_window_event_handlers`, `command_palette_entry`).

- [ ] **Step 6: Test the stripped-down config**

Restart WezTerm. Verify:
1. WezTerm starts without errors
2. No old menus accessible (Alt+M, Alt+P, Ctrl+Shift+, should do nothing)
3. `Ctrl+Space` and `Ctrl+Shift+P` open palette with categorized entries
4. All Tier 2 keybindings work (Ctrl+C, Ctrl+V, Ctrl+Shift+T, Ctrl+Tab, etc.)
5. Right-click pastes, Ctrl+click opens links

Run: `grep -c "show_.*_selector\|show_.*_menu\|dispatch\.\|os.getenv" ~/.wezterm.lua`
Expected: 0

- [ ] **Step 7: Commit**

```bash
git add ~/.wezterm.lua
git commit -m "feat(wezterm): delete all nested menus, flatten to 3-tier keybindings

Removed: pseudo menubar, 4-level InputSelector trees, UI Studio menu,
all Ctrl+Alt+Shift LLM shortcuts, utility Alt+E/W/Ctrl+Alt+E bindings,
UI selftest blocks, all show_*_selector/show_*_menu functions (~1500 lines)."
```

---

### Task 6c: Final cleanup, delete dispatch.lua, verify line count

**Files:**
- Modify: `~/.wezterm.lua`
- Delete: `~/.config/wezterm/codex_ui/dispatch.lua`

- [ ] **Step 1: Delete dispatch.lua**

```bash
rm ~/.config/wezterm/codex_ui/dispatch.lua
```

- [ ] **Step 2: Final cleanup pass on .wezterm.lua**

Remove any orphaned variables, unused functions, or dead code left from the deletions. Ensure:
- No references to `dispatch` remain
- No references to deleted functions remain
- The `llm_features` flag table is simplified (just `pref()` calls)
- The LLM agent definitions are kept (used by palette.lua via config_context)
- `config.launch_menu` is still built (used by + dropdown / ShowLauncher)
- `alternate_buffer_wheel_scroll_speed = 3` is preserved
- `embedded-dev-config.lua` conditional load is preserved
- Window config (decorations, padding, scrollback, cursor, bell) is preserved

- [ ] **Step 3: Test the final config**

Restart WezTerm. Full verification:
1. WezTerm starts without errors
2. Text renders with DirectWrite (visually sharper)
3. Tab bar shows process names with activity indicators
4. Command bar shows cwd, clock, and LLM badge
5. `Ctrl+Space` opens categorized command palette
6. `Ctrl+Shift+P` opens palette (alias)
7. `Ctrl+Shift+T` opens new tab
8. `Ctrl+C` copies selection or sends interrupt
9. Right-click pastes
10. Launch menu entries appear via ShowLauncher
11. No errors in WezTerm debug log

Run: `wc -l ~/.wezterm.lua`
Expected: ~250-350 lines

Run: `grep -c "dispatch\.\|show_.*_selector\|show_.*_menu\|os.getenv" ~/.wezterm.lua`
Expected: 0

**Rollback:** If anything is broken, restore with: `git checkout HEAD~1 -- ~/.wezterm.lua ~/.config/wezterm/codex_ui/dispatch.lua`

- [ ] **Step 4: Commit**

```bash
git rm ~/.config/wezterm/codex_ui/dispatch.lua
git add ~/.wezterm.lua ~/.config/wezterm/codex_ui/
git commit -m "feat(wezterm): complete Phase 2 - dispatch.lua removed, bootstrap finalized

.wezterm.lua: 3250 -> ~300 lines. All logic in codex_ui/ modules.
dispatch.lua merged into palette.lua. Config is clean."
```

---

## Spec Items Deferred Beyond Phase 2

These spec requirements are intentionally deferred, not forgotten:

| Spec Item | Deferred To | Reason |
|-----------|-------------|--------|
| `+` dropdown with categorized submenu (Spec 2.1) | Phase 2 uses `ShowLauncherArgs` as approximation; custom dropdown requires Rust-side tab bar changes | Phase 4 |
| Tab right-click context menu (Rename, Duplicate, Move) | WezTerm's native tab context menu handles Close; custom entries need Rust event hooks | Phase 4 |
| Command bar left-zone panel toggle icons | Requires `set_left_status` + panel state tracking | Phase 3 |
| Click cwd to copy path | Requires Rust-side click-zone support in status bar | Phase 4 |
| Tab drag-to-reorder | Verify native fancy tab bar supports this; may work out of the box | Phase 2 (verify in Task 6c) |

---

## Phase 3: Panel System (Follow-Up)

> Phase 3 tasks are outlined here for planning. Execute after Phase 2 is stable.

### Task 7: Create codex_ui/panels.lua (toggle system)

**Files:**
- Create: `~/.config/wezterm/codex_ui/panels.lua`
- Modify: `~/.config/wezterm/codex_ui/chrome.lua` (add panel state indicators to command bar)
- Modify: `~/.wezterm.lua` (wire toggle events and Alt+1/2/3 bindings)

- [ ] **Step 1: Create panels.lua with toggle state tracking and split pane management**
- [ ] **Step 2: Add panel state indicators to chrome.lua command bar (highlighted icons)**
- [ ] **Step 3: Wire toggle events in .wezterm.lua (toggle-explorer, toggle-watcher, toggle-editor)**
- [ ] **Step 4: Add Alt+1/2/3 keybindings that emit toggle events**
- [ ] **Step 5: Add panel size persistence to ui-preferences.lua**
- [ ] **Step 6: Test: Alt+1 opens/closes explorer, icon highlights in command bar**
- [ ] **Step 7: Commit**

### Task 8: Settings feature tab

- [ ] **Step 1: Add Settings entry to + dropdown and palette**
- [ ] **Step 2: Create a simple settings info panel (PowerShell pane showing current config values)**
- [ ] **Step 3: Test and commit**

---

## Phase 4: Rust-Side Investment (Separate Plan)

> Phase 4 involves `~/wezterm/` fork crate work. Coordinate with the other Claude agent via agent-bus. This should be a separate implementation plan once Phases 1-3 are stable.

Scope:
- Build `wezterm-fs-explorer` and `wezterm-watch` to `~/bin/`
- DirectWrite rendering param tuning in `wezterm-gui` (if needed)
- IPC daemon activation
- Module framework for extensible feature tabs
- Performance: move tab title and status bar rendering to Rust
