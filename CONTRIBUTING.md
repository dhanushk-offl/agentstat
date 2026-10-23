# Contributing to agentstat

Help improve provider coverage, fix parsing bugs, clarify documentation, or make
the crate easier to use in other Rust projects. Small, focused changes are welcome.
Please follow the [code of conduct](CODE_OF_CONDUCT.md).

## Before you start

Check [provider coverage](docs/providers.md) and existing
[issues](https://github.com/dhanushk-offl/agentstat/issues) before starting an adapter.
For a substantial API change, open an issue describing the problem and intended
behavior so the design can be discussed before implementation.

agentstat fetches current, provider-reported account data. UI, tray applications,
background polling, login flows, credential refresh, and estimated transcript
cost history are outside the library's scope. Callers own presentation and storage.

## Local setup

```sh
git clone https://github.com/dhanushk-offl/agentstat.git
cd agentstat
cargo test --all-features
```

Use stable Rust and Python 3.11 or newer for the coverage script. There is no
promised minimum Rust version for the initial release. Live provider accounts
are not required for the regular test suite.

## Reporting a bug

Include the crate version or commit, operating system, enabled Cargo features,
provider, credential type, and a minimal reproduction. Describe the expected and
actual behavior, including the typed error when available.

Remove tokens, cookies, account identifiers, and personal information from reports,
screenshots, and fixtures. Do not attach credential files or raw authenticated
HTTP captures. Report security issues using [SECURITY.md](SECURITY.md).

## Adding or updating a provider

1. Identify a read-only usage, quota, balance, or spend interface. A POST report
   query is acceptable; generating a completion to test access is not.
2. Add or update the Cargo feature, catalog descriptor, and adapter. Reuse the
   shared transport for authentication headers, response bounds, error handling,
   deadlines, and redirect policy.
3. Preserve the reported semantics. Do not infer token allowances from a
   percentage, treat missing data as zero, clamp valid overages, or mix currencies.
   Keep account scope explicit and reject unsupported routing.
4. Add synthetic request and normalization fixtures. Cover the important schema
   variants and failure cases, such as authentication failures, missing limits,
   resets, and exact amounts. Tests must not require live credentials or network.
5. Update `docs/coverage.json`, `docs/providers.md`, the README tables/counts, and
   relevant usage examples. A feature flag alone does not establish support.

Local file access and CLI probes require explicit opt-in. New probes must use
fixed, read-only arguments without a shell, bound output and execution time, and
clean up processes on cancellation. Document platform restrictions and any
behavior owned by the external CLI.

## Live provider checks

Checks with real accounts are especially helpful for the providers in the
[README table](README.md#currently-supported). Use a relevant command from
[Try your real account](README.md#try-your-real-account) or call `Client::fetch`
from your application with the provider's Cargo feature enabled. Compare the
reported windows, model labels, limits, reset times, balances, and spend with the
provider's own usage page.

Share the result in a [provider check](https://github.com/dhanushk-offl/agentstat/issues/new?template=provider_check.yml)
or pull request. Include the provider, crate version or commit, enabled features,
platform, credential type, date, and CLI version when relevant. Say which fields
matched or differed, and whether the fetch returned an error. Remove tokens,
cookies, account IDs, personal details, and raw authenticated response bodies.
If you can reproduce a mismatch with a synthetic response, add that as a fixture
with the fix.

## Checks before a pull request

```sh
cargo fmt --check
cargo test --all-features
cargo test --no-default-features
cargo test
cargo clippy --all-features --all-targets -- -D warnings
python3 scripts/check_coverage.py
```

For provider changes, also check the provider by itself with
`cargo check --no-default-features --features <provider>` and any local-access
features it needs. Use `scripts/check_features.sh` for the full feature matrix.
CI runs additional documentation, packaging, and socket checks; see the
[development guide](docs/development.md).

## Pull requests

Explain the concrete problem, resulting behavior, and validation performed.
Keep unrelated refactoring separate and mention known limitations. Documentation
changes should keep examples and relative links working. Do not commit credentials,
live account payloads, or build output.

Contributions are made under the project's [MIT license](LICENSE). Keep required
license notices when incorporating third-party work.
