-- codex_ui/schemes.lua
-- Color schemes, palette presets, font presets, and appearance profiles.
-- Part of the WezTerm UX/UI Redesign (Task 3).

local M = {}

-- ---------------------------------------------------------------------------
-- Core UI palette
-- ---------------------------------------------------------------------------
M.ui = {
  bg             = '#0b1220',
  surface        = '#111827',
  surface_alt    = '#172033',
  surface_active = '#1f2a40',
  border         = '#2a3a57',
  fg             = '#f5f7fb',
  muted          = '#b6c2d1',
  accent         = '#7dd3fc',
  accent_strong  = '#38bdf8',
  selection_bg   = '#1d4ed8',
  selection_fg   = '#f8fafc',
}

-- ---------------------------------------------------------------------------
-- Custom color schemes
-- ---------------------------------------------------------------------------
M.custom_schemes = {

  ['Codex PowerShell'] = {
    foreground    = '#f5f7fb',
    background    = '#0b1220',
    cursor_bg     = '#7dd3fc',
    cursor_fg     = '#0b1220',
    cursor_border = '#7dd3fc',
    selection_bg  = '#1d4ed8',
    selection_fg  = '#f8fafc',
    scrollbar_thumb  = '#314158',
    split            = '#2a3a57',
    compose_cursor   = '#f59e0b',

    ansi = {
      '#1b2333', -- black
      '#f87171', -- red
      '#4ade80', -- green
      '#facc15', -- yellow
      '#60a5fa', -- blue
      '#c084fc', -- magenta
      '#22d3ee', -- cyan
      '#cbd5e1', -- white
    },
    brights = {
      '#334155', -- bright black
      '#fca5a5', -- bright red
      '#86efac', -- bright green
      '#fde047', -- bright yellow
      '#93c5fd', -- bright blue
      '#d8b4fe', -- bright magenta
      '#67e8f9', -- bright cyan
      '#ffffff', -- bright white
    },

    tab_bar = {
      active_tab = {
        bg_color  = '#1f2a40',
        fg_color  = '#f5f7fb',
        intensity = 'Bold',
        italic    = false,
      },
      inactive_tab = {
        bg_color = '#111827',
        fg_color = '#b6c2d1',
      },
      inactive_tab_hover = {
        bg_color  = '#172033',
        fg_color  = '#f5f7fb',
        italic    = false,
      },
      new_tab = {
        bg_color = '#111827',
        fg_color = '#7dd3fc',
      },
      new_tab_hover = {
        bg_color  = '#1f2a40',
        fg_color  = '#38bdf8',
        italic    = false,
      },
    },
  },

  ['Codex Ember'] = {
    foreground   = '#fff7ed',
    background   = '#17120f',
    cursor_bg    = '#fb923c',
    cursor_fg    = '#17120f',
    selection_bg = '#9a3412',
    selection_fg = '#fff7ed',

    ansi = {
      '#221814', -- black
      '#f87171', -- red
      '#86efac', -- green
      '#fbbf24', -- yellow
      '#fb923c', -- blue
      '#f0abfc', -- magenta
      '#2dd4bf', -- cyan
      '#e7d8cf', -- white
    },
    brights = {
      '#4b2f26', -- bright black
      '#fca5a5', -- bright red
      '#bbf7d0', -- bright green
      '#fde68a', -- bright yellow
      '#fdba74', -- bright blue
      '#f5d0fe', -- bright magenta
      '#99f6e4', -- bright cyan
      '#fff7ed', -- bright white
    },
  },

  ['Codex Graphite'] = {
    foreground   = '#f8fafc',
    background   = '#111318',
    cursor_bg    = '#a5b4fc',
    cursor_fg    = '#111318',
    selection_bg = '#334155',
    selection_fg = '#f8fafc',

    ansi = {
      '#1f2937', -- black
      '#f87171', -- red
      '#34d399', -- green
      '#facc15', -- yellow
      '#818cf8', -- blue
      '#c084fc', -- magenta
      '#22d3ee', -- cyan
      '#cbd5e1', -- white
    },
    brights = {
      '#475569', -- bright black
      '#fca5a5', -- bright red
      '#6ee7b7', -- bright green
      '#fde047', -- bright yellow
      '#a5b4fc', -- bright blue
      '#d8b4fe', -- bright magenta
      '#67e8f9', -- bright cyan
      '#ffffff', -- bright white
    },
  },
}

-- ---------------------------------------------------------------------------
-- Curated built-in + custom scheme names for scheme cycling / picker
-- ---------------------------------------------------------------------------
M.curated_scheme_names = {
  'Codex PowerShell',
  'Codex Ember',
  'Codex Graphite',
  'OneHalfDark',
  'Builtin Solarized Dark',
  'Dracula',
  'Gruvbox Dark',
  'Campbell',
}

-- ---------------------------------------------------------------------------
-- Palette presets
-- Each preset drives the tab-bar and selection theming independent of the
-- active color scheme, so they can be mixed or matched freely.
-- ---------------------------------------------------------------------------
M.palette_presets = {

  studio = {
    background       = M.ui.bg,
    foreground       = M.ui.fg,
    tab_bar_bg       = M.ui.surface,
    active_tab_bg    = M.ui.surface_active,
    active_tab_fg    = M.ui.fg,
    inactive_tab_bg  = M.ui.surface,
    inactive_tab_fg  = M.ui.muted,
    selection_bg     = M.ui.selection_bg,
    selection_fg     = M.ui.selection_fg,
    accent_cursor    = M.ui.accent,
    border           = M.ui.border,
  },

  ember = {
    background       = '#17120f',
    foreground       = '#fff7ed',
    tab_bar_bg       = '#120f0d',
    active_tab_bg    = '#7c2d12',
    active_tab_fg    = '#fff7ed',
    inactive_tab_bg  = '#2a211b',
    inactive_tab_fg  = '#fed7aa',
    selection_bg     = '#9a3412',
    selection_fg     = '#fff7ed',
    accent_cursor    = '#fb923c',
    border           = '#7c2d12',
  },

  graphite = {
    background       = '#111318',
    foreground       = '#f8fafc',
    tab_bar_bg       = '#0f1217',
    active_tab_bg    = '#334155',
    active_tab_fg    = '#ffffff',
    inactive_tab_bg  = '#1f2937',
    inactive_tab_fg  = '#cbd5e1',
    selection_bg     = '#334155',
    selection_fg     = '#f8fafc',
    accent_cursor    = '#a5b4fc',
    border           = '#475569',
  },

  high_contrast = {
    background       = '#05070a',
    foreground       = '#ffffff',
    tab_bar_bg       = '#000000',
    active_tab_bg    = '#0f172a',
    active_tab_fg    = '#ffffff',
    inactive_tab_bg  = '#111827',
    inactive_tab_fg  = '#dbeafe',
    selection_bg     = '#1d4ed8',
    selection_fg     = '#ffffff',
    accent_cursor    = '#38bdf8',
    border           = '#60a5fa',
  },
}

-- ---------------------------------------------------------------------------
-- Font family presets (ordered preference list for the font picker)
-- ---------------------------------------------------------------------------
M.font_family_presets = {
  'Cascadia Mono',
  'Cascadia Code',
  'Consolas',
  'JetBrains Mono',
  'Aptos Mono',
  'FiraCode Nerd Font Mono',
  'Iosevka Term',
  'Source Code Pro',
}

-- ---------------------------------------------------------------------------
-- Appearance profiles
-- Quick-switch bundles: scheme + font + size (and optional flag overrides).
-- ---------------------------------------------------------------------------
M.appearance_profiles = {
  {
    id      = 'powershell-studio',
    label   = 'PowerShell Studio',
    overrides = {
      color_scheme = 'Codex PowerShell',
      font_family  = 'Cascadia Mono',
      font_size    = 12.5,
    },
  },
  {
    id      = 'presentation',
    label   = 'Presentation',
    overrides = {
      color_scheme = 'Codex Ember',
      font_family  = 'Cascadia Code',
      font_size    = 14.5,
    },
  },
  {
    id      = 'compact',
    label   = 'Compact Focus',
    overrides = {
      color_scheme      = 'Codex Graphite',
      font_family       = 'Consolas',
      font_size         = 11.5,
      use_fancy_tab_bar = false,
    },
  },
}

return M
