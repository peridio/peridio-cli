use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;

#[test]
fn without_command_usage_is_shown() {
    Command::cargo_bin("peridio-cli")
        .unwrap()
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "Usage: peridio-cli [OPTIONS] <COMMAND>",
        ))
        .stderr(
            predicates::str::contains("-a, --api-key <API_KEY>")
                .and(predicates::str::contains("[env: PERIDIO_API_KEY]")),
        )
        .stderr(
            predicates::str::contains("-b, --base-url <BASE_URL>")
                .and(predicates::str::contains("[env: PERIDIO_BASE_URL=]")),
        )
        .stderr(
            predicates::str::contains("-c, --ca-path <CA_PATH>")
                .and(predicates::str::contains("[env: PERIDIO_CA_PATH=]")),
        )
        .stderr(
            predicates::str::contains("-o, --organization-name <ORGANIZATION_NAME>").and(
                predicates::str::contains("[env: PERIDIO_ORGANIZATION_NAME=]"),
            ),
        )
        .stderr(
            predicates::str::contains("-p, --profile <PROFILE>")
                .and(predicates::str::contains("[env: PERIDIO_PROFILE=]")),
        )
        .stderr(
            predicates::str::contains("-d, --config-directory <CONFIG_DIRECTORY>").and(
                predicates::str::contains("[env: PERIDIO_CONFIG_DIRECTORY=]"),
            ),
        );
}
