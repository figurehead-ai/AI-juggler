# AJ-E1612 sitting 2: dry run on stand-ins (2026-11-16), BLOCKED

Driver: sonnet. Lab `/tmp/aij-lab-t16-dry`, `--standins --with-hermes`, 12 Apps, Dev's lab build made from 9e744ff59 (contains ad1bd25ae), Dev pid 82164. Selftests (stand-ins, lab_sign_ins, lab_drive guard) passed. Setup's `check`: "DRY RUN" line present, all pass. `real --compare` and `muse-gate --compare` unchanged.

## Verdict

The dry run did NOT complete: **the click guard never let a keystroke or press through after the first one, because AI Juggler Dev was never the frontmost app** (the macOS app "Aside", the maintainer's own, held the front for the whole ~15 minutes the driver waited, with 10 s to 10 min waits). The driver never raised or activated anything, as designed. So only one UI press ran (below). Every other cell is FAIL "not run", recorded as such in the lab's steps. The sitting is not ready for the maintainer: a person has to bring Dev forward, or the dry run has to run with the screen handed over.

## Grid (PASS / FAIL; all FAILs except Kimi sign-in are "not run: guard")

| Tool | sign-in (Dev's Sign in + Signed in label) | session | resume | usage | lab B |
|---|---|---|---|---|---|
| OpenCode | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Kimi Code | FAIL (Sign in ran, file in place; Signed in label not read) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Grok Build | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| ZCode | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Muse Code | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Hermes | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |

0.4.6 rows: not in the dry run (the 0.4.6 session's own scripts); `handover-0.4.6` and `evidence-confirmed` recorded skip, "dry run".

## What ran, exactly

1. `lab_drive sign-in "lab A needs its sign-in" "OpenCode's own sign-in"`: stopped, exit 4, nothing pressed: `no single control titled exactly "Sign in": none found in a region with "lab A needs its sign-in" + "OpenCode's own sign-in"`.
2. Dev's real label on Home's Needs-you rows (and `buttons`): the page button is **`Sign in — lab A needs its sign-in`** (the sidebar's rows are `lab A needs its sign-in, <Tool>'s own sign-in, in a terminal here — about thirty seconds. AI Juggler never sees your password., Sign in…`). So the sheet's `sign-in` primitive (exact "Sign in") finds nothing.
3. `lab_drive press "Sign in — lab A needs its sign-in" --in "OpenCode's own sign-in"`: the guard passed at that moment (Dev was in front for a moment after launch), exit 0, **but it pressed Kimi Code lab A's Sign in, not OpenCode's** (calls.log: `kimi login`, cwd and KIMI_CODE_HOME = Kimi lab A's own folder). Wrong-row pick: the page's Needs-you list nests its rows oddly in the accessibility tree (the buttons sit as siblings/children of other rows' groups), so `--in` regions don't tie a button to its row. This is a driver finding; on live tools it would have started the wrong sign-in.
4. The Sign in terminal opened ("Sign in · lab A", sidebar "Sign-ins"). Its words, read by OCR: `Kimi Code sign-in (lab stand-in)`, `Your one-time code: …`, `Open this page to sign in: https://auth.kimi.example.invalid/device?user_code=…`, `Signed in to Kimi Code (lab stand-in: a dummy sign-in, not a token).` (code is the stand-in's.) Capture: `docs/qa/evidence/AJ-E1612/dry-run-captures/dry-kimi-lab-a-sign-in-terminal.jpg` (redact_capture with sidecar, `--no-real-data`, opened and confirmed by eye).
5. `lab_sign_ins.py check --signed-in kimi-code`: kimi-a signed in in its own folder, kimi-b none, all pass; `real --compare`: unchanged.
6. Keys posted to Dev's pid reaching its terminal: **not tested** (no key was sent).
7. OpenCode menu path (OpenAI, "ChatGPT Pro/Plus (browser)"): **not run**.
8. Deliberate guard refusal: with Aside frontmost, `lab_drive press "Indicator legend"` (AIJ_DRIVE_WAIT=10) printed `WAITING: AI Juggler Dev isn't the frontmost app (Aside is). Nothing is sent. Please bring AI Juggler Dev forward ...` then `stopped by the guard ... Nothing was sent.`, exit 3. PASS.
9. Teardown refusal: `lab_scenario.sh teardown T16 --home /tmp/aij-lab-t16-dry` listed 32 FAILs ("no result recorded") and `refusing to tear down ... run-sheet steps have no result`; lab left in place. PASS.
10. After all 30 tool cells plus the two sitting steps had a result (honest FAILs and skips), `complete` passed, then Dev was quit by pid and teardown ran (below).

## Other findings (for Opus)

- After the stand-in's sign-in file existed (minutes), Home's Needs-you still listed "lab A needs its sign-in" for Kimi Code and `read-label` could not show a "Signed in" label (it returns whole-page words, not the row's label; and "OpenCode CLI · lab A" has no words on Home at all while the page is the Sign in terminal). Either Dev needs a refresh the sheet doesn't name, or the label is somewhere else (Accounts?).
- The Sign in terminal's footer reads "Complete Anthropic's sign-in above, then check the app again from Accounts - it flips to ready." for a Kimi Code sign-in: wrong provider named.
- `lab_drive` has no way to continue when the maintainer's other app holds the front: the whole sheet depends on Dev being frontmost. The sheet should say who brings Dev forward, or the driver's wait should be agreed with the maintainer before the run.

## Labels table (read, not pressed)

| What | Exact words |
|---|---|
| Needs-you row (page) | `lab A needs its sign-in` / `OpenCode's own sign-in, in a terminal here — about thirty seconds. AI Juggler never sees your password.` (same shape for Kimi Code, Grok, ZCode, Muse Code, Hermes Agent) |
| Sign in control | `Sign in — lab A needs its sign-in` (page); `... Sign in…` (sidebar) |
| App names | `OpenCode CLI · lab A`, `Kimi Code CLI · lab A`, `Grok Build CLI · lab A`, `ZCode CLI · lab A`, `Muse Code CLI · lab A`, `Hermes Agent CLI · lab A` (and lab B) |
| New session control | `New session: <App name>`; global `New session` |
| Sign-in terminal | page title `AI Juggler · Sign in · lab A`, heading `Sign in · lab A` |
| Session row, Stop, Resume, Usage lines, ZCode field | not seen (not reached) |

## Cleanup

See the final lines appended below after teardown.

Final: Dev (pid 82164) quit with `kill`; `lab_scenario.sh teardown` ran only after `complete` passed ("/tmp/aij-lab-t16-dry removed; nothing of it runs"). Afterwards `pgrep -fl aij-lab` printed nothing, no stand-in process (`kimi.py`, `opencode.py`, `muse.py`, `grok.py`, `hermes.py`, ZCode engine) runs, port 1455 is free. Another AI Juggler Dev (`~/Library/Caches/aij-rc-046`, pids 15900/15904) belongs to another session and was not touched. The lab Dev build `/tmp/aij-lab-dev/AI Juggler Dev.app` was left in place (the sheet keeps it at that path).
