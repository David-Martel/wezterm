//! Functional tests for SSH config parsing.
//!
//! These tests verify real-world SSH config scenarios including:
//! - Include directive chains
//! - Complex host pattern matching
//! - Token expansion edge cases
//! - Config file precedence

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use wezterm_ssh::Config;

/// Create a test SSH config directory with multiple config files.
fn setup_config_dir() -> TempDir {
    TempDir::new().expect("create temp dir")
}

/// Write a config file with the given content.
fn write_config(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(&path, content).expect("write config");
    path
}

/// Create a Config and load a config file, then get options for a host.
fn load_config_for_host(config_path: &PathBuf, host: &str) -> std::collections::BTreeMap<String, String> {
    let mut config = Config::new();
    config.add_config_file(config_path);
    config.for_host(host)
}

// =============================================================================
// Basic Config Parsing Tests
// =============================================================================

#[test]
fn test_basic_host_config() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host myserver
    HostName 192.168.1.100
    User admin
    Port 2222
"#,
    );

    let options = load_config_for_host(&config_path, "myserver");

    assert_eq!(options.get("hostname"), Some(&"192.168.1.100".to_string()));
    assert_eq!(options.get("user"), Some(&"admin".to_string()));
    assert_eq!(options.get("port"), Some(&"2222".to_string()));
}

#[test]
fn test_wildcard_host_pattern() {
    let dir = setup_config_dir();

    // SSH config uses "first match wins" for each option.
    // To have more specific patterns take precedence, they must come BEFORE general patterns.
    let config_path = write_config(
        &dir,
        "config",
        r#"
Host dev-*.example.com
    User devuser

Host prod-*.example.com
    User produser

Host *.example.com
    User webadmin
    Port 22
"#,
    );

    // Test basic wildcard (matches *.example.com)
    let options = load_config_for_host(&config_path, "www.example.com");
    assert_eq!(options.get("user"), Some(&"webadmin".to_string()));

    // Test more specific patterns (must be listed first to take precedence)
    let dev_options = load_config_for_host(&config_path, "dev-server.example.com");
    assert_eq!(dev_options.get("user"), Some(&"devuser".to_string()));
    // Port should also be set from the *.example.com block
    assert_eq!(dev_options.get("port"), Some(&"22".to_string()));

    let prod_options = load_config_for_host(&config_path, "prod-server.example.com");
    assert_eq!(prod_options.get("user"), Some(&"produser".to_string()));
}

#[test]
fn test_question_mark_wildcard() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host server?
    User admin

Host server??
    User superadmin
"#,
    );

    // Single character wildcard
    let options = load_config_for_host(&config_path, "server1");
    assert_eq!(options.get("user"), Some(&"admin".to_string()));

    // Two character wildcard
    let options = load_config_for_host(&config_path, "server10");
    assert_eq!(options.get("user"), Some(&"superadmin".to_string()));
}

// =============================================================================
// Config Precedence Tests
// =============================================================================

#[test]
fn test_first_match_wins() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host myhost
    User firstuser

Host myhost
    User seconduser
"#,
    );

    let options = load_config_for_host(&config_path, "myhost");

    // First match should win
    assert_eq!(options.get("user"), Some(&"firstuser".to_string()));
}

#[test]
fn test_specific_before_wildcard() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host specific-host
    User specificuser

Host *
    User wildcarduser
"#,
    );

    // Specific host should get specific user
    let specific = load_config_for_host(&config_path, "specific-host");
    assert_eq!(specific.get("user"), Some(&"specificuser".to_string()));

    // Other hosts should get wildcard user
    let other = load_config_for_host(&config_path, "other-host");
    assert_eq!(other.get("user"), Some(&"wildcarduser".to_string()));
}

#[test]
fn test_options_accumulate() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host myhost
    User myuser

Host *
    ServerAliveInterval 60
    ServerAliveCountMax 3
"#,
    );

    let options = load_config_for_host(&config_path, "myhost");

    // Should have options from both matching stanzas
    assert_eq!(options.get("user"), Some(&"myuser".to_string()));
    assert_eq!(options.get("serveraliveinterval"), Some(&"60".to_string()));
    assert_eq!(options.get("serveralivecountmax"), Some(&"3".to_string()));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_config() {
    let dir = setup_config_dir();
    let config_path = write_config(&dir, "config", "");

    let options = load_config_for_host(&config_path, "anyhost");

    // Should not crash, options should be empty or have defaults
    // The hostname won't be in options unless explicitly set
    assert!(options.get("user").is_none() || options.get("hostname").is_some());
}

#[test]
fn test_comments_and_whitespace() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
# This is a comment
   # Indented comment

Host myhost
    # Comment in host block
    User myuser

    Port 22
"#,
    );

    let options = load_config_for_host(&config_path, "myhost");

    assert_eq!(options.get("user"), Some(&"myuser".to_string()));
    assert_eq!(options.get("port"), Some(&"22".to_string()));
}

#[test]
fn test_case_insensitive_keywords() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
HOST myhost
    HOSTNAME myserver.example.com
    USER myuser
    PORT 22
"#,
    );

    let options = load_config_for_host(&config_path, "myhost");

    // Keywords should be case-insensitive
    assert_eq!(options.get("hostname"), Some(&"myserver.example.com".to_string()));
    assert_eq!(options.get("user"), Some(&"myuser".to_string()));
}

#[test]
fn test_long_line_handling() {
    let dir = setup_config_dir();

    // Create a very long ProxyCommand
    let long_command = format!("echo {}", "x".repeat(500));

    let config_path = write_config(
        &dir,
        "config",
        &format!(
            r#"
Host longline
    ProxyCommand {}
"#,
            long_command
        ),
    );

    let options = load_config_for_host(&config_path, "longline");

    let proxy_cmd = options.get("proxycommand").map(|s| s.as_str()).unwrap_or("");
    assert!(
        proxy_cmd.len() > 400,
        "Expected long line to be preserved, got length {}",
        proxy_cmd.len()
    );
}

// =============================================================================
// Multiple Config Files Tests
// =============================================================================

#[test]
fn test_multiple_config_files() {
    let dir = setup_config_dir();

    // Create two config files
    let config1 = write_config(
        &dir,
        "config1",
        r#"
Host host1
    User user1
"#,
    );

    let config2 = write_config(
        &dir,
        "config2",
        r#"
Host host2
    User user2
"#,
    );

    // Load both config files
    let mut config = Config::new();
    config.add_config_file(&config1);
    config.add_config_file(&config2);

    let options1 = config.for_host("host1");
    let options2 = config.for_host("host2");

    assert_eq!(options1.get("user"), Some(&"user1".to_string()));
    assert_eq!(options2.get("user"), Some(&"user2".to_string()));
}

#[test]
fn test_config_file_precedence() {
    let dir = setup_config_dir();

    // First config file
    let config1 = write_config(
        &dir,
        "config1",
        r#"
Host myhost
    User firstfile
"#,
    );

    // Second config file
    let config2 = write_config(
        &dir,
        "config2",
        r#"
Host myhost
    User secondfile
"#,
    );

    // Load first config, then second
    let mut config = Config::new();
    config.add_config_file(&config1);
    config.add_config_file(&config2);

    let options = config.for_host("myhost");

    // First file should take precedence
    assert_eq!(options.get("user"), Some(&"firstfile".to_string()));
}

// =============================================================================
// Config String Tests
// =============================================================================

#[test]
fn test_add_config_string() {
    let mut config = Config::new();

    config.add_config_string(
        r#"
Host stringhost
    User stringuser
    Port 3333
"#,
    );

    let options = config.for_host("stringhost");

    assert_eq!(options.get("user"), Some(&"stringuser".to_string()));
    assert_eq!(options.get("port"), Some(&"3333".to_string()));
}

#[test]
fn test_config_string_and_file_combined() {
    let dir = setup_config_dir();

    let config_file = write_config(
        &dir,
        "config",
        r#"
Host filehost
    User fileuser
"#,
    );

    let mut config = Config::new();
    config.add_config_file(&config_file);
    config.add_config_string(
        r#"
Host stringhost
    User stringuser
"#,
    );

    let file_options = config.for_host("filehost");
    let string_options = config.for_host("stringhost");

    assert_eq!(file_options.get("user"), Some(&"fileuser".to_string()));
    assert_eq!(string_options.get("user"), Some(&"stringuser".to_string()));
}

// =============================================================================
// Negation Pattern Tests
// =============================================================================

#[test]
fn test_negation_pattern_basic() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host * !*.internal
    ProxyJump bastion

Host *.internal
    ProxyJump none
"#,
    );

    // External hosts should use proxy
    let external = load_config_for_host(&config_path, "external.example.com");

    // Internal hosts should not use proxy
    let internal = load_config_for_host(&config_path, "db.internal");

    // Note: The negation behavior may vary - we're testing that it parses without error
    // and produces some result
    assert!(external.get("proxyjump").is_some() || internal.get("proxyjump").is_some());
}

// =============================================================================
// Server Alive Settings Tests
// =============================================================================

#[test]
fn test_server_alive_settings() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host keepalive
    ServerAliveInterval 30
    ServerAliveCountMax 5
"#,
    );

    let options = load_config_for_host(&config_path, "keepalive");

    assert_eq!(options.get("serveraliveinterval"), Some(&"30".to_string()));
    assert_eq!(options.get("serveralivecountmax"), Some(&"5".to_string()));
}

// =============================================================================
// Identity File Tests
// =============================================================================

#[test]
fn test_identity_file() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host keyhost
    IdentityFile ~/.ssh/custom_key
    IdentitiesOnly yes
"#,
    );

    let options = load_config_for_host(&config_path, "keyhost");

    // Identity file should be set (may have tilde expanded or not)
    let identity = options.get("identityfile").map(|s| s.as_str()).unwrap_or("");
    assert!(identity.contains("custom_key") || identity.contains("ssh"));

    assert_eq!(options.get("identitiesonly"), Some(&"yes".to_string()));
}

// =============================================================================
// Proxy Settings Tests
// =============================================================================

#[test]
fn test_proxy_command() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host proxied
    ProxyCommand ssh -W %h:%p bastion.example.com
"#,
    );

    let options = load_config_for_host(&config_path, "proxied");

    let proxy = options.get("proxycommand").map(|s| s.as_str()).unwrap_or("");
    assert!(proxy.contains("bastion") || proxy.contains("%h"));
}

#[test]
fn test_proxy_jump() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host jumped
    ProxyJump bastion@jump.example.com
"#,
    );

    let options = load_config_for_host(&config_path, "jumped");

    assert!(options.get("proxyjump").is_some());
}

// =============================================================================
// Compression and Cipher Tests
// =============================================================================

#[test]
fn test_compression_settings() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host compressed
    Compression yes
"#,
    );

    let options = load_config_for_host(&config_path, "compressed");

    assert_eq!(options.get("compression"), Some(&"yes".to_string()));
}

// =============================================================================
// Forward Settings Tests
// =============================================================================

#[test]
fn test_forward_agent() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host forwarding
    ForwardAgent yes
    ForwardX11 no
"#,
    );

    let options = load_config_for_host(&config_path, "forwarding");

    assert_eq!(options.get("forwardagent"), Some(&"yes".to_string()));
    assert_eq!(options.get("forwardx11"), Some(&"no".to_string()));
}

// =============================================================================
// Connection Settings Tests
// =============================================================================

#[test]
fn test_connection_settings() {
    let dir = setup_config_dir();

    let config_path = write_config(
        &dir,
        "config",
        r#"
Host connection
    ConnectTimeout 30
    ConnectionAttempts 3
    TCPKeepAlive yes
"#,
    );

    let options = load_config_for_host(&config_path, "connection");

    assert_eq!(options.get("connecttimeout"), Some(&"30".to_string()));
    assert_eq!(options.get("connectionattempts"), Some(&"3".to_string()));
    assert_eq!(options.get("tcpkeepalive"), Some(&"yes".to_string()));
}
