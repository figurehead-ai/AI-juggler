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
    pool_oracle.py l2-setup --truth T                       # L2's scenario
    pool_oracle.py l2-check --truth T --setup S --harness H # L2's check

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
        if pool["kind"] == "everyone" and pool["scope"] == "out":
            # "Everyone in scope" (pools-model.md §5): a session outside
            # the scope was never asked in, so it isn't left out either.
            continue
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


# --- L2 on the replica (AJ-E3304) -----------------------------------------
#
# `l2-setup --truth T` writes the scenario: which conversations stay
# Watched, the pools (scope, sealed, skip), and the joins in order, all in
# the replica's own terms (account labels, project folders, conversation
# ids). `l2-check --truth T --setup S --harness H` works out, from the
# replica's truth alone, what each session's Pools tab must say, and
# compares. The replica's truth is the generator's, not the app's readers'.

L2_KINDS = ["session", "app", "account", "tool", "mac", "everyone"]
L2_RANK = {kind: at for at, kind in enumerate(L2_KINDS)}


def _l2_facts(truth):
    """Each conversation's facts: account label, its sign-in's group, App,
    tool and project folder (a worktree's is its repository's)."""
    accounts = {a["label"]: a for a in truth["accounts"]}

    def signed_in(label):
        who = accounts[label].get("signed_in_as")
        return None if who is None else (accounts[label]["tool"], who)

    out = {}
    for c in truth["conversations"]:
        project = truth["projects"].get(c["project"])
        out[(c["account"], c["id"])] = {
            "account": c["account"],
            "sub": signed_in(c["account"]),
            "tool": c["tool"],
            "project": project["path"].rstrip("/") if project else None,
            "cwd": c["cwd"],
            "title": c.get("title"),
        }
    return out


def _pick(facts, account, project_path, skip, count=1, exists=True):
    import os
    found = sorted(
        key for key, f in facts.items()
        if f["account"] == account and f["project"] == project_path
        and key not in skip and (os.path.isdir(f["cwd"]) == exists)
    )
    if len(found) < count:
        raise SystemExit(f"l2-setup: the replica has too few {account} conversations in {project_path}")
    return found[:count]


def l2_setup(truth):
    facts = _l2_facts(truth)
    path = {name: p["path"].rstrip("/") for name, p in truth["projects"].items()}
    used = set()

    def take(account, project, count=1):
        got = _pick(facts, account, path[project], used, count)
        used.update(got)
        return [{"account": a, "id": i} for a, i in got]

    watched = take("work", "acme-web", 3)
    named = take("personal", "payments-api")[0]
    out_of_scope = take("personal", "infra")[0]
    sealed_one = take("client-acme", "blog")[0]
    scratch_one = take("work", "scratch")[0]
    pools = [
        {"name": "Feature team", "scope": [path["acme-web"], path["acme-web2"]]},
        {"name": "Reviews", "scope": "every"},
        {"name": "Client Acme", "scope": [path["blog"], path["marketing-site"]], "sealed": True},
        {"name": "Whole Mac", "scope": "every", "skip_held": True},
        {"name": "Scratchpad", "scope": [path["scratch"]]},
    ]
    joins = [
        ("Feature team", {"kind": "app", "account": "work"}, "lead"),
        ("Feature team", {"kind": "account", "account": "default"}, "reviewer"),
        ("Feature team", {"kind": "tool", "tool": "codex"}, "observer"),
        ("Reviews", {"kind": "tool", "tool": "claude-code"}, "reviewer"),
        ("Client Acme", {"kind": "account", "account": "client-acme"}, "member"),
        ("Whole Mac", {"kind": "mac"}, "observer"),
        ("Scratchpad", {"kind": "everyone"}, "member"),
        ("Reviews", {"kind": "session", **named}, "lead"),
        ("Feature team", {"kind": "session", **out_of_scope}, "member"),
        ("Whole Mac", {"kind": "session", **sealed_one}, "lead"),
        ("Scratchpad", {"kind": "session", **scratch_one}, "reviewer"),
    ] + [("Feature team", {"kind": "conversation", **w}, "member") for w in watched]
    return {
        "folders": {a["label"]: a["folder"] for a in truth["accounts"]},
        "watched": watched,
        "continue_later": watched[:1],
        "pools": pools,
        "joins": [{"step": n + 1, "pool": pool, "member": member, "role": role}
                  for n, (pool, member, role) in enumerate(joins)],
    }


def _l2_resolve(f, key, pools, members, named_in):
    """D-62 for one session: (placed, skipped). `members[pool]` is the
    pool's members in join order (kind, match, role); `named_in` the pools
    that name this session on its own."""
    matches = []
    for created, pool in enumerate(pools):
        # A project's own pool is about its project only.
        if pool.get("own") and pool["scope"] != [f["project"]]:
            continue
        found = []
        for joined, (kind, match, role) in enumerate(members[pool["name"]]):
            if kind == "session":
                hit = match == key
            elif kind == "app":
                hit = match == f["account"]
            elif kind == "account":
                hit = f["sub"] is not None and match == f["sub"]
            elif kind == "tool":
                hit = match == f["tool"]
            else:  # mac (one Mac) or everyone
                hit = True
            if hit:
                found.append((kind, role, joined))
        if found:
            best = min(found, key=lambda m: (L2_RANK[m[0]], -m[2]))
            matches.append({**pool, "created": created, "kind": best[0], "role": best[1]})
    placed, skipped = [], []
    for m in matches:
        scope = m["scope"]
        in_scope = scope == "every" or f["project"] in scope
        others = [p for p in named_in if p != m["name"]]
        if m["kind"] == "everyone" and not in_scope:
            continue
        if m["kind"] != "session" and not in_scope:
            skipped.append((m["name"], "out_of_scope", None))
        elif m["kind"] != "session" and m.get("skip_held") and others:
            skipped.append((m["name"], "held_elsewhere", others[0]))
        else:
            placed.append(m)
    seals = [m for m in placed if m.get("sealed")]
    if seals:
        seal = min(seals, key=lambda m: (L2_RANK[m["kind"]], m["created"]))
        for m in placed:
            if m is not seal:
                skipped.append((m["name"], "sealed_elsewhere", seal["name"]))
        placed = [seal]
    return placed, skipped


def l2_check(truth, setup, harness):
    import os
    facts = _l2_facts(truth)
    problems = []
    key_of = lambda v: (v["account"], v["id"])  # noqa: E731
    watched = {key_of(w) for w in setup["watched"]}
    later = {key_of(w) for w in setup["continue_later"]}

    # Continue: every conversation but the Watched, refused where its folder is gone.
    want = {k for k in facts if k not in watched}
    got = {key_of(c): c for c in harness["continued"]}
    for k in sorted(want - got.keys()):
        problems.append(f"continue: {k} not continued (not listed?)")
    for k in sorted(got.keys() - want):
        problems.append(f"continue: {k} continued but the setup kept it")
    for k, c in got.items():
        if k not in facts:
            continue
        gone = not os.path.isdir(facts[k]["cwd"])
        if gone and "refused" not in c:
            problems.append(f"continue: {k} continued though its folder is gone")
        if not gone and "refused" in c:
            problems.append(f"continue: {k} refused: {c['refused']}")
    sessions = {k for k, c in got.items() if "session" in c}
    sessions |= {key_of(c) for c in harness["continued_later"] if "session" in c}

    # The joins, in order: a session named on its own is checked when added.
    pools = setup["pools"]
    members = {p["name"]: [] for p in pools}
    named_in = {}
    accounts = {a["label"]: a for a in truth["accounts"]}

    def sub_of(label):
        who = accounts[label].get("signed_in_as")
        return None if who is None else (accounts[label]["tool"], who)

    steps = {s["step"]: s for s in harness["joins"]}
    for join in setup["joins"]:
        m, pool = join["member"], next(p for p in pools if p["name"] == join["pool"])
        kind = m["kind"]
        expect_ok, why = True, None
        if kind in ("session", "conversation"):
            k = key_of(m)
            if kind == "session":
                f = facts[k]
                placed, _ = _l2_resolve(f, k, pools, members, named_in.get(k, []))
                if pool["scope"] != "every" and f["project"] not in pool["scope"]:
                    expect_ok, why = False, "out of scope"
                elif any(p.get("sealed") and p["name"] != pool["name"] for p in placed):
                    expect_ok, why = False, "sealed elsewhere"
            if expect_ok:
                members[pool["name"]].append(("session", k, join["role"]))
                named_in.setdefault(k, []).append(pool["name"])
        else:
            match = {"app": lambda: m["account"], "account": lambda: sub_of(m["account"]),
                     "tool": lambda: m["tool"]}.get(kind, lambda: None)()
            members[pool["name"]].append((kind, match, join["role"]))
        step = steps.get(join["step"])
        if step is None:
            problems.append(f"join {join['step']}: not run")
        elif step["ok"] != expect_ok:
            problems.append(f"join {join['step']} ({kind} into {pool['name']}): "
                            f"{'refused: ' + str(step['refused']) if not step['ok'] else 'let in'}, want {'let in' if expect_ok else 'refused (' + why + ')'}")

    # Each project's own pool: everyone in its project, skipping sessions another pool names.
    projects = sorted({f["project"] for f in facts.values() if f["project"]})
    all_pools = pools + [{"name": f"project:{p}", "scope": [p], "skip_held": True, "own": True}
                         for p in projects]
    for p in projects:
        members[f"project:{p}"] = [("everyone", None, "member")]

    rows = {key_of(s): s for s in harness["sessions"]}
    for k in sorted(sessions):
        f = facts[k]
        placed, skipped = _l2_resolve(f, k, all_pools, members, named_in.get(k, []))
        want_in = sorted((p["name"], p["role"], p["kind"]) for p in placed)
        want_out = sorted((n, r, b or "") for n, r, b in skipped)
        row = rows.get(k)
        if row is None:
            problems.append(f"session {k}: no Pools tab")
            continue
        got_in = sorted((p["pool"], p["role"], p["via"]) for p in row["pools"])
        got_out = sorted((s["pool"], s["reason"], s["by"] or "") for s in row["skipped"])
        if got_in != want_in:
            problems.append(f"session {k} ({f['account']}, {f['project']}): in {got_in}, want {want_in}")
        if got_out != want_out:
            problems.append(f"session {k} ({f['account']}, {f['project']}): left out {got_out}, want {want_out}")
        if row["account_unknown"] != (f["sub"] is None):
            problems.append(f"session {k}: account_unknown {row['account_unknown']}")

    # Watched: listed with the index's title and tool, counted until continued.
    for moment, done in (("watched_before", set()), ("watched_after", later)):
        for pool in pools:
            name = pool["name"]
            seen = harness[moment][name]
            want_w = sorted(k for k in watched if any(
                kind == "session" and match == k for kind, match, _ in members[name]))
            got_w = sorted(key_of(w) for w in seen["watched"])
            if got_w != want_w:
                problems.append(f"{moment} {name}: Watched {got_w}, want {want_w}")
            for w in seen["watched"]:
                k = key_of(w)
                if k in facts and (w["title"] != facts[k]["title"] or w["tool"] != facts[k]["tool"]):
                    problems.append(f"{moment} {name}: {k} shows {w['title']!r} on {w['tool']}")
                if w["continued"] != (k in done):
                    problems.append(f"{moment} {name}: {k} continued={w['continued']}")
            in_pool = 0
            for k in sessions:
                if moment == "watched_before" and k in later:
                    continue
                placed, _ = _l2_resolve(facts[k], k, all_pools, members, named_in.get(k, []))
                in_pool += any(p["name"] == name for p in placed)
            want_n = in_pool + len([k for k in want_w if k not in done])
            if seen["members"] != want_n:
                problems.append(f"{moment} {name}: {seen['members']} members, want {want_n}")

    home = truth["home"].rstrip("/") + "/"
    for line in problems:
        print(line.replace(home, "~/"))
    print(f"pools L2: {len(sessions)} sessions, {len(setup['joins'])} joins, "
          f"{len(problems)} problem(s)", file=sys.stderr)
    return 1 if problems else 0


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
        (membership(("mac", "out", "full", "1", "0"), ("app", "in", "full", "0", "0")), "p2:lead:app|p1:out_of_scope"),
        # Everyone in scope never asked it in: not left out either.
        (membership(("everyone", "out", "full", "1", "0"), ("app", "in", "full", "0", "0")), "p2:lead:app|-"),
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
    if argv[1:2] in (["l2-setup"], ["l2-check"]):
        import argparse
        import json
        parser = argparse.ArgumentParser(prog=f"pool_oracle.py {argv[1]}")
        parser.add_argument("--truth", required=True)
        parser.add_argument("--setup")
        parser.add_argument("--harness")
        args = parser.parse_args(argv[2:])
        truth = json.load(open(args.truth))
        if argv[1] == "l2-setup":
            json.dump(l2_setup(truth), sys.stdout, indent=1)
            return 0
        return l2_check(truth, json.load(open(args.setup)), json.load(open(args.harness)))
    if len(argv) != 2 or argv[1] not in tables:
        print(__doc__, file=sys.stderr)
        return 2
    out = sys.stdout
    for key, expected in tables[argv[1]]():
        out.write(f"{key}\t{expected}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
