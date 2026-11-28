# Evidence manifest: AJ-E1555 (prime only a system prompt that can appear; every priming sheet says something real)

Built 2026-11-25 on release/0.5.2 (Opus). Unit tests (Rust, os_prompts and code_trust) and vitest
(the sheet). No real helper was started and no system prompt was caused: the "differs" check reads
only files in temp folders and the app's own settings.

R1: PASS — `os_prompts::tests::helper_changed_only_when_its_signature_on_disk_differs` (never for a helper never opened here, never because a Dev data folder sits beside ours, true after the file on disk changed, false again once the changed one opened, and the next change is told again), `os_prompts::tests::a_changed_helper_waits_for_its_one_sheet`, `juggler_platform::code_trust::tests::a_helpers_signature_is_its_team_or_its_bytes`. `needs::helper_changed` = the helper can run and its signature on disk (`team:<team>`, or the file's hash for ad-hoc code) differs from `os_prompt_helper_opened:<helper>`, kept after each hello (`helper::launch_checked` → `os_prompts::helper_opened`). The Dev-folder rule (`helper_renamed_here`) is gone.
R2: PASS — the registry gained `after` (what works now, per entry); `os_prompts::outcome_words` builds every outcome body from the registry (`after`, `if_declined` with where to change it, `offered_again`); `os_prompts::tests::every_sheet_says_something_real` fails on a repeated title or a placeholder/state-word body for every entry × status; vitest `os-prompt-sheet.test.tsx` "never repeats its title or shows a state word as its body" (heading = registry title in every phase, the line under it never the title, no "Asked."/"Allowed"/empty body). The preview's outcome no longer says "Asked.".
R3: PASS — the helper's priming call returns what came of the start (`helper_outcome`: started → the entry's `after`; blocked → `if_declined`; no answer yet → said so, never "asked"); the call itself is the action (`mac_checked`, the helper's start). Done closes the sheet; Check again then checks (`models-actions.checkAgain`), unchanged.

## Usability answers (PAT-USE), as built

1. Clicking "Use Apple's AI" / Check this Mac starts the check; a sheet shows only when macOS can really ask: match — `helper-changed` is needed only for a changed signature; a Nightly start beside a Dev build no longer primes it.
2. The sheet names the prompt, why, what to click, from the registry: match — heading = title, body = why + dialog picture + what to click; after: the registry's `after`/`if_declined`.
3. It replaces nothing; the gate stays the one way prompts are asked: match — same gate, same ledger.
4. No prompt can appear: no sheet, the action runs: match — `helper_start_through(gate, false)` passes once `helper` is primed.
5. This is the priming itself: match.
6. Watch for a sheet promising a prompt that never comes, placeholder words, a sheet twice: covered by the three tests above; Continue on `helper-changed` primes `helper` too (one sheet).

PAT-SURF/PAT-COLOR: the sheet's frame and colours are unchanged (DLG-1); only its words moved. PAT-OSP: the registry doc (`docs/design/os-prompts.md`) says the new rule. PAT-SER-7: no internal terms on screen.

## Verdicts

| Req | Verdict | Date |
|---|---|---|
| R1 | PASS | 2026-11-25 |
| R2 | PASS | 2026-11-25 |
| R3 | PASS | 2026-11-25 |
