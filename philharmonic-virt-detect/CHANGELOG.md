# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Moved the substantive `detect-virt` probe into this crate.
  `detect_virtualization()` now catches panics and I/O failures
  at the crate boundary and returns `"none"` for every failure
  path.
