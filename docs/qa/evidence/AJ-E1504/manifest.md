# Evidence manifest — AJ-E1504

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1504 (keys: a cheat sheet,
inspector collapse, go-to keys). Status: **DONE-EVIDENCED**. 2026-10-21,
reviewer claude-opus.

## What exists now

- **One registry** (`ui/src/lib/shortcuts.ts`). Every shortcut has its
  area, keys, words and key test, plus who handles it. App's handler map
  is typed `satisfies Record<AppShortcutId, …>`, so an app-owned shortcut
  without a handler doesn't compile. Home's J/K/A and the inspector's
  tab and back keys match through it. App and Home no longer test a key
  by hand (a test reads their source). Home's "Keyboard:" line is
  generated from the same entries.
- **The ? sheet** (`ShortcutSheet.tsx`, on the shared `Modal`). `?`
  outside a text field opens every shortcut in five areas: Anywhere,
  Screens, Session, Home, From any app. It is rendered from `SHORTCUTS`,
  so the sheet is the registry.
- **⌘1–4** open Home, Accounts, Usage and Fleet. Fleet keeps ⌘⇧F too.
- **⌘⇧I folds the inspector.** Since ADR-0007 iteration 5 the inspector
  *is* the work panel, so folding keeps its header line (name, state
  chip, Switch account, Stop, and a fold toggle with `aria-expanded`) and
  the open tab. A session's terminal gets the room. The fold is
  remembered per window (`aij.inspector.folded.<window label>`).
- **Inspector tabs moved to ⌥⌘1–6.** ⌘1–6 used to switch them (AJ-E406
  Req.6, amended in the E4 epic), and ⌘1–4 now open the screens. The
  digit comes from `event.code`, since ⌥ changes what `key` reads.
- **The typing rule, made general** (Req.3). While typing in a field or
  the terminal, a plain key or Ctrl-combo never fires a shortcut, so
  Ctrl+K, Ctrl+B, Ctrl+N and Ctrl+F reach the shell (before, all four
  were taken by the app). ⌘ shortcuts still fire, except ⌘Z, which is the
  field's undo. Before this, only Ctrl+D was guarded.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 `?` opens a sheet listing every shortcut by area, generated from the registry | vitest `app-keys`: the sheet's rows are exactly `SHORTCUTS` in area order, each with its words and keys; ids unique; only the natively handled keys (Esc, Enter, Ctrl+Tab, the system-wide hotkey) have no key test; App and Home read no key by hand; `?` opens it in the app, not from the search field; Esc closes it | PASS |
| Req.2 ⌘⇧I folds and restores, remembered; ⌘1–4 open the screens | vitest `app-keys`: ⌘2/3/4/1 give Accounts, Usage, Fleet, Home; ⌘⇧I folds (tab strip hidden, open tab kept, toggle `aria-expanded=false`), stored, still folded after a remount, unfolded by the toggle; ⌥⌘2 picks Usage while ⌘2 leaves for Accounts; `inspector-shell` (⌥⌘3 and ⌥⌘6 switch tabs, ⌘3 alone doesn't) | PASS |
| Req.3 no shortcut while typing (the typing-guard tests) | vitest `app-keys`: for every shortcut with a key test, Ctrl and plain keys never fire in an input or the terminal, ⌘ ones do unless the field owns them; Ctrl+K and Ctrl+D in the terminal are the shell's; ⌘K there still searches; ⌘Z in a field is the field's; Ctrl+K in the app's terminal opens nothing | PASS |
| Visual: the cheat sheet; the collapsed inspector | captures `76-keys/sheet-*`, `76-keys/folded-*` (1280×800 and 1024×680, both themes); regression `shortcut-sheet-*`, `inspector-folded-*` (8 cells each) | PASS |
| a11y | a11y "keys — the ? sheet and the folded inspector": a named modal dialog, focus inside and held on Tab, Esc closes; the fold toggle's state; axe 0 critical/serious in both themes | PASS |

## Gates

All 2026-10-21, on release/0.1. Two batches: b1 is the AJ-E1504/E1505
head, b3 the AJ-E1525/E2011/E1524 head that carries it.

| Gate | Result | Verdict |
|---|---|---|
| `just verify` | exit 0 on the b3 head. On b1 it failed in `tests/ports.rs`: an earlier killed run had leaked its http.server fixtures onto :5173. The test now reaps its listeners however it ends. `processes` and `fleet_switch` time out under machine load (load average over 30) and pass alone, 3 of 3. | PASS |
| `cargo deny check` | advisories, bans, licenses, sources ok | PASS |
| vitest | 592/592 | PASS |
| Playwright, full suite | 932 passed (b1); 940 passed (b3) | PASS |
| `scripts/a11y_gate.sh` | 698 tests; 872 gallery states with 0 critical, serious, moderate or minor (b1 and b3) | PASS |
| `scripts/visual_gate.sh --update` | 185 passed (b1), 193 passed (b3). Every changed baseline is explained in the release PR's "## Visual baselines". | PASS |
| `scripts/evidence_index.sh` | regenerated; evidence-check OK | PASS |

## Walked by hand

The vite dev server with the mock bridge, at 1280×800. The browser pane
swallows ⌘-digits and sends an empty key for Shift+/, so the walk
dispatched real `keydown` events on the page:
- `?` opened the sheet with 26 rows and focus inside; Esc closed it.
- ⌘2, ⌘3, ⌘4 and ⌘1 gave Accounts, Usage, Fleet and Home.
- On a running session, ⌘⇧I folded the inspector (tab strip hidden,
  stored "1").
- Ctrl+K on the terminal's textarea opened no palette.
- ⌥⌘2 picked the Usage tab.

## Not covered here

- The real Mac app: ⌘1–4 and ⌘⇧I aren't app-menu items, so the webview
  gets them (as it gets ⌘K today). Not pressed in the Dev window: agent
  sessions have no Accessibility grant.
- The wireframes' edge tab (an inspector beside a work panel) predates
  ADR-0007 iteration 5. A pop-out window (AJ-E1506) gets its own fold
  key by its window label.
