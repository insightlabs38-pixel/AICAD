# cad-cli

WP-13 (CLI + schemas). Stable command surface (`cad build/test/inspect/...`),
JSON tool schemas, exit codes, human output formatting, cancellation/progress.
All IDE/agent backends should reuse this crate's service-layer APIs rather
than shelling out when embedded; CLI behavior remains the conformance
reference.

Plan references: `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md`;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-13.
