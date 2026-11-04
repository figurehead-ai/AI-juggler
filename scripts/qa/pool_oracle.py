#!/usr/bin/env python3
"""The pools reference model (AJ-E3301, `docs/qa/0.4-validation-plan.md` §6).

Written from D-62 (decisions 8-16) and `docs/research/pools-model.md`, not
from the Rust code, so the L1 tables check the resolver and the talk rules
against an independent reading. Each table is printed as `case<TAB>expected`
lines; the Rust L1 tests (`crates/juggler-core/tests/pools_l1.rs`) build the
same cases and compare line by line.

    pool_oracle.py membership   # 28 224 cases: one session, two pools
    pool_oracle.py talk         #  9 216 cases: one message in one pool
    pool_oracle.py inbound      #     27 cases: Claude's own channel
    pool_oracle.py --selftest   # the model's own hand-checked cases

The rules, as D-62 states them:

- Scope governs who a broad member (App, Account, Tool, Mac, everyone)
  brings in. A session named on its own was checked against the scope when
  added, so it stays.
- Inside one pool the most specific member wins: Session > App > Account >
  Tool > Mac > everyone; its role is the session's there.
- Across pools the session is in each pool that matches it.
- A sealed pool takes its sessions out of every other; two sealed pools:
  the more specific match, then the older pool.
- Talk: a connection's own rule, else the role pair's, else allowed.
  Blocked refuses; ask first holds; allowed delivers, waking only at Full;
  a receiver that can only be watched can't read it until continued in AI
  Juggler, so it's held. Nothing crosses a link out of or into a sealed
  pool.
- Claude's own channel per receiving session: refuse if any of its pools is
  sealed, hold if any pair into it holds or blocks, else accept.
"""

from __future__ import annotations

import itertools
import sys

KINDS = ["session", "app", "account", "tool", "mac", "everyone", "none"]
RANK = {kind: at for at, kind in enumerate(KINDS)}
SCOPES = ["in", "out"]
CAPABILITIES = ["full", "shell", "watched"]
FLAGS = ["0", "1"]


def pool_cases():
    """Each pool's setting: (member kind, scope, capability, sealed, twice)."""
    return list(itertools.product(KINDS, SCOPES, CAPABILITIES, FLAGS, FLAGS))


def members_of(kind: str, twice: str):
    """The pool's members that match the session: (kind, role, joined)."""
    out = []
    if kind != "none":
        out.append((kind, "lead", 10))
    # Matched twice: a second, broader member matches it too.
    if twice == "1" and kind != "everyone":
        out.append(("everyone", "observer", 20))
    return out


def membership(one, two):
    pools = []
    for pool_id, created, (kind, scope, _cap, sealed, twice) in (("p1", 1, one), ("p2", 2, two)):
        matched = members_of(kind, twice)
        if not matched:
            continue
        best = min(matched, key=lambda m: (RANK[m[0]], -m[2]))
        pools.append({"id": pool_id, "created": created, "kind": best[0], "role": best[1],
                      "joined": best[2], "scope": scope, "sealed": sealed == "1"})
    placed, skipped = [], []
    for pool in pools:
        if pool["kind"] != "session" and pool["scope"] == "out":
            skipped.append((pool["id"], "out_of_scope"))
        else:
            placed.append(pool)
    seals = [pool for pool in placed if pool["sealed"]]
    if seals:
        seal = min(seals, key=lambda p: (RANK[p["kind"]], p["created"], p["id"]))
        for pool in placed:
            if pool["id"] != seal["id"]:
                skipped.append((pool["id"], f"sealed_by:{seal['id']}"))
        placed = [seal]
    placed.sort(key=lambda p: (not p["sealed"], RANK[p["kind"]], -p["joined"], p["id"]))
    skipped.sort()
    ins = ",".join(f"{p['id']}:{p['role']}:{p['kind']}" for p in placed) or "-"
    outs = ",".join(f"{pool}:{why}" for pool, why in skipped) or "-"
    return f"{ins}|{outs}"


def membership_table():
    for one, two in itertools.product(pool_cases(), repeat=2):
        yield ",".join(one) + "|" + ",".join(two), membership(one, two)


ROLES = ["lead", "member", "reviewer", "observer"]
RULES = ["allowed", "ask", "blocked"]
OVERRIDES = ["none", "allowed", "ask", "blocked"]
LEVELS = ["read", "work_together", "full", "custom"]


def talk(pair, override, sealed, reach, level, linked):
    if linked == "1" and sealed == "1":
        return "refuse:sealed"
    rule = pair if override == "none" else override
    if rule == "blocked":
        return "refuse:blocked"
    if rule == "ask":
        return "hold:ask"
    if reach == "watched":
        return "hold:watched"
    # Custom's own set decides elsewhere; the table's Custom has no wake.
    return "deliver:wake" if level == "full" else "deliver"


def talk_table():
    for sender, receiver, pair, override, sealed, reach, level, linked in itertools.product(
        ROLES, ROLES, RULES, OVERRIDES, FLAGS, CAPABILITIES, LEVELS, FLAGS
    ):
        key = ",".join([sender, receiver, pair, override, sealed, reach, level, linked])
        yield key, talk(pair, override, sealed, reach, level, linked)


POOL_STATES = ["sealed", "holds", "open"]


def inbound(states):
    if "sealed" in states:
        return "refuse"
    if "holds" in states:
        return "hold"
    return "accept"


def inbound_table():
    for states in itertools.product(POOL_STATES, repeat=3):
        yield ",".join(states), inbound(states)


def selftest() -> int:
    checks = [
        # Named on its own, out of scope: it stays (checked when added).
        (membership(("session", "out", "full", "0", "0"), ("none", "in", "full", "0", "0")), "p1:lead:session|-"),
        # A broad member out of scope: skipped, with its reason.
        (membership(("app", "out", "full", "0", "0"), ("none", "in", "full", "0", "0")), "-|p1:out_of_scope"),
        # Most specific wins inside a pool.
        (membership(("account", "in", "full", "0", "1"), ("none", "in", "full", "0", "0")), "p1:lead:account|-"),
        # Two pools: in both, the more specific first.
        (membership(("everyone", "in", "full", "0", "0"), ("app", "in", "full", "0", "0")), "p2:lead:app,p1:lead:everyone|-"),
        # Sealed takes it out of the other.
        (membership(("tool", "in", "full", "0", "0"), ("mac", "in", "full", "1", "0")), "p2:lead:mac|p1:sealed_by:p2"),
        # Two sealed: the more specific match wins.
        (membership(("mac", "in", "full", "1", "0"), ("app", "in", "full", "1", "0")), "p2:lead:app|p1:sealed_by:p2"),
        # Two sealed, equally specific: the older pool.
        (membership(("app", "in", "full", "1", "0"), ("app", "in", "full", "1", "0")), "p1:lead:app|p2:sealed_by:p1"),
        # A sealed pool it's out of scope of doesn't hold it.
        (membership(("everyone", "out", "full", "1", "0"), ("app", "in", "full", "0", "0")), "p2:lead:app|p1:out_of_scope"),
        (talk("allowed", "blocked", "0", "full", "full", "0"), "refuse:blocked"),
        (talk("blocked", "allowed", "0", "full", "full", "0"), "deliver:wake"),
        (talk("allowed", "none", "0", "watched", "full", "0"), "hold:watched"),
        (talk("allowed", "none", "1", "full", "read", "1"), "refuse:sealed"),
        (talk("allowed", "none", "1", "full", "read", "0"), "deliver"),
        (inbound(("open", "holds", "open")), "hold"),
        (inbound(("holds", "sealed", "open")), "refuse"),
    ]
    failed = [(got, want) for got, want in checks if got != want]
    for got, want in failed:
        print(f"selftest: got {got!r}, want {want!r}", file=sys.stderr)
    counts = (sum(1 for _ in membership_table()), sum(1 for _ in talk_table()), sum(1 for _ in inbound_table()))
    if counts != (28_224, 9_216, 27):
        print(f"selftest: table sizes {counts}", file=sys.stderr)
        return 1
    print(f"pool_oracle selftest: {len(checks) - len(failed)}/{len(checks)} ok; tables {counts}")
    return 1 if failed else 0


def main(argv: list[str]) -> int:
    tables = {"membership": membership_table, "talk": talk_table, "inbound": inbound_table}
    if argv[1:] == ["--selftest"]:
        return selftest()
    if len(argv) != 2 or argv[1] not in tables:
        print(__doc__, file=sys.stderr)
        return 2
    out = sys.stdout
    for key, expected in tables[argv[1]]():
        out.write(f"{key}\t{expected}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
