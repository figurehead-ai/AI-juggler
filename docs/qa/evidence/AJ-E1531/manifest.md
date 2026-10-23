# Evidence manifest — AJ-E1531

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1531 (a realistic replica for
walks and tests; a 0.1 blocker). Status: **DONE-EVIDENCED**. 2026-10-21,
reviewer claude-opus.

## How to use it

```sh
just replica                # build /tmp/aij-replica (AIJ_REPLICA_HOME to move it)
just replica-live 60        # live sessions for 60 minutes (Ctrl-C stops them)
just replica-walk           # AI Juggler Dev with HOME=/tmp/aij-replica
python3 -I scripts/replica.py truth --home /tmp/aij-replica   # the ground truth
```

`replica-live` takes the generator's options after the minutes, for
example `just replica-live 30 --claude 8 --codex 2 --pressure-mb 4096`.

## What it builds

Seed 1531, 120 days:

| | |
|---|---|
| Claude | `~/.claude` and six profiles (`work`, `personal`, `client-acme`, `client-globex`, `max-2`, `oss`), each with `settings.json`, `.claude.json` and fake credentials. Two already have a status line of their own (for AJ-E1530's chaining). |
| Codex | `~/.codex` and `~/.codex-work`, with `config.toml`, a fake `auth.json`, and rollouts whose `token_count` events carry `rate_limits` (`primary`/`secondary`, `used_percent`, `window_minutes`, `resets_at`). |
| Projects | 16 git repositories under `~/Dev`, among them three clones of one repo (`acme-web`, `acme-web2`, `acme-web3`: the same origin) and a Codex-only project. |
| Worktrees | five agent worktrees under `<repo>/.claude/worktrees/<name>`, in each state seen on the maintainer's Mac: present (2); removed, so only their transcripts remain (2); pruned, so the folder remains but its `.git` file is gone (1). |
| History | about 340 conversations across the accounts and projects, from minutes to months old. Each Claude transcript has a `summary` title, turns with `message.usage`, `cwd`, `gitBranch` and `sessionId`, and its mtime set to its last line. |
| Icons | the kinds AJ-E1532 reads: a Tauri icon, a favicon, a root logo, a `.github` logo, an oversized logo (600 KB) and a `logo.png` that isn't an image. |
| Shell | `.zshrc` aliases pointing `CLAUDE_CONFIG_DIR` and `CODEX_HOME` at profiles. |

Live (`replica.py live`), all exiting when the walk ends:

- `claude` and `codex` processes: the fake harness's new `write` mode under those names, appending Claude- and Codex-shaped lines every few seconds;
- a `Claude.app/Contents/MacOS/Claude` stand-in;
- a `ChatGPT.app` helper, which is not a Claude tool;
- optional memory pressure (`--pressure-mb`).

Nothing has real content. Addresses are `@example.test` (RFC 6761), and
tokens read `replica-not-a-real-token`.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 the build, deterministic, refusing a real HOME | `replica.py selftest`: the same seed builds the same truth; 7 Claude accounts, 13+ projects, all three worktree states; every conversation's project exists; every transcript and rollout line is JSON; Claude's folder names follow the cwd rule; rollouts carry `rate_limits`; a non-empty, non-replica HOME is refused and left untouched | PASS |
| Req.1 the app reads it as a real Mac | `crates/juggler-adapters/tests/replica_scan.rs`: the adapters' own scans find exactly the replica's 7 Claude and 2 Codex homes, no more and no fewer | PASS |
| Req.2 live sessions | run for a minute with 3 Claude and 1 Codex. `ps` showed `…/bin/claude` ×3, `…/bin/codex`, `…/Claude.app/Contents/MacOS/Claude` and `…/ChatGPT Helper`; their transcripts grew; `truth.json` listed the live sessions and cleared them when the run ended; no process was left behind | PASS |
| Req.3 ground truth | `~/.replica/truth.json`: accounts, projects with conversation counts, worktrees with their state, conversations (tool, account, project, cwd, id, title, last, tokens) and live sessions | PASS |
| Req.4 `just replica-walk` | AI Juggler Dev started on the replica HOME, and its data folder was created under `/tmp/aij-replica/Library/Application Support/AI Juggler Dev`; cargo, rustup and corepack kept their own homes | PASS |

Req.5 (Playwright fixtures made by the backend's own readers from the
replica) moved to AJ-E1529, whose readers produce them.

## Not touched

The generator writes only inside the HOME it's given. It never reads the
real HOME, and it refuses to run there.
