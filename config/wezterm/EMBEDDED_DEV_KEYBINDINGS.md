# WezTerm Embedded Development Keybindings Reference

Quick reference guide for embedded systems development workflows in WezTerm.

## 🎯 Quick Start

1. **Copy the configuration:**
   ```bash
   # Include in your main wezterm.lua:
   # local embedded = require('embedded-dev-config')
   # Or source directly: require('embedded-dev-config')
   ```

2. **Test keybindings:**
   - Press `Alt+F12` to open the full embedded development dashboard
   - Press `Alt+F1` for quick serial console launcher

## 📟 Serial Console Management (Alt+F1-F4)

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Alt+F1` | Quick Serial Console | Opens template for picocom serial connection |
| `Alt+F2` | Multiple Serial Monitors | Split view for two devices simultaneously |
| `Alt+F3` | Serial with Logging | Logs all serial output to timestamped file |
| `Alt+F4` | Screen Session | Detachable screen session (Ctrl-A D to detach) |

### Common Serial Tools
- **picocom**: `picocom -b 115200 /dev/ttyUSB0`
- **screen**: `screen /dev/ttyUSB0 115200` (Ctrl-A K to exit)
- **minicom**: `minicom -D /dev/ttyUSB0 -b 115200`
- **cu**: `cu -l /dev/ttyUSB0 -s 115200`

### Baud Rates Reference
- 9600, 19200, 38400, 57600, **115200** (most common), 230400, 460800, 921600

## ⚙️ Build System Shortcuts (Alt+F5-F8)

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Alt+F5` | Quick Build | `make -j$(nproc)` |
| `Alt+F6` | Clean Build | `make clean && make -j$(nproc)` |
| `Alt+F7` | Flash Device | `make flash` |
| `Alt+F8` | Build + Flash | Combined build and flash in one command |

### Supported Build Systems
- **GNU Make**: Standard Makefile projects
- **CMake**: `cmake --build build -j$(nproc)`
- **PlatformIO**: `pio run`, `pio run -t upload`
- **Cargo**: `cargo build --release` (Rust embedded)

## 🐛 Debugging Workflows (Alt+F9-F12)

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Alt+F9` | Start OpenOCD | Launches OpenOCD server for debugging |
| `Alt+F10` | GDB Session | Opens GDB connected to OpenOCD |
| `Alt+F11` | J-Link GDB Server | Starts J-Link GDB server |
| `Alt+F12` | **Dev Dashboard** | 4-pane layout: build, serial, debugger, logs |

### GDB Quick Commands
```gdb
target remote localhost:3333   # Connect to OpenOCD
monitor reset halt             # Reset and halt target
load                           # Flash firmware
continue                       # Resume execution
break main                     # Set breakpoint
info registers                 # View registers
x/16x 0x20000000              # Examine memory
```

### OpenOCD Common Interfaces
- ST-Link: `interface/stlink.cfg`
- J-Link: `interface/jlink.cfg`
- CMSIS-DAP: `interface/cmsis-dap.cfg`

## 🔧 Cross-Compilation Environments (Ctrl+Alt+Letter)

| Keybinding | Toolchain | Environment |
|------------|-----------|-------------|
| `Ctrl+Alt+A` | ARM | arm-none-eabi-gcc (Cortex-M, Cortex-A) |
| `Ctrl+Alt+R` | RISC-V | riscv64-unknown-elf-gcc |
| `Ctrl+Alt+V` | AVR | avr-gcc (Arduino, ATmega) |
| `Ctrl+Alt+P` | PlatformIO | pio CLI environment |

### Toolchain Verification
```bash
# Check installed toolchains
arm-none-eabi-gcc --version
riscv64-unknown-elf-gcc --version
avr-gcc --version
pio --version
```

### Common Target Architectures
- **ARM Cortex-M**: STM32, NRF52, SAMD21, ESP32-C3
- **RISC-V**: SiFive, ESP32-C3/C6, GD32VF103
- **AVR**: Arduino Uno (ATmega328P), Arduino Mega (ATmega2560)
- **Xtensa**: ESP32, ESP8266 (via PlatformIO)

## 🔍 Device & Port Management (Ctrl+Shift+Letter)

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Ctrl+Shift+D` | List Devices | Shows all /dev/ttyUSB* and /dev/ttyACM* |
| `Ctrl+Shift+P` | Device Permissions | Check device permissions and group membership |
| `Ctrl+Shift+M` | Device Monitor | Live monitoring of connected USB serial devices |

### Fix Device Permissions (Linux)
```bash
# Add user to dialout group (required for serial access)
sudo usermod -a -G dialout $USER

# Then logout and login, or:
newgrp dialout

# Verify group membership
groups | grep dialout
```

### Windows COM Port Management
```powershell
# List COM ports
mode

# Device Manager
devmgmt.msc

# PowerShell device listing
Get-WmiObject Win32_SerialPort | Select-Object DeviceID,Description
```

## 🔢 Binary File Operations (Ctrl+Alt+Shift+Letter)

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Ctrl+Alt+Shift+H` | Hexdump | View binary files in hex format |
| `Ctrl+Alt+Shift+B` | Binary Info | Show ELF file sections and sizes |
| `Ctrl+Alt+Shift+C` | Convert Formats | ELF → bin/hex conversion |

### Binary Analysis Tools
```bash
# View file format
file firmware.elf

# Check sizes (flash/RAM usage)
arm-none-eabi-size firmware.elf

# View sections
arm-none-eabi-objdump -h firmware.elf

# Disassemble
arm-none-eabi-objdump -d firmware.elf | less

# Convert to binary
arm-none-eabi-objcopy -O binary firmware.elf firmware.bin

# Convert to Intel HEX
arm-none-eabi-objcopy -O ihex firmware.elf firmware.hex
```

### Hex Viewers
- **hexdump**: `hexdump -C firmware.bin | less`
- **xxd**: `xxd firmware.bin | less`
- **od**: `od -A x -t x1z firmware.bin`
- **hexyl** (modern): `hexyl firmware.bin`

## 📊 Log Parsing & Filtering (Ctrl+Alt+Number)

| Keybinding | Filter | Description |
|------------|--------|-------------|
| `Ctrl+Alt+1` | ERROR | Show only error messages |
| `Ctrl+Alt+2` | WARNING | Show only warnings |
| `Ctrl+Alt+3` | DEBUG | Show only debug messages |
| `Ctrl+Alt+4` | Timestamp | Add timestamps to each log line |

### Advanced Log Parsing
```bash
# Real-time filtering
tail -f serial.log | grep --color=always -i error

# Multiple patterns
tail -f serial.log | grep -E '(ERROR|WARN|CRITICAL)'

# Case-insensitive
tail -f serial.log | grep -i 'error\|fail\|critical'

# With timestamps
tail -f serial.log | ts "[%Y-%m-%d %H:%M:%S]"

# With line numbers
tail -f serial.log | nl

# Filter by time range (using awk)
awk '/2024-01-20 15:00/,/2024-01-20 16:00/' serial.log
```

## 🎨 Pane Navigation & Management

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Ctrl+Shift+H/J/K/L` | Navigate Panes | Vim-style pane navigation |
| `Ctrl+Alt+H/J/K/L` | Resize Panes | Adjust pane sizes |
| `Ctrl+Shift+\|` | Split Horizontal | Create side-by-side panes |
| `Ctrl+Shift+_` | Split Vertical | Create top/bottom panes |

## 🚀 Launcher Menu (Ctrl+Shift+L)

Access pre-configured environments through the launcher menu:

### Serial Console Options
- picocom 115200 baud
- screen 115200 baud
- Serial with logging enabled

### Development Environments
- ARM development (arm-none-eabi-gcc)
- RISC-V development (riscv64-unknown-elf-gcc)
- AVR development (avr-gcc)

### PlatformIO Quick Actions
- Build project
- Upload firmware
- Serial monitor

### Debugging Tools
- OpenOCD with ST-Link
- OpenOCD with J-Link
- GDB for ARM

### Device Management
- List serial devices
- USB device monitor

## 💡 Practical Workflows

### Workflow 1: First Connection to New Device
1. `Ctrl+Shift+D` - List available serial devices
2. `Alt+F1` - Open serial console
3. Adjust baud rate if needed: `picocom -b 9600 /dev/ttyUSB0`
4. `Alt+F3` - Enable logging for troubleshooting

### Workflow 2: Build-Flash-Debug Cycle
1. `Alt+F8` - Build and flash firmware
2. `Alt+F2` - Open dual serial monitors (bootloader + app)
3. `Alt+F9` - Start OpenOCD in background
4. `Alt+F10` - Connect GDB for debugging

### Workflow 3: Multi-Device Testing
1. `Alt+F12` - Open embedded dev dashboard
2. Configure each pane:
   - Top-left: Build commands
   - Top-right: Device 1 serial (picocom /dev/ttyUSB0)
   - Bottom-left: Device 2 serial (picocom /dev/ttyUSB1)
   - Bottom-right: Log aggregation
3. Monitor all devices simultaneously

### Workflow 4: Continuous Integration Testing
1. `Alt+F6` - Clean build
2. `Alt+F7` - Flash device
3. `Alt+F3` - Serial console with logging
4. `Ctrl+Alt+1` - Filter for errors in logs
5. Review build logs and test results

## 🛠️ Customization Tips

### Adding Custom Serial Baud Rates
Edit the launcher menu in `embedded-dev-config.lua`:
```lua
{
  label = '📟 Serial Console (921600)',
  args = { 'bash', '-c', 'picocom -b 921600 /dev/ttyUSB0' },
}
```

### Custom Build Commands
Add project-specific build shortcuts:
```lua
{
  key = 'F5',
  action = wezterm.action_callback(function(window, pane)
    pane:send_text('cmake --build build --target my_firmware -j8\n')
  end),
  mods = 'ALT|SHIFT',
}
```

### Device-Specific Configurations
Create device profiles for common targets:
```lua
-- STM32F4 Discovery
{
  label = '🎯 STM32F4 Discovery',
  args = { 'bash', '-c', 'openocd -f board/stm32f4discovery.cfg' },
}

-- ESP32 Development
{
  label = '🎯 ESP32 Flash',
  args = { 'bash', '-c', 'esptool.py --chip esp32 write_flash 0x1000 firmware.bin' },
}
```

## 📚 Additional Resources

### Essential Tools to Install
```bash
# Serial console tools
sudo apt install picocom minicom screen cu

# Cross-compilation toolchains
sudo apt install gcc-arm-none-eabi gcc-avr

# Debugging tools
sudo apt install openocd gdb-multiarch

# Binary utilities
sudo apt install binutils-arm-none-eabi binutils-avr

# PlatformIO
pip install platformio

# Modern hex viewer
cargo install hexyl

# Log timestamping
sudo apt install moreutils  # for 'ts' command
```

### Useful Documentation
- **OpenOCD**: https://openocd.org/doc/
- **GDB for Embedded**: https://sourceware.org/gdb/documentation/
- **ARM Cortex-M Programming**: https://developer.arm.com/documentation/
- **PlatformIO**: https://docs.platformio.org/

### Common Gotchas
1. **Permission denied on /dev/ttyUSB0**: Add user to `dialout` group
2. **OpenOCD can't find interface**: Check `interface/` and `target/` config files
3. **picocom not exiting**: Press `Ctrl-A` then `Ctrl-X`
4. **GDB connection refused**: Ensure OpenOCD is running first
5. **Build failing**: Check toolchain PATH and CROSS_COMPILE environment variables

## 🎓 Learning Resources

### Getting Started with Embedded Development
1. Start with Arduino (AVR) for basics
2. Move to STM32 (ARM Cortex-M) for professional development
3. Explore RISC-V for cutting-edge open architecture
4. Use PlatformIO for unified development experience

### Recommended Projects
- **STM32 Blue Pill**: Cheap ARM Cortex-M3 board (~$2)
- **ESP32**: WiFi/Bluetooth with excellent tooling
- **Arduino Uno**: Classic AVR learning platform
- **Raspberry Pi Pico**: RP2040 with dual-core ARM Cortex-M0+

---

**Quick Help**: Press `Ctrl+Shift+L` to open the launcher menu and explore all available options.

**Emergency Exit**: Most serial console tools use `Ctrl-A` then `X` or `K` to exit.