//! SSH channel wrapper for PTY and command execution.
//!
//! This module provides [`RusshChannel`], which wraps russh's async channel
//! operations for terminal sessions and command execution.
//!
//! ## Channel Lifecycle
//!
//! ```text
//! 1. Open channel      ──► channel_open_session()
//! 2. Request PTY       ──► request_pty("xterm-256color", size)
//! 3. Start shell/exec  ──► request_shell() or exec("command")
//! 4. Data transfer     ──► read()/write() bidirectional I/O
//! 5. Close             ──► eof() + close()
//! ```
//!
//! ## Supported Operations
//!
//! | Operation | Method | Description |
//! |-----------|--------|-------------|
//! | PTY | [`request_pty`](RusshChannel::request_pty) | Allocate pseudo-terminal |
//! | Shell | [`request_shell`](RusshChannel::request_shell) | Start interactive shell |
//! | Exec | [`exec`](RusshChannel::exec) | Execute single command |
//! | Resize | [`resize`](RusshChannel::resize) | Change terminal dimensions |
//! | Signal | [`send_signal`](RusshChannel::send_signal) | Send signal (TERM, INT, etc.) |
//!
//! ## Data Flow
//!
//! Data is transferred via async I/O internally but exposed through sync wrappers
//! in the parent module via `block_on()`.

use portable_pty::PtySize;
use russh::{Channel, ChannelMsg, Sig};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Wrapper around a russh session channel.
///
/// This type provides methods for PTY operations, shell execution,
/// and data transfer over an SSH channel.
pub struct RusshChannel {
    /// The underlying russh channel
    inner: Channel<russh::client::Msg>,
    /// Exit status once the channel closes
    exit_status: Option<u32>,
    /// Exit signal if terminated by signal
    exit_signal: Option<String>,
}

impl RusshChannel {
    /// Create a new channel wrapper.
    pub fn new(channel: Channel<russh::client::Msg>) -> Self {
        Self {
            inner: channel,
            exit_status: None,
            exit_signal: None,
        }
    }

    /// Request a pseudo-terminal on this channel.
    pub async fn request_pty(&mut self, term: &str, size: PtySize) -> anyhow::Result<()> {
        self.inner
            .request_pty(
                true, // want_reply
                term,
                size.cols as u32,
                size.rows as u32,
                size.pixel_width as u32,
                size.pixel_height as u32,
                &[], // Terminal modes (empty for default)
            )
            .await?;
        Ok(())
    }

    /// Request a shell on this channel.
    pub async fn request_shell(&mut self) -> anyhow::Result<()> {
        self.inner.request_shell(true).await?;
        Ok(())
    }

    /// Execute a command on this channel.
    pub async fn exec(&mut self, command: &str) -> anyhow::Result<()> {
        self.inner.exec(true, command).await?;
        Ok(())
    }

    /// Request environment variable to be set.
    pub async fn request_env(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        self.inner.set_env(true, name, value).await?;
        Ok(())
    }

    /// Resize the pseudo-terminal.
    pub async fn resize(&mut self, size: PtySize) -> anyhow::Result<()> {
        self.inner
            .window_change(
                size.cols as u32,
                size.rows as u32,
                size.pixel_width as u32,
                size.pixel_height as u32,
            )
            .await?;
        Ok(())
    }

    /// Send a signal to the remote process.
    pub async fn send_signal(&mut self, signame: &str) -> anyhow::Result<()> {
        // Convert signal name to russh Sig enum
        // Note: russh 0.57 only supports a subset of signals
        let sig = match signame.to_uppercase().as_str() {
            "HUP" | "SIGHUP" => Sig::HUP,
            "INT" | "SIGINT" => Sig::INT,
            "QUIT" | "SIGQUIT" => Sig::QUIT,
            "ILL" | "SIGILL" => Sig::ILL,
            "ABRT" | "SIGABRT" => Sig::ABRT,
            "FPE" | "SIGFPE" => Sig::FPE,
            "KILL" | "SIGKILL" => Sig::KILL,
            "USR1" | "SIGUSR1" => Sig::USR1,
            "SEGV" | "SIGSEGV" => Sig::SEGV,
            "PIPE" | "SIGPIPE" => Sig::PIPE,
            "ALRM" | "SIGALRM" => Sig::ALRM,
            "TERM" | "SIGTERM" => Sig::TERM,
            _ => anyhow::bail!("Unknown or unsupported signal: {}", signame),
        };
        self.inner.signal(sig).await?;
        Ok(())
    }

    /// Write data to the channel (stdin).
    pub async fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.inner.data(data).await?;
        Ok(())
    }

    /// Write EOF to indicate end of input.
    pub async fn eof(&mut self) -> anyhow::Result<()> {
        self.inner.eof().await?;
        Ok(())
    }

    /// Close the channel.
    pub async fn close(&mut self) -> anyhow::Result<()> {
        self.inner.close().await?;
        Ok(())
    }

    /// Wait for the next message from the channel.
    ///
    /// Returns None when the channel is closed.
    pub async fn wait(&mut self) -> Option<ChannelMsg> {
        self.inner.wait().await
    }

    /// Get the channel ID.
    pub fn id(&self) -> russh::ChannelId {
        self.inner.id()
    }

    /// Get the exit status if the channel has closed.
    pub fn exit_status(&self) -> Option<u32> {
        self.exit_status
    }

    /// Get the exit signal if terminated by signal.
    pub fn exit_signal(&self) -> Option<&str> {
        self.exit_signal.as_deref()
    }

    /// Process a channel message and update internal state.
    pub fn process_message(&mut self, msg: &ChannelMsg) {
        match msg {
            ChannelMsg::ExitStatus { exit_status } => {
                self.exit_status = Some(*exit_status);
            }
            ChannelMsg::ExitSignal { signal_name, .. } => {
                // Convert Sig enum to string
                let sig_str = match signal_name {
                    Sig::ABRT => "ABRT",
                    Sig::ALRM => "ALRM",
                    Sig::FPE => "FPE",
                    Sig::HUP => "HUP",
                    Sig::ILL => "ILL",
                    Sig::INT => "INT",
                    Sig::KILL => "KILL",
                    Sig::PIPE => "PIPE",
                    Sig::QUIT => "QUIT",
                    Sig::SEGV => "SEGV",
                    Sig::TERM => "TERM",
                    Sig::USR1 => "USR1",
                    _ => "UNKNOWN",
                };
                self.exit_signal = Some(sig_str.to_string());
            }
            _ => {}
        }
    }

    /// Request SSH agent forwarding on this channel.
    pub async fn request_agent_forwarding(&mut self) -> anyhow::Result<()> {
        self.inner.agent_forward(true).await?;
        Ok(())
    }
}

impl std::fmt::Debug for RusshChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RusshChannel")
            .field("id", &self.inner.id())
            .field("exit_status", &self.exit_status)
            .field("exit_signal", &self.exit_signal)
            .finish()
    }
}

/// Synchronous read/write wrapper for a russh channel.
///
/// This bridges russh's async channel to synchronous Read/Write traits
/// using internal buffering and the tokio runtime.
pub struct RusshChannelStream {
    /// Channel for sending data to write
    write_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Buffer for received data
    read_buffer: Arc<Mutex<Vec<u8>>>,
    /// Position in read buffer
    read_pos: usize,
}

impl RusshChannelStream {
    /// Create a new channel stream wrapper.
    ///
    /// Spawns background tasks to handle async read/write operations.
    pub fn new(mut channel: RusshChannel) -> Self {
        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let read_buffer = Arc::new(Mutex::new(Vec::new()));
        let read_buffer_clone = read_buffer.clone();

        // Spawn task to handle channel I/O
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle outgoing data
                    Some(data) = write_rx.recv() => {
                        if let Err(e) = channel.write(&data).await {
                            log::error!("Error writing to channel: {}", e);
                            break;
                        }
                    }
                    // Handle incoming messages
                    msg = channel.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { data }) => {
                                let mut buf = read_buffer_clone.lock().unwrap();
                                buf.extend_from_slice(&data);
                            }
                            Some(ChannelMsg::ExtendedData { data, ext }) => {
                                // Extended data (usually stderr, ext=1)
                                if ext == 1 {
                                    let mut buf = read_buffer_clone.lock().unwrap();
                                    buf.extend_from_slice(&data);
                                }
                            }
                            Some(msg) => {
                                channel.process_message(&msg);
                                if matches!(msg, ChannelMsg::Eof | ChannelMsg::Close) {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        Self {
            write_tx,
            read_buffer,
            read_pos: 0,
        }
    }
}

impl Read for RusshChannelStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read_buf = self.read_buffer.lock().unwrap();
        if self.read_pos >= read_buf.len() {
            // No data available
            return Ok(0);
        }

        let available = &read_buf[self.read_pos..];
        let to_copy = std::cmp::min(buf.len(), available.len());
        buf[..to_copy].copy_from_slice(&available[..to_copy]);
        self.read_pos += to_copy;

        // Clear consumed data periodically
        if self.read_pos > 8192 {
            read_buf.drain(..self.read_pos);
            self.read_pos = 0;
        }

        Ok(to_copy)
    }
}

impl Write for RusshChannelStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_tx
            .send(buf.to_vec())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
