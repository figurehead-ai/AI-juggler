#!/usr/bin/env python3
"""Every link in the public docs resolves (the 0.1 checklist, 8.8).

The README, the community files and the user guide: each relative link
and image must name a file in the repository, and a `#heading` must name
a heading in that file (GitHub's anchor rules). Links to other sites are
checked only with --online (they need the network).

Usage: scripts/docs_links_check.py [--online]
"""
import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PUBLIC = [
    "README.md",
    "CONTRIBUTING.md",
    "SUPPORT.md",
    "SECURITY.md",
    "CODE_OF_CONDUCT.md",
    "TRADEMARKS.md",
    "docs/usage-stats.md",
    *sorted(str(p.relative_to(ROOT)) for p in (ROOT / "docs/guide").glob("*.md")),
]
LINK = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
HTML = re.compile(r"""(?:href|src|srcset)=["']([^"']+)["']""")
FENCE = re.compile(r"^(```|~~~)")


def text_without_code(text: str) -> str:
    """The file without fenced code blocks or inline code (no links there)."""
    out, inside = [], False
    for line in text.splitlines():
        if FENCE.match(line.strip()):
            inside = not inside
            continue
        if not inside:
            out.append(re.sub(r"`[^`]*`", "", line))
    return "\n".join(out)


def anchors(path: Path) -> set[str]:
    """The `#anchor`s GitHub makes for a Markdown file's headings."""
    seen: dict[str, int] = {}
    out = set()
    for line in text_without_code(path.read_text(encoding="utf-8")).splitlines():
        match = re.match(r"^#{1,6}\s+(.*?)\s*#*\s*$", line)
        if not match:
            continue
        heading = re.sub(r"<[^>]+>", "", match.group(1))
        slug = re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-")
        count = seen.get(slug, 0)
        seen[slug] = count + 1
        out.add(slug if count == 0 else f"{slug}-{count}")
    return out


def check(online: bool) -> list[str]:
    problems = []
    for name in PUBLIC:
        source = ROOT / name
        if not source.exists():
            continue
        body = text_without_code(source.read_text(encoding="utf-8"))
        targets = LINK.findall(body) + HTML.findall(body)
        for target in targets:
            target = target.strip()
            if target.startswith(("mailto:", "tel:")):
                continue
            if target.startswith(("http://", "https://")):
                if online:
                    try:
                        request = urllib.request.Request(target, method="HEAD", headers={"User-Agent": "docs-links-check"})
                        urllib.request.urlopen(request, timeout=15)
                    except Exception as error:  # noqa: BLE001 — any failure is a broken link
                        problems.append(f"{name}: {target} — {error}")
                continue
            path_part, _, anchor = target.partition("#")
            resolved = source if not path_part else (source.parent / path_part).resolve()
            if path_part and not resolved.exists():
                problems.append(f"{name}: {target} — no such file")
                continue
            if anchor and resolved.suffix == ".md" and resolved.is_file() and anchor not in anchors(resolved):
                problems.append(f"{name}: {target} — no heading #{anchor} in {resolved.relative_to(ROOT)}")
    return problems


def main() -> int:
    problems = check("--online" in sys.argv[1:])
    for problem in problems:
        print(f"docs-links: {problem}")
    if problems:
        return 1
    print(f"docs-links: OK ({len(PUBLIC)} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
