# Evidence manifest — AJ-0.4.2-presence

The UI half of 0.4.2's presence work (ADR-0032, D-100): Desktop and CLI
by icon and word, "Running, not added" in the rail, a CLI App's last
folder, and the top bar's "N accounts out" landing on Accounts filtered
to the out ones. Captures are from the Vite preview's fixtures (generic
names), 1280×800 at 2×, for the design critique.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| UI unit tests | `pnpm exec vitest run` (ui/) | all pass | 396 files, 2693 tests passed | PASS | 05a1b0404 | 2026-11-11 | Opus (agent) |
| Out chip lands filtered | `AIJ_PW_PORT=4687 pnpm exec playwright test tests/visual/accounts-out-filter.spec.ts` | passes | 1 passed | PASS | 7b2d61a79 | 2026-11-11 | Opus (agent) |

## Captures

- `rail-apps-{dark,light}.png`: the rail's App lens: Open now headers with the kind mark beside the icon, "Running, not added" with Add, Inactive apps rows saying "Desktop · …" / "CLI · …".
- `home-your-apps-{dark,light}.png`: Home's Your apps, the kind mark beside each App's name.
- `accounts-apps-{dark,light}.png`: Accounts › Apps, each row's kind chip ("CLI", "Desktop"), a CLI App's "Last used 22 h ago in ~/payments-api".
- `picker-goto-apps-{dark,light}.png`: the Go to palette's Apps: mark beside the icon, "Desktop · open now" / "CLI · not running".
- `accounts-out-filter-{dark,light}.png`: after the top bar's out count: Accounts tab, "Out · 1" chip with ×, the out card's "Out · resets 9:14 PM" line.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout (preview fixtures only)
- [x] Transcript excerpts scrubbed of personal content (none)

## The rail header keeps the App's name (0.4.2 review, D-100)

The profile is what tells one App from another. In the 264 px rail the
header gives way in this order: its secondary words (its app's word, a
folded App's summary) drop out whole, wrapping out of a one-line box;
the Out/Low mark is its icon alone; and only a name that alone doesn't
fit is cut, at its end, its full name on hover. (An earlier attempt with
flex-shrink weights and a middle cut still cut the names on the real Mac:
"book…h-anela", while "Cl… no…" kept room.) Captured in the Vite
preview at 1280×800, rail 264 px, dark; the real profile names and the
two marks were put into the fixture header's DOM (the preview has no
fixture with both).

- `rail-header-264-long-nochips-dark.png`: an earlier capture, "acme-personal-max" with no marks: whole.
- `rail-header-264-olivia-whole-dark.png`: "acme-olivia" with ⚠ 1 and the Out icon: whole (92 of 92 px); "acme-anela" below it, whole; the app words wrapped away.
