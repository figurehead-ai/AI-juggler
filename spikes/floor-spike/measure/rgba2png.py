#!/usr/bin/env python3
"""Converts the spike's raw captures (8-byte header: width, height; then RGBA) to PNG, optionally scaled up."""
import struct, sys, zlib, pathlib

def png(path, w, h, rgba):
    rows = b''.join(b'\x00' + rgba[y * w * 4:(y + 1) * w * 4] for y in range(h))
    chunk = lambda t, d: struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    pathlib.Path(path).write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b''))

def scale(w, h, rgba, k):
    out = bytearray()
    for y in range(h * k):
        row = rgba[(y // k) * w * 4:(y // k + 1) * w * 4]
        out += b''.join(row[x * 4:x * 4 + 4] * k for x in range(w))
    return w * k, h * k, bytes(out)

for f in sys.argv[2:]:
    b = pathlib.Path(f).read_bytes()
    w, h = struct.unpack('<II', b[:8])
    k = int(sys.argv[1])
    W, H, px = scale(w, h, b[8:], k) if k > 1 else (w, h, b[8:])
    png(f.replace('.rgba', '.png'), W, H, px)
    print(f.replace('.rgba', '.png'), W, H)
