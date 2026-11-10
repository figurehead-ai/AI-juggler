#!/usr/bin/env python3
"""AJ-E2008: the fuzz package builds no app.

The fuzz targets read through light crates only (backup_staging through
juggler-core's archive reader). One dependency on juggler-app put GTK and
an instrumented release build of the whole Tauri stack under every target,
and the nightly fuzz jobs ran out of time building it (#218). This reads
the fuzz package's dependency tree, for every target platform, and fails
naming each Tauri or GTK crate in it.

Usage: fuzz_light.py [--selftest]
"""
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "fuzz" / "Cargo.toml"
HEAVY = re.compile(r"tauri|gtk", re.IGNORECASE)


def heavy(tree: str) -> list[str]:
    """The Tauri or GTK crates in `cargo tree --prefix none` output."""
    names = {line.split()[0] for line in tree.splitlines() if line.strip()}
    return sorted(name for name in names if HEAVY.search(name))


def main() -> int:
    tree = subprocess.run(
        [
            "cargo", "tree", "--manifest-path", str(MANIFEST),
            "--target", "all", "-e", "normal,build", "--prefix", "none",
        ],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    if not tree.strip():
        print("fuzz_light: cargo tree printed nothing", file=sys.stderr)
        return 1
    found = heavy(tree)
    for name in found:
        print(f"fuzz/Cargo.toml: {name} is in the fuzz build; depend on light crates only "
              "(no juggler-app, Tauri or GTK)", file=sys.stderr)
    return 1 if found else 0


def selftest() -> int:
    tree = "\n".join([
        "juggler-fuzz v0.0.0 (/r/fuzz)",
        "juggler-core v0.4.0 (/r/crates/juggler-core)",
        "tar v0.4.46",
        "tauri-runtime v2.12.2",
        "webkit2gtk-sys v2.0.1",
        "gtk v0.18.2 (*)",
        "",
    ])
    assert heavy(tree) == ["gtk", "tauri-runtime", "webkit2gtk-sys"], heavy(tree)
    assert heavy("juggler-core v0.4.0\nflate2 v1.1.10\n") == []
    print("fuzz_light selftest: ok")
    return 0


if __name__ == "__main__":
    sys.exit(selftest() if "--selftest" in sys.argv[1:] else main())
