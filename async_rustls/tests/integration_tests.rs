//! Integration tests for async_rustls TLS handshakes and certificate verification.
//!
//! These tests perform actual TLS connections between server and client to verify
//! the implementation works end-to-end.

use async_rustls::{
    build_client_config_with_custom_verifier, build_server_config, extract_cn, TlsClientStream,
    TlsServerStream, WezTermCertVerifier,
};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa};
use rustls::pki_types::CertificateDer;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

/// Test certificate authority and issued certificates.
struct TestPki {
    ca_cert_der: CertificateDer<'static>,
    ca_cert_pem: String,
    _ca_key_pem: String,
    server_cert_pem: String,
    server_key_pem: String,
    client_cert_pem: String,
    client_key_pem: String,
}

impl TestPki {
    /// Generate a complete PKI for testing.
    ///
    /// Uses rcgen 0.12 API to create certificates.
    fn new(client_cn: &str) -> Self {
        // Generate CA using Certificate::from_params
        let mut ca_params = CertificateParams::new(vec!["Test CA".into()]);
        ca_params
            .distinguished_name
            .push(DnType::CommonName, "Test CA");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

        let ca_cert = rcgen::Certificate::from_params(ca_params).unwrap();
        let ca_cert_pem = ca_cert.serialize_pem().unwrap();
        let ca_key_pem = ca_cert.get_key_pair().serialize_pem();
        let ca_cert_der = CertificateDer::from(ca_cert.serialize_der().unwrap());

        // Generate server certificate (self-signed for simplicity in testing)
        let mut server_params = CertificateParams::new(vec!["localhost".into()]);
        server_params
            .distinguished_name
            .push(DnType::CommonName, "localhost");

        let server_cert = rcgen::Certificate::from_params(server_params).unwrap();
        let server_cert_pem = server_cert.serialize_pem().unwrap();
        let server_key_pem = server_cert.get_key_pair().serialize_pem();

        // Generate client certificate
        let mut client_params = CertificateParams::new(vec![client_cn.into()]);
        client_params
            .distinguished_name
            .push(DnType::CommonName, client_cn);

        let client_cert = rcgen::Certificate::from_params(client_params).unwrap();
        let client_cert_pem = client_cert.serialize_pem().unwrap();
        let client_key_pem = client_cert.get_key_pair().serialize_pem();

        Self {
            ca_cert_der,
            ca_cert_pem,
            _ca_key_pem: ca_key_pem,
            server_cert_pem,
            server_key_pem,
            client_cert_pem,
            client_key_pem,
        }
    }

    /// Generate a self-signed certificate not issued by the CA.
    fn generate_untrusted_cert() -> (String, String) {
        let cert = rcgen::generate_simple_self_signed(vec!["untrusted.local".into()]).unwrap();
        (
            cert.serialize_pem().unwrap(),
            cert.serialize_private_key_pem(),
        )
    }
}

/// Find an available TCP port for testing.
fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

// =============================================================================
// Integration Tests: Basic TLS Handshake
// =============================================================================

#[test]
fn test_basic_tls_handshake_no_client_auth() {
    // Get username for CN
    #[cfg(unix)]
    let username = std::env::var("USER").unwrap_or_else(|_| "testuser".into());
    #[cfg(windows)]
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "testuser".into());

    let pki = TestPki::new(&username);
    let port = find_available_port();
    let addr = format!("127.0.0.1:{}", port);

    // Build server config (no client auth)
    let server_config = build_server_config(
        pki.server_cert_pem.as_bytes(),
        pki.server_key_pem.as_bytes(),
        None,
        false, // No client auth
    )
    .expect("Failed to build server config");

    // Spawn server thread
    let addr_clone = addr.clone();
    let server_handle = thread::spawn(move || {
        let listener = TcpListener::bind(&addr_clone).unwrap();
        listener.set_nonblocking(false).unwrap();

        let (stream, _) = listener.accept().unwrap();
        stream.set_nodelay(true).unwrap();

        let mut tls_stream = TlsServerStream::accept(stream, server_config).unwrap();

        // Read message from client
        let mut buf = [0u8; 64];
        let n = tls_stream.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"Hello from client");

        // Send response
        tls_stream.write_all(b"Hello from server").unwrap();
        tls_stream.flush().unwrap();
    });

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Build client config with custom verifier for self-signed server cert
    // We use the server's cert as the "CA" since it's self-signed
    let server_cert_der = {
        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut std::io::Cursor::new(pki.server_cert_pem.as_bytes()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        certs[0].clone()
    };

    let client_config = build_client_config_with_custom_verifier(
        None,
        None,
        server_cert_der,
        true, // Accept invalid hostnames for testing
    )
    .expect("Failed to build client config");

    // Connect client
    let stream = TcpStream::connect(&addr).unwrap();
    stream.set_nodelay(true).unwrap();

    let mut tls_stream =
        TlsClientStream::connect(stream, client_config, "localhost").expect("TLS handshake failed");

    // Send message to server
    tls_stream.write_all(b"Hello from client").unwrap();
    tls_stream.flush().unwrap();

    // Read response
    let mut buf = [0u8; 64];
    let n = tls_stream.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"Hello from server");

    server_handle.join().unwrap();
}

#[test]
fn test_tls_handshake_with_mutual_auth_config() {
    // This test verifies the configuration path for mutual TLS
    // Get username for CN
    #[cfg(unix)]
    let username = std::env::var("USER").unwrap_or_else(|_| "testuser".into());
    #[cfg(windows)]
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "testuser".into());

    let pki = TestPki::new(&username);

    // Build server config with mutual TLS enabled
    let _server_config = build_server_config(
        pki.server_cert_pem.as_bytes(),
        pki.server_key_pem.as_bytes(),
        Some(pki.ca_cert_pem.as_bytes()),
        true, // Require client auth
    )
    .expect("Failed to build server config with mutual TLS");

    // Server config created successfully - it has a client cert verifier configured

    // Build client config with client certificate
    let server_cert_der = {
        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut std::io::Cursor::new(pki.server_cert_pem.as_bytes()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        certs[0].clone()
    };

    let client_config = build_client_config_with_custom_verifier(
        Some(pki.client_cert_pem.as_bytes()),
        Some(pki.client_key_pem.as_bytes()),
        server_cert_der,
        true,
    )
    .expect("Failed to build client config with mutual TLS");

    // Verify client config has client auth configured
    assert!(client_config.client_auth_cert_resolver.has_certs());
}

// =============================================================================
// Certificate Verification Tests
// =============================================================================

#[test]
fn test_untrusted_server_certificate_rejected() {
    // Generate untrusted (self-signed) server certificate
    let (untrusted_cert_pem, untrusted_key_pem) = TestPki::generate_untrusted_cert();

    // Generate a separate CA that the client trusts
    #[cfg(unix)]
    let username = std::env::var("USER").unwrap_or_else(|_| "testuser".into());
    #[cfg(windows)]
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "testuser".into());
    let pki = TestPki::new(&username);

    let port = find_available_port();
    let addr = format!("127.0.0.1:{}", port);

    // Server uses untrusted certificate
    let server_config = build_server_config(
        untrusted_cert_pem.as_bytes(),
        untrusted_key_pem.as_bytes(),
        None,
        false,
    )
    .expect("Failed to build server config");

    let addr_clone = addr.clone();
    let server_handle = thread::spawn(move || {
        let listener = TcpListener::bind(&addr_clone).unwrap();
        let (stream, _) = listener.accept().unwrap();
        // Server attempts handshake - client should reject
        let _ = TlsServerStream::accept(stream, server_config);
    });

    thread::sleep(Duration::from_millis(100));

    // Client trusts only the test CA, not the untrusted cert
    let client_config =
        build_client_config_with_custom_verifier(None, None, pki.ca_cert_der.clone(), true)
            .expect("Failed to build client config");

    let stream = TcpStream::connect(&addr).unwrap();

    // This should fail because server cert isn't signed by our CA
    let result = TlsClientStream::connect(stream, client_config, "localhost");
    assert!(
        result.is_err(),
        "Expected handshake to fail with untrusted cert"
    );

    let _ = server_handle.join();
}

// =============================================================================
// WezTermCertVerifier Unit Tests
// =============================================================================

#[test]
fn test_wezterm_cert_verifier_rejects_different_ca() {
    // Generate two separate self-signed CAs
    let mut ca1_params = CertificateParams::new(vec!["CA 1".into()]);
    ca1_params
        .distinguished_name
        .push(DnType::CommonName, "CA 1");
    ca1_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca1_cert = rcgen::Certificate::from_params(ca1_params).unwrap();

    let mut ca2_params = CertificateParams::new(vec!["CA 2".into()]);
    ca2_params
        .distinguished_name
        .push(DnType::CommonName, "CA 2");
    ca2_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca2_cert = rcgen::Certificate::from_params(ca2_params).unwrap();

    // Create verifier that trusts CA1
    let ca1_der = CertificateDer::from(ca1_cert.serialize_der().unwrap());
    let verifier = WezTermCertVerifier::new(ca1_der, true).unwrap();

    // Use CA2's cert (which is self-signed) as the "server cert"
    // Since it's not signed by CA1, it should be rejected
    let server_cert_der = CertificateDer::from(ca2_cert.serialize_der().unwrap());
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let now = rustls::pki_types::UnixTime::now();

    use rustls::client::danger::ServerCertVerifier;
    let result = verifier.verify_server_cert(&server_cert_der, &[], &server_name, &[], now);

    assert!(result.is_err(), "Should reject cert signed by different CA");
}

#[test]
fn test_wezterm_cert_verifier_accepts_self_signed_as_ca() {
    // Generate a self-signed cert that will serve as both CA and server cert
    // (This mimics the behavior of a self-signed CA certificate)
    let mut params = CertificateParams::new(vec!["localhost".into()]);
    params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let cert = rcgen::Certificate::from_params(params).unwrap();

    let cert_der = CertificateDer::from(cert.serialize_der().unwrap());

    // Create verifier that trusts this cert as CA
    let verifier = WezTermCertVerifier::new(cert_der.clone(), true).unwrap();

    // Verify the same cert - should succeed since it's self-signed CA
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let now = rustls::pki_types::UnixTime::now();

    use rustls::client::danger::ServerCertVerifier;
    let result = verifier.verify_server_cert(&cert_der, &[], &server_name, &[], now);

    assert!(
        result.is_ok(),
        "Should accept self-signed CA cert: {:?}",
        result.err()
    );
}

// =============================================================================
// CN Extraction Tests
// =============================================================================

#[test]
fn test_extract_cn_standard_format() {
    // Test standard CN
    let mut params = CertificateParams::new(vec!["test".into()]);
    params
        .distinguished_name
        .push(DnType::CommonName, "standard_user");
    let cert = rcgen::Certificate::from_params(params).unwrap();
    let cert_der = CertificateDer::from(cert.serialize_der().unwrap());

    let cn = extract_cn(&cert_der).unwrap();
    assert_eq!(cn, "standard_user");
}

#[test]
fn test_extract_cn_encoded_format() {
    // Test encoded CN
    let mut params = CertificateParams::new(vec!["test".into()]);
    params
        .distinguished_name
        .push(DnType::CommonName, "user:john/dept=eng");
    let cert = rcgen::Certificate::from_params(params).unwrap();
    let cert_der = CertificateDer::from(cert.serialize_der().unwrap());

    let cn = extract_cn(&cert_der).unwrap();
    assert_eq!(cn, "user:john/dept=eng");
}

#[test]
fn test_extract_cn_from_simple_self_signed() {
    // generate_simple_self_signed uses a default CN
    let cert = rcgen::generate_simple_self_signed(vec!["test.example.com".into()]).unwrap();
    let cert_der = CertificateDer::from(cert.serialize_der().unwrap());

    let cn = extract_cn(&cert_der).unwrap();
    // rcgen uses "rcgen self signed cert" as default CN for generate_simple_self_signed
    assert_eq!(cn, "rcgen self signed cert");
}

// =============================================================================
// Data Transfer Tests
// =============================================================================

#[test]
fn test_large_data_transfer() {
    #[cfg(unix)]
    let username = std::env::var("USER").unwrap_or_else(|_| "testuser".into());
    #[cfg(windows)]
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "testuser".into());

    let pki = TestPki::new(&username);
    let port = find_available_port();
    let addr = format!("127.0.0.1:{}", port);

    let server_config = build_server_config(
        pki.server_cert_pem.as_bytes(),
        pki.server_key_pem.as_bytes(),
        None,
        false,
    )
    .unwrap();

    // Generate 100KB of test data
    let test_data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
    let test_data_clone = test_data.clone();

    let addr_clone = addr.clone();
    let server_handle = thread::spawn(move || {
        let listener = TcpListener::bind(&addr_clone).unwrap();
        let (stream, _) = listener.accept().unwrap();
        let mut tls_stream = TlsServerStream::accept(stream, server_config).unwrap();

        // Read all data
        let mut received = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match tls_stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => received.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("Read error: {}", e),
            }
            if received.len() >= test_data_clone.len() {
                break;
            }
        }

        assert_eq!(received.len(), test_data_clone.len());
        assert_eq!(received, test_data_clone);
    });

    thread::sleep(Duration::from_millis(100));

    // Use server's cert as trusted CA since it's self-signed
    let server_cert_der = {
        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut std::io::Cursor::new(pki.server_cert_pem.as_bytes()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        certs[0].clone()
    };

    let client_config =
        build_client_config_with_custom_verifier(None, None, server_cert_der, true).unwrap();

    let stream = TcpStream::connect(&addr).unwrap();
    let mut tls_stream = TlsClientStream::connect(stream, client_config, "localhost").unwrap();

    // Write all data
    tls_stream.write_all(&test_data).unwrap();
    tls_stream.flush().unwrap();

    server_handle.join().unwrap();
}

#[test]
fn test_bidirectional_communication() {
    #[cfg(unix)]
    let username = std::env::var("USER").unwrap_or_else(|_| "testuser".into());
    #[cfg(windows)]
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "testuser".into());

    let pki = TestPki::new(&username);
    let port = find_available_port();
    let addr = format!("127.0.0.1:{}", port);

    let server_config = build_server_config(
        pki.server_cert_pem.as_bytes(),
        pki.server_key_pem.as_bytes(),
        None,
        false,
    )
    .unwrap();

    let addr_clone = addr.clone();
    let server_handle = thread::spawn(move || {
        let listener = TcpListener::bind(&addr_clone).unwrap();
        let (stream, _) = listener.accept().unwrap();
        let mut tls_stream = TlsServerStream::accept(stream, server_config).unwrap();

        for i in 0..10 {
            // Read message
            let mut buf = [0u8; 32];
            let n = tls_stream.read(&mut buf).unwrap();
            let msg = String::from_utf8_lossy(&buf[..n]);
            assert!(msg.starts_with("ping"), "Expected ping, got: {}", msg);

            // Send response
            let response = format!("pong-{}", i);
            tls_stream.write_all(response.as_bytes()).unwrap();
            tls_stream.flush().unwrap();
        }
    });

    thread::sleep(Duration::from_millis(100));

    // Use server's cert as trusted CA
    let server_cert_der = {
        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut std::io::Cursor::new(pki.server_cert_pem.as_bytes()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        certs[0].clone()
    };

    let client_config =
        build_client_config_with_custom_verifier(None, None, server_cert_der, true).unwrap();

    let stream = TcpStream::connect(&addr).unwrap();
    let mut tls_stream = TlsClientStream::connect(stream, client_config, "localhost").unwrap();

    for i in 0..10 {
        // Send ping
        let msg = format!("ping-{}", i);
        tls_stream.write_all(msg.as_bytes()).unwrap();
        tls_stream.flush().unwrap();

        // Read pong
        let mut buf = [0u8; 32];
        let n = tls_stream.read(&mut buf).unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(response, format!("pong-{}", i));
    }

    server_handle.join().unwrap();
}

// =============================================================================
// ALPN Protocol Negotiation Tests
// =============================================================================

#[test]
fn test_alpn_config_set() {
    #[cfg(unix)]
    let username = std::env::var("USER").unwrap_or_else(|_| "testuser".into());
    #[cfg(windows)]
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "testuser".into());

    let pki = TestPki::new(&username);

    let server_config = build_server_config(
        pki.server_cert_pem.as_bytes(),
        pki.server_key_pem.as_bytes(),
        None,
        false,
    )
    .unwrap();

    // Verify ALPN is set on server config
    assert!(
        server_config
            .alpn_protocols
            .contains(&b"wezterm-mux".to_vec()),
        "Server config should have wezterm-mux ALPN"
    );

    // Use server's cert as trusted CA
    let server_cert_der = {
        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut std::io::Cursor::new(pki.server_cert_pem.as_bytes()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        certs[0].clone()
    };

    let client_config =
        build_client_config_with_custom_verifier(None, None, server_cert_der, true).unwrap();

    // Verify ALPN is set on client config
    assert!(
        client_config
            .alpn_protocols
            .contains(&b"wezterm-mux".to_vec()),
        "Client config should have wezterm-mux ALPN"
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_empty_cert_pem_rejected() {
    let result = build_server_config(b"", b"", None, false);
    assert!(result.is_err(), "Should reject empty certificate PEM");
}

#[test]
fn test_invalid_cert_pem_rejected() {
    let result = build_server_config(b"not a valid pem", b"not a key", None, false);
    assert!(result.is_err(), "Should reject invalid PEM");
}

#[test]
fn test_mismatched_cert_key_rejected() {
    // Generate two different certificates
    let cert1 = rcgen::generate_simple_self_signed(vec!["cert1.local".into()]).unwrap();
    let cert2 = rcgen::generate_simple_self_signed(vec!["cert2.local".into()]).unwrap();

    // Use cert from one and key from another
    let result = build_server_config(
        cert1.serialize_pem().unwrap().as_bytes(),
        cert2.serialize_private_key_pem().as_bytes(),
        None,
        false,
    );

    // This should fail because the key doesn't match the cert
    assert!(
        result.is_err(),
        "Should reject mismatched certificate and key"
    );
}

// =============================================================================
// Thread Safety Tests
// =============================================================================

#[test]
fn test_stream_is_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<TlsServerStream>();
    assert_sync::<TlsServerStream>();
    assert_send::<TlsClientStream>();
    assert_sync::<TlsClientStream>();
}
