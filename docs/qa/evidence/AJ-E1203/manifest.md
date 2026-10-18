# Evidence manifest — AJ-E1203

Task: `docs/tasks/E12-optional-account.md` AJ-E1203 (Settings → Account
& Cloud, onboarding step, Cloud chips, consent screen). 2026-10-18,
reviewer claude-opus. D-16 (Appearance), D-18, ADR-0014.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| A Settings screen with Appearance (System / Light / Dark, in sync with ⌘⇧L) and Account & Cloud | Req.1 | `ui/src/screens/Settings.tsx`; top-right **Settings** and ⌘, (the macOS convention). Appearance is a labelled radio group; `theme.ts` now fires `aij:theme-changed` on every change so ⌘⇧L and Settings always agree (vitest `follows ⌘⇧L and writes the choice`) | PASS | 2026-10-18 | claude-opus |
| Signed out: two paths in one paragraph; Connect account + Use a pairing code; Cloud chips open the section; no nagging | Req.2 | Signed-out copy leads with "You can do everything in AI Juggler without an account" and says Provider sign-ins never leave the Mac (vitest). Fleet shows one quiet "Find your Macs through your Figurehead account · Cloud" chip that opens Account & Cloud; Backups and usage stats get theirs when those features exist. No pop-ups, banners or reminders anywhere | PASS | 2026-10-18 | claude-opus |
| Onboarding: one optional step after the policy choice, equal buttons, skipping is the default and costs no click | Req.3 | A card on the Summary: "Connect a Figurehead account (optional)" with **Connect account** and **Continue without an account** (both secondary). The Summary's own exits work without touching it — the unchanged `flow (a) … stays ≤ 5 clicks, 0 typed` test still passes; skip hides it for good (`Continue without an account hides the card for good`); Connect lands on Account & Cloud | PASS | 2026-10-18 | claude-opus |
| Consent screen "Turn on Cloud for this Mac": per-feature toggle, what it does, exactly what it sends; defaults backup ON, discovery ON, usage sync OFF and disabled; stored with timestamp + `cloud_consent_version`; re-shown on a version bump | Req.4 | vitest `the consent screen defaults …`; every row carries "Sends: …"; backend `figurehead_save_choices` stores choices with `consented_at_ms` and `CONSENT_VERSION` (1); the screen shows while `consent_version < consent_current`; a feature the plan lacks — or usage sync before E11 — is stored off (`consent_stores_only_what_the_plan_and_this_build_allow`). Backup and discovery say plainly that they arrive in an upcoming build and that the choice is kept | PASS | 2026-10-18 | claude-opus |
| Connected: e-mail + plan, editable Mac name, toggles, Open dashboard, Unlink, delete-data link; paid-and-missing = disabled + Upgrade | Req.5 | Gallery `44-settings/connected`, `paid-locked-offline`; vitest `a paid feature the plan lacks is a disabled toggle with Upgrade`; Unlink is an alertdialog with Cancel focused that says what stays (`Unlink asks first …`). The offline state says the Mac keeps working and when the account was last refreshed | PASS | 2026-10-18 | claude-opus |
| Copy never implies the desktop path is lesser; says what is sent and that it's encrypted | Req.6 | Copy review across all states; "Sends:" on every feature; backups and config "Encrypted … Figurehead can't read it"; the unlinked notice adds "Everything on this Mac is unchanged" | PASS | 2026-10-18 | claude-opus |
| Captures: signed out, connecting, consent, connected, paid-locked, unlinked, offline × 1280×800 + 1024×680 × light/dark; onboarding step at 1280×800 + 390×844 | visual | Gallery `44-settings` (signed-out, pairing-code, linking, consent, connected, paid-locked-offline, unlink-confirm, unlinked; onboarding-card at both sizes) | PASS | 2026-10-18 | claude-opus |
| a11y: axe 0 serious/critical, both themes | quality gate | `settings — appearance + account & cloud — axe 0` scans signed out, linking, consent, the Unlink dialog and the offline + paid-locked state, dark and light; feature switches are `role=switch` named by their titles | PASS | 2026-10-18 | claude-opus |
| Settings opens ≤ 100 ms (cached account) | perf | Status is a local read (no network call; asserted in AJ-E1202 `signed_out_is_silent` / `status and gates are local`) | PASS | 2026-10-18 | claude-opus |

## Test counts

- vitest `settings-screen.test.tsx` 8/8.
- Playwright: `44 settings` 12, `settings — … axe 0` 2, onboarding card 2,
  and the unchanged flow (a) budget test.
- `cargo test -p juggler-app --test figurehead` 8/8.

## Also fixed while here

The Usage cost matrix (AJ-E609) rendered unstyled above the inspector on
every selection — even with no rows — while the Usage screen was still
a "lands in the M2+ build-out" placeholder. It now lives on the Usage
screen, styled, with an empty state; the placeholder is gone (gallery
`25-costs` re-captured on the Usage screen).
