# WezTerm AI Assistant Module - Comprehensive Design Specification

## Executive Summary

This document outlines the design and implementation strategy for integrating a powerful local LLM-based AI assistant into WezTerm through a modular framework architecture. The design leverages insights from existing implementations (gemma.cpp, rust-mistral, rust-mcp-filesystem) and modern terminal AI patterns (Warp Terminal) to create a high-performance, extensible system.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Module Framework Core](#module-framework-core)
3. [AI Assistant Module](#ai-assistant-module)
4. [LLM Integration Layer](#llm-integration-layer)
5. [Filesystem & Commander Utilities](#filesystem--commander-utilities)
6. [Performance & Memory Optimization](#performance--memory-optimization)
7. [Security & Sandboxing](#security--sandboxing)
8. [Implementation Roadmap](#implementation-roadmap)
9. [API Reference](#api-reference)

---

## 1. Architecture Overview

### 1.1 High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         WezTerm Core                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Terminal   │  │     Mux      │  │   Lua API    │          │
│  │    Engine    │  │  Multiplexer │  │   Context    │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│               WezTerm Module Framework                           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Module Manager                                           │  │
│  │  - Discovery & Loading                                    │  │
│  │  - Lifecycle Management                                   │  │
│  │  - Inter-Module IPC                                       │  │
│  │  - Resource Management                                    │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │
│  │ AI Assistant │  │  Filesystem  │  │  Commander   │        │
│  │    Module    │  │    Module    │  │    Module    │        │
│  └──────────────┘  └──────────────┘  └──────────────┘        │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                    LLM Integration Layer                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │
│  │   Mistral.rs │  │  Gemma.cpp   │  │  RAG System  │        │
│  │   Inference  │  │   (C++ FFI)  │  │  (rag-redis) │        │
│  └──────────────┘  └──────────────┘  └──────────────┘        │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Tool Execution Engine (MCP Protocol)                     │  │
│  │  - Filesystem tools                                       │  │
│  │  - Command execution                                      │  │
│  │  - Web search / API calls                                 │  │
│  └──────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### 1.2 Design Principles

1. **Modularity**: Clear separation of concerns, pluggable components
2. **Performance**: Async I/O, lazy loading, minimal overhead (<100MB memory)
3. **Security**: Sandboxing, capability-based permissions, path validation
4. **Extensibility**: Plugin API for third-party modules
5. **Cross-Platform**: Windows, Linux, macOS support
6. **User Experience**: Non-intrusive, context-aware, streaming responses

### 1.3 Technology Stack

**Core Framework**:
- **Language**: Rust (edition 2021)
- **Async Runtime**: Tokio (already in WezTerm)
- **Lua Integration**: mlua (existing WezTerm dependency)
- **Serialization**: serde, serde_json

**LLM Inference**:
- **Primary Engine**: mistral.rs (v0.6.0+)
- **Alternative**: gemma.cpp via FFI
- **RAG System**: rag-redis-system (optional)

**Utilities**:
- **Filesystem**: rust-mcp-filesystem patterns
- **IPC**: JSON-RPC 2.0 over channels/stdio
- **Tool Protocol**: MCP (Model Context Protocol)

---

## 2. Module Framework Core

### 2.1 Module Trait Definition

**Location**: `wezterm-module-framework/src/lib.rs`

```rust
use mlua::Lua;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Core trait that all WezTerm modules must implement
#[async_trait(?Send)]
pub trait WezTermModule: Send + Sync {
    /// Module metadata
    fn metadata(&self) -> ModuleMetadata;

    /// Initialize module with dependencies
    async fn initialize(&mut self, ctx: ModuleContext) -> anyhow::Result<()>;

    /// Register Lua APIs (called during Lua context creation)
    fn register_lua_api(&self, lua: &Lua) -> anyhow::Result<()>;

    /// Register event handlers
    fn register_events(&self) -> Vec<EventHandler>;

    /// Optional: Create custom domain for multiplexer
    fn create_domain(&self) -> Option<Arc<dyn Domain>>;

    /// Optional: Create custom pane type
    fn create_pane(&self) -> Option<Box<dyn Pane>>;

    /// Optional: Create UI overlay
    fn create_overlay(&self, ctx: OverlayContext) -> Option<Box<dyn Pane>>;

    /// Handle inter-module messages
    async fn handle_message(&self, msg: ModuleMessage) -> anyhow::Result<ModuleMessage>;

    /// Shutdown/cleanup
    async fn shutdown(&mut self) -> anyhow::Result<()>;

    /// Health check for monitoring
    fn health_check(&self) -> ModuleHealth;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: Vec<ModuleCapability>,
    pub dependencies: Vec<String>,
    pub lua_namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleCapability {
    FileSystemRead,
    FileSystemWrite,
    ProcessSpawn,
    NetworkAccess,
    ClipboardAccess,
    UIOverlay,
    CustomDomain,
    CustomPane,
    LLMInference,
    ToolExecution,
}

pub struct ModuleContext {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub ipc: Arc<ModuleIpc>,
    pub mux: Arc<Mux>,
    pub config: serde_json::Value,
}

pub struct EventHandler {
    pub event_name: String,
    pub handler: Arc<dyn Fn(EventData) -> Pin<Box<dyn Future<Output = bool>>>>,
}

#[derive(Debug, Clone)]
pub enum ModuleMessage {
    Request { id: String, from: String, method: String, params: serde_json::Value },
    Response { id: String, to: String, result: serde_json::Value },
    Error { id: String, to: String, error: String },
    Broadcast { from: String, event: String, data: serde_json::Value },
}

#[derive(Debug, Clone)]
pub struct ModuleHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
```

### 2.2 Module Manager Implementation

**Location**: `wezterm-module-framework/src/manager.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ModuleManager {
    modules: RwLock<HashMap<String, Arc<dyn WezTermModule>>>,
    ipc: Arc<ModuleIpc>,
    config: ModuleManagerConfig,
}

impl ModuleManager {
    pub fn new(config: ModuleManagerConfig) -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
            ipc: Arc::new(ModuleIpc::new()),
            config,
        }
    }

    /// Discover modules from filesystem and built-ins
    pub async fn discover_modules(&self) -> anyhow::Result<Vec<ModuleDescriptor>> {
        let mut descriptors = Vec::new();

        // 1. Load built-in modules (compiled into binary)
        descriptors.extend(self.discover_builtin_modules()?);

        // 2. Scan ~/.config/wezterm/modules/ for external modules
        descriptors.extend(self.discover_external_modules().await?);

        // 3. Validate dependencies and capabilities
        self.validate_modules(&descriptors)?;

        Ok(descriptors)
    }

    /// Load and initialize a module
    pub async fn load_module(&self, descriptor: ModuleDescriptor) -> anyhow::Result<()> {
        let module = match descriptor.module_type {
            ModuleType::BuiltIn(factory) => factory()?,
            ModuleType::Dynamic(path) => self.load_dynamic_module(&path)?,
        };

        // Create module context
        let ctx = ModuleContext {
            config_dir: self.config.config_dir.clone(),
            data_dir: self.config.data_dir.clone(),
            ipc: Arc::clone(&self.ipc),
            mux: Mux::get(),
            config: descriptor.config,
        };

        // Initialize module
        let mut module = module;
        module.initialize(ctx).await?;

        // Register with IPC
        self.ipc.register_module(&module.metadata().name, Arc::clone(&module));

        // Store module
        self.modules.write().await.insert(
            module.metadata().name.clone(),
            Arc::new(module),
        );

        Ok(())
    }

    /// Register all modules with Lua context
    pub async fn register_lua_apis(&self, lua: &Lua) -> anyhow::Result<()> {
        for module in self.modules.read().await.values() {
            module.register_lua_api(lua)?;
        }
        Ok(())
    }

    /// Subscribe to mux notifications and dispatch to modules
    pub fn subscribe_to_mux(&self) {
        let modules = Arc::clone(&self.modules);

        Mux::get().subscribe(move |notif| {
            let modules = Arc::clone(&modules);
            tokio::spawn(async move {
                for module in modules.read().await.values() {
                    // Dispatch relevant notifications
                    if let Err(e) = module.handle_notification(notif.clone()).await {
                        log::warn!("Module notification error: {}", e);
                    }
                }
            });
            true
        });
    }

    /// Health check all modules
    pub async fn health_check_all(&self) -> HashMap<String, ModuleHealth> {
        let mut results = HashMap::new();
        for (name, module) in self.modules.read().await.iter() {
            results.insert(name.clone(), module.health_check());
        }
        results
    }

    /// Shutdown all modules gracefully
    pub async fn shutdown_all(&self) -> anyhow::Result<()> {
        for module in self.modules.write().await.values_mut() {
            if let Err(e) = Arc::get_mut(module).unwrap().shutdown().await {
                log::error!("Module shutdown error: {}", e);
            }
        }
        Ok(())
    }
}

pub struct ModuleDescriptor {
    pub metadata: ModuleMetadata,
    pub module_type: ModuleType,
    pub config: serde_json::Value,
}

pub enum ModuleType {
    BuiltIn(Box<dyn Fn() -> anyhow::Result<Box<dyn WezTermModule>>>),
    Dynamic(PathBuf),
}
```

### 2.3 Inter-Module IPC

**Location**: `wezterm-module-framework/src/ipc.rs`

```rust
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ModuleIpc {
    channels: RwLock<HashMap<String, mpsc::UnboundedSender<ModuleMessage>>>,
    broadcast_subscribers: RwLock<Vec<mpsc::UnboundedSender<ModuleMessage>>>,
}

impl ModuleIpc {
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            broadcast_subscribers: RwLock::new(Vec::new()),
        }
    }

    /// Register a module's message channel
    pub async fn register_module(&self, name: &str, sender: mpsc::UnboundedSender<ModuleMessage>) {
        self.channels.write().await.insert(name.to_string(), sender);
    }

    /// Send message to specific module
    pub async fn send(&self, target: &str, msg: ModuleMessage) -> anyhow::Result<()> {
        let channels = self.channels.read().await;
        if let Some(sender) = channels.get(target) {
            sender.send(msg)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Module not found: {}", target))
        }
    }

    /// Send request and wait for response
    pub async fn request(&self, target: &str, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let id = uuid::Uuid::new_v4().to_string();
        let msg = ModuleMessage::Request {
            id: id.clone(),
            from: "system".to_string(),
            method: method.to_string(),
            params,
        };

        // Create response channel
        let (tx, mut rx) = mpsc::unbounded_channel();
        // Register temporary response handler
        // ... (implementation with timeout)

        self.send(target, msg).await?;

        // Wait for response with timeout
        match tokio::time::timeout(Duration::from_secs(30), rx.recv()).await {
            Ok(Some(ModuleMessage::Response { result, .. })) => Ok(result),
            Ok(Some(ModuleMessage::Error { error, .. })) => Err(anyhow::anyhow!(error)),
            _ => Err(anyhow::anyhow!("Request timeout")),
        }
    }

    /// Broadcast message to all modules
    pub async fn broadcast(&self, from: &str, event: &str, data: serde_json::Value) {
        let msg = ModuleMessage::Broadcast {
            from: from.to_string(),
            event: event.to_string(),
            data,
        };

        for sender in self.broadcast_subscribers.read().await.iter() {
            let _ = sender.send(msg.clone());
        }
    }

    /// Subscribe to broadcasts
    pub async fn subscribe(&self) -> mpsc::UnboundedReceiver<ModuleMessage> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.broadcast_subscribers.write().await.push(tx);
        rx
    }
}
```

### 2.4 Module Configuration

**User Configuration** (`~/.config/wezterm/wezterm.lua`):

```lua
local wezterm = require 'wezterm'
local config = {}

-- Enable module framework
config.enable_module_framework = true

-- Module-specific configuration
config.modules = {
  ai_assistant = {
    enabled = true,
    model = {
      engine = "mistral",  -- "mistral" or "gemma"
      model_id = "microsoft/Phi-3.5-mini-instruct",
      quantization = "Q4_0",
      max_tokens = 2048,
      temperature = 0.7,
    },
    rag = {
      enabled = true,
      redis_url = "redis://localhost:6379",
      embedding_model = "sentence-transformers/all-MiniLM-L6-v2",
    },
    tools = {
      filesystem = true,
      command_execution = true,
      web_search = false,
    },
    ui = {
      keybinding = { key = 'a', mods = 'CTRL|SHIFT' },
      position = "bottom",  -- "overlay", "bottom", "right"
      height_percent = 30,
    }
  },

  filesystem = {
    enabled = true,
    allowed_directories = {
      "~/projects",
      "~/documents",
    },
    read_only = false,
  },
}

return config
```

---

## 3. AI Assistant Module

### 3.1 Module Implementation

**Location**: `wezterm-builtin-modules/ai-assistant/src/lib.rs`

```rust
use wezterm_module_framework::*;
use mistralrs::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AiAssistantModule {
    config: AiAssistantConfig,
    llm_engine: Arc<RwLock<Option<TextModel>>>,
    rag_system: Arc<RwLock<Option<RagClient>>>,
    tool_executor: Arc<ToolExecutor>,
    conversation_history: Arc<RwLock<ConversationHistory>>,
}

impl AiAssistantModule {
    pub fn new(config: AiAssistantConfig) -> Self {
        Self {
            config,
            llm_engine: Arc::new(RwLock::new(None)),
            rag_system: Arc::new(RwLock::new(None)),
            tool_executor: Arc::new(ToolExecutor::new()),
            conversation_history: Arc::new(RwLock::new(ConversationHistory::new())),
        }
    }

    /// Lazy initialization of LLM engine (on first request)
    async fn ensure_llm_ready(&self) -> anyhow::Result<()> {
        let mut engine = self.llm_engine.write().await;
        if engine.is_none() {
            log::info!("Initializing LLM engine: {}", self.config.model.model_id);

            let model = TextModelBuilder::new(&self.config.model.model_id)
                .with_isq(IsqType::from_str(&self.config.model.quantization)?)
                .with_max_num_seqs(4)  // Allow 4 concurrent requests
                .with_prefix_cache_n(Some(16))  // Cache recent conversations
                .with_device(best_device(false)?)  // Auto-select GPU/CPU
                .with_mcp_client(self.build_mcp_config()?)  // Tool integration
                .build()
                .await?;

            *engine = Some(model);
        }
        Ok(())
    }

    /// Build MCP configuration for tool execution
    fn build_mcp_config(&self) -> anyhow::Result<McpClientConfig> {
        let mut servers = vec![];

        // Filesystem tools
        if self.config.tools.filesystem {
            servers.push(McpServerConfig {
                name: "Filesystem".to_string(),
                source: McpServerSource::Process {
                    command: "rust-mcp-filesystem".to_string(),
                    args: self.config.filesystem_allowed_dirs.clone(),
                    work_dir: None,
                    env: None,
                },
                tool_prefix: Some("fs".to_string()),
                ..Default::default()
            });
        }

        // Command execution tools
        if self.config.tools.command_execution {
            servers.push(McpServerConfig {
                name: "Commander".to_string(),
                source: McpServerSource::Process {
                    command: "rust-commander".to_string(),
                    args: vec![],
                    work_dir: None,
                    env: None,
                },
                tool_prefix: Some("cmd".to_string()),
                ..Default::default()
            });
        }

        Ok(McpClientConfig {
            servers,
            auto_register_tools: true,
            tool_timeout_secs: Some(30),
            max_concurrent_calls: Some(5),
        })
    }

    /// Handle user query with streaming response
    pub async fn query_streaming(
        &self,
        prompt: &str,
        context: QueryContext,
    ) -> anyhow::Result<impl Stream<Item = String>> {
        self.ensure_llm_ready().await?;

        // Build messages with context
        let mut messages = TextMessages::new();

        // System prompt
        messages = messages.add_message(
            TextMessageRole::System,
            &self.build_system_prompt(&context),
        );

        // Add recent conversation history
        for msg in self.conversation_history.read().await.recent(5) {
            messages = messages.add_message(msg.role.clone(), &msg.content);
        }

        // Add current query with context
        let query_with_context = self.augment_query_with_context(prompt, &context).await?;
        messages = messages.add_message(TextMessageRole::User, &query_with_context);

        // Stream response
        let engine = self.llm_engine.read().await;
        let model = engine.as_ref().unwrap();
        let stream = model.stream_chat_request(messages).await?;

        // Transform stream to extract text content
        Ok(stream.filter_map(|chunk| async move {
            match chunk {
                Response::Chunk(response) => {
                    response.choices.first()
                        .and_then(|c| c.delta.content.clone())
                }
                _ => None,
            }
        }))
    }

    /// Build system prompt with terminal context
    fn build_system_prompt(&self, context: &QueryContext) -> String {
        format!(r#"You are a helpful AI assistant integrated into WezTerm terminal emulator.

Current Context:
- Working Directory: {}
- Current Shell: {}
- Terminal Size: {}x{}
- Recent Commands: {}

You have access to the following tools:
- Filesystem operations (read, write, search files)
- Command execution
- Web search (if enabled)

Provide concise, accurate responses. When suggesting commands, explain what they do.
If you need to perform actions, use the available tools."#,
            context.cwd.display(),
            context.shell,
            context.terminal_cols,
            context.terminal_rows,
            context.recent_commands.join(", "),
        )
    }

    /// Augment query with RAG context
    async fn augment_query_with_context(
        &self,
        prompt: &str,
        context: &QueryContext,
    ) -> anyhow::Result<String> {
        if !self.config.rag.enabled {
            return Ok(prompt.to_string());
        }

        // Search RAG system for relevant context
        if let Some(rag) = self.rag_system.read().await.as_ref() {
            let results = rag.search(prompt, 3).await?;

            if !results.is_empty() {
                let context_snippets: Vec<String> = results
                    .into_iter()
                    .map(|r| format!("- {}", r.text))
                    .collect();

                return Ok(format!(
                    "Relevant context from terminal history:\n{}\n\nUser query: {}",
                    context_snippets.join("\n"),
                    prompt
                ));
            }
        }

        Ok(prompt.to_string())
    }
}

#[async_trait(?Send)]
impl WezTermModule for AiAssistantModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: "ai-assistant".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: "WezTerm Contributors".to_string(),
            description: "AI-powered terminal assistant with local LLM inference".to_string(),
            capabilities: vec![
                ModuleCapability::LLMInference,
                ModuleCapability::ToolExecution,
                ModuleCapability::NetworkAccess,
                ModuleCapability::UIOverlay,
            ],
            dependencies: vec![],
            lua_namespace: Some("ai".to_string()),
        }
    }

    async fn initialize(&mut self, ctx: ModuleContext) -> anyhow::Result<()> {
        log::info!("Initializing AI Assistant module");

        // Initialize RAG system if enabled
        if self.config.rag.enabled {
            let rag_client = RagClient::connect(&self.config.rag.redis_url).await?;
            *self.rag_system.write().await = Some(rag_client);
        }

        // Register with IPC
        ctx.ipc.register_module("ai-assistant", /* channel */).await;

        Ok(())
    }

    fn register_lua_api(&self, lua: &Lua) -> anyhow::Result<()> {
        let ai_module = lua.create_table()?;

        // wezterm.ai.query(prompt)
        let query_fn = {
            let module = Arc::new(self.clone());
            lua.create_async_function(move |_lua, prompt: String| {
                let module = Arc::clone(&module);
                async move {
                    let context = QueryContext::from_current_pane()?;
                    let response = module.query(&prompt, context).await?;
                    Ok(response)
                }
            })?
        };
        ai_module.set("query", query_fn)?;

        // wezterm.ai.stream(prompt, callback)
        let stream_fn = {
            let module = Arc::new(self.clone());
            lua.create_async_function(move |lua, (prompt, callback): (String, mlua::Function)| {
                let module = Arc::clone(&module);
                async move {
                    let context = QueryContext::from_current_pane()?;
                    let mut stream = module.query_streaming(&prompt, context).await?;

                    while let Some(chunk) = stream.next().await {
                        callback.call::<_, ()>(chunk)?;
                    }

                    Ok(())
                }
            })?
        };
        ai_module.set("stream", stream_fn)?;

        // Register in wezterm table
        let wezterm_table: mlua::Table = lua.globals().get("wezterm")?;
        wezterm_table.set("ai", ai_module)?;

        Ok(())
    }

    fn register_events(&self) -> Vec<EventHandler> {
        vec![
            EventHandler {
                event_name: "user-var-changed".to_string(),
                handler: Arc::new(|data| {
                    Box::pin(async move {
                        // Handle OSC 1337 user variables for context
                        true
                    })
                }),
            },
        ]
    }

    fn create_overlay(&self, ctx: OverlayContext) -> Option<Box<dyn Pane>> {
        Some(Box::new(AiAssistantPane::new(
            Arc::clone(&self.llm_engine),
            ctx,
        )))
    }

    async fn handle_message(&self, msg: ModuleMessage) -> anyhow::Result<ModuleMessage> {
        match msg {
            ModuleMessage::Request { id, method, params, .. } => {
                match method.as_str() {
                    "query" => {
                        let prompt: String = serde_json::from_value(params["prompt"].clone())?;
                        let context = QueryContext::default();
                        let response = self.query(&prompt, context).await?;

                        Ok(ModuleMessage::Response {
                            id,
                            to: "system".to_string(),
                            result: serde_json::json!({ "response": response }),
                        })
                    }
                    _ => Ok(ModuleMessage::Error {
                        id,
                        to: "system".to_string(),
                        error: format!("Unknown method: {}", method),
                    }),
                }
            }
            _ => Ok(msg),
        }
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        log::info!("Shutting down AI Assistant module");
        *self.llm_engine.write().await = None;
        *self.rag_system.write().await = None;
        Ok(())
    }

    fn health_check(&self) -> ModuleHealth {
        let status = if self.llm_engine.try_read().is_ok() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded
        };

        ModuleHealth {
            status,
            message: None,
            metrics: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryContext {
    pub cwd: PathBuf,
    pub shell: String,
    pub terminal_cols: usize,
    pub terminal_rows: usize,
    pub recent_commands: Vec<String>,
    pub pane_id: Option<PaneId>,
}
```

### 3.2 AI Assistant UI Overlay

**Location**: `wezterm-builtin-modules/ai-assistant/src/pane.rs`

```rust
use termwiz::surface::{Surface, Position};
use termwiz::cell::*;

pub struct AiAssistantPane {
    pane_id: PaneId,
    llm_engine: Arc<RwLock<Option<TextModel>>>,
    input_buffer: String,
    output_buffer: Vec<String>,
    cursor_position: usize,
    scroll_offset: usize,
}

impl Pane for AiAssistantPane {
    fn pane_id(&self) -> PaneId {
        self.pane_id
    }

    fn renderer(&self) -> RefMut<dyn Renderable> {
        // Render chat interface
        let mut surface = Surface::new(self.dimensions.cols, self.dimensions.rows);

        // Header
        surface.add_text(0, 0, "🤖 AI Assistant (Ctrl+Enter to send, Esc to close)");

        // Output area (scrollable)
        let output_start = 2;
        let output_height = self.dimensions.rows - 5;
        for (idx, line) in self.output_buffer
            .iter()
            .skip(self.scroll_offset)
            .take(output_height)
            .enumerate()
        {
            surface.add_text(0, output_start + idx, line);
        }

        // Input area
        let input_y = self.dimensions.rows - 3;
        surface.add_text(0, input_y, "You: ");
        surface.add_text(5, input_y, &self.input_buffer);

        // Cursor
        surface.set_cursor_position(Position::new(5 + self.cursor_position, input_y));

        RefMut::new(surface)
    }

    fn key_down(&mut self, key: KeyCode, mods: KeyModifiers) -> Result<(), Error> {
        match (key, mods) {
            (KeyCode::Enter, KeyModifiers::CTRL) => {
                // Send query
                self.send_query()?;
            }
            (KeyCode::Escape, _) => {
                // Close overlay
                self.close()?;
            }
            (KeyCode::Char(c), _) => {
                self.input_buffer.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            (KeyCode::Backspace, _) => {
                if self.cursor_position > 0 {
                    self.input_buffer.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }
            // ... handle other keys
            _ => {}
        }
        Ok(())
    }

    fn send_query(&mut self) -> Result<(), Error> {
        let prompt = self.input_buffer.clone();
        self.output_buffer.push(format!("You: {}", prompt));
        self.input_buffer.clear();
        self.cursor_position = 0;

        let engine = Arc::clone(&self.llm_engine);
        let output_buffer = Arc::clone(&self.output_buffer);

        tokio::spawn(async move {
            let mut stream = /* get streaming response */;
            let mut response = String::new();

            while let Some(chunk) = stream.next().await {
                response.push_str(&chunk);
                // Update output_buffer incrementally
                output_buffer.write().await.last_mut().replace(format!("AI: {}", response));
            }
        });

        Ok(())
    }
}
```

---

## 4. LLM Integration Layer

### 4.1 Unified LLM Interface

**Location**: `wezterm-builtin-modules/ai-assistant/src/llm.rs`

```rust
#[async_trait]
pub trait LlmEngine: Send + Sync {
    async fn generate(&self, messages: Vec<ChatMessage>) -> anyhow::Result<String>;
    async fn stream(&self, messages: Vec<ChatMessage>) -> anyhow::Result<impl Stream<Item = String>>;
    fn model_info(&self) -> ModelInfo;
}

pub struct MistralRsEngine {
    model: TextModel,
}

impl MistralRsEngine {
    pub async fn new(config: &ModelConfig) -> anyhow::Result<Self> {
        let model = TextModelBuilder::new(&config.model_id)
            .with_isq(IsqType::from_str(&config.quantization)?)
            .with_max_num_seqs(4)
            .with_prefix_cache_n(Some(16))
            .with_device(best_device(false)?)
            .build()
            .await?;

        Ok(Self { model })
    }
}

#[async_trait]
impl LlmEngine for MistralRsEngine {
    async fn generate(&self, messages: Vec<ChatMessage>) -> anyhow::Result<String> {
        let mistral_messages = self.convert_messages(messages);
        let response = self.model.send_chat_request(mistral_messages).await?;
        Ok(response.choices[0].message.content.clone())
    }

    async fn stream(&self, messages: Vec<ChatMessage>) -> anyhow::Result<impl Stream<Item = String>> {
        let mistral_messages = self.convert_messages(messages);
        let stream = self.model.stream_chat_request(mistral_messages).await?;

        Ok(stream.filter_map(|chunk| async move {
            match chunk {
                Response::Chunk(resp) => resp.choices.first()?.delta.content.clone(),
                _ => None,
            }
        }))
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            name: "Mistral.rs".to_string(),
            model_id: self.model.model_id().to_string(),
            // ... other info
        }
    }
}

pub struct GemmaCppEngine {
    process: Arc<Mutex<Child>>,
}

impl GemmaCppEngine {
    pub fn new(config: &ModelConfig) -> anyhow::Result<Self> {
        // Spawn gemma.cpp process
        let process = Command::new("gemma")
            .args(&["--model", &config.model_path, "--interactive"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(Self {
            process: Arc::new(Mutex::new(process)),
        })
    }
}

#[async_trait]
impl LlmEngine for GemmaCppEngine {
    async fn generate(&self, messages: Vec<ChatMessage>) -> anyhow::Result<String> {
        let prompt = self.format_prompt(messages);

        let mut process = self.process.lock().await;
        let stdin = process.stdin.as_mut().unwrap();
        stdin.write_all(prompt.as_bytes()).await?;
        stdin.flush().await?;

        let mut stdout = process.stdout.as_mut().unwrap();
        let mut response = String::new();
        stdout.read_to_string(&mut response).await?;

        Ok(response)
    }

    async fn stream(&self, messages: Vec<ChatMessage>) -> anyhow::Result<impl Stream<Item = String>> {
        // Similar to generate but yield incremental output
        todo!()
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            name: "Gemma.cpp".to_string(),
            // ...
        }
    }
}
```

### 4.2 RAG System Integration

**Location**: `wezterm-builtin-modules/ai-assistant/src/rag.rs`

```rust
use redis::AsyncCommands;

pub struct RagClient {
    redis: redis::Client,
    embedding_service: Box<dyn EmbeddingService>,
}

impl RagClient {
    pub async fn connect(redis_url: &str) -> anyhow::Result<Self> {
        let redis = redis::Client::open(redis_url)?;
        let embedding_service = LocalEmbeddingService::new()?;

        Ok(Self {
            redis,
            embedding_service: Box::new(embedding_service),
        })
    }

    /// Ingest terminal session into RAG
    pub async fn ingest_session(&self, session: &TerminalSession) -> anyhow::Result<()> {
        let chunks = self.chunk_session(session);

        for chunk in chunks {
            let embedding = self.embedding_service.embed(&chunk.text).await?;

            // Store in Redis
            let mut conn = self.redis.get_async_connection().await?;
            let key = format!("session:{}:{}", session.id, chunk.id);

            conn.hset_multiple(
                &key,
                &[
                    ("text", chunk.text.as_str()),
                    ("embedding", &serde_json::to_string(&embedding)?),
                    ("metadata", &serde_json::to_string(&chunk.metadata)?),
                ],
            ).await?;
        }

        Ok(())
    }

    /// Search for relevant context
    pub async fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<SearchResult>> {
        let query_embedding = self.embedding_service.embed(query).await?;

        // Simple cosine similarity search
        let mut conn = self.redis.get_async_connection().await?;
        let keys: Vec<String> = conn.keys("session:*").await?;

        let mut results = Vec::new();
        for key in keys {
            let stored_embedding: String = conn.hget(&key, "embedding").await?;
            let embedding: Vec<f32> = serde_json::from_str(&stored_embedding)?;

            let similarity = cosine_similarity(&query_embedding, &embedding);
            if similarity > 0.7 {
                let text: String = conn.hget(&key, "text").await?;
                results.push(SearchResult {
                    text,
                    score: similarity,
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        Ok(results)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
```

---

## 5. Filesystem & Commander Utilities

### 5.1 Filesystem Module

**Location**: `wezterm-builtin-modules/filesystem/src/lib.rs`

**Pattern**: Adapt `rust-mcp-filesystem` patterns

```rust
pub struct FilesystemModule {
    service: FileSystemService,
    mcp_server: Option<Arc<McpServer>>,
}

impl FilesystemModule {
    pub fn new(allowed_dirs: Vec<PathBuf>, read_only: bool) -> Self {
        Self {
            service: FileSystemService::new(allowed_dirs, read_only),
            mcp_server: None,
        }
    }
}

#[async_trait(?Send)]
impl WezTermModule for FilesystemModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: "filesystem".to_string(),
            version: "0.1.0".to_string(),
            author: "WezTerm".to_string(),
            description: "High-performance filesystem operations".to_string(),
            capabilities: vec![
                ModuleCapability::FileSystemRead,
                ModuleCapability::FileSystemWrite,
            ],
            dependencies: vec![],
            lua_namespace: Some("fs".to_string()),
        }
    }

    async fn initialize(&mut self, ctx: ModuleContext) -> anyhow::Result<()> {
        // Start internal MCP server for tool integration
        let handler = FilesystemHandler::new(self.service.clone());
        let transport = InternalTransport::new();
        self.mcp_server = Some(Arc::new(create_server(handler, transport)));

        Ok(())
    }

    fn register_lua_api(&self, lua: &Lua) -> anyhow::Result<()> {
        let fs_module = lua.create_table()?;

        // wezterm.fs.read_file(path)
        let read_fn = {
            let service = self.service.clone();
            lua.create_async_function(move |_lua, path: String| {
                let service = service.clone();
                async move {
                    let content = service.read_file(Path::new(&path)).await?;
                    Ok(content)
                }
            })?
        };
        fs_module.set("read_file", read_fn)?;

        // ... 15 more tool functions

        let wezterm: mlua::Table = lua.globals().get("wezterm")?;
        wezterm.set("fs", fs_module)?;

        Ok(())
    }

    // ... rest of trait implementation
}
```

### 5.2 Commander Module

**Location**: `wezterm-builtin-modules/commander/src/lib.rs`

```rust
pub struct CommanderModule {
    executor: CommandExecutor,
    sandboxing: SandboxConfig,
}

pub struct CommandExecutor {
    allowed_commands: HashSet<String>,
    env_whitelist: HashSet<String>,
}

impl CommandExecutor {
    pub async fn execute(&self, cmd: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        // Validate command against whitelist
        if !self.allowed_commands.is_empty() && !self.allowed_commands.contains(cmd) {
            return Err(anyhow::anyhow!("Command not allowed: {}", cmd));
        }

        // Build command with sanitized environment
        let output = Command::new(cmd)
            .args(args)
            .env_clear()
            .envs(self.get_safe_env())
            .output()
            .await?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }
}

#[async_trait(?Send)]
impl WezTermModule for CommanderModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: "commander".to_string(),
            version: "0.1.0".to_string(),
            author: "WezTerm".to_string(),
            description: "Safe command execution with sandboxing".to_string(),
            capabilities: vec![ModuleCapability::ProcessSpawn],
            dependencies: vec![],
            lua_namespace: Some("cmd".to_string()),
        }
    }

    // ... implementation
}
```

---

## 6. Performance & Memory Optimization

### 6.1 Memory Budget

**Target Resource Usage**:
- **Module Framework Core**: <10MB
- **AI Assistant (idle)**: <20MB
- **AI Assistant (LLM loaded)**: <500MB (Phi-3.5 Mini Q4)
- **RAG System**: <150MB (10K documents)
- **Filesystem Module**: <5MB
- **Total Overhead**: <700MB when AI active

### 6.2 Optimization Strategies

**1. Lazy Loading**:
```rust
// Don't load LLM until first query
pub struct LazyLlm {
    config: ModelConfig,
    engine: OnceCell<Arc<dyn LlmEngine>>,
}

impl LazyLlm {
    async fn get(&self) -> anyhow::Result<&Arc<dyn LlmEngine>> {
        self.engine.get_or_try_init(|| async {
            log::info!("Loading LLM on demand");
            let engine = MistralRsEngine::new(&self.config).await?;
            Ok(Arc::new(engine) as Arc<dyn LlmEngine>)
        }).await
    }
}
```

**2. Quantization**:
- Default to Q4_0 for balance of speed/quality
- Offer Q8_0 for higher quality (2x memory)
- Offer Q2K for ultra-low memory (<300MB)

**3. Prefix Caching**:
```rust
TextModelBuilder::new(model_id)
    .with_prefix_cache_n(Some(16))  // Cache 16 recent conversation prefixes
    .with_no_kv_cache(false)        // Keep KV cache for fast generation
```

**4. Streaming Responses**:
- Stream tokens as generated (no full response buffering)
- Update UI incrementally
- Cancel generation on user interrupt

**5. Background Processing**:
```rust
// Don't block UI thread
tokio::spawn(async move {
    // Ingest terminal session into RAG in background
    rag_client.ingest_session(session).await.ok();
});
```

### 6.3 Cargo Build Profiles

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 4
panic = "abort"
strip = "symbols"

[profile.release-small]
inherits = "release"
opt-level = "z"      # Optimize for size
lto = "fat"
codegen-units = 1

[profile.release-fast]
inherits = "release"
lto = "thin"
codegen-units = 16   # Faster compilation
```

---

## 7. Security & Sandboxing

### 7.1 Capability-Based Permissions

```rust
pub enum ModuleCapability {
    FileSystemRead,
    FileSystemWrite,
    ProcessSpawn,
    NetworkAccess,
    ClipboardAccess,
    UIOverlay,
    CustomDomain,
    CustomPane,
    LLMInference,
    ToolExecution,
}

impl ModuleManager {
    /// Check if module has required capability
    fn check_capability(&self, module_name: &str, cap: ModuleCapability) -> bool {
        if let Some(module) = self.modules.get(module_name) {
            module.metadata().capabilities.contains(&cap)
        } else {
            false
        }
    }
}
```

### 7.2 Path Validation (from rust-mcp-filesystem)

```rust
impl FileSystemService {
    pub fn validate_path(&self, requested_path: &Path) -> Result<PathBuf> {
        // 1. Expand home directory
        let expanded = expand_home(requested_path.to_path_buf());

        // 2. Resolve to absolute path
        let absolute = if expanded.is_absolute() {
            expanded
        } else {
            env::current_dir()?.join(&expanded)
        };

        // 3. Normalize (resolve .., ., symlinks)
        let normalized = normalize_path(&absolute);

        // 4. Check against allowed directories
        let is_allowed = self.allowed_dirs.iter().any(|allowed| {
            normalized.starts_with(allowed) ||
            normalized.starts_with(normalize_path(allowed))
        });

        if !is_allowed {
            return Err(anyhow::anyhow!("Access denied - path outside allowed directories"));
        }

        // 5. Detect symlink escapes
        if contains_symlink(&absolute)? {
            return Err(anyhow::anyhow!("Access denied - symlink target outside allowed dirs"));
        }

        Ok(absolute)
    }
}
```

### 7.3 Command Execution Sandboxing

```rust
pub struct SandboxConfig {
    pub allowed_commands: Option<HashSet<String>>,  // None = allow all
    pub blocked_commands: HashSet<String>,          // Explicit blacklist
    pub env_whitelist: HashSet<String>,             // Allowed env vars
    pub timeout: Duration,
    pub max_output_size: usize,
}

impl CommandExecutor {
    fn validate_command(&self, cmd: &str) -> Result<()> {
        // Check blacklist
        if self.config.blocked_commands.contains(cmd) {
            return Err(anyhow::anyhow!("Blocked command: {}", cmd));
        }

        // Check whitelist if configured
        if let Some(allowed) = &self.config.allowed_commands {
            if !allowed.contains(cmd) {
                return Err(anyhow::anyhow!("Command not in whitelist: {}", cmd));
            }
        }

        Ok(())
    }
}
```

---

## 8. Implementation Roadmap

### Phase 1: Core Framework (4-6 weeks)

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

### Phase 2: Filesystem & Commander Modules (2-3 weeks)

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

### Phase 3: LLM Integration (4-5 weeks)

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

### Phase 4: UI & UX (3-4 weeks)

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

### Phase 5: Advanced Features (2-3 weeks, optional)

**Week 19-20: gemma.cpp Integration**
- [ ] C++ FFI bindings
- [ ] Process-based communication
- [ ] Fallback engine selection

**Week 21: Advanced RAG**
- [ ] Vector index optimization
- [ ] Multi-tier memory system
- [ ] Conversation history management

### Phase 6: Release & Documentation (1-2 weeks)

**Week 22-23**
- [ ] User documentation
- [ ] API reference documentation
- [ ] Configuration guide
- [ ] Tutorial videos
- [ ] Beta release

---

## 9. API Reference

### 9.1 Lua API

#### wezterm.modules

```lua
-- List all loaded modules
local modules = wezterm.modules.list()
-- Returns: { {name="ai-assistant", version="0.1.0", status="loaded"}, ... }

-- Get module info
local info = wezterm.modules.get_info("ai-assistant")

-- Check module health
local health = wezterm.modules.health_check("ai-assistant")
```

#### wezterm.ai

```lua
-- Query AI assistant (blocking)
local response = wezterm.ai.query("How do I list all files recursively?")

-- Query with streaming callback
wezterm.ai.stream("Explain git rebase", function(chunk)
  print(chunk)  -- Called for each token
end)

-- Open AI assistant overlay
wezterm.action.ActivateAiAssistant

-- Configuration in wezterm.lua
config.modules = {
  ai_assistant = {
    enabled = true,
    model = {
      engine = "mistral",
      model_id = "microsoft/Phi-3.5-mini-instruct",
      quantization = "Q4_0",
    },
    ui = {
      keybinding = { key = 'a', mods = 'CTRL|SHIFT' },
    }
  }
}
```

#### wezterm.fs

```lua
-- Read file
local content = wezterm.fs.read_file("/path/to/file.txt")

-- Write file
wezterm.fs.write_file("/path/to/file.txt", "content")

-- Search files
local results = wezterm.fs.search_files("/path", "*.rs")
-- Returns: { "/path/main.rs", "/path/lib.rs", ... }

-- Search content
local matches = wezterm.fs.search_content("/path", "*.rs", "fn main")
-- Returns: { {file="/path/main.rs", line=1, text="fn main() {"}, ... }

-- List directory
local entries = wezterm.fs.list_directory("/path")
```

#### wezterm.cmd

```lua
-- Execute command
local result = wezterm.cmd.execute("ls", {"-la"})
-- Returns: { stdout="...", stderr="...", exit_code=0 }

-- Execute with timeout
local result = wezterm.cmd.execute_timeout("sleep", {"10"}, 5000)  -- 5s timeout
```

### 9.2 Rust Module API

#### Core Trait

```rust
#[async_trait(?Send)]
pub trait WezTermModule: Send + Sync {
    fn metadata(&self) -> ModuleMetadata;
    async fn initialize(&mut self, ctx: ModuleContext) -> anyhow::Result<()>;
    fn register_lua_api(&self, lua: &Lua) -> anyhow::Result<()>;
    fn register_events(&self) -> Vec<EventHandler>;
    async fn handle_message(&self, msg: ModuleMessage) -> anyhow::Result<ModuleMessage>;
    async fn shutdown(&mut self) -> anyhow::Result<()>;
    fn health_check(&self) -> ModuleHealth;
}
```

#### Creating a Module

```rust
use wezterm_module_framework::*;

pub struct MyModule {
    config: MyConfig,
}

#[async_trait(?Send)]
impl WezTermModule for MyModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: "my-module".to_string(),
            version: "0.1.0".to_string(),
            author: "Author".to_string(),
            description: "My custom module".to_string(),
            capabilities: vec![ModuleCapability::FileSystemRead],
            dependencies: vec![],
            lua_namespace: Some("my".to_string()),
        }
    }

    async fn initialize(&mut self, ctx: ModuleContext) -> anyhow::Result<()> {
        // Setup module
        Ok(())
    }

    fn register_lua_api(&self, lua: &Lua) -> anyhow::Result<()> {
        let module = lua.create_table()?;

        module.set("my_function", lua.create_function(|_lua, arg: String| {
            Ok(format!("Hello, {}", arg))
        })?)?;

        let wezterm: mlua::Table = lua.globals().get("wezterm")?;
        wezterm.set("my", module)?;

        Ok(())
    }

    // ... implement other methods
}

// Register as built-in module
pub fn register_builtin_modules(manager: &mut ModuleManager) {
    manager.register_builtin("my-module", || Box::new(MyModule::new()));
}
```

---

## 10. Configuration Examples

### 10.1 Basic AI Assistant

```lua
local wezterm = require 'wezterm'
local config = {}

config.enable_module_framework = true

config.modules = {
  ai_assistant = {
    enabled = true,
    model = {
      engine = "mistral",
      model_id = "microsoft/Phi-3.5-mini-instruct",
      quantization = "Q4_0",  -- Balance of speed/quality
      max_tokens = 2048,
      temperature = 0.7,
    },
    ui = {
      keybinding = { key = 'a', mods = 'CTRL|SHIFT' },
      position = "overlay",
    }
  }
}

return config
```

### 10.2 Advanced Configuration with RAG

```lua
config.modules = {
  ai_assistant = {
    enabled = true,
    model = {
      engine = "mistral",
      model_id = "mistralai/Mistral-7B-Instruct-v0.3",
      quantization = "Q8_0",  -- Higher quality
    },
    rag = {
      enabled = true,
      redis_url = "redis://localhost:6379",
      embedding_model = "sentence-transformers/all-MiniLM-L6-v2",
      auto_ingest = true,  -- Automatically ingest terminal sessions
      ingest_interval = 300,  -- Every 5 minutes
    },
    tools = {
      filesystem = true,
      command_execution = true,
      web_search = true,
    },
    system_prompt = [[
You are a helpful terminal assistant with deep knowledge of Unix/Linux systems.
Provide concise, accurate commands and explanations.
    ]],
  },

  filesystem = {
    enabled = true,
    allowed_directories = {
      wezterm.home_dir .. "/projects",
      wezterm.home_dir .. "/documents",
    },
    read_only = false,
  },
}
```

### 10.3 Custom Event Handlers

```lua
-- React to AI assistant responses
wezterm.on('ai-assistant-response', function(response)
  -- Log to file
  local log = io.open(wezterm.home_dir .. '/.wezterm-ai.log', 'a')
  log:write(os.date() .. ': ' .. response .. '\n')
  log:close()
end)

-- Custom keybinding
config.keys = {
  {
    key = 'a',
    mods = 'CTRL|SHIFT',
    action = wezterm.action.ActivateAiAssistant,
  },
  {
    key = 'q',
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      -- Quick query without opening overlay
      local text = window:get_selection_text_for_pane(pane)
      if text and #text > 0 then
        local response = wezterm.ai.query("Explain: " .. text)
        window:toast_notification('AI Assistant', response, nil, 10000)
      end
    end),
  },
}
```

---

## 11. Testing Strategy

### 11.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_module_initialization() {
        let mut module = AiAssistantModule::new(default_config());
        let ctx = test_context();

        let result = module.initialize(ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_llm_inference() {
        let module = setup_test_module().await;
        let response = module.query("Hello", test_context()).await.unwrap();
        assert!(!response.is_empty());
    }
}
```

### 11.2 Integration Tests

```rust
#[tokio::test]
async fn test_full_conversation_flow() {
    let manager = ModuleManager::new(test_config());
    manager.load_all_modules().await.unwrap();

    // Simulate user query
    let response = manager
        .ipc
        .request("ai-assistant", "query", json!({"prompt": "test"}))
        .await
        .unwrap();

    assert!(response["response"].as_str().unwrap().len() > 0);
}
```

### 11.3 Performance Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_module_loading(c: &mut Criterion) {
    c.bench_function("load ai-assistant module", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let module = AiAssistantModule::new(default_config());
                black_box(module)
            })
        })
    });
}

criterion_group!(benches, benchmark_module_loading);
criterion_main!(benches);
```

---

## 12. Future Enhancements

### 12.1 WASM Module Support

- Load modules compiled to WebAssembly
- Full sandboxing via WASM runtime
- Language-agnostic module development

### 12.2 Remote Module Registry

- Centralized module repository
- Version management and updates
- Community-contributed modules

### 12.3 Multi-Modal Support

- Vision models for screenshot analysis
- Audio transcription for voice commands
- Code generation from UI mockups

### 12.4 Advanced RAG Features

- Graph-based knowledge representation
- Multi-modal embeddings
- Automatic knowledge base curation

---

## Appendix A: Dependencies

### Core Dependencies

```toml
[dependencies]
# Already in WezTerm
tokio = { version = "1.45", features = ["full"] }
mlua = { version = "0.9", features = ["async", "serialize"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
async-trait = "0.1"

# New dependencies
mistralrs = "0.6"
redis = { version = "0.32", features = ["aio", "tokio-comp"] }
uuid = { version = "1.0", features = ["v4"] }
```

### Build Dependencies

```toml
[build-dependencies]
cbindgen = "0.26"  # For gemma.cpp FFI (optional)
```

---

## Appendix B: File Structure

```
wezterm/
├── Cargo.toml                              # Add workspace members
├── wezterm-module-framework/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                          # Module trait + core types
│       ├── manager.rs                      # Module manager
│       ├── ipc.rs                          # Inter-module IPC
│       ├── loader.rs                       # Module discovery/loading
│       └── registry.rs                     # Module registry
│
├── lua-api-crates/
│   └── module-framework/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs                      # Lua API bindings
│
├── wezterm-builtin-modules/
│   ├── ai-assistant/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # Module implementation
│   │       ├── llm.rs                      # LLM integration layer
│   │       ├── rag.rs                      # RAG system client
│   │       ├── pane.rs                     # UI overlay pane
│   │       └── tools.rs                    # Tool execution
│   │
│   ├── filesystem/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # Module implementation
│   │       ├── service.rs                  # Filesystem service
│   │       └── tools/                      # 16 filesystem tools
│   │
│   └── commander/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                      # Module implementation
│           └── executor.rs                 # Command executor
│
└── config/src/
    └── lua.rs                              # Hook module framework here
```

---

## Appendix C: References

1. **mistral.rs**: https://github.com/EricLBuehler/mistral.rs
2. **gemma.cpp**: https://github.com/google/gemma.cpp
3. **rust-mcp-filesystem**: Reference implementation for MCP servers
4. **Warp Terminal AI**: https://www.warp.dev/warp-ai
5. **Model Context Protocol**: https://modelcontextprotocol.io/
6. **WezTerm Documentation**: https://wezfurlong.org/wezterm/

---

**Document Version**: 1.0
**Last Updated**: 2025-01-30
**Status**: Design Specification - Ready for Implementation