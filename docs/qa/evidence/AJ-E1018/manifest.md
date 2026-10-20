# Evidence manifest — AJ-E1018

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1018 (0.0.6 dogfood
fixes). Status: **DONE-EVIDENCED**. 2026-10-20, reviewer claude-opus.

## What the maintainer hit, and what changed

1. **"?" did nothing.**
   - Cause: the legend opened inside the rail footer's `overflow: hidden` row, so it was clipped out of sight. The suites only checked that it opened.
   - Now: the panel is fixed-positioned above the button, portalled to the page, kept inside the window, and closes on Escape or a click elsewhere, with focus back on "?".
   - Its contents were pre-redesign (`waiting_user`, `quota_blocked`, "MEAS/COMP/EST", sparkline notes). It now explains the rail's own words (one shared `STATE_WORDS` list, so the two can't drift) and the notes a row adds.
   - The "?" is a 24 px target.
2. **"Found 5 desktop and one CLI, but I'm not running the CLI."**
   - Cause, confirmed read-only on the maintainer's Mac: the sixth Claude desktop window runs on the default `~/.claude`. First run grouped by folder location and called that folder "Terminal".
   - Now: the scan reads, from the running Claude processes, which setup each uses and where (`crates/juggler-app/src/running_setups.rs`; only `CLAUDE_CONFIG_DIR` is read from a process's environment).
   - First run groups by kind of setup ("Your Claude profiles", "The default setup", "Other Claude setups"), names the default "Default", and says "Open in the Claude app" / "Running in a terminal".
   - "Where we looked" probe labels are plain words.
   - Live, read-only probe on that Mac: all six setups (five profiles plus `~/.claude`) open in the desktop app.
3. **The old "Get your accounts ready" screen** replaced Home whenever an account needed its sign-in (found while walking the app by hand).
   - Now it's a "Needs you" item on Home: "Personal needs its sign-in · Sign in". Sign in opens Claude's own sign-in in the login terminal, and the row polls every 2 s and clears when signed in.
   - The screen is removed. The sign-in-card captures go through Accounts › Sign in.

## 0.0.7 dogfood follow-up (same task, 2026-10-20)

4. **Bringing in six setups failed at `skills/ego-browser`.**
   - Cause, confirmed read-only on the maintainer's Mac: `~/.claude/skills/ego-browser` is a symlink to a folder elsewhere. The import copied only regular files (and links to regular files), so it stopped and removed the partial copy.
   - Now: a linked folder is recreated in the copy as a link, never followed or copied through. A link into the setup points at the same place in the copy; a link elsewhere points at its real target (absolute, so a relative "Stow" link still resolves from the copy). A linked file is copied by content, as before, so the copy never shares a file with the original (review of #73). A broken link, or one the system refuses, is left out with a note. The result lists each link it kept.
5. **No hand cursor on anything clickable.**
   - Cause: the shared `Button` and several rows set `cursor-default`.
   - Now: `cursor-pointer` at the source, plus a base rule for every native control (buttons, links, summaries, selects, checkbox and radio rows). Disabled controls keep the arrow.
6. **No way to close the bring-in screen at first run.**
   - Close and Escape worked only in Accounts › Add account.
   - Now: an outlined **Not now** (first run) or **Close** (add mode) is always there, and Esc leaves in both modes. An Esc that a popover inside already took leaves the screen open.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 legend | `home-gallery.spec.ts` "legend": in view, topmost at its centre, says "Waiting for you", axe 0, Escape closes and refocuses. It **fails on the original code** (checked by restoring the clipped panel). Walked by hand in the browser: open, outside click, Escape, no row scroll. | PASS |
| Req.2 where setups are open | Rust `running_setups::tests` (desktop windows → profiles and the default folder; the desktop app's own `claude` child stays desktop; a terminal `claude`; a folder in both). The ignored live probe on the maintainer's Mac lists the six setups as desktop. vitest `first-run.test.tsx` (groups, "Default", "Open in the Claude app" …). Walked by hand with a scan shaped like that Mac. | PASS |
| Req.3 sign-ins on Home | vitest `home.test.tsx` (the item; Sign in opens "Sign in · Work Max"; the row says where to finish). Captures "ready home" (now Home's item) and "login card" (via Accounts). Walked by hand: Sign in opens the embedded sign-in terminal. | PASS |
| Req.4 links kept | Rust `claude_environment :: linked_folders_stay_links_and_linked_files_are_copied`: folder links absolute outside, relative outside (Stow), absolute with `..`, relative and absolute inside, dangling; a linked file copied, and editing the copy leaves the original. | PASS |
| Req.5 hand cursor | `cursor.spec.ts`: every enabled clickable element on Home, Accounts, History, Settings, Triage, Overview, Usage, Fleet, Cleanup, the More menu and the open legend shows `pointer`, and no disabled one does. It **fails on the old styles** (115 elements). Checked by hand in the browser: rail rows, filters, setup rows, checkboxes. | PASS |
| Req.6 a way out | vitest `first-run.test.tsx` (Not now and Esc at first run; Esc in add mode; a handled Esc keeps it open). Walked by hand: Accounts › Add account → Esc returns to Accounts; Close returns to Accounts. | PASS |
| Suites | vitest 468/468; `cargo test -p juggler-app` (no failures); clippy `-D warnings` in both configurations; Playwright 911/911; accessibility gate 940 states, 0 issues; visual gate 81/81 re-rendered in the pinned container | PASS |

## Not covered here

- A terminal `claude` running on a profile is detected only while it runs. Past terminal use shows as recency, as before.
- The classifier knows the Claude desktop app's bundle path (`/Claude.app/Contents/MacOS/Claude`) and the `claude` CLI (native binary or the npm package). Other launchers aren't recognised.
