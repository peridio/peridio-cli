# Changelog for peridio 0.2

Peridio CLI v0.2 introduces many new commands expanding support for the [Peridio Admin API](https://docs.peridio.com/admin-api) as well as adding support for upgrading the CLI in place.

## v0.2.0

### Breaking Changes

- `peridio --version` no longer concatenates the SemVer and Git SHA with a hyphen.
  - Old: `peridio 0.1.0-97aaa29`.
  - New: `peridio 0.2.0 c665657`.
- Global options (`--api-key` and `--base-url`) are now only accepted by the `peridio` command instead of by all subcommands. Global options can still be supplied via environment variables as before.
  - Old: `peridio [SUBCOMMANDS] [GLOBAL_OPTIONS] [SUBCOMMAND OPTIONS]`.
  - New: `peridio [GLOBAL_OPTIONS] [SUBCOMMANDS] [SUBCOMMAND OPTIONS]`.

### Enhancements

- Add `peridio deployments`.
  - Add `create`.
  - Add `delete`.
  - Add `get`.
  - Add `list`.
  - Add `update`.
- Add `peridio device-certificates`.
  - Add `create`.
  - Add `delete`.
  - Add `get`.
  - Add `list`.
- Add `peridio devices`.
  - Add `authenticate`.
  - Add `create`.
  - Add `delete`.
  - Add `get`.
  - Add `list`.
  - Add `update`.
- Add `peridio firmwares`.
  - Add `create`.
  - Add `delete`.
  - Add `get`.
  - Add `list`.
- Add `peridio firmwares`.
  - Add `create`.
  - Add `delete`.
  - Add `get`.
  - Add `list`.
- Add `peridio organizations`.
  - Add `add-user`.
  - Add `get-user`.
  - Add `list-users`.
  - Add `remove-user`.
  - Add `update-user`.
- Add `peridio products`.
  - Add `create`.
  - Add `delete`.
  - Add `get`.
  - Add `list`.
  - Add `update`.
  - Add `add-user`.
  - Add `remove-user`.
  - Add `get-user`.
  - Add `list-users`.
  - Add `update-user`.
- Add `peridio upgrade`.
