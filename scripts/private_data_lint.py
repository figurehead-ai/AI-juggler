#!/usr/bin/env python3
"""No real person's data in the public repository (0.4.2, D-101).

Generic patterns only, so no real name has to live here to be caught:
- a home folder `/Users/<name>` (or `-Users-<name>-`, the form tools use
  in folder names) other than the placeholders below;
- a Claude profile folder `ClaudeProfiles/<name>` or
  `ClaudeCodeProfiles/<name>` other than the fixtures below;
- an e-mail address outside example.com/.org/.net, GitHub's noreply
  and the project's own addresses.

Real names (the maintainer's profiles, people) are checked before they
leave the Mac by the hooks in scripts/hooks/ against a private list
outside the repository; see scripts/hooks/README.md.

    python3 scripts/private_data_lint.py            every tracked text file
    python3 scripts/private_data_lint.py --selftest
"""
from __future__ import annotations

import re
import subprocess
import sys

# Placeholders a fixture or a doc may use for a home folder.
HOMES = {
    "you", "dev", "me", "someone", "someone-q9", "nobody-z", "a", "b", "x", "u",
    "real", "win", "sam", "runner", "user", "example", "Shared", "test", "alice", "bob",
}
# Fixture profile folders (generic words, never a person's).
PROFILES = {
    "aij-test", "work", "work-a", "work-2", "Work", "b-work", "acct-2", "max", "lab",
    "P", "b", "wor", "team-a", "studio-main", "client-a", "client-b", "acme-max",
    "a-north", "b-north", "b-east", "b-south", "b-west", "b-main",
    "acme-north", "acme-east", "acme-south", "acme-west", "<profile>", "x", "y",
}
EMAIL_OK = re.compile(
    r"@(?:[a-z0-9-]+\.)*example(?:\.(?:com|org|net))?$|@users\.noreply\.github\.com$|^noreply@|"
    r"\.(?:invalid|test|example|local|localhost|mcp)$|@github\.com$|\.(?:png|jpe?g|svg|gif|webp)$",
    re.I,
)
# Published addresses (vendors' contact points) and old fixture addresses
# on made-up mailboxes.
EMAIL_ALLOW = {
    "product-security@apple.com", "partnercomms@openai.com", "help@anoma.ly",
    "info@figurehead.ai", "marketing@anthropic.com",
    "a@b.co", "sam@work.com", "o@work.com",
}
# Third-party records (cargo-vet's imported audits name their auditors).
SKIP_PREFIXES = ("supply-chain/",)

HOME = re.compile(r"/Users/([A-Za-z0-9._-]+)|-Users-([A-Za-z0-9._]+)-")
PROFILE = re.compile(r"Claude(?:Code)?Profiles/([A-Za-z0-9._<>-]+)")
EMAIL = re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}")
# Lines that name a pattern rather than a person (this lint, its docs).
SKIP_FILES = {"scripts/private_data_lint.py"}


def findings(path: str, text: str) -> list[str]:
    out = []
    for n, line in enumerate(text.splitlines(), 1):
        for m in HOME.finditer(line):
            name = (m.group(1) or m.group(2)).rstrip(".-")
            if name not in HOMES and not name.startswith(("<", "$", "{")):
                out.append(f"{path}:{n}: a home folder that isn't a placeholder: {m.group(0)}")
        for m in PROFILE.finditer(line):
            if m.group(1).rstrip(".") not in PROFILES and not m.group(1).startswith("<"):
                out.append(f"{path}:{n}: a profile folder that isn't a fixture: {m.group(0)}")
        for m in EMAIL.finditer(line):
            address = m.group(0)
            if not EMAIL_OK.search(address) and address not in EMAIL_ALLOW and "…" not in address:
                out.append(f"{path}:{n}: an e-mail address outside example.com/noreply: {address}")
    return out


def tracked() -> list[str]:
    files = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout
    return [f.decode() for f in files.split(b"\0") if f]


def main() -> int:
    problems = []
    for path in tracked():
        if path in SKIP_FILES or path.startswith(SKIP_PREFIXES):
            continue
        try:
            raw = open(path, "rb").read()
        except OSError:
            continue
        if b"\0" in raw[:8000]:
            continue
        problems += findings(path, raw.decode("utf-8", "replace"))
    for p in problems:
        print(p)
    if problems:
        print(f"private-data: {len(problems)} finding(s); use a placeholder (/Users/you, a fixture profile, example.com) — scripts/hooks/README.md")
        return 1
    print("private-data: no home folder, profile folder or address outside the placeholders")
    return 0


def selftest() -> int:
    bad = "/Users/jane" + "doe/x ClaudeCodeProfiles/jane" + "corp-jane mail jane" + "@janecorp.io -Users-jane" + "doe-Dev"
    good = "/Users/you/x ClaudeCodeProfiles/aij-test w…@example.com 1+x@users.noreply.github.com"
    assert len(findings("t", bad)) == 4, findings("t", bad)
    assert findings("t", good) == [], findings("t", good)
    print("private-data: selftest passed")
    return 0


if __name__ == "__main__":
    sys.exit(selftest() if sys.argv[1:] == ["--selftest"] else main())
