"""Exercise release baseline selection with real Git repositories."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "run-release-pr.sh"
WORKFLOW = SCRIPT.parents[1] / ".github/workflows/release-plz.yml"


class ReleaseBaselineTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="reinhardt release test ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.run_git("init", "-q", "-b", "main")
        self.run_git("config", "user.name", "Release Test")
        self.run_git("config", "user.email", "release-test@example.invalid")
        (self.repo / "src").mkdir()
        (self.repo / "src/lib.rs").write_text("pub fn stable_api() {}\n")
        (self.repo / "release-plz.toml").write_text("[workspace]\nsemver_check = true\n")
        self.write_version("0.3.16")
        self.commit("chore: release")
        self.run_git("tag", "reinhardt-web@v0.3.16")
        self.stable_commit = self.run_git("rev-parse", "HEAD")
        self.run_git("checkout", "-q", "-b", "develop/0.4.0")
        self.write_version("0.4.0-alpha.15")
        (self.repo / "src/lib.rs").write_text("pub fn develop_api() {}\n")
        self.commit("feat!: develop API")
        self.run_git("tag", "reinhardt-web@v0.4.0-alpha.15")
        self.run_git("checkout", "-q", "main")
        (self.repo / "src/lib.rs").write_text("pub fn stable_api() { /* fixed */ }\n")
        self.commit("fix: stable API")

        self.bin = self.root / "bin"
        self.bin.mkdir()
        mock = self.bin / "release-plz"
        mock.write_text('''#!/usr/bin/env python3
import json, os, pathlib, subprocess, sys
args = sys.argv[1:]
record = {"args": args}
if "--registry-manifest-path" in args:
    manifest = pathlib.Path(args[args.index("--registry-manifest-path") + 1])
    record["baseline"] = str(manifest.parent)
    record["version"] = manifest.read_text()
    record["dependencies"] = json.loads(subprocess.check_output(
        ["cargo", "read-manifest", "--manifest-path", str(manifest)], text=True))["dependencies"]
    record["source"] = (manifest.parent / "src/lib.rs").read_text()
    record["sha"] = subprocess.check_output(
        ["git", "-C", str(manifest.parent), "rev-parse", "HEAD"], text=True).strip()
pathlib.Path(os.environ["RELEASE_TEST_CAPTURE"]).write_text(json.dumps(record))
print(os.environ.get("RELEASE_TEST_OUTPUT", '{"prs": []}'))
sys.exit(int(os.environ.get("RELEASE_TEST_EXIT_CODE", "0")))
''')
        mock.chmod(0o755)
        self.capture = self.root / "capture.json"
        self.env = {
            **os.environ,
            "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            "GITHUB_REF_NAME": "main",
            "GITHUB_REPOSITORY": "test/reinhardt-web",
            "GITHUB_TOKEN": "test-token",
            "RUNNER_TEMP": str(self.root),
            "RELEASE_TEST_CAPTURE": str(self.capture),
        }

    def run_git(self, *args):
        return subprocess.check_output(
            ["git", "-C", str(self.repo), *args], text=True, stderr=subprocess.PIPE
        ).strip()

    def write_version(self, version):
        (self.repo / "Cargo.toml").write_text(
            f'[package]\nname = "reinhardt-web"\nversion = "{version}"\nedition = "2024"\n'
        )

    def commit(self, message):
        self.run_git("add", ".")
        self.run_git("commit", "-qm", message)

    def invoke(self, *args, **env):
        return subprocess.run(
            ["bash", str(SCRIPT), *args], cwd=self.repo,
            env={**self.env, **env}, text=True, capture_output=True,
        )

    def assert_no_baseline_left(self):
        worktrees = self.run_git("worktree", "list", "--porcelain")
        self.assertEqual(worktrees.count("worktree "), 1)
        self.assertEqual(list(self.root.glob("reinhardt-release-baseline.*")), [])

    def test_main_uses_current_stable_tag_instead_of_newer_develop_tag(self):
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"prs": []})
        record = json.loads(self.capture.read_text())
        self.assertEqual(record["sha"], self.stable_commit)
        self.assertEqual(record["source"], "pub fn stable_api() {}\n")
        self.assertEqual(record["args"][0], "release-pr")
        self.assertFalse(Path(record["baseline"]).exists())
        self.assert_no_baseline_left()

    def test_failure_keeps_exit_code_and_removes_baseline(self):
        result = self.invoke(RELEASE_TEST_EXIT_CODE="7")
        self.assertEqual(result.returncode, 7, result.stderr)
        self.assertTrue(self.capture.exists())
        self.assert_no_baseline_left()

    def test_missing_stable_tag_fails_before_release_plz(self):
        self.run_git("tag", "-d", "reinhardt-web@v0.3.16")
        result = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr.splitlines()[-1], "::error::Missing release tag 'reinhardt-web@v0.3.16'.")
        self.assertFalse(self.capture.exists())
        self.assert_no_baseline_left()

    def test_unreachable_stable_tag_fails_before_release_plz(self):
        self.run_git("tag", "-f", "reinhardt-web@v0.3.16", "develop/0.4.0")
        result = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr.splitlines()[-1], "::error::Release tag 'reinhardt-web@v0.3.16' is not an ancestor of HEAD.")
        self.assertFalse(self.capture.exists())
        self.assert_no_baseline_left()

    def test_mislabeled_tag_fails_before_release_plz(self):
        self.write_version("0.3.15")
        self.commit("chore: incorrect version")
        self.run_git("tag", "-f", "reinhardt-web@v0.3.16")
        self.write_version("0.3.16")
        self.commit("chore: restore version")
        result = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr.splitlines()[-1], "::error::Release tag 'reinhardt-web@v0.3.16' contains 'reinhardt-web@0.3.15'.")
        self.assertFalse(self.capture.exists())
        self.assert_no_baseline_left()

    def test_main_rejects_prerelease_manifest(self):
        self.write_version("0.4.0-alpha.15")
        result = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr.splitlines()[-1], "::error::main requires a stable package version, found '0.4.0-alpha.15'.")
        self.assertFalse(self.capture.exists())

    def test_develop_uses_its_published_tag(self):
        self.run_git("checkout", "-q", "develop/0.4.0")
        result = self.invoke(GITHUB_REF_NAME="develop/0.4.0")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(self.capture.read_text())
        self.assertEqual(record["sha"], self.run_git("rev-parse", "reinhardt-web@v0.4.0-alpha.15"))
        self.assertEqual(record["source"], "pub fn develop_api() {}\n")
        self.assert_no_baseline_left()

    def test_develop_rejects_mismatched_release_line(self):
        self.run_git("checkout", "-q", "develop/0.4.0")
        result = self.invoke(GITHUB_REF_NAME="develop/0.5.0")
        self.assertEqual(result.returncode, 1)
        self.assertFalse(self.capture.exists())
        self.assert_no_baseline_left()

    def test_aws_baseline_pin_preserves_source_and_checkout(self):
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text() +
                            '\n[dependencies]\naws-config = { version = "1", optional = true }\n')
        self.commit("fix: AWS fixture")
        self.run_git("tag", "-f", "reinhardt-web@v0.3.16")
        original = manifest.read_text()
        result = self.invoke("update")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(self.capture.read_text())
        self.assertEqual(record["version"], original +
                         '\n[dependencies.aws-smithy-types]\nversion = "=1.6.3"\n')
        self.assertEqual(record["source"], "pub fn stable_api() { /* fixed */ }\n")
        self.assertEqual(manifest.read_text(), original)
        self.assert_no_baseline_left()

    def test_conflicting_baseline_constraint_stops_and_cleans_up(self):
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text() +
                            '\n[dependencies]\naws-config = "1"\naws-smithy-types = "=1.7.0"\n')
        self.commit("fix: incompatible AWS fixture")
        self.run_git("tag", "-f", "reinhardt-web@v0.3.16")
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.capture.exists())
        self.assert_no_baseline_left()

    def test_existing_constraint_is_preserved(self):
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text() +
                            '\n[dependencies]\naws-config = "1"\naws-smithy-types = "=1.6.3"\n')
        self.commit("fix: compatible AWS fixture")
        self.run_git("tag", "-f", "reinhardt-web@v0.3.16")
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(self.capture.read_text())
        self.assertEqual(record["version"], manifest.read_text())
        self.assert_no_baseline_left()

    def shell_baseline(self, extra="", version="0.22.0"):
        manifest = self.repo / "Cargo.toml"
        self.write_version("0.3.16")
        manifest.write_text(manifest.read_text() +
                            f'\n[dependencies]\nevcxr = {{ version = "{version}", optional = true }}\n' + extra)
        self.commit("test: shell baseline fixture")
        self.run_git("tag", "-f", "reinhardt-web@v0.3.16")
        return manifest

    def test_shell_baseline_constraints_preserve_source_and_checkout(self):
        manifest = self.shell_baseline('aws-config = "1"\n')
        original = manifest.read_text()
        result = self.invoke("update")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(self.capture.read_text())
        dependencies = {dep["name"]: dep for dep in record["dependencies"]}
        self.assertEqual(dependencies["unicode-ident"]["req"], "=1.0.24")
        self.assertEqual(dependencies["salsa"]["req"], "=0.28.2")
        self.assertFalse(dependencies["salsa"]["uses_default_features"])
        self.assertEqual(dependencies["salsa-macro-rules"]["req"], "=0.28.2")
        self.assertEqual(dependencies["aws-smithy-types"]["req"], "=1.6.3")
        self.assertEqual(record["source"], "pub fn stable_api() { /* fixed */ }\n")
        self.assertEqual(manifest.read_text(), original)
        self.assert_no_baseline_left()

    def test_shell_baseline_preparation_is_idempotent(self):
        manifest = self.shell_baseline()
        command = ["python3", str(SCRIPT.parent / "prepare-release-baseline.py"), str(self.repo)]
        subprocess.run(command, check=True, capture_output=True)
        prepared = manifest.read_text()
        subprocess.run(command, check=True, capture_output=True)
        self.assertEqual(manifest.read_text(), prepared)

    def test_conflicting_shell_constraints_stop_and_clean_up(self):
        for name, version in (("unicode-ident", "1.0.26"), ("salsa", "0.28.4"),
                              ("salsa-macro-rules", "0.28.4")):
            with self.subTest(dependency=name):
                manifest = self.shell_baseline(f'{name} = "={version}"\n')
                original = manifest.read_text()
                result = self.invoke()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f"Unexpected {name} compatibility constraint", result.stderr)
                self.assertFalse(self.capture.exists())
                self.assertEqual(manifest.read_text(), original)
                self.assert_no_baseline_left()

    def test_unaffected_evcxr_version_is_not_constrained(self):
        manifest = self.shell_baseline(version="0.23.0")
        result = self.invoke("update")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(self.capture.read_text())
        self.assertEqual(record["version"], manifest.read_text())
        self.assert_no_baseline_left()

    def test_local_update_uses_same_baseline_without_opening_pr(self):
        result = self.invoke("update", "--no-changelog", "--package", "reinhardt-web")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(self.capture.read_text())
        self.assertEqual(record["args"][0], "update")
        self.assertEqual(record["args"][-3:], ["--no-changelog", "--package", "reinhardt-web"])
        self.assertEqual(record["sha"], self.stable_commit)
        self.assert_no_baseline_left()

    def invoke_workflow(self, **env):
        (self.repo / "scripts").mkdir()
        shutil.copyfile(SCRIPT, self.repo / "scripts/run-release-pr.sh")
        shutil.copyfile(SCRIPT.parent / "prepare-release-baseline.py", self.repo / "scripts/prepare-release-baseline.py")
        step = subprocess.check_output([
            "ruby", "-ryaml", "-e",
            'print YAML.load_file(ARGV[0])["jobs"]["release-plz-pr"]["steps"].find { |s| s["id"] == "release-plz-pr" }.fetch("run")',
            str(WORKFLOW),
        ], text=True)
        self.outputs = self.root / "outputs"
        return subprocess.run(
            ["bash", "-c", step], cwd=self.repo, text=True, capture_output=True,
            env={**self.env, "GITHUB_OUTPUT": str(self.outputs), **env},
        )

    def test_workflow_preserves_valid_no_op_outputs(self):
        result = self.invoke_workflow()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.outputs.read_text(), "pr={}\nprs=[]\nprs_created=false\n")
        self.assert_no_baseline_left()

    def test_workflow_preserves_created_pr_metadata(self):
        pr = {"number": 1, "head_branch": "release-plz-test", "base_branch": "main"}
        result = self.invoke_workflow(RELEASE_TEST_OUTPUT=json.dumps({"prs": [pr]}))
        self.assertEqual(result.returncode, 0, result.stderr)
        outputs = dict(line.split("=", 1) for line in self.outputs.read_text().splitlines())
        self.assertEqual(json.loads(outputs["pr"]), pr)
        self.assertEqual(json.loads(outputs["prs"]), [pr])
        self.assertEqual(outputs["prs_created"], "true")

    def test_workflow_does_not_mask_cli_failure(self):
        result = self.invoke_workflow(RELEASE_TEST_EXIT_CODE="7")
        self.assertEqual(result.returncode, 7, result.stderr)
        self.assertFalse(self.outputs.exists())
        self.assert_no_baseline_left()

    def test_workflow_rejects_missing_or_malformed_pr_array(self):
        for output in ('{}', '{"prs": null}', '{"prs": {}}', 'invalid JSON'):
            with self.subTest(output=output):
                scripts = self.repo / "scripts"
                if scripts.exists():
                    shutil.rmtree(scripts)
                result = self.invoke_workflow(RELEASE_TEST_OUTPUT=output)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(self.outputs.exists())
                self.assert_no_baseline_left()


if __name__ == "__main__":
    unittest.main()
