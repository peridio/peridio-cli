# Changelog for peridio 0.4

Peridio CLI 0.4 adds supports for profile-based configs and improves the CLI's UX via additional global options, aliases, and improved errors.

## Configuration Files

Allows global options to be specified within profiles within configuration files. There is a default location for configuration files, but this can be overridden. Profiles are selected via a single `--profile` flag. This enables one to switch between sets of base URLs, CA paths, organization names, and API keys with a single option.

For an exhaustive explanation and example usage, see the [Peridio CLI documentation](https://docs.peridio.com/cli).

## 0.4.0

### Breaking Changes

- Delta updates are no longer configurable at a product level, but instead at a deployment level.
  - Note that preexisting deployments for products that were delta updatable are automatically flagged as delta updatable themselves.

### Enhancements

- Add support for profile-based configs.
- Add `--organization-name` and `--ca-path` global options.
- Add `--delta-updatable` option to deployments.
- Add short aliases for a variety of commands.
- Improve existing error messages and produce error messages instead of panicing in many cases.
