# WezTerm Config Architecture

This config is organized as a thin entrypoint plus a few small support modules.

## Files

- `C:\Users\david\.wezterm.lua`
  - Main orchestration layer
  - Defines visual policy, launch menus, keybindings, panels, diagnostics, and event handlers
- `C:\Users\david\.config\wezterm\codex_ui\shared.lua`
  - Shared filesystem, path, merge, serialization, and color helpers
- `C:\Users\david\.config\wezterm\codex_ui\prefs.lua`
  - UI preference load/save helpers for `ui-preferences.lua`
- `C:\Users\david\.config\wezterm\codex_ui\dispatch.lua`
  - `InputSelector`, `PromptInputLine`, launcher, and command palette helpers

## Runtime Model

1. `.wezterm.lua` loads the local modules from `~/.config/wezterm/codex_ui`.
2. Base config values are defined in `.wezterm.lua`.
3. Saved UI preferences are loaded from:
   - `C:\Users\david\.config\wezterm\ui-preferences.lua`
4. Preferences are normalized into real WezTerm config keys.
5. Per-window changes are applied through `window:set_config_overrides(...)`.
6. Some values, such as `font_weight`, are stored preferences only and are used to synthesize other real config keys like `font`.

## Important Constraints

- Only real WezTerm config keys should go into `window:set_config_overrides(...)`.
- Synthetic preference values should be converted first.
- TUI-heavy apps are stabilized at runtime by forcing safer geometry and line placement.

## Local WezTerm Doc References

- Config files:
  - `C:\Users\david\wezterm\docs\config\files.md`
- Font configuration:
  - `C:\Users\david\wezterm\docs\config\fonts.md`
- Font fallback scaling:
  - `C:\Users\david\wezterm\docs\config\lua\config\use_cap_height_to_scale_fallback_fonts.md`
  - `C:\Users\david\wezterm\docs\config\lua\wezterm\font_with_fallback.md`
- Font rasterization:
  - `C:\Users\david\wezterm\docs\config\lua\config\freetype_load_target.md`
  - `C:\Users\david\wezterm\docs\config\lua\config\freetype_render_target.md`
- Dynamic overrides:
  - `C:\Users\david\wezterm\docs\config\lua\window\get_config_overrides.md`
  - `C:\Users\david\wezterm\docs\config\lua\window\set_config_overrides.md`
- Interactive menus:
  - `C:\Users\david\wezterm\docs\config\lua\keyassignment\InputSelector.md`
  - `C:\Users\david\wezterm\docs\config\lua\keyassignment\PromptInputLine.md`
  - `C:\Users\david\wezterm\docs\config\lua\keyassignment\ShowLauncherArgs.md`
- Command palette augmentation:
  - `C:\Users\david\wezterm\docs\config\lua\window-events\augment-command-palette.md`
