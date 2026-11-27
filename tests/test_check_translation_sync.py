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

    def test_explicit_model_review_sends_evidence_without_advancing_baseline(self):
        before = (self.repo / "docs/translation-sync.json").read_bytes()
        with patch.object(sync, "review_prompt", return_value=('{}', False)) as review:
            result, out, err = self.run_script("review", "--pair", "guide", "--model", "openai:example")
        self.assertEqual((result, err), (0, ""))
        self.assertIn(self.head(), out)
        prompt, model, timeout = review.call_args.args
        self.assertIn("English initial", prompt)
        self.assertEqual((model, timeout), ("openai:example", 180))
        self.assertEqual(before, (self.repo / "docs/translation-sync.json").read_bytes())

    def test_incomplete_review_stops_before_next_pair_and_does_not_advance(self):
        before = (self.repo / "docs/translation-sync.json").read_bytes()
        with patch.object(sync, "review_prompt", return_value=('{"incomplete": true}', True)) as review:
            result, out, err = self.run_script("review", "--all", "--model", "github-copilot:example")
        self.assertEqual(result, 1)
        self.assertIn('"incomplete": true', out)
        self.assertIn("incomplete", err)
        self.assertEqual(review.call_count, 1)
        self.assertEqual(before, (self.repo / "docs/translation-sync.json").read_bytes())

    def test_review_requires_explicit_provider_and_model_and_valid_timeout(self):
        self.assertIn("--model is required", self.run_script("review", "--pair", "guide")[2])
        for model in ("example", "openai:", ":example", "openai:has spaces"):
            self.assertIn("--model must", self.run_script("review", "--pair", "guide", "--model", model)[2])
        for timeout in ("0", "-1", "nan", "inf"):
            self.assertIn("--timeout", self.run_script("review", "--pair", "guide", "--print-prompt",
                                                        "--timeout", timeout)[2])

    def test_model_names_allow_gateway_prefixes_and_colon_tags(self):
        for model in ("gateway/anthropic:example", "ollama:example:latest"):
            with self.subTest(model=model), patch.object(sync, "review_prompt", return_value=('{}', False)):
                result, _, err = self.run_script("review", "--pair", "guide", "--model", model)
                self.assertEqual((result, err), (0, ""))

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
        self.assertIn("Binary files", out)
        self.assertNotIn("GIT binary patch", out)
        self.assertIn("Full tracked content 'reverted.py' (changed in commits; no net diff)", out)

    def test_selective_evidence_includes_pair_and_sources_but_not_other_markdown(self):
        self.write("en.md", "old English\n")
        self.write("ja.md", "old Japanese\n")
        self.write("docs/other 日本.md", "PRIVATE OTHER ORIGINAL\n")
        self.write("docs/rename old.md", "OTHER RENAMED CONTENT\n")
        self.write("docs/deleted.MARKDOWN", "OTHER DELETED CONTENT\n")
        self.write("impl.py", "source header far from diff\n" + "padding\n" * 20 + "old source\n")
        self.write("old source.py", "renamed source content\n")
        self.git("add", ".")
        self.commit("prepare selective evidence")
        self.config["pairs"][0]["baseline"] = self.head()
        self.save_config()
        self.write("en.md", "new English\n")
        self.write("ja.md", "new Japanese\n")
        self.write("docs/other 日本.md", "PRIVATE OTHER UPDATED\n")
        self.git("mv", "docs/rename old.md", "docs/rename 新.md")
        self.git("rm", "docs/deleted.MARKDOWN")
        self.git("mv", "old source.py", "new source.py")
        self.write("impl.py", "source header far from diff\n" + "padding\n" * 20 + "new source\n")
        self.git("add", ".")
        self.commit("change selected and related files")
        result, out, err = self.run_script("review", "--pair", "guide", "--print-prompt")
        self.assertEqual((result, err), (0, ""))
        for expected in ("old English", "new English", "old Japanese", "new Japanese",
                         "-old English", "+new English", "-old Japanese", "+new Japanese",
                         "source header far from diff\n" + "padding\n" * 20 + "new source",
                         "-old source", "+new source", "renamed source content",
                         "docs/other 日本.md", "docs/rename 新.md", "docs/rename old.md",
                         "docs/deleted.MARKDOWN", "new source.py", "old source.py",
                         "omitted other Markdown", "inspect related docs", "change selected and related files"):
            self.assertIn(expected, out)
        for omitted in ("PRIVATE OTHER ORIGINAL", "PRIVATE OTHER UPDATED", "OTHER RENAMED CONTENT",
                        "OTHER DELETED CONTENT"):
            self.assertNotIn(omitted, out)

    def test_empty_review_range_has_no_unrestricted_diff(self):
        head = self.head()
        prompt = sync.make_prompt(self.repo, {"id": "same", "en": "en.md", "ja": "ja.md",
                                              "baseline": head}, head)
        self.assertIn("English initial", prompt)
        self.assertIn("Japanese initial", prompt)
        self.assertIn("[No changed paths.]", prompt)
        self.assertNotIn("diff --git", prompt)
        self.assertNotIn("old behavior", prompt)

    def test_only_other_markdown_changes_produce_no_diff(self):
        baseline = self.head()
        self.write("docs/unrelated.md", "OMITTED RELATED CONTENT\n")
        self.git("add", ".")
        self.commit("add unrelated documentation")
        prompt = sync.make_prompt(self.repo, {"id": "same", "en": "en.md", "ja": "ja.md",
                                              "baseline": baseline}, self.head())
        self.assertIn("docs/unrelated.md", prompt)
        self.assertIn("omitted other Markdown", prompt)
        self.assertNotIn("OMITTED RELATED CONTENT", prompt)
        self.assertNotIn("diff --git", prompt)

    def test_file_to_directory_change_excludes_nested_markdown_diff(self):
        self.write("node", "old executable\n")
        self.git("add", ".")
        self.commit("add node")
        baseline = self.head()
        (self.repo / "node").unlink()
        self.write("node/notes.MD", "NESTED OTHER MARKDOWN\n")
        self.write("node/code.py", "nested code\n")
        self.git("add", ".")
        self.commit("replace node with directory")
        prompt = sync.make_prompt(self.repo, {"id": "same", "en": "en.md", "ja": "ja.md",
                                              "baseline": baseline}, self.head())
        self.assertIn("node/notes.MD", prompt)
        self.assertNotIn("NESTED OTHER MARKDOWN", prompt)
        self.assertIn("-old executable", prompt)
        self.assertIn("+nested code", prompt)

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


class PydanticReviewTests(unittest.TestCase):
    def setUp(self):
        try:
            from pydantic_ai.models.function import FunctionModel
            from pydantic_ai.messages import ModelResponse, ToolCallPart
        except ImportError:
            self.skipTest("Pydantic AI not installed; run with the documented uv test command")
        self.FunctionModel, self.ModelResponse, self.ToolCallPart = FunctionModel, ModelResponse, ToolCallPart
        self.report = {"summary": "Review observations", "findings": [],
                       "incomplete": False, "incomplete_reason": ""}

    def model(self, report, finish_reason="tool_call"):
        def respond(messages, info):
            self.assertEqual(info.function_tools, [])
            self.assertEqual(len(info.output_tools), 1)
            self.assertIn("evidence", str(messages))
            return self.ModelResponse(
                parts=[self.ToolCallPart(info.output_tools[0].name, report)], finish_reason=finish_reason
            )
        return self.FunctionModel(respond)

    def test_structured_output_uses_no_function_tools(self):
        output, incomplete = sync.review_prompt("evidence", self.model(self.report), 5)
        self.assertFalse(incomplete)
        self.assertEqual(json.loads(output), self.report)

    def test_non_utf8_stdout_receives_ascii_json_without_losing_japanese(self):
        report = dict(self.report, summary="日本語の報告")
        buffer = io.BytesIO()
        stream = io.TextIOWrapper(buffer, encoding="ascii")
        with contextlib.redirect_stdout(stream):
            output, incomplete = sync.review_prompt("evidence", self.model(report), 5)
            print(output, flush=True)
        self.assertFalse(incomplete)
        self.assertEqual(json.loads(buffer.getvalue()), report)
        stream.close()

    def test_supported_providers_construct_without_network_or_real_credentials(self):
        from pydantic_ai.models import infer_model

        credentials = {"OPENAI_API_KEY": "test", "ANTHROPIC_API_KEY": "test",
                       "GOOGLE_API_KEY": "test", "GITHUB_COPILOT_API_KEY": "test"}
        with patch.dict(os.environ, credentials):
            for name in ("openai:example", "anthropic:example", "google:example", "github-copilot:example"):
                with self.subTest(name=name):
                    self.assertIsNotNone(infer_model(name))

    def test_agent_run_applies_one_request_limit(self):
        from pydantic_ai import Agent

        actual_run = Agent.run_sync
        calls = []

        def observe(agent, *args, **kwargs):
            calls.append(kwargs["usage_limits"].request_limit)
            return actual_run(agent, *args, **kwargs)

        with patch.object(Agent, "run_sync", observe):
            sync.review_prompt("evidence", self.model(self.report), 5)
        self.assertEqual(calls, [1])

    def test_incomplete_reason_or_flag_is_never_success(self):
        for change in ({"incomplete": True}, {"incomplete_reason": "Missing context"}):
            _, incomplete = sync.review_prompt("evidence", self.model({**self.report, **change}), 5)
            self.assertTrue(incomplete)

    def test_invalid_empty_truncated_and_filtered_responses_fail(self):
        cases = [(dict(self.report, summary="  "), "tool_call"),
                 ({"summary": "missing fields"}, "tool_call"),
                 (self.report, "length"), (self.report, "content_filter"), (self.report, "error")]
        for report, reason in cases:
            with self.subTest(report=report, reason=reason), self.assertRaises(sync.CheckError):
                sync.review_prompt("evidence", self.model(report, reason), 5)

    def test_total_timeout_interrupts_a_slow_model(self):
        import asyncio

        async def slow(messages, info):
            await asyncio.sleep(5)
        with self.assertRaises(sync.CheckError) as caught:
            sync.review_prompt("evidence", self.FunctionModel(slow), 0.01)
        self.assertIn("timed out", str(caught.exception))

    def test_provider_errors_and_timeout_are_incomplete_without_leaking_details(self):
        for error, expected in ((ValueError("secret credential"), "ValueError"),
                                (TimeoutError("secret credential"), "timed out")):
            def fail(messages, info):
                raise error
            with self.subTest(error=error), self.assertRaises(sync.CheckError) as caught:
                sync.review_prompt("evidence", self.FunctionModel(fail), 5)
            self.assertIn(expected, str(caught.exception))
            self.assertIn("incomplete", str(caught.exception))
            self.assertNotIn("secret credential", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
