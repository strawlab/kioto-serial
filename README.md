# Crate `kioto-serial`

[![Crates.io](https://img.shields.io/crates/v/kioto-serial.svg)](https://crates.io/crates/kioto-serial)
[![Documentation](https://docs.rs/kioto-serial/badge.svg)](https://docs.rs/kioto-serial/)
[![Crate License](https://img.shields.io/crates/l/kioto-serial.svg)](https://crates.io/crates/kioto-serial)

Provide serial port I/O using tokio.

The implementation uses synchronous blocking I/O to the serial port in newly
spawned threads (one for reading and one for writing) and then wraps these with
asynchronous channels.

This crate provides an interface very similar to
[`tokio-serial`](https://crates.io/crates/tokio-serial) with a different
implementation. Ideally, it can serve as a drop-in replacement. In other words,
in `Cargo.toml` `[dependencies]` section, you could use

```
tokio-serial = { package = "kioto-serial", version = "0.1.0" }
```

and continue with code originally written for `tokio-serial`.

## Why write this and not just use `tokio-serial`?

As noted above, this crate uses spawned threads and blocking serial I/O.
Theoretically, this is not optimal because it is an async facade over a
fundamentally blocking implementation. So why are we taking this approach?

We were experiencing long latencies from the tokio scheduler when using
`tokio-serial`. Specifically, in a linux program involving simultaneous serial
connections to different devices and other tokio tasks such as a webserver and a
`tokio::time::Interval` timer with 1 msec resolution, the ticks from the timer
would be very irregular. In the process of debugging, we wrote `kioto-serial`
and found that using it instead of `tokio-serial` solved this latency issue. Since
that point, we have not delved deeper into `tokio-serial` to attempt to localize
the issue. See https://github.com/berkowski/tokio-serial/issues/72.

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
