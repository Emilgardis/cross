[![crates.io](https://img.shields.io/crates/v/cross.svg)](https://crates.io/crates/cross)
[![crates.io](https://img.shields.io/crates/d/cross.svg)](https://crates.io/crates/cross)

# `cross`

> "Zero setup" cross compilation and "cross testing" of Rust crates

<p align="center">
<img
  alt="`cross test`ing a crate for the aarch64-unknown-linux-gnu target"
  src="assets/cross-test.png"
  title="`cross test`ing a crate for the aarch64-unknown-linux-gnu target"
>
<br>
<em>`cross test`ing a crate for the aarch64-unknown-linux-gnu target</em>
</p>

**Disclaimer**: Only works on a x86_64 Linux host (e.g. Travis CI is supported)

## Features

- `cross` will provide all the ingredients needed for cross compilation without
  touching your system installation.

- `cross` provides an environment, cross toolchain and cross compiled libraries
  (e.g. OpenSSL), that produces the most portable binaries.

- "cross testing", `cross` can test crates for architectures other than i686 and
  x86_64.

- The stable, beta and nightly channels are supported.

## Dependencies

- [rustup](https://rustup.rs/)

- [Docker](https://www.docker.com/)

- A Linux kernel with [binfmt_misc] support is required for cross testing.

[binfmt_misc]: https://www.kernel.org/doc/Documentation/binfmt_misc.txt

## Installation

```
$ cargo install cross
```

## Usage

`cross` has the exact same CLI as [Cargo](https://github.com/rust-lang/cargo)
but as it relies on Docker you'll have to start the daemon before you can use
it.

```
# (ONCE PER BOOT)
# Start the Docker daemon, if it's not already running
$ sudo systemctl start docker

# (ONCE PER CARGO PROJECT)
# `cross` can't generate .lock files itself (see caveats section)
# if compiling a library, we'll have to use Cargo to generate the lock file
$ cargo generate-lockfile

# MAGIC! This Just Works
$ cross build --target aarch64-unknown-linux-gnu

# EVEN MORE MAGICAL! This also Just Works
$ cross test --target mips64-unknown-linux-gnuabi64

# Obviously, this also Just Works
$ cross rustc --target powerpc-unknown-linux-gnu --release -- -C lto
```

## Supported targets

A target is considered as "supported" if `cross` can cross compile a
"non-trivial" (binary) crate, usually Cargo, for that target.

Testing support is more complicated. It relies on QEMU user emulation, so
testing may sometimes fail due to QEMU bug sand not because there's a bug in the
crate. That being said, `cross test` is assumed to "work" (`test` column in the
table below) if it can successfully
run [compiler-builtins](https://github.com/rust-lang-nursery/compiler-builtins)
test suite.

Also, testing is very slow. `cross` will actually run units tests *sequentially*
because QEMU gets upset when you spawn several threads. This also means that, if
one of your unit tests spawns several threads then it's more likely to fail or,
worst, "hang" (never terminate).

| Target                               |  libc  | cc     | QEMU  | OpenSSL | `test` |
|--------------------------------------|--------|--------|-------|---------|:------:|
| `aarch64-unknown-linux-gnu`          | 2.19   | 4.8.2  | 2.8.0 | 1.0.2j  |   ✓    |
| `arm-unknown-linux-gnueabi`          | 2.19   | 4.8.2  | 2.8.0 | 1.0.2j  |   ✓    |
| `armv7-unknown-linux-gnueabihf`      | 2.15   | 4.6.3  | 2.8.0 | 1.0.2j  |   ✓    |
| `asm-unknown-emscripten`             | N/A    | 1.37.1 | N/A   | N/A     |        |
| `i686-unknown-linux-gnu`             | 2.15   | 4.6.3  | N/A   | 1.0.2j  |   ✓    |
| `i686-unknown-linux-musl`            | 1.1.15 | 5.3.1  | N/A   | N/A     |   ✓    |
| `mips-unknown-linux-gnu`             | 2.23   | 5.3.1  | 2.8.0 | 1.0.2j  |   ✓    |
| `mips64-unknown-linux-gnuabi64`      | 2.23   | 5.3.1  | 2.8.0 | 1.0.2j  |   ✓    |
| `mips64el-unknown-linux-gnuabi64`    | 2.23   | 5.3.1  | 2.8.0 | 1.0.2j  |   ✓    |
| `mipsel-unknown-linux-gnu`           | 2.23   | 5.3.1  | 2.8.0 | 1.0.2j  |   ✓    |
| `powerpc-unknown-linux-gnu`          | 2.19   | 4.8.2  | 2.7.1 | 1.0.2j  |   ✓    |
| `powerpc64-unknown-linux-gnu`        | 2.19   | 4.8.2  | 2.7.1 | 1.0.2j  |   ✓    |
| `powerpc64le-unknown-linux-gnu`      | 2.19   | 4.8.2  | 2.7.1 | 1.0.2j  |   ✓    |
| `s390x-unknown-linux-gnu`            | 2.23   | 5.3.1  | 2.8.0 | 1.0.2j  |        |
| `thumbv6m-none-eabi`                 | N/A    | 5.3.1  | N/A   | N/A     |        |
| `thumbv7em-none-eabi`                | N/A    | 5.3.1  | N/A   | N/A     |        |
| `thumbv7em-none-eabihf`              | N/A    | 5.3.1  | N/A   | N/A     |        |
| `thumbv7m-none-eabi`                 | N/A    | 5.3.1  | N/A   | N/A     |        |
| `wasm32-unknown-emscripten`          | N/A    | 1.37.1 | N/A   | N/A     |        |
| `x86_64-unknown-linux-gnu`           | 2.15   | 4.6.3  | N/A   | 1.0.2j  |   ✓    |
| `x86_64-unknown-linux-musl`          | 1.1.15 | 5.3.1  | N/A   | 1.0.2j  |   ✓    |

## Caveats / gotchas

- `cross` will mount the Cargo project as READ ONLY. Thus, if any crate attempts
  to modify its "source", the build will fail. Well behaved crates should only
  ever write to `$OUT_DIR` and never modify `$CARGO_MANIFEST_DIR` though.
  - This is the reason why `cross` can't generate .lock files and you have to
    manually call `cargo generate-lockfile`.

- Versions `0.7.*` and older of the `openssl` crate are NOT supported. `cross`
  supports `openssl` via the `OPENSSL_DIR` "feature", which seems to have been
  introduced in `0.8.*`. There's no work around, other than bumping the
  `openssl` dependency of the crates you are using.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
