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

    let daemon_mod = get_or_create_sub_module(lua, "daemon")?;

    // -- wezterm.daemon.ping() -> bool -----------------------------------
    daemon_mod.set(
        "ping",
        lua.create_function(|_, ()| {
            match blocking_ping() {
                Ok(_) => Ok(true),
                Err(e) => {
                    log::debug!("daemon.ping failed: {e}");
                    Ok(false)
                }
            }
        })?,
    )?;

    // -- wezterm.daemon.status() -> table | nil --------------------------
    daemon_mod.set(
        "status",
        lua.create_function(|lua_ctx, ()| {
            match blocking_status() {
                Ok(value) => json_value_to_lua(lua_ctx, value),
                Err(e) => {
                    log::debug!("daemon.status failed: {e}");
                    Ok(mlua::Value::Nil)
                }
            }
        })?,
    )?;

    // -- wezterm.daemon.broadcast(event_type, data) -> table | nil -------
    daemon_mod.set(
        "broadcast",
        lua.create_function(|lua_ctx, (event_type, data): (String, mlua::Value)| {
            let json_data = lua_to_json_value(data, &mut std::collections::HashSet::new())?;
            match blocking_broadcast(event_type, json_data) {
                Ok(value) => json_value_to_lua(lua_ctx, value),
                Err(e) => {
                    log::debug!("daemon.broadcast failed: {e}");
                    Ok(mlua::Value::Nil)
                }
            }
        })?,
    )?;

    // -- wezterm.daemon.register(name, capabilities) -> table | nil ------
    daemon_mod.set(
        "register",
        lua.create_function(|lua_ctx, (name, caps): (String, Vec<String>)| {
            match blocking_register(name, caps) {
                Ok(value) => json_value_to_lua(lua_ctx, value),
                Err(e) => {
                    log::debug!("daemon.register failed: {e}");
                    Ok(mlua::Value::Nil)
                }
            }
        })?,
    )?;

    log::debug!("Registered wezterm.daemon Lua API");
    Ok(())
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
// Each function below spins up a lightweight current-thread tokio runtime,
// connects to the daemon, runs one request, and tears down the runtime.
// This is appropriate for infrequent calls (panel toggles, status checks)
// where the ~1 ms overhead of runtime construction is negligible.

/// Create a throwaway tokio runtime and connect to the daemon.
#[cfg(feature = "daemon-ipc")]
fn new_runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("failed to create tokio runtime: {e}"))
}

#[cfg(feature = "daemon-ipc")]
fn blocking_ping() -> anyhow::Result<serde_json::Value> {
    let rt = new_runtime()?;
    rt.block_on(async {
        let client = DaemonClient::connect()
            .await
            .map_err(|e| anyhow::anyhow!("daemon connection failed: {e}"))?;
        client
            .ping()
            .await
            .map_err(|e| anyhow::anyhow!("daemon ping failed: {e}"))
    })
}

#[cfg(feature = "daemon-ipc")]
fn blocking_status() -> anyhow::Result<serde_json::Value> {
    let rt = new_runtime()?;
    rt.block_on(async {
        let client = DaemonClient::connect()
            .await
            .map_err(|e| anyhow::anyhow!("daemon connection failed: {e}"))?;
        client
            .status()
            .await
            .map_err(|e| anyhow::anyhow!("daemon status failed: {e}"))
    })
}

#[cfg(feature = "daemon-ipc")]
fn blocking_broadcast(
    event_type: String,
    data: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let rt = new_runtime()?;
    rt.block_on(async {
        let client = DaemonClient::connect()
            .await
            .map_err(|e| anyhow::anyhow!("daemon connection failed: {e}"))?;
        client
            .broadcast(&event_type, &data)
            .await
            .map_err(|e| anyhow::anyhow!("daemon broadcast failed: {e}"))
    })
}

#[cfg(feature = "daemon-ipc")]
fn blocking_register(
    name: String,
    capabilities: Vec<String>,
) -> anyhow::Result<serde_json::Value> {
    let rt = new_runtime()?;
    rt.block_on(async {
        let client = DaemonClient::connect()
            .await
            .map_err(|e| anyhow::anyhow!("daemon connection failed: {e}"))?;
        client
            .register(&name, capabilities)
            .await
            .map_err(|e| anyhow::anyhow!("daemon register failed: {e}"))
    })
}

// ---------------------------------------------------------------------------
// serde_json::Value <-> mlua::Value conversion
// ---------------------------------------------------------------------------

/// Convert a `serde_json::Value` into an `mlua::Value`.
///
/// Objects become Lua tables, arrays become 1-indexed Lua tables, numbers
/// are represented as integers when possible, and `null` maps to `nil`.
#[cfg(feature = "daemon-ipc")]
fn json_value_to_lua<'lua>(
    lua: &'lua mlua::Lua,
    value: serde_json::Value,
) -> mlua::Result<mlua::Value<'lua>> {
    use mlua::IntoLua;

    Ok(match value {
        serde_json::Value::Null => mlua::Value::Nil,
        serde_json::Value::Bool(b) => mlua::Value::Boolean(b),
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(i) => mlua::Value::Integer(i),
            None => match n.as_f64() {
                Some(f) => mlua::Value::Number(f),
                None => {
                    return Err(mlua::Error::external(format!(
                        "cannot represent {n:?} as i64 or f64"
                    )));
                }
            },
        },
        serde_json::Value::String(s) => s.into_lua(lua)?,
        serde_json::Value::Array(arr) => {
            let tbl = lua.create_table_with_capacity(arr.len(), 0)?;
            for (idx, v) in arr.into_iter().enumerate() {
                tbl.set(idx + 1, json_value_to_lua(lua, v)?)?;
            }
            mlua::Value::Table(tbl)
        }
        serde_json::Value::Object(map) => {
            let tbl = lua.create_table_with_capacity(0, map.len())?;
            for (key, v) in map {
                let lua_key = key.into_lua(lua)?;
                let lua_val = json_value_to_lua(lua, v)?;
                tbl.set(lua_key, lua_val)?;
            }
            mlua::Value::Table(tbl)
        }
    })
}

/// Convert an `mlua::Value` into a `serde_json::Value`.
///
/// The `visited` set guards against circular table references.
#[cfg(feature = "daemon-ipc")]
fn lua_to_json_value(
    value: mlua::Value,
    visited: &mut std::collections::HashSet<usize>,
) -> mlua::Result<serde_json::Value> {
    match value {
        mlua::Value::Nil => Ok(serde_json::Value::Null),
        mlua::Value::Boolean(b) => Ok(serde_json::Value::Bool(b)),
        mlua::Value::Integer(i) => Ok(serde_json::json!(i)),
        mlua::Value::Number(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| mlua::Error::external(format!("cannot represent {f} in JSON"))),
        mlua::Value::String(s) => Ok(serde_json::Value::String(s.to_str()?.to_owned())),
        mlua::Value::Table(tbl) => {
            let ptr = tbl.to_pointer() as usize;
            if !visited.insert(ptr) {
                // Circular reference -- return null to avoid infinite recursion
                return Ok(serde_json::Value::Null);
            }

            // Detect whether this is an array (sequential integer keys starting at 1)
            // or an object (string keys).
            let len = tbl.raw_len();
            if len > 0 {
                // Treat as array
                let mut arr = Vec::with_capacity(len);
                for i in 1..=len {
                    let v: mlua::Value = tbl.raw_get(i)?;
                    arr.push(lua_to_json_value(v, visited)?);
                }
                visited.remove(&ptr);
                Ok(serde_json::Value::Array(arr))
            } else {
                // Treat as object
                let mut map = serde_json::Map::new();
                for pair in tbl.pairs::<mlua::Value, mlua::Value>() {
                    let (k, v) = pair?;
                    let key = match k {
                        mlua::Value::String(s) => s.to_str()?.to_owned(),
                        mlua::Value::Integer(i) => i.to_string(),
                        mlua::Value::Number(f) => f.to_string(),
                        _ => continue, // skip non-stringifiable keys
                    };
                    map.insert(key, lua_to_json_value(v, visited)?);
                }
                visited.remove(&ptr);
                Ok(serde_json::Value::Object(map))
            }
        }
        // Other Lua types (function, userdata, etc.) -> null
        _ => Ok(serde_json::Value::Null),
    }
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
        let package: mlua::Table = lua
            .globals()
            .get("package")
            .expect("package should exist");
        let loaded: mlua::Table = package.get("loaded").expect("loaded should exist");
        let wezterm: mlua::Table = loaded.get("wezterm").expect("wezterm module should exist");
        let daemon: mlua::Table = wezterm.get("daemon").expect("daemon sub-module should exist");

        // ping should be callable and return false (no daemon running)
        let ping_fn: mlua::Function = daemon.get("ping").expect("ping function should exist");
        let result: bool = ping_fn.call(()).expect("ping should not error");
        assert!(!result, "ping should return false when daemon is not running");
    }

    #[cfg(feature = "daemon-ipc")]
    mod conversion_tests {
        use super::super::*;
        use serde_json::json;
        use std::collections::HashSet;

        #[test]
        fn test_json_null_to_lua() {
            let lua = mlua::Lua::new();
            let result = json_value_to_lua(&lua, json!(null)).expect("conversion");
            assert!(matches!(result, mlua::Value::Nil));
        }

        #[test]
        fn test_json_bool_to_lua() {
            let lua = mlua::Lua::new();
            let result = json_value_to_lua(&lua, json!(true)).expect("conversion");
            assert!(matches!(result, mlua::Value::Boolean(true)));
        }

        #[test]
        fn test_json_integer_to_lua() {
            let lua = mlua::Lua::new();
            let result = json_value_to_lua(&lua, json!(42)).expect("conversion");
            assert!(matches!(result, mlua::Value::Integer(42)));
        }

        #[test]
        fn test_json_float_to_lua() {
            let lua = mlua::Lua::new();
            let result = json_value_to_lua(&lua, json!(3.14)).expect("conversion");
            match result {
                mlua::Value::Number(n) => assert!((n - 3.14).abs() < f64::EPSILON),
                other => panic!("expected Number, got {other:?}"),
            }
        }

        #[test]
        fn test_json_string_to_lua() {
            let lua = mlua::Lua::new();
            let result = json_value_to_lua(&lua, json!("hello")).expect("conversion");
            match result {
                mlua::Value::String(s) => assert_eq!(s.to_str().expect("str"), "hello"),
                other => panic!("expected String, got {other:?}"),
            }
        }

        #[test]
        fn test_json_object_to_lua() {
            let lua = mlua::Lua::new();
            let result =
                json_value_to_lua(&lua, json!({"key": "value", "num": 7})).expect("conversion");
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
            let result = json_value_to_lua(&lua, json!([1, 2, 3])).expect("conversion");
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

            let result =
                lua_to_json_value(mlua::Value::Table(tbl), &mut HashSet::new())
                    .expect("conversion");
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
            let lua_val = json_value_to_lua(&lua, original.clone()).expect("to lua");
            let back = lua_to_json_value(lua_val, &mut HashSet::new()).expect("to json");
            assert_eq!(original, back);
        }
    }
}
