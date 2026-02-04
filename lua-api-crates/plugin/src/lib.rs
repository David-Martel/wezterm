//! WezTerm Plugin System
//!
//! Provides Lua API for managing Git-based plugins using pure Rust gix library.
//! This replaces the previous git2/libgit2-sys implementation with gitoxide.

use anyhow::{anyhow, Context};
use config::lua::get_or_create_sub_module;
use config::lua::mlua::{self, Lua, Value};
use luahelper::to_lua;
use std::path::PathBuf;
use tempfile::TempDir;
use wezterm_dynamic::{FromDynamic, ToDynamic};

#[derive(FromDynamic, ToDynamic, Debug)]
struct RepoSpec {
    url: String,
    component: String,
    plugin_dir: PathBuf,
}

/// Given a URL, generate a string that can be used as a directory name.
/// The returned name must be a single valid filesystem component
fn compute_repo_dir(url: &str) -> String {
    let mut dir = String::new();
    for c in url.chars() {
        match c {
            '/' | '\\' => {
                dir.push_str("sZs");
            }
            ':' => {
                dir.push_str("sCs");
            }
            '.' => {
                dir.push_str("sDs");
            }
            '-' | '_' => dir.push(c),
            c if c.is_alphanumeric() => dir.push(c),
            c => dir.push_str(&format!("u{}", c as u32)),
        }
    }
    if dir.ends_with("sZs") {
        dir.truncate(dir.len() - 3);
    }
    dir
}

/// Get the remote URL from a repository.
fn get_remote_url(repo: &gix::Repository) -> anyhow::Result<Option<String>> {
    // Try to find the default remote (usually "origin")
    if let Some(remote) = repo.find_default_remote(gix::remote::Direction::Fetch) {
        let remote = remote?;
        if let Some(url) = remote.url(gix::remote::Direction::Fetch) {
            return Ok(Some(url.to_bstring().to_string()));
        }
    }

    // Fall back to checking named remotes
    let remote_names = repo.remote_names();
    for name in remote_names.iter() {
        if let Ok(remote) = repo.find_remote(name.as_ref()) {
            if let Some(url) = remote.url(gix::remote::Direction::Fetch) {
                return Ok(Some(url.to_bstring().to_string()));
            }
        }
    }

    Ok(None)
}

impl RepoSpec {
    fn parse(url: String) -> anyhow::Result<Self> {
        let component = compute_repo_dir(&url);
        if component.starts_with('.') {
            anyhow::bail!("invalid repo spec {url}");
        }

        let plugin_dir = RepoSpec::plugins_dir().join(&component);

        Ok(Self {
            url,
            component,
            plugin_dir,
        })
    }

    fn load_from_dir(path: PathBuf) -> anyhow::Result<Self> {
        let component = path
            .file_name()
            .ok_or_else(|| anyhow!("missing file name!?"))?
            .to_str()
            .ok_or_else(|| anyhow!("{path:?} isn't unicode"))?
            .to_string();

        let plugin_dir = RepoSpec::plugins_dir().join(&component);

        let repo = gix::open(&path).context("open repository")?;
        let url = get_remote_url(&repo)?
            .ok_or_else(|| anyhow!("no remotes found in repository"))?;

        Ok(Self {
            component,
            url,
            plugin_dir,
        })
    }

    fn plugins_dir() -> PathBuf {
        config::DATA_DIR.join("plugins")
    }

    fn checkout_path(&self) -> PathBuf {
        Self::plugins_dir().join(&self.component)
    }

    fn is_checked_out(&self) -> bool {
        self.checkout_path().exists()
    }

    /// Update the plugin by performing a fresh clone.
    ///
    /// This is a simpler approach than complex fetch+merge operations.
    /// We backup the current checkout, clone fresh, and restore on failure.
    fn update(&self) -> anyhow::Result<()> {
        let path = self.checkout_path();

        // Verify we have a valid repo
        let _repo = gix::open(&path).context("open repository")?;

        log::debug!("Updating {} via fresh clone", self.component);

        // Create backup
        let plugins_dir = Self::plugins_dir();
        let backup_path = plugins_dir.join(format!("{}.backup", self.component));

        // Remove old backup if exists
        if backup_path.exists() {
            std::fs::remove_dir_all(&backup_path).ok();
        }

        // Move current to backup
        std::fs::rename(&path, &backup_path).context("backup existing plugin")?;

        // Try to clone fresh
        match self.check_out() {
            Ok(_) => {
                // Success - remove backup
                std::fs::remove_dir_all(&backup_path).ok();
                log::info!("Updated {}", self.component);
                Ok(())
            }
            Err(e) => {
                // Failed - restore backup
                log::error!("Failed to update {}: {e:#}", self.component);
                if let Err(restore_err) = std::fs::rename(&backup_path, &path) {
                    log::error!("Failed to restore backup: {restore_err:#}");
                }
                Err(e)
            }
        }
    }

    fn check_out(&self) -> anyhow::Result<()> {
        let plugins_dir = Self::plugins_dir();
        std::fs::create_dir_all(&plugins_dir)?;
        let target_dir = TempDir::new_in(&plugins_dir)?;

        log::debug!("Cloning {} into temporary dir {target_dir:?}", self.url);

        // Parse the URL
        let url = gix::url::parse(self.url.as_str().into()).context("parse URL")?;

        // Prepare clone
        let mut prepare_clone =
            gix::prepare_clone(url, target_dir.path()).context("prepare clone")?;

        // Fetch and checkout
        let (mut prepare_checkout, _outcome) = prepare_clone
            .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .context("fetch then checkout")?;

        // Perform main worktree checkout
        let (_repo, _outcome) = prepare_checkout
            .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .context("checkout main worktree")?;

        // Keep the temp dir and rename to final location
        let target_dir = target_dir.keep();
        let checkout_path = self.checkout_path();

        match std::fs::rename(&target_dir, &checkout_path) {
            Ok(_) => {
                log::info!("Cloned {} into {checkout_path:?}", self.url);
                Ok(())
            }
            Err(err) => {
                log::error!(
                    "Failed to rename {target_dir:?} -> {:?}, removing temporary dir",
                    self.checkout_path()
                );
                if let Err(err) = std::fs::remove_dir_all(&target_dir) {
                    log::error!(
                        "Failed to remove {target_dir:?}: {err:#}, \
                         you will need to remove it manually"
                    );
                }
                Err(err.into())
            }
        }
    }
}

fn require_plugin(lua: &Lua, url: String) -> anyhow::Result<Value<'_>> {
    let spec = RepoSpec::parse(url)?;

    if !spec.is_checked_out() {
        spec.check_out()?;
    }

    let require: mlua::Function = lua.globals().get("require")?;
    match require.call::<_, Value>(spec.component.to_string()) {
        Ok(value) => Ok(value),
        Err(err) => {
            log::error!(
                "Failed to require {} which is stored in {:?}: {err:#}",
                spec.component,
                spec.checkout_path()
            );
            Err(err.into())
        }
    }
}

fn list_plugins() -> anyhow::Result<Vec<RepoSpec>> {
    let mut plugins = vec![];

    let plugins_dir = RepoSpec::plugins_dir();
    std::fs::create_dir_all(&plugins_dir)?;

    for entry in plugins_dir.read_dir()? {
        let entry = entry?;
        if entry.path().is_dir() {
            match RepoSpec::load_from_dir(entry.path()) {
                Ok(spec) => plugins.push(spec),
                Err(e) => {
                    log::warn!("Failed to load plugin from {:?}: {e}", entry.path());
                }
            }
        }
    }

    Ok(plugins)
}

pub fn register(lua: &Lua) -> anyhow::Result<()> {
    let plugin_mod = get_or_create_sub_module(lua, "plugin")?;
    plugin_mod.set(
        "require",
        lua.create_function(|lua: &Lua, repo_spec: String| {
            require_plugin(lua, repo_spec).map_err(|e| mlua::Error::external(format!("{e:#}")))
        })?,
    )?;

    plugin_mod.set(
        "list",
        lua.create_function(|lua, _: ()| {
            let plugins = list_plugins().map_err(|e| mlua::Error::external(format!("{e:#}")))?;
            to_lua(lua, plugins)
        })?,
    )?;

    plugin_mod.set(
        "update_all",
        lua.create_function(|_, _: ()| {
            let plugins = list_plugins().map_err(|e| mlua::Error::external(format!("{e:#}")))?;
            for p in plugins {
                match p.update() {
                    Ok(_) => log::info!("Updated {p:?}"),
                    Err(err) => log::error!("Failed to update {p:?}: {err:#}"),
                }
            }
            Ok(())
        })?,
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compute_repo_dir() {
        for (input, expect) in &[
            ("foo", "foo"),
            (
                "githubsDscom/wezterm/wezterm-plugins",
                "githubsDscomsZsweztermsZswezterm-plugins",
            ),
            ("localhost:8080/repo", "localhostsCs8080sZsrepo"),
        ] {
            let result = compute_repo_dir(input);
            assert_eq!(&result, expect, "for input {input}");
        }
    }
}
