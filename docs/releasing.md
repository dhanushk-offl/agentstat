# Releasing agentstat

Pushing a version tag starts the release workflow. It validates the tag, runs the
same checks as pull requests, publishes to crates.io, and then creates a GitHub
Release with generated notes and the `.crate` archive.

## One-time setup

1. Create a crates.io account and verify its email address. Confirm that you can
   publish the `agentstat` crate name, or that your account is an owner if the
   crate already exists.
2. Create a crates.io API token authorized to publish `agentstat`. For the first
   release, the token must also permit creating the crate. Do not use a
   read-only or update-only token for the first publication.
3. In the GitHub repository, open **Settings → Secrets and variables → Actions**
   and add a repository secret named **`CARGO_REGISTRY_TOKEN`**. Paste the token
   as its value. Do not commit it or put it in a workflow file.
4. Ensure GitHub Actions is enabled. The GitHub Release job uses the automatic
   `GITHUB_TOKEN` with job-level `contents: write`; it needs no separate PAT.

You can also set the secret through the GitHub CLI's interactive prompt:

```sh
gh secret set CARGO_REGISTRY_TOKEN --repo dhanushk-offl/agentstat
```

Configure the **Checks passed** status as a required check in the `main` branch
rules if merges should wait for CI. Fork pull requests do not receive the
publishing secret; ordinary checks never reference it.

## Prepare and publish a version

Update `package.version` in `Cargo.toml` and refresh `Cargo.lock` with Cargo.
Update version-specific documentation, then commit and merge the change to `main`.
Every release needs a new version: crates.io versions cannot be overwritten.

Run the checks from [the development guide](development.md). An optional
`cargo publish --registry crates-io --locked --all-features --dry-run` also checks
the package without uploading it; registry access is required.

For version `0.1.1`, tag the merged commit and push the tag:

```sh
git switch main
git pull --ff-only origin main
git tag -a v0.1.1 -m "Release v0.1.1"
git push origin v0.1.1
```

The tag must exactly equal `v` followed by the package version. Its commit must
be reachable from `origin/main`. The workflow checks out the tag, including for
the reusable CI jobs and publication, rather than publishing a newer main commit.
It does not bump versions or create tags automatically.

Use a version such as `0.2.0-beta.1` and tag `v0.2.0-beta.1` for a prerelease.
The crate is published to crates.io normally and its GitHub Release is marked
as a prerelease. Push one release at a time: publication runs are serialized
and an active publication is not cancelled by a later tag.

## What runs

| Stage | Required behavior |
| --- | --- |
| Tag validation | Match the package name/version, checked-out tag commit, and main ancestry |
| Shared CI | Tests on Linux/macOS/Windows, formatting, Clippy, documentation, verified packaging, provider feature checks, coverage checks, release guard tests |
| Publish | Require the secret, run a publish dry run, then publish with locked dependencies and all features |
| Archive | Download the exact published `.crate` archive from crates.io |
| GitHub Release | Generate notes and attach the archive; use the prerelease flag when appropriate |

There is no manual approval step in the workflows. Pushing a valid release tag
authorizes publication after the checks succeed. The publishing token is scoped
to the secret-check and actual publication steps. GitHub Release creation has a
separate job and token permission.

Watch progress on [GitHub Actions](https://github.com/dhanushk-offl/agentstat/actions/workflows/release.yml)
or use `gh run list --workflow release.yml` and `gh run watch <run-id>`.

## Recovering a failed release

- **Tag mismatch or unmerged commit:** fix and merge the version change before
  creating the intended release tag. Never move a tag for an already-published
  version.
- **Missing or rejected token:** correct the Actions secret and its crates.io
  permissions, then rerun failed jobs after confirming no upload succeeded.
- **Validation, test, or package failure:** fix it on main and prepare a new
  release tag. No publishing job runs until the shared checks pass.
- **Upload timeout or uncertain publication:** check crates.io before retrying.
  A failed command can occur after an upload was accepted. An existing version
  is not silently skipped or overwritten by this workflow.
- **Publication succeeded but a later release step failed:** run the manual
  recovery workflow for the existing tag. It downloads the published archive
  from crates.io and creates the GitHub Release, or replaces its attached archive
  if the release already exists. It does not run checks or publish again:

  ```sh
  gh workflow run release.yml --ref main -f tag=v0.1.1
  gh run list --workflow release.yml --limit 5
  gh run watch <run-id> --exit-status
  gh release view v0.1.1
  ```

  Use the correct existing tag in place of `v0.1.1`. The updated workflow must
  be present on `main` before starting this recovery run. Never retry
  `cargo publish` for a version already listed on crates.io.

## Use the published crate

After crates.io lists the version, consumers can install it with:

```sh
cargo add agentstat@0.1.1
```

Or add it to `Cargo.toml`:

```toml
[dependencies]
agentstat = "0.1.1"
```

Provider and local-access features work the same way as for the local path
dependency. Publishing does not change an adapter's verification status; live
account compatibility remains separate from CI and release checks.
