# Probe C2 (and C3), headless, at a shared `codex app-server`: runbook

Task AJ-E3013 (D-116, D-119, D-122: C2 stays in 0.4.6, the maintainer's lab
Codex sign-in first). Route: option 2 (the coordinating session,
2026-11-16): the question "when several threads share one `codex
app-server`, can the MCP server it spawns tell which thread is calling?"
is answered at its source, the `codex` CLI's own app-server, without
launching the ChatGPT app. Research: `docs/research/desktop-pool-members.md`
§4.4, §7.0, §7.4. Driver: `scripts/qa/c2_probe.py` (standard library, run
with `python3 -I`). Staging record: `c2-staging.md` beside this file.

## The lab

- `/tmp/aij-lab-c2/` (0700): `codex-home/` (the lab `CODEX_HOME`),
  `work/` (the threads' folder, outside the home folder, so no project
  config lookup climbs to `~/.codex`), `bin/juggler-agent` (the debug
  probe, copied and pinned by `stage`; its sha256 is in every report),
  `evidence/` (reports, 0600), `probe.jsonl` (the probe's records, 0600).
- `codex-home/config.toml` (written by `stage`, refused if edited):
  - `cli_auth_credentials_store = "file"`: the sign-in goes to the lab's
    `auth.json`, never the Keychain. Checked against codex 0.163.0: the
    enum `AuthCredentialsStoreMode` is `file | keyring | auto | ephemeral`
    (binary strings; `codex --help` only documents `-c`).
    `mcp_oauth_credentials_store = "file"` too.
  - `approval_policy = "on-request"`, `sandbox_mode = "read-only"`.
  - `check_for_update_on_startup = false`, `[analytics] enabled = false`.
  - `[features]` off: `plugins`, `remote_plugin`, `apps`,
    `skill_mcp_dependency_install`, `tool_suggest`, `in_app_updates`,
    `daemon_auto_start`. The first dry run found the app-server cloning
    github.com/openai/plugins (93 MB) into the lab home at start (deleted).
    These flags stop that and any other fetch or shared daemon.
  - `[mcp_servers.juggler-probe]`: `command` is the pinned probe, `args =
    ["--mcp", "--probe", "/tmp/aij-lab-c2/probe.jsonl"]`, `enabled_tools =
    ["probe"]`; `[mcp_servers.juggler-probe.tools.probe] approval_mode =
    "approve"` (`AppToolApproval`: `auto | prompt | writes | approve`, from
    the generated schema). Every run starts the server with
    `--strict-config`, so an unknown key stops it.

## The guards (every step that runs codex)

- **Refuses** when `CODEX_HOME` names another folder, a lab folder isn't
  this user's 0700, another Codex runs (the ChatGPT app, or any codex
  program not walled into its own lab home), the lab still has processes,
  or `~/.codex` differs from the staged baseline (the names and
  modification dates of all its entries; no file in it is opened).
- **Runs codex** with a small environment (HOME, USER, LOGNAME, SHELL,
  TMPDIR, LANG, PATH, `CODEX_HOME`, `CODEX_SQLITE_HOME`) inside a
  `sandbox-exec` profile that denies every read and write under
  `~/.codex` (`--no-guard` drops only the sandbox). The server's own
  `initialize` answer must name the lab home, or it is stopped.
- **Watches** ps and lsof every 0.5 s over the server's whole tree: every
  process started and its `CODEX_HOME`, anything open under `~/.codex` or
  `~/Library/Keychains`, every internet socket.
- **Answers** what a person would: yes only to the probe's own tool and to
  exactly `<probe> --probe-shell`, no to all else. Each request is recorded:
  it is C2's "approval prompt" answer.
- **After:** `~/.codex` compared again; every process reaped (its process
  group, checked to carry the lab home, or the pinned probe's path).
  Exit 0 only when `~/.codex` is unchanged, nothing was open under it and
  nothing is left running.

## Steps

0. **Staged (2026-11-16):** `stage`, `dry` (guarded and not), `preflight`
   (`c2-staging.md`).
1. **The sitting (the maintainer's one step).** The ChatGPT app and any
   terminal codex are quit first (the driver refuses otherwise). Never a
   bare `codex login`: without `CODEX_HOME` it signs in on `~/.codex`. The
   stager runs `python3 -I scripts/qa/c2_probe.py login`. It runs, in the
   guard:
   `CODEX_HOME=/tmp/aij-lab-c2/codex-home codex login -c
   cli_auth_credentials_store="file"`. Codex starts its sign-in server on
   localhost:1455, tries to open the browser itself and prints the sign-in
   page's address. The driver also opens that address with `/usr/bin/open`
   (the Mac's default browser), every time, because the maintainer can't
   see the stager's shell (the sitting failed once on exactly that); two
   tabs of the same page are harmless. The maintainer signs in on the
   page; codex exits. It is stopped after 15 minutes. Checks: the lab home
   has `auth.json` (by name, never opened); `~/.codex` unchanged (names and
   dates, so its own `auth.json` too); nothing left running.
2. **The run (one command):** `python3 -I scripts/qa/c2_probe.py run`
   (about 3 to 6 minutes, three model turns). One app-server, two threads;
   each asked "Call the `probe` tool of the juggler-probe MCP server exactly
   once…"; then thread 1 asked to run exactly `<probe> --probe-shell` (C3),
   and thread 2 the same through `thread/shellCommand` (no model).
3. **Read** `/tmp/aij-lab-c2/evidence/run-<stamp>.md` (and `.json`). Copy
   the `.md` here as `c2-run.md` (the home folder already shows as `~`; the
   account's email is never read). Opus checks the verdicts before they are
   recorded.
4. **Clean up:** `reap` if anything is left. Delete `/tmp/aij-lab-c2` when
   done: it holds the lab sign-in.

## What `run` answers, and what it means

| C2 question | Where | Expected |
|---|---|---|
| One probe process, or one per thread | `Probe processes` (pids per thread) | per thread (preview: 2 for 2) |
| Each one's parent | the driver's own ps chain | the native `codex app-server` |
| A thread id in the MCP server's environment | `CODEX_THREAD_ID in the MCP server's environment` | no |
| `_meta` names the calling thread, or a connection lease | `meta_keys`, `meta_nested`, `meta_ids` (values of id keys only) | `threadId` (preview) |
| An approval prompt | `Approval prompts during the calls` | none (`approve`) |
| C3: the thread's shell carries `CODEX_THREAD_ID` | `C3 (...)` lines | matches; ancestry reaches the app-server |

- **Each thread's calls carry its own thread id in `_meta`** ("C2 RESULT:
  each call's _meta names its own thread"): Codex app threads **can** be
  members. The next step is the attestation design (Opus, ADR-0020): the
  id is the app-server's word, so it rests on the kernel naming that
  app-server as the MCP server's parent and on its signature (OpenAI's),
  as M1 does for the Claude app.
- **No thread id in `_meta`:** Codex app threads **stay Watched** (D-119,
  D-122). The shell route (C3: `CODEX_THREAD_ID` plus ancestry) is a
  separate ruling.

## Limits of this answer

- **Version.** The CLI is 0.163.0; the ChatGPT app bundles
  0.169.0-alpha.11.1 (`codex-package.json`, read only). 0.169 has
  `codex/connectionLease` and "missing MCP request metadata object",
  0.163 has neither. A 0.163 result carries to the app as far as 0.169
  keeps `_meta.threadId`. Updating the CLI is a download: the
  maintainer's yes.
- **The desktop UI's wiring is inferred**, not observed: the app spawns
  the same `codex app-server`, but no ChatGPT app ran (ADR-0020, D-116
  addendum).
- `/bin/ps` is setuid root, so a sandboxed process (Codex's seatbelt, the
  guard) can't run it: the probe then records only its pid and parent's,
  and the driver's own ps names the chain.
- Codex runs the user's login shell for its shell snapshot (`zsh -lc`,
  sourcing the dotfiles) without `CODEX_HOME`. The dotfiles don't mention
  codex (2026-11-16), and the guard denies `~/.codex` to that shell too.
