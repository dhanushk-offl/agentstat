#!/usr/bin/env python3
"""Exercise release-version and checkout guards without a registry or credentials."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
from validate_release import release_metadata, validate_checkout


def manifest(version: str) -> dict:
    return {"package": {"name": "agentstat", "version": version}}


class ReleaseMetadataTests(unittest.TestCase):
    def test_stable_and_prerelease_versions(self):
        for version, prerelease in [
            ("0.1.0", "false"),
            ("1.2.3-beta.1", "true"),
            ("1.2.3-rc.0+build.7", "true"),
            ("1.2.3+build.7", "false"),
        ]:
            with self.subTest(version=version):
                self.assertEqual(release_metadata(manifest(version), f"v{version}"), {
                    "version": version, "tag": f"v{version}", "prerelease": prerelease,
                })

    def test_mismatched_and_unsafe_tags_are_rejected(self):
        for tag in ["v0.2.0", "0.1.0", "v0.1.0\nprerelease=true", "v0.1.0;echo unsafe"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release_metadata(manifest("0.1.0"), tag)

    def test_invalid_versions_are_rejected(self):
        for version in ["01.2.3", "1.2", "1.2.3-01", "1.2.3-beta..1", "1.2.3+", "latest", None]:
            with self.subTest(version=version), self.assertRaises(ValueError):
                release_metadata(manifest(version), f"v{version}")

    def test_wrong_package_is_rejected(self):
        with self.assertRaises(ValueError):
            release_metadata({"package": {"name": "different", "version": "0.1.0"}}, "v0.1.0")


class ReleaseCheckoutTests(unittest.TestCase):
    def test_tag_must_point_to_checkout_and_be_reachable_from_main(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def git(*args):
                return subprocess.run([
                    "git", "-c", "user.name=Release Test", "-c", "user.email=release@example.invalid",
                    "-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false", *args,
                ], cwd=root, check=True, capture_output=True, text=True).stdout.strip()

            git("init", "-b", "main")
            git("commit", "--allow-empty", "-m", "initial")
            main = git("rev-parse", "HEAD")
            git("update-ref", "refs/remotes/origin/main", main)
            git("tag", "-a", "v0.1.0", "-m", "Release")
            validate_checkout(root, "v0.1.0")
            with self.assertRaises(ValueError):
                validate_checkout(root, "v9.9.9")
            git("checkout", "-b", "feature")
            git("commit", "--allow-empty", "-m", "unmerged")
            with self.assertRaises(ValueError):
                validate_checkout(root, "v0.1.0")
            git("tag", "v0.2.0")
            with self.assertRaises(ValueError):
                validate_checkout(root, "v0.2.0")
            # An older tagged commit remains releasable after main advances.
            git("update-ref", "refs/remotes/origin/main", git("rev-parse", "HEAD"))
            git("checkout", "--detach", main)
            validate_checkout(root, "v0.1.0")


if __name__ == "__main__":
    unittest.main()
