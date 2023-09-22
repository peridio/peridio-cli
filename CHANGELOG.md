# Changelog for Peridio 0.8

Peridio CLI 0.8 adds versioned configs, a notable UX improvement for binary creation, and 20 new commands.

## Versioned configs

- Peridio CLI versions >= 0.8 that encounter an old config require that it be upgraded to the new format.
  - Upgrade automatically via `peridio config upgrade`.

**Old**

```json
{
  "profile-name": {
    "organization_name": "org-name"
  }
}
```

**New**

- Add a version key.
- Nest profile content into a profiles section.
- Add a signing key pairs section that signing commands can leverage to improve their UX.

```json
{
  "version": 1,
  "profiles": {
    "profile-name": {
      "organization_name": "org-name"
    }
  },
  "signing_key_pairs": {
    "signing-key-pair-name": {
      "signing_key_prn": "signing-key-prn",
      "signing_key_private_path": "path-to-private-key-pem-file"
    }
  }
}
```

## UX improvements for binary creation

Where applicable, we recommend you use the following to create binaries:

```
peridio --profile profile-name binaries create \
  --artifact-version-prn artifact-version-prn \
  --content-path path-to-binary-content-file \
  --signing-key-pair signing-key-pair-name \
  --target target
```

In a single command, the above will:

1. Create the binary record.
2. Execute a resumable, parallel, multipart upload via binary parts to upload binary content.
3. Progress the binaries state to hashable, then hashing.
4. Poll the binary waiting for Peridio Cloud to finish hash verification.
5. Sign the binary via binary signatures.
6. The binary is signed and ready for bundling.

Providing the content path and signing key pair allows us to automate the error prone process of: measuring binary sizes, calculating rolling binary part request params, calculating and formatting hashes and signatures, and executing parallel requests in a resumable fashion.

## 0.8.0

### Features

- Add `peridio artifact-versions [create|list|get|update]`.
- Add `peridio binaries [create|list|get|update]`.
- Add `peridio binary-parts [create|list]`.
- Add `peridio binary-signatures [create|delete]`.
- Add `peridio bundles [create|list|get]`.
- Add `peridio config upgrade`.
- Add `peridio releases [create|list|get|update]`.

### Improvements

- Depend on the Peridio Rust SDK via [peridio-rust](https://github.com/peridio/peridio-rust) rather than its private origins in `reishi`.
