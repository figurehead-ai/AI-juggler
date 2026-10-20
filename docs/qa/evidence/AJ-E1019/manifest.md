# Evidence manifest — AJ-E1019

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1019 (Cleanup in
plain words; "tool" on screen instead of "Provider", D-25). Status:
**DONE-EVIDENCED**. 2026-10-20, reviewer claude-opus.

## What exists now

- **Cleanup** is titled "Cleanup" with one line: what your sessions left
  running; you choose what to stop; automatic cleanup is off unless you
  turned it on. The "Tier 0 observe · verified manual kills" chip is gone;
  the tiers keep their names only in Settings › Cleanup.
- The header's **⋯** holds Pause automatic cleanup (for every account,
  until resumed) and Cleanup settings… (opens Settings at its Cleanup
  section). While paused, the banner carries Resume automatic cleanup. The
  kill-switch is still in the header (AJ-E508 Req.4), one step away.
- The **machine filter** shows once another Mac is paired or reporting
  (fleet peers or remote reports); This machine stays the default.
- Groups name the tool ("Claude Code · Work Max · web"). The backend's hog
  and idle alerts end "— review it in Cleanup."
- **"Tool" on screen.** History's search line ("project, account, tool,
  model, branch…"), Settings › Account & Cloud, pack secrets ("Your choice
  (all tools)", "Per-tool choice", the restatement "(all tools)"), the
  rail's Filters ("Tool: all", Sort/Group "Tool", display names), the
  session details (the remote Overview's Tool field, the tool inspector's
  title and actions without internal task codes), a switch target ("Another
  tool — full-story replay"), Accounts' empty state and its Add a tool
  button, the backup's "not included" list, and quota's "No verified quota
  source for this tool yet". The code keeps `Provider`.
- `scripts/terminology_lint.sh` now refuses "Provider" in JSX text (single
  and multi-line), visible attributes (aria-label, title, placeholder,
  label, alt, reason, hint) and user-facing strings in components.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 title, no tier words | vitest `cleanup-centre.test.tsx` "is titled Cleanup, with no internal tier names on screen" | PASS |
| Req.2 ⋯ and Resume | vitest: default state has no pause on screen, ⋯ offers it; while paused Resume is on the banner and the menu doesn't repeat it; Cleanup settings… dispatches `aij:open-settings` with `cleanup` and closes the menu; capture `21-tiers/paused-*` | PASS |
| Req.3 machine filter | vitest: none without another Mac; with one, the labelled combobox widens the load; captures `24-fleet/machine-filter-*`, `48-remote-cleanup/*` and the a11y fleet scans seed a paired Mac | PASS |
| Req.4 tool names, alert words | vitest attribution row; `crates/juggler-app/tests/guardrails.rs` suffixes updated with the strings (CI runs the Rust tests) | PASS |
| Req.5 wording + lint | `./scripts/terminology_lint.sh` OK with the new rules; vitest `inspector-shell` (tool kind titled Tool), `inspector-aggregate` (update check disabled, says it never updates a tool by itself), `QuotaRings`; first-run-policy suites expect "(all tools)" | PASS |
| Full suites | vitest 472/472; Playwright 921/921 (on the merge with main); a11y gate 940 states, 0 critical/serious; visual gate 81/81 after the re-render | PASS |

## Not covered here

- The other filter labels ("All managed", "Re-adopted") are unchanged:
  "Re-adopted" matches the recovery summary's word for the same thing.
- The Rust string changes are verified by CI (no local Rust build, to
  save disk).

## Review fixes (2026-10-20, after #63)

- **Focus.** Opening ⋯ focuses its first item. Escape, Pause and the
  banner's Resume hand focus back to ⋯, never to `<body>` (WCAG 2.4.3).
  A click elsewhere closes it without moving focus. Cleanup settings…
  opens Settings, which focuses its Cleanup heading. vitest
  `cleanup-centre.test.tsx` "the ⋯ menu keeps keyboard focus" covers
  open, Escape, Pause, then Resume.
- **AJ-E508 Req.4** ("in Settings AND in the Cleanup Centre header";
  pausing takes effect within one tick; a persistent banner). The header
  switch is one step away behind ⋯, still in the header. The banner text
  is unchanged and carries Resume. Settings never had the switch, so the
  requirement didn't hold before #63 either. Settings › Cleanup now has
  "Pause automatic cleanup" (off by default, shows a pause made
  elsewhere). Proof: vitest `settings-sections.test.tsx` (two Req.4 tests).
- **More focus.** Re-confirm on the "suspended after a crash" banner
  also returns focus to ⋯ (vitest).
- **Settings pause switch.** If its status can't be read, it says so
  (role=alert) with Try again, instead of staying disabled with no
  word (vitest).
- **Lint.** The "Provider" rules match `[Pp]roviders?`. The lowercase
  rule skips code contexts (`session.provider`, `provider < 3`,
  `fn(provider)`). The script now self-tests at every run against a
  fixture of words that must be flagged and code that must not, and it
  fails if either drifts. The lowercase
  leftovers now say "tool":
  - "No accounts for this tool yet."
  - the rail search's name "…model or tool"
  - the session panel's "tool:"
  - "per pair of tools"
  - "All N tool pairs ship in beta"

  Tooltips show tool names, not ids (switch graph, rail rows, the
  Accounts glyph).
- **Walked by hand** (vite preview with the mock bridge, 1280×800 dark
  and light). In Cleanup I opened ⋯ (focus on Pause), pressed Escape
  (focus on ⋯), Enter then Enter (paused, the banner shows, focus on ⋯),
  Resume (the banner goes, focus on ⋯), ⋯ › Cleanup settings… (Settings
  at Cleanup, heading focused), then ticked Settings' Pause switch ("paused").
- **Walked by hand after the #75 review** (vite preview, 1280×800,
  dark). In Cleanup, Re-confirm on the suspended banner removed the
  banner and left focus on ⋯. In Settings › Cleanup, with the status read
  failing (mock seam), it said "Couldn't read whether automatic cleanup is
  paused: …" with Try again, and the switch was disabled. After Try again
  the message went and the switch was enabled.
