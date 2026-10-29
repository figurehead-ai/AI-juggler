#!/usr/bin/env python3
"""AJ-E2303 (D-40): the Codex app-server surface, recorded per version.

AI Juggler pins the app-server protocol it speaks. Each release, generate
the schema from the installed Codex OFFLINE, with a scratch home (no
sign-in, no network, nothing of yours read):

    scratch=$(mktemp -d)
    env -i HOME="$scratch" PATH=/usr/bin:/bin:/opt/homebrew/bin \
      CODEX_HOME="$scratch/codex" codex app-server generate-json-schema --out "$scratch/schema"
    scripts/codex_app_server_surface.py "$scratch/schema" \
      > crates/juggler-adapters/tests/fixtures/codex-app-server/surface-<version>.json

This writes every method's name, kind and required parameter fields, and
the enum values of the answers AI Juggler sends; the adapter's tests check
that every method and value the codec uses is in it
(`codex_appserver::surface`). Only names and shapes are kept, never the
schema's descriptions.

Usage: codex_app_server_surface.py SCHEMA_DIR
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

KINDS = {
    "ClientRequest.json": "client_request",
    "ServerRequest.json": "server_request",
    "ServerNotification.json": "notification",
    "ClientNotification.json": "client_notification",
}
# The answers AI Juggler sends: their allowed values are pinned too.
ENUMS = (
    "CommandExecutionApprovalDecision",
    "FileChangeApprovalDecision",
    "PermissionGrantScope",
    "McpServerElicitationAction",
    "TurnStatus",
    "ThreadStatus",
    "AskForApproval",
)


def definitions(schema: Path) -> dict:
    defs: dict = {}
    for path in sorted(schema.glob("*.json")):
        doc = json.loads(path.read_text())
        for key in ("definitions", "$defs"):
            for name, body in (doc.get(key) or {}).items():
                defs.setdefault(name, body)
        if doc.get("title"):
            defs.setdefault(doc["title"], doc)
    return defs


def values(node: dict) -> list[str]:
    """The string values or object keys a decision-like enum allows."""
    out: list[str] = []
    for variant in node.get("oneOf", node.get("anyOf", [])):
        if "enum" in variant:
            out += [str(v) for v in variant["enum"]]
        elif variant.get("type") == "object":
            props = variant.get("properties", {})
            tag = props.get("type", {}).get("enum")
            out += [str(v) for v in tag] if tag else list(props)
    if "enum" in node:
        out += [str(v) for v in node["enum"]]
    return sorted(set(out))


def main(schema: Path) -> int:
    defs = definitions(schema)
    methods = []
    for file, kind in KINDS.items():
        path = schema / file
        if not path.exists():
            continue
        for variant in json.loads(path.read_text()).get("oneOf", []):
            props = variant.get("properties", {})
            method = (props.get("method", {}).get("enum") or [None])[0]
            if not method:
                continue
            ref = props.get("params", {}).get("$ref", "").split("/")[-1]
            required = sorted(defs.get(ref, {}).get("required", [])) if ref else []
            methods.append({"method": method, "kind": kind, "params": ref or None, "required": required})
    out = {
        "methods": sorted(methods, key=lambda m: (m["kind"], m["method"])),
        "enums": {name: values(defs[name]) for name in ENUMS if name in defs},
    }
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(Path(sys.argv[1])))
