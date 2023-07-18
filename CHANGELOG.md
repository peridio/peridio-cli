# Changelog for peridio 0.6

**NOTE:** if you are currently using a version < 0.6.3, you must upgrade manually from
https://github.com/peridio/morel/releases.

Peridio CLI 0.6 expands the number of targets we provide pre-built binaries for.

## 0.6.4

### Fixes

- Fixed bug during device creation.

## 0.6.3

### Fixes

- Fixed bug where resolving a new version via `peridio upgrade` would fail.

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
