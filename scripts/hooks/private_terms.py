#!/usr/bin/env python3
"""The private-terms guard (0.4.2, D-101): no real name leaves this Mac.

Reads a denylist kept OUTSIDE the repository (it holds the real names):
  $AIJ_PRIVATE_TERMS, else ~/.config/ai-juggler/private-terms
one term per line, matched case-insensitively as a substring; `#` starts
a comment. Never commit that file.

  private_terms.py staged          the staged changes' added lines and paths (pre-commit)
  private_terms.py message FILE    a commit message (commit-msg)
  private_terms.py push OLD NEW    every outgoing commit's added lines, paths and message (pre-push)
  private_terms.py --selftest

Without a denylist it says so and lets the change through: the public
check (scripts/private_data_lint.py, in CI) still catches home folders,
profile folders and addresses by pattern.
"""
from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

ZERO = "0" * 40


def denylist_path() -> Path:
    env = os.environ.get("AIJ_PRIVATE_TERMS")
    return Path(env) if env else Path.home() / ".config/ai-juggler/private-terms"


def load_terms(path: Path) -> list[str]:
    if not path.is_file():
        return []
    terms = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            terms.append(line.lower())
    return terms


def git(*args: str) -> str:
    return subprocess.run(["git", *args], capture_output=True, text=True, errors="replace", check=True).stdout


def hits(text: str, terms: list[str]) -> list[str]:
    low = text.lower()
    return [t for t in terms if t in low]


def added_lines(diff: str) -> list[tuple[str, str]]:
    """(path, line) for each added line of a unified diff, and each path."""
    out, path = [], "?"
    for line in diff.splitlines():
        if line.startswith("+++ "):
            path = line[6:] if line.startswith("+++ b/") else line[4:]
            out.append((path, path))
        elif line.startswith("diff --git "):
            out.append(("(path)", line.split(" b/", 1)[-1]))
        elif line.startswith("+") and not line.startswith("+++"):
            out.append((path, line[1:]))
    return out


def report(where: str, found: list[tuple[str, str]]) -> int:
    # Never echo the term itself into logs: say where, not what.
    for place, n in found[:20]:
        print(f"private-terms: a denylisted term in {where}: {place} (term #{n})", file=sys.stderr)
    if found:
        print(
            f"private-terms: {len(found)} hit(s). Replace the real name with a generic one (acme-*, /Users/you, example.com); "
            "see scripts/hooks/README.md.",
            file=sys.stderr,
        )
        return 1
    return 0


def check_diff(diff: str, terms: list[str], where: str) -> list[tuple[str, str]]:
    found = []
    for path, line in added_lines(diff):
        for t in hits(line, terms):
            found.append((f"{where} {path}", str(terms.index(t) + 1)))
    return found


def staged(terms: list[str]) -> int:
    diff = git("diff", "--cached", "-U0", "--no-color", "--text", "--no-ext-diff")
    return report("the staged changes", check_diff(diff, terms, "staged"))


def message(path: str, terms: list[str]) -> int:
    text = Path(path).read_text(encoding="utf-8", errors="replace")
    text = "\n".join(l for l in text.splitlines() if not l.startswith("#"))
    return report("the commit message", [("message", str(terms.index(t) + 1)) for t in hits(text, terms)])


def push(old: str, new: str, terms: list[str]) -> int:
    if new == ZERO:
        return 0  # a delete
    span = new if old == ZERO else f"{old}..{new}"
    # A new branch: only the commits no remote branch has yet.
    commits = git("rev-list", span, "--not", "--remotes").split() if old == ZERO else git("rev-list", span).split()
    found = []
    for c in commits:
        found += check_diff(git("show", "--format=", "-U0", "--no-color", "--text", c), terms, c[:9])
        found += [(f"{c[:9]} message", str(terms.index(t) + 1)) for t in hits(git("log", "-1", "--format=%B", c), terms)]
    return report("the commits to push", found)


def selftest() -> int:
    with tempfile.TemporaryDirectory() as d:
        repo = Path(d)
        terms_file = repo.parent / (repo.name + "-terms")
        terms_file.write_text("# test\nZebracorp\n")
        env = dict(os.environ, AIJ_PRIVATE_TERMS=str(terms_file), GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1")
        run = lambda *a, **k: subprocess.run(a, cwd=repo, env=env, capture_output=True, text=True, **k)
        run("git", "init", "-q")
        run("git", "config", "user.email", "t@example.com")
        run("git", "config", "user.name", "t")
        (repo / "a.txt").write_text("fine\n")
        run("git", "add", "a.txt")
        me = str(Path(__file__).resolve())
        assert run(sys.executable, me, "staged").returncode == 0
        (repo / "a.txt").write_text("fine\nthe zebracorp-ann profile\n")
        run("git", "add", "a.txt")
        bad = run(sys.executable, me, "staged")
        assert bad.returncode == 1 and "zebracorp" not in bad.stderr.lower(), bad.stderr
        msg = repo / "MSG"
        msg.write_text("fix: for ZEBRACORP\n")
        assert run(sys.executable, me, "message", str(msg)).returncode == 1
        run("git", "commit", "-qm", "a clean message")
        head = run("git", "rev-parse", "HEAD").stdout.strip()
        assert run(sys.executable, me, "push", ZERO, head).returncode == 1
        terms_file.unlink()
    print("private-terms: selftest passed")
    return 0


def main(argv: list[str]) -> int:
    if argv[:1] == ["--selftest"]:
        return selftest()
    terms = load_terms(denylist_path())
    if not terms:
        print(f"private-terms: no denylist at {denylist_path()} (see scripts/hooks/README.md); only CI's pattern check applies", file=sys.stderr)
        return 0
    if argv[:1] == ["staged"]:
        return staged(terms)
    if argv[:1] == ["message"] and len(argv) == 2:
        return message(argv[1], terms)
    if argv[:1] == ["push"] and len(argv) == 3:
        return push(argv[1], argv[2], terms)
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
