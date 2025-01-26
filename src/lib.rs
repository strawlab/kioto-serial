//! Provide serial port I/O using tokio.
//!
//! This crate provides an interface very similar to
//! [`tokio-serial`](https://crates.io/crates/tokio-serial) with a different
//! implementation. Ideally, it can serve as a drop-in replacement.
//!
//! Except on Windows (see below), the implementation uses synchronous blocking
//! I/O to the serial port and then wraps these with asynchronous channels.
//!
//! In Windows, `tokio-serial` is re-rexported because the approach used here,
//! cloning the serial port handle, simply does not work. Specifically, a
//! blocking read from the port blocks writing.
#![deny(missing_docs)]

#[cfg(target_os = "windows")]
pub use tokio_serial::*;

#[cfg(not(target_os = "windows"))]
mod posix;

#[cfg(not(target_os = "windows"))]
pub use posix::*;
