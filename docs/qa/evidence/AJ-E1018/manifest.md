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

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 legend | `home-gallery.spec.ts` "legend": in view, topmost at its centre, says "Waiting for you", axe 0, Escape closes and refocuses. It **fails on the original code** (checked by restoring the clipped panel). Walked by hand in the browser: open, outside click, Escape, no row scroll. | PASS |
| Req.2 where setups are open | Rust `running_setups::tests` (desktop windows → profiles and the default folder; the desktop app's own `claude` child stays desktop; a terminal `claude`; a folder in both). The ignored live probe on the maintainer's Mac lists the six setups as desktop. vitest `first-run.test.tsx` (groups, "Default", "Open in the Claude app" …). Walked by hand with a scan shaped like that Mac. | PASS |
| Req.3 sign-ins on Home | vitest `home.test.tsx` (the item; Sign in opens "Sign in · Work Max"; the row says where to finish). Captures "ready home" (now Home's item) and "login card" (via Accounts). Walked by hand: Sign in opens the embedded sign-in terminal. | PASS |
| Suites | vitest 468/468; `cargo test -p juggler-app` (no failures); clippy `-D warnings` in both configurations; Playwright 911/911; accessibility gate 940 states, 0 issues; visual gate 81/81 re-rendered in the pinned container | PASS |

## Not covered here

- A terminal `claude` running on a profile is detected only while it runs. Past terminal use shows as recency, as before.
- The classifier knows the Claude desktop app's bundle path (`/Claude.app/Contents/MacOS/Claude`) and the `claude` CLI (native binary or the npm package). Other launchers aren't recognised.
