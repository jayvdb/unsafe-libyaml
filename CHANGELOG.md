# Changelog

## Unreleased

### Breaking changes
- Migrated to Rust Edition 2024 (MSRV 1.85).
- `Scanner` and `Parser` are now generic over the input stream, instead of using dynamic
  dispatch `dyn BufRead`. This allows the input to be owned by the parser using `std::io::Cursor`.

## 0.2.0 - 2025-11-26

### Bugfixes
- Fix handling of CRLF line endings (@dougvalenta).
- Use 1-based mark offsets (@jayvdb).

## 0.1.1 - 2024-02-11
### Added
- Implement `PartialEq` and `Debug` for `Event` and `Token`.
### Bugfixes
- Fix a bug where marks would not be correctly set for tokens and events.
