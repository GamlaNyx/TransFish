#!/usr/bin/env python3
"""Fail if credentials look staged for commit.

Scans tracked files (or just the index with --staged) and reports only the
rule name and location, never the matched text, so running this can't leak
a key into a terminal log or CI transcript.
"""

from __future__ import annotations

import re
import subprocess
import sys

MAX_BYTES = 1_500_000
DENY_NAMES = re.compile(
    r"(^|/)(settings.*\.json|AccessKey.*\.txt|.*\.(pem|key|pfx|p12|secret))$",
    re.IGNORECASE,
)
RULES = [
    (
        "volcengine-access-key",
        re.compile(rb"\bAKLT[0-9A-Za-z]{16,}\b"),
    ),
    (
        "secret-access-key-assignment",
        re.compile(
            rb"(?i)secret[_-]?access[_-]?key[A-Za-z0-9_\-]*\"?\s*[:=]\s*\"[^\"]{16,}\""
        ),
    ),
    (
        "private-key-block",
        re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
    ),
    (
        "bearer-token-assignment",
        re.compile(rb"(?i)\b(?:api[_-]?token|auth[_-]?token)\"?\s*[:=]\s*\"[A-Za-z0-9_\-\.]{24,}\""),
    ),
]


def git(*args: str, binary: bool = False):
    return subprocess.check_output(["git", *args], stderr=subprocess.DEVNULL)


def index_files() -> list[str]:
    out = git("diff", "--cached", "--name-only", "--diff-filter=ACMR")
    return [line for line in out.decode("utf-8", "replace").splitlines() if line]


def tracked_files() -> list[str]:
    out = git("ls-files", "-z")
    return [p for p in out.decode("utf-8", "replace").split("\0") if p]


def read(path: str, staged: bool) -> bytes | None:
    try:
        if staged:
            return git("show", f":{path}", binary=True)
        with open(path, "rb") as handle:
            return handle.read()
    except (subprocess.CalledProcessError, OSError):
        return None


def main() -> int:
    staged = "--staged" in sys.argv[1:]
    files = index_files() if staged else tracked_files()
    problems: list[str] = []

    for path in files:
        if DENY_NAMES.search(path):
            problems.append(f"{path}: credential-shaped filename")
            continue
        data = read(path, staged)
        if data is None or len(data) > MAX_BYTES or b"\0" in data[:4096]:
            continue
        for name, pattern in RULES:
            match = pattern.search(data)
            if match:
                line = data.count(b"\n", 0, match.start()) + 1
                problems.append(f"{path}:{line}: {name}")

    if problems:
        print("Refusing to continue: possible credentials found.", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        print("Remove the value (or the file) before committing.", file=sys.stderr)
        return 1

    print(f"secret scan clean ({len(files)} files, {'index' if staged else 'worktree'})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
