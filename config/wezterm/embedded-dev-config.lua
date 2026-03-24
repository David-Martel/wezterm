-- WezTerm Configuration for Embedded Systems Development
-- Designed for serial monitoring, cross-compilation, debugging, and device management

local wezterm = require 'wezterm'
local act = wezterm.action
local config = {}

if wezterm.config_builder then
  config = wezterm.config_builder()
end

-- Normalize CWD for both string (older WezTerm) and Url (newer WezTerm) returns.
local function cwd_to_path(cwd)
  if not cwd then
    return nil
  end

  if type(cwd) == 'table' then
    if cwd.file_path and cwd.file_path ~= '' then
      return cwd.file_path
    end
    return tostring(cwd)
  end

  local path = tostring(cwd)
  path = path:gsub('^file://[^/]*/', '')
  path = path:gsub('^file://', '')
  return path
end

-- ============================================================================
-- EMBEDDED DEVELOPMENT KEY BINDINGS
-- ============================================================================

config.keys = {
  -- -------------------------------------------------------------------------
  -- SERIAL CONSOLE MANAGEMENT (F1-F4 + Alt)
  -- -------------------------------------------------------------------------

  -- F1: Quick serial console launcher
  {
    key = 'F1',
    action = wezterm.action_callback(function(window, pane)
      local tab = window:mux_window():spawn_tab({
        label = '📟 Serial Console',
      })
      tab:active_pane():send_text('# Serial Console Quick Launcher\n')
      tab:active_pane():send_text('# Common devices: /dev/ttyUSB0, /dev/ttyACM0, COM3\n')
      tab:active_pane():send_text('# picocom -b 115200 /dev/ttyUSB0\n')
    end),
    mods = 'ALT',
  },

  -- F2: Multiple serial monitors (split view)
  {
    key = 'F2',
    action = wezterm.action_callback(function(window, pane)
      -- Split horizontally for two serial monitors
      pane:split({ direction = 'Right', size = 0.5 })
      window:perform_action(act.ActivatePaneDirection('Left'), pane)
      pane:send_text('# Device 1: picocom -b 115200 /dev/ttyUSB0\n')
      window:perform_action(act.ActivatePaneDirection('Right'), pane)
      pane:send_text('# Device 2: picocom -b 115200 /dev/ttyUSB1\n')
    end),
    mods = 'ALT',
  },

  -- F3: Serial console with logging
  {
    key = 'F3',
    action = wezterm.action_callback(function(window, pane)
      local timestamp = os.date('%Y%m%d-%H%M%S')
      pane:send_text('picocom -b 115200 /dev/ttyUSB0 | tee serial-log-' .. timestamp .. '.txt\n')
    end),
    mods = 'ALT',
  },

  -- F4: Screen serial session (detachable)
  {
    key = 'F4',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('screen /dev/ttyUSB0 115200\n')
      pane:send_text('# Press Ctrl-A then K to kill, Ctrl-A then D to detach\n')
    end),
    mods = 'ALT',
  },

  -- -------------------------------------------------------------------------
  -- BUILD SYSTEM SHORTCUTS (F5-F8 + Alt)
  -- -------------------------------------------------------------------------

  -- F5: Quick build
  {
    key = 'F5',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('make -j$(nproc)\n')
    end),
    mods = 'ALT',
  },

  -- F6: Clean build
  {
    key = 'F6',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('make clean && make -j$(nproc)\n')
    end),
    mods = 'ALT',
  },

  -- F7: Flash to device
  {
    key = 'F7',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('make flash\n')
    end),
    mods = 'ALT',
  },

  -- F8: Build and flash
  {
    key = 'F8',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('make -j$(nproc) && make flash\n')
    end),
    mods = 'ALT',
  },

  -- -------------------------------------------------------------------------
  -- DEBUGGING WORKFLOWS (F9-F12 + Alt)
  -- -------------------------------------------------------------------------

  -- F9: Start OpenOCD server
  {
    key = 'F9',
    action = wezterm.action_callback(function(window, pane)
      local tab = window:mux_window():spawn_tab({
        label = '🔌 OpenOCD',
      })
      tab:active_pane():send_text('# OpenOCD Server\n')
      tab:active_pane():send_text('openocd -f interface/stlink.cfg -f target/stm32f4x.cfg\n')
    end),
    mods = 'ALT',
  },

  -- F10: GDB debugging session
  {
    key = 'F10',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('arm-none-eabi-gdb -ex "target remote localhost:3333" build/firmware.elf\n')
    end),
    mods = 'ALT',
  },

  -- F11: J-Link GDB server
  {
    key = 'F11',
    action = wezterm.action_callback(function(window, pane)
      local tab = window:mux_window():spawn_tab({
        label = '🔗 J-Link',
      })
      tab:active_pane():send_text('JLinkGDBServer -device STM32F407VG -if SWD -speed 4000\n')
    end),
    mods = 'ALT',
  },

  -- F12: Embedded development dashboard (4-pane layout)
  {
    key = 'F12',
    action = wezterm.action_callback(function(window, pane)
      -- Create 4-pane embedded dev layout
      -- Top-left: Build/code
      -- Top-right: Serial console
      -- Bottom-left: Debugger
      -- Bottom-right: Logs/monitoring

      local tab = window:mux_window():spawn_tab({
        label = '🛠️ Embedded Dev Dashboard',
      })

      local main_pane = tab:active_pane()

      -- Split horizontally first (top/bottom)
      local bottom = main_pane:split({ direction = 'Bottom', size = 0.5 })

      -- Split top pane vertically (left/right)
      local top_right = main_pane:split({ direction = 'Right', size = 0.5 })

      -- Split bottom pane vertically (left/right)
      local bottom_right = bottom:split({ direction = 'Right', size = 0.5 })

      -- Configure each pane
      main_pane:send_text('# Build & Code\necho "make -j$(nproc)"\n')
      top_right:send_text('# Serial Console\necho "picocom -b 115200 /dev/ttyUSB0"\n')
      bottom:send_text('# Debugger (GDB/OpenOCD)\necho "arm-none-eabi-gdb"\n')
      bottom_right:send_text('# Logs & Monitoring\ntail -f build.log\n')
    end),
    mods = 'ALT',
  },

  -- -------------------------------------------------------------------------
  -- CROSS-COMPILATION ENVIRONMENTS (Ctrl+Alt+Letter)
  -- -------------------------------------------------------------------------

  -- Ctrl+Alt+A: ARM toolchain environment
  {
    key = 'a',
    action = wezterm.action_callback(function(window, pane)
      local tab = window:mux_window():spawn_tab({
        label = '🔧 ARM',
      })
      tab:active_pane():send_text('export PATH=/usr/local/arm-none-eabi/bin:$PATH\n')
      tab:active_pane():send_text('export CROSS_COMPILE=arm-none-eabi-\n')
      tab:active_pane():send_text('arm-none-eabi-gcc --version\n')
    end),
    mods = 'CTRL|ALT',
  },

  -- Ctrl+Alt+R: RISC-V toolchain environment
  {
    key = 'r',
    action = wezterm.action_callback(function(window, pane)
      local tab = window:mux_window():spawn_tab({
        label = '🔧 RISC-V',
      })
      tab:active_pane():send_text('export PATH=/opt/riscv/bin:$PATH\n')
      tab:active_pane():send_text('export CROSS_COMPILE=riscv64-unknown-elf-\n')
      tab:active_pane():send_text('riscv64-unknown-elf-gcc --version\n')
    end),
    mods = 'CTRL|ALT',
  },

  -- Ctrl+Alt+V: AVR toolchain environment (Arduino)
  {
    key = 'v',
    action = wezterm.action_callback(function(window, pane)
      local tab = window:mux_window():spawn_tab({
        label = '🔧 AVR',
      })
      tab:active_pane():send_text('export PATH=/usr/local/avr/bin:$PATH\n')
      tab:active_pane():send_text('avr-gcc --version\n')
    end),
    mods = 'CTRL|ALT',
  },

  -- Ctrl+Alt+P: PlatformIO environment
  {
    key = 'p',
    action = wezterm.action_callback(function(window, pane)
      local tab = window:mux_window():spawn_tab({
        label = '📦 PlatformIO',
      })
      tab:active_pane():send_text('pio --version\n')
      tab:active_pane():send_text('# pio run - Build\n')
      tab:active_pane():send_text('# pio run -t upload - Flash\n')
      tab:active_pane():send_text('# pio device monitor - Serial monitor\n')
    end),
    mods = 'CTRL|ALT',
  },

  -- -------------------------------------------------------------------------
  -- DEVICE & PORT MANAGEMENT (Ctrl+Shift+Letter)
  -- -------------------------------------------------------------------------

  -- Ctrl+Shift+D: List serial devices
  {
    key = 'D',
    action = wezterm.action_callback(function(window, pane)
      if wezterm.target_triple:find("windows") then
        pane:send_text('mode | findstr COM\n')
      else
        pane:send_text('ls -l /dev/tty{USB,ACM}* 2>/dev/null || echo "No devices found"\n')
      end
    end),
    mods = 'CTRL|SHIFT',
  },

  -- Ctrl+Shift+P: Show device permissions
  {
    key = 'P',
    action = wezterm.action_callback(function(window, pane)
      if not wezterm.target_triple:find("windows") then
        pane:send_text('ls -l /dev/tty{USB,ACM}* 2>/dev/null\n')
        pane:send_text('# Add user to dialout group: sudo usermod -a -G dialout $USER\n')
      end
    end),
    mods = 'CTRL|SHIFT',
  },

  -- Ctrl+Shift+M: Device monitoring dashboard
  {
    key = 'M',
    action = wezterm.action_callback(function(window, pane)
      if not wezterm.target_triple:find("windows") then
        pane:send_text('watch -n 1 "ls -lh /dev/tty{USB,ACM}* 2>/dev/null; echo; lsusb | grep -i \'serial\\|uart\\|ftdi\\|cp210\\|ch340\'"\n')
      end
    end),
    mods = 'CTRL|SHIFT',
  },

  -- -------------------------------------------------------------------------
  -- HEX/BINARY FILE OPERATIONS (Ctrl+Alt+Shift+Letter)
  -- -------------------------------------------------------------------------

  -- Ctrl+Alt+Shift+H: Hexdump viewer
  {
    key = 'H',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('# Hexdump viewer - Usage: hexdump -C firmware.bin | less\n')
      pane:send_text('# Or: xxd firmware.bin | less\n')
    end),
    mods = 'CTRL|ALT|SHIFT',
  },

  -- Ctrl+Alt+Shift+B: Binary file info
  {
    key = 'B',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('# Binary file analysis\n')
      pane:send_text('file firmware.elf\n')
      pane:send_text('arm-none-eabi-size firmware.elf\n')
      pane:send_text('arm-none-eabi-objdump -h firmware.elf\n')
    end),
    mods = 'CTRL|ALT|SHIFT',
  },

  -- Ctrl+Alt+Shift+C: Convert ELF to bin/hex
  {
    key = 'C',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('# Convert ELF to binary formats\n')
      pane:send_text('arm-none-eabi-objcopy -O binary firmware.elf firmware.bin\n')
      pane:send_text('arm-none-eabi-objcopy -O ihex firmware.elf firmware.hex\n')
    end),
    mods = 'CTRL|ALT|SHIFT',
  },

  -- -------------------------------------------------------------------------
  -- LOG PARSING & FILTERING (Ctrl+Alt+Number)
  -- -------------------------------------------------------------------------

  -- Ctrl+Alt+1: Filter ERROR logs
  {
    key = '1',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('tail -f serial.log | grep --color=always -i error\n')
    end),
    mods = 'CTRL|ALT',
  },

  -- Ctrl+Alt+2: Filter WARNING logs
  {
    key = '2',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('tail -f serial.log | grep --color=always -i "warn\\|warning"\n')
    end),
    mods = 'CTRL|ALT',
  },

  -- Ctrl+Alt+3: Filter DEBUG logs
  {
    key = '3',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('tail -f serial.log | grep --color=always -i debug\n')
    end),
    mods = 'CTRL|ALT',
  },

  -- Ctrl+Alt+4: Timestamp logs
  {
    key = '4',
    action = wezterm.action_callback(function(window, pane)
      pane:send_text('tail -f serial.log | ts "[%Y-%m-%d %H:%M:%S]"\n')
    end),
    mods = 'CTRL|ALT',
  },

  -- -------------------------------------------------------------------------
  -- QUICK ACTIONS (Leader key sequences)
  -- -------------------------------------------------------------------------

  -- Set leader key (like tmux)
  {
    key = 'Space',
    mods = 'CTRL',
    action = act.SendKey { key = 'Space', mods = 'CTRL' },
  },

  -- Standard pane navigation
  { key = 'h', mods = 'CTRL|SHIFT', action = act.ActivatePaneDirection('Left') },
  { key = 'l', mods = 'CTRL|SHIFT', action = act.ActivatePaneDirection('Right') },
  { key = 'k', mods = 'CTRL|SHIFT', action = act.ActivatePaneDirection('Up') },
  { key = 'j', mods = 'CTRL|SHIFT', action = act.ActivatePaneDirection('Down') },

  -- Pane resizing
  { key = 'h', mods = 'CTRL|ALT', action = act.AdjustPaneSize { 'Left', 2 } },
  { key = 'l', mods = 'CTRL|ALT', action = act.AdjustPaneSize { 'Right', 2 } },
  { key = 'k', mods = 'CTRL|ALT', action = act.AdjustPaneSize { 'Up', 2 } },
  { key = 'j', mods = 'CTRL|ALT', action = act.AdjustPaneSize { 'Down', 2 } },

  -- Quick splits
  { key = '|', mods = 'CTRL|SHIFT', action = act.SplitHorizontal { domain = 'CurrentPaneDomain' } },
  { key = '_', mods = 'CTRL|SHIFT', action = act.SplitVertical { domain = 'CurrentPaneDomain' } },
}

-- ============================================================================
-- LAUNCHER MENU FOR EMBEDDED DEVELOPMENT
-- ============================================================================

config.launch_menu = {
  -- Serial Console Tools
  {
    label = '📟 Serial Console (picocom 115200)',
    args = { 'bash', '-c', 'echo "Device: /dev/ttyUSB0" && picocom -b 115200 /dev/ttyUSB0' },
  },
  {
    label = '📟 Serial Console (screen 115200)',
    args = { 'bash', '-c', 'screen /dev/ttyUSB0 115200' },
  },
  {
    label = '📟 Serial Console with Logging',
    args = { 'bash', '-c', 'picocom -b 115200 /dev/ttyUSB0 | tee "serial-$(date +%Y%m%d-%H%M%S).log"' },
  },

  -- Cross-Compilation Environments
  {
    label = '🔧 ARM Development Environment',
    args = { 'bash', '-c', 'export PATH=/usr/local/arm-none-eabi/bin:$PATH && exec bash' },
  },
  {
    label = '🔧 RISC-V Development Environment',
    args = { 'bash', '-c', 'export PATH=/opt/riscv/bin:$PATH && exec bash' },
  },
  {
    label = '🔧 AVR Development Environment',
    args = { 'bash', '-c', 'export PATH=/usr/local/avr/bin:$PATH && exec bash' },
  },

  -- PlatformIO
  {
    label = '📦 PlatformIO Build',
    args = { 'bash', '-c', 'pio run' },
  },
  {
    label = '📦 PlatformIO Upload',
    args = { 'bash', '-c', 'pio run -t upload' },
  },
  {
    label = '📦 PlatformIO Monitor',
    args = { 'bash', '-c', 'pio device monitor' },
  },

  -- Debugging Tools
  {
    label = '🔌 OpenOCD (ST-Link)',
    args = { 'bash', '-c', 'openocd -f interface/stlink.cfg -f target/stm32f4x.cfg' },
  },
  {
    label = '🔌 OpenOCD (J-Link)',
    args = { 'bash', '-c', 'openocd -f interface/jlink.cfg -f target/stm32f4x.cfg' },
  },
  {
    label = '🐛 GDB (ARM)',
    args = { 'bash', '-c', 'arm-none-eabi-gdb -ex "target remote localhost:3333"' },
  },

  -- Device Management
  {
    label = '🔍 List Serial Devices',
    args = { 'bash', '-c', 'ls -l /dev/tty{USB,ACM}* 2>/dev/null && read -p "Press enter to continue..."' },
  },
  {
    label = '🔍 USB Device Monitor',
    args = { 'bash', '-c', 'watch -n 1 "lsusb; echo; ls -l /dev/tty{USB,ACM}* 2>/dev/null"' },
  },

  -- Build Systems
  {
    label = '⚙️ Make Build',
    args = { 'bash', '-c', 'make -j$(nproc)' },
  },
  {
    label = '⚙️ CMake Build',
    args = { 'bash', '-c', 'cmake --build build -j$(nproc)' },
  },
  {
    label = '⚙️ Clean Build',
    args = { 'bash', '-c', 'make clean && make -j$(nproc)' },
  },
}

-- ============================================================================
-- EMBEDDED DEVELOPMENT VISUAL SETTINGS
-- ============================================================================

-- Color scheme optimized for reading logs and code
config.color_scheme = 'Monokai Pro (Gogh)'

-- Font settings for readability
config.font = wezterm.font_with_fallback({
  'JetBrains Mono',
  'Fira Code',
  'Cascadia Code',
  'Consolas',
})
config.font_size = 10.0
config.line_height = 1.1

-- Enable ligatures for better code readability
config.harfbuzz_features = { 'calt=1', 'clig=1', 'liga=1' }

-- Scrollback buffer (important for log viewing)
config.scrollback_lines = 10000

-- Enable hyperlinks for file paths in logs
config.hyperlink_rules = {
  -- Match file paths with line numbers (common in compiler output)
  {
    regex = [[["]?([a-zA-Z]:[/\\]|\.{0,2}/)?[^:"\s]+\.(?:c|cpp|h|hpp|rs|py|s|S):\d+]],
    format = '$0',
  },
  -- Match hex addresses (common in embedded debugging)
  {
    regex = [[0x[0-9a-fA-F]+]],
    format = '$0',
  },
}

-- Tab bar styling
config.use_fancy_tab_bar = true
config.tab_bar_at_bottom = false
config.tab_max_width = 32

-- Window appearance
config.window_padding = {
  left = 4,
  right = 4,
  top = 4,
  bottom = 4,
}

config.inactive_pane_hsb = {
  saturation = 0.8,
  brightness = 0.7,
}

-- ============================================================================
-- EMBEDDED DEVELOPMENT HELPER FUNCTIONS
-- ============================================================================

-- Status bar showing serial port info (if applicable)
wezterm.on('update-status', function(window, pane)
  local cwd = cwd_to_path(pane:get_current_working_dir())
  if not cwd then
    cwd = ''
  end

  local stat = window:active_workspace()
  local time = wezterm.strftime '%H:%M'

  window:set_left_status(wezterm.format {
    { Text = wezterm.nerdfonts.md_folder .. ' ' .. cwd },
    { Text = ' | ' },
    { Text = wezterm.nerdfonts.md_clock .. ' ' .. time },
    { Text = ' ' },
  })
end)

-- Automatic serial device detection
wezterm.on('window-config-reloaded', function(window, pane)
  -- Could add logic here to detect available serial devices
  -- and update launcher menu dynamically
end)

return config
