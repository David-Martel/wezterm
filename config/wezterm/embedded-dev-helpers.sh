#!/usr/bin/env bash
# Embedded Development Helper Scripts for WezTerm
# Source this file in your shell: source ~/.config/wezterm/embedded-dev-helpers.sh

# ============================================================================
# SERIAL CONSOLE HELPERS
# ============================================================================

# Quick serial console with common baud rates
serial() {
    local device="${1:-/dev/ttyUSB0}"
    local baud="${2:-115200}"
    local tool="${3:-picocom}"

    case "$tool" in
        picocom)
            picocom -b "$baud" "$device"
            ;;
        screen)
            screen "$device" "$baud"
            ;;
        minicom)
            minicom -D "$device" -b "$baud"
            ;;
        *)
            echo "Unknown tool: $tool"
            echo "Usage: serial [device] [baud] [tool]"
            echo "Tools: picocom, screen, minicom"
            return 1
            ;;
    esac
}

# Serial console with automatic logging
serial-log() {
    local device="${1:-/dev/ttyUSB0}"
    local baud="${2:-115200}"
    local logfile="serial-$(date +%Y%m%d-%H%M%S).log"

    echo "Logging to: $logfile"
    picocom -b "$baud" "$device" | tee "$logfile"
}

# List all serial devices
list-serial() {
    echo "=== Serial Devices ==="
    if [[ -d /dev ]]; then
        ls -l /dev/tty{USB,ACM}* 2>/dev/null || echo "No serial devices found"
    fi

    echo ""
    echo "=== USB Serial Devices ==="
    lsusb | grep -i 'serial\|uart\|ftdi\|cp210\|ch340\|prolific' || echo "No USB serial devices found"

    echo ""
    echo "=== Device Permissions ==="
    if groups | grep -q dialout; then
        echo "✓ User is in dialout group"
    else
        echo "✗ User NOT in dialout group"
        echo "  Fix: sudo usermod -a -G dialout $USER (then logout/login)"
    fi
}

# Monitor serial device connections
monitor-serial() {
    watch -n 1 'ls -lh /dev/tty{USB,ACM}* 2>/dev/null; echo; lsusb | grep -i "serial\|uart\|ftdi\|cp210\|ch340"'
}

# Fix serial permissions
fix-serial-permissions() {
    echo "Adding $USER to dialout group..."
    sudo usermod -a -G dialout "$USER"
    echo "Done. Please logout and login for changes to take effect."
    echo "Or run: newgrp dialout"
}

# ============================================================================
# BUILD SYSTEM HELPERS
# ============================================================================

# Quick build with parallel jobs
build() {
    local jobs=$(nproc)
    if [[ -f "Makefile" ]]; then
        make -j"$jobs" "$@"
    elif [[ -f "CMakeLists.txt" ]]; then
        cmake --build build -j"$jobs" "$@"
    elif [[ -f "platformio.ini" ]]; then
        pio run "$@"
    elif [[ -f "Cargo.toml" ]]; then
        cargo build --release "$@"
    else
        echo "No build system detected (Makefile, CMakeLists.txt, platformio.ini, Cargo.toml)"
        return 1
    fi
}

# Clean build
clean-build() {
    echo "Cleaning..."
    if [[ -f "Makefile" ]]; then
        make clean
    elif [[ -d "build" ]]; then
        rm -rf build && mkdir build
    elif [[ -f "platformio.ini" ]]; then
        pio run -t clean
    elif [[ -f "Cargo.toml" ]]; then
        cargo clean
    fi

    echo "Building..."
    build
}

# Flash firmware to device
flash() {
    if [[ -f "Makefile" ]]; then
        make flash "$@"
    elif [[ -f "platformio.ini" ]]; then
        pio run -t upload "$@"
    else
        echo "No flash target found"
        echo "Usage examples:"
        echo "  make flash"
        echo "  pio run -t upload"
        echo "  openocd -f interface/stlink.cfg -f target/stm32f4x.cfg -c 'program firmware.bin 0x08000000 verify reset exit'"
        return 1
    fi
}

# Build and flash in one command
build-flash() {
    build && flash
}

# ============================================================================
# DEBUGGING HELPERS
# ============================================================================

# Start OpenOCD with common configurations
openocd-start() {
    local interface="${1:-stlink}"
    local target="${2:-stm32f4x}"

    echo "Starting OpenOCD with $interface and $target"
    openocd -f "interface/${interface}.cfg" -f "target/${target}.cfg"
}

# Start GDB and connect to OpenOCD
gdb-connect() {
    local toolchain="${1:-arm-none-eabi}"
    local elf_file="${2:-build/firmware.elf}"
    local port="${3:-3333}"

    if [[ ! -f "$elf_file" ]]; then
        echo "Error: ELF file not found: $elf_file"
        return 1
    fi

    "${toolchain}-gdb" -ex "target remote localhost:$port" "$elf_file"
}

# Quick GDB commands file
gdb-init() {
    cat > .gdbinit <<'EOF'
# GDB initialization for embedded development
target remote localhost:3333
monitor reset halt
load
monitor reset init
break main
continue
EOF
    echo "Created .gdbinit with standard embedded startup sequence"
}

# ============================================================================
# CROSS-COMPILATION HELPERS
# ============================================================================

# Set up ARM toolchain environment
setup-arm() {
    export PATH="/usr/local/arm-none-eabi/bin:$PATH"
    export CROSS_COMPILE="arm-none-eabi-"
    export CC="arm-none-eabi-gcc"
    export CXX="arm-none-eabi-g++"
    export AR="arm-none-eabi-ar"
    export AS="arm-none-eabi-as"
    export LD="arm-none-eabi-ld"
    echo "✓ ARM toolchain configured"
    arm-none-eabi-gcc --version | head -n1
}

# Set up RISC-V toolchain environment
setup-riscv() {
    export PATH="/opt/riscv/bin:$PATH"
    export CROSS_COMPILE="riscv64-unknown-elf-"
    export CC="riscv64-unknown-elf-gcc"
    export CXX="riscv64-unknown-elf-g++"
    echo "✓ RISC-V toolchain configured"
    riscv64-unknown-elf-gcc --version | head -n1
}

# Set up AVR toolchain environment
setup-avr() {
    export PATH="/usr/local/avr/bin:$PATH"
    export CC="avr-gcc"
    export CXX="avr-g++"
    echo "✓ AVR toolchain configured"
    avr-gcc --version | head -n1
}

# Check installed toolchains
check-toolchains() {
    echo "=== Checking Installed Toolchains ==="

    echo -n "ARM (arm-none-eabi-gcc): "
    if command -v arm-none-eabi-gcc &>/dev/null; then
        arm-none-eabi-gcc --version | head -n1
    else
        echo "Not installed"
    fi

    echo -n "RISC-V (riscv64-unknown-elf-gcc): "
    if command -v riscv64-unknown-elf-gcc &>/dev/null; then
        riscv64-unknown-elf-gcc --version | head -n1
    else
        echo "Not installed"
    fi

    echo -n "AVR (avr-gcc): "
    if command -v avr-gcc &>/dev/null; then
        avr-gcc --version | head -n1
    else
        echo "Not installed"
    fi

    echo -n "PlatformIO (pio): "
    if command -v pio &>/dev/null; then
        pio --version
    else
        echo "Not installed"
    fi
}

# ============================================================================
# BINARY FILE HELPERS
# ============================================================================

# Analyze ELF file
analyze-elf() {
    local elf_file="${1:-firmware.elf}"
    local toolchain="${2:-arm-none-eabi}"

    if [[ ! -f "$elf_file" ]]; then
        echo "Error: File not found: $elf_file"
        return 1
    fi

    echo "=== File Information ==="
    file "$elf_file"

    echo ""
    echo "=== Size Information ==="
    "${toolchain}-size" "$elf_file"

    echo ""
    echo "=== Section Headers ==="
    "${toolchain}-objdump" -h "$elf_file"

    echo ""
    echo "=== Symbol Table (top 20) ==="
    "${toolchain}-nm" -S --size-sort "$elf_file" | tail -n 20
}

# Convert ELF to binary formats
convert-elf() {
    local elf_file="${1:-firmware.elf}"
    local toolchain="${2:-arm-none-eabi}"

    if [[ ! -f "$elf_file" ]]; then
        echo "Error: File not found: $elf_file"
        return 1
    fi

    local base="${elf_file%.elf}"

    echo "Converting $elf_file..."

    # Binary format
    "${toolchain}-objcopy" -O binary "$elf_file" "${base}.bin"
    echo "✓ Created ${base}.bin"

    # Intel HEX format
    "${toolchain}-objcopy" -O ihex "$elf_file" "${base}.hex"
    echo "✓ Created ${base}.hex"

    # Show sizes
    ls -lh "${base}".{elf,bin,hex}
}

# View binary file in hex
hexview() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        echo "Error: File not found: $file"
        return 1
    fi

    if command -v hexyl &>/dev/null; then
        hexyl "$file"
    elif command -v xxd &>/dev/null; then
        xxd "$file" | less
    else
        hexdump -C "$file" | less
    fi
}

# Calculate CRC32 of binary file
crc32() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        echo "Error: File not found: $file"
        return 1
    fi

    if command -v crc32 &>/dev/null; then
        crc32 "$file"
    else
        python3 -c "import zlib; print(hex(zlib.crc32(open('$file','rb').read())))"
    fi
}

# ============================================================================
# LOG PARSING HELPERS
# ============================================================================

# Filter logs by severity
log-errors() {
    local logfile="${1:--}"  # Default to stdin
    grep --color=always -i 'error\|fail\|critical' "$logfile"
}

log-warnings() {
    local logfile="${1:--}"
    grep --color=always -i 'warn\|warning' "$logfile"
}

log-debug() {
    local logfile="${1:--}"
    grep --color=always -i 'debug' "$logfile"
}

# Add timestamps to log stream
log-timestamp() {
    if command -v ts &>/dev/null; then
        ts "[%Y-%m-%d %H:%M:%S]"
    else
        while IFS= read -r line; do
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] $line"
        done
    fi
}

# Colorize log levels
log-colorize() {
    sed -e 's/\(ERROR\|CRITICAL\|FATAL\)/\o033[1;31m\1\o033[0m/g' \
        -e 's/\(WARN\|WARNING\)/\o033[1;33m\1\o033[0m/g' \
        -e 's/\(INFO\)/\o033[1;32m\1\o033[0m/g' \
        -e 's/\(DEBUG\)/\o033[1;34m\1\o033[0m/g'
}

# ============================================================================
# DEVICE MANAGEMENT HELPERS
# ============================================================================

# Find device by vendor ID
find-device() {
    local vendor_id="$1"
    if [[ -z "$vendor_id" ]]; then
        echo "Usage: find-device <vendor_id>"
        echo "Example: find-device 0403  # FTDI devices"
        return 1
    fi

    lsusb | grep -i "$vendor_id"
}

# Show device information
device-info() {
    local device="${1:-/dev/ttyUSB0}"

    echo "=== Device Information ==="
    echo "Device: $device"

    if [[ -e "$device" ]]; then
        ls -l "$device"

        echo ""
        echo "=== USB Information ==="
        udevadm info "$device" | grep -E 'ID_VENDOR|ID_MODEL|ID_SERIAL|ID_BUS'
    else
        echo "Device not found: $device"
    fi
}

# Reset USB device
reset-usb() {
    local device="${1:-/dev/ttyUSB0}"

    # Get USB bus and device number
    local usb_path=$(udevadm info "$device" 2>/dev/null | grep DEVPATH | cut -d= -f2)

    if [[ -n "$usb_path" ]]; then
        echo "Resetting USB device: $usb_path"
        echo "$usb_path" | sudo tee /sys/bus/usb/drivers/usb/unbind
        sleep 1
        echo "$usb_path" | sudo tee /sys/bus/usb/drivers/usb/bind
        echo "Device reset complete"
    else
        echo "Could not find USB path for $device"
    fi
}

# ============================================================================
# PROJECT INITIALIZATION HELPERS
# ============================================================================

# Initialize embedded project structure
init-embedded-project() {
    local project_name="${1:-embedded-project}"

    mkdir -p "$project_name"/{src,inc,build,tools,docs}

    cat > "$project_name/Makefile" <<'EOF'
# Embedded Project Makefile

PROJECT = firmware
BUILD_DIR = build

# Toolchain
CROSS_COMPILE ?= arm-none-eabi-
CC = $(CROSS_COMPILE)gcc
OBJCOPY = $(CROSS_COMPILE)objcopy
SIZE = $(CROSS_COMPILE)size

# Flags
CFLAGS = -Wall -Wextra -O2 -g

# Sources
SRCS = $(wildcard src/*.c)
OBJS = $(SRCS:src/%.c=$(BUILD_DIR)/%.o)

all: $(BUILD_DIR)/$(PROJECT).elf

$(BUILD_DIR)/%.o: src/%.c | $(BUILD_DIR)
	$(CC) $(CFLAGS) -c $< -o $@

$(BUILD_DIR)/$(PROJECT).elf: $(OBJS)
	$(CC) $(CFLAGS) $^ -o $@
	$(SIZE) $@
	$(OBJCOPY) -O binary $@ $(BUILD_DIR)/$(PROJECT).bin
	$(OBJCOPY) -O ihex $@ $(BUILD_DIR)/$(PROJECT).hex

$(BUILD_DIR):
	mkdir -p $@

clean:
	rm -rf $(BUILD_DIR)

flash:
	@echo "Implement flash target for your device"

.PHONY: all clean flash
EOF

    cat > "$project_name/src/main.c" <<'EOF'
#include <stdint.h>

int main(void) {
    // Hardware initialization

    while (1) {
        // Main loop
    }

    return 0;
}
EOF

    cat > "$project_name/README.md" <<EOF
# $project_name

Embedded systems project

## Build
\`\`\`bash
make
\`\`\`

## Flash
\`\`\`bash
make flash
\`\`\`

## Debug
\`\`\`bash
openocd -f interface/stlink.cfg -f target/stm32f4x.cfg &
arm-none-eabi-gdb build/firmware.elf
\`\`\`
EOF

    echo "✓ Created embedded project: $project_name"
    ls -R "$project_name"
}

# ============================================================================
# HELP SYSTEM
# ============================================================================

embedded-help() {
    cat <<'EOF'
=== Embedded Development Helper Functions ===

SERIAL CONSOLE:
  serial [device] [baud] [tool]    - Open serial console
  serial-log [device] [baud]       - Serial console with logging
  list-serial                      - List all serial devices
  monitor-serial                   - Monitor device connections
  fix-serial-permissions           - Add user to dialout group

BUILD SYSTEM:
  build [args]                     - Smart build (auto-detect system)
  clean-build                      - Clean and rebuild
  flash [args]                     - Flash firmware to device
  build-flash                      - Build and flash in one command

DEBUGGING:
  openocd-start [interface] [target] - Start OpenOCD server
  gdb-connect [toolchain] [elf] [port] - Connect GDB to OpenOCD
  gdb-init                         - Create .gdbinit with defaults

TOOLCHAINS:
  setup-arm                        - Configure ARM toolchain
  setup-riscv                      - Configure RISC-V toolchain
  setup-avr                        - Configure AVR toolchain
  check-toolchains                 - Check installed toolchains

BINARY FILES:
  analyze-elf [file] [toolchain]   - Analyze ELF file
  convert-elf [file] [toolchain]   - Convert ELF to bin/hex
  hexview [file]                   - View binary in hex
  crc32 [file]                     - Calculate CRC32

LOG PARSING:
  log-errors [file]                - Filter error messages
  log-warnings [file]              - Filter warnings
  log-debug [file]                 - Filter debug messages
  log-timestamp                    - Add timestamps (pipe)
  log-colorize                     - Colorize log levels (pipe)

DEVICE MANAGEMENT:
  find-device [vendor_id]          - Find USB device by vendor ID
  device-info [device]             - Show device information
  reset-usb [device]               - Reset USB device

PROJECT SETUP:
  init-embedded-project [name]     - Initialize new project

HELP:
  embedded-help                    - Show this help message

Examples:
  serial /dev/ttyUSB0 115200
  build-flash
  openocd-start stlink stm32f4x
  analyze-elf build/firmware.elf arm-none-eabi
  tail -f serial.log | log-colorize | log-timestamp
EOF
}

# ============================================================================
# INITIALIZATION
# ============================================================================

echo "✓ Embedded development helpers loaded"
echo "  Type 'embedded-help' for available commands"