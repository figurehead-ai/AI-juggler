# Evidence manifest — AJ-E1022

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1022 (Add account
without the old wizard: a fresh account, a folder, D-25). Status:
**DONE-EVIDENCED**. 2026-10-20, reviewer claude-opus.

## What exists now

- `FirstRun` has two more first steps beside "what's here":
  - **A fresh account**: "Add a fresh account." with a Name field, focused
    and filled in with a free name ("Claude Code", "Claude Code 2", …);
    Enter or Create and sign in creates it and goes to the same sign-in
    step; Back (when there was a list) and "Import from a folder
    instead…".
  - **A folder**: "Import from another folder…" opens the system picker
    (cancel: nothing changes). The check shows the path, files and size,
    signed in or not, the Keychain note, and the breakdown behind "What's
    in it". Bring it in copies it (`account_import`, origin manual) and
    goes to its sign-in when needed. Not a setup (alert, no Bring it in),
    already in AI Juggler, too large (Bring in settings only / Add a fresh
    account instead), already imported ("Already imported as «Work»",
    Done) and moved or vanished (pick it again) each say so plainly.
- Accounts › Add account opens these in add mode (straight to a fresh
  account when nothing new is on this Mac). New project's "Add an account"
  opens the same over the sheet; Escape returns to the sheet with its
  folder still chosen. First run's two links open the steps instead of
  the wizard.
- Retired: `OnboardingWizard.tsx` (2,412 lines) and its test file, the
  `__AIJ_FIRST_RUN__ = "wizard"` seam in five specs, `wizard-first-install`
  and `first-run-policy` specs, the `PackPolicyChooser` component (its
  options and restatement stay for Settings), the bridge wrappers only the
  wizard called (`recommendationFor`, `scanCancel`, `isolationCheck`) and
  the wizard's gallery (16, 34, 35, 36, 37 flow a, 40-login summary, 41
  first-run policy, 44 onboarding card, 63 first-run consent). The backend
  commands behind those wrappers are untouched.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 fresh | vitest `first-run.test.tsx`: free names; name focused and filled, rename, Create and sign in → one sign-in row → Sign in opens the login terminal; empty name disabled; add mode with nothing new goes straight here (no Back, the folder link present); nothing found → Add an account opens it. `keyboard-flows.spec.ts`: Add account → Enter creates → Sign in reached by Tab → Escape | PASS |
| Req.2 folder | vitest: picker cancelled changes nothing; the check (128 files · 43.9 MB, ~/old-claude, Signed in, Keychain, breakdown, name); Bring it in → its sign-in. Capture `39-import/f-normal`, `f-signin` | PASS |
| Req.3 plain failures | vitest: too large (settings only / fresh), not a setup (role=alert, no Bring it in), duplicate (Done closes), moved (pick again → the new folder is checked). Captures `f-oversize`, `f-non-config`, `f-duplicate`, `f-moved`; a11y "confirm-source" scan on the check and on the rejected folder (alert) | PASS |
| Req.4 one place, no pack question | vitest `first-run-policy.test.tsx`: bring-in, fresh and folder steps never show the pack question and write no policy; `new-project-sheet.test.tsx` and `new-project.spec.ts` (detour over the sheet; Escape returns, folder kept); `keyboard-flows` first run end to end | PASS |
| Req.5 funnel, flow (d) | vitest: every event of all three paths has allow-listed, free-text-free fields; `onboarding.login` (AJ-E1103) counts completed, skipped and expired sign-ins with the tool only (it was the wizard's); `flows-cde.spec.ts` flow (d) = 3 primary clicks to the sign-in | PASS |
| Req.6 retired with tests | the inventory in the PR maps each wizard test to "survives (where)" or "went with the wizard" | PASS |
| Full suites | vitest 452/452; Playwright 813/813 (the wizard-only tests are gone, see the PR); a11y gate 828 states, 0 critical/serious; visual gate re-rendered (Accounts: the separate tool button is gone) | PASS |

Gallery: `39-import/`, `37-flows/b-detour-*`, `71-add-account/`.

## Not covered here

- **Usage-statistics consent in first run** (D-22, AJ-E1105): first run
  stopped offering it when it was rebuilt (#52); only the wizard still
  did. Statistics stay off until chosen in Settings › Privacy. Where it
  should be asked again is the maintainer's call (follow-up).
- The optional Figurehead account card (AJ-E1203) and a renewal date at
  setup (D-12) went with the wizard; both stay in Settings.
- A fresh account no longer asks for its plan; it is set in Accounts.
