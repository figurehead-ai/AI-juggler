# Evidence manifest — AJ-E1612

Task: `docs/tasks/E16-more-tools.md` AJ-E1612 (the new tools' live check, D-107).
Run sheet: [`docs/qa/0.5-walks/AJ-E1612.md`](../../0.5-walks/AJ-E1612.md) (sitting 2); the maintainer's page beside it.
Status: **IN PROGRESS.** Sitting 1 (2026-11-16) proved step 1 only. The tools for sitting 2 are built: the guarded driver, stand-ins, setup switches and teardown guard. The dry run on stand-ins comes next (sonnet), then sitting 2.
2026-11-16, reviewer claude-opus.

## The sitting's tools (built 2026-11-16)

- `scripts/qa/lab_drive.swift`: drives AI Juggler Dev by pid only (refuses any pid that isn't an `AI Juggler Dev.app`). Every press and key goes through the click guard: Dev must be frontmost, and its window must be topmost at the point. If not, it asks and waits, and never activates anything. Controls are matched by exact title within a region named by its words. Keys are posted to Dev's pid. It waits for text by accessibility, or by OCR of the terminal's canvas. Captures go by window id through `redact_capture.swift`.
- `scripts/qa/standins/`: stand-ins for OpenCode, Kimi Code, Grok Build, ZCode, Muse Code and Hermes. They are lab-only, refuse the network, and use fake pages on `example.invalid`. `install.py` sets them up, and `lab_scenario.sh setup T16 --standins` points Dev at them through the adapters' test overrides. `lab_env.sh` passes those overrides only into `<lab>/standins`.
- `lab_scenario.sh --with-hermes/--no-hermes`: T16's Hermes Apps (`setup.optional_apps`, D-123) are seeded only when asked, or once Hermes is installed. `lab_sign_ins.py` counts Hermes only in such a lab.
- `lab_sign_ins.py`:
  - `complete` also waits for the sitting's two steps: the 0.4.6 rows, and the evidence confirmed.
  - `real` compares the real `~/.codex/auth.json` by digest, size and date. The digest is kept in the lab only.
  - `check` fails a live lab whose Sign in runs a stand-in.

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Step 1, separate sign-in, five tools (sitting 1: in a terminal with `sign-ins.json`'s command, not Dev's Sign in) | `lab_sign_ins.py check --signed-in …`, `real --compare`, `muse-gate --compare` ([sitting-2026-11-16](sitting-2026-11-16.md)) | lab A only, real files unchanged | all five: lab A only; real files and the Muse Keychain date unchanged | PASS | 9a007e84c | 2026-11-16 | claude-opus |
| Step 1 through Dev's own Sign in and its "Signed in" label | run sheet "Per tool: step 1" with `lab_drive sign-in` / `read-label` | lab A Signed in, lab B Not signed in, `check` passes | not run yet | UNPROVEN | — | 2026-11-16 | claude-opus |
| OpenCode signed in with ChatGPT (OpenAI → ChatGPT Pro/Plus (browser)), not an API key | run sheet step 1, OpenCode's menu (the active row's hint checked) | an oauth sign-in | sitting 1 took the default (an API key) | UNPROVEN | — | 2026-11-16 | claude-opus |
| Steps 2–4, 6 per tool: session starts, resumes, usage read, lab B stays out | run sheet "steps 2–6"; `lab_sign_ins.py step` per result | every step has a result, checked by Opus | not run yet | UNPROVEN | — | 2026-11-16 | claude-opus |
| The teardown waits for every result, plus the sitting's own two steps | `python3 -I scripts/qa/lab_sign_ins.py --selftest`; `lab_scenario.sh teardown` refuses while `complete` fails | selftest passes | selftest passes | PASS | db81d2c27 | 2026-11-16 | claude-opus |
| The click guard: nothing is sent unless Dev is frontmost and topmost at the point; exact titles in a region, no prefix, ambiguity refused; a non-Dev pid refused | `swiftc -O -o $S/lab_drive scripts/qa/lab_drive.swift && $S/lab_drive --selftest`; `AIJ_DEV_PID=<Finder's pid> $S/lab_drive guard` | selftest passes; exit 2 | passes; "isn't AI Juggler Dev … refusing", exit 2 | PASS | 5ba4d5cce | 2026-11-16 | claude-opus |
| The guard live: Dev behind another window gets `WAITING`, and nothing is sent | the dry run's on-purpose guard check | WAITING printed, no press | not run yet | UNPROVEN | — | 2026-11-16 | claude-opus |
| Stand-ins: each answers its adapter's commands in a lab only, with no network | `python3 -I scripts/qa/standins/install.py --selftest` (each stand-in's selftest + an install into a throwaway lab) | all pass | all six pass | PASS | 6ed14639c | 2026-11-16 | claude-opus |
| The dry run of the whole sheet on stand-ins (sonnet), before the maintainer is invited | run sheet "The dry run" → `dry-run.md` with the labels table | every step with a result; findings fixed | not run yet | UNPROVEN | — | 2026-11-16 | claude-opus |
| Muse's usage read runs on the App's file backend (never the Mac's Keychain sign-in) and only on the proven build | ad1bd25ae; `cargo check` + the Muse adapter tests (pending: the machine's CPU window) | builds, tests pass | written, not yet built | UNPROVEN | ad1bd25ae | 2026-11-16 | claude-opus |
| Hermes (AJ-E1607) | skipped, waiting for install (D-123); its Apps, steps and stand-in are ready behind `--with-hermes` | — | — | UNPROVEN | — | 2026-11-16 | claude-opus |
| AJ-E1610's desktop-app parts | skipped: the sheet forbids opening the tools' own app windows | — | — | UNPROVEN | — | 2026-11-16 | claude-opus |

## For the maintainer

- Sitting 2 starts only after the dry run passes. Your part: one browser page per tool, one at a time, 15 minutes each (your page lists them).
- The Muse fix (ad1bd25ae) must be in the lab build before Muse is signed in. Without it, Dev's usage read of a signed-in lab Muse would use your Keychain sign-in.

## Redaction checklist

- [x] No credentials, tokens or keys (stand-ins write `lab-not-a-token` only, in a lab)
- [x] Paths generalised (`<lab>`, `~`)
- [x] No transcript content; OCR output is never recorded
