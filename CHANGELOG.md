# Changelog for peridio 0.7

**NOTE:** if you are currently using a version < 0.6.3, you must upgrade manually from
https://github.com/peridio/morel/releases.

Peridio CLI 0.7 adds support for setting a device's target during creation and update.

## 0.7.2

### Fixes

- Fix issue with creation of deployments

### Improvements

- Add experimental `peridio artifacts`.
- Add experimental `peridio cohorts`.
- Add `--jitp_cohort_prn` option to `peridio ca_certificates create` command.
- Add `--jitp_cohort_prn` option to `peridio ca_certificates update` command.


## 0.7.1

### Fixes

- Fix authn regression in CI release creation.

## 0.7.0

### Improvements

- Add `--target` option to `peridio devices create` command.
- Add `--target` option to `peridio devices update` command.
