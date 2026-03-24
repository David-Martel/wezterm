-- ============================================================================
-- WEZTERM-UTILS IPC MODULE
-- Inter-process communication client (Lua side)
-- ============================================================================

local wezterm = require('wezterm')
local M = {}

M.socket_path = nil
M.connected = false

function M.init(socket_path)
  M.socket_path = socket_path
  wezterm.log_info('IPC module initialized (socket: ' .. socket_path .. ')')
  wezterm.log_warn('IPC functionality not yet implemented in Lua - requires FFI or helper process')
  M.connected = false
  return false
end

function M.send_message(message)
  if not M.connected then
    wezterm.log_warn('IPC not connected - cannot send message')
    return false
  end

  wezterm.log_info('IPC send (stub): ' .. wezterm.json_encode(message))
  return true
end

function M.receive_message()
  if not M.connected then
    return nil
  end

  return nil
end

function M.shutdown()
  if M.connected then
    wezterm.log_info('IPC connection closed')
    M.connected = false
  end
end

function M.status()
  return {
    socket_path = M.socket_path,
    connected = M.connected,
    implementation = 'stub',
  }
end

return M
