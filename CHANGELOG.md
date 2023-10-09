# Changelog for Peridio 0.11

Peridio CLI 0.11 adds support for updating `next_release_prn` on releases.


## 0.11.0

### Features

- Add command `peridio products-v2`
- Add options to `peridio releases update`
  - `next_release_prn`
- Add options to `peridio devices create`
  - `cohort_prn`
- Add options to `peridio signing-keys create`
  - `key` (path to a public key raw file)
  - `path` (path to a public key pem file)

### Improvements

- `peridio binaries create` will now internally reuse hash calculations where possible. This improves total run-time of this command.
