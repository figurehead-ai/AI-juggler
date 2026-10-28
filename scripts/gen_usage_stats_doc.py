#!/usr/bin/env python3
"""AJ-E1106: generates docs/usage-stats.md from stats/schema.v1.json.

The metric table is generated, so the public document can never drift
from what the app is able to send. `--check` exits non-zero when the
committed document differs (run by `just verify`).

Usage: scripts/gen_usage_stats_doc.py [--check]
"""
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCHEMA = ROOT / "stats" / "schema.v1.json"
DOC = ROOT / "docs" / "usage-stats.md"

FAMILIES = [
    ("accounts", "Environment"), ("provider", "Environment"), ("launch_settings", "Environment"),
    ("projects", "Environment"), ("fleet", "Fleet"), ("cleanup", "Processes and cleanup"),
    ("auto_restart", "Environment"), ("pack", "Switching"), ("renewal_dates", "Environment"),
    ("worktree", "Environment"), ("update", "Reliability"), ("ui", "Interface"),
    ("crash_reports", "Environment"), ("instance", "Instances and sessions"),
    ("session", "Instances and sessions"), ("switch", "Switching"), ("context_sync", "Switching"),
    ("usage", "Usage clocks"), ("renewal", "Usage clocks"), ("process", "Processes and cleanup"),
    ("leftover", "Processes and cleanup"), ("resource", "Processes and cleanup"),
    ("cleanup_centre", "Processes and cleanup"), ("onboarding", "Onboarding"),
    ("app", "Reliability"), ("error", "Reliability"), ("storage", "Reliability"),
    ("stats", "Reliability"), ("perf", "Performance"),
]
ORDER = ["Environment", "Instances and sessions", "Switching", "Usage clocks",
         "Processes and cleanup", "Onboarding", "Fleet", "Interface", "Reliability", "Performance"]


def family(metric: str) -> str:
    head = metric.split(".")[0]
    for prefix, name in FAMILIES:
        if head == prefix:
            return name
    return "Other"


def values(schema: dict, spec) -> str:
    if isinstance(spec, list):
        return " · ".join(f"`{v}`" for v in spec)
    if isinstance(spec, str) and spec in schema["enums"]:
        return " · ".join(f"`{v}`" for v in schema["enums"][spec])
    return f"`{spec}`"


def render(schema: dict) -> str:
    groups: dict = {}
    for name, metric in schema["metrics"].items():
        groups.setdefault(family(name), []).append((name, metric))
    lines = []
    for group in ORDER + sorted(set(groups) - set(ORDER)):
        if group not in groups:
            continue
        lines.append(f"### {group}\n")
        lines.append("| Metric | Kind | Dimensions | Value |")
        lines.append("|---|---|---|---|")
        for name, metric in groups[group]:
            kind = metric["type"]
            dims = "; ".join(f"`{d}`: {values(schema, v)}" for d, v in metric.get("dims", {}).items()) or "—"
            if kind == "counter":
                value = f"a weekly count (at most {metric.get('max', schema['limits']['counter_max']):,})"
            elif kind == "histogram":
                value = "counts per bucket: " + " · ".join(f"`{b}`" for b in metric["buckets"])
            elif "values" in metric:
                value = values(schema, metric["values"])
            else:
                value = "`major.minor` of the CLI, or `other`"
            lines.append(f"| `{name}` | {kind} | {dims} | {value} |")
        lines.append("")
    return "\n".join(lines)


HEADER = """# Usage statistics

AI Juggler can tell its maintainers which features people use. It does
this only if you choose to, and never by learning anything about your
work. This page lists **everything** it can ever send. The table is
generated from the schema the app and the server both enforce
(`stats/schema.v1.json`), so it can't drift from the code.

## The three modes

| Mode | What happens |
|---|---|
| **Off** (the default) | Nothing is counted, nothing is stored, and no request is made. There is no stats folder. |
| **Keep stats on this computer only** | AI Juggler counts which features you use so you can see it in Settings ▸ Privacy. Nothing is sent. |
| **Share weekly usage stats** | Once a week, AI Juggler sends one small report of those counts. |

You choose in onboarding (no option is preselected) or at any time in
Settings ▸ Privacy. Switching to Off deletes everything stored locally.

## How a report works

- Counting happens on your Mac. Only weekly totals exist; individual
  actions are never stored or sent.
- One report covers one ISO week (UTC). It is sent at a random time in
  the 6 days after the week ends, so reports from many Macs spread out.
- A week is sent only if you were sharing for all of it. Data counted
  before you opted in is never sent.
- A report carries **no identifier**: no user, account, machine or
  install id. Its random report id is used once, to drop duplicates, so
  two reports from the same Mac can't be linked.
- Counts are rounded (for example, Account counts go into buckets like
  `4-5`; token totals keep two significant figures). Durations and sizes
  are only buckets.
- You can read every report before it is sent, as the exact JSON, in
  Settings ▸ Privacy.
- The service keeps raw reports for 35 days and only aggregates after
  that. Because a report has no id, a report that was already sent can't
  be found and deleted later.

## The one number we watch

AI Juggler is there so you can keep working when a limit stops you. The
number that says whether it does is the **juggling loop**: a session
reached its usage limit, and its work went on, on another account or
tool.

In a report, that's `switch.completed` with `trigger` `quota_wall` and
`outcome` `success`. A switch away from a session at its limit counts as
`quota_wall` however you opened the switch screen. It's counted per week,
like everything else. With **Keep stats on this computer only**, you can see
your own count in Settings ▸ Privacy.

## Never collected

- Prompts, responses, transcripts, code, diffs, file or folder names,
  paths, git remotes or branch names.
- Project names, Account labels, Launch-settings names, machine names,
  host names, user names, emails.
- Anything read from, derived from or hashed from a vendor's credential
  or identity files, including the account ids in `~/.claude.json` and
  `~/.codex/auth.json`.
- Environment variables, setting values beyond the choices listed
  below, command lines, ports, process ids.
- Error messages, stack traces and log lines. Errors are only a fixed
  code from the list below.
- IP addresses (the service doesn't read or store them), and identifiers
  of any kind.

The app enforces this in its types: every value in a report is one of
the choices listed below or a bounded number, so free text can't be
sent. Tests check it on every build.

## Turning it off, or keeping it out entirely

- **Settings ▸ Privacy ▸ Off.** This also deletes local stats.
- **`DO_NOT_TRACK=1`** in your environment forces it off, along with
  crash reports.
- **`AI_JUGGLER_STATS=off`** forces it off; **`AI_JUGGLER_STATS=local`**
  never lets it go beyond local-only.
- **Builds from source:** `cargo build -p juggler-app --no-default-features`
  leaves usage statistics out of the app entirely.

Debug builds and CI never upload.

## Showing your own stats on figurehead.ai (optional)

If you share usage stats **and** this Mac is linked to a Figurehead
account, Settings ▸ Privacy offers "Show my stats in my Figurehead
dashboard". With it ticked, the weekly report goes to your account
(where only you can see it, and it is deleted with your account)
**instead of** the anonymous upload, never as well.

## Crash reports

Crash reports have their own setting: never (the default), ask (you see
the report and choose), or always. They are scrubbed on your Mac first:
your home folder becomes `~`, and no user name, environment or URLs are
included.

## A sample report

```json
{
  "schema": 1,
  "report_id": "0b6f2d4e-6a3c-4f0e-9d7b-3c1e5a9f8b21",
  "week": "2026-W42",
  "sample_rate": 1.0,
  "app": { "version": "0.3.1", "channel": "beta" },
  "os": { "name": "macos", "major": 15, "arch": "arm64" },
  "install_age": "1-4w",
  "days_active": 5,
  "consent_version": 1,
  "entries": [
    { "m": "instance.started", "d": { "provider": "claude-code", "kind": "terminal", "origin": "new" }, "v": 14 },
    { "m": "switch.duration_s", "d": { "kind": "provider" }, "h": [0, 2, 1, 0] },
    { "m": "accounts.configured", "d": { "provider": "claude-code" }, "g": "2" }
  ]
}
```

## Every metric

Each report carries the app version and channel, the OS name, major
version and chip architecture, how long ago AI Juggler was installed (as
a bucket), how many days that week it ran (0–7), the consent version and
the sampling rate. The entries below are the only other things it can
contain.

"""


def main() -> int:
    schema = json.loads(SCHEMA.read_text())
    text = HEADER + render(schema)
    if "--check" in sys.argv:
        if not DOC.exists() or DOC.read_text() != text:
            print("usage-stats-doc: docs/usage-stats.md is out of date — run scripts/gen_usage_stats_doc.py", file=sys.stderr)
            return 1
        print("usage-stats-doc: up to date")
        return 0
    DOC.write_text(text)
    print(f"usage-stats-doc: wrote {DOC.relative_to(ROOT)} ({len(schema['metrics'])} metrics)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
