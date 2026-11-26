#!/usr/bin/env python3
"""g11.py: G11, the fabrication gate (D-130; D-149 amendment 2): a claimed outcome, benefit, state, count or cause must trace to a checked
clause. v8's feed design B showed the hole: its gate refused contradictions, and 7 of 16 half-sentences claimed what no clause said
("smaller assets speed up loading for every user"). Contradiction alone isn't enough; support is required.

  claims(text)                  -> the claim words in a model's text (states, outcomes, benefits, causes, counts)
  unsupported(text, clauses)    -> the claim words no checked clause supports (by stem; a number by its value)

  python3 g11.py --digest-v6    replay the v6 digest round (v6-results/v6-apple-digest_mr.json; its corpus read by the script, nothing
                                of it printed but the model's own lines) and count the picked lines G11 refuses
  python3 g11.py --bot-e3829    the eight real answers AJ-E3829 and AJ-E3832 R6 recorded (their manifest), against their fixture records
  python3 g11.py --selftest"""
import json, os, re, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE); EV = os.path.dirname(H)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                        # noqa: E402
from core import norm_item, facts_sentence                          # noqa: E402

STATE = (r"pass(?:es|ed|ing)?|fail\w*|fix(?:ed|es|ing)?|works?|working|broke\w*|done|finish\w*|complet\w*|ready|green|red|merged|deploy\w*|shipped|commit\w*|push\w*|"
         r"succe\w*|resolv\w*|solv\w*|stable|correct\w*|clean|bestanden|fehl\w*|behoben|funktionier\w*|fertig|kaputt|erledigt|läuft")
BENEFIT = (r"improv\w*|ensur\w*|speed\w*|faster|slower|quicker|reliab\w*|sav\w*|reduc\w*|cleaner|consistent\w*|safer|safe|secure|prevent\w*|enabl\w*|"
           r"allow\w*|better|simpler|easier|noise|clear\s+path|required|requires|forced|optimi\w*|centrali\w*|tidier|schneller|zuverlässig\w*|sicherer|einfacher")
CAUSE = r"because|since|so\s+that|due\s+to|thanks\s+to|means|meaning|therefore|hence|caus\w*|weil|damit|dadurch|wegen|deshalb|sodass|daher"
CLAIM = re.compile(r"\b(?:" + STATE + "|" + BENEFIT + "|" + CAUSE + r")\b|\b\d[\d.,]*%?", re.I)
def _stem(w): return re.sub(r"(?:ing|ed|es|s|en|er|t|e)$", "", w.lower())[:5]
CITE = re.compile(r"\(\d+\)|\[\d+\]")      # Bot's citation marks are no counts
def claims(text): return [m.group(0) for m in CLAIM.finditer(CITE.sub(" ", text or ""))]
def unsupported(text, clauses):
    toks = [t.strip(".,") for c in clauses for t in re.findall(r"\d[\d.,]*%?|[^\W\d_][\w-]*|so\s+that|due\s+to|thanks\s+to", c.lower())]
    stems = {_stem(t) for t in toks}; nums = {re.sub(r"[^\d]", "", t) for t in toks if re.search(r"\d", t)}
    out = []
    for c in claims(text):
        if re.match(r"\d", c):
            if re.sub(r"[^\d]", "", c) not in nums: out.append(c)
        elif " " in c:
            if not any(c.lower() in x.lower() for x in clauses): out.append(c)
        elif _stem(c) not in stems: out.append(c)
    return out
def checked_clauses(t):
    """a turn's checked clauses: pick.py's choices (every v5 check applied), the caveats the composer keeps in the agent's own words
    (v4's caveat kinds), and the facts' own sentence"""
    import pick as P6
    import gate, gate_v4
    caveats = [s for s in gate.sentences_v3(t["ctx"]["cleaned"]) if gate_v4.caveat_kinds(s)]
    return [x.get("clause") or x["words"] for x in P6.choices(t)] + caveats + [facts_sentence(t)]

def digest_v6():
    import digest_mr                                                # noqa: F401  (v6's job, registered)
    from core import REGISTRY
    R = json.load(open(os.path.join(EV, "v6-results", "v6-apple-digest_mr.json")))
    items = {x["id"]: norm_item(x) for x in json.load(open(os.path.join(EV, "heldout", "v6", "digest-v6.json")))}
    job = REGISTRY["digest_mr"]; shown = flagged = 0; per = []; windows = set()
    for r in R["results"]:
        d = items[r["id"]]; rep = core6.replay6(job, d, r["setting"], r, True, R["meta"].get("retries", 2))
        for x in rep["final"].get("items") or []:
            if x.get("source") != "ai": continue
            a = d["by_ref"][x["agent"]]; shown += 1
            bad = unsupported(" ".join(x["words"]), checked_clauses(a["newest"]))
            if bad: flagged += 1; windows.add(r["id"]); per.append({"window": r["id"], "line": x["words"][0], "unsupported": bad})
    return {"ai_lines_shown": shown, "refused_by_g11": flagged, "refused_pct": round(100.0 * flagged / shown, 1) if shown else None,
            "windows_with_a_refused_line": len(windows), "windows": len(R["results"]), "lines": per}
BOT_RECORDS = ["“api” is waiting on you in “Work”", "“web” is working in “Work”", "You called “api” “the payments agent”", "api asks you", "web working"]
BOT_ANSWERS = [   # AJ-E3832's manifest, R6: the real engine's answers, word for word (records-grounded ones; the general ones carry no record claim)
 "“api” is waiting on you in “Work”", "“web” is working in “Work”", "You called “api” “the payments agent” (1).",
 "“api” is waiting on you “web” is working", "“api” is asking for your response right now."]
BOT_GENERAL = ["I am a foundation model running on Apple's on-device model, version 27.0.1."]
def bot_e3829():
    out = []
    for a in BOT_ANSWERS:
        bad = unsupported(a, BOT_RECORDS + ["asking for your response, a reply, an answer, input"])     # the asks-you state's words (check.rs)
        out.append({"answer": a, "unsupported": bad})
    return {"answers": len(BOT_ANSWERS), "refused_by_g11": sum(1 for x in out if x["unsupported"]), "each": out,
            "general_answers_not_checked_here": len(BOT_GENERAL)}
def selftest():
    fails = []
    def check(name, cond, d=""):
        if not cond: fails.append(f"{name}: {d}")
    cl = ["I moved the garnet importer to the streaming parser", "the nightly import no longer loads the whole file into memory", "Tests were not run."]
    for t, want in (("smaller assets speed up loading for every user", ["speed"]), ("big imports stop running out of memory overnight", []),
                    ("which fixes the crash in the import", ["fixes"]), ("because the parser streams", ["because"]), ("all 12 tests pass", ["12", "pass"]),
                    ("so the import no longer loads the whole file", [])):
        got = unsupported(t, cl)
        check(f"G11 {t!r}", got == want, got)
    check("v8 B's seven, each refused", all(unsupported(t, ["Added a drag handle to the lantern board; tidied the CSS vars."]) for t in
          ("drag handle improves interaction, tidy vars reduce CSS noise", "smaller assets speed up loading for every user",
           "standard parser ensures consistent date handling across projects", "makes uploads more reliable after flaky timing")))
    print(f"g11 selftest: {'ok' if not fails else 'FAIL'}"); [print("FAIL", f) for f in fails]
    return not fails
if __name__ == "__main__":
    if "--digest-v6" in sys.argv: print(json.dumps(digest_v6(), indent=1, ensure_ascii=False))
    elif "--bot-e3829" in sys.argv: print(json.dumps(bot_e3829(), indent=1, ensure_ascii=False))
    else: sys.exit(0 if selftest() else 1)
