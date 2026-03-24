-- ============================================================================
-- WEZTERM-UTILS EVENTS MODULE
-- Custom event handlers for utility integration
-- ============================================================================

local wezterm = require('wezterm')
local M = {}

function M.on_utility_launched(utility_name, mode, pane_id)
  wezterm.log_info(string.format(
    'Event: utility-launched (name=%s, mode=%s, pane=%s)',
    utility_name,
    mode,
    tostring(pane_id)
  ))

  local state = require('wezterm-utils.state')
  if state.initialized then
    local history = state.load_state('launch_history') or { launches = {} }

    table.insert(history.launches, {
      utility = utility_name,
      mode = mode,
      pane_id = pane_id,
      timestamp = os.time(),
    })

    if #history.launches > 50 then
      table.remove(history.launches, 1)
    end

    state.save_state('launch_history', history)
  end
end

function M.on_utility_closed(utility_name, pane_id)
  wezterm.log_info(string.format(
    'Event: utility-closed (name=%s, pane=%s)',
    utility_name,
    tostring(pane_id)
  ))
end

function M.on_state_saved(utility_name, state_data)
  wezterm.log_info(string.format(
    'Event: state-saved (name=%s, keys=%d)',
    utility_name,
    state_data and #state_data or 0
  ))
end

function M.on_state_loaded(utility_name, state_data)
  wezterm.log_info(string.format(
    'Event: state-loaded (name=%s, keys=%d)',
    utility_name,
    state_data and #state_data or 0
  ))
end

function M.register_handlers()
  wezterm.log_info('Event handlers registered')
end

return M
