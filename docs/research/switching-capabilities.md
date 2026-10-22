# Switching: sessions, capabilities and how they translate

Research for AJ-E1539 to AJ-E1542. Checked against Claude Code 2.1.302 and
Codex CLI 0.163.0, and their official docs, on 2026-10-22: Claude Code's at
code.claude.com/docs/en/ (mcp, settings, memory, skills, sessions,
cli-reference, headless), and Codex's at learn.chatgpt.com/docs/ (MCP,
config reference, AGENTS.md, skills, subagents, non-interactive mode), where
developers.openai.com/codex now redirects. Where the Codex docs were silent,
the openai/codex source on main was read (2026-10-21). Unverified points are
marked; the live lab (AJ-E1539) settles them.

`D` is a Claude account's folder (`$CLAUDE_CONFIG_DIR`, default `~/.claude`)
and `G` its global config (`$CLAUDE_CONFIG_DIR/.claude.json`, or
`~/.claude.json` for the default folder). `H` is a Codex home (`$CODEX_HOME`,
default `~/.codex`).

## Sessions

**Claude Code.**

- **The transcript:** `D/projects/<cwd, every non-alphanumeric as "-">/<session>.jsonl`.
  Beside it are `<session>/subagents/`, `<session>/tool-results/` and
  `D/file-history/<session>/`.
- **Resuming:** `--resume <id>` looks in the current project, then the other
  projects of the same `D`. It resolves only when exactly one other project
  holds the id. `--resume <path to .jsonl>` also works, and `--fork-session`
  resumes under a new id.
- **What resuming doesn't carry:** flags (`--mcp-config`, `--settings`,
  `--add-dir`) must be passed again.
- **Task lists:** they live in `D/tasks/`, one folder per list; `todos/` is
  legacy. TaskCreate, TaskList and the rest replace TodoWrite on current
  models (`CLAUDE_CODE_ENABLE_TASKS=0` brings TodoWrite back). Tasks survive
  resume.
- **Headless:** `claude -p "<prompt>" --session-id <uuid> --output-format json
  --permission-mode acceptEdits --allowedTools … --max-turns N`, then
  `claude -p --resume <id> "<next>"`. The JSON result has `session_id`,
  `result` and `is_error`. Don't use `--bare`: it skips MCP and skills.

**Codex.**

- **The rollout:** `H/sessions/YYYY/MM/DD/rollout-<time>-<uuid>.jsonl`, maybe
  compressed (`.jsonl.zst`), indexed in `H/state_5.sqlite` and
  `session_index.jsonl`.
- **Lookup by id:** it falls back to scanning `sessions/` file names when the
  index lacks the id, then repairs the index. So a rollout copied with its
  own name should resume in another home (from the source; unverified). The
  picker and `--last` may not see it until then.
- **Plans:** `update_plan` keeps no separate state; the plan lives in the
  rollout's tool calls and survives resume.
- **Headless:** `codex exec --json -s workspace-write --skip-git-repo-check -C
  <dir> "<prompt>"`; the `thread.started` event has the id. Then `codex exec
  resume <id> "<next>"`. `--full-auto` is gone. Whether `exec resume` takes
  `--json` is unverified; `-c sandbox_mode=…` is the fallback.
- **Copies:** never copy a rollout while `H/thread-writer-locks` holds it.

## Capabilities: where each lives

| Capability | Claude Code | Codex |
| --- | --- | --- |
| MCP, user | `G` → `mcpServers` | `H/config.toml` `[mcp_servers.<name>]` |
| MCP, per project (private) | `G` → `projects["<abs path>"].mcpServers` | none |
| MCP, per project (shared) | `<repo>/.mcp.json` | `<repo>/.codex/config.toml`, trusted projects only |
| MCP, enabled state | `enabledMcpjsonServers` / `disabledMcpjsonServers` in settings, and per-project toggles in `G` | `enabled = false` |
| Skills | `D/skills/<name>/SKILL.md`, `.claude/skills/`; `skills/synced/` comes from the claude.ai account | `~/.agents/skills/` (shared by every Codex home), `<repo>/.agents/skills/` |
| Commands / prompts | `D/commands/*.md`, `.claude/commands/` | `H/prompts/*.md` (deprecated in favour of skills) |
| Subagents | `D/agents/*.md`, `.claude/agents/` | `H/agents/*.toml`, `.codex/agents/` |
| Rules | `D/CLAUDE.md`, `D/rules/`, project `CLAUDE.md`, `CLAUDE.local.md` | `H/AGENTS.md` (or `AGENTS.override.md`), project `AGENTS.md` files |
| Plugins | `enabledPlugins` in settings; `D/plugins/` | `[plugins."name@mkt"]`; `H/plugins/` |
| Hooks | `hooks` in settings | `H/hooks.json`, `<repo>/.codex/hooks.json` |

**Reading rules across tools:**

- **Claude reading `AGENTS.md`:** it does so natively, but only when no
  `CLAUDE.md` exists in the working folder or above it.
- **Codex reading `CLAUDE.md`:** it reads it only when it's named in
  `project_doc_fallback_filenames`.

**Detection:** parse these files; never run `claude mcp list`, which may
start the servers. `codex mcp list --json` only reads config.

## Translating MCP servers, Claude ↔ Codex

| Claude | Codex | |
| --- | --- | --- |
| `command`, `args` | `command`, `args` | direct |
| `env: {X: "${X}"}` | `env_vars = ["X"]` | direct |
| `env` literal value | `env` | direct, but usually a secret: never copied |
| `env: {X: "${Y}"}`, `${X:-default}`, `${}` in command, args or url | none | needs the user |
| `type: http` / `streamable-http`, `url` | `url` | direct |
| `headers.Authorization: "Bearer ${T}"` | `bearer_token_env_var = "T"` | direct |
| `headers.K: "${V}"` | `env_http_headers.K = "V"` | direct |
| `headers.K` literal | `http_headers` | usually a secret: never copied |
| `headersHelper` | `http_headers_helper` | needs review |
| `timeout` (ms) | `tool_timeout_sec` | direct (÷1000) |
| `oauth` client fields | `oauth` client fields | direct; the token needs a new login |
| `type: sse`, `ws` | none | impossible: Codex has no SSE or WebSocket transport |
| plugin servers, claude.ai connectors | none | impossible: they come with a plugin or an account |

**Claude ↔ Claude:** the same format, copied between the two accounts'
global configs. A project's `.mcp.json` is in the repo and already shared.
OAuth tokens never move; the user logs in on the target.

## Translating the rest

| From | To | |
| --- | --- | --- |
| `D/CLAUDE.md` | `H/AGENTS.md` | copy, with `@imports` inlined; under 32 KiB |
| project `CLAUDE.md` | Codex | add `"CLAUDE.md"` to `project_doc_fallback_filenames` (one line) |
| `D/rules/*.md` without `paths` | `H/AGENTS.md` | append |
| Claude skill | `~/.agents/skills/<name>/` | copy; `name` and `description` required; Claude-only frontmatter, `!`cmd`` and `$N` (0-based) need review |
| Claude command | Codex skill | add `name`; arguments need review |
| Codex prompt | Claude command | `$1..$9` → `$0..$8` |
| Claude subagent | `H/agents/<name>.toml` | name, description, body as `developer_instructions`; tools, model, MCP and hooks don't map |
| Claude task list | Codex | summarised in the handover, not carried as state |
| claude.ai-synced skills, plugins across tools, output styles, hooks | — | can't be set up automatically; listed for the user |

## Secrets

- **Fields that often hold one:**
  - `env` and `headers` values;
  - a `url` with credentials or `?token=` / `?api_key=`;
  - `args` like `--token=…`;
  - Codex's `oauth.client_secret`.
- **How to spot one:**
  - its key name (key, token, secret, password, auth, bearer, cookie,
    session, credential, private, dsn);
  - a known prefix (`sk-`, `ghp_`, `github_pat_`, `glpat-`, `xox`, `AKIA`,
    `AIza`, `eyJ`);
  - or a long high-entropy value.

  A value that is already a `${VAR}` reference is safe.
- **The rule:** a literal secret is never copied. It becomes a named
  variable, and the user is told the target needs it.
