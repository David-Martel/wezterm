# WezTerm Customization & Optimization Plan

**Created**: 2026-02-04
**Status**: In Progress
**Branch**: main @ 823a31959

---

## Executive Summary

This plan outlines the roadmap for building customizations and optimizations to the WezTerm fork, incorporating Microsoft Rust Guidelines, merging upstream updates, and implementing the AI Assistant Module.

---

## Phase 0: Housekeeping & Quick Wins (Current)

### 0.1 Update .gitignore
- [ ] Add temp/log files: `TODO.md`, `*.log`, `clippy-output.txt`
- [ ] Add IDE/editor files if missing
- [ ] Verify `.github/` overrides work correctly

### 0.2 Incorporate Microsoft Rust Guidelines
- [ ] Add guidelines reference to `CLAUDE.md`
- [ ] Create `AGENTS.md` with Rust-specific agent instructions
- [ ] Document key guidelines for contributors

### 0.3 Repository Cleanup
- [ ] Remove untracked temp files or commit them appropriately
- [ ] Consider adding `wezterm-fs-explorer` to workspace
- [ ] Verify all clippy warnings resolved

---

## Phase 1: Merge Upstream Updates

**Upstream commits available**: 34 commits

### 1.1 Key Bug Fixes to Incorporate
| Commit | Description | Priority |
|--------|-------------|----------|
| `30ef869d7` | Fix parsing of partial SGR mouse sequences | High |
| `d2fc83559` | Fix macOS notifications to display as toast popups | Medium |
| `c8a649684` | Fix memory leak in macOS MetalLayer management | High |
| `bb297506d` | Fix boundary check condition in renderstate | High |
| `c6f25ea3f` | Fix fractional scaling issues in hyprland 0.51.0 | Medium |

### 1.2 Merge Strategy
```bash
git fetch upstream
git merge upstream/main
# Resolve conflicts (expected: minimal - changes are additive)
git push origin main
```

### 1.3 Post-Merge Validation
- [ ] Run `just full-local-ci`
- [ ] Verify custom utilities still build
- [ ] Test on Windows environment

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

### 3.1 wezterm-fs-explorer
- [ ] Add to workspace (optional - has benefits/tradeoffs)
- [ ] Increase test coverage
- [ ] Add integration tests with WezTerm
- [ ] Performance profiling

### 3.2 wezterm-watch
- [ ] Add comprehensive tests
- [ ] Document WezTerm integration patterns
- [ ] Consider additional output formats

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

### Phase 0 Complete When:
- [ ] .gitignore updated and committed
- [ ] CLAUDE.md includes Microsoft Rust Guidelines
- [ ] AGENTS.md created with Rust-specific instructions
- [ ] All temp files handled appropriately

### Phase 1 Complete When:
- [ ] Upstream merged successfully
- [ ] All tests pass
- [ ] Custom utilities verified working

### Phase 2 Complete When:
- [ ] Module framework functional
- [ ] AI assistant responds to queries
- [ ] Tool execution works end-to-end
- [ ] Memory usage < 700MB with AI active

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
