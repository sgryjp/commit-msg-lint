import contextlib
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts import sync_config_schema as sync


class SyncConfigSchemaTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="schema hook ")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.git("init", "--quiet")
        self.path = self.root / sync.SCHEMA_PATH
        self.path.parent.mkdir()
        self.schema = b'{"title": "generated schema"}\n'

    def git(self, *args):
        return subprocess.run(
            ["git", "-c", "core.autocrlf=false", *args],
            cwd=self.root,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def stage_schema(self, content):
        self.path.write_bytes(content)
        self.git("add", "--", sync.SCHEMA_PATH.as_posix())

    def run_sync(self, output=None, exit_code=0):
        if output is None:
            output = self.schema
        program = (
            "import sys; "
            f"sys.stdout.buffer.write(bytes.fromhex({output.hex()!r})); "
            f"sys.exit({exit_code})"
        )
        stderr = io.StringIO()
        with patch.object(sync, "GENERATOR_COMMAND", [sys.executable, "-c", program]):
            with contextlib.redirect_stderr(stderr):
                result = sync.sync_schema(self.root)
        return result, stderr.getvalue()

    def test_generator_uses_the_locked_cargo_command(self):
        with patch.object(sync.subprocess, "run") as run:
            run.return_value.stdout = self.schema
            self.assertEqual(sync.generate_schema(self.root), self.schema)
        run.assert_called_once_with(
            ["cargo", "run", "--quiet", "--locked", "--", "--print-config-schema"],
            cwd=self.root,
            stdout=subprocess.PIPE,
            check=True,
        )

    def test_tracked_matching_schema_passes_without_replacing_it(self):
        self.stage_schema(self.schema)
        with patch.object(sync, "replace_schema") as replace:
            result, message = self.run_sync()
        self.assertEqual((result, message), (0, ""))
        replace.assert_not_called()
        self.assertEqual(self.path.read_bytes(), self.schema)

    def test_staged_outdated_schema_is_updated_and_requires_staging(self):
        self.stage_schema(b"outdated\n")
        result, message = self.run_sync()
        self.assertEqual(result, 1)
        self.assertIn("Review", message)
        self.assertIn("git add site/commit-msg-lint.schema.json", message)
        self.assertEqual(self.path.read_bytes(), self.schema)
        self.assertEqual(self.run_sync()[0], 1)
        self.git("add", "--", sync.SCHEMA_PATH.as_posix())
        self.assertEqual(self.run_sync()[0], 0)

    def test_staged_deletion_is_recreated_and_fails_until_added(self):
        self.stage_schema(self.schema)
        self.git(
            "-c", "user.name=Schema Test", "-c", "user.email=schema@example.invalid",
            "-c", f"core.hooksPath={self.root / 'no-hooks'}", "-c", "commit.gpgsign=false",
            "commit", "--quiet", "-m", "test schema",
        )
        self.git("rm", "--quiet", "--", sync.SCHEMA_PATH.as_posix())
        self.assertFalse(self.path.exists())
        self.assertIn(b"D", self.git("diff", "--cached", "--name-status").stdout)

        result, message = self.run_sync()
        self.assertEqual(result, 1)
        self.assertIn("git add site/commit-msg-lint.schema.json", message)
        self.assertEqual(self.path.read_bytes(), self.schema)
        self.assertEqual(self.run_sync()[0], 1)
        self.git("add", "--", sync.SCHEMA_PATH.as_posix())
        self.assertEqual(self.run_sync()[0], 0)

    def test_unstaged_deletion_is_recreated_and_fails_even_when_index_matches(self):
        self.stage_schema(self.schema)
        self.path.unlink()
        result, message = self.run_sync()
        self.assertEqual(result, 1)
        self.assertIn("Review", message)
        self.assertEqual(self.path.read_bytes(), self.schema)
        self.assertEqual(self.run_sync()[0], 0)

    def test_missing_schema_is_created_and_fails_until_added(self):
        result, message = self.run_sync()
        self.assertEqual(result, 1)
        self.assertIn("Review", message)
        self.assertEqual(self.path.read_bytes(), self.schema)
        self.assertEqual(self.run_sync()[0], 1)
        self.git("add", "--", sync.SCHEMA_PATH.as_posix())
        self.assertEqual(self.run_sync()[0], 0)

    def test_matching_untracked_schema_fails_until_added(self):
        self.path.write_bytes(self.schema)
        with patch.object(sync, "replace_schema") as replace:
            result, message = self.run_sync()
        self.assertEqual(result, 1)
        self.assertIn("untracked", message)
        self.assertIn("git add site/commit-msg-lint.schema.json", message)
        replace.assert_not_called()
        self.git("add", "--", sync.SCHEMA_PATH.as_posix())
        self.assertEqual(self.run_sync()[0], 0)

    def test_outdated_untracked_schema_is_updated_and_fails_until_added(self):
        self.path.write_bytes(b"outdated\n")
        result, message = self.run_sync()
        self.assertEqual(result, 1)
        self.assertIn("git add site/commit-msg-lint.schema.json", message)
        self.assertEqual(self.path.read_bytes(), self.schema)
        self.assertEqual(self.run_sync()[0], 1)
        self.git("add", "--", sync.SCHEMA_PATH.as_posix())
        self.assertEqual(self.run_sync()[0], 0)

    def test_generator_failure_preserves_existing_schema_even_after_partial_output(self):
        original = b"existing schema\n"
        self.stage_schema(original)
        result, message = self.run_sync(output=b"partial output", exit_code=2)
        self.assertEqual(result, 1)
        self.assertIn("Could not synchronize", message)
        self.assertEqual(self.path.read_bytes(), original)
        self.assertEqual(list(self.path.parent.iterdir()), [self.path])

    def test_generator_failure_does_not_create_a_missing_schema(self):
        result, message = self.run_sync(output=b"partial output", exit_code=2)
        self.assertEqual(result, 1)
        self.assertIn("Could not synchronize", message)
        self.assertFalse(self.path.exists())

    def test_failed_atomic_replacement_preserves_original_and_cleans_up(self):
        original = b"existing schema\n"
        self.stage_schema(original)
        with patch.object(sync.os, "replace", side_effect=OSError("replace failed")):
            result, message = self.run_sync()
        self.assertEqual(result, 1)
        self.assertIn("replace failed", message)
        self.assertEqual(self.path.read_bytes(), original)
        self.assertEqual(list(self.path.parent.iterdir()), [self.path])


if __name__ == "__main__":
    unittest.main()
