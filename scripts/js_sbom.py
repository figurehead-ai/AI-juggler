#!/usr/bin/env python3
"""AJ-E2007 R5: a CycloneDX 1.5 SBOM of the UI's shipped JavaScript.

pnpm 10 has no SBOM command, and a generator from npm would itself be a
new dependency in the release job, so this reads pnpm's own list of the
production dependencies (`pnpm list --prod --json --depth Infinity`) and
writes each package once, with its npm purl, its registry tarball and
what it depends on.

Usage: js_sbom.py <version> <out.json> [--selftest]
"""
from __future__ import annotations

import datetime
import json
import pathlib
import subprocess
import sys
import uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent


def purl(name: str, version: str) -> str:
    # pkg:npm/%40scope/name@version (the scope's @ is percent-encoded).
    return f"pkg:npm/{name.replace('@', '%40', 1) if name.startswith('@') else name}@{version}"


def bom(tree: dict, version: str, now: str, serial: str) -> dict:
    components: dict[str, dict] = {}
    edges: dict[str, set[str]] = {}

    def walk(deps: dict, parent: str) -> None:
        for name, dep in sorted((deps or {}).items()):
            ref = purl(name, dep["version"])
            edges.setdefault(parent, set()).add(ref)
            if ref in components:
                continue
            component = {"type": "library", "bom-ref": ref, "name": name, "version": dep["version"], "purl": ref}
            if dep.get("resolved"):
                component["externalReferences"] = [{"type": "distribution", "url": dep["resolved"]}]
            components[ref] = component
            walk(dep.get("dependencies"), ref)

    root = f"pkg:npm/{tree['name']}@{version}"
    walk(tree.get("dependencies"), root)
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": serial,
        "version": 1,
        "metadata": {
            "timestamp": now,
            "tools": {"components": [{"type": "application", "name": "scripts/js_sbom.py"}]},
            "component": {"type": "application", "bom-ref": root, "name": tree["name"], "version": version},
        },
        "components": [components[key] for key in sorted(components)],
        "dependencies": [{"ref": ref, "dependsOn": sorted(on)} for ref, on in sorted(edges.items())],
    }


def selftest() -> None:
    tree = {"name": "ui", "dependencies": {
        "@scope/a": {"version": "1.0.0", "resolved": "https://registry.npmjs.org/a.tgz",
                     "dependencies": {"b": {"version": "2.0.0"}}},
        "b": {"version": "2.0.0"},
    }}
    out = bom(tree, "0.4.1", "t", "urn:uuid:x")
    refs = [c["purl"] for c in out["components"]]
    assert refs == ["pkg:npm/%40scope/a@1.0.0", "pkg:npm/b@2.0.0"], refs
    assert {"ref": "pkg:npm/%40scope/a@1.0.0", "dependsOn": ["pkg:npm/b@2.0.0"]} in out["dependencies"]
    print("js_sbom: selftest ok")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    version, out = sys.argv[1], pathlib.Path(sys.argv[2])
    listed = subprocess.run(
        ["pnpm", "--dir", str(ROOT / "ui"), "list", "--prod", "--json", "--depth", "Infinity"],
        check=True, capture_output=True, text=True,
    )
    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    doc = bom(json.loads(listed.stdout)[0], version, now, f"urn:uuid:{uuid.uuid4()}")
    out.write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    print(f"js_sbom: {len(doc['components'])} packages → {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
