# WezTerm UI Studio Wireframes

## Design Inputs

- Primary shell: `pwsh.exe`
- Primary host: Windows desktop with DPI scaling
- Interaction model available in WezTerm Lua:
  - current pane working directory
  - foreground process name
  - current selection text
  - split panes, tabs, overlays, launchers, prompts
- Custom tool surface available from `~/wezterm`:
  - `wezterm-fs-explorer`
  - `wezterm-watch`
  - `wezterm-utils-daemon`
  - placeholder hooks for `wezterm-fs-utils` and `wezterm-module-framework`

## Layout Principles

- Keep the terminal as the hero surface; configuration is a controlled overlay, not a separate app chrome.
- Use a compact left-hand navigation model and larger preview-oriented detail panes.
- Prefer curated presets plus explicit overrides over free-form configuration sprawl.
- Reflect process context in recommendations so PowerShell, git repos, and agent workflows get different launch suggestions.

## Wireframe: UI Studio

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ WezTerm UI Studio                                                         │
├───────────────┬────────────────────────────────────────────────────────────┤
│ Appearance    │ Profile: PowerShell Studio                                │
│ Typography    │ Scheme: Codex PowerShell                                  │
│ Palette Lab   │ Font: Cascadia Mono 12.5 pt                               │
│ Tabs + Frame  │ Line Height: 1.15   Cell Width: 1.00                      │
│ Panels        │ Opacity: 98%      Tab Width: 42                           │
│ Workbench     │                                                            │
│ Diagnostics   │ Preview Tokens                                             │
│ Reset         │  background     foreground     active tab     accent       │
│               │  tab bar bg     inactive tab   selection      border       │
│               │                                                            │
│               │ [Open Palette Lab] [Open Workbench] [Reset to Defaults]    │
└───────────────┴────────────────────────────────────────────────────────────┘
```

## Wireframe: Palette Lab

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Palette Lab                                                               │
├───────────────────────┬────────────────────────────────────────────────────┤
│ Theme Base            │ Live Preview                                       │
│  Codex PowerShell     │                                                    │
│  Codex Ember          │  PS C:\Users\david> git status                     │
│  Codex Graphite       │  On branch main                                    │
│  Dracula              │  modified: .wezterm.lua                            │
│                       │                                                    │
│ Slots                 │  Tabs:  PowerShell | Explorer | Watch              │
│  background           │                                                    │
│  foreground           │                                                    │
│  tab bar background   │                                                    │
│  active tab bg        │ HEX / Preset                                       │
│  active tab fg        │  #0b1220   [Prompt Hex] [Apply Preset]             │
│  inactive tab fg      │                                                    │
│  selection bg         │                                                    │
│  accent cursor        │                                                    │
└───────────────────────┴────────────────────────────────────────────────────┘
```

## Wireframe: Workbench

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Smart Workbench                                                           │
├────────────────────────────────────────────────────────────────────────────┤
│ Main Terminal (pwsh / active task)              │ Context Dock            │
│                                                  │-------------------------│
│                                                  │ Process: pwsh.exe       │
│                                                  │ Repo: ~/wezterm         │
│                                                  │ Selection: 3 lines      │
│                                                  │ Suggest: Explorer+Watch │
├──────────────────────────────────────────────────┼─────────────────────────┤
│ Watch Panel (bottom, adjustable)                │ Explorer Panel          │
│ git-aware change stream                          │ file tree / preview     │
└──────────────────────────────────────────────────┴─────────────────────────┘
```

## Wireframe: Utility Dock Menu

```text
Menu
├─ File
├─ Edit
├─ View
├─ Settings
└─ Panels
   ├─ Explorer Right
   ├─ Watch Bottom
   ├─ Editor Right
   ├─ Context Dock
   ├─ Smart Workbench
   ├─ Daemon Status
   ├─ FS Utils Placeholder
   └─ Module Framework Placeholder
```

## Process-Driven Context Model

The design should react to a lightweight context snapshot gathered from the active pane:

- `foreground_process_name`
- `current_working_dir`
- selected text length and preview
- optional repo detection by path convention

This is not a full transcript-capture system. Instead, it is a context dock that informs panel recommendations and launch defaults:

- `pwsh` in a repo -> recommend `Explorer + Watch + Context Dock`
- editor process -> recommend `Watch + Context Dock`
- agent/LLM shell -> recommend `Context Dock + Explorer`

## Initial Implementation Map

- `Appearance`: profiles, scheme selection, opacity presets
- `Typography`: font family, font size, line height, cell width
- `Palette Lab`: curated slot editing with preset colors and hex prompt
- `Tabs + Frame`: tab style, tab width, titlebar font size
- `Panels`: panel sizes and direct launchers
- `Workbench`: multi-panel layouts driven by current process/cwd
- `Diagnostics`: renderer, DPI, font metrics, current effective settings

