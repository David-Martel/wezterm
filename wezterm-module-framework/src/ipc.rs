//! Optional IPC bridge to wezterm-utils-daemon for cross-window panel state sync.
//!
//! When the `daemon-ipc` feature is enabled, modules can use the daemon
//! client to register, subscribe to events, and broadcast state changes
//! across WezTerm windows.
//!
//! When the feature is disabled, [`try_connect`] returns `None` immediately,
//! allowing modules to operate in standalone mode without conditional
//! compilation at the call site.
//!
//! ## Lua API
//!
//! When `daemon-ipc` is enabled, this module registers a `wezterm.daemon`
//! sub-module with the following functions:
//!
//! ```lua
//! -- Health check (returns true/false)
//! wezterm.daemon.ping()
//!
//! -- Get daemon status (returns table with version, uptime, etc. or nil)
//! wezterm.daemon.status()
//!
//! -- Broadcast an event to all subscribers
//! wezterm.daemon.broadcast('panel-state', { explorer = true })
//!
//! -- Register this client with the daemon
//! wezterm.daemon.register('window-42', {'state-sync'})
//! ```

#[cfg(feature = "daemon-ipc")]
pub use wezterm_utils_daemon::client::DaemonClient;

/// Try to connect to the running daemon instance.
///
/// Returns `Some(DaemonClient)` if the daemon is running and reachable,
/// `None` otherwise. Modules should fall back to standalone mode when
/// this returns `None`.
#[cfg(feature = "daemon-ipc")]
pub async fn try_connect() -> Option<DaemonClient> {
    match DaemonClient::connect().await {
        Ok(client) => {
            log::info!("Connected to wezterm-utils-daemon");
            Some(client)
        }
        Err(e) => {
            log::debug!("Daemon not available (standalone mode): {e}");
            None
        }
    }
}

/// Stub when the `daemon-ipc` feature is disabled.
///
/// Always returns `None` -- modules continue in standalone mode.
#[cfg(not(feature = "daemon-ipc"))]
pub async fn try_connect() -> Option<()> {
    log::debug!("Daemon IPC disabled at compile time");
    None
}

// ---------------------------------------------------------------------------
// Lua API registration
// ---------------------------------------------------------------------------

/// Register `wezterm.daemon.*` Lua bindings backed by the daemon client.
///
/// Each Lua call creates a short-lived tokio current-thread runtime,
/// connects to the daemon, executes one request, and returns the result.
/// This keeps the implementation simple and avoids holding long-lived
/// connections across Lua context reloads.
///
/// When the daemon is not running, functions degrade gracefully:
/// - `ping()` returns `false`
/// - `status()` returns `nil`
/// - `broadcast()` / `register()` return `nil`
#[cfg(feature = "daemon-ipc")]
pub fn register_lua_api(lua: &mlua::Lua) -> anyhow::Result<()> {
    use config::lua::get_or_create_sub_module;
    use mlua::LuaSerdeExt;

    let daemon_mod = get_or_create_sub_module(lua, "daemon")?;

    // -- wezterm.daemon.ping() -> bool -----------------------------------
    daemon_mod.set(
        "ping",
        lua.create_function(|_, ()| {
            let ok = blocking_call(|c| async move { c.ping().await }).is_ok();
            Ok(ok)
        })?,
    )?;

    // -- wezterm.daemon.status() -> table | nil --------------------------
    daemon_mod.set(
        "status",
        lua.create_function(|lua_ctx, ()| {
            blocking_call_to_lua(lua_ctx, "status", |c| async move { c.status().await })
        })?,
    )?;

    // -- wezterm.daemon.broadcast(event_type, data) -> table | nil -------
    daemon_mod.set(
        "broadcast",
        lua.create_function(|lua_ctx, (event_type, data): (String, mlua::Value)| {
            let json_data: serde_json::Value = lua_ctx.from_value(data)?;
            blocking_call_to_lua(lua_ctx, "broadcast", |c| async move {
                c.broadcast(&event_type, &json_data).await
            })
        })?,
    )?;

    // -- wezterm.daemon.register(name, capabilities) -> table | nil ------
    daemon_mod.set(
        "register",
        lua.create_function(|lua_ctx, (name, caps): (String, Vec<String>)| {
            blocking_call_to_lua(lua_ctx, "register", |c| async move {
                c.register(&name, caps).await
            })
        })?,
    )?;

    log::debug!("Registered wezterm.daemon Lua API");
    Ok(())
}

/// Execute a daemon call and convert the result to a Lua value.
///
/// On success, converts the `serde_json::Value` to an `mlua::Value` via
/// `LuaSerdeExt`. On failure, logs a debug message and returns `nil`.
#[cfg(feature = "daemon-ipc")]
fn blocking_call_to_lua<'lua, F, Fut, E>(
    lua: &'lua mlua::Lua,
    label: &str,
    op: F,
) -> mlua::Result<mlua::Value<'lua>>
where
    F: FnOnce(DaemonClient) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, E>>,
    E: Into<anyhow::Error>,
{
    use mlua::LuaSerdeExt;

    match blocking_call(op) {
        Ok(value) => lua.to_value(&value),
        Err(e) => {
            log::debug!("daemon.{label} failed: {e}");
            Ok(mlua::Value::Nil)
        }
    }
}

/// Stub when daemon-ipc is disabled -- registers an empty `wezterm.daemon`
/// table so Lua scripts can check for its existence without errors.
#[cfg(not(feature = "daemon-ipc"))]
pub fn register_lua_api(lua: &mlua::Lua) -> anyhow::Result<()> {
    use config::lua::get_or_create_sub_module;

    let daemon_mod = get_or_create_sub_module(lua, "daemon")?;

    // Provide a ping stub that always returns false so scripts can detect
    // the missing daemon without pcall gymnastics.
    daemon_mod.set("ping", lua.create_function(|_, ()| Ok(false))?)?;

    log::debug!("Registered wezterm.daemon Lua API (stub -- daemon-ipc disabled)");
    Ok(())
}

// ---------------------------------------------------------------------------
// Blocking bridge: sync Lua context -> async daemon client
// ---------------------------------------------------------------------------
//
// Each call spins up a lightweight current-thread tokio runtime, connects
// to the daemon, runs one request, and tears down the runtime.
// This is appropriate for infrequent calls (panel toggles, status checks)
// where the ~1 ms overhead of runtime construction is negligible.

/// Connect to the daemon and execute `op` inside a throwaway tokio runtime.
///
/// The closure receives a connected [`DaemonClient`] and returns a
/// `serde_json::Value` result. Connection and runtime errors are
/// propagated as `anyhow::Error`.
#[cfg(feature = "daemon-ipc")]
fn blocking_call<F, Fut, E>(op: F) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce(DaemonClient) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, E>>,
    E: Into<anyhow::Error>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("failed to create tokio runtime: {e}"))?;

    rt.block_on(async {
        let client = DaemonClient::connect()
            .await
            .map_err(|e| anyhow::anyhow!("daemon connection failed: {e}"))?;
        op(client).await.map_err(Into::into)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_register_lua_api_does_not_panic() {
        // Smoke test: creating a Lua state and registering the daemon API
        // should not panic, even without a running daemon.
        let lua = mlua::Lua::new();

        // get_or_create_sub_module uses package.loaded["wezterm"], which
        // Lua::new() provides via the standard library. No manual setup
        // is needed.
        super::register_lua_api(&lua).expect("register_lua_api should succeed");

        // Retrieve the wezterm module from package.loaded
        let package: mlua::Table = lua.globals().get("package").expect("package should exist");
        let loaded: mlua::Table = package.get("loaded").expect("loaded should exist");
        let wezterm: mlua::Table = loaded.get("wezterm").expect("wezterm module should exist");
        let daemon: mlua::Table = wezterm
            .get("daemon")
            .expect("daemon sub-module should exist");

        // ping should be callable and return false (no daemon running)
        let ping_fn: mlua::Function = daemon.get("ping").expect("ping function should exist");
        let result: bool = ping_fn.call(()).expect("ping should not error");
        assert!(
            !result,
            "ping should return false when daemon is not running"
        );
    }

    #[cfg(feature = "daemon-ipc")]
    mod conversion_tests {
        use mlua::LuaSerdeExt;
        use serde_json::json;

        #[test]
        fn test_json_null_to_lua() {
            let lua = mlua::Lua::new();
            let result = lua.to_value(&json!(null)).expect("conversion");
            // LuaSerdeExt represents JSON null as a special NULL light
            // userdata (not Lua nil), so that null values survive in tables.
            assert!(result.is_null(), "expected NULL sentinel, got {result:?}");
        }

        #[test]
        fn test_json_bool_to_lua() {
            let lua = mlua::Lua::new();
            let result = lua.to_value(&json!(true)).expect("conversion");
            assert!(matches!(result, mlua::Value::Boolean(true)));
        }

        #[test]
        fn test_json_integer_to_lua() {
            let lua = mlua::Lua::new();
            let result = lua.to_value(&json!(42)).expect("conversion");
            assert!(matches!(result, mlua::Value::Integer(42)));
        }

        #[test]
        fn test_json_float_to_lua() {
            let lua = mlua::Lua::new();
            let result = lua.to_value(&json!(3.14)).expect("conversion");
            match result {
                mlua::Value::Number(n) => assert!((n - 3.14).abs() < f64::EPSILON),
                other => panic!("expected Number, got {other:?}"),
            }
        }

        #[test]
        fn test_json_string_to_lua() {
            let lua = mlua::Lua::new();
            let result = lua.to_value(&json!("hello")).expect("conversion");
            match result {
                mlua::Value::String(s) => assert_eq!(s.to_str().expect("str"), "hello"),
                other => panic!("expected String, got {other:?}"),
            }
        }

        #[test]
        fn test_json_object_to_lua() {
            let lua = mlua::Lua::new();
            let result = lua
                .to_value(&json!({"key": "value", "num": 7}))
                .expect("conversion");
            match result {
                mlua::Value::Table(tbl) => {
                    let val: String = tbl.get("key").expect("get key");
                    assert_eq!(val, "value");
                    let num: i64 = tbl.get("num").expect("get num");
                    assert_eq!(num, 7);
                }
                other => panic!("expected Table, got {other:?}"),
            }
        }

        #[test]
        fn test_json_array_to_lua() {
            let lua = mlua::Lua::new();
            let result = lua.to_value(&json!([1, 2, 3])).expect("conversion");
            match result {
                mlua::Value::Table(tbl) => {
                    let v1: i64 = tbl.get(1).expect("get 1");
                    let v2: i64 = tbl.get(2).expect("get 2");
                    let v3: i64 = tbl.get(3).expect("get 3");
                    assert_eq!((v1, v2, v3), (1, 2, 3));
                }
                other => panic!("expected Table, got {other:?}"),
            }
        }

        #[test]
        fn test_lua_table_to_json_object() {
            let lua = mlua::Lua::new();
            let tbl = lua.create_table().expect("create table");
            tbl.set("explorer", true).expect("set");
            tbl.set("watcher", false).expect("set");

            let result: serde_json::Value =
                lua.from_value(mlua::Value::Table(tbl)).expect("conversion");
            match result {
                serde_json::Value::Object(map) => {
                    assert_eq!(map.get("explorer"), Some(&json!(true)));
                    assert_eq!(map.get("watcher"), Some(&json!(false)));
                }
                other => panic!("expected Object, got {other:?}"),
            }
        }

        #[test]
        fn test_roundtrip_json_lua_json() {
            let lua = mlua::Lua::new();
            let original = json!({"panels": {"explorer": true}, "count": 3});
            let lua_val = lua.to_value(&original).expect("to lua");
            let back: serde_json::Value = lua.from_value(lua_val).expect("to json");
            assert_eq!(original, back);
        }
    }
}
