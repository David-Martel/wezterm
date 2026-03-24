-- ============================================================================
-- WEZTERM-UTILS INIT MODULE
-- Module initialization and version information
-- ============================================================================

local M = {}

M.VERSION = '1.0.0'
M.DESCRIPTION = 'WezTerm utilities integration module'

M.modules = {
  'launcher',
  'ipc',
  'state',
  'events',
  'config',
}

M.initialized = false

function M.init()
  if M.initialized then
    return true
  end

  M.initialized = true
  return true
end

return M
