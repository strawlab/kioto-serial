# Crate `kioto-serial`

[![Crates.io](https://img.shields.io/crates/v/kioto-serial.svg)](https://crates.io/crates/kioto-serial)
[![Documentation](https://docs.rs/kioto-serial/badge.svg)](https://docs.rs/kioto-serial/)
[![Crate License](https://img.shields.io/crates/l/kioto-serial.svg)](https://crates.io/crates/kioto-serial)

Provide serial port I/O using tokio.

This crate provides an interface very similar to
[`tokio-serial`](https://crates.io/crates/tokio-serial) with a different
implementation. Ideally, it can serve as a drop-in replacement.

The implementation uses synchronous blocking I/O to the serial port and then
wraps these with asynchronous channels.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
