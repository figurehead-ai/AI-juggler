Harness v5 round 2, jobs a-d (feed line, review, attention, chat), six models, one repeat. Synthetic held-out only; not checked on real messages.
Sets: heldout/v5r2/turns-v5r2.json (feed_line, review, attention), heldout/v5r2/chat-v5r2.json (chat). Run by Sonnet; criticals/omissions are script candidates (candidates-<model>/), pending Opus.
Qwen3-4B-Instruct-2507 = THIRD-PARTY GGUF conversion. Gemma 4 E2B run with --sampling low; card sampling UNVERIFIED for min_p/presence_penalty.
Blind packets: blind-review/v5r2-<model>/; keys (never opened): blind-review-key/v5r2-<model>.json. Facts-hidden attention is not a v5 group (v5 decides attention from the facts).
