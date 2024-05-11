# Creating a CLI release

We manage CLI releases with GitHub tags and GitHub actions

## Update Cargo.toml

Update the `Cargo.toml` version according to semantic versioning

Save the file and make sure to run `cargo build` for `Cargo.lock` to update 

Push the changes with the commit message `stage x.y.z release`, the commit message is not a requirement but we do it for consistency

## Add a new tag 

Once the Cargo file changes are pushed and merged into main

- make sure to be in `main` and with the latest Cargo changes pulled 
- run `git tag x.y.z` which `x.y.z` should match your Cargo changes
- run `git push origin x.y.z` which `x.y.z` should match your Cargo changes

Once you do, the GitHub Action will build all the artifacts and generate a new release version automatically

# Checking the release

When the GitHub Action is finished check your release by running in your shell `peridio upgrade` and check it upgrades successfully to the version you released
