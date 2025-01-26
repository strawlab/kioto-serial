#[cfg(target_os = "windows")]
pub use tokio_serial::*;

#[cfg(not(target_os = "windows"))]
mod posix;

#[cfg(not(target_os = "windows"))]
pub use posix::*;
