#!/usr/bin/env python3
"""Stage "this session asks you" on a LAB session, for walks and persona runs.

It goes through the product's own path: `juggler-agent hook Notification`
with a permission-prompt notification, run in that session's own hook
environment (its socket, its per-session report key and the agent program
AI Juggler gave it), read from the session's process. AI Juggler then marks
the session "Asks you" exactly as a real prompt would.

Lab sessions only: the process must run under the lab's HOME
(`/tmp/aij-lab…`), and nothing outside it is read or written.

Usage: scripts/qa/lab_ask.py <session pid> <lab HOME>
       (the pid from launch_session's answer, or the lab's process list)

A launched session carries the hook environment only when the app trusts
its `juggler-agent`: beside the app's own binary, in folders only you can
write (`/tmp` isn't one), and with the pool page's "Let agents … work
together?" answered (the lab route `agent_bridge_answer`).
"""
from __future__ import annotations

import json
import re
import subprocess
import sys

NAMES = ("JUGGLER_AGENT", "JUGGLER_AGENT_SOCKET", "JUGGLER_REPORT_TOKEN")


def hook_env(command: str) -> dict[str, str]:
    """The hook variables in `ps eww` output (values may hold spaces)."""
    found = {}
    for name in NAMES:
        m = re.search(rf"(?:^| ){name}=(.+?)(?= [A-Za-z_][A-Za-z0-9_]*=|$)", command)
        if m:
            found[name] = m.group(1)
    return found


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__)
        return 2
    pid, lab = argv
    if not re.fullmatch(r"/tmp/aij-lab[\w.-]*", lab) or not pid.isdigit():
        print("lab_ask: give a session's pid and a lab HOME under /tmp/aij-lab…", file=sys.stderr)
        return 2
    command = subprocess.run(["ps", "eww", "-o", "command=", "-p", pid], capture_output=True, text=True).stdout
    if f"HOME={lab} " not in command and f"HOME={lab}\n" not in command:
        print("lab_ask: that process doesn't run in the lab", file=sys.stderr)
        return 1
    env = hook_env(command)
    if len(env) != len(NAMES):
        print(f"lab_ask: no hook environment in that process (has {sorted(env)}); see this script's notes", file=sys.stderr)
        return 1
    payload = {"hook_event_name": "Notification", "notification_type": "permission_prompt", "message": "Claude needs your permission to use Bash"}
    out = subprocess.run(
        [env["JUGGLER_AGENT"], "hook", "Notification"],
        input=json.dumps(payload),
        text=True,
        capture_output=True,
        env={"PATH": "/usr/bin:/bin", "HOME": lab, "JUGGLER_AGENT_SOCKET": env["JUGGLER_AGENT_SOCKET"], "JUGGLER_REPORT_TOKEN": env["JUGGLER_REPORT_TOKEN"]},
    )
    if out.returncode != 0:
        print(f"lab_ask: the hook said {out.returncode}: {out.stderr.strip()[:200]}", file=sys.stderr)
        return 1
    print(f"lab_ask: session {pid} now asks you")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
