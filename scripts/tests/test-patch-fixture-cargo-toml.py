#!/usr/bin/env python3
"""Tests for .github/scripts/patch-fixture-cargo-toml.py.

Covers the enable_features() comma-separator fix introduced in the PR
(Issue #4784 / reinhardt-web#5xxx): when the existing features array body
has content that does NOT end with a trailing comma, a comma separator must
be inserted before the new features to produce valid TOML.
"""
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

# ---------------------------------------------------------------------------
# Load the script as a module (it has no .py package structure)
# ---------------------------------------------------------------------------

_SCRIPT = Path(__file__).resolve().parent.parent.parent / ".github" / "scripts" / "patch-fixture-cargo-toml.py"


def _load_script() -> object:
    spec = importlib.util.spec_from_file_location("patch_fixture_cargo_toml", _SCRIPT)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_mod = _load_script()
enable_features = _mod.enable_features
workspace_form = _mod.workspace_form


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _write(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


# ---------------------------------------------------------------------------
# Tests for enable_features()
# ---------------------------------------------------------------------------

class TestEnableFeaturesCommaSeparator(unittest.TestCase):
    """The central fix: ensure a comma is inserted when the last feature in the
    existing array does not already end with one."""

    def setUp(self) -> None:
        self._tmpdir = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmpdir.name)

    def tearDown(self) -> None:
        self._tmpdir.cleanup()

    def _manifest(self, content: str) -> Path:
        p = self.tmp / "Cargo.toml"
        _write(p, content)
        return p

    # ------------------------------------------------------------------
    # Core comma-separator scenarios (the bug fix)
    # ------------------------------------------------------------------

    def test_no_trailing_comma_single_feature(self) -> None:
        """Body has one feature WITHOUT a trailing comma — fix must add comma."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n\t"full"\n] }\n'
        )
        enable_features(manifest, ["client-router"])
        result = _read(manifest)
        # "full" must be followed by a comma before "client-router"
        self.assertIn('"full",', result)
        self.assertIn('"client-router"', result)
        # Validate the features array is syntactically well-formed (no double comma)
        self.assertNotIn(',,', result)

    def test_no_trailing_comma_multiple_existing_features(self) -> None:
        """Array has multiple features, last one has no trailing comma."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"full",\n'
            '\t"admin"\n'
            '] }\n'
        )
        enable_features(manifest, ["client-router"])
        result = _read(manifest)
        self.assertIn('"admin",', result)
        self.assertIn('"client-router"', result)
        self.assertNotIn(',,', result)

    def test_with_trailing_comma_no_duplicate_comma(self) -> None:
        """When the last feature already has a trailing comma, no extra comma
        must be added."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"full",\n'
            '] }\n'
        )
        enable_features(manifest, ["client-router"])
        result = _read(manifest)
        self.assertIn('"client-router"', result)
        # Must not produce double commas
        self.assertNotIn(',,', result)
        # The separator between "full," and new feature should not double-up
        self.assertNotIn('"full",,', result)

    # ------------------------------------------------------------------
    # Empty-body scenario
    # ------------------------------------------------------------------

    def test_empty_features_array(self) -> None:
        """An empty features array body should receive features without
        any leading comma (separator must be empty string)."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [] }\n'
        )
        enable_features(manifest, ["client-router"])
        result = _read(manifest)
        self.assertIn('"client-router"', result)
        # Must not start the array body with a comma
        self.assertNotIn('[,', result)

    def test_whitespace_only_body(self) -> None:
        """A features array containing only whitespace/newlines is treated as
        effectively empty; no leading comma should appear."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n\n] }\n'
        )
        enable_features(manifest, ["client-router"])
        result = _read(manifest)
        self.assertIn('"client-router"', result)
        self.assertNotIn('[,', result)

    # ------------------------------------------------------------------
    # Idempotency
    # ------------------------------------------------------------------

    def test_idempotent_feature_already_present(self) -> None:
        """Calling enable_features with a feature that already exists is a no-op."""
        original = (
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"client-router",\n'
            '] }\n'
        )
        manifest = self._manifest(original)
        enable_features(manifest, ["client-router"])
        self.assertEqual(_read(manifest), original)

    def test_idempotent_no_trailing_comma_already_present(self) -> None:
        """If feature is already present (no trailing comma), file is unchanged."""
        original = (
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"client-router"\n'
            '] }\n'
        )
        manifest = self._manifest(original)
        enable_features(manifest, ["client-router"])
        self.assertEqual(_read(manifest), original)

    def test_idempotent_called_twice(self) -> None:
        """Calling enable_features twice does not duplicate entries."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"full"\n'
            '] }\n'
        )
        enable_features(manifest, ["client-router"])
        after_first = _read(manifest)
        enable_features(manifest, ["client-router"])
        after_second = _read(manifest)
        self.assertEqual(after_first, after_second)
        self.assertEqual(after_second.count('"client-router"'), 1)

    # ------------------------------------------------------------------
    # Multiple new features
    # ------------------------------------------------------------------

    def test_multiple_new_features_no_trailing_comma(self) -> None:
        """Adding multiple features to array without trailing comma still produces
        valid TOML (comma injected once, not per-feature)."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"full"\n'
            '] }\n'
        )
        enable_features(manifest, ["client-router", "admin"])
        result = _read(manifest)
        self.assertIn('"client-router"', result)
        self.assertIn('"admin"', result)
        self.assertIn('"full",', result)
        self.assertNotIn(',,', result)

    def test_multiple_new_features_with_trailing_comma(self) -> None:
        """Adding multiple features when array already ends with comma."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"full",\n'
            '] }\n'
        )
        enable_features(manifest, ["client-router", "admin"])
        result = _read(manifest)
        self.assertIn('"client-router"', result)
        self.assertIn('"admin"', result)
        self.assertNotIn(',,', result)

    def test_partial_overlap_no_trailing_comma(self) -> None:
        """When some features to add already exist and array has no trailing comma,
        only missing ones are added and comma logic is still correct."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"full",\n'
            '\t"client-router"\n'
            '] }\n'
        )
        enable_features(manifest, ["client-router", "admin"])
        result = _read(manifest)
        self.assertIn('"admin"', result)
        self.assertEqual(result.count('"client-router"'), 1)
        self.assertIn('"client-router",', result)
        self.assertNotIn(',,', result)

    # ------------------------------------------------------------------
    # Missing features block → sys.exit(4)
    # ------------------------------------------------------------------

    def test_missing_features_block_exits(self) -> None:
        """A manifest with no `features = [...]` inside a reinhardt block must
        call sys.exit(4)."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0" }\n'
        )
        with self.assertRaises(SystemExit) as ctx:
            enable_features(manifest, ["client-router"])
        self.assertEqual(ctx.exception.code, 4)

    # ------------------------------------------------------------------
    # Output structure sanity
    # ------------------------------------------------------------------

    def test_new_feature_appended_inside_array(self) -> None:
        """The new feature line must appear inside the features array, not after
        the closing `}` of the dependency block."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"full"\n'
            '] }\n'
        )
        enable_features(manifest, ["client-router"])
        result = _read(manifest)
        feat_pos = result.index('"client-router"')
        close_brace_pos = result.index("] }")
        self.assertLess(feat_pos, close_brace_pos,
                        "new feature must appear before the closing `] }` of the block")

    def test_inline_quoted_feature_name(self) -> None:
        """Each appended feature must be double-quoted (TOML string syntax)."""
        manifest = self._manifest(
            'reinhardt = { version = "0.1.0", features = [\n'
            '\t"full",\n'
            '] }\n'
        )
        enable_features(manifest, ["client-router"])
        result = _read(manifest)
        self.assertIn('\t"client-router",\n', result)


# ---------------------------------------------------------------------------
# Tests for workspace_form() — covers the call to enable_features() it makes
# ---------------------------------------------------------------------------

class TestWorkspaceFormEnableFeatures(unittest.TestCase):
    """workspace_form calls enable_features(manifest, ["client-router"]).
    Verify the comma-separator fix holds end-to-end through workspace_form."""

    def setUp(self) -> None:
        self._tmpdir = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmpdir.name)
        self.reinhardt_path = Path("/fake/repo/reinhardt-web")

    def tearDown(self) -> None:
        self._tmpdir.cleanup()

    def _manifest(self, content: str) -> Path:
        p = self.tmp / "Cargo.toml"
        _write(p, content)
        return p

    def test_workspace_form_adds_client_router_no_trailing_comma(self) -> None:
        """workspace_form must correctly add client-router even when the
        existing features array has no trailing comma — end-to-end test."""
        manifest = self._manifest(
            '[dependencies]\n'
            'reinhardt = { version = "0.1.0", package = "reinhardt-web", features = [\n'
            '\t"full",\n'
            '\t"admin"\n'
            '] }\n'
        )
        workspace_form(manifest, self.reinhardt_path)
        result = _read(manifest)
        self.assertIn('"client-router"', result)
        # The inserted feature should be preceded by a comma on the prior line
        self.assertIn('"admin",', result)
        self.assertNotIn(',,', result)

    def test_workspace_form_adds_client_router_with_trailing_comma(self) -> None:
        """workspace_form must not double-comma when features already end with ','."""
        manifest = self._manifest(
            '[dependencies]\n'
            'reinhardt = { version = "0.1.0", package = "reinhardt-web", features = [\n'
            '\t"full",\n'
            '] }\n'
        )
        workspace_form(manifest, self.reinhardt_path)
        result = _read(manifest)
        self.assertIn('"client-router"', result)
        self.assertNotIn(',,', result)

    def test_workspace_form_replaces_version_with_path(self) -> None:
        """workspace_form must also rewrite version to path (unrelated to the
        comma fix but important for regression coverage)."""
        manifest = self._manifest(
            '[dependencies]\n'
            'reinhardt = { version = "0.1.0", package = "reinhardt-web", features = [\n'
            '\t"full",\n'
            '] }\n'
        )
        workspace_form(manifest, self.reinhardt_path)
        result = _read(manifest)
        self.assertIn(f'path = "{self.reinhardt_path}"', result)
        self.assertNotIn('version = "0.1.0"', result)


if __name__ == "__main__":
    unittest.main()
