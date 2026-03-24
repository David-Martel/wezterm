# WezTerm Utilities - Installation Verification

Follow these steps to verify your installation is complete and working correctly.

## Pre-Flight Checklist

### Files Present ✅

Verify all files exist:

```powershell
# Check main module
Test-Path "C:\Users\david\.config\wezterm\wezterm-utils.lua"

# Check submodules
Test-Path "C:\Users\david\.config\wezterm\wezterm-utils\init.lua"
Test-Path "C:\Users\david\.config\wezterm\wezterm-utils\launcher.lua"
Test-Path "C:\Users\david\.config\wezterm\wezterm-utils\state.lua"
Test-Path "C:\Users\david\.config\wezterm\wezterm-utils\ipc.lua"
Test-Path "C:\Users\david\.config\wezterm\wezterm-utils\events.lua"
Test-Path "C:\Users\david\.config\wezterm\wezterm-utils\config.lua"

# Check documentation
Test-Path "C:\Users\david\.config\wezterm\WEZTERM-UTILS-README.md"
Test-Path "C:\Users\david\.config\wezterm\WEZTERM-UTILS-EXAMPLES.md"
Test-Path "C:\Users\david\.config\wezterm\WEZTERM-UTILS-TROUBLESHOOTING.md"
Test-Path "C:\Users\david\.config\wezterm\WEZTERM-UTILS-SUMMARY.md"
```

**Expected Output:** All `True`

---

### Integration Check ✅

Verify `.wezterm.lua` has been updated:

```powershell
# Check for utilities integration section
Select-String -Path "C:\Users\david\.wezterm.lua" -Pattern "UTILITIES INTEGRATION"

# Check for setup call
Select-String -Path "C:\Users\david\.wezterm.lua" -Pattern "utils.setup"

# Check for keybindings
Select-String -Path "C:\Users\david\.wezterm.lua" -Pattern "utils.explorer_split"
```

**Expected Output:** Should find matches for all patterns

---

## Verification Steps

### Step 1: Syntax Validation

Restart WezTerm and check for Lua syntax errors:

1. Open WezTerm
2. Press `Ctrl+Shift+L` to view logs
3. Look for error messages

**Expected:** No Lua syntax errors

**If errors appear:**
- Check line numbers in error messages
- Review corresponding file
- Verify no typos or missing characters

---

### Step 2: Module Loading

Check if module loads successfully:

**In WezTerm logs** (Ctrl+Shift+L), look for:
```
WezTerm utilities initialized successfully
```

**If you see:**
```
WezTerm utilities module not found
```

**Solution:** Verify files exist (see Pre-Flight Checklist)

---

### Step 3: Keybinding Test (Without Binaries)

Test graceful degradation when binaries are missing:

1. Press `Alt+E` (or `Alt+W`, `Ctrl+Alt+E`)
2. Should see toast notification: **"Explorer binary not found"**

**Expected Behavior:**
- ✅ Toast notification appears
- ✅ No crash or error
- ✅ WezTerm continues to work normally

**If nothing happens:**
- Check WezTerm logs for errors
- Verify keybindings added (see Integration Check)
- Check for keybinding conflicts

---

### Step 4: State Directory Creation

Verify state directory is auto-created:

```powershell
Test-Path "C:\Users\david\.config\wezterm\wezterm-utils-state"
```

**Expected Output:** `True` (directory should exist)

**If directory doesn't exist:**
- Manually create it: `mkdir -Force "C:\Users\david\.config\wezterm\wezterm-utils-state"`
- Check WezTerm logs for permission errors

---

### Step 5: Configuration Validation

Run the test suite to verify configuration:

**Method 1: Add to .wezterm.lua temporarily:**

```lua
-- Add before "return config" at end of .wezterm.lua
dofile(wezterm.config_dir .. '/test-wezterm-utils.lua')
```

Restart WezTerm and check logs for test results.

**Method 2: Manual diagnostics:**

Add this keybinding to `.wezterm.lua`:

```lua
table.insert(config.keys, {
  key = 'F12',
  mods = 'NONE',
  action = wezterm.action_callback(function(window, pane)
    if utils_available and utils then
      local diag = utils.diagnostics()
      local message = wezterm.json_encode(diag, { indent = true })
      window:toast_notification('Diagnostics', message, nil, 15000)
      wezterm.log_info('Diagnostics: ' .. message)
    else
      window:toast_notification('Error', 'Utils not available', nil, 4000)
    end
  end),
})
```

Press `F12` to view diagnostics.

---

### Step 6: Lazy Loading Verification

Verify lazy loading is working:

1. Restart WezTerm
2. Check WezTerm logs immediately after startup
3. Should see: "WezTerm utilities initialized successfully"
4. Should NOT see: "Lazy-loaded wezterm-utils.launcher" (until first use)
5. Press `Alt+E`
6. Now should see: "Lazy-loaded wezterm-utils.launcher"

**Expected:** Launcher module only loaded when first utility invoked

---

### Step 7: Binary Installation (Optional)

If you want to use the utilities now, build and install binaries:

**Filesystem Explorer:**
```powershell
cd "C:\Projects\wezterm-fs-explorer"
cargo build --release
mkdir -Force "$env:USERPROFILE\bin"
copy "target\release\wezterm-fs-explorer.exe" "$env:USERPROFILE\bin\"
```

**File Watcher:**
```powershell
cd "C:\Projects\wezterm-watch"
cargo build --release
copy "target\release\wezterm-watch.exe" "$env:USERPROFILE\bin\"
```

**Text Editor:**
```powershell
cd "C:\Projects\wedit"
uv pip install -e .
```

**Test:** Press `Alt+E` - should now launch explorer instead of showing toast

---

## Verification Checklist

Use this checklist to confirm installation:

- [ ] All module files exist (7 files)
- [ ] All documentation files exist (4 files)
- [ ] `.wezterm.lua` has integration code (lines 19-47, 422-478)
- [ ] WezTerm starts without Lua errors
- [ ] WezTerm logs show "WezTerm utilities initialized successfully"
- [ ] Pressing `Alt+E` shows toast notification (even without binary)
- [ ] State directory auto-created: `~\.config\wezterm\wezterm-utils-state\`
- [ ] No crashes or errors when pressing utility keybindings
- [ ] Diagnostics keybinding works (if added)
- [ ] Test suite passes (if run)
- [ ] (Optional) Binaries installed and utilities launch

---

## Success Criteria

Installation is **SUCCESSFUL** if:

1. ✅ WezTerm starts without errors
2. ✅ Logs show "WezTerm utilities initialized successfully"
3. ✅ Pressing `Alt+E` triggers a response (toast or launch)
4. ✅ State directory exists
5. ✅ No crashes when using keybindings

Installation is **COMPLETE** if additionally:

6. ✅ Binaries installed
7. ✅ Utilities launch successfully
8. ✅ State persists across restarts

---

## Common Issues During Verification

### Issue: Module Not Loading

**Symptoms:**
- Logs show "module not found"
- No initialization message

**Quick Fix:**
```powershell
# Verify files exist
dir "C:\Users\david\.config\wezterm\wezterm-utils.lua"

# Check WezTerm config directory
wezterm.exe --show-config-dir
```

---

### Issue: Keybindings Do Nothing

**Symptoms:**
- Pressing Alt+E does nothing
- No toast, no launch, no error

**Quick Fix:**
```lua
-- Add diagnostic logging to .wezterm.lua after utils.setup()
wezterm.log_info('Utils available: ' .. tostring(utils_available))
wezterm.log_info('Utils is nil: ' .. tostring(utils == nil))
```

---

### Issue: State Directory Not Created

**Symptoms:**
- Directory missing: `~\.config\wezterm\wezterm-utils-state\`

**Quick Fix:**
```powershell
# Create manually
mkdir -Force "$env:USERPROFILE\.config\wezterm\wezterm-utils-state"

# Verify permissions
icacls "$env:USERPROFILE\.config\wezterm\wezterm-utils-state"
```

---

## Next Steps After Verification

### If All Tests Pass ✅

1. **Build Utility Binaries** (see Step 7 above)
2. **Customize Configuration** (see EXAMPLES.md)
3. **Add Custom Keybindings** (see EXAMPLES.md)
4. **Explore Documentation** (README.md, EXAMPLES.md)

### If Tests Fail ❌

1. **Check WezTerm Logs** (Ctrl+Shift+L)
2. **Review Error Messages** (copy to troubleshooting)
3. **Consult TROUBLESHOOTING.md**
4. **Run Test Suite** (test-wezterm-utils.lua)
5. **Verify File Contents** (check for corruption)

---

## Automated Verification Script

Run this PowerShell script to verify installation:

```powershell
# Save as: verify-wezterm-utils.ps1

Write-Host "=== WezTerm Utilities Installation Verification ===" -ForegroundColor Cyan
Write-Host ""

$errors = 0
$warnings = 0

# Check files
$files = @(
  "$env:USERPROFILE\.config\wezterm\wezterm-utils.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\init.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\launcher.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\state.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\ipc.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\events.lua",
  "$env:USERPROFILE\.config\wezterm\wezterm-utils\config.lua"
)

Write-Host "[1] Checking module files..." -ForegroundColor Yellow
foreach ($file in $files) {
  if (Test-Path $file) {
    Write-Host "  OK: $file" -ForegroundColor Green
  } else {
    Write-Host "  MISSING: $file" -ForegroundColor Red
    $errors++
  }
}

# Check documentation
$docs = @(
  "$env:USERPROFILE\.config\wezterm\WEZTERM-UTILS-README.md",
  "$env:USERPROFILE\.config\wezterm\WEZTERM-UTILS-EXAMPLES.md",
  "$env:USERPROFILE\.config\wezterm\WEZTERM-UTILS-TROUBLESHOOTING.md",
  "$env:USERPROFILE\.config\wezterm\WEZTERM-UTILS-SUMMARY.md"
)

Write-Host ""
Write-Host "[2] Checking documentation files..." -ForegroundColor Yellow
foreach ($doc in $docs) {
  if (Test-Path $doc) {
    Write-Host "  OK: $doc" -ForegroundColor Green
  } else {
    Write-Host "  MISSING: $doc" -ForegroundColor Red
    $errors++
  }
}

# Check .wezterm.lua integration
Write-Host ""
Write-Host "[3] Checking .wezterm.lua integration..." -ForegroundColor Yellow

if (Select-String -Path "$env:USERPROFILE\.wezterm.lua" -Pattern "UTILITIES INTEGRATION" -Quiet) {
  Write-Host "  OK: Integration section found" -ForegroundColor Green
} else {
  Write-Host "  MISSING: Integration section not found" -ForegroundColor Red
  $errors++
}

if (Select-String -Path "$env:USERPROFILE\.wezterm.lua" -Pattern "utils\.setup" -Quiet) {
  Write-Host "  OK: Setup call found" -ForegroundColor Green
} else {
  Write-Host "  MISSING: Setup call not found" -ForegroundColor Red
  $errors++
}

# Check state directory
Write-Host ""
Write-Host "[4] Checking state directory..." -ForegroundColor Yellow

if (Test-Path "$env:USERPROFILE\.config\wezterm\wezterm-utils-state") {
  Write-Host "  OK: State directory exists" -ForegroundColor Green
} else {
  Write-Host "  INFO: State directory will be created on first use" -ForegroundColor Cyan
  $warnings++
}

# Check binaries (optional)
Write-Host ""
Write-Host "[5] Checking utility binaries (optional)..." -ForegroundColor Yellow

if (Test-Path "$env:USERPROFILE\bin\wezterm-fs-explorer.exe") {
  Write-Host "  OK: Explorer binary found" -ForegroundColor Green
} else {
  Write-Host "  INFO: Explorer binary not installed (optional)" -ForegroundColor Cyan
  $warnings++
}

if (Test-Path "$env:USERPROFILE\bin\wezterm-watch.exe") {
  Write-Host "  OK: Watcher binary found" -ForegroundColor Green
} else {
  Write-Host "  INFO: Watcher binary not installed (optional)" -ForegroundColor Cyan
  $warnings++
}

# Summary
Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
if ($errors -eq 0) {
  Write-Host "PASS: Installation verified successfully!" -ForegroundColor Green
  if ($warnings -gt 0) {
    Write-Host "INFO: $warnings optional components missing (see above)" -ForegroundColor Cyan
  }
} else {
  Write-Host "FAIL: $errors critical issues found" -ForegroundColor Red
  Write-Host "Please review errors above and consult TROUBLESHOOTING.md" -ForegroundColor Yellow
}
Write-Host ""
```

Save and run:
```powershell
powershell -ExecutionPolicy Bypass -File verify-wezterm-utils.ps1
```

---

## Final Status

After completing all verification steps, your status should be:

**✅ INSTALLATION COMPLETE** - Ready to build binaries and use utilities

**Next:** Build utility binaries (see SUMMARY.md) or start customizing (see EXAMPLES.md)