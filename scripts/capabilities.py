#!/usr/bin/env python3
"""What each tool lets AI Juggler do, per layer (CLI and desktop app), kept
current: `docs/research/capabilities/manifest.json`.

The maintainer, 2026-10-27: "we should have an ongoing list of manifest of
capabilities for each of the models we support, along with versions, and
it should be kept updated and we should periodically check and keep fresh,
and this should be committed".

- `check`  — the manifest's shape, one entry per layer of every tool the
  app registers (`juggler-adapters`' `*_ID` constants), sources on every
  verified claim, and the README rendered from it is current. Runs in
  `just verify`; it reads no network.
- `render` — writes `docs/research/capabilities/README.md` from the JSON.
- `latest` — asks each entry's public version feed (npm, Homebrew, GitHub,
  PyPI) for the newest version and lists the entries to re-verify: a newer
  release than the one checked, or a check older than `stale_after_days`.
  `--json` prints the list for the weekly workflow, which opens an issue.
  It sends nothing of the user's: package names only.
"""

from __future__ import annotations

import datetime as dt
import json
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
DIR = ROOT / "docs/research/capabilities"
MANIFEST = DIR / "manifest.json"
README = DIR / "README.md"
ADAPTERS = ROOT / "crates/juggler-adapters/src"

LAYERS = ("cli", "desktop")
STATUSES = ("available", "partial", "unavailable", "unknown")
EVIDENCE = ("VP", "VS", "NE", "I", "U")
SUPPORT = ("built", "planned", "track-only", "not-planned")
FEEDS = {
    "npm": "https://registry.npmjs.org/{name}/latest",
    "brew-cask": "https://formulae.brew.sh/api/cask/{name}.json",
    "brew-formula": "https://formulae.brew.sh/api/formula/{name}.json",
    "github-release": "https://api.github.com/repos/{name}/releases/latest",
    "pypi": "https://pypi.org/pypi/{name}/json",
}
# Test doubles, never real tools.
NOT_TOOLS = {"fake", "fake-desktop"}


def load() -> dict:
    return json.loads(MANIFEST.read_text())


def registered_tools() -> set[str]:
    """The tool ids the app's adapters declare."""
    ids = set()
    for path in ADAPTERS.glob("*.rs"):
        for match in re.finditer(r'pub const [A-Z_]+_ID: &str = "([a-z0-9-]+)";', path.read_text()):
            ids.add(match.group(1))
    return ids - NOT_TOOLS


def problems(manifest: dict) -> list[str]:
    out = []
    for key in ("schema", "stale_after_days", "entries"):
        if key not in manifest:
            out.append(f"manifest: no '{key}'")
    seen: set[tuple[str, str]] = set()
    for entry in manifest.get("entries", []):
        name = f"{entry.get('tool')}/{entry.get('layer')}"
        for key in ("tool", "layer", "product", "support", "checked_at", "capabilities"):
            if key not in entry:
                out.append(f"{name}: no '{key}'")
        if entry.get("layer") not in LAYERS:
            out.append(f"{name}: layer must be one of {LAYERS}")
        if entry.get("support") not in SUPPORT:
            out.append(f"{name}: support must be one of {SUPPORT}")
        try:
            dt.date.fromisoformat(entry.get("checked_at", ""))
        except ValueError:
            out.append(f"{name}: checked_at is not a date")
        feed = entry.get("version_feed")
        if feed is not None and (feed.get("kind") not in FEEDS or not feed.get("name")):
            out.append(f"{name}: version_feed needs a kind in {sorted(FEEDS)} and a name")
        pair = (entry.get("tool"), entry.get("layer"))
        if pair in seen:
            out.append(f"{name}: listed twice")
        seen.add(pair)
        ids = set()
        for cap in entry.get("capabilities", []):
            where = f"{name}: {cap.get('id')}"
            if cap.get("id") in ids:
                out.append(f"{where}: listed twice")
            ids.add(cap.get("id"))
            for key in ("id", "name", "status", "how", "evidence", "sources", "ai_juggler_uses"):
                if key not in cap:
                    out.append(f"{where}: no '{key}'")
            if cap.get("status") not in STATUSES:
                out.append(f"{where}: status must be one of {STATUSES}")
            if cap.get("evidence") not in EVIDENCE:
                out.append(f"{where}: evidence must be one of {EVIDENCE}")
            # A verified claim names where it was verified.
            if cap.get("evidence") in ("VP", "VS") and not cap.get("sources"):
                out.append(f"{where}: {cap.get('evidence')} with no source")
    for entry in manifest.get("entries", []):
        research = entry.get("research")
        if research and not (DIR / research).is_file():
            out.append(f"{entry.get('tool')}/{entry.get('layer')}: its research file {research} isn't there")
    # On a Mac's case-insensitive disk `claude.md` *is* `CLAUDE.md`, which
    # coding agents load as instructions for the folder: never such a name.
    for path in DIR.iterdir():
        if path.name.lower() in ("claude.md", "agents.md", "gemini.md", "agent.md"):
            out.append(f"{path.name}: an agent instruction file's name; rename it")
    tools = {entry.get("tool") for entry in manifest.get("entries", [])}
    for tool in sorted(registered_tools() - tools):
        out.append(f"{tool}: the app registers it, the manifest doesn't list it")
    for tool in sorted(tools):
        for layer in LAYERS:
            if (tool, layer) not in seen:
                out.append(f"{tool}: no '{layer}' entry (say 'unavailable' when there is none)")
    return out


def cell(text: object) -> str:
    return str(text if text not in (None, "") else "—").replace("|", "\\|").replace("\n", " ")


def render(manifest: dict) -> str:
    lines = [
        "# Tool capabilities",
        "",
        "What each tool lets AI Juggler observe and control, per layer, with the",
        "version it was checked against. Generated from `manifest.json` by",
        "`python3 scripts/capabilities.py render`; edit the JSON, not this file.",
        f"Each entry is re-verified when its tool ships a newer version or after "
        f"{manifest['stale_after_days']} days (the weekly `capabilities` workflow",
        "opens an issue). Evidence labels: `docs/research/EVIDENCE-STANDARD.md`.",
        "Research notes per tool family are beside this file; `FINDINGS.md` says what to do about them.",
        "",
        "| Tool | Layer | Product | In AI Juggler | Checked | Latest | Checked on |",
        "|---|---|---|---|---|---|---|",
    ]
    entries = sorted(manifest["entries"], key=lambda e: (e["tool"], LAYERS.index(e["layer"])))
    for entry in entries:
        lines.append(
            f"| {cell(entry['tool'])} | {entry['layer']} | {cell(entry['product'])} | {entry['support']} "
            f"| {cell(entry.get('checked_version'))} | {cell(entry.get('latest_version'))} | {entry['checked_at']} |"
        )
    for entry in entries:
        lines += ["", f"## {entry['product']} ({entry['tool']}, {entry['layer']})", ""]
        if entry.get("notes"):
            lines += [entry["notes"], ""]
        lines += ["| Capability | Status | How | Limits | Evidence | AI Juggler uses |", "|---|---|---|---|---|---|"]
        for cap in entry["capabilities"]:
            lines.append(
                f"| {cell(cap['name'])} | {cap['status']} | {cell(cap['how'])} | {cell(cap.get('limits'))} "
                f"| {cap['evidence']} | {cell(cap['ai_juggler_uses'])} |"
            )
    return "\n".join(lines) + "\n"


def fetch_latest(feed: dict) -> str | None:
    url = FEEDS[feed["kind"]].format(name=feed["name"])
    request = urllib.request.Request(url, headers={"User-Agent": "ai-juggler-capabilities-check"})
    with urllib.request.urlopen(request, timeout=20) as response:  # noqa: S310 (fixed https hosts)
        body = json.load(response)
    if feed["kind"] == "npm":
        return body.get("version")
    if feed["kind"] in ("brew-cask", "brew-formula"):
        # A cask's version may carry its build after a comma ("1.4.8,5447…").
        version = body.get("version") or (body.get("versions") or {}).get("stable")
        return version.split(",")[0] if version else None
    if feed["kind"] == "github-release":
        return (body.get("tag_name") or "").lstrip("v") or None
    return (body.get("info") or {}).get("version")


def due(manifest: dict, today: dt.date) -> list[dict]:
    """The entries to re-verify, and why."""
    out = []
    for entry in manifest["entries"]:
        reasons = []
        age = (today - dt.date.fromisoformat(entry["checked_at"])).days
        if age > manifest["stale_after_days"]:
            reasons.append(f"checked {age} days ago")
        feed = entry.get("version_feed")
        latest = None
        if feed:
            try:
                latest = fetch_latest(feed)
            except Exception as error:  # a feed that's down is reported, not fatal
                reasons.append(f"couldn't read its version feed ({type(error).__name__})")
            if latest and latest != entry.get("latest_version"):
                reasons.append(f"{latest} is out (the manifest has {entry.get('latest_version')})")
        if reasons:
            out.append({"tool": entry["tool"], "layer": entry["layer"], "product": entry["product"],
                        "latest": latest, "reasons": reasons})
    return out


def main(argv: list[str]) -> int:
    command = argv[1] if len(argv) > 1 else "check"
    manifest = load()
    if command == "render":
        README.write_text(render(manifest))
        print(f"capabilities: wrote {README.relative_to(ROOT)}")
        return 0
    if command == "check":
        found = problems(manifest)
        if README.read_text() != render(manifest):
            found.append("README.md is not what manifest.json renders: run `python3 scripts/capabilities.py render`")
        for problem in found:
            print(f"capabilities: {problem}")
        if not found:
            print(f"capabilities: {len(manifest['entries'])} entries OK")
        return 1 if found else 0
    if command == "latest":
        result = due(manifest, dt.date.today())
        if "--json" in argv:
            print(json.dumps(result, indent=2))
        else:
            for row in result:
                print(f"{row['tool']}/{row['layer']}: " + "; ".join(row["reasons"]))
            if not result:
                print("capabilities: every entry is current")
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
