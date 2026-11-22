# Evidence manifest — AJ-E3709

Task: AJ-E3709 (on-device summary model selection, D-120 B, ADR-0033). This round: the evaluation set, the harness, and Apple's Foundation Model as the control. The open models wait for the maintainer's download yes. Verdicts by Sonnet; Opus checks every FAIL before it is acted on. Full write-up: [model-eval.md](model-eval.md).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Evaluation set built: about 100 redacted real agent-written messages plus adversarial cases, each with its source | `python3 harness/build_corpus.py <repo> corpus.json` → [corpus-424cce8f.json](corpus-424cce8f.json) | about 100 items; 25 short, 40 medium, 25 long, 10+ adversarial | 104 items: 25, 40, 25, 14; sources recorded per item; any body with a name, email, path, link or key dropped | PASS | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Harness runs a model command and returns outputs, latency, memory and check results | `python3 harness/runner.py --label apple --cmd probe/apple_probe --corpus corpus.json --out results-apple.json` | outputs + latency + peak memory + checks per item | 104 results per run, timed per request, memory sampled every 100 ms; energy not measured (`powermetrics` needs root) | PASS | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Apple's model, ADR-0033's prompt: non-answers at most 1% | same, results in [results-apple-6850a6a8.json](results-apple-6850a6a8.json) | at most 1% | 29 of 104 (27.9%) answered "I am a foundation model created by Apple" | FAIL | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Apple's model, text wrapped in a labelled block: non-answers at most 1% | same with `--wrap`, results in [results-apple-wrapped-057ae5f2.json](results-apple-wrapped-057ae5f2.json) | at most 1% | 1 of 104 (0.96%) | PASS | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Apple's model (wrapped): format, one sentence of at most 20 words | same | at least 98% | 95 of 104 (91.3%) | FAIL | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Apple's model (wrapped): polarity errors | same, plus a by-eye read | at most 1 per 100 | 6 in 104 (5.8 per 100), two of them obeyed injections | FAIL | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Apple's model (wrapped): entity check, no invented number or identifier | same | 0 invented; at least 95% pass | 102 of 103 answers (99.0%); 1 flagged (a counted "Two", benign to a reader) | PASS | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Apple's model: stays on the Mac, never Private Cloud Compute, no refusal retried | probe source imports Foundation and FoundationModels only; `SystemLanguageModel.default` | no off-Mac path | no network code; 0 refusals, so the permissive retry never fired | PASS | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Qwen3-1.7B against every gate | `runner.py --label qwen3-1.7b ...` | the research plan's hard gates | PENDING: awaiting the maintainer's download yes | UNPROVEN | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Nemotron 3 Nano 4B against every gate | `runner.py --label nemotron-3-nano-4b ...` | the research plan's hard gates | PENDING: awaiting the maintainer's download yes | UNPROVEN | b5b3827e8 | 2026-11-16 | claude-sonnet |
| Mean faithfulness score against Apple's (HHEM and a second scorer) | not run | at least Apple's | PENDING: the scorers are model downloads | UNPROVEN | b5b3827e8 | 2026-11-16 | claude-sonnet |
| v2 run: guard removes nothing from the 90 commit bodies; removes A06 and A07's injections | `python3 harness/report_v2.py results-apple-v2-66d0c729.json corpus-v2-8f7c7474.json` | 0 removed from the 90 | 0 of 90; 3 sentences removed from A06 and A07 | PASS | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: shown lines, invented numbers or identifiers | same → [report-apple-v2-77141184.json](report-apple-v2-77141184.json) | 0 | 0 of 312 trials, all three facts settings | PASS | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: shown lines, obeyed injection, question turned into a decision, failure lost | same | 0 each | 0 each, all three settings | PASS | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: shown lines, polarity errors by script | same | at most 1 per 100 | 3 of 312 (1.0 per 100; all A11, a line silent on "tests not run"); 1.9 with facts unknown | UNPROVEN (at the bar; Opus reads A11 and the blind review) | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: drafts, format (one sentence, at most 20 words) | same | at least 98% | 165 of 306 (53.9%) | FAIL | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: drafts, non-answers | same | at most 1% | 3 of 306 (0.98%, one item) | PASS (at the bar) | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: drafts, entity check | same | 0 invented; at least 95% | 303 of 303 | PASS | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: what the user sees (main) | same | report | model line 46.2%, agent's own sentence 31.7%, excerpt 22.1% | RECORDED | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: v1's misses replayed through the gate | `python3 harness/replay_v1.py ...` → [replay-v1-misses-5f09b8bd.json](replay-v1-misses-5f09b8bd.json) | each rejected | 9 of 9 rejected (A07 and A01 only by the facts) | PASS | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: latency (M5 Ultra, loaded, not an M1) | same | median at most 3 s, p95 at most 6 s | write median 1.16 s, p95 1.78 s; path median 1.39 s, p95 2.31 s | PASS on this Mac only | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: energy | `ps` CPU time and `top` Energy Impact | estimate | 160 CPU-s over 556 s for 624 calls; GPU and Neural Engine not visible | ESTIMATE | PENDING | 2026-11-16 | claude-sonnet |
| v2 run: blind review by Opus | [blind-review/input-v2.md](blind-review/input-v2.md) (key: [blind-review-key/key-v2.json](blind-review-key/key-v2.json)) | 47 entries scored | not scored | UNPROVEN | PENDING | 2026-11-16 | claude-sonnet |

## Binary artifacts

Content-addressed JSON, listed in the rows above: `corpus-424cce8f.json`, `results-apple-6850a6a8.json`, `results-apple-wrapped-057ae5f2.json`, `corpus-v2-8f7c7474.json`, `results-apple-v2-66d0c729.json`, `report-apple-v2-77141184.json`, `replay-v1-misses-5f09b8bd.json`. The harness sources are in `harness/` (stdlib Python and the throwaway Swift probe; the compiled probe was deleted).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran; the one key-shaped string, in adversarial case A05, is fake)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (commit bodies with names, emails, paths, links or keys were dropped)
