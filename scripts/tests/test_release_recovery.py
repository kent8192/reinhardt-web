"""Exercise partial publication and recovery with real Git/Cargo fixtures."""

from contextlib import redirect_stderr, redirect_stdout
from copy import deepcopy
from datetime import datetime, timezone
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest
from urllib.error import HTTPError

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from release_state import (
    ReleaseError,
    audit,
    packages,
    previous_release_packages,
    registry_version,
)
from run_release_publish import publish, retry_delay, run_attempt
from verify_release_source import verify


class RegistryTests(unittest.TestCase):
    def opener(self, entries):
        return lambda *_args, **_kwargs: io.BytesIO(
            "\n".join(json.dumps(entry) for entry in entries).encode()
        )

    def test_exact_version_and_yank_status(self):
        entry = {"name": "crate-a", "vers": "0.4.0-alpha.16", "yanked": False}
        self.assertFalse(
            registry_version("crate-a", "0.4.0-alpha.17", self.opener([entry]))
        )
        entry["vers"] = "0.4.0-alpha.17"
        self.assertTrue(
            registry_version("crate-a", entry["vers"], self.opener([entry]))
        )
        entry["yanked"] = True
        with self.assertRaisesRegex(ReleaseError, "yanked"):
            registry_version("crate-a", entry["vers"], self.opener([entry]))

    def test_only_404_means_unpublished(self):
        for code in (404, 403, 429, 503):

            def opener(*_args, **_kwargs):
                raise HTTPError(
                    "https://index.crates.io/test", code, "fixture", None, None
                )

            with self.subTest(code=code):
                if code == 404:
                    self.assertFalse(registry_version("crate-a", "1.0.0", opener))
                else:
                    with self.assertRaisesRegex(ReleaseError, f"HTTP {code}"):
                        registry_version("crate-a", "1.0.0", opener)

    def test_malformed_registry_response_is_not_unpublished(self):
        for data in (
            [],
            [{"name": "different", "vers": "1.0.0"}],
            [{"name": "crate-a"}],
        ):
            with self.subTest(data=data), self.assertRaisesRegex(
                ReleaseError, "Malformed"
            ):
                registry_version("crate-a", "1.0.0", self.opener(data))


class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(
            prefix="release-recovery-test-", dir="/tmp"
        )
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.git("init", "-q", "-b", "develop/0.4.0")
        self.git("config", "user.name", "Release Test")
        self.git("config", "user.email", "release-test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "tag.gpgsign", "false")
        (self.repo / "Cargo.toml").write_text(
            '[package]\nname = "reinhardt-web"\nversion = "0.4.0-alpha.17"\nedition = "2024"\n'
            '[workspace]\nmembers = ["crates/*"]\nresolver = "3"\n'
        )
        (self.repo / "src").mkdir()
        (self.repo / "src/lib.rs").write_text("pub fn facade() {}\n")
        (self.repo / "release-plz.toml").write_text(
            "[workspace]\ngit_release_enable = false\n"
            'git_tag_name = "{{ package }}@v{{ version }}"\n'
            '[[package]]\nname = "reinhardt-web"\ngit_release_enable = true\n'
        )
        for name in ("core", "auth"):
            root = self.repo / "crates" / name
            (root / "src").mkdir(parents=True)
            (root / "Cargo.toml").write_text(
                f'[package]\nname = "reinhardt-{name}"\nversion = "0.4.0-alpha.17"\nedition = "2024"\n'
            )
            (root / "src/lib.rs").write_text(f"pub fn {name}() {{}}\n")
        self.original = self.commit("chore: release")
        self.git("tag", "reinhardt-core@v0.4.0-alpha.17")
        with (self.repo / "crates/auth/Cargo.toml").open("a") as manifest:
            manifest.write("# Correct local test packaging\n")
        self.source = self.commit("fix(auth): repair packaging")
        self.workflow = self.source
        self.inventory = packages(self.repo)
        self.pr = {
            "number": 42,
            "merged": True,
            "state": "closed",
            "merge_commit_sha": self.original,
            "base": {
                "ref": "develop/0.4.0",
                "repo": {"full_name": "test/reinhardt-web"},
            },
            "head": {
                "ref": "develop-release-plz-fixture",
                "repo": {"full_name": "test/reinhardt-web"},
            },
            "labels": [{"name": "release"}],
        }

    def git(self, *args):
        return subprocess.check_output(
            ["git", *args], cwd=self.repo, text=True, stderr=subprocess.PIPE
        ).strip()

    def commit(self, message):
        self.git("add", ".")
        self.git("commit", "-qm", message)
        return self.git("rev-parse", "HEAD")

    def state(self, published=("reinhardt-core",), release=False):
        return audit(
            self.repo,
            "test/reinhardt-web",
            self.inventory,
            registry=lambda name, _version: name in published,
            release=lambda *_args: release,
        )

    def check(self, **kwargs):
        args = dict(
            repo=self.repo,
            repository="test/reinhardt-web",
            base="develop/0.4.0",
            workflow_sha=self.workflow,
            source_sha=self.source,
            pr=self.pr,
            resume=True,
            inventory=self.inventory,
            state=self.state(),
        )
        args.update(kwargs)
        return verify(**args)

    def test_partial_release_reports_each_missing_artifact(self):
        result = self.state()
        self.assertEqual(result["state"], "incomplete")
        self.assertEqual(
            result["unpublished"],
            ["reinhardt-auth@0.4.0-alpha.17", "reinhardt-web@0.4.0-alpha.17"],
        )
        self.assertEqual(
            result["missing_tags"],
            ["reinhardt-auth@v0.4.0-alpha.17", "reinhardt-web@v0.4.0-alpha.17"],
        )
        self.assertEqual(result["missing_releases"], ["reinhardt-web@v0.4.0-alpha.17"])

    def test_registry_tag_and_github_release_are_all_required(self):
        names = [item["name"] for item in self.inventory]
        self.assertEqual(self.state(names, True)["state"], "incomplete")
        for item in self.inventory:
            if item["name"] != "reinhardt-core":
                self.git("tag", item["tag"])
        self.assertEqual(self.state(names, False)["state"], "incomplete")
        self.assertEqual(self.state(names, True)["state"], "complete")

    def test_mislabeled_tag_is_rejected(self):
        manifest = self.repo / "crates/core/Cargo.toml"
        manifest.write_text(manifest.read_text().replace("alpha.17", "alpha.18"))
        self.commit("fixture: wrong version tag")
        self.git("tag", "-f", "reinhardt-core@v0.4.0-alpha.17")
        manifest.write_text(manifest.read_text().replace("alpha.18", "alpha.17"))
        self.commit("fixture: restore version")
        with self.assertRaisesRegex(ReleaseError, "different package/version"):
            self.state()

    def test_verified_recovery_is_accepted(self):
        result = self.check()
        self.assertEqual(
            (result["mode"], result["source_sha"], result["release_pr"]),
            ("resume-release", self.source, 42),
        )

    def test_repeated_recovery_accepts_the_already_published_repair(self):
        self.git("tag", "reinhardt-auth@v0.4.0-alpha.17")
        state = self.state(published=("reinhardt-core", "reinhardt-auth"))
        self.assertEqual(self.check(state=state)["mode"], "resume-release")
        (self.repo / "crates/auth/src/lib.rs").write_text("pub fn changed_again() {}\n")
        self.source = self.workflow = self.commit(
            "fixture: changed after recovery publication"
        )
        with self.assertRaisesRegex(
            ReleaseError, "already-published package reinhardt-auth"
        ):
            self.check(state=self.state(published=("reinhardt-core", "reinhardt-auth")))

    def test_dirty_source_is_rejected_before_publication(self):
        (self.repo / "crates/auth/src/lib.rs").write_text("pub fn uncommitted() {}\n")
        with self.assertRaisesRegex(ReleaseError, "uncommitted"):
            self.check()

    def test_complete_release_is_not_resumed(self):
        self.git("tag", "reinhardt-auth@v0.4.0-alpha.17")
        self.git("tag", "reinhardt-web@v0.4.0-alpha.17")
        state = self.state(
            published=tuple(item["name"] for item in self.inventory), release=True
        )
        with self.assertRaisesRegex(ReleaseError, "already complete"):
            self.check(state=state)

    def test_removed_package_membership_is_rejected(self):
        self.git("rm", "-r", "crates/auth")
        self.source = self.workflow = self.commit("fixture: removed package")
        self.inventory = packages(self.repo)
        with self.assertRaisesRegex(ReleaseError, "package membership"):
            self.check()

    def test_new_packages_do_not_block_the_next_release_pr(self):
        self.git("tag", "reinhardt-auth@v0.4.0-alpha.17")
        self.git("tag", "reinhardt-web@v0.4.0-alpha.17")
        root = self.repo / "crates/new"
        (root / "src").mkdir(parents=True)
        (root / "Cargo.toml").write_text(
            '[package]\nname = "reinhardt-new"\nversion = "0.1.0"\nedition = "2024"\n'
        )
        (root / "src/lib.rs").write_text("pub fn new_package() {}\n")
        inventory = previous_release_packages(self.repo)
        self.assertNotIn("reinhardt-new", [item["name"] for item in inventory])
        state = audit(
            self.repo,
            "test/reinhardt-web",
            inventory,
            registry=lambda name, _version: name != "reinhardt-new",
            release=lambda *_args: True,
        )
        self.assertEqual(state["state"], "complete")
        self.assertIn("reinhardt-new", [item["name"] for item in packages(self.repo)])

    def test_ordinary_repair_merge_cannot_publish(self):
        with self.assertRaisesRegex(ReleaseError, "normal release"):
            self.check(resume=False)
        self.git("checkout", "--detach", self.original)
        result = self.check(
            resume=False, source_sha=self.original, workflow_sha=self.original
        )
        self.assertEqual(result["mode"], "release")

    def test_wrong_pr_metadata_is_rejected(self):
        for mutation in ("unmerged", "open", "label", "prefix", "base", "fork"):
            pr = deepcopy(self.pr)
            if mutation == "unmerged":
                pr["merged"] = False
            if mutation == "open":
                pr["state"] = "open"
            if mutation == "label":
                pr["labels"] = []
            if mutation == "prefix":
                pr["head"]["ref"] = "release-plz-stable"
            if mutation == "base":
                pr["base"]["ref"] = "main"
            if mutation == "fork":
                pr["head"]["repo"]["full_name"] = "fork/reinhardt-web"
            with self.subTest(mutation=mutation), self.assertRaisesRegex(
                ReleaseError, "merged, release-labelled"
            ):
                self.check(pr=pr)

    def test_sha_and_release_line_boundaries(self):
        for args, expected in (
            ({"source_sha": "develop/0.4.0"}, "full 40"),
            ({"source_sha": self.original}, "Checked-out"),
            ({"base": "feature/test"}, "requires main"),
            ({"base": "develop/0.5.0"}, "Source version"),
            ({"workflow_sha": self.original}, "already be integrated"),
        ):
            with self.subTest(args=args), self.assertRaisesRegex(
                ReleaseError, expected
            ):
                self.check(**args)

    def test_original_release_must_be_in_source_history(self):
        self.git("checkout", "-q", "-b", "unrelated", self.original)
        (self.repo / "unrelated.txt").write_text("another branch")
        different = self.commit("fixture: unrelated")
        self.git("checkout", "--detach", self.source)
        pr = deepcopy(self.pr)
        pr["merge_commit_sha"] = different
        with self.assertRaisesRegex(ReleaseError, "contain the original"):
            self.check(pr=pr)

    def test_published_package_changes_are_rejected(self):
        (self.repo / "crates/core/src/lib.rs").write_text("pub fn changed() {}\n")
        self.source = self.workflow = self.commit("fixture: changed published source")
        with self.assertRaisesRegex(
            ReleaseError, "already-published package reinhardt-core"
        ):
            self.check()

    def test_shared_manifest_changes_are_rejected(self):
        with (self.repo / "Cargo.toml").open("a") as manifest:
            manifest.write("\n[workspace.dependencies]\nserde = '1'\n")
        self.source = self.workflow = self.commit(
            "fixture: changed shared dependencies"
        )
        with self.assertRaisesRegex(ReleaseError, "root manifest"):
            self.check()

    def test_package_version_change_is_rejected(self):
        manifest = self.repo / "crates/auth/Cargo.toml"
        manifest.write_text(manifest.read_text().replace("alpha.17", "alpha.18"))
        self.source = self.workflow = self.commit("fixture: changed package version")
        self.inventory = packages(self.repo)
        with self.assertRaisesRegex(ReleaseError, "changed the release version"):
            self.check()

    def test_newer_release_on_selected_branch_is_rejected(self):
        manifest = self.repo / "crates/auth/Cargo.toml"
        manifest.write_text(manifest.read_text().replace("alpha.17", "alpha.18"))
        self.workflow = self.commit("fixture: a later release")
        self.git("checkout", "--detach", self.source)
        with self.assertRaisesRegex(ReleaseError, "advanced past"):
            self.check()

    def test_disabled_packages_are_not_release_prerequisites(self):
        with (self.repo / "release-plz.toml").open("a") as config:
            config.write('[[package]]\nname = "reinhardt-auth"\nrelease = false\n')
        self.assertEqual(
            [item["name"] for item in packages(self.repo)],
            ["reinhardt-core", "reinhardt-web"],
        )


class PublishTests(unittest.TestCase):
    def setUp(self):
        # Expected failures must not emit GitHub error annotations from passing tests.
        self.output = io.StringIO()
        self.errors = io.StringIO()
        self.enterContext(redirect_stdout(self.output))
        self.enterContext(redirect_stderr(self.errors))

    def test_dependency_failure_does_not_wait_or_retry(self):
        calls, waits = [], []

        def runner(*args):
            calls.append(args)
            return (
                101,
                'failed to select a version for the requirement `reinhardt-urls = "^0.4.0-alpha.17"`',
            )

        self.assertEqual(
            publish(["release-plz"], Path("/tmp"), runner, waits.append), 101
        )
        self.assertEqual((len(calls), waits), (1, []))

    def test_rate_limit_then_dependency_failure_stops_on_second_attempt(self):
        outcomes = iter(
            [
                (1, "status 429 Too Many Requests\nRetry-After: 8"),
                (101, "failed to select a version"),
            ]
        )
        waits = []
        result = publish([], Path("/tmp"), lambda *_args: next(outcomes), waits.append)
        self.assertEqual((result, waits), (101, [1200]))

    def test_transient_failures_retry_to_success_and_have_a_limit(self):
        for final in (0, 2):
            outcomes = iter(
                [(1, "HTTP 503"), (1, "connection reset"), (final, "HTTP 503")]
            )
            waits = []
            result = publish(
                [], Path("/tmp"), lambda *_args: next(outcomes), waits.append
            )
            self.assertEqual((result, waits), (final, [30, 60]))

    def test_crates_io_reset_deadline_and_unknown_errors(self):
        now = datetime(2026, 9, 26, 4, 10, 53, tzinfo=timezone.utc)
        message = "status 429 Too Many Requests: Please try again after Sat, 26 Sep 2026 04:11:28 GMT"
        self.assertEqual(retry_delay(message, 1, now), 1200)
        self.assertEqual(retry_delay("HTTP 429\nRetry-After: 1900", 1), 1905)
        self.assertEqual(retry_delay("HTTP 429", 2), 1800)
        self.assertIsNone(retry_delay("unclassified publication error", 1))
        self.assertIsNone(retry_delay("HTTP 429 then failed to select a version", 1))
        self.assertIsNone(retry_delay("status 403", 1))
        self.assertIsNone(retry_delay("failed at source line 429", 1))

    def test_child_exit_and_diagnostics_are_preserved(self):
        code, log = run_attempt(
            [sys.executable, "-c", "import sys; print('package error'); sys.exit(7)"],
            Path("/tmp"),
        )
        self.assertEqual((code, log), (7, "package error\n"))
        self.assertEqual(self.output.getvalue(), "package error\n")


class WorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        workflow = (
            Path(__file__).resolve().parents[2] / ".github/workflows/release-plz.yml"
        )
        cls.jobs = json.loads(
            subprocess.check_output(
                [
                    "ruby",
                    "-ryaml",
                    "-rjson",
                    "-e",
                    'puts JSON.generate(YAML.load_file(ARGV[0])["jobs"])',
                    str(workflow),
                ],
                text=True,
            )
        )

    def condition(
        self,
        job,
        event,
        mode="",
        classified="false",
        classification="skipped",
        result="skipped",
        released="false",
        cancelled=False,
    ):
        values = {
            "github.event_name": event,
            "inputs.mode": mode,
            "needs.classify-release-push.result": classification,
            "needs.classify-release-push.outputs.is_release_merge": classified,
            "needs.release-plz-release.result": result,
            "needs.release-plz-release.outputs.released": released,
            "github.ref": "refs/heads/develop/0.4.0",
        }
        expression = self.jobs[job]["if"].strip()
        for key in sorted(values, key=len, reverse=True):
            expression = expression.replace(key, repr(values[key]))
        expression = expression.replace("always()", "True").replace(
            "!cancelled()", repr(not cancelled)
        )
        expression = expression.replace("&&", " and ").replace("||", " or ")
        expression = re.sub(
            r"startsWith\(([^,]+), ([^)]+)\)", r"str.startswith(\1, \2)", expression
        )
        return eval(" ".join(expression.split()), {"__builtins__": {}, "str": str})

    def test_publish_gate_accepts_only_verified_push_or_explicit_recovery(self):
        job = "release-plz-release"
        self.assertFalse(self.condition(job, "push", classification="success"))
        self.assertTrue(
            self.condition(job, "push", classification="success", classified="true")
        )
        self.assertFalse(
            self.condition(job, "push", classification="failure", classified="true")
        )
        self.assertTrue(self.condition(job, "workflow_dispatch", "resume-release"))
        for mode in ("release", "backfill", ""):
            self.assertFalse(self.condition(job, "workflow_dispatch", mode))
        self.assertFalse(
            self.condition(job, "workflow_dispatch", "resume-release", cancelled=True)
        )

    def test_failed_or_incomplete_recovery_cannot_announce(self):
        for result in ("failure", "cancelled", "skipped"):
            self.assertFalse(
                self.condition(
                    "release-announcement-pr",
                    "workflow_dispatch",
                    "resume-release",
                    result=result,
                    released="true",
                )
            )
        self.assertTrue(
            self.condition(
                "release-announcement-pr",
                "workflow_dispatch",
                "resume-release",
                result="success",
                released="true",
            )
        )
        self.assertFalse(
            self.condition(
                "release-announcement-pr",
                "workflow_dispatch",
                "resume-release",
                result="success",
            )
        )

    def test_release_jobs_provision_supported_python_before_scripts(self):
        for name in ("release-plz-pr", "release-plz-release"):
            with self.subTest(job=name):
                steps = self.jobs[name]["steps"]
                setup = next(
                    i
                    for i, step in enumerate(steps)
                    if step.get("uses", "").startswith("actions/setup-python@")
                )
                self.assertEqual(steps[setup]["with"]["python-version"], "3.12")
                first_python = next(
                    i
                    for i, step in enumerate(steps)
                    if "python3 scripts/" in step.get("run", "")
                )
                self.assertLess(setup, first_python)


if __name__ == "__main__":
    unittest.main()
