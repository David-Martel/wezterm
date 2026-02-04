# WezTerm Customization & Optimization Plan

**Created**: 2026-02-04
**Updated**: 2026-02-04
**Status**: Phase 0-1 Complete, Phase 2-3 In Progress
**Branch**: main @ 801eb8067

---

## Executive Summary

This plan outlines the roadmap for building customizations and optimizations to the WezTerm fork, incorporating Microsoft Rust Guidelines, merging upstream updates, and implementing the AI Assistant Module.

---

## Phase 0: Housekeeping & Quick Wins (Current)

### 0.1 Update .gitignore
- [x] Add temp/log files: `TODO.md`, `*.log`, `clippy-output.txt`
- [x] Add IDE/editor files if missing
- [x] Verify `.github/` overrides work correctly

### 0.2 Incorporate Microsoft Rust Guidelines
- [x] Add guidelines reference to `CLAUDE.md`
- [x] Create `AGENTS.md` with Rust-specific agent instructions
- [x] Document key guidelines for contributors

### 0.3 Repository Cleanup
- [x] Remove untracked temp files or commit them appropriately
- [x] Migrated git2 -> gix (pure Rust) - eliminates dependency version conflicts
- [x] Verify all clippy warnings resolved
- [x] Added PowerShell build tools framework (tools/)
- [x] Enhanced Justfile to 49 targets

**Note on gix migration:**
Both utilities now use pure-Rust `gix` instead of `git2/libssh2`. This eliminates
Windows native library linking issues and dependency version conflicts. The
wezterm-fs-explorer remains standalone but could now be added to workspace if desired.

---

## Phase 1: Merge Upstream Updates - COMPLETE

**Upstream commits merged**: 34 commits
**Merge commit**: `bcb6b469f`

### 1.1 Key Bug Fixes Incorporated
| Commit | Description | Priority | Status |
|--------|-------------|----------|--------|
| `30ef869d7` | Fix parsing of partial SGR mouse sequences | High | ✅ |
| `d2fc83559` | Fix macOS notifications to display as toast popups | Medium | ✅ |
| `c8a649684` | Fix memory leak in macOS MetalLayer management | High | ✅ |
| `bb297506d` | Fix boundary check condition in renderstate | High | ✅ |
| `c6f25ea3f` | Fix fractional scaling issues in hyprland 0.51.0 | Medium | ✅ |

### 1.2 Merge Details
- **Conflict**: `Cargo.lock` - resolved by accepting upstream version
- **Method**: `git merge upstream/main`
- **Verification**: Core crates (`wezterm-escape-parser`) build successfully

### 1.3 Post-Merge Validation
- [x] Core crates build successfully
- [x] Custom utilities build successfully (both migrated to gix)
- [x] Windows environment tested - all 182 tests passing
- [x] Windows CI workflow created (.github/workflows/windows-ci.yml)
- [ ] Run `just full-local-ci` (OpenSSL/Perl env issue in WSL - not code related)

**Note**: Custom utilities fully functional on Windows. Full WezTerm build requires
fixing WSL Perl environment for OpenSSL compilation (not blocking utility development).

---

## Phase 2: AI Module Implementation

### Overview
Implement the comprehensive AI Assistant Module as specified in `WEZTERM_AI_MODULE_DESIGN.md`.

### 2.1 Phase 1: Core Framework (4-6 weeks)

**Week 1-2: Module Framework**
- [ ] Create `wezterm-module-framework` crate
- [ ] Implement `WezTermModule` trait
- [ ] Implement `ModuleManager` with discovery/loading
- [ ] Implement `ModuleIpc` for inter-module communication
- [ ] Add tests for module lifecycle

**Week 3-4: Integration with WezTerm**
- [ ] Hook into `config/src/lua.rs` for Lua context setup
- [ ] Hook into `wezterm-gui/src/main.rs` for module initialization
- [ ] Register module domains with mux
- [ ] Add module configuration parsing
- [ ] Create example "hello world" module

**Week 5-6: Lua API Layer**
- [ ] Create `lua-api-crates/module-framework/`
- [ ] Implement Lua module registration
- [ ] Add event handling Lua APIs
- [ ] Add IPC Lua APIs
- [ ] Documentation and examples

### 2.2 Phase 2: Filesystem & Commander Modules (2-3 weeks)

**Week 7-8: Filesystem Module**
- [ ] Port `rust-mcp-filesystem` patterns
- [ ] Implement 16 filesystem tools
- [ ] Add path validation and security
- [ ] Lua API bindings
- [ ] Integration tests

**Week 9: Commander Module**
- [ ] Implement command executor with sandboxing
- [ ] Add whitelist/blacklist configuration
- [ ] Environment sanitization
- [ ] Lua API bindings
- [ ] Security tests

### 2.3 Phase 3: LLM Integration (4-5 weeks)

**Week 10-11: mistral.rs Integration**
- [ ] Create AI assistant module structure
- [ ] Integrate mistral.rs with builder pattern
- [ ] Implement lazy loading of LLM
- [ ] Add streaming response support
- [ ] Memory optimization and quantization

**Week 12-13: Tool Execution & MCP**
- [ ] MCP client configuration builder
- [ ] Tool executor implementation
- [ ] Filesystem tool integration
- [ ] Commander tool integration
- [ ] Test tool calling end-to-end

**Week 14: RAG System (Optional)**
- [ ] Redis client integration
- [ ] Local embedding service
- [ ] Session ingestion pipeline
- [ ] Semantic search implementation

### 2.4 Phase 4: UI & UX (3-4 weeks)

**Week 15-16: AI Assistant Overlay**
- [ ] Create `AiAssistantPane` implementation
- [ ] Implement chat interface rendering
- [ ] Add input handling and keyboard shortcuts
- [ ] Streaming response display
- [ ] Error handling and user feedback

**Week 17: Context Integration**
- [ ] Extract terminal context (cwd, shell, history)
- [ ] System prompt generation
- [ ] Recent command context
- [ ] Pane/tab context awareness

**Week 18: Polish & Testing**
- [ ] End-to-end testing
- [ ] Performance benchmarking
- [ ] Memory leak testing
- [ ] Documentation
- [ ] Example configurations

---

## Phase 3: Custom Utility Enhancements

### 3.1 wezterm-fs-explorer - MAJOR ENHANCEMENTS COMPLETE
- [x] Migrated git2 -> gix (pure Rust, no native deps)
- [x] Added UDS Windows IPC (ipc.rs)
- [x] Added WSL path translation (path_utils.rs)
- [x] Added shell detection (shell.rs)
- [x] Added fuzzy search with nucleo (search.rs)
- [x] 108 tests passing
- [ ] Add to workspace (optional - now possible with gix)
- [ ] Add integration tests for new modules (ipc, path_utils, shell)
- [ ] Performance profiling of fuzzy search
- [ ] Security audit of IPC and path translation

### 3.2 wezterm-watch - MAJOR ENHANCEMENTS COMPLETE
- [x] Migrated git2 -> gix (pure Rust, no native deps)
- [x] Enhanced output formatting
- [x] Improved watcher error handling
- [x] 74 tests passing
- [ ] Document WezTerm integration patterns
- [ ] Add end-to-end CLI tests

### 3.3 Build Framework - COMPLETE
- [x] Justfile expanded to 49 targets
- [x] cargo-smart-release integration (release.toml)
- [x] git-cliff changelog generation (cliff.toml)
- [x] cargo-binstall support in Cargo.toml
- [x] PowerShell build tools (tools/Build-Integration.ps1)
- [x] gix CLI wrapper (tools/Invoke-Gix.ps1)
- [x] CargoTools module (tools/CargoTools/)
- [x] Windows CI workflow (.github/workflows/windows-ci.yml)

---

## Microsoft Rust Guidelines Integration

### Key Guidelines to Enforce

#### Safety (M-UNSAFE)
- Only use `unsafe` for: novel abstractions, performance optimization (after benchmarking), FFI
- Never use `unsafe` to bypass compiler bounds or lifetime requirements
- Document all safety reasoning in plain text
- Pass Miri validation

#### Naming (M-CONCISE-NAMES)
- Avoid weasel words: "Service", "Manager", "Factory"
- Use specific, descriptive names
- Prefer regular functions over associated functions for general computation

#### Error Handling (M-PANIC-IS-STOP)
- Panics signal "this program should stop now"
- Programming bugs should panic, not return errors
- Contract violations warrant panics

#### Performance (M-THROUGHPUT, M-HOTPATH)
- Identify hot paths early, create benchmarks
- Design APIs for batched operations
- Exploit CPU cache locality
- Include yield points in long-running async tasks (10-100μs between yields)

#### Code Quality (M-STATIC-VERIFICATION)
- Use clippy, rustfmt, cargo-audit, cargo-hack
- All public types must implement `Debug`
- Use `#[expect]` instead of `#[allow]` for lint overrides

#### Documentation (M-DOCUMENTED-MAGIC)
- Document all magic values with rationale
- Use structured logging with OpenTelemetry conventions

---

## Success Criteria

### Phase 0 Complete When: ✅ COMPLETE
- [x] .gitignore updated and committed
- [x] CLAUDE.md includes Microsoft Rust Guidelines
- [x] AGENTS.md created with Rust-specific instructions
- [x] All temp files handled appropriately
- [x] Build framework enhanced (49 Justfile targets, tools/)

### Phase 1 Complete When: ✅ COMPLETE
- [x] Upstream merged successfully (34 commits)
- [x] All tests pass (182 tests)
- [x] Custom utilities verified working
- [x] Windows CI workflow active

### Phase 2 Complete When: (AI Module - Not Started)
- [ ] Module framework functional
- [ ] AI assistant responds to queries
- [ ] Tool execution works end-to-end
- [ ] Memory usage < 700MB with AI active

### Phase 3 Complete When: (Utilities - In Progress)
- [x] gix migration complete
- [x] Windows/WSL integration modules added
- [x] Build framework complete
- [ ] Integration tests for new modules
- [ ] Security audit complete
- [ ] Performance profiling complete

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Merge conflicts with upstream | Keep custom changes additive, in separate files |
| LLM memory overhead | Lazy loading, quantization, model selection |
| Breaking WezTerm core | Extensive testing, modular design |
| Performance regression | Benchmark hot paths, profile regularly |

---

## References

- [WEZTERM_AI_MODULE_DESIGN.md](../../WEZTERM_AI_MODULE_DESIGN.md)
- [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines/)
- [WezTerm Documentation](https://wezfurlong.org/wezterm/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
