#!/usr/bin/env python3
"""How colourful a capture is, per area (0.5, D-117, AJ-E3601 R7).

Hasler and Süsstrunk's colourfulness M (2003) and the share of "loud"
pixels (a channel spread of 70 or more, the brightest channel over 60),
for the rail (the left `--rail` pixels) and the main area, read from the
gallery's PNG captures with no third-party module (zlib only).

The calm-attention targets (docs/research/calm-attention/proposal.md §f),
checked with --check:
  - rail loud ≤ 1.5 %, main loud ≤ 1.2 % (with needs on screen);
  - rail M ≤ main M + 1: the rail no calmer than the content is a failure;
  - an all-clear capture (its name holds "all-clear") stays under 0.6 %.

Usage:
  python3 scripts/qa/colourfulness.py [--rail PX] [--check] [--json] PNG...
  python3 scripts/qa/colourfulness.py --selftest
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
import zlib
from pathlib import Path

RAIL_LOUD_MAX = 1.5
MAIN_LOUD_MAX = 1.2
RAIL_OVER_MAIN_MAX = 1.0
ALL_CLEAR_LOUD_MAX = 0.6


def read_png(path: Path) -> tuple[int, int, int, bytes]:
    """Width, height, channels (3 or 4) and the unfiltered rows of an
    8-bit, non-interlaced RGB or RGBA PNG."""
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    at, idat = 8, []
    width = height = channels = 0
    while at < len(data):
        (length,) = struct.unpack(">I", data[at : at + 4])
        kind = data[at + 4 : at + 8]
        body = data[at + 8 : at + 8 + length]
        if kind == b"IHDR":
            width, height, depth, colour, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or colour not in (2, 6) or interlace:
                raise ValueError(f"{path}: only 8-bit, non-interlaced RGB/RGBA")
            channels = 3 if colour == 2 else 4
        elif kind == b"IDAT":
            idat.append(body)
        elif kind == b"IEND":
            break
        at += 12 + length
    raw = zlib.decompress(b"".join(idat))
    stride = width * channels
    out = bytearray(height * stride)
    prev = bytearray(stride)
    at = 0
    for y in range(height):
        kind = raw[at]
        line = bytearray(raw[at + 1 : at + 1 + stride])
        at += 1 + stride
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = prev[i]
            corner = prev[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + left) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + up) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + ((left + up) >> 1)) & 0xFF
            elif kind == 4:
                p = left + up - corner
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - corner)
                pred = left if pa <= pb and pa <= pc else up if pb <= pc else corner
                line[i] = (line[i] + pred) & 0xFF
        out[y * stride : (y + 1) * stride] = line
        prev = line
    return width, height, channels, bytes(out)


def area_stats(png: tuple[int, int, int, bytes], x0: int, x1: int) -> tuple[float, float]:
    """M and the loud share (%) of columns x0..x1, every second pixel."""
    width, height, channels, rows = png
    n = 0
    sa = sb = saa = sbb = 0.0
    loud = 0
    for y in range(0, height, 2):
        base = y * width * channels
        for x in range(max(0, x0), min(width, x1), 2):
            i = base + x * channels
            r, g, b = rows[i], rows[i + 1], rows[i + 2]
            rg = r - g
            yb = 0.5 * (r + g) - b
            sa += rg
            sb += yb
            saa += rg * rg
            sbb += yb * yb
            n += 1
            hi, lo = max(r, g, b), min(r, g, b)
            if hi > 60 and hi - lo >= 70:
                loud += 1
    if n == 0:
        return 0.0, 0.0
    ma, mb = sa / n, sb / n
    va, vb = max(0.0, saa / n - ma * ma), max(0.0, sbb / n - mb * mb)
    return math.sqrt(va + vb) + 0.3 * math.sqrt(ma * ma + mb * mb), 100.0 * loud / n


def measure(path: Path, rail: int) -> dict:
    png = read_png(path)
    rail_m, rail_loud = area_stats(png, 0, rail)
    main_m, main_loud = area_stats(png, rail, png[0])
    return {"file": str(path), "rail_m": round(rail_m, 1), "rail_loud": round(rail_loud, 2), "main_m": round(main_m, 1), "main_loud": round(main_loud, 2)}


def verdicts(row: dict) -> list[str]:
    """What misses the §f targets."""
    name = Path(row["file"]).name
    out = []
    if "all-clear" in name:
        if row["rail_loud"] > ALL_CLEAR_LOUD_MAX:
            out.append(f"all-clear rail loud {row['rail_loud']}% > {ALL_CLEAR_LOUD_MAX}%")
        return out
    if row["rail_loud"] > RAIL_LOUD_MAX:
        out.append(f"rail loud {row['rail_loud']}% > {RAIL_LOUD_MAX}%")
    if row["main_loud"] > MAIN_LOUD_MAX:
        out.append(f"main loud {row['main_loud']}% > {MAIN_LOUD_MAX}%")
    if row["rail_m"] > row["main_m"] + RAIL_OVER_MAIN_MAX:
        out.append(f"rail M {row['rail_m']} > main M {row['main_m']} + {RAIL_OVER_MAIN_MAX}")
    return out


def _png(width: int, height: int, pixel) -> bytes:
    raw = b"".join(b"\x00" + b"".join(bytes(pixel(x, y)) for x in range(width)) for y in range(height))
    def chunk(kind: bytes, body: bytes) -> bytes:
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


def selftest() -> int:
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        grey = Path(tmp, "grey.png")
        grey.write_bytes(_png(40, 10, lambda x, y: (30, 30, 30)))
        loud = Path(tmp, "loud.png")
        # A red rail, a grey main area.
        loud.write_bytes(_png(40, 10, lambda x, y: (240, 40, 40) if x < 10 else (30, 30, 30)))
        g = measure(grey, 10)
        r = measure(loud, 10)
        assert g["rail_m"] == 0 and g["rail_loud"] == 0 and verdicts(g) == [], g
        assert r["rail_loud"] == 100.0 and r["main_loud"] == 0.0, r
        assert any("rail loud" in v for v in verdicts(r)), verdicts(r)
        # Filters: a Sub-filtered row reads the same as a plain one.
        width, height, channels, rows = read_png(loud)
        assert (width, height, channels) == (40, 10, 3) and rows[:3] == bytes((240, 40, 40))
    print("colourfulness selftest: OK")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("pngs", nargs="*", type=Path)
    parser.add_argument("--rail", type=int, default=264, help="the rail's width in pixels (default 264)")
    parser.add_argument("--check", action="store_true", help="exit 1 when a capture misses the §f targets")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        return selftest()
    rows = [measure(path, args.rail) for path in args.pngs]
    failed = False
    if args.json:
        print(json.dumps(rows, indent=2))
    else:
        print("| Capture | Rail M | Rail loud | Main M | Main loud | Targets |")
        print("|---|---|---|---|---|---|")
    for row in rows:
        misses = verdicts(row)
        failed = failed or bool(misses)
        if not args.json:
            print(f"| {Path(row['file']).name} | {row['rail_m']} | {row['rail_loud']}% | {row['main_m']} | {row['main_loud']}% | {'; '.join(misses) or 'met'} |")
    return 1 if args.check and failed else 0


if __name__ == "__main__":
    sys.exit(main())
