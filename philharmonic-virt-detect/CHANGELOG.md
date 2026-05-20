# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial crate scaffold with a stub `detect_virtualization()`
  public function returning `"none"`. The never-fail contract
  is fixed at the API surface; substantive probe logic is
  being moved over from the in-tree
  `xtask/src/bin/detect-virt.rs` xtask binary in a follow-up
  round.
