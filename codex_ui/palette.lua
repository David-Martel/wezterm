-- codex_ui/palette.lua
-- Categorized command palette entries and InputSelector helpers.
-- Absorbs dispatch.lua utility patterns.

local M = {}

-- ---------------------------------------------------------------------------
-- Constructor
-- ---------------------------------------------------------------------------
-- @param wezterm   the wezterm module
-- @param act       wezterm.action table
-- @param schemes   codex_ui/schemes.lua module (has .ui, .curated_scheme_names)
-- @param prefs     persistent preferences table (optional, reserved for future use)
function M.new(wezterm, act, schemes, prefs)
  local api = {}

  -- -------------------------------------------------------------------------
  -- api.show_selector(window, pane, title, choices, on_select, description)
  -- Thin wrapper around wezterm's InputSelector action.
  -- choices: array of strings or {id, label} tables
  -- on_select: function(window, pane, id, label)
  -- -------------------------------------------------------------------------
  function api.show_selector(window, pane, title, choices, on_select, description)
    local items = {}
    for i, choice in ipairs(choices) do
      if type(choice) == "table" then
        items[i] = { id = tostring(choice.id or choice[1] or i), label = tostring(choice.label or choice[2] or choice.id or choice[1] or i) }
      else
        items[i] = { id = tostring(choice), label = tostring(choice) }
      end
    end

    window:perform_action(
      act.InputSelector({
        title = title,
        fuzzy = true,
        description = description or "",
        choices = items,
        action = wezterm.action_callback(function(inner_win, inner_pane, sel_id, sel_label)
          if sel_id ~= nil then
            on_select(inner_win, inner_pane, sel_id, sel_label)
          end
        end),
      }),
      pane
    )
  end

  -- -------------------------------------------------------------------------
  -- api.command_palette_entry(brief, action, doc, icon)
  -- Creates a palette entry table in the shape WezTerm expects.
  -- -------------------------------------------------------------------------
  function api.command_palette_entry(brief, action, doc, icon)
    local entry = {
      brief = brief,
      action = action,
    }
    if doc then entry.doc = doc end
    if icon then entry.icon = icon end
    return entry
  end

  -- -------------------------------------------------------------------------
  -- api.build_palette(config_context)
  -- Returns the full categorized palette entries array.
  --
  -- config_context fields:
  --   .llm_agents  table of { label, args, cwd, env }
  --   .apply_ui    function(window, updates, message)
  -- -------------------------------------------------------------------------
  function api.build_palette(config_context)
    local entries = {}
    local ctx = config_context or {}

    -- -----------------------------------------------------------------------
    -- LLM entries — one per agent
    -- -----------------------------------------------------------------------
    local llm_agents = ctx.llm_agents or {}
    for name, agent in pairs(llm_agents) do
      local spawn_args = {
        args = agent.args,
      }
      if agent.cwd then
        spawn_args.cwd = agent.cwd
      end
      if agent.env then
        spawn_args.set_environment_variables = agent.env
      end

      entries[#entries + 1] = api.command_palette_entry(
        "[LLM] Launch " .. (agent.label or name),
        act.SpawnCommandInNewTab(spawn_args),
        "Launch LLM agent: " .. (agent.label or name)
      )
    end

    -- -----------------------------------------------------------------------
    -- UI entries
    -- -----------------------------------------------------------------------

    -- Color Scheme selector
    entries[#entries + 1] = api.command_palette_entry(
      "[UI] Settings: Color Scheme",
      wezterm.action_callback(function(window, pane)
        local scheme_names = schemes and schemes.curated_scheme_names or {}
        local choices = {}
        for _, name in ipairs(scheme_names) do
          choices[#choices + 1] = { id = name, label = name }
        end
        api.show_selector(
          window,
          pane,
          "Select Color Scheme",
          choices,
          function(win, _pane, id, _label)
            if ctx.apply_ui then
              ctx.apply_ui(win, { color_scheme = id }, "Color scheme changed to: " .. id)
            end
          end,
          "Choose a curated color scheme"
        )
      end),
      "Change the terminal color scheme"
    )

    -- Font Size selector
    local font_sizes = { 10.5, 11.0, 11.5, 12.0, 12.5, 13.0, 14.0, 15.0, 16.0 }
    entries[#entries + 1] = api.command_palette_entry(
      "[UI] Settings: Font Size",
      wezterm.action_callback(function(window, pane)
        local choices = {}
        for _, sz in ipairs(font_sizes) do
          local label = tostring(sz) .. "pt"
          choices[#choices + 1] = { id = tostring(sz), label = label }
        end
        api.show_selector(
          window,
          pane,
          "Select Font Size",
          choices,
          function(win, _pane, id, _label)
            local size = tonumber(id)
            if size and ctx.apply_ui then
              ctx.apply_ui(win, { font_size = size }, "Font size set to: " .. tostring(size))
            end
          end,
          "Choose a font size"
        )
      end),
      "Change the terminal font size"
    )

    -- Appearance Profiles
    entries[#entries + 1] = api.command_palette_entry(
      "[UI] Settings: Appearance Profiles",
      wezterm.action_callback(function(window, pane)
        window:perform_action(act.EmitEvent("show-profiles"), pane)
      end),
      "Browse and apply appearance profiles"
    )

    -- Reset Defaults
    entries[#entries + 1] = api.command_palette_entry(
      "[UI] Settings: Reset Defaults",
      wezterm.action_callback(function(window, pane)
        window:perform_action(act.EmitEvent("reset-ui-defaults"), pane)
      end),
      "Reset UI to default settings"
    )

    -- Font Family selector
    entries[#entries + 1] = api.command_palette_entry(
      "[UI] Settings: Font Family",
      wezterm.action_callback(function(window, pane)
        local font_presets = schemes and schemes.font_family_presets or {}
        local choices = {}
        for _, family in ipairs(font_presets) do
          choices[#choices + 1] = { id = family, label = family }
        end
        api.show_selector(
          window,
          pane,
          "Select Font Family",
          choices,
          function(win, _pane, id, _label)
            if ctx.apply_ui then
              ctx.apply_ui(win, { font_family = id }, "Font family changed to: " .. id)
            end
          end,
          "Choose from available font families"
        )
      end),
      "Change the terminal font family"
    )

    -- Window Opacity selector
    entries[#entries + 1] = api.command_palette_entry(
      "[UI] Settings: Window Opacity",
      wezterm.action_callback(function(window, pane)
        local choices = {}
        -- 0.80 to 1.00 in 0.02 increments
        local opacity = 0.80
        while opacity <= 1.005 do
          local val = string.format("%.2f", opacity)
          local pct = string.format("%d%%", math.floor(opacity * 100 + 0.5))
          choices[#choices + 1] = { id = val, label = pct .. "  (" .. val .. ")" }
          opacity = opacity + 0.02
        end
        api.show_selector(
          window,
          pane,
          "Select Window Opacity",
          choices,
          function(win, _pane, id, _label)
            local value = tonumber(id)
            if value and ctx.apply_ui then
              ctx.apply_ui(win, { window_background_opacity = value }, "Window opacity set to: " .. id)
            end
          end,
          "Choose window background opacity"
        )
      end),
      "Adjust window background opacity"
    )

    -- Backdrop selector
    entries[#entries + 1] = api.command_palette_entry(
      "[UI] Settings: Backdrop",
      wezterm.action_callback(function(window, pane)
        local choices = {
          { id = "None",    label = "None     — No system backdrop" },
          { id = "Acrylic", label = "Acrylic  — Translucent blur (Win10+)" },
          { id = "Mica",    label = "Mica     — Adaptive tint (Win11)" },
          { id = "Tabbed",  label = "Tabbed   — Tabbed Mica (Win11)" },
        }
        api.show_selector(
          window,
          pane,
          "Select Window Backdrop",
          choices,
          function(win, _pane, id, _label)
            if ctx.apply_ui then
              ctx.apply_ui(win, { backdrop = id }, "Backdrop set to: " .. id .. "  (restart may be required)")
            end
          end,
          "Choose system backdrop effect (may require restart)"
        )
      end),
      "Change window backdrop effect"
    )

    -- -----------------------------------------------------------------------
    -- TOOL entries
    -- -----------------------------------------------------------------------

    entries[#entries + 1] = api.command_palette_entry(
      "[TOOL] Toggle Explorer Panel",
      wezterm.action_callback(function(window, pane)
        window:perform_action(act.EmitEvent("toggle-explorer"), pane)
      end),
      "Open or close the file explorer panel"
    )

    entries[#entries + 1] = api.command_palette_entry(
      "[TOOL] Toggle Watcher Panel",
      wezterm.action_callback(function(window, pane)
        window:perform_action(act.EmitEvent("toggle-watcher"), pane)
      end),
      "Open or close the file watcher panel"
    )

    entries[#entries + 1] = api.command_palette_entry(
      "[TOOL] Toggle Editor Panel",
      wezterm.action_callback(function(window, pane)
        window:perform_action(act.EmitEvent("toggle-editor"), pane)
      end),
      "Open or close the editor panel"
    )

    entries[#entries + 1] = api.command_palette_entry(
      "[TOOL] Show Render Diagnostics",
      wezterm.action_callback(function(window, pane)
        window:perform_action(act.EmitEvent("show-render-diagnostics"), pane)
      end),
      "Display render diagnostics information"
    )

    -- -----------------------------------------------------------------------
    -- SHELL entries
    -- -----------------------------------------------------------------------

    entries[#entries + 1] = api.command_palette_entry(
      "[SHELL] New PowerShell Tab",
      act.SpawnCommandInNewTab({ args = { "pwsh.exe" } }),
      "Open a new PowerShell (pwsh) tab"
    )

    entries[#entries + 1] = api.command_palette_entry(
      "[SHELL] New Command Prompt Tab",
      act.SpawnCommandInNewTab({ args = { "cmd.exe" } }),
      "Open a new Command Prompt tab"
    )

    entries[#entries + 1] = api.command_palette_entry(
      "[SHELL] New WSL Ubuntu Tab",
      act.SpawnCommandInNewTab({
        args = { "wsl.exe", "--distribution", "Ubuntu", "--cd", "~" },
      }),
      "Open a new WSL Ubuntu tab in the home directory"
    )

    -- -----------------------------------------------------------------------
    -- NAV entries
    -- -----------------------------------------------------------------------

    entries[#entries + 1] = api.command_palette_entry(
      "[NAV] Open Launcher",
      act.ShowLauncherArgs({
        flags = "FUZZY|LAUNCH_MENU_ITEMS|COMMANDS|WORKSPACES|TABS",
      }),
      "Open the fuzzy launcher overlay"
    )

    entries[#entries + 1] = api.command_palette_entry(
      "[NAV] Search Scrollback",
      act.Search({ CaseSensitiveString = "" }),
      "Search the scrollback buffer"
    )

    entries[#entries + 1] = api.command_palette_entry(
      "[NAV] Split Pane Auto",
      wezterm.action_callback(function(window, pane)
        window:perform_action(act.EmitEvent("split-auto"), pane)
      end),
      "Automatically split the current pane"
    )

    return entries
  end

  return api
end

return M
