# Evidence manifest — AJ-E2110

Task: `docs/tasks/E21-desktop-first.md` AJ-E2110 (Settings panes, docs
and the Privacy page; the approved "desktop first" canvas, D-35 to D-37).
Status: **IN-PROGRESS**. The panes, Desktop apps, the docs and the claims
are done and checked. Three things wait on other tasks: the rail reading
"Show sessions from" (AJ-E2104), the backend tests behind the Last
messages claim (AJ-E2108), and the central visual run. 2026-10-24, branch
`e21-settings` (on `feat/desktop-first` with AJ-E2101..E2103 merged).

## Every setting's home

Nothing is orphaned. Every card that was on the one long page is in a
pane; vitest `settings-sections.test.tsx` › "every setting that moved
still has a home" opens each pane and finds each card.

| Before (section › card) | Now (pane › section) |
|---|---|
| General › Appearance (theme, session list density) | General › Appearance |
| General › Open projects in (editor) | General › Open projects in |
| General › About & updates | General › About & updates |
| General › Report a problem (diagnostics) | Advanced › Diagnostics |
| General › Privacy (usage statistics, crash reports, Search the words of every session) | Privacy › Usage statistics, crash reports and search |
| Account & Cloud | Advanced › Figurehead & Cloud (renamed) |
| Context packs & secrets › Pack secrets | Switching › Context packs & secrets |
| Sessions › Stuck after, Terminal, Keep work going, Open at login | Sessions › Sessions AI Juggler starts |
| Sessions › Go to a session's own tab (added by AJ-E2102) | Desktop apps › Going to a session › Select the right terminal tab |
| Cleanup (default, pause, per profile) | Advanced › Cleanup |
| Accounts & renewal dates | Accounts › Renewal dates (and Open Accounts) |
| Notifications (in the app, macOS kinds, quiet hours, digest) | Notifications › Alerts and the digest |
| Fleet permissions | Your Macs › What your other Macs may do here (shown once a Mac is paired, or when a link opens it) |
| Data › Storage & retention | Advanced › Data: storage & retention |
| Data › Backup & restore | Advanced › Data: backup & restore |
| Data › Before you uninstall | Advanced › Before you uninstall |
| The go-to key (only in ⌘K's footer, default ⌥Space) | Desktop apps › Open AI Juggler from anywhere (no default; ⌘K's footer still changes it) |
| (new) | Desktop apps › Show sessions from (one switch per app) |
| (new) | Desktop apps › Bring the exact window to the front (Set up…) |
| (new) | Desktop apps › Last messages (`show_last_messages`, on by default) |
| (new) | Privacy › What AI Juggler reads |

Agent access is in the pane list with `shown: () => false` (pools are
not on this line); 0.2 turns it on.

## Links into Settings

| Link | Lands on |
|---|---|
| ⌘, and the header's Settings | the pane viewed last (this window's storage), else General |
| `aij:open-settings` `updates` (the update prompt's Review) | General › About & updates |
| `cleanup` (Cleanup Centre) | Advanced › Cleanup |
| `packs` (Accounts' pack secrets line) | Switching › Context packs & secrets |
| `sessions` (This Mac's keep-awake line) | Sessions |
| `focusCloud` (Fleet's Find my Macs offer) | Advanced › Figurehead & Cloud |
| `account`, `renewal`, `notifications`, `fleet`, `data`, `general`, `privacy` | their new pane and section |
| `last-messages` (Privacy's Last messages…) | Desktop apps › Last messages |

A link while Settings is already open opens its section again
(`Settings.tsx` listens too).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 panes; every setting keeps a home | vitest `screens/settings-sections.test.tsx`: the panes in order, each with a tile; Your Macs once paired; never Agent access; every older link lands on its pane and section with the focus on its header; Cloud lands on Figurehead & Cloud; ⌘, comes back to the last pane; every moved card found. Lazy: each pane is its own chunk (`lazyNamed`) | PASS |
| Req.2 sections with headers and one-line footers | vitest: each of Advanced's 6 sections has a header and ends with its footer; every pane is built from `PaneSection` (`screens/settings/parts.tsx`) | PASS |
| Desktop apps | vitest `screens/settings/desktop-apps.test.tsx`: each app with what was found (`1 open · Work Max`, ChatGPT's Dock name); a switch kept only when off; an unlisted app found open joins the list; the views read the same choice (`useHiddenApps`); Set up… asks macOS here and says On; no key until chosen, ⌥Space refused, ⌃⌥Space saved and shown, Turn off; the physical key read (⌥J is J, not ∆); Last messages on until turned off. `components/terminal-tabs.test.tsx`: the tab switch, off until turned on | PASS |
| No go-to key by default | `lib.rs` registers only a stored key; `go_to_hotkey_set("")` unregisters and clears it. vitest `goto-palette.test.tsx`: ⌥Space refused, ⌃⌥Space rebinds. (Rust: built by the trust test; its own tests not run here, by the brief) | PASS |
| Req.3 the Privacy page names the new reads | `trust-claims.json` + `docs/guide/trust.md`: `open-sessions` (from AJ-E2101), `parent-chain` reworded in plain words and checked by the no-environment guardrail too, `no-prompt-click` (AJ-E2102), new `last-messages`. `cargo test -p juggler-app --test trust`: 13 passed (`every_trust_claim_names_tests_that_exist` included). vitest `privacy-page.test.tsx` draws every claim | PASS |
| Last messages claim, fully proven | Its line cites the Settings switch test only. "Read on open, kept nowhere, secrets taken out" is AJ-E2108's backend; its tests join this claim when they land | UNPROVEN |
| Req.4 profile and account per D-35 in guide, README, charter | ADR-0008 amendment A-2; SPEC §9.1 and §9.3 amendment notes (D-36); the guide and README rewritten; `docs_links_check.py`, `terminology_lint.sh` (taught D-35's words narrowly), `plain_words_lint.py` OK | PASS |
| a11y | The sidebar is a `nav` named Settings with a list of buttons, the shown pane `aria-current="page"`; every switch is `role="switch"` named by its row title and described by its line (vitest checks every switch on Desktop apps has a name); tiles and icons `aria-hidden`; card titles inside a section stay as `sr-only` headings for links and screen readers. Text uses the AJ-E804 pairs (ink and muted on surface, ≥ 4.5:1 in both themes); tile icons are decorative, the label beside them carries the name | PASS |
| Visual: each pane, both themes | `capture.spec.ts` 65 gallery now shoots every pane; specs that scrolled the long page open the pane first. Not run here (Docker gate runs centrally) | PENDING (central run) |
| "Show sessions from" changes the list | The choice is stored and exposed (`lib/desktop-apps.ts` `useHiddenApps`, `DESKTOP_APPS_EVENT`); the rail and Home are AJ-E2104/E2105's files and don't read it yet | UNPROVEN |

Checks on the branch: `npx vitest run` 117 files, 892 tests pass; `npm
run typecheck`; `size_budget.py` (Settings.tsx left the allowlist, now
106 lines); `just structure`; `plain_words_lint.py`, `terminology_lint.sh`,
`tool_literal_lint.py`, `docs_links_check.py` OK.

## Walked by hand

The vite dev server with the replica fixture (`tests/visual/fixtures/
replica.json`, three live sessions given places for the walk), 1280×860,
dark then light:

- ⌘, opened General: Appearance, Open projects in, About & updates, each
  with its footer;
- Desktop apps: Claude app "2 open · claude, claude-client-acme",
  Terminal "1 open · claude-client-globex", Codex app "Nothing open now.
  In the Dock it's ChatGPT.", Cursor; turning Claude app off kept
  `show_sessions_from.claude-app = 0`;
- Choose… then ⌥Space: "ChatGPT uses ⌥Space. Pick another key."; then
  ⌃⌥J: shown as ⌃⌥J with Change… and Turn off;
- Privacy in light: What AI Juggler reads, Last messages… opened Desktop
  apps at Last messages (focus on its header), also when clicked a
  second time (a bug found and fixed on the walk);
- Advanced: Cleanup, Data, Diagnostics, Figurehead & Cloud, Before you
  uninstall.

## Baselines the central run will change

- gallery `44-settings/*` (Settings opens on Advanced for Cloud; new
  layout), `56-history/retention-*`, `57-backup/*`, `58-updates/*`,
  `59-vault/*`, `60-cloud-backup/*`, `62-privacy/*`, `64-crash/*`,
  `65-settings-sections/*` (now one per pane: general, desktop-apps,
  accounts, sessions, switching, notifications, privacy, macs, advanced;
  the old `packs`, `cleanup`, `renewal`, `fleet` shots go), `41-policy/
  settings-pack-secrets-*`, `keep-going/settings-keep-going-*`,
  `diagnostics`, `notifications`, `outside-search` and `privacy-gallery`
  Settings shots;
- regression shots that include Settings (`regression.spec.ts` Sessions
  › Terminal);
- README shots are no longer referenced (`docs/assets/readme/home-*`,
  `project-*`, `accounts-*`); `readme-shots.spec.ts` still writes them.

## Not done

- The rail and Home don't read "Show sessions from" yet (AJ-E2104,
  AJ-E2105 own them; `useHiddenApps` is ready, and a hidden app must
  leave a count, as the law says).
- The Last messages claim needs AJ-E2108's tests (tail read on open,
  redaction, nothing stored). The `show_last_messages` setting is added
  here, with the name and default AJ-E2108 uses.
- Accounts' Profiles screen still says "Settings › Context packs &
  secrets" on its link (AJ-E2109's file); the link itself lands right.
- The desktop apps' own icons: the rows show the kind's icon (window,
  terminal, editor) until the place headers bring app icons.
