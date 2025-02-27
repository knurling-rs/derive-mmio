# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Compile time check for padding now works reliably
- Add `no_ctors` attribute which allows the user to write custom constructors.

## [v0.3.0] - 2025-02-26

### Changed

* The `fn new_mmio` and `fn new_mmio_at` functions are now cost
* We no longer emit a different version of `fn new_mmio_at` using exposed
  provenence on Rust version 1.84 or higher - because that API is not (yet)
  const.

## [v0.2.0] - 2025-02-14

### Added

* `pointer_to_xxx` methods
* Support for `mmio(RO)` and `mmio(RW)` attributes to mark fields as read-only or read-write
* A check for padding within the struct (which is not allowed)

### Changed

* `read_xxx` methods now require `&mut self`

## [v0.1.0] - 2025-02-14

* First release
* Provides `read_xxx`, `write_xxx` and `modify_xxx` methods

[Unreleased]: https://github.com/knurling-rs/derive-mmio/compare/derive-mmio-v0.3.0...HEAD
[v0.3.0]: https://github.com/knurling-rs/derive-mmio/compare/derive-mmio-v0.2.0...derive-mmio-v0.3.0
[v0.2.0]: https://github.com/knurling-rs/derive-mmio/compare/derive-mmio-v0.1.0...derive-mmio-v0.2.0
[v0.1.0]: https://github.com/knurling-rs/derive-mmio/releases/tag/derive-mmio-v0.1.0
