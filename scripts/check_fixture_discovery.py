#!/usr/bin/env python3
"""Verify Cargo discovers fixture-case additions and deletions incrementally."""

from __future__ import annotations

import os
import shutil
import subprocess
import uuid
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
CASES_ROOT = ROOT / "tests" / "cases"
LIST_COMMAND = ["test", "--locked", "--test", "fixtures", "--", "--list"]


def run_cargo(*arguments: str) -> str:
    environment = os.environ.copy()
    environment["CARGO_TERM_COLOR"] = "never"
    result = subprocess.run(
        ["cargo", *arguments],
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    output = result.stdout + result.stderr
    if result.returncode != 0:
        command = " ".join(["cargo", *arguments])
        raise RuntimeError(f"{command} failed with status {result.returncode}:\n{output}")
    return output


def listed_fixture_tests(output: str, identifier: str) -> list[str]:
    suffix = f"{identifier}: test"
    return [
        line[: -len(": test")]
        for line in output.splitlines()
        if line.endswith(suffix)
    ]


def main() -> None:
    case_name = f"automated-discovery-check-{uuid.uuid4().hex}"
    case_identifier = "case_automated_discovery_check_" + case_name.rsplit("-", 1)[1]
    case_directory = CASES_ROOT / case_name
    created = False

    try:
        case_directory.mkdir()
        created = True
        (case_directory / "case.toml").write_bytes(b"args = []\nexit_code = 0\n")
        (case_directory / "message.txt").write_bytes(b"feat: verify fixture discovery\n")
        (case_directory / "stdout.txt").write_bytes(b"")
        (case_directory / "stderr.txt").write_bytes(b"")

        discovered = listed_fixture_tests(run_cargo(*LIST_COMMAND), case_identifier)
        if len(discovered) != 1:
            raise RuntimeError(
                f"expected one generated test for {case_name}, found {discovered!r}"
            )
        test_name = discovered[0]

        run_output = run_cargo(
            "test",
            "--locked",
            "--test",
            "fixtures",
            "--",
            test_name,
            "--exact",
        )
        if f"test {test_name} ... ok" not in run_output:
            raise RuntimeError(
                f"the generated test {test_name!r} did not run successfully:\n{run_output}"
            )
        print(f"Added fixture was discovered and passed as {test_name}.")
    finally:
        if created:
            shutil.rmtree(case_directory)
            remaining = listed_fixture_tests(run_cargo(*LIST_COMMAND), case_identifier)
            if remaining:
                raise RuntimeError(
                    f"generated tests remained after removing {case_name}: {remaining!r}"
                )
            print(f"Removed fixture {case_name}; Cargo no longer lists its test.")


if __name__ == "__main__":
    main()
