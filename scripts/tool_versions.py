#!/usr/bin/env python3
"""D-128: the daily tool-version watcher.

The maintainer, 2026-11-18: a script that "periodically checks versions of
all of our tools... and if it finds change it alerts... so we can double
check nothing broke". Tools' updates never block AI Juggler; this says when
one happened so the shape checks and the fidelity register get a look.

What it reads: the adapter catalog. Each adapter's `ToolSpec` names the
public lists of its tool's versions (`release_feed`, `more_release_feeds`,
crates/juggler-adapters/src/*.rs): an npm package, a GitHub repository, or
a desktop app's Sparkle appcast. No tool is listed here; a tool with a
feed in its spec is watched. Only version metadata is fetched, never a tool.

What it does: compares each feed's newest version with the last one seen
(a small JSON state file the workflow keeps in the Actions cache) and, on a
change, opens ONE issue per tool and version, labelled `tool-update`,
unless an issue with that title already exists (open or closed). The first
sight of a feed only records its version. A feed that is down or malformed
is a warning, never an issue, and its recorded version is kept. Only
GITHUB_TOKEN is used, through the gh CLI.

Usage: tool_versions.py [--state FILE] [--repo OWNER/NAME] [--dry-run]
       tool_versions.py --selftest   (recorded fixtures, no network)
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.request
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

ROOT = Path(__file__).resolve().parent.parent
ADAPTERS = ROOT / "crates/juggler-adapters/src"
FIXTURES = ROOT / "scripts/fixtures/tool_versions"
LABEL = "tool-update"
NEXT_STEP = (
    "Next step: re-check the shape checks and the fidelity register "
    "(release_scope lists due gates)."
)
NPM_REGISTRY = "https://registry.npmjs.org"
SPARKLE = "{http://www.andymatuschak.org/xml-namespaces/sparkle}"
VERSION = re.compile(r"^[0-9][0-9A-Za-z.+-]{0,63}$")
# The spec's own words for each feed variant: ReleaseFeed::<Variant> { field: "…" }.
FEED_KINDS = {"Npm": ("npm", "package"), "GithubRelease": ("github", "repo"), "Appcast": ("appcast", "url")}
NOT_TOOLS = {"fake", "fake-desktop"}


class FeedError(Exception):
    """A feed answered with something that is not a version."""


@dataclass(frozen=True)
class Feed:
    tool: str
    name: str
    kind: str  # npm | github | appcast
    ref: str  # package, repo or url

    @property
    def key(self) -> str:
        return f"{self.tool}|{self.kind}|{self.ref}"

    @property
    def url(self) -> str:
        if self.kind == "npm":
            return f"{NPM_REGISTRY}/{self.ref.replace('/', '%2F')}/latest"
        if self.kind == "github":
            return f"https://api.github.com/repos/{self.ref}/releases/latest"
        return self.ref

    @property
    def source(self) -> str:
        return {"npm": f"npm package `{self.ref}`", "github": f"GitHub repository `{self.ref}`",
                "appcast": f"update feed {self.ref}"}[self.kind]


@dataclass(frozen=True)
class Found:
    version: str
    notes: str | None = None


# --- the catalog ---------------------------------------------------------

def catalog(directory: Path = ADAPTERS) -> list[Feed]:
    """Every feed the adapters' specs name, read from the adapter sources."""
    feeds: list[Feed] = []
    for path in sorted(directory.glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        tool = re.search(r'pub const [A-Z_]+_ID: &str = "([a-z0-9-]+)";', text)
        if not tool or tool.group(1) in NOT_TOOLS:
            continue
        name = re.search(r'display_name: "([^"]+)"', text)
        for match in re.finditer(r'ReleaseFeed::(\w+)\s*\{\s*(\w+):\s*"([^"]+)"', text):
            variant, field, value = match.groups()
            kind = FEED_KINDS.get(variant)
            if kind and kind[1] == field:
                feeds.append(Feed(tool.group(1), name.group(1) if name else tool.group(1), kind[0], value))
    return feeds


# --- reading a feed ------------------------------------------------------

def version_ok(value: object) -> str:
    if not isinstance(value, str) or not VERSION.match(value):
        raise FeedError(f"not a version: {str(value)[:40]!r}")
    return value


def parse_npm(body: bytes, feed: Feed) -> Found:
    data = json.loads(body)
    if not isinstance(data, dict):
        raise FeedError("not an object")
    version = version_ok(data.get("version"))
    return Found(version, f"https://www.npmjs.com/package/{feed.ref}/v/{version}")


def parse_github(body: bytes, feed: Feed) -> Found:
    data = json.loads(body)
    if not isinstance(data, dict):
        raise FeedError("not an object")
    version = version_ok(str(data.get("tag_name") or "").removeprefix("v"))
    link = data.get("html_url")
    return Found(version, link if isinstance(link, str) and link.startswith("https://") else None)


def _vkey(version: str) -> tuple:
    return tuple((0, int(p), "") if p.isdigit() else (1, 0, p) for p in re.split(r"[.+-]", version))


def parse_appcast(body: bytes, feed: Feed) -> Found:
    text = body.decode("utf-8", "replace").upper()
    if "<!DOCTYPE" in text or "<!ENTITY" in text:
        raise FeedError("a feed with a DTD")
    root = ET.fromstring(body)
    best: Found | None = None
    for item in root.iter("item"):
        enclosure = item.find("enclosure")
        short = item.findtext(f"{SPARKLE}shortVersionString")
        build = item.findtext(f"{SPARKLE}version")
        if enclosure is not None:
            short = short or enclosure.get(f"{SPARKLE}shortVersionString")
            build = build or enclosure.get(f"{SPARKLE}version")
        version = version_ok(short or build)
        notes = item.findtext(f"{SPARKLE}releaseNotesLink") or item.findtext("link")
        notes = notes.strip() if notes and notes.strip().startswith("https://") else None
        if best is None or _vkey(version) > _vkey(best.version):
            best = Found(version, notes)
    if best is None:
        raise FeedError("no items")
    return best


PARSERS: dict[str, Callable[[bytes, Feed], Found]] = {
    "npm": parse_npm, "github": parse_github, "appcast": parse_appcast,
}


def read_feed(feed: Feed, fetch: Callable[[str], bytes]) -> Found:
    try:
        return PARSERS[feed.kind](fetch(feed.url), feed)
    except FeedError:
        raise
    except (ValueError, ET.ParseError, KeyError) as error:  # json, xml, shape
        raise FeedError(f"{type(error).__name__}: {str(error)[:80]}") from error


def http_fetch(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "ai-juggler-tool-versions"})
    with urllib.request.urlopen(request, timeout=20) as response:  # noqa: S310 (https URLs from the catalog)
        return response.read(2_000_000)


# --- issues --------------------------------------------------------------

def issue_title(feed: Feed, version: str) -> str:
    return f"Tool update: {feed.name} {version}"


def issue_body(feed: Feed, old: str | None, found: Found) -> str:
    lines = [
        f"{feed.name} has a new public version: **{found.version}**"
        + (f" (last seen: {old})." if old else "."),
        "",
        f"Source: {feed.source}.",
    ]
    if found.notes:
        lines.append(f"Release notes: {found.notes}")
    lines += ["", NEXT_STEP, "", "Opened by `scripts/tool_versions.py` (D-128). Updates never block the app."]
    return "\n".join(lines)


class GhIssues:
    """Issues through the gh CLI, with GITHUB_TOKEN as GH_TOKEN."""

    def __init__(self, repo: str):
        self.repo = repo

    def _gh(self, *args: str) -> str:
        env = dict(os.environ)
        env.setdefault("GH_TOKEN", env.get("GITHUB_TOKEN", ""))
        done = subprocess.run(["gh", *args], capture_output=True, text=True, env=env, check=False)
        if done.returncode:
            raise RuntimeError(done.stderr.strip()[:200] or "gh failed")
        return done.stdout

    def ensure_label(self) -> None:
        try:
            self._gh("label", "create", LABEL, "--repo", self.repo, "--color", "0E8A16",
                     "--description", "A supported tool published a new version (D-128)")
        except RuntimeError:
            pass  # it is there already

    def exists(self, title: str) -> bool:
        out = self._gh("issue", "list", "--repo", self.repo, "--state", "all", "--label", LABEL,
                       "--search", f'"{title}" in:title', "--json", "title", "--limit", "20")
        return any(row["title"] == title for row in json.loads(out or "[]"))

    def create(self, title: str, body: str) -> None:
        self._gh("issue", "create", "--repo", self.repo, "--title", title, "--body", body, "--label", LABEL)


class DryIssues:
    def exists(self, title: str) -> bool:
        return False

    def create(self, title: str, body: str) -> None:
        print(f"--- would open: {title}\n{body}\n")


# --- the run -------------------------------------------------------------

def run(feeds: list[Feed], state: dict, fetch, issues, warn=print) -> tuple[dict, int]:
    """One pass. Returns the new state and how many issues were opened."""
    state = dict(state)
    opened = 0
    for feed in feeds:
        try:
            found = read_feed(feed, fetch)
        except Exception as error:  # a feed that's down is a warning, never an issue
            warn(f"::warning::{feed.tool}: could not read {feed.url} ({type(error).__name__}: {str(error)[:100]})")
            continue
        old = state.get(feed.key)
        if old == found.version:
            continue
        if old is not None:
            title = issue_title(feed, found.version)
            try:
                if not issues.exists(title):
                    issues.create(title, issue_body(feed, old, found))
                    opened += 1
            except Exception as error:  # try again tomorrow; keep the old version
                warn(f"::warning::{feed.tool}: could not open the issue ({str(error)[:100]})")
                continue
        state[feed.key] = found.version
    return state, opened


def load_state(path: Path) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        return {k: v for k, v in data.items() if isinstance(k, str) and isinstance(v, str)}
    except (OSError, ValueError, AttributeError):
        return {}


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--selftest", action="store_true")
    parser.add_argument("--state", default=os.environ.get("TOOL_VERSIONS_STATE", "tool-versions-state.json"))
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    parser.add_argument("--dry-run", action="store_true", help="print issues instead of opening them")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    feeds = catalog()
    if not feeds:
        print("tool_versions: the adapter catalog names no feed", file=sys.stderr)
        return 1
    if args.dry_run:
        issues = DryIssues()
    else:
        if not args.repo:
            print("tool_versions: no --repo or GITHUB_REPOSITORY", file=sys.stderr)
            return 1
        issues = GhIssues(args.repo)
        issues.ensure_label()
    path = Path(args.state)
    state, opened = run(feeds, load_state(path), http_fetch, issues)
    path.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"tool_versions: {len(feeds)} feeds, {opened} issue(s) opened")
    return 0


# --- selftest ------------------------------------------------------------

class FakeIssues:
    def __init__(self, existing=()):
        self.titles = list(existing)
        self.created: list[tuple[str, str]] = []

    def exists(self, title: str) -> bool:
        return title in self.titles

    def create(self, title: str, body: str) -> None:
        self.titles.append(title)
        self.created.append((title, body))


def selftest() -> int:
    failures: list[str] = []

    def check(ok: bool, what: str) -> None:
        if not ok:
            failures.append(what)

    def fixture(name: str) -> bytes:
        return (FIXTURES / name).read_bytes()

    # The catalog: the sample spec, then the real adapters.
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / "sample.rs").write_text((FIXTURES / "catalog-sample.rs.txt").read_text(), encoding="utf-8")
        sample = catalog(Path(tmp))
    check([(f.kind, f.ref) for f in sample] == [("npm", "@example/tool-cli"),
          ("appcast", "https://example.com/app/appcast.xml"), ("github", "example/tool")], f"sample catalog {sample}")
    check(sample[0].url == "https://registry.npmjs.org/@example%2Ftool-cli/latest", "npm url")
    real = catalog()
    check(len(real) >= 1 and all(f.kind in PARSERS for f in real), "the real catalog names feeds")
    check(not any(f.tool in NOT_TOOLS for f in real), "no test double watched")

    npm, app, gh = sample
    files = {npm.url: "npm-latest.json", app.url: "appcast.xml", gh.url: "github-latest.json"}

    def ok_fetch(url: str) -> bytes:
        return fixture(files[url])

    # One parser per feed kind.
    check(parse_npm(fixture("npm-latest.json"), npm) == Found("2.4.1", "https://www.npmjs.com/package/@example/tool-cli/v/2.4.1"), "npm parse")
    check(parse_github(fixture("github-latest.json"), gh) == Found("0.9.3", "https://github.com/example/tool/releases/tag/v0.9.3"), "github parse")
    check(parse_appcast(fixture("appcast.xml"), app) == Found("1.2.0", "https://example.com/app/notes/1.2.0.html"), "appcast parse")
    check(parse_appcast(fixture("appcast-attrs.xml"), app) == Found("0.50.1", "https://example.com/app/changelog"), "appcast attribute form")

    warnings: list[str] = []
    # First sight: recorded, no issue.
    state, n = run(sample, {}, ok_fetch, FakeIssues(), warnings.append)
    check(n == 0 and len(state) == 3, "baseline opens nothing")
    # Unchanged: nothing.
    issues = FakeIssues()
    state2, n = run(sample, state, ok_fetch, issues, warnings.append)
    check(n == 0 and state2 == state and not issues.created, "unchanged opens nothing")
    # Changed: one issue per tool and version.
    old = {npm.key: "2.4.0", app.key: "1.1.9", gh.key: "0.9.2"}
    issues = FakeIssues()
    state3, n = run(sample, old, ok_fetch, issues, warnings.append)
    titles = [t for t, _ in issues.created]
    check(n == 3 and state3[npm.key] == "2.4.1", "changed opens one each")
    check(titles[0] == "Tool update: Sample Tool 2.4.1", f"title {titles}")
    bodies = "\n".join(b for _, b in issues.created)
    check("last seen: 2.4.0" in bodies and "https://example.com/app/notes/1.2.0.html" in bodies, "body says what changed and links notes")
    check(NEXT_STEP in bodies and "release_scope lists due gates" in bodies, "body says the next step")
    # Dedupe: the same title already exists (open or closed): no second issue, state moves on.
    issues = FakeIssues(existing=["Tool update: Sample Tool 2.4.1"])
    state4, n = run([npm], {npm.key: "2.4.0"}, ok_fetch, issues, warnings.append)
    check(n == 0 and not issues.created and state4[npm.key] == "2.4.1", "dedupe")
    run([npm], {npm.key: "2.3.0"}, ok_fetch, issues, warnings.append)
    check(len(issues.created) == 0, "dedupe holds on a rerun")
    # Malformed feeds: a warning, no issue, the old version kept.
    bad = {npm.url: "npm-malformed.json", app.url: "appcast-malformed.xml", gh.url: "github-malformed.json"}
    warnings.clear()
    issues = FakeIssues()
    state5, n = run(sample, old, lambda url: fixture(bad[url]), issues, warnings.append)
    check(n == 0 and not issues.created and state5 == old, "malformed opens nothing, keeps state")
    check(len(warnings) == 3 and all(w.startswith("::warning::") for w in warnings), f"malformed warns {warnings}")

    # A feed that cannot be reached is the same.
    def down(url: str) -> bytes:
        raise OSError("offline")

    warnings.clear()
    state6, n = run([npm], old, down, FakeIssues(), warnings.append)
    check(n == 0 and state6 == old and len(warnings) == 1, "unreachable warns")
    # A DTD in an appcast is refused.
    try:
        parse_appcast(b'<!DOCTYPE x [<!ENTITY a "b">]><rss/>', app)
        check(False, "DTD refused")
    except FeedError:
        pass

    for failure in failures:
        print(f"tool_versions selftest FAILED: {failure}", file=sys.stderr)
    if not failures:
        print("tool_versions selftest: ok")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
