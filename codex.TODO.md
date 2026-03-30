# Codex Validator Plan

Last updated: 2026-03-29
Scope: repo-managed WezTerm config validation, pre-launch enforcement, and live-runtime safety

## Goals

- Prevent `.wezterm.lua` and repo-vendored Lua modules from crashing `wezterm-gui.exe`.
- Add a first-class pre-launch validator to the application, not just an external smoke test.
- Validate the real repo-managed runtime layout:
  - `~/.wezterm.lua`
  - `~/.config/wezterm/*`
  - installed `~/bin/wezterm*.{exe,cmd,ps1}`
  - Windows Terminal launch path
- Fold the work into existing TODOs for Tier 5.P and Tier 5.Q.

## Phase 1: Validator Framework In The App

- [x] Add a new `wezterm validate-config` CLI subcommand in `wezterm/src/main.rs`.
- [x] Expose a structured validation snapshot from the `config` crate:
  - resolved config file
  - watch paths / required module paths
  - warnings
  - validation errors
  - generation / loaded-state metadata needed by the CLI
- [x] Extend the Lua config loader with a validator registration API so config code can declare deeper checks.
- [x] Support both human-readable and JSON output from the validator command.
- [x] Return non-zero on validation failure so launchers can block the GUI.

## Phase 2: Repo-Specific Lua Validators

- [x] Add a repo-owned validator module for `.wezterm.lua` and vendored modules.
- [x] Register validators from `.wezterm.lua` early enough that module/load-path issues are caught before GUI launch.
- [x] Validate required module resolution for:
  - `codex_ui.*`
  - `wezterm-utils`
  - optional embedded-dev config
- [~] Validate repo/runtime assumptions:
  - config file and config dir resolve correctly
  - repo-vendored modules are reachable from `package.path`
  - [x] state directories stay outside watched config trees
  - [ ] review whether any additional watched roots should be inferred from the symlinked home layout
  - required utility binaries and paths are coherent with the installed/runtime layout
- [x] Surface warnings separately from hard failures so optional integrations do not block launch.
- [x] Keep Lua validators backward compatible with already-installed binaries that do not yet expose `wezterm.add_config_validator`.

## Phase 3: Pre-Launch Integration

- [x] Update installed launchers in `build-all.ps1` so `wezterm-launch.cmd`, `wezterm-gui.cmd`, and `wezterm-launch.ps1` run `wezterm validate-config` before `wezterm-gui.exe`.
- [ ] Keep `wezterm-cli.cmd` direct for non-GUI usage.
- [x] Ensure launcher validation can be bypassed intentionally only via an explicit environment flag for diagnostics/recovery.
- [x] Replace the current "GUI stayed alive for 5 seconds" config check in `install-verification.ps1` with the validator command.

## Phase 4: Runtime/Deployment Validation

- [~] Re-run validation against the live symlinked home config layout (Tier 5.P).
- [ ] Confirm the repo-vendored `codex_ui` and `wezterm-utils` trees stay aligned with the live runtime after restart/reload (Tier 5.Q).
- [ ] Validate Windows Terminal still launches through the repo-managed home config path.
- [x] Keep `wezterm --config-file $env:USERPROFILE\\.wezterm.lua show-keys` as a secondary smoke probe, not the only validator.
- [x] Verify the current installed binary tolerates the new Lua validator module before the rebuilt executable is deployed.

## Phase 5: Follow-On Hardening

- [ ] Review `wezterm-gui/src/main.rs` startup hook changes against the new validator path so GUI bootstrap stays consistent with CLI validation.
- [ ] Consider a second validator tranche for static Lua lint checks over watched config files:
  - suspicious `wezterm.GLOBAL` key usage
  - writes under watched config paths
  - unsafe module assumptions that can survive syntax validation
- [ ] Decide whether to emit validator artifacts/log snapshots for installer diagnostics.

## Related Existing TODOs To Track Together

- [ ] Tier 5.P: Validate repo-managed symlinked home config with GUI smoke test and Windows Terminal launch path
- [ ] Tier 5.Q: Verify repo-vendored `codex_ui` and `wezterm-utils` stay in sync with live runtime after restart/reload
- [ ] P1: Run focused WezTerm config validation after panel persistence changes
- [ ] P1: Run GUI smoke test against repo-managed home symlink layout
- [ ] P1: Validate Windows Terminal profile still launches through the symlinked home config path
- [ ] Codex owner item: review `wezterm-gui/src/main.rs` module init hook

## Immediate Execution Order

- [x] Write validator framework plan
- [x] Implement Rust validation command + config snapshot API
- [x] Implement Lua validator registration and repo-specific validators
- [x] Wire launchers/install verification to the validator
- [~] Run focused validation against live config and installed binaries
- [ ] Update `TODO.md` once the tranche lands cleanly
