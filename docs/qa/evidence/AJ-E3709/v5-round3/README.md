Harness v5 round 3, jobs a-d (feed line, review, attention, chat), six models, one repeat, fresh blind sets. Synthetic held-out only; not checked on real messages.
Code: snapshot of committed HEAD 162a3e055 (git archive; working-tree edits to digest.py/watch.py excluded). gate.py identical to 7acec4f50; selftest_v5 368/368 in the snapshot (run with GIT_DIR/GIT_WORK_TREE pointing at the repo, because the selftest needs a git root). HEAD contains 084822ce0, ad50d2232, b44735916.
Facts guard: accepted the round-3 sets at load; no refusal, no FactsUnmapped stop (all six runs rc=0).
Sets: heldout/v5r3/turns-v5r3.json (feed_line, review, attention), heldout/v5r3/chat-v5r3.json (chat). Run by Sonnet; criticals/omissions are script candidates (candidates-<model>/), pending Opus.
Qwen3-4B-Instruct-2507 = THIRD-PARTY GGUF conversion. Gemma 4 E2B run with --sampling low; card sampling UNVERIFIED for min_p/presence_penalty.
Blind packets: blind-review/v5r3-<model>/ (feed_line, review, chat; no attention packet: v5 decides attention from the facts); keys (never opened): blind-review-key/v5r3-<model>.json. The tool wrote each key inside its packet folder; moved unopened.
Apple probe binary was newer than the last apple_probe.swift commit; not rebuilt.
