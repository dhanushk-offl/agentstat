# Development and validation

This is the first staged release of the provider port, not the full 106-provider
implementation. The manifest records every upstream provider and distinguishes
implemented, planned, and incompatible sources. Do not mark a planned adapter
implemented until it has both a verified request and normalization fixture.

## Architecture

`Client` applies feature/source/account checks and a whole-fetch deadline. Provider
adapters construct authenticated requests; `Transport` guards origins/redirects,
classifies errors, and bounds bodies. Private wire formats map into the public
snapshot types. Single-response adapters use audited JSON-pointer specifications;
Codex, Claude, organization cost pagination, and more complex quota arrays have
Rust implementations. There is no embedded scripting runtime.

New providers need a Cargo feature, catalog/coverage entry, upstream source
reference, request implementation, synthetic fixture, and expected normalized
data. Never implement an inference probe, login flow, or credential refresh to
make an adapter appear supported. POST is allowed for audited read-only report
queries (Gemini quota retrieval and Warp GraphQL, for example).

Some implemented providers expose fewer enrichments than CodexBar. The coverage
table names the supported data, not every source or fallback upstream offers.

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

## Upstream maintenance

The pinned commit is `aa232d7f9ffce83dcb559bf7a56abc93c477b40c`. Source references
are in `docs/coverage.json` and the original license is shipped under
`THIRD_PARTY_LICENSES`. Before changing the pin, diff upstream registered IDs,
fetchers, bundled plugins, and their tests. Update endpoint/normalization fixtures,
coverage notes, the catalog revision constant, and attribution together.
Do not silently inherit upstream UI fallback or account-switching behavior.

The fixtures are deliberately synthetic examples of the inspected upstream wire
shapes. They are not production captures and do not establish live credentials
or endpoint reachability. For a live check, run an example manually with a token
for the intended provider and account, compare the returned windows against its
usage page, and record the revision/platform/date without retaining secrets.
Never require live account access in normal CI.

## Packaging

`cargo package` is local validation only; publishing to crates.io is a separate
release action. Inspect the packaged file list and ensure the coverage manifest,
license, attribution, and examples are included. Release incremental 0.x versions
as more compatible providers are ported. A future full-catalog claim requires
resolving all planned entries, not just compiling their feature flags.
