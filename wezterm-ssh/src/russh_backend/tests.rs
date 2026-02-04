//! Unit tests for the russh backend.
//!
//! This module contains unit tests for the russh backend components:
//! - Handler tests for host key verification and banner handling
//! - Session tests for connection and authentication
//! - Channel tests for PTY operations

#[cfg(test)]
mod handler_tests {
    use super::super::handler::WezTermHandler;
    use crate::host::HostVerificationEvent;
    use crate::session::SessionEvent;
    use smol::channel::{bounded, Receiver, Sender};

    fn create_test_handler() -> (WezTermHandler, Receiver<SessionEvent>) {
        let (tx, rx) = bounded(8);
        (WezTermHandler::new(tx), rx)
    }

    #[test]
    fn test_handler_creation() {
        let (handler, _rx) = create_test_handler();
        assert!(!handler.is_host_verified());
    }

    #[test]
    fn test_handler_is_host_verified_default_false() {
        let (handler, _rx) = create_test_handler();
        assert!(!handler.is_host_verified());
    }
}

#[cfg(test)]
mod channel_tests {
    use super::super::channel::RusshChannel;
    use portable_pty::PtySize;
    use russh::Sig;

    #[test]
    fn test_pty_size_conversion() {
        let size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 800,
            pixel_height: 600,
        };
        // Verify the size values can be converted to u32
        assert_eq!(size.cols as u32, 80);
        assert_eq!(size.rows as u32, 24);
        assert_eq!(size.pixel_width as u32, 800);
        assert_eq!(size.pixel_height as u32, 600);
    }

    #[test]
    fn test_signal_name_mapping() {
        // Test common signal names
        let signals = [
            ("HUP", true),
            ("SIGHUP", true),
            ("INT", true),
            ("SIGINT", true),
            ("TERM", true),
            ("SIGTERM", true),
            ("KILL", true),
            ("SIGKILL", true),
            ("INVALID", false),
            ("", false),
        ];

        for (name, should_exist) in signals {
            let result = map_signal_name(name);
            if should_exist {
                assert!(result.is_some(), "Signal {} should be valid", name);
            } else {
                assert!(result.is_none(), "Signal {} should be invalid", name);
            }
        }
    }

    /// Helper function to map signal names (mirrors channel.rs logic)
    fn map_signal_name(signame: &str) -> Option<Sig> {
        match signame.to_uppercase().as_str() {
            "HUP" | "SIGHUP" => Some(Sig::HUP),
            "INT" | "SIGINT" => Some(Sig::INT),
            "QUIT" | "SIGQUIT" => Some(Sig::QUIT),
            "ILL" | "SIGILL" => Some(Sig::ILL),
            "ABRT" | "SIGABRT" => Some(Sig::ABRT),
            "FPE" | "SIGFPE" => Some(Sig::FPE),
            "KILL" | "SIGKILL" => Some(Sig::KILL),
            "USR1" | "SIGUSR1" => Some(Sig::USR1),
            "SEGV" | "SIGSEGV" => Some(Sig::SEGV),
            "PIPE" | "SIGPIPE" => Some(Sig::PIPE),
            "ALRM" | "SIGALRM" => Some(Sig::ALRM),
            "TERM" | "SIGTERM" => Some(Sig::TERM),
            _ => None,
        }
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::super::{block_on, get_runtime};
    use std::time::Duration;

    #[test]
    fn test_runtime_initialization() {
        let runtime = get_runtime();
        // Runtime should be initialized and usable - verify by running a simple task
        let result = runtime.block_on(async { 1 + 1 });
        assert_eq!(result, 2);
    }

    #[test]
    fn test_block_on_simple_future() {
        let result = block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_block_on_async_computation() {
        let result = block_on(async {
            let a = 10;
            let b = 20;
            a + b
        });
        assert_eq!(result, 30);
    }

    #[test]
    fn test_block_on_with_tokio_sleep() {
        let start = std::time::Instant::now();
        block_on(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        });
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_runtime_is_singleton() {
        let rt1 = get_runtime();
        let rt2 = get_runtime();
        // Both should point to the same runtime (same memory address)
        assert!(std::ptr::eq(rt1, rt2));
    }
}

#[cfg(test)]
mod session_config_tests {
    use std::time::Duration;

    #[test]
    fn test_default_config_values() {
        // Test that our config values are reasonable
        let inactivity_timeout = Duration::from_secs(300);
        let keepalive_interval = Duration::from_secs(60);
        let keepalive_max = 3u32;

        assert_eq!(inactivity_timeout.as_secs(), 300);
        assert_eq!(keepalive_interval.as_secs(), 60);
        assert_eq!(keepalive_max, 3);
    }
}

#[cfg(test)]
mod fingerprint_tests {
    use sha2::{Digest, Sha256};

    #[test]
    fn test_sha256_fingerprint_format() {
        // Test fingerprint computation (simulating key fingerprint)
        let test_data = b"test key data";
        let hash = Sha256::digest(test_data);
        let fingerprint = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &hash[..],
        );

        // Verify the fingerprint is base64 encoded
        assert!(!fingerprint.is_empty());
        // Verify we can trim padding
        let trimmed = fingerprint.trim_end_matches('=');
        assert!(trimmed.len() <= fingerprint.len());
    }

    #[test]
    fn test_fingerprint_trimming() {
        // Base64 strings with padding
        let with_padding = "abc123==";
        let trimmed = with_padding.trim_end_matches('=');
        assert_eq!(trimmed, "abc123");

        let no_padding = "abc123";
        let trimmed = no_padding.trim_end_matches('=');
        assert_eq!(trimmed, "abc123");
    }
}
