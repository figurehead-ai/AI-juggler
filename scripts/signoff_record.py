#!/usr/bin/env python3
"""D-109 (AJ-E807): a release's sign-off is the maintainer's approval of the
`stable` environment in the Promote workflow. This writes its record.

  signoff_record.py record --repo OWNER/NAME --run ID --version X.Y.Z
                           --sha SHA --scope scope.json --scope-sha SHA --out DIR
      promote.yml's `record` job, once the channel has moved: reads the
      run's environment approvals and jobs through `gh api` and writes
      DIR/signoff-X.Y.Z.md. scope.json is what the run's own check wrote
      (`release_scope.py check X.Y.Z --channel stable --json`), at SCOPE_SHA.
      Exits 1 when the run holds no approval of the environment.
  signoff_record.py rehearse --repo OWNER/NAME --run ID --version X.Y.Z
                             [--environment beta] --out DIR
      The dry run: the same record from a past Promote run, marked as a
      rehearsal. The scope is checked here, at HEAD; the commit is the
      version's tag. It reads only: nothing is promoted, uploaded or
      committed.
  signoff_record.py pull X.Y.Z [--repo OWNER/NAME]
      Copies the record attached to release vX.Y.Z into
      docs/qa/signoffs/X.Y.Z.md, to commit on the release branch.
  signoff_record.py --selftest

The record holds the version, the commit, the approving reviewer and the
approval's comment, when it was approved and recorded, the run, the
roadmap scope's result with a link to each task's evidence manifest (at
the commit the scope was checked on), and the review inputs AJ-E807 R2
names: the accessibility gate (the run's), the visual gallery delta since
the previous Stable, the measured performance table (AJ-E805) and the
unresolved items.

Why a release attachment and not a pull request: the repository doesn't
let GitHub Actions open pull requests ("Allow GitHub Actions to create and
approve pull requests" is off), and one opened with the workflow's token
would start no CI. The record goes on the release and in the run's
summary; `pull` brings it into the repository through the normal route.
"""
from __future__ import annotations

import argparse
import contextlib
import datetime as dt
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SIGNOFFS = "docs/qa/signoffs"
GALLERY = "docs/qa/evidence/_gallery"
VERSION = re.compile(r"^\d+\.\d+\.\d+$")


def server() -> str:
    return os.environ.get("GITHUB_SERVER_URL", "https://github.com")


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def gh_json(path: str) -> object:
    out = subprocess.run(["gh", "api", path], capture_output=True, text=True, check=True).stdout
    return json.loads(out)


def approvers(approvals: list[dict], environment: str) -> list[dict]:
    """The run's approvals of `environment` (a rejection never promotes)."""
    return [
        a for a in approvals
        if a.get("state") == "approved" and any(e.get("name") == environment for e in a.get("environments", []))
    ]


def approved_at(jobs: list[dict], environment: str) -> str | None:
    """When the approved job started: GitHub starts it on the approval."""
    for job in jobs:
        if job.get("name") == f"promote to {environment}" and job.get("started_at"):
            return job["started_at"]
    return None


def gallery_delta(sha: str, version: str, blob: str) -> str:
    """Captures at `sha`, and how many changed since the previous Stable."""
    pngs = [p for p in git("ls-tree", "-r", "--name-only", sha, "--", GALLERY).splitlines() if p.endswith(".png")]
    index = f"[the gallery]({blob}/{GALLERY}/index.html)"
    previous = [
        tag for tag in git("tag", "-l", "stable/v*").splitlines()
        if VERSION.match(tag.removeprefix("stable/v")) and tag != f"stable/v{version}"
    ]
    previous.sort(key=lambda tag: tuple(int(part) for part in tag.removeprefix("stable/v").split(".")))
    if not previous:
        return f"{len(pngs)} captures; no earlier Stable to compare with, so review {index} whole."
    tag = previous[-1]
    since = git("rev-parse", f"{tag}^{{commit}}")
    changed = git("diff", "--name-only", f"{since}..{sha}", "--", GALLERY).splitlines()
    return f"{len(pngs)} captures; {len(changed)} changed since {tag} (`{since[:10]}`): {index}."


def has_manifest(scope_sha: str, task: str) -> bool:
    path = f"docs/qa/evidence/{task}/manifest.md"
    return subprocess.run(["git", "cat-file", "-e", f"{scope_sha}:{path}"], cwd=ROOT, capture_output=True).returncode == 0


def render(*, version: str, sha: str, environment: str, repo: str, run_id: str, approvals: list[dict],
           jobs: list[dict], scope: dict, scope_sha: str, now: str, gallery: str, manifest=has_manifest,
           rehearsal: str | None = None) -> str:
    """The record, as Markdown. Raises SystemExit without an approval."""
    run_url = f"{server()}/{repo}/actions/runs/{run_id}"
    blob = f"{server()}/{repo}/blob/{scope_sha}"
    approved = approvers(approvals, environment)
    if not approved:
        raise SystemExit(f"signoff_record: run {run_id} holds no approval of the `{environment}` environment; "
                         "without one there is no sign-off to record")
    reviewers = ", ".join(f"`{a['user']['login']}`" for a in approved)
    comments = "; ".join(a["comment"].replace("|", "/").replace("\n", " ") for a in approved if a.get("comment")) or "none"
    tasks = scope.get("tasks", [])
    counts = {state: sum(1 for t in tasks if t["state"] == state) for state in ("proven", "deferred", "open", "unproven", "unknown")}
    verdict = "complete" if scope.get("exit") == 0 else f"NOT complete (exit {scope.get('exit')})"
    rows = []
    for t in tasks:
        state = t["state"] + (f" ({t['decision']})" if t.get("decision") else "")
        link = (f"[manifest]({blob}/docs/qa/evidence/{t['id']}/manifest.md)"
                if t["state"] != "unknown" and manifest(scope_sha, t["id"]) else "no manifest")
        title = t.get("title", "").replace("|", "/")
        rows.append(f"| {t['id']} {title} | {state} | {link} |")
    unresolved = [line.strip() for line in scope.get("output", [])[1:] if not line.startswith("release_scope:")]
    lines = [f"# Sign-off: AI Juggler {version} on {environment.capitalize()}", ""]
    if rehearsal:
        lines += [f"> **Rehearsal.** {rehearsal} Nothing was promoted, uploaded or committed.", ""]
    lines += [
        "D-109: the sign-off is the maintainer's approval of the "
        f"`{environment}` environment in the Promote workflow; this record was written from that run.",
        "",
        "| Field | Value |",
        "|---|---|",
        f"| Version | {version} |",
        f"| Commit | [`{sha}`]({server()}/{repo}/commit/{sha}) |",
        f"| Approved by | {reviewers} (GitHub `{environment}` environment) |",
        f"| Approval comment | {comments} |",
        f"| Approved at | {approved_at(jobs, environment) or 'not in the run'} (the approved job's start) |",
        f"| Recorded at | {now} |",
        f"| Run | {run_url} |",
        f"| Roadmap scope | {verdict}: {counts['proven']} done and proven, {counts['deferred']} deferred, "
        f"{counts['open']} open, {counts['unproven']} unproven, {counts['unknown']} unknown; checked at `{scope_sha[:10]}` |",
        "",
        f"## Roadmap scope (`release_scope.py check {version} --channel stable`)",
        "",
        "```text",
        *scope.get("output", []),
        "```",
        "",
        "| Task | State | Evidence |",
        "|---|---|---|",
        *rows,
        "",
        "## Review inputs (AJ-E807 R2)",
        "",
        f"- **Accessibility (AJ-E804):** the run's accessibility gate passed on this commit before the approval ({run_url}).",
        f"- **Visual gallery:** {gallery}",
        f"- **Performance, measured (AJ-E805):** [the measured budgets]({blob}/docs/qa/evidence/AJ-E805/manifest.md#measured-budgets).",
        "- **Unresolved:**" + ("" if unresolved else " none."),
        *[f"  - {line}" for line in unresolved],
        "",
    ]
    return "\n".join(lines)


def write(out: Path, version: str, text: str) -> Path:
    out.mkdir(parents=True, exist_ok=True)
    target = out / f"signoff-{version}.md"
    target.write_text(text, encoding="utf-8")
    return target


def now_utc() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def fetch(repo: str, run: str) -> tuple[list, list]:
    approvals = gh_json(f"repos/{repo}/actions/runs/{run}/approvals")
    jobs = gh_json(f"repos/{repo}/actions/runs/{run}/jobs?per_page=100")["jobs"]
    return approvals, jobs


def record(a: argparse.Namespace) -> int:
    approvals, jobs = fetch(a.repo, a.run)
    scope = json.loads(Path(a.scope).read_text(encoding="utf-8"))
    text = render(version=a.version, sha=a.sha, environment=a.environment, repo=a.repo, run_id=a.run,
                  approvals=approvals, jobs=jobs, scope=scope, scope_sha=a.scope_sha, now=now_utc(),
                  gallery=gallery_delta(a.sha, a.version, f"{server()}/{a.repo}/blob/{a.scope_sha}"))
    print(write(Path(a.out), a.version, text))
    return 0


def rehearse(a: argparse.Namespace) -> int:
    import release_scope

    sha = git("rev-parse", f"v{a.version}^{{commit}}")
    scope_sha = git("rev-parse", "HEAD")
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "scope.json"
        with contextlib.redirect_stdout(io.StringIO()):
            release_scope.run(["check", a.version, "--channel", "stable", "--json", str(path)])
        scope = json.loads(path.read_text(encoding="utf-8"))
    approvals, jobs = fetch(a.repo, a.run)
    text = render(version=a.version, sha=sha, environment=a.environment, repo=a.repo, run_id=a.run,
                  approvals=approvals, jobs=jobs, scope=scope, scope_sha=scope_sha, now=now_utc(),
                  gallery=gallery_delta(sha, a.version, f"{server()}/{a.repo}/blob/{scope_sha}"),
                  rehearsal=f"Built by `signoff_record.py rehearse` from Promote run {a.run}'s "
                            f"`{a.environment}` approval, with the scope checked at HEAD.")
    print(write(Path(a.out), a.version, text))
    return 0


def pull(a: argparse.Namespace) -> int:
    with tempfile.TemporaryDirectory() as tmp:
        cmd = ["gh", "release", "download", f"v{a.version}", "--pattern", f"signoff-{a.version}.md", "--dir", tmp]
        subprocess.run(cmd + (["--repo", a.repo] if a.repo else []), check=True)
        target = ROOT / SIGNOFFS / f"{a.version}.md"
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(Path(tmp) / f"signoff-{a.version}.md", target)
    print(target.relative_to(ROOT))
    return 0


def selftest() -> int:
    """The record from made-up approvals (placeholder login), no network."""
    scope = {
        "exit": 0,
        "output": ["release scope 9.9.1 (stable: the whole 9.9): 2 tasks named, 1 done and proven, "
                   "0 marked done but unproven, 1 deferred, 0 open", "  deferred  AJ-E8 (D-1)"],
        "tasks": [
            {"id": "AJ-E7", "title": "Made up | piped", "status": "DONE-EVIDENCED", "state": "proven"},
            {"id": "AJ-E8", "title": "Moved", "status": "TODO", "state": "deferred", "decision": "D-1"},
        ],
    }
    user = {"login": "octocat", "type": "User"}
    approvals = [
        {"state": "approved", "comment": "ship it", "user": user, "environments": [{"name": "stable"}]},
        {"state": "approved", "comment": "", "user": {"login": "hubot"}, "environments": [{"name": "beta"}]},
    ]
    jobs = [{"name": "check", "started_at": "2026-01-02T03:00:00Z"},
            {"name": "promote to stable", "started_at": "2026-01-02T03:04:05Z"}]
    base = dict(version="9.9.1", sha="a" * 40, environment="stable", repo="octo-org/octo-repo", run_id="42",
                jobs=jobs, scope=scope, scope_sha="b" * 40, now="2026-01-02T03:05:00Z",
                gallery="3 captures; no earlier Stable.", manifest=lambda sha, task: task == "AJ-E7")
    text = render(approvals=approvals, **base)
    for want in ("# Sign-off: AI Juggler 9.9.1 on Stable", "| Version | 9.9.1 |", f"`{'a' * 40}`",
                 "| Approved by | `octocat` (GitHub `stable` environment) |", "| Approval comment | ship it |",
                 "| Approved at | 2026-01-02T03:04:05Z", "| Recorded at | 2026-01-02T03:05:00Z |",
                 "/octo-org/octo-repo/actions/runs/42", "| Roadmap scope | complete: 1 done and proven, 1 deferred",
                 f"| AJ-E7 Made up / piped | proven | [manifest](https://github.com/octo-org/octo-repo/blob/{'b' * 40}/docs/qa/evidence/AJ-E7/manifest.md) |",
                 "| AJ-E8 Moved | deferred (D-1) | no manifest |", "  - deferred  AJ-E8 (D-1)",
                 "**Visual gallery:** 3 captures", "AJ-E805/manifest.md#measured-budgets"):
        assert want in text, (want, text)
    assert "hubot" not in text and "Rehearsal" not in text, text
    # No approval of the environment (only beta's, or a rejection): no record.
    for bad in ([approvals[1]], [{**approvals[0], "state": "rejected"}], []):
        try:
            render(approvals=bad, **base)
        except SystemExit as stop:
            assert "no approval of the `stable` environment" in str(stop), stop
        else:
            raise AssertionError(f"recorded without an approval: {bad}")
    # A scope that failed says so; a rehearsal says nothing moved.
    text = render(approvals=approvals, **{**base, "scope": {**scope, "exit": 1}}, rehearsal="From run 42.")
    assert "| Roadmap scope | NOT complete (exit 1)" in text and "> **Rehearsal.** From run 42." in text, text
    assert approved_at([], "stable") is None
    print("signoff-record selftest: OK")
    return 0


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("record", "rehearse"):
        p = sub.add_parser(name)
        p.add_argument("--repo", required=True)
        p.add_argument("--run", required=True)
        p.add_argument("--version", required=True)
        p.add_argument("--environment", default="stable")
        p.add_argument("--out", required=True)
        if name == "record":
            p.add_argument("--sha", required=True)
            p.add_argument("--scope", required=True)
            p.add_argument("--scope-sha", required=True)
    p = sub.add_parser("pull")
    p.add_argument("version")
    p.add_argument("--repo")
    a = parser.parse_args()
    if not VERSION.match(a.version):
        parser.error(f"{a.version} is not X.Y.Z")
    return {"record": record, "rehearse": rehearse, "pull": pull}[a.command](a)


if __name__ == "__main__":
    sys.exit(main())
