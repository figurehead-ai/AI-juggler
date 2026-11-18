Harness v5 round 2, jobs e (digest) and f (watch), six models, one repeat. Synthetic held-out only; not checked on real messages.
Sets: heldout/v5r2/digest-v5r2.json (30), heldout/v5r2/watch-v5r2.json (60). Run by Sonnet; criticals/omissions are script candidates (candidates-<model>/), pending Opus.
Quiet-hours accuracy on watch-v5r2 is not blind (Opus read the 8 quiet-hours requests while adding the gold field, 0a333a536).
Qwen3-4B-Instruct-2507 = THIRD-PARTY GGUF conversion. Gemma 4 E2B run with --sampling low; card sampling UNVERIFIED for min_p/presence_penalty.
Blind packets (digest): blind-review/v5r2ef-<model>/. The report tool wrote no key files (blind-review-key/v5r2ef-* does not exist); nothing was opened.
Run: runner_v5.py --job digest,watch with corpora digest5r2 and watch5r2 (HEAD includes ec53c1abe and 0a333a536; gate.py unchanged vs 7acec4f50; selftest_ef 146/146, selftest_v5 261/261).
