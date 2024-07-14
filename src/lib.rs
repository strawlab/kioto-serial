//! Provide serial port I/O using tokio.
//!
//! This crate provides an interface very similar to
//! [`tokio-serial`](https://crates.io/crates/tokio-serial) with a different
//! implementation. Ideally, it can serve as a drop-in replacement.
//!
//! The implementation uses synchronous blocking I/O to the serial port and then
//! wraps these with asynchronous channels.
#![deny(missing_docs)]

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{future::FutureExt, stream::StreamExt};
use pin_project_lite::pin_project;
use serialport::SerialPort;
use tokio::io::{AsyncWriteExt, ReadBuf};

// ensure that we never instantiate a NeverOk type
macro_rules! assert_never {
    ($never: expr) => {{
        let _: NeverOk = $never;
        unreachable!("NeverOk was instantiated");
    }};
}

/// Builder to open a serial port.
///
/// Create this by calling [new]. Open the port by calling
/// [SerialPortBuilderExt::open_native_async].
pub struct SerialPortBuilder {
    path: String,
    baud_rate: u32,
    max_buf_size: usize,
}

/// Create a [SerialPortBuilder] from a device path and a baud rate.
pub fn new<'a>(path: impl Into<std::borrow::Cow<'a, str>>, baud_rate: u32) -> SerialPortBuilder {
    SerialPortBuilder {
        path: path.into().into_owned(),
        baud_rate,
        max_buf_size: 1024,
    }
}

impl SerialPortBuilder {
    /// Set the maximum buffer size in the internal buffer.
    pub fn max_buf_size(self, max_buf_size: usize) -> Self {
        Self {
            max_buf_size,
            ..self
        }
    }
}

/// Provides a convenience function for maximum compatibility with `tokio-serial`.
pub trait SerialPortBuilderExt {
    /// Open a serial port and return it as a [SerialStream].
    fn open_native_async(self) -> std::io::Result<SerialStream>;
}

impl SerialPortBuilderExt for SerialPortBuilder {
    fn open_native_async(self) -> std::io::Result<SerialStream> {
        let port = serialport::new(self.path, self.baud_rate).open()?;
        open(port, self.max_buf_size)
    }
}

pin_project! {
    /// An asynchronous implementation of a serial port.
    ///
    /// Implements both [tokio::io::AsyncRead] and [tokio::io::AsyncWrite].
    ///
    /// This could be wrapped with
    /// [`tokio_util::codec::Framed`](https://docs.rs/tokio-util/0.7.11/tokio_util/codec/struct.Framed.html),
    /// for example.
    pub struct SerialStream {
        #[pin]
        read_err: Pin<Box<dyn Future<Output = Result<NeverOk, Error>> + Send>>,
        #[pin]
        write_err: Pin<Box<dyn Future<Output = Result<NeverOk, Error>> + Send>>,
        #[pin]
        reader_duplex: tokio::io::ReadHalf<tokio::io::DuplexStream>,
        #[pin]
        writer_duplex: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    }
}

// ----------- implementation details below here -----------

impl tokio::io::AsyncRead for SerialStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let this = self.project();
        match this.read_err.poll(cx) {
            Poll::Pending => this.reader_duplex.poll_read(cx, buf),
            Poll::Ready(res) => Poll::Ready(to_std_io(res)),
        }
    }
}

impl tokio::io::AsyncWrite for SerialStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.project();
        match this.write_err.poll(cx) {
            Poll::Pending => this.writer_duplex.poll_write(cx, buf),
            Poll::Ready(res) => Poll::Ready(to_std_io(res)),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        let this = self.project();
        match this.write_err.poll(cx) {
            Poll::Pending => this.writer_duplex.poll_flush(cx),
            Poll::Ready(res) => Poll::Ready(to_std_io(res)),
        }
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let this = self.project();
        match this.write_err.poll(cx) {
            Poll::Pending => this.writer_duplex.poll_shutdown(cx),
            Poll::Ready(res) => Poll::Ready(to_std_io(res)),
        }
    }
}

/// A zero-sized type which is never created to indicate that Ok(_) never
/// happens.
#[derive(Debug)]
enum NeverOk {}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("IO error {0}")]
    Io(#[from] std::io::Error),
    #[error("sending thread paniced {0}")]
    OneshotRecv(tokio::sync::oneshot::error::RecvError),
    #[error("sending channel closed")]
    SenderClosed,
}

/// Read loop, launched on own thread. Returns only on error.
fn reader(
    mut port: Box<dyn SerialPort>,
    mut tx: tokio::io::WriteHalf<tokio::io::DuplexStream>,
) -> Result<NeverOk, Error> {
    let mut buffer = vec![0u8; 1024];
    loop {
        let sz = port.read(&mut buffer)?;
        futures::executor::block_on(tx.write_all(&buffer[..sz]))?;
    }
}

/// Write loop, launched on own thread. Returns only on error.
fn writer(
    mut port: Box<dyn SerialPort>,
    rx: tokio::io::ReadHalf<tokio::io::DuplexStream>,
) -> Result<NeverOk, Error> {
    let mut rx = tokio_util::io::ReaderStream::new(rx);
    while let Some(buf) = futures::executor::block_on(rx.next()) {
        let buf = buf?;
        port.write_all(&buf[..])?
    }
    Err(Error::SenderClosed)
}

/// Opens a serial port and returns a [SerialStream] to read and write to
/// it.
///
/// Reading and writing to the serial port is handled by two newly spawned
/// threads.
fn open(
    mut port: Box<dyn serialport::SerialPort>,
    max_buf_size: usize,
) -> std::io::Result<SerialStream> {
    // Convert port to blocking (more-or-less). Actually a 100 year timeout.
    // See https://github.com/serialport/serialport-rs/pull/185 for full blocking.
    port.set_timeout(std::time::Duration::from_secs(60 * 60 * 24 * 365 * 100))?;

    let write_port = port.try_clone()?;

    let (for_rw_threads, duplex) = tokio::io::duplex(max_buf_size);
    let (read_half, write_half) = tokio::io::split(for_rw_threads);
    let (read_thread_result_tx, read_thread_result_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        read_thread_result_tx
            .send(reader(port, write_half))
            .unwrap();
    });
    let (write_thread_result_tx, write_thread_result_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        write_thread_result_tx
            .send(writer(write_port, read_half))
            .unwrap();
    });

    let (reader_duplex, writer_duplex) = tokio::io::split(duplex);
    Ok(SerialStream {
        read_err: Box::pin(read_thread_result_rx.map(flatten)),
        write_err: Box::pin(write_thread_result_rx.map(flatten)),
        reader_duplex,
        writer_duplex,
    })
}

/// convert our Result type to Result from std::io
fn to_std_io<T>(res: Result<NeverOk, Error>) -> std::io::Result<T> {
    match res {
        Ok(never) => assert_never!(never),
        Err(e) => match e {
            Error::Io(e) => Err(e),
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{other}"),
            )),
        },
    }
}

/// flatten Result<Result<_>> to Result<_>
fn flatten(
    full: Result<Result<NeverOk, Error>, tokio::sync::oneshot::error::RecvError>,
) -> Result<NeverOk, Error> {
    match full {
        Ok(res) => res,
        Err(e) => Err(Error::OneshotRecv(e)),
    }
}
