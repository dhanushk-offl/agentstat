# Development and validation

This is the first staged release of the library. The catalog tracks 106 provider
IDs and distinguishes implemented, planned, and unsupported integrations. Do not mark a planned adapter
implemented until it has both a verified request and normalization fixture.

## Architecture

`Client` applies feature/source/account checks and a whole-fetch deadline. Provider
adapters construct authenticated requests; `Transport` guards origins/redirects,
classifies errors, and bounds bodies. Private wire formats map into the public
snapshot types. Single-response adapters use audited JSON-pointer specifications;
Codex, Claude, organization cost pagination, and more complex quota arrays have
Rust implementations. There is no embedded scripting runtime.

New providers need a Cargo feature, catalog/coverage entry, request implementation,
synthetic fixture, and expected normalized data. Never implement an inference probe, login flow, or credential refresh to
make an adapter appear supported. POST is allowed for audited read-only report
queries (Gemini quota retrieval and Warp GraphQL, for example).

The coverage table describes the data and authentication paths each adapter
implements. Additional endpoints and enrichments require their own verification.

## Checks

```sh
cargo fmt --check
cargo test --all-features
cargo test --no-default-features
cargo test
cargo clippy --all-features --all-targets -- -D warnings
cargo doc --all-features --no-deps
cargo package --allow-dirty --offline --no-verify
python3 scripts/check_coverage.py
python3 scripts/test_release.py
```

Compile individual feature combinations with `scripts/check_features.sh`.
The CI matrix validates stable Rust on Ubuntu, macOS, and Windows. This workspace
only verifies the local Linux toolchain; CI results must establish other platforms.
No explicit minimum Rust version is promised for this 0.1 release.

The ordinary fixture suite runs entirely in memory, asserts actual constructed
HTTP methods/URLs/auth headers and normalized outputs, and checks every implemented
remote provider has a fixture. It covers errors, scope isolation, unknown and
unlimited limits, overages, missing data, exact money, and optional failures.
Socket smoke tests live in `tests/http.rs`; CI invokes them explicitly because
some agent sandboxes prohibit even loopback listeners.

## Provider maintenance

When a provider changes its endpoints or response schema, update the adapter,
request/normalization fixtures, and coverage notes together. Keep account routing
explicit and preserve missing values rather than inventing usage or allowances.
Changes to provider IDs must keep the Cargo features, catalog, coverage manifest,
and fixtures consistent.

Fixtures are synthetic examples of provider response schemas. They are not
production captures and do not establish live credentials or endpoint reachability.
For a live check, run an example manually with a token for the intended provider
and account, compare the returned windows against its usage page, and record the
crate version, platform, and date without retaining secrets. Never require live
account access in normal CI.

OpenCode has ignored live integration tests for its two credential paths. Run each
test only when its account and credential are available:

```bash
# Go API key: checks five-hour, weekly, and monthly percentages and reset times.
read -r -s -p "OpenCode Go API key: " OPENCODE_GO_API_KEY; echo
export OPENCODE_GO_API_KEY
cargo test --locked --features opencodego \
  --test live_opencode opencode_go_live_quota_windows -- --ignored
unset OPENCODE_GO_API_KEY

# Zen pay-as-you-go Console session cookie: checks 30-day spend.
# Set OPENCODE_WORKSPACE_ID too when the Console account has multiple workspaces.
read -r -s -p "OpenCode Console cookie: " OPENCODE_COOKIE; echo
export OPENCODE_COOKIE
cargo test --locked --features opencode \
  --test live_opencode opencode_zen_live_spend -- --ignored
unset OPENCODE_COOKIE OPENCODE_WORKSPACE_ID
```

Use an actual Go subscription for the first test and a Zen pay-as-you-go workspace
for the second. The tests fail on missing credentials, fetch errors, or missing
expected data; they do not print credentials or usage values. Compare the results
with the OpenCode Console before treating the account path as verified. These
tests are never part of normal CI.

## Packaging

`cargo package` is local validation only. Inspect the packaged file list and ensure the coverage manifest,
license, and examples are included. Release incremental 0.x versions
as more compatible providers are implemented. A future full-catalog claim requires
resolving all planned entries, not just compiling their feature flags.

The [release guide](releasing.md) describes token setup, tag validation, automatic
crates.io publication, GitHub Releases, and recovery. CI also runs as a reusable
workflow for release tags, so publication waits for the same checks as a PR.
