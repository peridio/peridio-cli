# Changelog for Peridio 0.9

Peridio CLI 0.9 adds resumability and improvements to `peridio binaries create` as well fixes to `peridio config upgrade`.

## Resumability

`peridio binaries create` can resume if interrupted.

In particular, this is helpful when uploading a large binary, e.g., if you uploaded 4 GB of a 5 GB binary and the CLI exited for whatever reason, you could run the same command again, and the CLI will leverage binary part evaluation and pickup the upload from where you left off, instead of having to reupload all of the content you already did. This dramatically reduces the time and bandwidth spent uploading binaries to Peridio.

If you provide different binary content than initially, the command will clear any stale data and proceed with the new data.

## 0.9.0

### Fixes

- `peridio config upgrade` writes formatted instead of minified json.

### Improvements

- `peridio binaries create`
  - add resumability.
  - prints progress to stderr instead of stdout.
