//! Rust-native configuration validators.
//!
//! Provides path safety checks and binary existence validation that were
//! previously implemented in `codex_ui/validator.lua`.  These checks are
//! pure filesystem / path logic and do not depend on the Lua runtime.
//!
//! Exposed to Lua via `wezterm.validation.*` so that the existing
//! `validator.lua` can delegate the filesystem-heavy work to Rust while
//! keeping the Lua-specific checks (module resolution, `package.path`).
//!
//! ## Functions
//!
//! | Lua API | Rust function |
//! |---------|---------------|
//! | `wezterm.validation.check_state_dir(state_dir, config_file, home_dir)` | [`validate_state_dir_safety`] |
//! | `wezterm.validation.check_binaries(binaries_table)` | [`validate_binary_paths`] |
//! | `wezterm.validation.check_config_paths(config_file, config_dir)` | [`validate_config_paths`] |

use config::lua::get_or_create_sub_module;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Outcome of a validation pass.
///
/// Mirrors the `{errors = [], warnings = []}` table returned by the Lua
/// validators so that both sides use the same shape.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ValidationResult {
    /// Hard errors that should block startup / config reload.
    pub errors: Vec<String>,
    /// Soft warnings that are logged but do not block.
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Returns `true` when no errors were recorded.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Merge another result into this one (consumes `other`).
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

// ---------------------------------------------------------------------------
// Validators
// ---------------------------------------------------------------------------

/// Check that `state_dir` is not under any config-reload-watched directory.
///
/// WezTerm watches certain directories for config changes and reloads when
/// files in them are modified.  If `state_dir` is placed inside one of
/// those directories, the panel state writes will trigger an infinite
/// reload loop.
///
/// The reload-sensitive roots are:
/// 1. The parent directory of `config_file` (unless it is `$HOME/.wezterm.lua`).
/// 2. `$HOME/.config/wezterm/` (if it exists).
pub fn validate_state_dir_safety(
    state_dir: &Path,
    config_file: Option<&Path>,
    home_dir: Option<&Path>,
) -> ValidationResult {
    let mut result = ValidationResult::default();

    let mut roots: Vec<PathBuf> = Vec::new();

    if let Some(cf) = config_file {
        // The home-dir config file ($HOME/.wezterm.lua) is not inside a
        // dedicated config directory, so its parent is just $HOME which is
        // too broad to treat as a reload root.  Skip it.
        let is_home_config = home_dir
            .map(|home| {
                let home_config = home.join(".wezterm.lua");
                normalize_path(&home_config) == normalize_path(cf)
            })
            .unwrap_or(false);

        if !is_home_config {
            if let Some(parent) = cf.parent() {
                roots.push(parent.to_path_buf());
            }
        }
    }

    if let Some(home) = home_dir {
        let wezterm_config = home.join(".config").join("wezterm");
        if wezterm_config.exists() {
            roots.push(wezterm_config);
        }
    }

    let state_canonical = normalize_path(state_dir);
    for root in &roots {
        let root_canonical = normalize_path(root);
        if is_within_path(&root_canonical, &state_canonical) {
            result.errors.push(format!(
                "state_dir must stay outside config/reload roots to avoid reload loops: \
                 {} (under {})",
                state_dir.display(),
                root.display()
            ));
            break;
        }
    }

    result
}

/// Check that expected utility binaries exist on disk.
///
/// Each entry is a `(display_name, path)` pair.  Missing binaries produce
/// warnings (not errors) because they are optional utilities.
pub fn validate_binary_paths(binaries: &[(&str, &Path)]) -> ValidationResult {
    let mut result = ValidationResult::default();
    for (name, path) in binaries {
        if !path.exists() {
            result.warnings.push(format!(
                "optional {} binary missing: {}",
                name,
                path.display()
            ));
        }
    }
    result
}

/// Verify that the active config file and config directory exist.
pub fn validate_config_paths(
    config_file: Option<&Path>,
    config_dir: Option<&Path>,
) -> ValidationResult {
    let mut result = ValidationResult::default();
    if let Some(cf) = config_file {
        if !cf.exists() {
            result.errors.push(format!(
                "wezterm.config_file does not exist: {}",
                cf.display()
            ));
        }
    }
    if let Some(cd) = config_dir {
        if !cd.is_dir() {
            result.errors.push(format!(
                "wezterm.config_dir does not exist: {}",
                cd.display()
            ));
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Normalize a path for case-insensitive, separator-agnostic comparison.
///
/// On Windows this lower-cases the path and replaces `/` with `\`.
/// On Unix it only collapses duplicate separators.
pub fn normalize_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    #[cfg(windows)]
    {
        let normalized = s.replace('/', "\\").to_lowercase();
        // Collapse runs of backslashes
        let collapsed = collapse_separators(&normalized, '\\');
        PathBuf::from(collapsed)
    }
    #[cfg(not(windows))]
    {
        let collapsed = collapse_separators(&s, '/');
        PathBuf::from(collapsed)
    }
}

/// Collapse consecutive occurrences of `sep` into a single one.
fn collapse_separators(s: &str, sep: char) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_was_sep = false;
    for c in s.chars() {
        if c == sep {
            if !prev_was_sep {
                out.push(c);
            }
            prev_was_sep = true;
        } else {
            out.push(c);
            prev_was_sep = false;
        }
    }
    out
}

/// Returns `true` if `candidate` is equal to or a child of `parent`.
///
/// Both paths should already be normalized via [`normalize_path`].
pub fn is_within_path(parent: &Path, candidate: &Path) -> bool {
    let parent_str = parent.to_string_lossy();
    let candidate_str = candidate.to_string_lossy();

    if parent_str.is_empty() || candidate_str.is_empty() {
        return false;
    }

    if parent_str == candidate_str {
        return true;
    }

    #[cfg(windows)]
    let sep = '\\';
    #[cfg(not(windows))]
    let sep = '/';

    candidate_str.starts_with(&format!("{}{}", parent_str, sep))
}

// ---------------------------------------------------------------------------
// Lua API
// ---------------------------------------------------------------------------

/// Register `wezterm.validation.*` Lua functions.
///
/// ```lua
/// local v = require('wezterm').validation
///
/// -- Check state directory safety
/// local r = v.check_state_dir(state_dir, config_file, home_dir)
/// -- r.errors: string[]
/// -- r.warnings: string[]
///
/// -- Check binary existence
/// local r = v.check_binaries({
///     { name = "explorer", path = "C:/Users/david/bin/wezterm-fs-explorer.exe" },
///     { name = "watcher",  path = "C:/Users/david/bin/wezterm-watch.exe" },
/// })
///
/// -- Check config file / dir existence
/// local r = v.check_config_paths(config_file, config_dir)
/// ```
pub fn register_lua_api(lua: &mlua::Lua) -> anyhow::Result<()> {
    let validation_mod = get_or_create_sub_module(lua, "validation")?;

    // -- wezterm.validation.check_state_dir(state_dir, config_file, home_dir)
    validation_mod.set(
        "check_state_dir",
        lua.create_function(
            |lua_ctx,
             (state_dir, config_file, home_dir): (
                String,
                mlua::Value,
                mlua::Value,
            )| {
                let cf = match &config_file {
                    mlua::Value::String(s) => Some(PathBuf::from(s.to_str()?)),
                    _ => None,
                };
                let hd = match &home_dir {
                    mlua::Value::String(s) => Some(PathBuf::from(s.to_str()?)),
                    _ => None,
                };

                let result = validate_state_dir_safety(
                    Path::new(&state_dir),
                    cf.as_deref(),
                    hd.as_deref(),
                );

                result_to_lua_table(lua_ctx, &result)
            },
        )?,
    )?;

    // -- wezterm.validation.check_binaries(binaries_table)
    validation_mod.set(
        "check_binaries",
        lua.create_function(|lua_ctx, table: mlua::Table| {
            let mut binaries: Vec<(String, PathBuf)> = Vec::new();

            for pair in table.sequence_values::<mlua::Table>() {
                let entry = pair?;
                let name: String = entry.get("name")?;
                let path: String = entry.get("path")?;
                binaries.push((name, PathBuf::from(path)));
            }

            let refs: Vec<(&str, &Path)> = binaries
                .iter()
                .map(|(n, p)| (n.as_str(), p.as_path()))
                .collect();

            let result = validate_binary_paths(&refs);
            result_to_lua_table(lua_ctx, &result)
        })?,
    )?;

    // -- wezterm.validation.check_config_paths(config_file, config_dir)
    validation_mod.set(
        "check_config_paths",
        lua.create_function(
            |lua_ctx, (config_file, config_dir): (mlua::Value, mlua::Value)| {
                let cf = match &config_file {
                    mlua::Value::String(s) => Some(PathBuf::from(s.to_str()?)),
                    _ => None,
                };
                let cd = match &config_dir {
                    mlua::Value::String(s) => Some(PathBuf::from(s.to_str()?)),
                    _ => None,
                };

                let result =
                    validate_config_paths(cf.as_deref(), cd.as_deref());
                result_to_lua_table(lua_ctx, &result)
            },
        )?,
    )?;

    log::debug!("Registered wezterm.validation Lua API");
    Ok(())
}

/// Convert a [`ValidationResult`] into a Lua table `{errors=[], warnings=[]}`.
fn result_to_lua_table<'lua>(
    lua: &'lua mlua::Lua,
    result: &ValidationResult,
) -> mlua::Result<mlua::Table<'lua>> {
    let table = lua.create_table()?;

    let errors = lua.create_table()?;
    for (i, e) in result.errors.iter().enumerate() {
        errors.set(i + 1, e.as_str())?;
    }

    let warnings = lua.create_table()?;
    for (i, w) in result.warnings.iter().enumerate() {
        warnings.set(i + 1, w.as_str())?;
    }

    table.set("errors", errors)?;
    table.set("warnings", warnings)?;

    Ok(table)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // -- normalize_path -------------------------------------------------------

    #[test]
    fn test_normalize_path_forward_slashes() {
        let p = normalize_path(Path::new("C:/Users/david/.config/wezterm"));
        #[cfg(windows)]
        assert_eq!(
            p,
            PathBuf::from("c:\\users\\david\\.config\\wezterm")
        );
        #[cfg(not(windows))]
        assert_eq!(
            p,
            PathBuf::from("C:/Users/david/.config/wezterm")
        );
    }

    #[test]
    fn test_normalize_path_collapses_separators() {
        #[cfg(windows)]
        {
            let p = normalize_path(Path::new("C:\\\\Users\\\\david"));
            assert_eq!(p, PathBuf::from("c:\\users\\david"));
        }
        #[cfg(not(windows))]
        {
            let p = normalize_path(Path::new("/home//david"));
            assert_eq!(p, PathBuf::from("/home/david"));
        }
    }

    #[test]
    fn test_normalize_path_empty() {
        let p = normalize_path(Path::new(""));
        assert_eq!(p, PathBuf::from(""));
    }

    // -- is_within_path -------------------------------------------------------

    #[test]
    fn test_is_within_path_exact_match() {
        let parent = normalize_path(Path::new("/home/david/.config/wezterm"));
        let child = normalize_path(Path::new("/home/david/.config/wezterm"));
        assert!(is_within_path(&parent, &child));
    }

    #[test]
    fn test_is_within_path_child() {
        let parent = normalize_path(Path::new("/home/david/.config"));
        let child = normalize_path(Path::new("/home/david/.config/wezterm/state"));
        assert!(is_within_path(&parent, &child));
    }

    #[test]
    fn test_is_within_path_not_child() {
        let parent = normalize_path(Path::new("/home/david/.config/wezterm"));
        let child = normalize_path(Path::new("/home/david/.local/state"));
        assert!(!is_within_path(&parent, &child));
    }

    #[test]
    fn test_is_within_path_prefix_overlap_not_ancestor() {
        // "wezterm-state" starts with "wezterm" but is not a child
        let parent = normalize_path(Path::new("/home/david/.config/wezterm"));
        let candidate =
            normalize_path(Path::new("/home/david/.config/wezterm-state"));
        assert!(!is_within_path(&parent, &candidate));
    }

    #[test]
    fn test_is_within_path_empty_strings() {
        assert!(!is_within_path(Path::new(""), Path::new("/foo")));
        assert!(!is_within_path(Path::new("/foo"), Path::new("")));
        assert!(!is_within_path(Path::new(""), Path::new("")));
    }

    // -- validate_state_dir_safety --------------------------------------------

    #[test]
    fn test_state_dir_under_config_is_error() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).expect("mkdir config");

        let config_file = config_dir.join("wezterm.lua");
        fs::write(&config_file, "-- config").expect("write config");

        // state_dir inside the config directory
        let state_dir = config_dir.join("state");

        let result = validate_state_dir_safety(
            &state_dir,
            Some(&config_file),
            None,
        );

        assert!(
            !result.is_ok(),
            "expected error when state_dir is under config root"
        );
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.errors[0].contains("reload loops"),
            "error message should mention reload loops: {}",
            result.errors[0]
        );
    }

    #[test]
    fn test_state_dir_outside_config_is_ok() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).expect("mkdir config");

        let config_file = config_dir.join("wezterm.lua");
        fs::write(&config_file, "-- config").expect("write config");

        // state_dir outside the config directory
        let state_dir = tmp.path().join("state");

        let result = validate_state_dir_safety(
            &state_dir,
            Some(&config_file),
            None,
        );

        assert!(
            result.is_ok(),
            "state_dir outside config should be fine, got errors: {:?}",
            result.errors
        );
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_state_dir_safety_home_config_skipped() {
        // When config_file is $HOME/.wezterm.lua the parent ($HOME) is too
        // broad to be a reload root, so it should NOT trigger an error even
        // if state_dir is under $HOME.
        let tmp = tempfile::tempdir().expect("create tempdir");
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).expect("mkdir home");

        let config_file = home.join(".wezterm.lua");
        fs::write(&config_file, "-- config").expect("write");

        let state_dir = home.join(".local").join("state").join("wezterm-utils");

        let result = validate_state_dir_safety(
            &state_dir,
            Some(&config_file),
            Some(&home),
        );

        assert!(
            result.is_ok(),
            "home config file should not make $HOME a reload root: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_state_dir_under_dot_config_wezterm_is_error() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let home = tmp.path().join("home");
        let wezterm_config = home.join(".config").join("wezterm");
        fs::create_dir_all(&wezterm_config).expect("mkdir .config/wezterm");

        // state_dir inside ~/.config/wezterm/
        let state_dir = wezterm_config.join("state");

        let result = validate_state_dir_safety(
            &state_dir,
            None,
            Some(&home),
        );

        assert!(
            !result.is_ok(),
            "state_dir under .config/wezterm should be an error"
        );
    }

    #[test]
    fn test_state_dir_safety_no_roots() {
        // When neither config_file nor home_dir is supplied, no roots exist
        // and the check should pass trivially.
        let result = validate_state_dir_safety(
            Path::new("/some/state"),
            None,
            None,
        );
        assert!(result.is_ok());
    }

    // -- validate_binary_paths ------------------------------------------------

    #[test]
    fn test_binary_exists() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let bin = tmp.path().join("tool.exe");
        fs::write(&bin, b"fake").expect("write bin");

        let result = validate_binary_paths(&[("tool", &bin)]);
        assert!(result.is_ok());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_binary_missing() {
        let result = validate_binary_paths(&[(
            "watcher",
            Path::new("/nonexistent/wezterm-watch.exe"),
        )]);
        assert!(result.is_ok(), "missing binary is a warning, not an error");
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("watcher"));
        assert!(result.warnings[0].contains("missing"));
    }

    #[test]
    fn test_binary_mixed() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let existing = tmp.path().join("exists.exe");
        fs::write(&existing, b"ok").expect("write");

        let result = validate_binary_paths(&[
            ("good", &existing),
            ("bad", Path::new("/no/such/binary")),
        ]);
        assert!(result.is_ok());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("bad"));
    }

    // -- validate_config_paths ------------------------------------------------

    #[test]
    fn test_config_paths_both_exist() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let config_file = tmp.path().join("wezterm.lua");
        fs::write(&config_file, "-- ok").expect("write");

        let result = validate_config_paths(
            Some(&config_file),
            Some(tmp.path()),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_file_missing() {
        let result = validate_config_paths(
            Some(Path::new("/no/such/file.lua")),
            None,
        );
        assert!(!result.is_ok());
        assert!(result.errors[0].contains("config_file"));
    }

    #[test]
    fn test_config_dir_missing() {
        let result = validate_config_paths(
            None,
            Some(Path::new("/no/such/dir")),
        );
        assert!(!result.is_ok());
        assert!(result.errors[0].contains("config_dir"));
    }

    // -- ValidationResult::merge ----------------------------------------------

    #[test]
    fn test_merge_results() {
        let mut a = ValidationResult {
            errors: vec!["err1".into()],
            warnings: vec!["warn1".into()],
        };
        let b = ValidationResult {
            errors: vec!["err2".into()],
            warnings: vec!["warn2".into()],
        };
        a.merge(b);
        assert_eq!(a.errors, vec!["err1", "err2"]);
        assert_eq!(a.warnings, vec!["warn1", "warn2"]);
    }

    // -- Lua API smoke tests --------------------------------------------------

    #[test]
    fn lua_api_registers_without_panic() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register_lua_api should succeed");

        let package: mlua::Table =
            lua.globals().get("package").expect("package");
        let loaded: mlua::Table = package.get("loaded").expect("loaded");
        let wezterm: mlua::Table = loaded.get("wezterm").expect("wezterm");
        let validation: mlua::Table =
            wezterm.get("validation").expect("validation sub-module");

        for name in &["check_state_dir", "check_binaries", "check_config_paths"]
        {
            let _: mlua::Function = validation
                .get(*name)
                .unwrap_or_else(|_| panic!("function '{name}' should exist"));
        }
    }

    #[test]
    fn lua_check_binaries_returns_table() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        // Call with one missing binary
        let result: mlua::Table = lua
            .load(
                r#"
                local v = require('wezterm').validation
                return v.check_binaries({
                    { name = "tool", path = "/nonexistent/tool.exe" },
                })
                "#,
            )
            .eval()
            .expect("eval check_binaries");

        let errors: mlua::Table = result.get("errors").expect("errors");
        let warnings: mlua::Table = result.get("warnings").expect("warnings");

        assert_eq!(errors.len().expect("len"), 0);
        assert_eq!(warnings.len().expect("len"), 1);
    }

    #[test]
    fn lua_check_state_dir_returns_table() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        // No config_file / home_dir => trivially OK
        let result: mlua::Table = lua
            .load(
                r#"
                local v = require('wezterm').validation
                return v.check_state_dir("/some/path", nil, nil)
                "#,
            )
            .eval()
            .expect("eval check_state_dir");

        let errors: mlua::Table = result.get("errors").expect("errors");
        assert_eq!(errors.len().expect("len"), 0);
    }

    #[test]
    fn lua_check_config_paths_returns_table() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        // Non-existent config file should produce an error
        let result: mlua::Table = lua
            .load(
                r#"
                local v = require('wezterm').validation
                return v.check_config_paths("/no/such/file.lua", nil)
                "#,
            )
            .eval()
            .expect("eval check_config_paths");

        let errors: mlua::Table = result.get("errors").expect("errors");
        assert!(
            errors.len().expect("len") > 0,
            "missing config file should produce an error"
        );
    }
}
