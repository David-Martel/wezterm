# WezTerm Embedded Development Environment - Setup Guide

Complete embedded systems development configuration for WezTerm with keybindings, helpers, and tooling.

## 📦 What's Included

This setup provides a comprehensive embedded development environment with:

1. **WezTerm Configuration** (`embedded-dev-config.lua`)
   - 40+ specialized keybindings
   - Pre-configured launcher menu
   - Optimized for serial console, debugging, and build workflows

2. **Shell Helper Functions** (`embedded-dev-helpers.sh`)
   - 30+ convenience functions
   - Smart build system detection
   - Serial port management
   - Binary file analysis tools

3. **Automated Setup** (`setup-embedded-dev.sh`)
   - Prerequisite checking
   - Configuration integration
   - Permission fixes
   - Optional tool installation

4. **Documentation** (`EMBEDDED_DEV_KEYBINDINGS.md`)
   - Complete keybinding reference
   - Workflow examples
   - Common gotchas and solutions

## 🚀 Quick Start

### 1. Run Setup Script

```bash
# From WSL or Linux
cd ~/.config/wezterm
./setup-embedded-dev.sh
```

The setup script will:
- Check installed tools and suggest missing ones
- Integrate configuration into your wezterm.lua
- Add helper functions to your shell RC file
- Fix serial port permissions (dialout group)
- Optionally create example projects

### 2. Restart Environment

```bash
# Reload shell configuration
source ~/.bashrc  # or source ~/.zshrc

# Restart WezTerm to load new configuration
```

### 3. Verify Installation

```bash
# Check helper functions
embedded-help

# Check toolchains
check-toolchains

# List serial devices
list-serial
```

## 🎯 Key Features by Workflow

### Serial Console Monitoring

**Keybindings:**
- `Alt+F1` - Quick serial console
- `Alt+F2` - Dual serial monitors
- `Alt+F3` - Serial with logging
- `Alt+F4` - Detachable screen session

**Shell Commands:**
```bash
serial /dev/ttyUSB0 115200        # Quick serial connection
serial-log                        # Serial with automatic logging
list-serial                       # Show all serial devices
monitor-serial                    # Live device monitoring
```

### Build Systems

**Keybindings:**
- `Alt+F5` - Quick build
- `Alt+F6` - Clean build
- `Alt+F7` - Flash firmware
- `Alt+F8` - Build and flash

**Shell Commands:**
```bash
build                  # Smart build (auto-detects Makefile/CMake/PlatformIO/Cargo)
clean-build           # Clean and rebuild
flash                 # Flash to device
build-flash           # Build and flash in one command
```

**Supported Build Systems:**
- GNU Make
- CMake
- PlatformIO
- Cargo (Rust embedded)

### Cross-Compilation

**Keybindings:**
- `Ctrl+Alt+A` - ARM environment
- `Ctrl+Alt+R` - RISC-V environment
- `Ctrl+Alt+V` - AVR environment
- `Ctrl+Alt+P` - PlatformIO environment

**Shell Commands:**
```bash
setup-arm             # Configure ARM toolchain
setup-riscv           # Configure RISC-V toolchain
setup-avr             # Configure AVR toolchain
check-toolchains      # Verify installed toolchains
```

### Debugging

**Keybindings:**
- `Alt+F9` - Start OpenOCD
- `Alt+F10` - Connect GDB
- `Alt+F11` - J-Link GDB server
- `Alt+F12` - Full 4-pane dev dashboard

**Shell Commands:**
```bash
openocd-start stlink stm32f4x     # Start OpenOCD server
gdb-connect arm-none-eabi         # Connect GDB to OpenOCD
gdb-init                          # Create .gdbinit with defaults
```

**Supported Debug Interfaces:**
- ST-Link (STM32)
- J-Link (Segger)
- CMSIS-DAP
- OpenOCD compatible adapters

### Binary File Analysis

**Keybindings:**
- `Ctrl+Alt+Shift+H` - Hexdump viewer
- `Ctrl+Alt+Shift+B` - Binary info (sizes, sections)
- `Ctrl+Alt+Shift+C` - Convert ELF to bin/hex

**Shell Commands:**
```bash
analyze-elf firmware.elf arm-none-eabi    # Detailed ELF analysis
convert-elf firmware.elf                  # Convert to bin/hex formats
hexview firmware.bin                      # Hex viewer
crc32 firmware.bin                        # Calculate CRC32
```

### Log Parsing

**Keybindings:**
- `Ctrl+Alt+1` - Filter errors
- `Ctrl+Alt+2` - Filter warnings
- `Ctrl+Alt+3` - Filter debug
- `Ctrl+Alt+4` - Add timestamps

**Shell Commands:**
```bash
# Filter logs
tail -f serial.log | log-errors
tail -f serial.log | log-warnings

# Add timestamps and colors
tail -f serial.log | log-timestamp | log-colorize
```

## 🔧 Required Tools

### Essential (Must Have)
```bash
# Ubuntu/Debian
sudo apt install build-essential git

# Serial console tools
sudo apt install picocom screen minicom
```

### Cross-Compilation Toolchains
```bash
# ARM Cortex-M (STM32, NRF52, etc.)
sudo apt install gcc-arm-none-eabi binutils-arm-none-eabi gdb-multiarch

# AVR (Arduino)
sudo apt install gcc-avr avr-libc binutils-avr

# RISC-V (optional)
# Download from: https://github.com/riscv-collab/riscv-gnu-toolchain
```

### Debugging Tools
```bash
# OpenOCD for JTAG/SWD debugging
sudo apt install openocd

# J-Link (if using Segger probes)
# Download from: https://www.segger.com/downloads/jlink/
```

### Optional But Recommended
```bash
# Modern hex viewer
cargo install hexyl

# Timestamp utility
sudo apt install moreutils

# PlatformIO
pip3 install --user platformio

# Binary analysis
sudo apt install xxd hexdump
```

## 🐛 Common Issues and Fixes

### Issue: Permission Denied on Serial Port

**Symptoms:**
```bash
picocom: FATAL: cannot open /dev/ttyUSB0: Permission denied
```

**Solution:**
```bash
# Add user to dialout group
sudo usermod -a -G dialout $USER

# Logout and login, or:
newgrp dialout

# Verify
groups | grep dialout
```

**Quick Fix via Helper:**
```bash
fix-serial-permissions
```

### Issue: No Serial Devices Found

**Symptoms:**
```bash
ls: cannot access '/dev/ttyUSB*': No such file or directory
```

**Solution:**
```bash
# Check if device is connected
lsusb

# Look for serial adapters (FTDI, CP210x, CH340, Prolific)
lsusb | grep -i 'serial\|uart\|ftdi\|cp210\|ch340'

# Check kernel messages
dmesg | tail -20

# Monitor device connections
monitor-serial
```

### Issue: OpenOCD Can't Find Interface

**Symptoms:**
```
Error: unable to find a matching CMSIS-DAP device
```

**Solution:**
```bash
# List available OpenOCD configs
ls /usr/share/openocd/scripts/interface/
ls /usr/share/openocd/scripts/target/

# Check USB connection
lsusb | grep -i 'stlink\|jlink\|cmsis'

# Try with sudo (permission issue)
sudo openocd -f interface/stlink.cfg -f target/stm32f4x.cfg

# Add udev rules (permanent fix)
sudo cp /usr/share/openocd/contrib/60-openocd.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Issue: GDB Connection Refused

**Symptoms:**
```
target remote localhost:3333
localhost:3333: Connection refused
```

**Solution:**
```bash
# Ensure OpenOCD is running first
openocd-start stlink stm32f4x

# Check if port is listening
netstat -tuln | grep 3333

# If OpenOCD is running, try:
gdb-connect arm-none-eabi build/firmware.elf
```

### Issue: Build Fails with Toolchain Errors

**Symptoms:**
```
arm-none-eabi-gcc: command not found
```

**Solution:**
```bash
# Check installed toolchains
check-toolchains

# Set up environment
setup-arm

# Verify toolchain is in PATH
which arm-none-eabi-gcc
arm-none-eabi-gcc --version

# If not installed:
sudo apt install gcc-arm-none-eabi
```

## 📚 Example Workflows

### Workflow 1: First Time Device Connection

```bash
# 1. Connect device and identify it
list-serial
# Shows: /dev/ttyUSB0

# 2. Check device permissions
device-info /dev/ttyUSB0

# 3. Open serial console
serial /dev/ttyUSB0 115200

# Or use keybinding: Alt+F1
```

### Workflow 2: Build and Flash Firmware

```bash
# 1. Set up toolchain
setup-arm

# 2. Build project
build

# 3. Flash to device
flash

# Or use keybinding: Alt+F8 (build and flash)
```

### Workflow 3: Debug Session

```bash
# 1. Build with debug symbols
build

# 2. Start OpenOCD in background (Alt+F9)
openocd-start stlink stm32f4x &

# 3. Connect GDB (Alt+F10)
gdb-connect arm-none-eabi build/firmware.elf

# In GDB:
(gdb) monitor reset halt
(gdb) load
(gdb) break main
(gdb) continue
```

### Workflow 4: Multi-Device Testing

```bash
# Press Alt+F12 for 4-pane dashboard
# Configure each pane:

# Pane 1 (top-left): Build
cd ~/project && build

# Pane 2 (top-right): Device 1 serial
serial /dev/ttyUSB0 115200

# Pane 3 (bottom-left): Device 2 serial
serial /dev/ttyUSB1 115200

# Pane 4 (bottom-right): Log monitoring
tail -f serial.log | log-colorize
```

### Workflow 5: Log Analysis

```bash
# Serial console with logging (Alt+F3)
serial-log /dev/ttyUSB0 115200

# In another pane, analyze logs in real-time:
tail -f serial-*.log | log-errors
tail -f serial-*.log | log-timestamp | log-colorize

# Or use keybindings:
# Ctrl+Alt+1 for errors
# Ctrl+Alt+2 for warnings
# Ctrl+Alt+4 for timestamps
```

## 🎓 Learning Resources

### Getting Started Guides
- **ARM Cortex-M**: https://developer.arm.com/documentation/
- **OpenOCD**: https://openocd.org/doc/html/index.html
- **GDB for Embedded**: https://sourceware.org/gdb/documentation/

### Hardware Platforms
- **STM32 (ARM)**: Cheap, powerful, excellent ecosystem
- **ESP32 (Xtensa/RISC-V)**: WiFi/Bluetooth, great community
- **Arduino (AVR)**: Beginner-friendly, huge library support
- **Raspberry Pi Pico (RP2040)**: Dual-core ARM, great documentation

### Development Boards to Try
1. **STM32F4 Discovery** (~$20) - ARM Cortex-M4, ST-Link built-in
2. **ESP32 DevKit** (~$5) - WiFi, Bluetooth, USB programming
3. **Arduino Uno** (~$10) - Classic AVR, beginner-friendly
4. **Raspberry Pi Pico** (~$4) - RP2040, excellent for learning

## 🔄 Updating Configuration

### Modify Keybindings

Edit `~/.config/wezterm/embedded-dev-config.lua`:
```lua
-- Add custom keybinding
{
  key = 'F5',
  action = wezterm.action_callback(function(window, pane)
    pane:send_text('my-custom-build-command\n')
  end),
  mods = 'ALT|SHIFT',
}
```

### Add Helper Functions

Edit `~/.config/wezterm/embedded-dev-helpers.sh`:
```bash
# Custom build function
my-build() {
    echo "Running custom build..."
    make -j$(nproc) TARGET=custom
}
```

Then reload:
```bash
source ~/.config/wezterm/embedded-dev-helpers.sh
```

### Add Launcher Menu Items

Edit launcher menu in `embedded-dev-config.lua`:
```lua
{
  label = '🎯 My Custom Target',
  args = { 'bash', '-c', 'my-custom-command' },
}
```

## 📋 Checklist: Full Setup

- [ ] Run `setup-embedded-dev.sh`
- [ ] Install serial console tools (picocom, screen)
- [ ] Install cross-compilation toolchains
- [ ] Install debugging tools (OpenOCD, GDB)
- [ ] Fix serial permissions (dialout group)
- [ ] Install optional tools (hexyl, moreutils, PlatformIO)
- [ ] Restart WezTerm
- [ ] Reload shell configuration
- [ ] Test keybindings (Alt+F1, Alt+F12)
- [ ] Test helper functions (`embedded-help`)
- [ ] Create test project (`init-embedded-project`)
- [ ] Test serial connection to device
- [ ] Test build and flash workflow
- [ ] Test debugging workflow

## 🆘 Getting Help

### Built-in Help
```bash
# Show all helper functions
embedded-help

# Check toolchains
check-toolchains

# List serial devices
list-serial
```

### Quick Reference
```bash
# View keybindings reference
less ~/.config/wezterm/EMBEDDED_DEV_KEYBINDINGS.md

# View this setup guide
less ~/.config/wezterm/EMBEDDED_DEV_SETUP.md
```

### Community Resources
- **WezTerm**: https://wezfurlong.org/wezterm/
- **OpenOCD**: https://openocd.org/
- **PlatformIO**: https://community.platformio.org/
- **r/embedded**: https://reddit.com/r/embedded

## 🚀 Next Steps

1. **Test Basic Workflow**: Connect a device and open serial console (Alt+F1)
2. **Try Dashboard**: Press Alt+F12 to see the full 4-pane layout
3. **Explore Helpers**: Run `embedded-help` to see all shell functions
4. **Create Project**: Use `init-embedded-project myproject` to start new project
5. **Customize**: Modify keybindings and helpers to match your workflow

## 📝 Notes

- All keybindings use Alt, Ctrl+Alt, or Ctrl+Shift modifiers to avoid conflicts
- Helper functions follow consistent naming: `<category>-<action>`
- Configuration is modular and can be extended without breaking existing setup
- Logs are saved with timestamps in current directory
- Serial console tools use `Ctrl+A` then `X` or `K` to exit

---

**Happy Embedded Development!** 🎉

For issues or improvements, check the configuration files in `~/.config/wezterm/`