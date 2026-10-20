# Evidence manifest — AJ-E1407

Task: `docs/tasks/E14-machine-health.md` AJ-E1407 (the machine launch
advisor in New session). Status: **DONE-EVIDENCED**.

## What exists now

- `machine_advice` (one verdict per launch setting) from the pure
  `advise()`:
  - **No**: critical memory pressure, or under 5 % of memory left after the
    launch, or a "pressure"-level CPU, thermal or swap reason.
  - **Tight**: memory-pressure warning, under 15 % left, or a CPU, thermal
    or swap reason.
  - **OK** otherwise.
  - The projection is the setting's median peak memory from ≥ 3 footprints,
    else 1 GB, labelled "not measured yet".
  - When not OK and the heaviest running session holds ≥ 500 MB, the advice
    names it: "Closing feat/read-model frees about 3.2 GB."
- The sheet shows it under the preselected launch setting, beside the
  account's own quota verdict: two questions, two answers, never one merged
  colour. Rows under Customise get a "This Mac: tight / not now" chip only
  when not OK. Every non-OK verdict ends "You can still start it." Launch is
  never disabled by it.

## Walked by hand (built-in browser, mock bridge)

At 1440×900, dark (`?aij_machine=tight&aij_load=heavy`), the Tight line
shows the reasons and "Closing feat/read-model frees about 3.2 GB. You can
still start it.", and Launch is enabled. Under Customise all three rows show
"This Mac: tight". In light, plenty: "This Mac can take another session ·
about 2.1 GB a session, from your last 6".

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1–2 rules, reasons, relief | `cargo test -p juggler-app --lib -- machine_health` (`the_advisor_says_ok_tight_or_no_and_why`, `until_three_sessions_are_measured_the_projection_is_the_default`) | pass | pass | PASS | 49722aad+ | 2026-10-20 | claude-opus |
| Req.3 warns, never blocks | `pnpm --dir ui exec vitest run src/components/machine-advice.test.tsx` (a "no" verdict: Launch enabled and launches); gallery spec asserts Launch enabled in every state | pass | 4 passed; 8 passed | PASS | 49722aad+ | 2026-10-20 | claude-opus |
| Req.4 projection ±20 % (SPEC §1) | `cargo test -p juggler-app --test machine_fixtures -- --nocapture`: three real memory-hog sessions of one setting (260, 300, 340 MB) teach the footprint; a fourth (330 MB) is checked | ≤ 20 % | projection 312 MB vs observed peak 342 MB, error 8.8 % (twice, identical) | PASS | 49722aad+ | 2026-10-20 | claude-opus |
| Gallery + axe | `pnpm --dir ui exec playwright test tests/visual/launch-advisor-gallery.spec.ts` → [`_gallery/73-launch-advisor/`](docs/qa/evidence/_gallery/73-launch-advisor/) (ok, not measured, tight, no × light/dark) | 0 critical/serious | 8 passed | PASS | 49722aad+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
