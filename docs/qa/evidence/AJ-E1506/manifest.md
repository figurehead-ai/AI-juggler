# Evidence manifest — AJ-E1506

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1506, Terminal tools (P0).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). One live check needs a
person in the running app: the terminal's own window (below).

Gallery: `docs/qa/evidence/_gallery/82-terminal-tools/` (the toolbar with Find, maximized,
the main window while the terminal is in its own window, a link's hover hint; both
themes) and `search-timings.json`.

## What exists now

1. **A strip above every terminal** (`TerminalToolbar.tsx`): whether it's Live,
   Reconnecting (until its retained output is back and it streams) or Ended (its
   process gone: the exit event, or a finished, stopped, failed, handed-over or
   limit-stopped session); a word on what just happened; and its tools, each with its
   key in the tooltip.
2. **Find** (`TerminalSearch.tsx`, the xterm search addon): ⌘F (Ctrl+Shift+F on
   Windows and Linux) or Find; Enter for the next match, Shift+Enter or ↑ for the
   previous, "3 of 12" as it goes, matches marked in the text and the scrollbar; Esc
   closes it and gives the keys back to the terminal.
3. **Links**: web addresses (the web-links addon) and file paths (`terminalLinks.ts`:
   what agents and compilers print, `src/a.rs:42:7`, `./x`, `~/x`, absolute) are
   underlined on hover, which says "⌘-click opens …" (Ctrl-click elsewhere); a plain
   click still selects. The backend decides what opens (`features/terminal/links.rs`):
   `http`/`https` in the browser; a path that's really there, relative to the
   session's folder, a file in the editor chosen for projects (else shown in Finder),
   a folder in Finder. Nothing the terminal printed is ever run or handed to the Mac's
   default app for its type; other schemes and control characters are refused.
4. **Copy**: the selection, or else what's on screen.
5. **Text size**: A− and A+, ⌘− ⌘+ and ⌘0 (as set) while typing in it; saved to
   Settings › Sessions, so every terminal follows.
6. **Jump to the bottom** when scrolled up.
7. **Maximize** (⌘↩): the terminal fills the window; ⌘↩ or Restore puts it back, and
   Esc does too from anywhere but the terminal itself (where Esc is the agent's).
8. **Its own window** ("Own window"): the same terminal in a real window
   (`features/terminal/window.rs`, label `terminal-<id>`, its own capability that grants
   only events) for another display. The main window then says "This terminal is in its
   own window" with Show its window and Bring it back here. The session keeps running
   and supervised either way.
9. The terminal's keys are in the shortcut registry (the "?" sheet), matched by the
   same rule as every other key while typing.

Two choices that differ from wireframe 05:
- **One view drives the terminal's size.** The main window shows where the terminal
  went, not a second live copy: two views of one PTY at two widths would wrap its
  output wrongly in one of them.
- **Closing the main window still asks to quit** (AJ-E1503), and its own windows close
  with the app; the wireframe's "stays live when the main window closes" predates that
  decision.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 find, next and previous; ⌘F | Playwright `terminal-tools` (⌘F or Ctrl+Shift+F opens it, "1 of 1", "No matches", Esc closes and refocuses the terminal) | PASS |
| Search over 10,000 lines < 100 ms | Playwright `terminal-tools`: 10,000 lines written, then one match 15.8 ms, no match 0.7 ms, every line marked 7.9 ms ([search-timings.json](../_gallery/82-terminal-tools/search-timings.json)) | PASS |
| Req.1 links and file paths (a path opens in the editor) | vitest `terminal-tools` (what is and isn't a path; positions); Playwright (hover says "⌘-click opens …"; a plain click opens nothing; ⌘-click hands the path and the web address over); Rust `links.rs` (http/https only; relative to the session's folder; line and column dropped; nothing there refused; a login terminal can't guess a relative path) | PASS |
| Req.1 copy, font size ±, jump to the bottom | Playwright (Copy takes what's on screen; A+ and ⌘−/⌘0 set 13, 12, 12.5; scrolled up, the button shows and takes it back down) | PASS |
| Req.2 Live / Ended / Reconnecting | the strip's word from the attach, the exit event and the session's status; Playwright (Live in the preview) | PASS |
| Req.3 maximize, and Esc outside the terminal restores | Playwright (fills the window exactly; Esc in the terminal is the agent's; Esc on the toolbar restores) | PASS |
| Req.3 its own window, the session still supervised in the main window | Playwright (the main window's placeholder, Show its window, Bring it back here); Rust `window.rs` (only plain ids name a window); the capability validated at build time | PASS |
| Req.3 the real second window, same PTY and scrollback | needs a person in the running app (the dev bridge refuses writes) | NEEDS A HAND WALK |
| No frame drops while output streams | not measured beyond the existing 16 ms batching (M1-4) | NOT MEASURED |
| The keys on the "?" sheet, one rule | vitest `app-keys` (every terminal key has a sample: ⌘ or Ctrl+Shift, never plain Ctrl) | PASS |
| a11y | axe 0 critical/serious on the toolbar with Find, maximized, and the own-window placeholder, both themes | PASS |
