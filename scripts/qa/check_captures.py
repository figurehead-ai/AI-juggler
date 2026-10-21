#!/usr/bin/env python3
"""Every image in a walk is redacted, and someone looked at it.

A walk by hand (docs/qa/evidence/walks/) may run on a real setup, so its
captures can show real names. Each image there needs a sidecar,
`<image>.redacted.json`, written by scripts/qa/redact_capture.swift:

- `sha256` matches the image as committed (a re-made image needs a new sidecar);
- `boxes` is not empty, or `no_real_data` says why no redaction is needed;
- `checked_by_eye` is true: someone opened the image and saw nothing real.

Usage: scripts/qa/check_captures.py              (every walk image; exit 1 on any finding)
       scripts/qa/check_captures.py --staged     (only staged images: a pre-commit hook)
       scripts/qa/check_captures.py --confirm IMAGE...
                                    (after opening each image: mark its sidecar checked)
       scripts/qa/check_captures.py --record IMAGE (--box X,Y,W,H... | --no-real-data REASON)
                                    (a sidecar for an image already redacted, or one with
                                     nothing real in it; the image itself is not touched)
       scripts/qa/check_captures.py --selftest

As a pre-commit hook: `printf '#!/bin/sh\\nexec scripts/qa/check_captures.py --staged\\n'
> .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit`.
"""
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
WALKS = "docs/qa/evidence/walks"
IMAGE = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".heic", ".tiff"}


def sidecar_of(image: Path) -> Path:
    return image.with_name(image.name + ".redacted.json")


def problems(image: Path) -> list[str]:
    side = sidecar_of(image)
    if not side.is_file():
        return [f"{image}: no sidecar {side.name} (make it with scripts/qa/redact_capture.swift)"]
    try:
        meta = json.loads(side.read_text())
    except ValueError:
        return [f"{image}: sidecar is not JSON"]
    out = []
    if meta.get("sha256") != hashlib.sha256(image.read_bytes()).hexdigest():
        out.append(f"{image}: changed since its sidecar was written (redact it again)")
    if not meta.get("boxes") and not str(meta.get("no_real_data") or "").strip():
        out.append(f"{image}: no boxes and no no_real_data reason")
    if meta.get("checked_by_eye") is not True:
        out.append(f"{image}: not checked by eye (open it, then --confirm)")
    return out


def walk_images(root: Path) -> list[Path]:
    base = root / WALKS
    return sorted(p for p in base.rglob("*") if p.is_file() and p.suffix.lower() in IMAGE) if base.is_dir() else []


def staged_images(root: Path) -> list[Path]:
    out = subprocess.run(["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"],
                         cwd=root, capture_output=True, text=True, check=False).stdout
    return [root / line for line in out.splitlines()
            if line.startswith(WALKS + "/") and Path(line).suffix.lower() in IMAGE]


def confirm(paths: list[str]) -> int:
    status = 0
    for raw in paths:
        image = Path(raw)
        side = sidecar_of(image)
        if not side.is_file():
            print(f"{image}: no sidecar to confirm"); status = 1; continue
        meta = json.loads(side.read_text())
        if meta.get("sha256") != hashlib.sha256(image.read_bytes()).hexdigest():
            print(f"{image}: changed since its sidecar was written; not confirmed"); status = 1; continue
        meta["checked_by_eye"] = True
        side.write_text(json.dumps(meta, indent=2, sort_keys=True) + "\n")
        print(f"{image}: confirmed")
    return status


def record(args: list[str]) -> int:
    if not args:
        print("--record IMAGE (--box X,Y,W,H... | --no-real-data REASON)"); return 2
    image, rest = Path(args[0]), args[1:]
    boxes, reason = [], None
    while rest:
        flag = rest.pop(0)
        if not rest:
            print(f"{flag} needs a value"); return 2
        value = rest.pop(0)
        if flag == "--box":
            parts = value.split(",")
            if len(parts) != 4 or not all(p.strip().isdigit() for p in parts) or int(parts[2]) <= 0 or int(parts[3]) <= 0:
                print(f"box {value!r} is not X,Y,W,H"); return 2
            boxes.append([int(p) for p in parts])
        elif flag == "--no-real-data" and value.strip():
            reason = value
        else:
            print(f"unknown or empty {flag}"); return 2
    if bool(boxes) == bool(reason):
        print("give --box (the boxes already drawn on it) or --no-real-data REASON, not both or neither"); return 2
    if not image.is_file():
        print(f"{image}: no such image"); return 2
    meta = {"file": image.name, "sha256": hashlib.sha256(image.read_bytes()).hexdigest(), "boxes": boxes,
            "checked_by_eye": False, "recorded": "redacted before redact_capture.swift existed" if boxes else "no redaction"}
    if reason:
        meta["no_real_data"] = reason
    sidecar_of(image).write_text(json.dumps(meta, indent=2, sort_keys=True) + "\n")
    print(f"{image}: recorded; open it, then --confirm")
    return 0


def selftest() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / WALKS / "w").mkdir(parents=True)
        img = root / WALKS / "w" / "a.jpg"
        img.write_bytes(b"jpeg bytes")
        assert any("no sidecar" in p for p in problems(img))
        good = {"sha256": hashlib.sha256(b"jpeg bytes").hexdigest(), "boxes": [[0, 0, 1, 1]], "checked_by_eye": False}
        sidecar_of(img).write_text(json.dumps(good))
        assert any("not checked by eye" in p for p in problems(img))
        assert confirm([str(img)]) == 0 and problems(img) == []
        img.write_bytes(b"other bytes")
        assert any("changed since" in p for p in problems(img))
        assert confirm([str(img)]) == 1
        sidecar_of(img).write_text(json.dumps({**good, "sha256": hashlib.sha256(b"other bytes").hexdigest(),
                                                "boxes": [], "checked_by_eye": True}))
        assert any("no boxes" in p for p in problems(img))
        sidecar_of(img).write_text(json.dumps({**good, "sha256": hashlib.sha256(b"other bytes").hexdigest(),
                                                "boxes": [], "no_real_data": "fixture HOME", "checked_by_eye": True}))
        assert problems(img) == []
        assert walk_images(root) == [img]
        assert record([str(img)]) == 2 and record([str(img), "--box", "1,2,3"]) == 2
        assert record([str(img), "--no-real-data", "fixture"]) == 0 and "not checked by eye" in problems(img)[0]
    print("check_captures selftest: OK")


def main() -> int:
    args = sys.argv[1:]
    if args[:1] == ["--selftest"]:
        selftest()
        return 0
    if args[:1] == ["--confirm"]:
        return confirm(args[1:])
    if args[:1] == ["--record"]:
        return record(args[1:])
    images = staged_images(ROOT) if args[:1] == ["--staged"] else walk_images(ROOT)
    found = [p for image in images for p in problems(image)]
    for line in found:
        print(f"UNREDACTED OR UNCHECKED CAPTURE: {line}")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
