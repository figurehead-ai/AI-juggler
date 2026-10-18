#!/usr/bin/env python3
"""AJ-E604: ZCode credit recomputation from a CLONED db.sqlite.

Formula (docs.z.ai/devpack/overview, revised 2026-07-30):
  credits = (input*in_mult + cached*cached_mult + output*out_mult) / 10000
  off-peak windows bill at 50%; peak = weekdays 14:00-18:00 UTC+8.

BUCKET MAPPING (an assumption this script makes explicit — graded
`inferred` in zcode-captures.md until a vendor-shown comparison pins
it): `input_tokens` in model_usage counts ALL prompt tokens (verified:
input+output == computed_total on inspected rows), so the formula's
"input" bucket = input_tokens - cache_read - cache_creation; "cached"
= cache_read + cache_creation; "output" = output_tokens (reasoning
rides output per the usage-revision note).

Usage: python3 zcode-recompute.py <clone.sqlite> <YYYY-MM-DD>
"""
import sqlite3
import sys
from datetime import datetime, timedelta, timezone

MULT = {  # per-model multipliers (in, cached, out)
    "GLM-5.3": (6.9, 1.7, 24.0),
    "GLM-5.3-Flash": (2.3, 0.56, 8.0),
}
TZ8 = timezone(timedelta(hours=8))  # vendor publishes windows in UTC+8


def peak(utc_ms: int) -> bool:
    t = datetime.fromtimestamp(utc_ms / 1000, TZ8)
    return t.weekday() < 5 and 14 <= t.hour < 18


def credits(model, inp, cached, out, offpeak):
    m = MULT.get(model)
    if m is None:
        raise SystemExit(f"no published multiplier for {model}")
    raw = (inp * m[0] + cached * m[1] + out * m[2]) / 10_000
    return raw * (0.5 if offpeak else 1.0)


def main():
    db, day = sys.argv[1], sys.argv[2]
    lo = int(datetime.strptime(day, "%Y-%m-%d").replace(tzinfo=TZ8)
             .timestamp() * 1000)
    hi = lo + 86_400_000
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    rows = con.execute(
        "SELECT model_id, input_tokens, output_tokens,"
        " cache_creation_input_tokens, cache_read_input_tokens,"
        " started_at FROM model_usage"
        " WHERE status='completed' AND started_at >= ? AND started_at < ?",
        (lo, hi)).fetchall()
    total = {"peak": 0.0, "off": 0.0}
    per_model = {}
    for model, inp, out, ccreate, cread, at in rows:
        fresh = inp - cread - ccreate
        off = not peak(at)
        c = credits(model, fresh, cread + ccreate, out, off)
        total["off" if off else "peak"] += c
        a, b = per_model.get(model, (0.0, 0))
        per_model[model] = (a + c, b + 1)
    print(f"day={day} tz=UTC+8 requests={len(rows)}")
    for model, (c, n) in sorted(per_model.items()):
        print(f"  {model}: {n} requests, {c:.2f} credits")
    print(f"  peak-window credits: {total['peak']:.2f}")
    print(f"  off-peak credits (50% rate): {total['off']:.2f}")
    print(f"  TOTAL: {total['peak'] + total['off']:.2f} credits")
    con.close()


if __name__ == "__main__":
    main()
