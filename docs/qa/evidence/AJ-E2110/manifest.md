# Evidence manifest — AJ-E2110

Task: `docs/tasks/E21-desktop-first.md` AJ-E2110 (Settings panes, docs
and the Privacy page; the approved "desktop first" canvas, D-35 to D-37).
Built in 0.1.3 (ac0e41dc0); re-audited against the code on
`release/0.4.1` (HEAD 051e71e4b), 2026-11-07.

Since the 2026-10-24 manifest: the Last messages claim is fully proven
(AJ-E2108's backend tests and the 0.3.5 feed's now back it in
`trust-claims.json`), and "Show sessions from" is read by the rail, Home
and the pages (`lib/shown-apps.ts`; `place-board.test.tsx`: "3 in Terminal
hidden · Settings"). Panes added later: The Floor (AJ-E3511), Agent access
(AJ-E1702, now shown), Keyboard (AJ-E2905).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 panes: General (About & updates), Desktop apps, Accounts, Sessions, Switching, Notifications, Privacy, Your Macs (when paired), Advanced (Cleanup, Data, Diagnostics, Figurehead & Cloud); every existing setting keeps a home | `cd ui && pnpm -s exec vitest run src/screens/settings-sections.test.tsx`: "lists the panes in order, each with its tile, and marks the one shown", "shows Your Macs once a Mac is paired", "every older link lands on its pane and section", "opened for Cloud, lands on Advanced › Figurehead & Cloud", "every setting that moved still has a home"; `screens/settings/panes.ts` (`PANES`, `ANCHORS`); Playwright `capture.spec.ts` "65 settings sections — <pane> — <theme>"; gallery `_gallery/65-settings-sections/advanced-1280x800-light.png`, `privacy-1280x800-dark.png` (looked at) | every named pane in order; Your Macs only when paired; every older card found in its pane | pass: the named panes in order, with The Floor, Agent access and Keyboard between them; Advanced holds Cleanup, Data (storage, backup), Local cache, Diagnostics, Debug mode, Figurehead & Cloud, Before you uninstall; 15 homes found. The 65 captures (6a06e7483, 2026-10-31) predate The Floor and Keyboard panes; the spec shoots every `PANE_IDS` entry, so the gallery gate adds them | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R2 sections have headers and one-line footers saying what a setting does | vitest `screens/settings-sections.test.tsx` "<pane>: each section has a header and a one-line footer saying what it does", one test per pane (Keyboard's list: its first section's footer says it once); The Floor's two sections got theirs | every section of every pane | 71/71 pass (2026-11-07) | PASS | bd74c53c8 | 2026-11-07 | Claude (self) |
| R3 the Privacy page and its tests name the new reads: the apps' session files, the parent chain of processes, transcript tails | `ui/src/lib/trust-claims.json` claims `open-sessions`, `parent-chain`, `last-messages` (and `docs/guide/trust.md`); `cd ui && pnpm -s exec vitest run src/screens/privacy-page.test.tsx`: "says every claim, grouped as the list groups them", "every claim names at least one test", "Settings › Privacy holds every line, in one place (0.1.5)"; cargo (runs in CI) `juggler-app` `tests/trust.rs::every_trust_claim_names_tests_that_exist`, `::a_claim_needs_a_running_test`; the claims' own tests, e.g. `src/features/conversations/places.rs::a_desktop_session_is_in_its_claude_instance_by_its_argument`, `tests/trust.rs::no_process_environment_is_ever_read`, `src/features/conversations/last_messages.rs::nothing_is_kept_written_or_logged`; gallery `_gallery/65-settings-sections/privacy-1280x800-dark.png` | each read named, each claim backed by tests that exist | pass: Settings › Privacy › What AI Juggler reads says "Each tool's own session files, and the names of running programs and the apps that started them" and "The end of a session's transcript"; each claim lists its tests. Wording to align (not a gap): `PrivacyPane.tsx` 32 says "kept nowhere", the `last-messages` claim says "kept in memory only while it's open (and a minute after)" | PASS | 7a84fdf4f | 2026-11-07 | Claude (self) |
| R4 the guide, README and terminology charter use profile and account as D-35 defines them | `docs/adr/0008-terminology-charter.md` Amendment A-2 (D-35); `docs/guide/accounts.md` ("An **account** is a subscription you pay for", an app "in one of its profiles"); `docs/guide/README.md`; `README.md` 31, 94; CI step "Terminology and plain words (AJ-E108, AJ-E1535 gates)" (`scripts/terminology_lint.sh`, `scripts/plain_words_lint.py`) and `scripts/docs_links_check.py` (run in CI) | account = subscription, profile = the setup an app runs on, never one for the other | read: the charter's A-2 is D-35's definition; the guide and README say account = subscription and profile = an app's setup, with D-62's "App" (app · profile) on top (SPEC §3 amended by D-62). No vitest or cargo test covers docs; the evidence is the text and the CI lints. The charter has no D-62 amendment (outside R4) | PASS | 2d4a19771 | 2026-11-07 | Claude (self) |
| Gate | `cd ui && pnpm -s exec vitest run src/screens/settings-sections.test.tsx src/screens/settings/desktop-apps.test.tsx src/screens/privacy-page.test.tsx src/components/goto-palette.test.tsx src/components/terminal-tabs.test.tsx src/components/place-board.test.tsx` | all pass | 6 files, 62 tests pass. Cargo, the lints and Playwright cited as written: they run in CI / the gallery gate | PASS | 051e71e4b | 2026-11-07 | Claude (self) |

## R2's gap, exactly

- `ui/src/screens/settings/FloorPane.tsx` lines 50 and 68: give "Opening"
  and "The office" a `footer` (one line on what the section does, as every
  other pane's sections have).
- `ui/src/screens/settings-sections.test.tsx` "each section has a header and
  a one-line footer": loop over every pane id in `PANES` (not only
  `advanced`), expecting each `section[data-settings-section]` to have an
  `h3` and end with its footer `P`; or make `PaneSection`'s `footer`
  required so the type checker holds it.

## Every setting's home

Nothing is orphaned; vitest "every setting that moved still has a home"
opens each pane and finds each card.

| Before (section › card) | Now (pane › section) |
|---|---|
| General › Appearance, Open projects in, About & updates | General › the same |
| General › Report a problem (diagnostics) | Advanced › Diagnostics |
| General › Privacy (usage statistics, crash reports, search) | Privacy › Usage statistics, crash reports and search |
| Account & Cloud | Advanced › Figurehead & Cloud |
| Context packs & secrets › Pack secrets | Switching › Context packs & secrets |
| Sessions › Stuck after, Terminal, Keep work going, Open at login | Sessions › Sessions AI Juggler starts |
| Sessions › Go to a session's own tab | Desktop apps › Going to a session |
| Cleanup | Advanced › Cleanup |
| Accounts & renewal dates | Accounts › Renewal dates |
| Notifications | Notifications › Alerts and the digest |
| Fleet permissions | Your Macs › What your other Macs may do here |
| Data › Storage & retention, Backup & restore, Before you uninstall | Advanced › Data: storage & retention, Data: backup & restore, Before you uninstall |
| The go-to key (⌘K's footer) | Desktop apps › Open AI Juggler from anywhere |
| (new) | Desktop apps › Show sessions from, Bring the exact window to the front, Last messages (`show_last_messages`, on) |
| (new) | Privacy › What AI Juggler reads |

## Links into Settings

`screens/settings/panes.ts` `ANCHORS`: `updates` → General › About &
updates; `cleanup`, `data`, `diagnostics`, `uninstall`, `local-cache`,
`account`/`cloud` → Advanced; `packs` → Switching; `sessions` → Sessions;
`renewal`, `tools` → Accounts; `fleet` → Your Macs; `privacy`, `search` →
Privacy; `last-messages`, `go-to`, `keep-alive` → Desktop apps; ⌘, → the
pane viewed last. Checked by "every older link lands on its pane and
section" and "⌘, comes back to the pane viewed last".

## Binary artifacts

None added; cited: `_gallery/65-settings-sections/*-1280x800-{dark,light}.png`
(captured 2026-10-31, 6a06e7483; no The Floor or Keyboard shot yet).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
