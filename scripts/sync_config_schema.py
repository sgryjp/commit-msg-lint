#!/usr/bin/env python3
"""Keep the checked-in configuration schema synchronized with the CLI."""

from __future__ import annotations

import os
import stat
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SCHEMA_PATH = Path("site/commit-msg-lint.schema.json")
GENERATOR_COMMAND = ["cargo", "run", "--quiet", "--locked", "--", "--print-config-schema"]


def generate_schema(root: Path) -> bytes:
    return subprocess.run(
        GENERATOR_COMMAND, cwd=root, stdout=subprocess.PIPE, check=True
    ).stdout


def indexed_schema(root: Path) -> bytes | None:
    result = subprocess.run(
        ["git", "ls-files", "-z", "--", SCHEMA_PATH.as_posix()],
        cwd=root,
        stdout=subprocess.PIPE,
        check=True,
    )
    if not result.stdout:
        return None
    return subprocess.run(
        ["git", "cat-file", "blob", f":{SCHEMA_PATH.as_posix()}"],
        cwd=root,
        stdout=subprocess.PIPE,
        check=True,
    ).stdout


def replace_schema(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    mode = stat.S_IMODE(path.stat().st_mode) if path.exists() else 0o644
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as output:
            temporary = Path(output.name)
            output.write(content)
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def sync_schema(root: Path) -> int:
    path = root / SCHEMA_PATH
    try:
        generated = generate_schema(root)
        indexed = indexed_schema(root)
        current = path.read_bytes() if path.exists() else None
        changed = current != generated
        if changed:
            replace_schema(path, generated)
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"Could not synchronize the configuration schema: {error}", file=sys.stderr)
        return 1

    if changed or indexed != generated:
        if changed:
            action = "Updated"
        elif indexed is None:
            action = "Found untracked"
        else:
            action = "Found an unstaged update to"
        print(
            f"{action} {SCHEMA_PATH.as_posix()}. "
            f"Review it and stage it with git add {SCHEMA_PATH.as_posix()} before committing.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(sync_schema(ROOT))
