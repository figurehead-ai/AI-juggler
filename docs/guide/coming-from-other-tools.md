# Coming from another tool

AI Juggler reads your coding tools' own files, so most of what you did in
another manager is already there when you open it. This page says what
comes across from each, and what to do about the rest.

## Claude Squad

Claude Squad runs each session in its own git worktree, under
`~/.claude-squad/worktrees/`.

- **Your sessions:** they're ordinary Claude Code and Codex conversations,
  so AI Juggler lists them like any other session, and under their projects
  when you choose **View** › **Group by** › **Project**. Each row shows its
  branch, which carries the session's name.
- **Their worktrees:** each one is grouped with its repository. Claude
  Squad removes a worktree when you pause or kill its session. AI Juggler
  reads which repository each worktree belongs to from Claude Squad's own
  list while the session is there, so the session stays with its project
  afterwards, rather than becoming a project named after the worktree's
  folder.
- **Its profiles:** if a Claude Squad profile starts Claude Code or Codex
  with a config folder of its own (`CLAUDE_CONFIG_DIR=~/.claude-work
  claude`), adding a profile on the Accounts page lists that folder under
  **Other Claude setups**, to bring in as a profile in AI Juggler.
- **What doesn't come across:** Claude Squad's tmux sessions. A session
  still running there keeps running. To carry it on in AI Juggler, stop it
  in Claude Squad, then choose **Continue in AI Juggler** on its row.

From Claude Squad's files, AI Juggler reads only each worktree's folder
and repository (`state.json`) and the commands its profiles start
(`config.json`), without changing either.

## Vibe Kanban

Vibe Kanban keeps each task's workspace in a worktree, by default in your
Mac's temporary folder.

- **Your sessions:** listed like any other, with their branches.
- **Their worktrees:** grouped with their repositories, the same way as
  Claude Squad's, including after Vibe Kanban or macOS removes them.
- **Its profiles:** an executor profile that sets `CLAUDE_CONFIG_DIR` or
  `CODEX_HOME` in its environment is offered when you add a profile, the
  same way.
- **Copy anything you want to keep:** macOS clears the temporary folder
  from time to time. Move work you want to keep into your repository
  before then.
- **What doesn't come across:** the board, its tasks and their reviews.
  AI Juggler doesn't have a kanban board.

AI Juggler reads Vibe Kanban's database without changing it, and only each
workspace's folder and repositories. From its executor profiles it reads
only those two variables, never another value there such as an API key. It
never opens Vibe Kanban's sign-in or key files, or its `config.json`.

## Crystal

Crystal is no longer maintained.

- **Your sessions:** listed like any other. A Crystal worktree is in your project's `worktrees/` folder, so it's
  grouped with its repository.
- **What doesn't come across:** Crystal's own session names. Each row
  shows its branch and what it was about instead.

## cc-switch

cc-switch switches the provider and key that Claude Code and Codex use, by
rewriting `~/.claude/settings.json` (or Codex's `config.toml` and
`auth.json`) in place.

- **One setup, several providers.** For AI Juggler that's one profile:
  your `~/.claude` (or `~/.codex`). Its usage and limits are counted for
  whichever provider is in use when each session ran.
- **Separate profiles instead:** if you want AI Juggler to show each
  provider apart, with its own limits, add a separate copy of the setup for
  each one (add a profile, then **Make a separate copy**) and sign each in
  to its provider. Then switch between them in AI Juggler, not with
  cc-switch.
- **AI Juggler never reads cc-switch's stored keys.** When it finds
  cc-switch, it reads only how many providers there are, and their names.
- **Measuring Claude limits** writes one setting, `statusLine`, into the
  same `settings.json` cc-switch rewrites. If cc-switch overwrites the
  whole file, turn measuring off and on again in AI Juggler to put it back.

## Something missing?

Tell us which tool you came from and what didn't come across, in
[Discussions](https://github.com/figurehead-ai/AI-juggler/discussions).
