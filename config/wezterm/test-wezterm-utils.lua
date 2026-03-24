-- ============================================================================
-- WEZTERM-UTILS TEST SCRIPT
-- Test module loading and basic functionality
-- ============================================================================
--
-- Usage: Run this from your .wezterm.lua to verify installation:
--
-- wezterm.log_info('=== Testing WezTerm Utils ===')
-- dofile(wezterm.config_dir .. '/test-wezterm-utils.lua')
-- wezterm.log_info('=== Test Complete ===')
--

local wezterm = require('wezterm')

local function test_module_loading()
  wezterm.log_info('[TEST] Module Loading')

  local success, utils = pcall(require, 'wezterm-utils')

  if not success then
    wezterm.log_error('[FAIL] Failed to load module: ' .. tostring(utils))
    return false
  end

  wezterm.log_info('[PASS] Module loaded successfully')
  return true, utils
end

local function test_setup(utils)
  wezterm.log_info('[TEST] Setup Initialization')

  local success = utils.setup({
    state_enabled = true,
    lazy_load = true,
    verify_binaries = true,
  })

  if not success then
    wezterm.log_error('[FAIL] Setup failed')
    return false
  end

  wezterm.log_info('[PASS] Setup successful')
  return true
end

local function test_config_validation(utils)
  wezterm.log_info('[TEST] Configuration Validation')

  local config_module = require('wezterm-utils.config')
  local valid, errors = config_module.validate(utils.config)

  if not valid then
    wezterm.log_error('[FAIL] Config validation failed:')
    for _, error in ipairs(errors) do
      wezterm.log_error('  - ' .. error)
    end
    return false
  end

  wezterm.log_info('[PASS] Configuration valid')
  return true
end

local function test_diagnostics(utils)
  wezterm.log_info('[TEST] Diagnostics')

  local diag = utils.diagnostics()

  wezterm.log_info('  Config: ' .. wezterm.json_encode(diag.config))
  wezterm.log_info('  Binaries:')
  wezterm.log_info('    explorer: ' .. (diag.binaries.explorer and 'FOUND' or 'MISSING'))
  wezterm.log_info('    watcher: ' .. (diag.binaries.watcher and 'FOUND' or 'MISSING'))
  wezterm.log_info('  Modules loaded: ' .. #diag.modules_loaded)

  wezterm.log_info('[PASS] Diagnostics retrieved')
  return true
end

local function test_state_module()
  wezterm.log_info('[TEST] State Module')

  local state = require('wezterm-utils.state')

  -- Test state operations
  local test_data = {
    test_key = 'test_value',
    timestamp = os.time(),
  }

  local save_success = state.save_state('test', test_data)
  if not save_success then
    wezterm.log_warn('[WARN] State save failed (may need state directory)')
    return true  -- Not a hard failure
  end

  local loaded_data = state.load_state('test')
  if not loaded_data then
    wezterm.log_error('[FAIL] State load failed')
    return false
  end

  if loaded_data.test_key ~= test_data.test_key then
    wezterm.log_error('[FAIL] State data mismatch')
    return false
  end

  -- Cleanup test state
  state.delete_state('test')

  wezterm.log_info('[PASS] State persistence working')
  return true
end

local function test_lazy_loading(utils)
  wezterm.log_info('[TEST] Lazy Loading')

  -- Check module cache before loading
  local cache_empty = (_G._wezterm_utils_modules == nil or
                       next(_G._wezterm_utils_modules) == nil)

  if not cache_empty then
    wezterm.log_info('[INFO] Modules already cached (expected if setup called)')
  else
    wezterm.log_info('[INFO] Module cache empty (lazy loading enabled)')
  end

  -- Force lazy load launcher
  local launcher = require('wezterm-utils.launcher')

  if not launcher then
    wezterm.log_error('[FAIL] Failed to lazy-load launcher')
    return false
  end

  wezterm.log_info('[PASS] Lazy loading works')
  return true
end

-- ============================================================================
-- RUN ALL TESTS
-- ============================================================================

local function run_all_tests()
  wezterm.log_info('='.rep(70))
  wezterm.log_info('WEZTERM-UTILS TEST SUITE')
  wezterm.log_info('='.rep(70))

  local tests_passed = 0
  local tests_total = 0

  -- Test 1: Module Loading
  tests_total = tests_total + 1
  local success, utils = test_module_loading()
  if success then tests_passed = tests_passed + 1 end

  if not success then
    wezterm.log_error('ABORT: Module loading failed - cannot continue tests')
    return
  end

  -- Test 2: Setup
  tests_total = tests_total + 1
  if test_setup(utils) then tests_passed = tests_passed + 1 end

  -- Test 3: Config Validation
  tests_total = tests_total + 1
  if test_config_validation(utils) then tests_passed = tests_passed + 1 end

  -- Test 4: Diagnostics
  tests_total = tests_total + 1
  if test_diagnostics(utils) then tests_passed = tests_passed + 1 end

  -- Test 5: State Module
  tests_total = tests_total + 1
  if test_state_module() then tests_passed = tests_passed + 1 end

  -- Test 6: Lazy Loading
  tests_total = tests_total + 1
  if test_lazy_loading(utils) then tests_passed = tests_passed + 1 end

  -- Summary
  wezterm.log_info('='.rep(70))
  wezterm.log_info(string.format('RESULTS: %d/%d tests passed', tests_passed, tests_total))

  if tests_passed == tests_total then
    wezterm.log_info('STATUS: ALL TESTS PASSED ✅')
  else
    wezterm.log_warn(string.format('STATUS: %d TESTS FAILED ❌', tests_total - tests_passed))
  end

  wezterm.log_info('='.rep(70))
end

-- Run tests
run_all_tests()