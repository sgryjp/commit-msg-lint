#!/usr/bin/env python3
"""Exercise the published hook through pre-commit in an isolated Git repository."""

import os
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
HOOK_MANIFEST = ".pre-commit-hooks.yaml"


def run(command: list[str], cwd: Path, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise AssertionError(
            f"command failed ({result.returncode}): {command!r}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def initialize_repository(path: Path, env: dict[str, str]) -> None:
    path.mkdir(parents=True, exist_ok=True)
    run(["git", "init", "--quiet", str(path)], ROOT, env)
    run(["git", "config", "user.name", "Hook Test"], path, env)
    run(["git", "config", "user.email", "hook-test@example.invalid"], path, env)


def make_hook_source(path: Path, env: dict[str, str]) -> str:
    path.mkdir()
    tracked = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT,
        env=env,
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    paths = {Path(item.decode()) for item in tracked if item}
    paths.add(Path(HOOK_MANIFEST))
    for relative in paths:
        source = ROOT / relative
        if not source.is_file():
            continue
        destination = path / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)

    initialize_repository(path, env)
    run(["git", "add", "--all"], path, env)
    run(["git", "commit", "--quiet", "-m", "test hook source"], path, env)
    return run(["git", "rev-parse", "HEAD"], path, env).stdout.strip()


def write_consumer_config(path: Path, source: Path, revision: str, strict: bool) -> None:
    args = "        args: [--strict, --edit]\n" if strict else ""
    (path / ".pre-commit-config.yaml").write_text(
        "repos:\n"
        f"  - repo: {source.as_uri()}\n"
        f"    rev: {revision}\n"
        "    hooks:\n"
        "      - id: commit-msg-lint\n"
        f"{args}",
        encoding="utf-8",
    )


def commit_with_message(
    repo: Path, message: str, env: dict[str, str], index: int
) -> subprocess.CompletedProcess[str]:
    (repo / "tracked.txt").write_text(f"change {index}\n", encoding="utf-8")
    run(["git", "add", "tracked.txt"], repo, env)
    return subprocess.run(
        ["git", "commit", "--quiet", "-m", message],
        cwd=repo,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )


def verify_valid_commit_is_accepted(repo: Path, env: dict[str, str]) -> None:
    result = commit_with_message(repo, "feat: add hook integration", env, 1)
    if result.returncode:
        raise AssertionError(f"valid message was rejected:\n{result.stderr}")


def verify_invalid_commit_is_rejected(repo: Path, env: dict[str, str]) -> None:
    result = commit_with_message(repo, "feat:subject", env, 2)
    output = result.stdout + result.stderr
    if result.returncode == 0 or "error[header-format]" not in output:
        raise AssertionError(
            "invalid message was not rejected by the installed commit-msg hook:\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )


def verify_warnings_pass_without_strict_mode(repo: Path, env: dict[str, str]) -> None:
    warning_message = "feat: " + "x" * 45
    result = commit_with_message(repo, warning_message, env, 3)
    output = result.stdout + result.stderr
    if result.returncode or "warning[header-max-width]" not in output:
        raise AssertionError(
            "warning should be visible while the commit succeeds:\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )


def verify_strict_mode_rejects_warnings(repo: Path, env: dict[str, str]) -> None:
    warning_message = "feat: " + "x" * 45
    result = commit_with_message(repo, warning_message, env, 4)
    output = result.stdout + result.stderr
    if result.returncode == 0 or "warning[header-max-width]" not in output:
        raise AssertionError(
            "strict args did not reject a warning:\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )


def main() -> None:
    pre_commit = shutil.which("pre-commit")
    if pre_commit is None:
        raise SystemExit(
            "pre-commit is required; install it with `python -m pip install pre-commit`"
        )

    with tempfile.TemporaryDirectory(prefix="commit-msg-lint-pre-commit-") as temporary:
        temporary_root = Path(temporary)
        env = os.environ.copy()
        for key in (
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_COMMON_DIR",
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
        ):
            env.pop(key, None)
        for key in tuple(env):
            if key.startswith(("GIT_CONFIG_KEY_", "GIT_CONFIG_VALUE_")):
                env.pop(key)
        env.update(
            {
                "PRE_COMMIT_HOME": str(temporary_root / "pre-commit-cache"),
                "GIT_CONFIG_GLOBAL": str(temporary_root / "gitconfig"),
                "GIT_CONFIG_NOSYSTEM": "1",
                "GIT_TERMINAL_PROMPT": "0",
                "GIT_AUTHOR_NAME": "Hook Test",
                "GIT_AUTHOR_EMAIL": "hook-test@example.invalid",
                "GIT_COMMITTER_NAME": "Hook Test",
                "GIT_COMMITTER_EMAIL": "hook-test@example.invalid",
            }
        )
        source = temporary_root / "hook-source"
        revision = make_hook_source(source, env)
        consumer = temporary_root / "consumer"
        initialize_repository(consumer, env)
        write_consumer_config(consumer, source, revision, strict=False)
        (consumer / "tracked.txt").write_text("initial\n", encoding="utf-8")
        run(["git", "add", "--all"], consumer, env)
        run(["git", "commit", "--quiet", "-m", "chore: initialize test repo"], consumer, env)
        run([pre_commit, "install", "--hook-type", "commit-msg"], consumer, env)

        verify_valid_commit_is_accepted(consumer, env)
        verify_invalid_commit_is_rejected(consumer, env)
        verify_warnings_pass_without_strict_mode(consumer, env)
        write_consumer_config(consumer, source, revision, strict=True)
        run(["git", "add", ".pre-commit-config.yaml"], consumer, env)
        verify_strict_mode_rejects_warnings(consumer, env)

    print("pre-commit commit-msg hook integration passed")


if __name__ == "__main__":
    main()
