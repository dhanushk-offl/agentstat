#!/usr/bin/env python3
"""Check checked-in provider coverage, feature flags, specs, and test fixtures."""
import json
import pathlib
import tomllib

root = pathlib.Path(__file__).resolve().parents[1]
manifest = json.loads((root / "docs/coverage.json").read_text())
fixtures = json.loads((root / "tests/fixtures/providers.json").read_text())
specs = json.loads((root / "src/providers/specs.json").read_text())
cargo = tomllib.loads((root / "Cargo.toml").read_text())
providers = manifest["providers"]
ids = [provider["id"] for provider in providers]
assert len(ids) == len(set(ids)) == 106
assert set(ids) == set(cargo["features"]["all-providers"])
assert all(provider in cargo["features"] for provider in ids)
implemented = {provider["id"] for provider in providers if provider["status"] == "implemented"}
fixture_ids = {fixture["provider"] for fixture in fixtures}
assert implemented - {"coderabbit"} == fixture_ids
assert set(specs) <= implemented
assert all(provider["name"] for provider in providers)
for provider in providers:
    assert provider["status"] in {"implemented", "planned", "unsupported"}
    assert provider["status"] == "implemented" or provider["reason"]
print(f"Coverage checked: {len(implemented)} implemented, {len(ids)} catalogued.")
