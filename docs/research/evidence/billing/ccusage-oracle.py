#!/usr/bin/env python3
"""AJ-E609 ccusage-pattern cross-check oracle (independent of Rust).

Prices the SAME fixture corpus the Rust cost ledger prices, with the
SAME bundled seed rates, by independent python arithmetic (the ccusage
aggregation pattern: per-message usage × per-model rates). Run:

    python3 docs/research/evidence/billing/ccusage-oracle.py

Output is the corpus total; gate-M3.md records corpus hash + both
totals + delta (must be $0.00 to the cent).
"""
import hashlib
import json

CORPUS = "crates/juggler-adapters/tests/fixtures/quota/claude-transcript.jsonl"
RATES = {  # bundled seed v2 (per 1k tokens), from cost_ledger_seed.json
    "claude-opus-4": {"in": 0.015, "out": 0.075, "cr": 0.0015, "cw": 0.01875},
}

corpus_bytes = open(CORPUS, "rb").read()
corpus_hash = hashlib.sha256(corpus_bytes).hexdigest()
total = 0.0
messages = 0
seen_uuids = set()
for line in corpus_bytes.decode().splitlines():
    if not line.strip():
        continue
    record = json.loads(line)
    usage = (record.get("message") or {}).get("usage")
    uuid = record.get("uuid")
    if not usage or not uuid:
        continue  # the ledger's own law: unkeyed usage cannot dedupe
    if uuid in seen_uuids:
        continue  # the ccusage resumed-file dedupe (first wins)
    seen_uuids.add(uuid)
    model = (record.get("message") or {}).get("model")
    rates = RATES.get(model)
    if rates is None:
        continue  # unpriced model: tokens only, never $0.00
    messages += 1
    total += (
        usage.get("input_tokens", 0) / 1000 * rates["in"]
        + usage.get("output_tokens", 0) / 1000 * rates["out"]
        + usage.get("cache_read_input_tokens", 0) / 1000 * rates["cr"]
        + usage.get("cache_creation_input_tokens", 0) / 1000 * rates["cw"]
    )
print(f"corpus={CORPUS}")
print(f"sha256={corpus_hash}")
print(f"priced_messages={messages}")
print(f"total_usd={total:.7f}")
