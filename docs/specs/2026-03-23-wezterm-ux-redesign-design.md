# WezTerm UX/UI Redesign

> Design spec for overhauling the WezTerm configuration from a 3250-line monolith with nested menus to a Windows Terminal-inspired, mouse-first terminal with proper text rendering and modular architecture.

**Date:** 2026-03-23
**Status:** Approved
**Scope:** `~/.wezterm.lua`, `~/.config/wezterm/`, `~/wezterm/` fork

---

## Problem Statement

The current `.wezterm.lua` configuration is a 3250-line monolithic file that suffers from:

1. **Broken text rendering** -- FreeType with HorizontalLcd subpixel rendering looks like "ClearType without corrections" on Windows. The Harfbuzz rasterizer and DemiLight font weight compound the problem.
2. **Deeply nested menus** -- UI Studio requires up to 4 levels of InputSelector navigation to reach simple settings (e.g., change a font).
3. **Flat, generic visual design** -- tabs and chrome have no personality despite 3 custom color schemes.
4. **Clunky panel workflow** -- opening companion panels (explorer, watcher, editor) requires menu navigation with no toggle or state memory.
5. **Keybinding sprawl** -- 20+ shortcuts including 7 `Ctrl+Alt+Shift+Letter` combos for LLM agents that are impossible to remember.
6. **Env var pollution** -- 8 environment variables gate rendering, GPU, and feature flags instead of using config files.

## Design Direction

**Command Bar Terminal** -- a Windows Terminal-inspired interface with:
- WT-style tab bar with shell tabs + feature tabs
- Interactive command bar at the bottom with clickable panel toggles
- Mouse-first interaction model (every action has a click path)
- Flat command palette (`Ctrl+Space`) replacing all nested menus
- Rust-first processing where the `~/wezterm/` fork can support it

**Reference model:** Windows Terminal with add-on tab control.

## User Profile

- Primary workflow: single-pane TUI session (claude, codex) with ephemeral companion panels
- Mouse-first interaction style; rarely uses keybindings beyond copy/cut/paste
- Wants maximum graphics performance (WebGpu, GPU acceleration, high FPS)
- Configuration via files only, never env vars

---

## Section 1: Rendering Foundation

### 1.1 Front-End Switch

| Setting | Current | Target |
|---------|---------|--------|
| `front_end` | `'OpenGL'` (gated by env var) | `'WebGpu'` (default) |
| `font_rasterizer` | `'Harfbuzz'` | Removed (let WebGpu/DirectWrite handle it) |
| `freetype_load_target` | `'HorizontalLcd'` | Removed |
| `freetype_render_target` | `'HorizontalLcd'` | Removed |
| `webgpu_power_preference` | Not set (env-gated) | `'HighPerformance'` |

WebGpu on Windows uses DirectWrite for text rendering, which provides proper ClearType with gamma correction and stem darkening. The FreeType overrides are only needed for the OpenGL fallback path.

### 1.2 Fallback Strategy

If WebGpu initialization fails (RDP, VM, unsupported GPU):
1. Log the failure reason via `wezterm.log_warn`
2. Fall back to `front_end = 'OpenGL'` with FreeType `HorizontalLcd/HorizontalLcd`
3. No user intervention required -- automatic, silent degradation

**Implementation note:** WezTerm's `front_end` setting is static at config load time -- there is no runtime try/catch. The fallback is implemented by checking `wezterm.gui.enumerate_gpus()` at config time: if no suitable WebGpu adapter is found, set `front_end = 'OpenGL'` and apply FreeType overrides. Phase 4 can add Rust-side startup detection if needed.

### 1.3 GPU Adapter Selection

Keep the existing `gpu_score()` logic but enable it by default (no env var gate):
- Discrete NVIDIA > Intel Arc > Integrated GPU
- Filter out IddCx, Luminon, and Basic Render virtual adapters
- `webgpu_power_preference = 'HighPerformance'` for discrete GPUs
- `webgpu_power_preference = 'LowPower'` for Intel integrated

### 1.4 Font Stack

```lua
config.font = wezterm.font_with_fallback({
  { family = 'Cascadia Mono', weight = 'Regular' },
  { family = 'Cascadia Code', weight = 'Regular' },
  { family = 'Segoe UI Symbol' },
  { family = 'Consolas' },
})
config.font_size = 12.5
config.line_height = 1.0
config.cell_width = 1.0
config.harfbuzz_features = { 'calt=1', 'clig=1', 'liga=1' }
config.use_cap_height_to_scale_fallback_fonts = true
```

**Delta from current:** `Segoe UI Symbol` added as fallback for Unicode symbol/emoji coverage (not present in current config). `weight` key removed from Consolas entry (unnecessary for fallback). Weight changes from `DemiLight` to `Regular` -- DemiLight is too thin under FreeType and unnecessary under DirectWrite.

### 1.5 Performance

- `max_fps = 165` (kept, matches 165Hz display)
- `animation_fps` matched to display refresh
- `win32_system_backdrop = 'Acrylic'` enabled from config (not env var)
- `alternate_buffer_wheel_scroll_speed = 3` (kept)

### 1.6 Rust-Side Opportunity

If DirectWrite quality still isn't right after the config switch, the `wezterm-gui` crate in the fork supports tuning DWrite rendering parameters (gamma, enhanced contrast, cleartype level, rendering mode). This is Phase 4 work.

---

## Section 2: Tab Bar & Chrome

### 2.1 Tab Bar Layout

```
[shell tabs...] | [feature tabs...] [spacer] [+ dropdown]
```

**Shell tabs:** PowerShell, cmd, WSL, Git Bash, LLM agents. Show:
- Process name (no index numbers -- deliberate removal; `Ctrl+1..9` tab switching not included in the simplified keybinding set)
- Activity indicator (filled circle = active, hollow = idle)
- Active state badge for TUI processes (e.g., "active" for claude)
- Active tab has bottom accent border (2px, accent color)

**Feature tabs:** Explorer, Watch, Edit, Settings. Visually distinct:
- Separated from shell tabs by a thin vertical divider
- Icon-led labels (folder icon, eye icon, pencil icon, gear icon)
- Slightly different background when active
- Open from the + dropdown or command bar toggles

**+ dropdown button:**
- Click `+` directly: new shell tab (default shell)
- Click the dropdown arrow: full categorized menu
  - Shells: PowerShell, cmd, WSL, Git Bash, MSYS2, VS Dev Shell
  - LLM Agents: Claude, Codex, Gemini, Copilot, Ollama, LM Studio
  - Tools: Explorer, Watcher, Editor, Settings
  - Dev Environments: embedded dev entries (from embedded-dev-config.lua)

### 2.2 Tab Interactions (Mouse)

| Action | Trigger |
|--------|---------|
| Switch tab | Click |
| Close tab | Middle-click |
| Context menu | Right-click |
| Reorder | Drag |
| New default tab | Click + |
| Full launcher | Click + dropdown arrow |

Context menu entries: Rename, Duplicate, Move Left/Right, Close, Close Others.

### 2.3 Command Bar (Bottom)

Single-row, always visible. Three zones:

**Left zone -- Panel Toggles:**
- Clickable icons: Explorer (folder), Watch (eye), Edit (pencil)
- Highlighted (inverted colors) when the corresponding companion panel is open
- Click toggles the panel (open/close split pane)
- Keyboard equivalent: `Alt+1`, `Alt+2`, `Alt+3`

**Center zone -- Context:**
- Current working directory (shortened with `~`)
- Workspace name if not "default"
- Click cwd to copy path to clipboard (requires fork's Rust-side click-zone support; best-effort in Phase 2, full implementation in Phase 4)

**Right zone -- Status:**
- Active LLM agent badge (green pill, clickable to switch agent)
- Clock (HH:MM)
- Right-click the bar for a quick settings menu

### 2.4 Color Schemes

Keep the three custom schemes:
- **Codex PowerShell** -- cool blue, default
- **Codex Ember** -- warm orange, for presentations
- **Codex Graphite** -- neutral, for low-glare

Keep the four palette presets (studio, ember, graphite, high_contrast). These become selectable from the command palette instead of nested menus.

---

## Section 3: Keybinding Simplification

### 3.1 Tier 1: Instant Access

| Key | Action |
|-----|--------|
| `Ctrl+Space` | Command palette (universal search for all actions) |
| `Ctrl+Shift+P` | Command palette (alias, WT/VS Code convention) |
| `Alt+1` | Toggle Explorer companion panel |
| `Alt+2` | Toggle Watcher companion panel |
| `Alt+3` | Toggle Editor companion panel |

### 3.2 Tier 2: Standard Terminal (WT-Compatible)

| Key | Action |
|-----|--------|
| `Ctrl+C` | Copy selection or send interrupt |
| `Ctrl+V` | Paste from clipboard |
| `Ctrl+Shift+T` | New tab |
| `Ctrl+Shift+W` | Close tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+Shift+F` | Search scrollback |
| `Alt+Shift+D` | Auto split pane |
| `Shift+PageUp/Down` | Scroll by page |

### 3.3 Tier 3: Command Palette

Everything else accessed via `Ctrl+Space` then typing. Categorized with color-coded tags:

| Category | Tag Color | Contents |
|----------|-----------|----------|
| LLM | Green | Claude, Codex, Gemini, Copilot, Ollama, LM Studio, Docker, prompt capture |
| UI | Blue | Color scheme, font family, font size, profiles, palette lab, opacity, reset |
| TOOL | Amber | Explorer, Watcher, Editor, Diagnostics, Build utilities, IPC status |
| SHELL | Purple | PowerShell, cmd, WSL, Git Bash, MSYS2, VS Dev Shell, embedded envs |
| NAV | Red | Switch workspace, switch tab, split pane, close tab, launcher, search |

Palette hint bar at bottom: `Enter` = open, `Shift+Enter` = open in split, `Esc` = close.

### 3.4 Removed Bindings

All of these move to the command palette or + dropdown:
- `Ctrl+Alt+Shift+G/X/C/P/O/L/D` (7 LLM agent keys)
- `Ctrl+Alt+Shift+B` (build utils)
- `Ctrl+Alt+Shift+A` (AI design doc)
- `Ctrl+Alt+Shift+Enter` (prompt capture)
- `Alt+E` / `Alt+Shift+E` (explorer split/tab -- superseded by `Alt+1`)
- `Alt+W` / `Alt+Shift+W` (watcher split/tab -- superseded by `Alt+2`)
- `Ctrl+Alt+E` / `Ctrl+Alt+Shift+E` (editor split/tab -- superseded by `Alt+3`)
- `Alt+M` (pseudo menubar)
- `Alt+P` (panels menu)
- `Ctrl+Shift+,` (UI Studio)
- `Ctrl+Shift+L` (launcher -- absorbed into + dropdown and palette)

### 3.5 Mouse Bindings

| Action | Trigger |
|--------|---------|
| Select text | Left-drag |
| Paste | Right-click |
| Open link | Ctrl+click |
| Select word | Double-click |
| Select line | Triple-click |
| Scrollback | Scroll wheel |

---

## Section 4: Configuration Architecture

### 4.1 Env Var Elimination

All environment variables replaced with `ui-preferences.lua` entries:

| Env Var (removed) | Config Key |
|--------------------|------------|
| `WEZTERM_USE_WEBGPU` | `front_end = 'WebGpu'` |
| `WEZTERM_GPU_VENDOR` | `gpu_vendor = 'auto'` |
| `WEZTERM_MAX_FPS` | `max_fps = 165` |
| `WEZTERM_USE_ACRYLIC` | `backdrop = 'Acrylic'` |
| `WEZTERM_UI_SELFTEST` | Removed entirely |
| `WEZTERM_UI_SELFTEST_APPLY` | Removed entirely |
| `WEZTERM_DISABLE_CUSTOM_STATUS` | `custom_status_bar = true` |
| `WEZTERM_DISABLE_CUSTOM_TAB_TITLE` | `custom_tab_titles = true` |

**Semantic inversion note:** The env vars use a disable-by-default pattern (`DISABLE_X=1` turns off). The replacement config keys use enable-by-default (`true` = show custom UI). Implementers: the logic inverts.

### 4.2 File Structure

```
~/.wezterm.lua                          (~300 lines, down from 3250)
  Bootstrap, load modules, wire events

~/.config/wezterm/
  ui-preferences.lua                    All user settings (rendering, appearance, features)
  codex_ui/
    chrome.lua                          Tab bar + command bar rendering
    palette.lua                         Command palette entries + categories (absorbs dispatch.lua helpers)
    panels.lua                          Companion panel toggle + state persistence
    schemes.lua                         Color schemes + palette presets
    shared.lua                          Utilities (kept, trimmed)
    prefs.lua                           Persistence helpers (kept)
    dispatch.lua                        REMOVED -- InputSelector/PromptInputLine helpers merged into palette.lua
  wezterm-utils/                        (kept -- init.lua, launcher, state, events, config, ipc)
  embedded-dev-config.lua               (kept -- entries merged into + dropdown)
```

### 4.3 .wezterm.lua Responsibilities (After Redesign)

1. Load `ui-preferences.lua`
2. Apply rendering settings (WebGpu, GPU selection, font stack)
3. Load `codex_ui` modules (chrome, palette, panels, schemes)
4. Wire event handlers (`format-tab-title`, `update-right-status`, `augment-command-palette`)
5. Set keybindings (Tier 1 + Tier 2 only)
6. Set mouse bindings
7. Load `embedded-dev-config.lua` if present
8. Return config

Everything else lives in the modules.

---

## Section 5: Panel System

### 5.1 Companion Panels

Three toggleable companion panels that split alongside the active pane:

| Panel | Direction | Default Size | Tool |
|-------|-----------|-------------|------|
| Explorer | Right | 35% | `wezterm-fs-explorer` (Rust TUI) or placeholder |
| Watcher | Bottom | 26% | `wezterm-watch` (Rust TUI) or placeholder |
| Editor | Right | 35% | `uv run python -m wedit` or placeholder |

**Placeholder behavior:** When a Rust TUI binary is not installed, the panel opens a PowerShell pane showing the expected binary path, repo location, and build instructions. The command bar icon is dimmed (not hidden) to indicate the tool is available but not installed.

### 5.2 Toggle Behavior

- Click command bar icon or press `Alt+1/2/3` to toggle
- If panel is closed: open a split pane with the tool
- If panel is open: close the split pane
- Panel icon in command bar shows highlighted state when open

### 5.3 State Persistence

- Panel sizes saved to `ui-preferences.lua` when dragged
- Which panels are open saved per-workspace
- Restored on workspace switch

### 5.4 Settings Feature Tab

The Settings feature tab replaces the entire UI Studio menu tree. Opens as a tab (not a split) showing a ratatui-style settings TUI, or initially a simpler PowerShell-rendered info panel with shortcuts to the command palette settings entries.

---

## Section 6: Implementation Roadmap

### Phase 1: Rendering + Config Cleanup (Config changes only)

**Changes to `.wezterm.lua`:**
- Switch `front_end` to `'WebGpu'` by default with automatic OpenGL fallback
- Enable GPU adapter selection by default (remove env var gate)
- Update font stack: Regular weight, remove FreeType overrides
- Move all 8 env var-gated features to `ui-preferences.lua`
- Enable Acrylic backdrop from config

**No Lua module changes. No Rust changes. Immediately shippable.**

### Phase 2: Chrome Overhaul (Lua rewrite)

**New modules:**
- `codex_ui/chrome.lua` -- tab bar formatting (shell tabs + feature tabs + separator + dropdown indicator)
- `codex_ui/palette.lua` -- categorized command palette entries with tags

**Rewrite in `.wezterm.lua`:**
- Replace `format-tab-title` handler with chrome module
- Replace `update-right-status` handler with command bar renderer
- Replace `augment-command-palette` handler with categorized palette
- Flatten keybindings to Tier 1 + Tier 2
- Add mouse bindings for tab bar and command bar interactions
- Remove: pseudo menubar, InputSelector trees, UI Studio menu, all `Ctrl+Alt+Shift` bindings

**Delete from `.wezterm.lua`:**
- `show_settings_hub`, `show_app_menu`, and all nested `show_*_selector` functions
- `show_typography_menu`, `show_palette_lab`, `show_tabs_frame_menu`
- `show_panel_launch_selector`, `show_panel_size_selector`
- All appearance profile application logic (moves to palette.lua)

### Phase 3: Panel System (Lua + state)

**New module:**
- `codex_ui/panels.lua` -- toggle logic, state tracking, persistence

**Changes:**
- Command bar icons trigger panel toggle callbacks
- Panel state saved/restored per workspace in `ui-preferences.lua`
- Settings feature tab implementation (initially simple, upgradeable to ratatui)

### Phase 4: Rust-Side Investment (Fork crate work)

**In `~/wezterm/`:**
- Tune DirectWrite rendering params in `wezterm-gui` if needed
- Build `wezterm-fs-explorer` and `wezterm-watch`, install to `~/bin/`
- Activate IPC daemon for panel state sync
- Evaluate moving tab title and status bar rendering to Rust via the module framework
- Performance profiling of Lua event handlers vs Rust alternatives

---

## Decisions Log

| Decision | Rationale |
|----------|-----------|
| WebGpu default, not gated | Performance + DirectWrite text quality. Fallback handles edge cases. |
| Command Bar over Ghost | User is mouse-first; needs visible, clickable UI elements. |
| Feature tabs alongside shell tabs | Windows Terminal reference model; tools as first-class citizens. |
| Ctrl+Space for palette | Universal, one shortcut to rule them all. Discoverable. |
| Alt+1/2/3 for panels | Memorable, fast, single-hand. |
| No env vars | User preference. Config files are inspectable, versionable, reloadable. |
| 4-phase roadmap | Each phase is independently shippable. Phase 1 fixes worst pain immediately. |
| Rust-first for hot paths | Long-term investment in fork. Lua for glue, Rust for performance. |
