#!/usr/bin/env python3
"""No text metadata in a committed PNG (D-101).

A macOS screenshot carries XMP (`iTXt`) and EXIF (`eXIf`) chunks, and any
PNG can carry `tEXt`/`zTXt`: they can hold a user name, a path or a
window's title. Committed PNGs keep only their image: these four chunk
types fail the check; `--strip FILE…` removes them in place (the pixels
are untouched: only those chunks are dropped).

    python3 scripts/png_metadata_lint.py               every tracked PNG
    python3 scripts/png_metadata_lint.py --strip FILE…
    python3 scripts/png_metadata_lint.py --selftest
"""
from __future__ import annotations

import struct
import subprocess
import sys
import zlib

SIGNATURE = b"\x89PNG\r\n\x1a\n"
TEXT = {b"tEXt", b"zTXt", b"iTXt", b"eXIf"}


def chunks(data: bytes):
    pos = 8
    while pos + 8 <= len(data):
        (n,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        yield kind, data[pos : pos + 12 + n]
        pos += 12 + n
        if kind == b"IEND":
            break


def text_chunks(data: bytes) -> list[str]:
    if not data.startswith(SIGNATURE):
        return []
    return sorted({k.decode() for k, _ in chunks(data) if k in TEXT})


def stripped(data: bytes) -> bytes:
    return SIGNATURE + b"".join(raw for k, raw in chunks(data) if k not in TEXT)


def main(argv: list[str]) -> int:
    if argv[:1] == ["--selftest"]:
        return selftest()
    if argv[:1] == ["--strip"]:
        for path in argv[1:]:
            data = open(path, "rb").read()
            if text_chunks(data):
                open(path, "wb").write(stripped(data))
                print(f"stripped {path}")
        return 0
    files = subprocess.run(["git", "ls-files", "-z", "*.png"], capture_output=True, check=True).stdout
    bad = []
    for path in (f.decode() for f in files.split(b"\0") if f):
        found = text_chunks(open(path, "rb").read())
        if found:
            bad.append(f"{path}: {', '.join(found)}")
    for line in bad:
        print(f"png-metadata: {line}")
    if bad:
        print(f"png-metadata: {len(bad)} PNG(s) carry text metadata; `python3 scripts/png_metadata_lint.py --strip <file>` removes it")
        return 1
    print("png-metadata: no committed PNG carries text metadata")
    return 0


def _chunk(kind: bytes, body: bytes) -> bytes:
    return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)


def selftest() -> int:
    ihdr = _chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 0, 0, 0, 0))
    idat = _chunk(b"IDAT", zlib.compress(b"\x00\x00"))
    iend = _chunk(b"IEND", b"")
    meta = _chunk(b"tEXt", b"Author\x00/Users/jdoe") + _chunk(b"iTXt", b"XML:com.adobe.xmp\x00\x00\x00\x00\x00jdoe-work")
    png = SIGNATURE + ihdr + meta + idat + iend
    assert text_chunks(png) == ["iTXt", "tEXt"], text_chunks(png)
    clean = stripped(png)
    assert clean == SIGNATURE + ihdr + idat + iend and text_chunks(clean) == [] and b"jdoe" not in clean
    print("png-metadata: selftest passed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
