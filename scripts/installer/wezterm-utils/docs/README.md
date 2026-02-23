# WezTerm Utilities System

**Version**: 1.0.0
**Status**: Production Ready
**Platform**: Windows 10/11, Linux, macOS

## Overview

WezTerm Utilities is a comprehensive system for enhancing WezTerm terminal emulator with advanced filesystem navigation, file watching, and inter-process communication capabilities. Built with Rust for maximum performance and reliability.

## Features

### 🗂️ Filesystem Explorer (`wezterm-fs-explorer`)
- **Interactive Navigation**: Browse directories with keyboard shortcuts
- **Quick Preview**: View file contents without leaving the terminal
- **Fast Search**: Find files instantly with fuzzy matching
- **Multi-Select**: Perform batch operations on multiple files
- **Git Integration**: See git status inline with file listings

**Performance**: <50ms startup, <1ms response time

### 👁️ File Watcher (`wezterm-watch`)
- **Real-time Monitoring**: Track file changes as they happen
- **Pattern Matching**: Watch specific file types or patterns
- **Action Triggers**: Execute commands on file events
- **Recursive Watching**: Monitor entire directory trees
- **Low Overhead**: <5MB memory, minimal CPU usage

**Performance**: <100ms event detection, handles 10,000+ files

### 🔗 IPC Daemon (Optional)
- **Process Communication**: Coordinate between utilities
- **State Management**: Share state across terminal sessions
- **Event Broadcasting**: Notify all connected clients of changes
- **Secure**: Unix domain sockets (Linux/macOS) or named pipes (Windows)

**Performance**: <1ms message latency, supports 100+ concurrent connections

## Architecture

```
┌─────────────────────────────────────────────────┐
│              WezTerm Terminal                   │
│  ┌──────────────────────────────────────────┐  │
│  │         .wezterm.lua Configuration       │  │
│  │  (Lua keybindings and integration)       │  │
│  └────────────────┬─────────────────────────┘  │
│                   │                             │
│                   ▼                             │
│  ┌──────────────────────────────────────────┐  │
│  │       wezterm-utils.lua Module           │  │
│  │  • Launcher integration                  │  │
│  │  • State management                      │  │
│  │  • Keybinding setup                      │  │
│  └────────┬──────────────────┬──────────────┘  │
└───────────┼──────────────────┼─────────────────┘
            │                  │
            ▼                  ▼
    ┌───────────────┐  ┌───────────────┐
    │  fs-explorer  │  │  wezterm-watch│
    │  (Rust)       │  │  (Rust)       │
    │  • TUI        │  │  • Notify API │
    │  • Fast I/O   │  │  • Event loop │
    └───────┬───────┘  └───────┬───────┘
            │                  │
            └────────┬─────────┘
                     ▼
            ┌─────────────────┐
            │   IPC Daemon    │
            │   (Optional)    │
            │  • State sync   │
            │  • Event router │
            └─────────────────┘
```

## System Requirements

### Minimum Requirements
- **OS**: Windows 10 (1809+), Linux (kernel 4.4+), macOS 10.15+
- **RAM**: 50MB per utility
- **Disk**: 30MB for all binaries
- **WezTerm**: 20220101 or newer

### Recommended Requirements
- **OS**: Windows 11, Ubuntu 22.04+, macOS 13+
- **RAM**: 100MB for comfortable operation
- **Disk**: 100MB including logs and state
- **WezTerm**: Latest stable release

### Dependencies
- **Windows**: Visual C++ Redistributable (2015+)
- **Linux**: glibc 2.27+ or musl 1.2+
- **macOS**: System frameworks (included)

## Installation

### Quick Install (Windows)
```powershell
# Download and extract installer
cd T:\projects\wezterm-utilities-installer

# Run installer
.\install.ps1

# Validate installation
.\validate-deployment.ps1
```

### Development Install
```powershell
# Install with symlinks for active development
.\install.ps1 -Dev
```

### Advanced Options
```powershell
# Skip backup creation
.\install.ps1 -SkipBackup

# Verbose output
.\install.ps1 -Verbose

# Uninstall
.\install.ps1 -Uninstall
```

## Usage

### Filesystem Explorer

#### Launch
- **Keyboard**: `Alt+E` (default keybinding)
- **Command**: `wezterm-fs-explorer [directory]`

#### Navigation
- `↑/↓` or `j/k`: Move cursor
- `Enter`: Open file/directory
- `Backspace`: Go up one level
- `Space`: Select/deselect file
- `Ctrl+A`: Select all
- `Ctrl+D`: Deselect all
- `/`: Search files
- `q`: Quit

#### Operations
- `d`: Delete selected files
- `r`: Rename file
- `c`: Copy files
- `m`: Move files
- `n`: New file/directory
- `p`: Preview file content

### File Watcher

#### Basic Usage
```powershell
# Watch current directory
wezterm-watch .

# Watch specific directory
wezterm-watch C:\projects\myapp

# Watch with pattern
wezterm-watch . --pattern "*.rs"

# Execute command on change
wezterm-watch . --exec "cargo test"
```

#### Advanced Options
```powershell
# Recursive watching (default)
wezterm-watch . --recursive

# Non-recursive
wezterm-watch . --no-recursive

# Ignore patterns
wezterm-watch . --ignore "target" --ignore "node_modules"

# Debounce time (milliseconds)
wezterm-watch . --debounce 1000

# Verbose output
wezterm-watch . --verbose
```

## Configuration

### WezTerm Integration

Add to `.wezterm.lua`:
```lua
local wezterm = require 'wezterm'
local config = wezterm.config_builder()

-- Load WezTerm utilities
local utils = require('wezterm-utils')
utils.setup(config)

return config
```

### Custom Keybindings

```lua
local utils = require('wezterm-utils')

-- Custom filesystem explorer binding
config.keys = {
    {
        key = 'f',
        mods = 'CTRL|SHIFT',
        action = wezterm.action.SpawnCommandInNewTab {
            args = { 'wezterm-fs-explorer' }
        }
    }
}
```

### Configuration File

Location: `~/.config/wezterm/wezterm-utils-config.json`

```json
{
    "filesystem_explorer": {
        "default_directory": "~",
        "show_hidden": false,
        "sort_by": "name",
        "preview_max_size": 1048576
    },
    "file_watcher": {
        "default_debounce": 300,
        "max_events": 1000,
        "ignore_patterns": [".git", "target", "node_modules"]
    },
    "ipc": {
        "socket_path": "~/.config/wezterm/wezterm-utils.sock",
        "timeout_ms": 5000
    }
}
```

## Troubleshooting

### Common Issues

#### Filesystem Explorer Won't Launch
```powershell
# Check if binary is in PATH
where.exe wezterm-fs-explorer

# Test binary directly
wezterm-fs-explorer --version

# Check WezTerm integration
cat ~/.config/wezterm/wezterm-utils.lua
```

#### File Watcher Not Detecting Changes
```powershell
# Increase system watch limits (Linux)
sudo sysctl fs.inotify.max_user_watches=524288

# Check permissions
wezterm-watch . --verbose

# Test with simple pattern
wezterm-watch . --pattern "*.txt"
```

#### High Memory Usage
```powershell
# Check running processes
Get-Process | Where-Object {$_.ProcessName -like "*wezterm*"}

# Restart utilities
taskkill /F /IM wezterm-fs-explorer.exe
taskkill /F /IM wezterm-watch.exe
```

### Diagnostic Commands

```powershell
# Validate installation
.\validate-deployment.ps1 --Verbose

# Check binary versions
wezterm-fs-explorer --version
wezterm-watch --version

# View logs
Get-Content "$env:USERPROFILE\.config\wezterm\wezterm-utils.log" -Tail 50
```

### Getting Help

1. **Documentation**: See `docs/` directory
2. **Validation**: Run `validate-deployment.ps1`
3. **Rollback**: Use `rollback.ps1` if needed
4. **Logs**: Check `~/.config/wezterm/*.log`

## Performance

### Benchmarks (Windows 11, Ryzen 9 5900X)

| Operation | Time | Memory |
|-----------|------|--------|
| Explorer startup | 45ms | 8MB |
| Directory listing (1,000 files) | 12ms | 15MB |
| File search (10,000 files) | 180ms | 25MB |
| Watcher initialization | 85ms | 5MB |
| Event detection latency | <100ms | +2MB |
| IPC message roundtrip | 0.8ms | 3MB |

### Optimization Tips

1. **Large Directories**: Use search instead of scrolling
2. **File Watching**: Use specific patterns to reduce overhead
3. **IPC Daemon**: Only run if using multiple utilities concurrently
4. **Preview Size**: Limit preview to reasonable file sizes

## Security

### Best Practices

1. **File Permissions**: Utilities respect filesystem permissions
2. **No Elevation**: Never run as administrator unless necessary
3. **Sandboxing**: Operations are scoped to accessible directories
4. **No Network**: All operations are local (no remote access)

### Security Features

- **Path Validation**: Prevents directory traversal attacks
- **Safe Deletion**: Confirms before destructive operations
- **Process Isolation**: Each utility runs in separate process
- **Secure IPC**: Unix domain sockets with proper permissions

## Maintenance

### Updates

```powershell
# Backup current installation
.\rollback.ps1 --ListBackups

# Install new version
.\install.ps1

# Validate
.\validate-deployment.ps1
```

### Backup and Restore

```powershell
# List backups
.\rollback.ps1 --ListBackups

# Restore latest
.\rollback.ps1 --Latest

# Restore specific backup
.\rollback.ps1 -BackupTimestamp 20250130_143022
```

### Logs

Log locations:
- **Windows**: `%USERPROFILE%\.config\wezterm\*.log`
- **Linux/macOS**: `~/.config/wezterm/*.log`

Log rotation: Automatic (max 10MB, keep 5 files)

## Development

### Building from Source

```powershell
# Build filesystem explorer
cd C:\Users\david\wezterm\wezterm-fs-explorer
cargo build --release

# Build file watcher
cd C:\Users\david\wezterm\wezterm-watch
cargo build --release

# Run tests
cargo test --all-features

# Run benchmarks
cargo bench
```

### Contributing

See `CONTRIBUTING.md` for development guidelines.

## License

See `LICENSE` file for details.

## Changelog

### v1.0.0 (2025-01-30)
- Initial production release
- Filesystem explorer with TUI
- File watcher with pattern matching
- IPC daemon for process coordination
- Comprehensive test suite
- Full documentation

---

**Documentation Version**: 1.0.0
**Last Updated**: 2025-01-30
**Maintainer**: WezTerm Utilities Team