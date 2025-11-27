# Changelog

## Unreleased

### Changes

- Lower MSRV from 1.70 to 1.64 (@jayvdb).

## 0.2.0 - 2025-11-26

### Bugfixes

- Fix handling of CRLF line endings (@dougvalenta).
- Use 1-based mark offsets (@jayvdb).

## 0.1.1 - 2024-02-11

### Added

- Implement `PartialEq` and `Debug` for `Event` and `Token`.

### Bugfixes

- Fix a bug where marks would not be correctly set for tokens and events.
