# peridio-cli

## Unreleased

* Bug fixes
  * `binaries create` streams each binary part from disk instead of loading the whole file into memory
  * Failed binary part uploads are reported as errors, including in `bundles push`, instead of being ignored
  * Binary content that is shorter than expected, or whose size changed after hashing, fails the upload

## v0.22.2

* Bug fixes
  * Add support for optionally passing `--jitp-target` to `ca-certificates` `create` and `update`
