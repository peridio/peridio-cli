# Changelog for peridio 0.6

Peridio CLI 0.6 expands the number of targets we provide pre-built binaries for.

## 0.6.2

### Fixes

- Fixed bug where decoding device payloads that included firmware metadata would fail.

## 0.6.1

### Fixes

- Only suffix pre-built binaries for Windows with ".exe".

## 0.6.0

### Enhancements

- Expand the number of targets we provide pre-built binaries for. Supported targets now include:
  - x86_64-apple-darwin
  - x86_64-unknown-linux-gnu
  - aarch64-unknown-linux-musl
  - x86_64-unknown-linux-musl
  - x86_64-pc-windows-msvc
