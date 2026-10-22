#!/usr/bin/env python3
"""Validate a release tag before any publishing credentials are used."""

import argparse
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib


CORE = r"(?:0|[1-9][0-9]*)"
IDENTIFIER = r"(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
VERSION = re.compile(
    rf"{CORE}\.{CORE}\.{CORE}"
    rf"(?:-(?P<prerelease>{IDENTIFIER}(?:\.{IDENTIFIER})*))?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)


def release_metadata(manifest: dict, tag: str) -> dict[str, str]:
    package = manifest.get("package", {})
    if package.get("name") != "agentstat":
        raise ValueError("the release manifest must describe the agentstat package")
    version = package.get("version")
    match = VERSION.fullmatch(version) if isinstance(version, str) else None
    if match is None:
        raise ValueError("Cargo.toml must contain a valid explicit SemVer package version")
    if tag != f"v{version}":
        raise ValueError(f"release tag must be v{version}, received {tag!r}")
    return {
        "version": version,
        "tag": tag,
        "prerelease": "true" if match.group("prerelease") else "false",
    }


def validate_checkout(root: Path, tag: str) -> None:
    def git(*args: str) -> str:
        result = subprocess.run(
            ["git", *args], cwd=root, capture_output=True, text=True, check=False
        )
        if result.returncode:
            raise ValueError("release requires an existing tag checked out at a commit reachable from origin/main")
        return result.stdout.strip()

    tagged_commit = git("rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}")
    if tagged_commit != git("rev-parse", "HEAD"):
        raise ValueError("the checkout must be at the release tag's commit")
    git("merge-base", "--is-ancestor", "HEAD", "refs/remotes/origin/main")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    try:
        with (root / "Cargo.toml").open("rb") as file:
            metadata = release_metadata(tomllib.load(file), args.tag)
        validate_checkout(root, metadata["tag"])
    except (OSError, ValueError) as error:
        print(f"Release validation failed: {error}", file=sys.stderr)
        return 1

    if output := os.environ.get("GITHUB_OUTPUT"):
        with Path(output).open("a", encoding="utf-8") as file:
            for name, value in metadata.items():
                file.write(f"{name}={value}\n")
    print(f"Validated {metadata['tag']} at a commit on main")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
