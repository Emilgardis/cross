# Change Log

All notable changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

- Support for the `asmjs-unknown-emscripten` and `wasm32-unknown-emscripten`
  targets

### Added

- Support for the `arm-unknown-linux-gnueabi` target

## [v0.1.3] - 2017-01-01

### Changed

- Fix the `i686-unknown-linux-musl` target

## [v0.1.2] - 2016-12-31

### Added

- Support for `i686-unknown-linux-musl`
- Support for `cross build`ing crates for the `thumbv*-none-eabi*` targets.

## [v0.1.1] - 2016-12-28

### Added

- Support for `x86_64-unknown-linux-musl`
- Print shell commands when the verbose flag is used.
- Support crossing from x86_64 osx to i686 osx

## v0.1.0 - 2016-12-26

- Initial release. Supports 12 targets.

[Unreleased]: https://github.com/japaric/cross/compare/v0.1.3...HEAD
[v0.1.3]: https://github.com/japaric/cross/compare/v0.1.2...v0.1.3
[v0.1.2]: https://github.com/japaric/cross/compare/v0.1.1...v0.1.2
[v0.1.1]: https://github.com/japaric/cross/compare/v0.1.0...v0.1.1
