-- ui-preferences.lua
-- WezTerm UI preferences -- single source of truth for all appearance,
-- rendering, and feature-flag settings.
--
-- Replaces the following environment variables (no longer read):
--   WEZTERM_USE_WEBGPU              -> front_end
--   WEZTERM_GPU_VENDOR              -> gpu_vendor
--   WEZTERM_MAX_FPS                 -> max_fps
--   WEZTERM_USE_ACRYLIC             -> backdrop
--   WEZTERM_DISABLE_CUSTOM_STATUS   -> custom_status_bar (inverted)
--   WEZTERM_DISABLE_CUSTOM_TAB_TITLE -> custom_tab_titles (inverted)
--   WEZTERM_UI_SELFTEST             -> removed entirely
--   WEZTERM_UI_SELFTEST_APPLY       -> removed entirely
--
-- Usage: loaded automatically by prefs_io.load() on startup.
-- Edit this file to change settings; restart WezTerm to apply.
-- Keys are accessed via pref('key_name', fallback_value).

return {
  -- Rendering ----------------------------------------------------------------
  -- 'WebGpu' uses DirectWrite on Windows (proper ClearType).
  -- 'OpenGL' falls back to FreeType. 'Software' for headless/CI.
  -- WebGpu uses wgpu (Dx12/Vulkan) + DirectWrite for text rendering.
  -- Requires ANGLE DLLs in same dir as wezterm.exe.
  front_end = 'WebGpu',

  -- GPU vendor hint: 'auto', 'nvidia', 'intel'. Auto prefers discrete GPU.
  gpu_vendor = 'auto',

  -- Max frames per second. 165 matches a 165 Hz display.
  max_fps = 165,

  -- Windows compositor backdrop: 'Acrylic', 'Mica', 'Tabbed', 'None'
  backdrop = 'Acrylic',

  -- Appearance ---------------------------------------------------------------
  -- Color scheme name (must match a scheme in color_schemes table or builtins).
  color_scheme = 'Codex PowerShell',

  -- Primary font family. Falls back through Cascadia Code -> Segoe UI Symbol -> Consolas.
  font_family = 'Cascadia Mono',

  -- Font weight: 'Regular', 'DemiLight', 'Medium', 'DemiBold', 'Bold'
  font_weight = 'Regular',

  -- Font size in points.
  font_size = 12.5,

  -- Vertical spacing multiplier (1.0 = default cell height).
  line_height = 1.0,

  -- Horizontal spacing multiplier (1.0 = default cell width).
  cell_width = 1.0,

  -- Window background opacity [0.0 = transparent, 1.0 = opaque].
  window_background_opacity = 0.98,

  -- Tab bar ------------------------------------------------------------------
  -- true = styled GPU-rendered tab bar, false = compact retro tab bar.
  use_fancy_tab_bar = true,

  -- Maximum width of a single tab title in cells.
  tab_max_width = 42,

  -- Feature flags ------------------------------------------------------------
  -- Custom right-status bar (cwd, clock, LLM badge).
  custom_status_bar = true,

  -- Per-process tab titles with activity indicators.
  custom_tab_titles = true,

  -- LLM agent integration (launch menu entries, palette entries, status badge).
  llm_agents = true,

  -- Prompt capture for shell-integration / selection forwarding.
  llm_prompt_capture = true,

  -- Embedded development tooling (merged from embedded-dev-config.lua).
  embedded_dev = true,
}
