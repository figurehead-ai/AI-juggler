#!/usr/bin/env python3
"""AJ-E1905 (D-73): the Claude Code plugin, packaged with each release.

  plugin_package.py X.Y.Z OUTDIR    write OUTDIR/AI-Juggler-Claude-Code-plugin-X.Y.Z.zip
  plugin_package.py --selftest

The zip holds `plugins/claude-code/ai-juggler` as Claude Code loads it
(`claude --plugin-dir <zip>`), its `plugin.json` carrying the release's
version, so the plugin a release offers says which AI Juggler it came
with. The marketplace at the repository root (`.claude-plugin/`) serves
the same folder; `release_version.py stamp` writes the same version there
in the build's checkout.

The zip is reproducible: entries sorted, one fixed timestamp, fixed
permissions (the launcher executable), so the release's SHA256SUMS and
provenance name the same bytes on every run for one commit and version.
Nothing outside the plugin's folder goes in, and a file holding a key,
the bridge's socket, an address or a user's folder stops the packaging.
"""
from __future__ import annotations

import json
import re
import sys
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLUGIN = Path("plugins/claude-code/ai-juggler")
NAME = "AI-Juggler-Claude-Code-plugin-{version}.zip"
# 1980-01-01, the earliest time a zip entry can carry.
FIXED_TIME = (1980, 1, 1, 0, 0, 0)
# What must never be in a published plugin (the plugin test checks the
# same in the repository's copy).
FORBIDDEN = ("JUGGLER_AGENT_TOKEN=", "bridge.sock", "@", "/Users/", "CLAUDE_CONFIG_DIR")


def files(folder: Path) -> list[Path]:
    return sorted(p for p in folder.rglob("*") if p.is_file())


def package(root: Path, version: str, out: Path) -> Path:
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise SystemExit(f"not a release version: {version!r}")
    folder = root / PLUGIN
    manifest = folder / ".claude-plugin/plugin.json"
    if not manifest.is_file():
        raise SystemExit(f"no plugin manifest at {PLUGIN}/.claude-plugin/plugin.json")
    out.mkdir(parents=True, exist_ok=True)
    target = out / NAME.format(version=version)
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in files(folder):
            rel = path.relative_to(folder).as_posix()
            if path == manifest:
                plugin = json.loads(path.read_text(encoding="utf-8"))
                plugin["version"] = version
                data = (json.dumps(plugin, indent=2) + "\n").encode()
            else:
                data = path.read_bytes()
            text = data.decode("utf-8", errors="replace")
            for word in FORBIDDEN:
                if word in text:
                    target.unlink(missing_ok=True)
                    raise SystemExit(f"{PLUGIN}/{rel} holds {word!r}: not published")
            info = zipfile.ZipInfo(rel, FIXED_TIME)
            info.compress_type = zipfile.ZIP_DEFLATED
            mode = 0o755 if rel.startswith("bin/") else 0o644
            info.external_attr = (0o100000 | mode) << 16
            archive.writestr(info, data)
    return target


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "out"
        first = package(ROOT, "0.5.3", out)
        assert first.name == "AI-Juggler-Claude-Code-plugin-0.5.3.zip", first.name
        with zipfile.ZipFile(first) as archive:
            names = archive.namelist()
            assert names == sorted(names), "entries in a fixed order"
            assert ".claude-plugin/plugin.json" in names and ".mcp.json" in names, names
            assert "bin/juggler-mcp" in names and "skills/pool-tasks/SKILL.md" in names, names
            assert not any(n.startswith(("plugins/", "/", "..")) for n in names), "the plugin's own folder only"
            plugin = json.loads(archive.read(".claude-plugin/plugin.json"))
            assert plugin["version"] == "0.5.3" and plugin["name"] == "ai-juggler", plugin
            launcher = archive.getinfo("bin/juggler-mcp")
            assert (launcher.external_attr >> 16) & 0o777 == 0o755, "the launcher stays executable"
            assert all(i.date_time == FIXED_TIME for i in archive.infolist())
        again = package(ROOT, "0.5.3", Path(tmp) / "again")
        assert first.read_bytes() == again.read_bytes(), "reproducible: same bytes for one commit and version"
        for bad in ("0.5", "v0.5.3", "0.5.3-beta"):
            try:
                package(ROOT, bad, out)
            except SystemExit:
                continue
            raise AssertionError(f"accepted {bad!r}")
        # A plugin that holds an address isn't published.
        fake = Path(tmp) / "fake"
        (fake / PLUGIN / ".claude-plugin").mkdir(parents=True)
        (fake / PLUGIN / ".claude-plugin/plugin.json").write_text('{"name": "ai-juggler", "version": "0.0.0"}')
        (fake / PLUGIN / "notes.md").write_text("ask someone@example.com")
        try:
            package(fake, "0.5.3", out / "bad")
        except SystemExit as stop:
            assert "notes.md" in str(stop), stop
            assert not (out / "bad" / NAME.format(version="0.5.3")).exists(), "no half-written zip left"
        else:
            raise AssertionError("published a plugin holding an address")
    print("plugin-package selftest: OK")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    print(package(ROOT, args[0], Path(args[1])))
    return 0


if __name__ == "__main__":
    sys.exit(main())
