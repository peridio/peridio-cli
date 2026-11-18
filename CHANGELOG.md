# peridio-cli

## Unreleased

* Bug fixes
  * `binaries create` streams each binary part from disk instead of loading the whole file into memory
  * Failed binary part uploads are reported as errors, including in `bundles push`, instead of being ignored
  * A failed upload reports every failed binary part, and a part upload is abandoned when no data moves for 60 seconds instead of hanging
  * Binary content that needs more than 10,000 parts fails up front with the `--binary-part-size` to use
  * Binary content that is shorter than expected, or whose size changed after hashing, fails the upload

## v0.22.2

* Bug fixes
  * Add support for optionally passing `--jitp-target` to `ca-certificates` `create` and `update`
