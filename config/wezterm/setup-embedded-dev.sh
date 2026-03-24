#!/usr/bin/env bash
# Setup script for embedded development environment in WezTerm

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/embedded-dev-config.lua"
HELPERS_FILE="$SCRIPT_DIR/embedded-dev-helpers.sh"
WEZTERM_CONFIG="$HOME/.config/wezterm/wezterm.lua"

echo "=== WezTerm Embedded Development Setup ==="
echo ""

# ============================================================================
# Check Prerequisites
# ============================================================================

echo "Checking prerequisites..."

check_command() {
    if command -v "$1" &>/dev/null; then
        echo "✓ $1 installed"
        return 0
    else
        echo "✗ $1 not found"
        return 1
    fi
}

# Essential tools
MISSING_TOOLS=0

echo ""
echo "=== Essential Tools ==="
check_command "make" || MISSING_TOOLS=$((MISSING_TOOLS + 1))
check_command "gcc" || MISSING_TOOLS=$((MISSING_TOOLS + 1))

echo ""
echo "=== Serial Console Tools ==="
check_command "picocom" || echo "  Install: sudo apt install picocom"
check_command "screen" || echo "  Install: sudo apt install screen"
check_command "minicom" || echo "  Install: sudo apt install minicom"

echo ""
echo "=== Cross-Compilation Toolchains ==="
check_command "arm-none-eabi-gcc" || echo "  Install: sudo apt install gcc-arm-none-eabi"
check_command "avr-gcc" || echo "  Install: sudo apt install gcc-avr"
check_command "riscv64-unknown-elf-gcc" || echo "  Optional RISC-V toolchain"

echo ""
echo "=== Debugging Tools ==="
check_command "openocd" || echo "  Install: sudo apt install openocd"
check_command "gdb-multiarch" || echo "  Install: sudo apt install gdb-multiarch"

echo ""
echo "=== Binary Analysis Tools ==="
check_command "hexdump" || MISSING_TOOLS=$((MISSING_TOOLS + 1))
check_command "xxd" || echo "  Install: sudo apt install xxd"
check_command "hexyl" || echo "  Install: cargo install hexyl (optional)"

echo ""
echo "=== Build Systems ==="
check_command "cmake" || echo "  Install: sudo apt install cmake"
check_command "pio" || echo "  Install: pip install platformio"

echo ""
echo "=== Log Tools ==="
check_command "ts" || echo "  Install: sudo apt install moreutils (for timestamp)"

# ============================================================================
# Configure WezTerm
# ============================================================================

echo ""
echo "=== Configuring WezTerm ==="

# Backup existing config if it exists
if [[ -f "$WEZTERM_CONFIG" ]]; then
    echo "Backing up existing wezterm.lua..."
    cp "$WEZTERM_CONFIG" "$WEZTERM_CONFIG.backup.$(date +%Y%m%d-%H%M%S)"
fi

# Check if embedded config is already included
if [[ -f "$WEZTERM_CONFIG" ]] && grep -q "embedded-dev-config" "$WEZTERM_CONFIG"; then
    echo "✓ Embedded dev config already included in wezterm.lua"
else
    echo "Adding embedded dev config to wezterm.lua..."

    # Create or append to wezterm.lua
    cat >> "$WEZTERM_CONFIG" <<'EOF'

-- ============================================================================
-- Embedded Development Configuration
-- ============================================================================
local embedded_config = require('embedded-dev-config')

-- Merge with existing config or use embedded config directly
if config then
    -- Merge keys
    for _, key in ipairs(embedded_config.keys or {}) do
        table.insert(config.keys, key)
    end
    -- Merge launch menu
    for _, item in ipairs(embedded_config.launch_menu or {}) do
        table.insert(config.launch_menu, item)
    end
else
    config = embedded_config
end
EOF

    echo "✓ Added embedded dev config to wezterm.lua"
fi

# ============================================================================
# Configure Shell Integration
# ============================================================================

echo ""
echo "=== Configuring Shell Integration ==="

# Detect shell
SHELL_RC=""
if [[ -n "$BASH_VERSION" ]]; then
    SHELL_RC="$HOME/.bashrc"
elif [[ -n "$ZSH_VERSION" ]]; then
    SHELL_RC="$HOME/.zshrc"
else
    SHELL_RC="$HOME/.bashrc"
fi

echo "Detected shell config: $SHELL_RC"

# Add helper functions to shell RC
if grep -q "embedded-dev-helpers.sh" "$SHELL_RC" 2>/dev/null; then
    echo "✓ Helper functions already sourced in $SHELL_RC"
else
    echo "Adding helper functions to $SHELL_RC..."
    cat >> "$SHELL_RC" <<EOF

# ============================================================================
# Embedded Development Helpers
# ============================================================================
if [[ -f "$HELPERS_FILE" ]]; then
    source "$HELPERS_FILE"
fi
EOF
    echo "✓ Added helper functions to $SHELL_RC"
fi

# ============================================================================
# Check Serial Permissions
# ============================================================================

echo ""
echo "=== Checking Serial Port Permissions ==="

if groups | grep -q dialout; then
    echo "✓ User $USER is in dialout group"
else
    echo "✗ User $USER is NOT in dialout group"
    echo ""
    read -p "Add $USER to dialout group? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        sudo usermod -a -G dialout "$USER"
        echo "✓ Added $USER to dialout group"
        echo "⚠ Please logout and login for changes to take effect"
    fi
fi

# ============================================================================
# Install Optional Tools
# ============================================================================

echo ""
echo "=== Optional Tool Installation ==="

install_optional_tool() {
    local tool="$1"
    local install_cmd="$2"

    if command -v "$tool" &>/dev/null; then
        echo "✓ $tool already installed"
        return 0
    fi

    read -p "Install $tool? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Installing $tool..."
        eval "$install_cmd"
        echo "✓ $tool installed"
    fi
}

# Modern hex viewer
if ! command -v hexyl &>/dev/null && command -v cargo &>/dev/null; then
    install_optional_tool "hexyl" "cargo install hexyl"
fi

# Timestamp utility
if ! command -v ts &>/dev/null; then
    install_optional_tool "ts (moreutils)" "sudo apt-get install -y moreutils"
fi

# PlatformIO
if ! command -v pio &>/dev/null && command -v pip3 &>/dev/null; then
    install_optional_tool "platformio" "pip3 install --user platformio"
fi

# ============================================================================
# Create Example Projects
# ============================================================================

echo ""
echo "=== Example Projects ==="

read -p "Create example embedded project? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    EXAMPLE_DIR="$HOME/embedded-example"
    if [[ -d "$EXAMPLE_DIR" ]]; then
        echo "Example project already exists at $EXAMPLE_DIR"
    else
        # Source helpers to get init function
        source "$HELPERS_FILE"
        cd "$HOME"
        init-embedded-project "embedded-example"
        echo "✓ Created example project at $EXAMPLE_DIR"
    fi
fi

# ============================================================================
# Verification
# ============================================================================

echo ""
echo "=== Verification ==="

# Test WezTerm config
echo "Testing WezTerm configuration..."
if wezterm --version &>/dev/null; then
    echo "✓ WezTerm is installed"

    # Try to validate config
    if wezterm show-keys &>/dev/null; then
        echo "✓ WezTerm config is valid"
    else
        echo "⚠ Could not validate WezTerm config (this may be normal)"
    fi
else
    echo "✗ WezTerm not found. Please install WezTerm first."
fi

# Test helper functions
echo ""
echo "Testing helper functions..."
source "$HELPERS_FILE" &>/dev/null && echo "✓ Helper functions loaded successfully"

# ============================================================================
# Summary
# ============================================================================

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "1. Restart WezTerm to load new configuration"
echo "2. Open a new shell to load helper functions"
echo "3. If you added yourself to dialout group, logout and login"
echo ""
echo "Quick start:"
echo "  - Press Alt+F1 for serial console"
echo "  - Press Alt+F12 for full embedded dev dashboard"
echo "  - Press Ctrl+Shift+L for launcher menu"
echo "  - Type 'embedded-help' for available shell commands"
echo ""
echo "Documentation:"
echo "  - Keybindings: $SCRIPT_DIR/EMBEDDED_DEV_KEYBINDINGS.md"
echo "  - Configuration: $SCRIPT_DIR/embedded-dev-config.lua"
echo "  - Helper functions: $SCRIPT_DIR/embedded-dev-helpers.sh"
echo ""

if [[ $MISSING_TOOLS -gt 0 ]]; then
    echo "⚠ Warning: Some essential tools are missing"
    echo "  Review the tool list above and install missing packages"
fi

echo ""
echo "Happy embedded hacking! 🚀"