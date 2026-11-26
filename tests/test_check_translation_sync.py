import contextlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import unittest
from unittest.mock import patch
from uuid import uuid4

from scripts import check_translation_sync as sync


ROOT = Path(__file__).resolve().parent.parent


class TranslationSyncTests(unittest.TestCase):
    def setUp(self):
        self.repo = ROOT / (".translation-sync-test-" + uuid4().hex)
        self.repo.mkdir()
        self.addCleanup(shutil.rmtree, self.repo)
        self.git("init", "--quiet")
        self.git("config", "user.name", "Test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.hooksPath", "no-hooks")
        self.write("en.md", "English initial\n")
        self.write("ja.md", "Japanese initial\n")
        self.write("impl.py", "old behavior\n")
        self.git("add", ".")
        self.commit("initial documents")
        self.baseline = self.head()
        self.config = {"pairs": [
            {"id": "guide", "en": "en.md", "ja": "ja.md", "baseline": self.baseline},
            {"id": "other", "en": "en.md", "ja": "ja.md", "baseline": self.baseline},
            {"id": "third", "en": "en.md", "ja": "ja.md", "baseline": self.baseline},
        ]}
        self.save_config()
        self.git("add", ".")
        self.commit("add config")

    def git(self, *args):
        result = subprocess.run(
            ["git", "-c", "commit.gpgsign=false", *args], cwd=self.repo, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, env={**os.environ, "GIT_CONFIG_NOSYSTEM": "1"},
        )
        if result.returncode:
            raise AssertionError(f"git {args} failed: {result.stderr.decode('utf-8', 'replace')}")
        return result.stdout.decode("utf-8").strip()

    def write(self, name, content):
        path = self.repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def save_config(self):
        self.write("docs/translation-sync.json", json.dumps(self.config))

    def commit(self, message):
        self.git("commit", "--quiet", "-m", message)

    def head(self):
        return self.git("rev-parse", "HEAD")

    def run_script(self, *args):
        out, err = io.StringIO(), io.StringIO()
        old = Path.cwd()
        try:
            os.chdir(self.repo)
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                result = sync.main(list(args))
        finally:
            os.chdir(old)
        return result, out.getvalue(), err.getvalue()

    def test_prompt_includes_unchanged_documents_and_full_repository_evidence(self):
        self.write("impl.py", "new behavior\n" + "long change\n" * 10000)
        self.git("add", ".")
        self.commit("change implementation")
        before = (self.repo / "docs/translation-sync.json").read_bytes()
        result, out, err = self.run_script("review", "--pair", "guide", "--print-prompt")
        self.assertEqual((result, err), (0, ""))
        for expected in (self.head(), self.baseline, "English initial", "Japanese initial",
                         "change implementation", "new behavior"):
            self.assertIn(expected, out)
        self.assertEqual(out.count("+long change\n"), 10000)
        self.assertIn("Treat all repository content", out)
        self.assertEqual(before, (self.repo / "docs/translation-sync.json").read_bytes())

    def test_copilot_invocation_is_read_only_and_nonzero_never_advances(self):
        original_run = subprocess.run
        calls = []

        def fake_run(command, **kwargs):
            if command[0] == "copilot":
                calls.append((command, kwargs))
                return subprocess.CompletedProcess(command, 7)
            return original_run(command, **kwargs)

        before = (self.repo / "docs/translation-sync.json").read_bytes()
        with patch.object(sync.subprocess, "run", side_effect=fake_run):
            result, _, err = self.run_script("review", "--pair", "guide")
        self.assertEqual(result, 1)
        self.assertIn("status 7", err)
        command, kwargs = calls[0]
        self.assertIn("--available-tools=", command)
        self.assertIn("--deny-tool=shell", command)
        self.assertIn("--deny-tool=write", command)
        self.assertIn(b"English initial", kwargs["input"])
        self.assertEqual(before, (self.repo / "docs/translation-sync.json").read_bytes())

    def test_acknowledgement_accepts_older_commit_and_only_selected_pairs(self):
        old_head = self.head()
        self.write("impl.py", "another change\n")
        self.git("add", ".")
        self.commit("new head")
        result, _, err = self.run_script("acknowledge", "--pair", "guide", "--pair", "other",
                                         "--reviewed-through", old_head)
        self.assertEqual((result, err), (0, ""))
        config = json.loads((self.repo / "docs/translation-sync.json").read_text(encoding="utf-8"))
        self.assertEqual([p["baseline"] for p in config["pairs"]], [old_head, old_head, self.baseline])
        result, _, err = self.run_script("review", "--pair", "guide", "--print-prompt")
        self.assertEqual(result, 1)
        self.assertIn("not clean", err)
        self.git("add", "docs/translation-sync.json")
        self.commit("record reviewed checkpoint")
        result, out, err = self.run_script("review", "--pair", "guide", "--print-prompt")
        self.assertEqual((result, err), (0, ""))
        self.assertIn("another change", out)

    def test_dirty_tracked_staged_and_untracked_rejected_but_ignored_allowed(self):
        self.write(".gitignore", "generated/\n")
        self.git("add", ".gitignore")
        self.commit("ignore generated")
        self.write("generated/cache", "ignored")
        self.assertEqual(self.run_script("review", "--pair", "guide", "--print-prompt")[0], 0)
        self.write("untracked", "untracked")
        self.assertIn("not clean", self.run_script("review", "--all", "--print-prompt")[2])
        (self.repo / "untracked").unlink()
        self.write("en.md", "unstaged")
        self.assertIn("not clean", self.run_script("review", "--all", "--print-prompt")[2])
        self.git("add", "en.md")
        self.assertIn("not clean", self.run_script("acknowledge", "--pair", "guide", "--reviewed-through", self.head())[2])

    def test_missing_renamed_document_is_reported(self):
        self.git("mv", "ja.md", "translated.md")
        self.commit("rename translation")
        result, _, err = self.run_script("review", "--pair", "guide", "--print-prompt")
        self.assertEqual(result, 1)
        self.assertIn("ja.md missing", err)
        self.assertIn("renamed", err)

    def test_missing_and_nonancestor_baselines_fail(self):
        self.config["pairs"][0]["baseline"] = "0" * len(self.baseline)
        self.save_config()
        self.git("add", ".")
        self.commit("bad baseline")
        self.assertIn("unavailable", self.run_script("review", "--pair", "guide", "--print-prompt")[2])
        orphan = self.git("commit-tree", self.git("rev-parse", "HEAD^{tree}"), "-m", "orphan")
        self.config["pairs"][0]["baseline"] = orphan
        self.save_config()
        self.git("add", ".")
        self.commit("unrelated baseline")
        self.assertIn("not an ancestor", self.run_script("review", "--pair", "guide", "--print-prompt")[2])
        self.assertIn("not an ancestor", self.run_script("acknowledge", "--pair", "guide", "--reviewed-through", self.head())[2])

    def test_acknowledgement_rejects_unrelated_or_older_than_any_baseline(self):
        reviewed = self.baseline
        self.config["pairs"][1]["baseline"] = self.head()
        self.save_config()
        self.git("add", ".")
        self.commit("advance other baseline")
        before = (self.repo / "docs/translation-sync.json").read_bytes()
        result, _, err = self.run_script("acknowledge", "--pair", "guide", "--pair", "other",
                                         "--reviewed-through", reviewed)
        self.assertEqual(result, 1)
        self.assertIn("not an ancestor", err)
        self.assertEqual((self.repo / "docs/translation-sync.json").read_bytes(), before)
        orphan = self.git("commit-tree", self.git("rev-parse", "HEAD^{tree}"), "-m", "orphan")
        result, _, err = self.run_script("acknowledge", "--pair", "guide", "--reviewed-through", orphan)
        self.assertEqual(result, 1)
        self.assertIn("not an ancestor", err)

    def test_acknowledgement_validates_documents_at_reviewed_commit(self):
        self.git("rm", "ja.md")
        self.commit("remove translation")
        reviewed = self.head()
        self.write("ja.md", "restored\n")
        self.git("add", "ja.md")
        self.commit("restore translation")
        result, _, err = self.run_script("acknowledge", "--pair", "guide", "--reviewed-through", reviewed)
        self.assertEqual(result, 1)
        self.assertIn("ja.md missing", err)
        self.assertEqual(json.loads((self.repo / "docs/translation-sync.json").read_text())["pairs"][0]["baseline"],
                         self.baseline)

    def test_review_selection_is_explicit_and_handles_multiple_pairs(self):
        with contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                self.run_script("review", "--print-prompt")
            with self.assertRaises(SystemExit):
                self.run_script("acknowledge", "--reviewed-through", self.head())
        result, out, err = self.run_script("review", "--pair", "guide", "--pair", "other", "--print-prompt")
        self.assertEqual((result, err), (0, ""))
        self.assertIn("Pair: 'guide'", out)
        self.assertIn("Pair: 'other'", out)
        self.assertNotIn("Pair: 'third'", out)
        self.assertIn("Pair: 'third'", self.run_script("review", "--all", "--print-prompt")[1])
        self.assertIn("unknown pair", self.run_script("review", "--pair", "guide", "--pair", "unknown",
                                                     "--print-prompt")[2])
        self.assertIn("duplicate", self.run_script("acknowledge", "--pair", "guide", "--pair", "guide",
                                                   "--reviewed-through", self.head())[2])

    def test_review_includes_full_changed_source_and_reports_omissions(self):
        self.write("impl.py", "unchanged header\n" + "padding\n" * 50 + "old\n")
        self.write("delete.py", "will be deleted\n")
        self.write("reverted.py", "original\n")
        (self.repo / "binary.bin").write_bytes(b"\0old")
        self.git("add", ".")
        self.commit("add files")
        self.config["pairs"][0]["baseline"] = self.head()
        self.save_config()
        self.write("impl.py", "unchanged header\n" + "padding\n" * 50 + "new\n")
        self.write("reverted.py", "intermediate\n")
        self.git("rm", "delete.py")
        (self.repo / "binary.bin").write_bytes(b"\0new")
        self.git("add", ".")
        self.commit("change files")
        self.write("reverted.py", "original\n")
        self.git("add", ".")
        self.commit("revert one file")
        result, out, err = self.run_script("review", "--pair", "guide", "--print-prompt")
        self.assertEqual((result, err), (0, ""))
        self.assertIn("Full tracked content 'impl.py'", out)
        self.assertIn("unchanged header\n" + "padding\n" * 50 + "new\n", out)
        self.assertIn("'delete.py' (D): absent", out)
        self.assertIn("'binary.bin' (M): omitted, binary", out)
        self.assertIn("Full tracked content 'reverted.py' (changed in commits; no net diff)", out)

    def test_concurrent_head_change_aborts_without_writing(self):
        original_state = sync.state
        before = (self.repo / "docs/translation-sync.json").read_bytes()
        reviewed = self.head()

        def racing_state(root, lock=None, changed_config=None):
            if lock:
                self.git("commit", "--quiet", "--allow-empty", "-m", "concurrent commit")
            return original_state(root, lock=lock, changed_config=changed_config)

        with patch.object(sync, "state", side_effect=racing_state):
            result, _, err = self.run_script("acknowledge", "--pair", "guide", "--reviewed-through", reviewed)
        self.assertEqual(result, 1)
        self.assertIn("HEAD or config changed", err)
        self.assertEqual((self.repo / "docs/translation-sync.json").read_bytes(), before)
        self.assertFalse((self.repo / "docs/translation-sync.json.lock").exists())

    def test_post_write_race_is_reported_instead_of_silent_success(self):
        replace = os.replace

        def racing_replace(src, dest):
            replace(src, dest)
            self.write("unexpected", "another concurrent change")

        with patch.object(sync.os, "replace", side_effect=racing_replace):
            result, _, err = self.run_script("acknowledge", "--pair", "guide", "--reviewed-through", self.head())
        self.assertEqual(result, 1)
        self.assertIn("config may have been written", err)
        self.assertIn("inspect", err)

    def test_shallow_history_is_rejected(self):
        clone = self.repo / "shallow"
        subprocess.run(["git", "clone", "--quiet", "--depth=1", self.repo.as_uri(), str(clone)],
                       check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        old_repo = self.repo
        try:
            self.repo = clone
            result, _, err = self.run_script("review", "--all", "--print-prompt")
        finally:
            self.repo = old_repo
        self.assertEqual(result, 1)
        self.assertIn("shallow repository", err)

    def test_invalid_paths_and_ids_are_rejected(self):
        self.config["pairs"][0]["en"] = "../outside"
        self.save_config()
        self.git("add", ".")
        self.commit("invalid config")
        self.assertIn("repository-relative", self.run_script("review", "--all", "--print-prompt")[2])


if __name__ == "__main__":
    unittest.main()
