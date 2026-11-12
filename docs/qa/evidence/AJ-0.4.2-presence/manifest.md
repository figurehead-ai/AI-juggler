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

The profile is what tells one App from another, so in the 264 px rail the
app word gives way whole first, the compact marks (the amber count,
"Out") keep their few letters, and the name is cut in the middle
(`MiddleName`: its first four letters and its end), never at the end:
"acme-east" with both marks reads "acme-east", where an end cut
read "acme-l…" and hid the part that differs. Whole whenever it
fits; the full name is its hover title. Captured in the Vite preview at
1280×800, rail 264 px, dark; the long names and the two marks were put
into the fixture header's DOM to reach the worst case (the preview has no
fixture with both).

- `rail-header-264-long-nochips-dark.png`: "acme-acme-max", no marks: whole.
- `rail-header-264-long-chips-dark.png`: the same with ⚠ 1 and Out: "book…al-max".
- `rail-header-264-east-chips-dark.png`: "acme-east" with ⚠ 1 and Out: "acme-east".
- `rail-header-264-north-chips-dark.png`: the longest real profile name, "acme-north", with ⚠ 1 and Out in the 264 px rail: "acme-north"; "acme-east" below it, whole. Open now lists only what's open (no "Last N inactive" under an open App, D-100).
