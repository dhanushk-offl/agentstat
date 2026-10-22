#!/usr/bin/env bash
set -euo pipefail
cargo check --offline --no-default-features
cargo check --offline
cargo check --offline --all-features
while read -r provider; do
  cargo check --offline --no-default-features --features "$provider"
done < <(python3 -c 'import json; print("\n".join(p["id"] for p in json.load(open("docs/coverage.json"))["providers"] if p["status"] == "implemented" and p["id"] != "coderabbit"))')
cargo check --offline --no-default-features --features coderabbit,local-probes
