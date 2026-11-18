# peridio-cli

## Unreleased

* Bug fixes
  * `binaries create` streams each binary part from disk instead of loading the whole file into memory
  * Failed binary part uploads are reported as errors instead of being ignored

## v0.22.2

* Bug fixes
  * Add support for optionally passing `--jitp-target` to `ca-certificates` `create` and `update`
