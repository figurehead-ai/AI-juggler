# ADR-0020: The agent bridge — `juggler-agent`, a per-edition socket, a per-epoch key

- Status: Accepted for the read-only bridge (AJ-E1704, 2026-10-25). The
  live spike on spare accounts (AJ-E1701 Req.2) is the maintainer's and
  still open: the points it must confirm are marked **unverified** below.
- Date: 2026-10-25
- Deciders: the maintainer (D-28 to D-33, 2026-10-24), agents (AJ-E1704)
- Relates to: ADR-0002 (adapters), ADR-0003 (automation policy),
  ADR-0010 (spawn ledger), ADR-0018 (per-launch files in AI Juggler's
  folder), ADR-0019 (tools as data), D-28, D-30, D-33, D-37;
  SECURITY-GUIDELINES C7; AJ-E2002 (the bridge's security design)

## Context

Agents in AI Juggler's sessions should be able to ask who they are and
who's in their pool, across tools (Claude Code, Codex, and later tools
without MCP). The pools, their access levels and the one decision point
`allowed(pool, action)` exist (AJ-E1702); the context cards exist
(AJ-E1703). Research: `docs/research/pools-messaging-tasks.md` §2.3.

## Decision

1. **A program, not a server in the app.** `juggler-agent` (crate
   `juggler-agentbridge`, `unsafe_code = "forbid"`, depending on nothing
   of ours) ships in the bundle beside the app's executable. With `--mcp`
   it is an MCP server on stdio; without it, a plain command line
   (`whoami`, `roster`, `cards`, `status <text>`) for tools without MCP. It
   holds no state: each call is one request line to the app and one
   answer line back.
2. **MCP by hand.** Newline-delimited JSON-RPC 2.0, about 300 lines,
   unit-tested: the `initialize` handshake (2024-11-05 to 2025-11-25,
   echoing a known version) and the stateless 2026-07-28 revision
   (`server/discover`; `_meta` accepted and not needed). No MCP SDK: no
   new dependency to audit (serde and serde_json are already the
   workspace's), and nothing in it the bridge doesn't use.
3. **One socket per edition**, in the app's own data folder:
   `<data>/agent/bridge.sock`, the folder `0700`, the socket `0600`. Every
   connection's peer user is checked (`getpeereid` through tokio's
   `peer_cred`); one request per connection, at most 8 KiB, read within
   2 s, at most 16 at once. A stale socket is removed only when `lstat`
   shows it is a socket of ours. No TCP or HTTP listener. Served lazily
   (the first launch that gets the bridge, or the app's start when
   sessions still hold keys).
4. **Identity is a per-epoch key.** Each launch AI Juggler starts with the
   bridge on gets 32 random bytes (hex) in its **environment**
   (`JUGGLER_AGENT_TOKEN`), with the socket's path (`JUGGLER_AGENT_SOCKET`)
   and the program's (`JUGGLER_AGENT`). Never in argv, never in a file. A
   switch or a resume is a new launch, so a new key; the session's older
   keys stop at once, and a key of a session that ended stops working.
   The app keeps only each key's SHA-256 with its session, in memory and
   in a `0600` file in the `0700` folder (so sessions that outlive a
   restart keep their tools). The bridge reads its own environment; the
   app never reads another process's (D-37): it wrote the key into the
   launch. The bridge sends the key only to a socket in a folder no one
   else can write, owned by the same user.
5. **Injection through the adapters' launch specs** (D-30, ADR-0019): the
   app hands each adapter a `BridgeLaunch` (program, server name, its
   tools, the variables' *names*, and the launch's folder in
   `overlays/agent/<session>/`); the adapter returns options to insert and
   files to write (`HarnessAdapter::agent_bridge`, declared by
   `Capabilities::AGENT_BRIDGE`). The conformance kit holds every adapter
   to it (no value from the environment, files by plain name only in the
   launch's folder, nothing pointing into the account's folder, the
   launch's arguments kept in order, no wildcard).
   - Claude Code: `--mcp-config <launch folder>/juggler-mcp.json`
     (its `env` is `${JUGGLER_AGENT_TOKEN}`, `${JUGGLER_AGENT_SOCKET}`:
     filled by Claude Code from its own environment), never
     `--strict-mcp-config`; `--allowedTools` for
     `mcp__juggler__{whoami,pool_roster,set_status}` only. Both after the
     launch's other arguments (they take several values).
   - Codex: `-c mcp_servers.juggler.command=…`, `.args=["--mcp"]`,
     `.env_vars=[…names…]`, `.enabled_tools=[the three]`,
     `.default_tools_approval_mode="prompt"` and
     `.tools.<tool>.approval_mode="approve"` for each of the three, before
     the `--` that ends its options. (E17's text said `writes`; D-30
     rejected pre-allowing read-only tools only, and `writes` would ask
     about `set_status`.)
   - Tools without MCP by launch option: the environment only (the
     command line from their shell).
   On after the one-time question (D-30), off per project or profile in
   Settings › Agent access; a launch that can't get the bridge (no
   program, no socket, a file that can't be written) starts without it,
   never fails.
6. **Every call goes through `allowed(pool, action)`** (D-28), which
   decides and audits: `whoami` and `pool_roster` are `read_roster`, the
   cards resource `read_cards`, `set_status` `set_status`. Before that,
   in order: the request's one shape (`deny_unknown_fields`: no field
   names another identity; since D-62 a call may name which of the
   session's *own* pools it's for, see the D-62 addendum), the key, a rate limit (30 calls in
   10 s per session), the switches. An ask is a refusal that says the
   user decides, until E18 brings asks agents can wait on.
7. **What agents see** (D-33): names, places, tools, states, roles,
   profile labels (never an email, a plan, an id or a folder), and what
   each agent said of itself, marked as its own words (`says`, ≤ 140
   characters, one line, control characters removed). The status line
   is kept per member (Work item), in memory.
8. **The ledger** knows the program by name as an MCP server owned by its
   session (`guess_role`), so it is never cleaned up automatically
   (ADR-0010's tiers exclude `McpServer`); it exits when its session
   closes its stdin.
9. **Signing:** the release workflow builds it for both chips, `lipo`s
   it, and makes it Tauri's `externalBin` (`Contents/MacOS/juggler-agent`);
   `sign_macos.sh` names it as the bundle's one helper, signs it on its
   own (hardened runtime, no entitlements) before the app, and refuses a
   bundle without it.

## STRIDE (SECURITY-GUIDELINES C7)

| Threat | Where | Mitigation | Test |
|---|---|---|---|
| **S**poofing: another local process poses as an agent | the socket | `0700` folder, `0600` socket, peer uid = ours, then a 256-bit key only the launched session has; a key is checked by its SHA-256, never stored | `the_socket_is_0600_in_a_0700_folder_and_answers_only_current_keys` |
| Spoofing: an agent claims another session or pool | the request | identity is the key alone; the request has no field for a session (`deny_unknown_fields`); its `pool` field (D-62) only chooses among the pools the resolver puts the key's session in, and any other is refused | `each_level_refuses_…` (the "sneaky" request), `a_request_round_trips_and_refuses_strangers` |
| **T**ampering: a launch file changed to point elsewhere | `overlays/agent/<session>/` | written `0600` in `0700` folders that must be ours; the file holds no secret | `the_key_is_in_the_environment_only_…` |
| **R**epudiation | every call | `allowed()` writes the audit (pool, actor, action, decision) for every call that reaches a decision | `each_level_refuses_…_every_call_is_audited` |
| **I**nformation disclosure: the key leaks | argv, files, logs | environment only; only its hash on disk; the ledger records argv, which never has it | `the_key_is_in_the_environment_only_…`, `the_command_line_reads_the_key_from_its_environment_only` |
| Information disclosure: account details reach agents | roster, cards | labels only (D-33); no email, plan, id or folder | `agents_see_account_labels_never_emails_or_plans` |
| **D**enial of service: call storms | the socket | 16 at once, 8 KiB, 2 s to send, 30 calls / 10 s per session | (design; the rate limit's refusal is `Limit`) |
| **E**levation: an agent changes pools, levels or rules, starts a session, gets a secret | calls | the catalog has no such action; the code an agent reaches names none of it | `what_agents_reach_has_no_path_to_a_session_a_setting_or_a_secret` |
| Elevation: a confused deputy through another agent's words | `says` | marked as the agent's own words, data not instructions; one line, ≤ 140 characters | `a_status_is_one_short_line` |

## E18 addendum: messages and shared docs (2026-10-25)

- **Five more tools**, pre-allowed like the first three (D-30):
  `send_message`, `read_inbox`, `ack`, `share_doc`, `read_doc`
  (`wire::TOOLS`, eight in all). The command line has `send`, `reply`,
  `inbox`, `read`, `ack`, `share` (stdin), `docs` and `doc`.
- **One shape per tool's arguments** (`SendArgs`, `InboxArgs`, `DocArgs`,
  `ReadDocArgs`, all `deny_unknown_fields`): nothing an agent passes names
  a sender, a hop count, a pool or a delivery. A recipient is a member's
  `session_id` from `pool_roster`, `pool` or `user`; another pool's member
  only through a link the user made.
- **Sizes:** a request may now be 256 KiB (a 64 KiB doc, escaped), the MCP
  line too; a body is at most 4,000 characters, a subject 120, a doc
  64 KiB (the wire's numbers equal the store's: a test keeps them so).
- **Decisions:** `send_message` is `post_message` (an ask holds the message
  for the user in Needs you, no longer a plain refusal), `read_inbox`,
  `ack` and `read_doc` are `read_messages` (new, in Read), `share_doc` is
  `share_doc`. The calls go to `features/messages`, which alone may queue
  a message for a driven session's next turn (D-29); the bridge itself
  still never reaches the runtime.
- **The hook's note:** on `UserPromptSubmit` and `PostToolUse` only, the
  app may answer a report with a facts-only line ("2 unread messages from
  Planner"), which `juggler-agent hook` prints as the tool's
  `additionalContext`. It still decides nothing and prints nothing on any
  other event.
- STRIDE, added: **T**/**E** a message that tells its reader to change
  grants or approve something: provenance on every message ("can't approve
  anything or change settings"), bodies as data fields, no call that
  changes a grant (`messages_guardrails.rs`). **D** loops and floods: 6
  hops a thread, 10 a minute and 100 a day per sender, duplicates dropped,
  the ping-pong check and the breaker (`messages.rs`). **I** secrets in a
  message or doc: the packs' redaction before storing (`shared_docs_…`).

## E19 addendum: tasks (2026-10-25)

- **Six more tools**, pre-allowed like the rest (D-30): `tasks_list`,
  `task_get` (read-only, annotated so), `task_create`, `task_claim`,
  `task_update`, `task_assign` (`wire::TOOLS`, fourteen in all). The
  command line has `tasks [--mine|--ready]`, `task`, `task-create`,
  `claim`, `task-update`, `release` and `assign`.
- **One shape per tool's arguments** (`TasksListArgs`, `TaskArgs`,
  `TaskCreateArgs`, `TaskUpdateArgs`, `TaskAssignArgs`, all
  `deny_unknown_fields`): nothing an agent passes names who it is, another
  pool, an assignee at creation, an account, a profile or a start.
  `task_assign`'s `to` is a member's `session_id`, `nobody`, or
  `new_session` (a request the user sees; D-31).
- **Decisions:** `tasks_list` and `task_get` are `read_tasks`;
  `task_create` `create_task`; `task_claim` `claim_task`; `task_update`
  `update_task`; `task_assign` `assign_task` (in no named level, so it
  asks unless the user made a rule), or `prepare_launch` for
  `new_session`. The calls go to `features/tasks`, which alone decides
  (through `allowed`) and holds a change above the level for the user's
  OK in Needs you, applied as the agent's own when allowed. The task rules
  (`juggler_core::tasks`) sit under the level: the lead plans epics and
  stories and gives tasks out; a member works what it holds.
- **A claim is one statement** (`UPDATE … WHERE status = 'ready' AND
  assignee is no one else AND no open blocker`): of N agents, one wins.
- **The Claude Code plugin** (`plugins/claude-code/`, AJ-E1905) runs the
  same program as `juggler-agent --mcp --plugin`: in a session AI Juggler
  started (a key in its environment) it lists no tools, so the launch's
  own aren't listed twice; anywhere else each call says why it can't. It
  never carries a key: identity still comes only with a launch.
- STRIDE, added: **E** an agent starts a session or picks an account
  through a task: no tool argument names one, an agent's request only
  waits in Needs you, and only the user's command reaches the launch
  (`tasks_guardrails.rs`). **T** a task's words that tell its reader to
  change grants: provenance (`written_by`) on every task, words as data
  fields, no call that changes a grant. **D** claim storms: the call rate
  limit (30 in 10 s) and one statement per claim. **I** quota: nothing in
  tasks reads it (`assignment_never_reads_quota`).

## D-62 addendum: several pools per session (2026-11-04, AJ-E3004)

D-62 lets a session be in several pools (decision 10), so §6's "no field
names a pool" is amended:

- Every call may carry `pool`: the id of one of the session's pools, as
  `whoami`'s `pools` lists them (each with its role, how it got in and
  whether it's sealed). The MCP tools take it as an optional `pool`
  argument; the command line as `juggler-agent --pool <id> <call> …`.
- The app resolves the key's session first (identity is still the key
  alone), then checks the named pool is one the resolver puts it in. A
  pool it isn't in is refused ("This session isn't in the pool …"), and
  audited like any refusal.
- Without `pool`, a call acts in the session's first pool. A session in
  several pools must name one on every write (`send_message`,
  `share_doc`, `task_create`, `task_claim`, `task_update`,
  `task_assign`); reads use the first pool.
- What a call may do is that pool's level and rules, or the member's own
  level where it carries one (D-62 decision 12).

STRIDE: spoofing gains no route, because the field chooses only among the
caller's own pools; elevation gains none, because each pool's own level
applies to what's done in it. Tests: `a_request_round_trips_and_refuses_strangers`
(the field, and no identity field), and the app's bridge tests (a pool it
isn't in is refused; a write in two pools must name one).

## Capability-tagged data (AJ-E2002, 2026-11-06)

The rules above, stated once as the confused-deputy defence (the CaMeL
idea: data carries where it came from, and only the user's click turns
data into an action):

- **Every value an agent supplies is data tagged with its origin**: the
  calling session and its pool, from the connection, never from a field.
  A message, a task, a doc and a status keep that tag (`written_by`,
  provenance on every read), and the reader sees it as another agent's
  words.
- **Tagged data never chooses who is acted on.** A recipient is a
  `session_id` from the pool's own roster, or `pool` or `user`; no field
  names a pool, a grant, an account or a path (`deny_unknown_fields`).
- **Tagged data never becomes the app's instruction.** Nothing an agent
  sends changes a pool, a level, a rule or a grant, starts a session or
  reads a secret; what needs more waits in Needs you for the user's click
  (an ask holds the message, it never answers itself).
- **The only capability that crosses is the user's click**, in the
  window, on the target's id.

Threat row (SECURITY-GUIDELINES C7): *Elevation — a confused deputy,
where one agent's words make the app or another agent act with
authority it doesn't have.* Mitigation: the four rules above. Tests:
`messages_guardrails.rs`
(`grants_cant_change_through_the_bridge`,
`provenance_is_on_every_message_an_agent_reads_and_every_delivery`,
`a_message_reaches_a_sessions_input_only_through_its_turn_and_only_with_permission`,
`the_caps_hold`), `wire.rs` (`a_request_round_trips_and_refuses_strangers`)
and the guardrail suite's "no action without the user's command".

## D-116 addendum: attested desktop identity (AJ-E3013, 2026-11-15)

D-116 makes a desktop app's session a full pool member through AI
Juggler's connector (the bridge as an MCP server, and for the Claude
app's Code tab the plugin's hooks), and D-119 lets such a member be woken
through Claude Code's own inbox. AI Juggler doesn't start those sessions,
so they never get a key (§4). This addendum adds the one other way a call
gets an identity: **attestation**, checked by the app against the
operating system and the tool's own records on every call. Research:
`docs/research/desktop-pool-members.md` §1.1.

**The connector.** Written by AI Juggler with the user's consent, into
that App's own config folder only (D-116): the bridge runs as
`<agent> --mcp --plugin`, and the hooks as `<agent> hook <event>`, with
the edition's socket path in `JUGGLER_AGENT_SOCKET` (a path, not a
secret). Nothing it writes holds a key, a token or a credential.

**The call.** With no key in its environment and a socket named, the
bridge sends its usual request with an **empty key**. The request's one
shape is unchanged: no field names a session, a process, a folder or a
pool other than one of its own (D-62). Nothing the caller says is part of
the attestation.

**The check** (the app, `features/agent_bridge/attest.rs`; the tool's
part in its adapter, `juggler_adapter::peer::DesktopPeer`):

1. **The peer.** At accept, the socket's peer user must be this user
   (`getpeereid`, as before), and the socket's **audit token**
   (`LOCAL_PEERTOKEN`, read through nix's safe `getsockopt` right at
   accept) names the socket's last user at `getsockopt` time: its pid and
   that pid's version. The live process at that pid must have the same
   version (`juggler_proc_list::pid_version`, the kernel's `p_idversion`,
   which changes whenever a pid is reused, and on exec), so a pid reused
   after the accept is caught. Before the read there is only the moment
   between the connection and the accept (microseconds); a caller can't
   steer it to name a process that never used its socket, and whichever
   process the token names is the one checked, whole. The time of the
   accept is kept.
2. **The chain.** One listing of the processes (`juggler-proc-list`, the
   kernel's own `kinfo_proc` records): the peer must be listed, not a
   zombie, readable as this user's (`proc_pidinfo`), and started no later
   than the accept. Then its parent and its parent's parent, each listed,
   none a zombie, and each started no later than its child (a reparented
   or reused parent fails).
3. **The tool's registry** (Claude, `claude_peer.rs`). For the parent,
   then the grandparent (a hook runs under `sh`): in the App's own
   `CLAUDE_CONFIG_DIR`, `sessions/<pid>.json`: a regular file of this
   user's (never a link), small, its `pid` equal to its name. Of it only
   `pid`, `sessionId`, `hostSessionId`, `entrypoint`, `status` and
   `procStart` are read; the `*.key` files beside it are never opened.
   It must say `entrypoint: "claude-desktop"`, carry a `procStart` within
   one second of that process's real start time (**pid reuse**: a stale
   file for a reused pid fails), and a `hostSessionId` of the form
   `local_<id>`.
4. **The App's record.** That `hostSessionId` must name the App's own
   record, `<instance>/claude-code-sessions/<account>/<org>/<hostSessionId>.json`
   (a regular file, never a link), whose `cliSessionId` equals the
   registry's `sessionId`, and which the app hasn't archived. Its
   conversation is the member: `(the App's config folder row, that id)`.
4b. **The running code (M1, round 2).** The registry's word isn't
   enough, and neither is a file: the **code running** as the process it
   names, and as the App above it, is checked.
   - *A filter first:* the tool's executable (`proc_pidpath`, through
     `sysinfo`) has no link on its path and sits in the App's own folder
     (`<instance>/claude-code/<version>/…/Contents/MacOS/claude`). This
     only narrows; it proves nothing.
   - *The tool's running code:* the kernel's code-signing host is asked
     for the guest with the tool's pid (`SecCodeCopyGuestWithAttributes`)
     and its dynamic validity is checked against an explicit requirement
     (`SecCodeCheckValidity`), through `security-framework`'s safe API (no
     `unsafe`): `identifier "com.anthropic.claude-code" and anchor apple
     generic and certificate leaf[subject.OU] = "Q6L2SF6YDW"`.
   - *The host, by its running code only:* the tool's parent must meet
     the Claude app's own requirement, `identifier
     "com.anthropic.claudefordesktop"` (the bundle identifier read once
     from the installed app's `Info.plist` and pinned, with a test) and
     the same anchor and team; or the parent must be the app's own
     `disclaimer` helper, through which the app starts Claude Code
     (pinned: `identifier "disclaimer"`, the same anchor and team, so no
     other Anthropic-signed program, Claude Code itself or
     `app-cu-helper`, stands between them) and the grandparent meet the
     app's requirement. No path, bundle or folder name is read for this:
     a program at `~/x/Claude.app/Contents/MacOS/Claude` is no host.
   - *The same processes throughout:* each pid's version (`p_idversion`,
     which the kernel changes when the pid is reused and when the process
     execs) is read before its check and again after it, and must be the
     same; once all checks are done, the kernel's listing is read again
     and every process checked must still have the start time and the
     parent it had, and the pid version its check saw (an exec after the
     checks is caught too). Verdicts are kept per (pid, pid version, requirement),
     never per path (`juggler_platform::code_sign`).
   - *The instance (the third review's Low 1):* that verified main
     process's own `--user-data-dir`, read the strict way (only
     `--user-data-dir=<folder>`, once, before any `--`; any other form, a
     second one or one after `--` is refused, so what AI Juggler reads
     can't differ from what the app reads), else the app's default
     folder, is this App's instance folder, as real paths; and that very
     process holds the instance's own lock (the adapter names it: for the
     Claude app, Chromium's `Local Storage/leveldb/LOCK`, an `fcntl` lock
     asked about with `F_GETLK`, which takes none, the file reached
     through the no-link walk). The Claude app takes no `SingletonLock`
     (none of five running instances had one, 2026-11-15), so its storage
     lock is the proof that a process runs that instance.

     **What this rests on, and how it fails.** Chromium keeping its
     storage service, and so that lock, in the browser (main) process. An
     Electron update that moves it to a utility process makes the holder
     another process, or no one: every attestation then fails closed with
     "profile lock held by another process, maybe after a Claude app
     update", said to the caller and logged under H2 (scrubbed, once a
     minute a reason; test
     `a_profile_lock_held_elsewhere_is_refused_visibly_and_logged_once_a_minute`).
     **Each release candidate re-checks it on a real Mac:** `just
     desktop-host-checks` (read-only: it reads processes, signatures and
     the fuse wire, and asks who holds the lock; it starts, locks and
     changes nothing), a row in `docs/qa/0.4-walks/release-candidate.md`
     §2, so an app update that breaks the check is caught before the
     release, not by users.

   **What this guarantees, exactly:** the caller (the socket's last user
   at `getsockopt` time, still that process) descends, at most two steps
   down, from a process whose running code is Anthropic's Claude Code,
   whose parent, directly or through the app's `disclaimer` helper, is the
   Claude app's own running main process for this App's instance; each of
   those was the same process before and after its check. It does **not**
   guarantee that the caller is AI Juggler's connector rather than
   something else that session runs (a hook, an MCP server the user added,
   a command the model runs: they speak as the session, row below), nor
   which of the App's conversations it is beyond what the user-writable
   registry entry and App record say (the threat model below).

   **What the helper hop leaves (the third review, Low 2).** `disclaimer`
   is a generic launcher: the app starts programs through it, Claude Code
   among them. So "the app → `disclaimer` → genuine Claude Code" proves
   the Claude app started that Claude Code; it does not prove the Claude
   Code is a Code-tab session. That part rests on the registry entry
   (`entrypoint: "claude-desktop"`, its `hostSessionId`) and the App's
   record, files this user, and so any process of this user, can write.
   Inside the same-user boundary this leaves one case: a process that is
   already a descendant (at most two steps down) of a Claude Code the
   Claude app started, for any of its features, together with forged
   registry and record files, is attested as a Code-tab conversation of
   that App instance. It can't make a script outside such a tree pass,
   can't pass for another App or profile, and gains nothing a process of
   this user doesn't already have (it can read a launched session's key,
   §4; it can drive the app as the user does).

   **What it rests on:** macOS enforcing code signing for running code;
   both programs signed with the hardened runtime (`flags=0x10000`), with
   no `get-task-allow` and no `allow-dyld-environment-variables`, so
   another process of the same user can't attach to them or inject a
   library through the environment (the CLI's `disable-library-validation`
   lets only its own code choose what it loads); and the Claude app's
   Electron fuses: `RunAsNode`, `EnableNodeOptionsEnvironmentVariable` and
   `EnableNodeCliInspectArguments` **off**, with embedded asar integrity
   validation and only-load-app-from-asar **on** (the fuse wire read from
   the installed app 2026-11-15: `010011011`). If a Claude app update turns
   any of the first three on, a same-user process could start the genuine
   app binary as a Node interpreter running its own script, or under a
   debugger, and it would pass the host check: then the Claude connector
   must stop being offered (`DesktopPeer::not_offered`) until the check
   is rethought. The adapter's test
   `the_installed_claude_apps_fuses_stay_off` reads the installed app's
   fuse wire on every Mac that has it; re-check it with each Claude app
   update a release covers, and the identifiers and team if Anthropic
   ever changes them (the check then refuses, closed).
5. **One App.** Every desktop App the user added (D-84) whose tool's
   adapter can attest is asked; exactly one must attest. None, or two,
   is a refusal.
6. **Its place.** The app builds the session's facts (App, Account, Tool,
   conversation; project from the conversation index) under a minted id,
   `desktop:<folder>/<conversation>` (L3), which no real session can have
   (`SessionsRepo::insert` refuses the prefix), and keeps them in memory
   (`Storage::attested`), refreshed by each attested call and dropped ten
   minutes after the last one, or at once when a later call finds that
   session's registry entry stale or its conversation archived by the app
   (or its record gone, or naming another conversation). Only this path writes it. Then the call
   goes on exactly as a keyed one: the rate limit (per conversation),
   Settings › Agent access, the pool's level, the talk rules, and
   `allowed(pool, action)`, which decides and audits.

**Refused, never trusted.** Anything short of all of the above (no peer
pid, a process missing, a registry entry missing, of another shape, a
link, without `procStart`, with a start time that doesn't match, an App
record missing or naming another conversation, no App or two) is
`Refusal::Unattested`, with the reason. There is no fallback to the
caller's word, the environment, a cached verdict or a guess. A Codex app
thread is refused (one shared `codex app-server`, no per-thread
process: `codex_peer` answers "can't tell which thread is calling" until
a lab probe proves a way).

**Each call is attested again.** Nothing is cached between calls but the
facts' lifetime: a session resumed or `/clear`ed while its MCP server
lives on is checked against the registry as it is at that call (the
resume row below), and a server that outlives its session (its parent gone: the
chain ends at `launchd`) is refused.

**The threat model, plainly (the review, 2026-11-15).**

- **What attestation stops:** honest but wrong callers: another tool's
  process, a stale record or a reused pid, a session of another profile
  or App, a process that outlived its session.
- **What it doesn't stop:** a malicious process running as the same
  user, in one narrow way after M1. Claude Code's registry folder
  (`<config>/sessions/`) and the app's session records are writable by
  any process of this user. A script can no longer get attested by
  writing an entry for its own parent: that parent's running code must
  be Anthropic's Claude Code, under the Claude app's own running main
  process for this instance (4b). What remains is a same-user process
  rewriting a live Code-tab session's registry entry or App record to
  name another conversation of the same App instance: that session (or
  something it runs) would then act as that other conversation of the
  same profile.
- **Why that's no new exposure:** ADR-0020's key has the same limit: any
  process of the same user can read a launched session's environment
  (`KERN_PROCARGS2`), key included.
- **What the boundary is:** the OS user. The socket is `0600` in a `0700`
  folder of the user's, and every connection's peer user is checked
  (`getpeereid`) before anything is read. Within that boundary,
  attestation gives accountability: each call is tied to the one session
  the system and the tool's own records name, and audited as it.

**Settled with the review's questions (the coordinating session,
2026-11-15).**

- **The peer is named by its audit token (L4, corrected after the
  review).** An earlier draft took tokio's `LOCAL_PEEREPID` and argued
  it was safe because it differs from `LOCAL_PEERPID` only for a
  delegated socket. That was the wrong question: either value is just a
  pid, and a pid can be reused once that process exits. The token
  (`LOCAL_PEERTOKEN`) names the socket's last user at `getsockopt` time,
  read right at accept, and carries the pid's version as well, and the app compares it with the
  live process's version (one audited `proc_pidinfo` block in
  `juggler-proc-list`, under D-110's rules): a process that took the pid
  since has another version and is refused. Reading the token needs no
  `unsafe` (nix's `getsockopt`). The chain, the start times along it, the
  registry entry's `procStart`, the running code's signatures (M1) and
  the App's own record still have to agree after that.
- **No tool allow rule in the user's `settings.json` in 0.4.6.** Add
  writes only AI Juggler's own folder (`skills/ai-juggler/`), so Remove
  stays exact. Claude Code asks once per tool in the Code tab; the window
  says so and suggests "Always allow". What a member may do is still the
  pool's level.
- **The Codex app's connector isn't offered.** Its writer (`[mcp_servers.juggler]`
  in the App's `config.toml`) and its tests stay, but `connector_status`
  and `connector_preview` say `unavailable` ("Codex can't yet tell which
  thread is calling", from its adapter's `DesktopPeer::not_offered`) and
  `connector_add` refuses, until lab probe C2 proves which thread calls.

### STRIDE, added

| Threat | Mitigation | Test |
|---|---|---|
| **S** another user's process | peer uid = ours, before anything is read | `local_ipc` (as before) |
| **S** a same-user process outside the session's tree claims a session | no field claims anything; the chain comes from the kernel; its parent/grandparent must be a Code-tab process with a matching registry entry and App record | `a_process_outside_the_session_is_refused` |
| **S** pid reuse: the session ended and its pid now runs something else | `procStart` must match that pid's real start time (±1 s); each process in the chain started no later than its child; the peer started no later than the accept | `a_reused_pid_is_refused` |
| **S** a process that outlives its session (an orphaned MCP server) | its parent is `launchd`: no registry entry; refused | `an_orphan_is_refused` |
| **S** resume and `/clear` (the MCP server outlives a conversation) | the registry and App record are read at every call; the call acts as the conversation they name now, or is refused when they disagree | `the_registry_is_read_at_every_call` |
| **S** a descendant of the session (a hook, a shell command the model runs) | accepted, as that session: it is inside the session's own trust (the model can call the tools itself); at most two steps up | `a_hook_under_sh_is_attested` |
| **S** a same-user process that **forges** a registry entry for its own parent (M1) | the parent's **running** code must meet Claude Code's pinned requirement (identifier, Apple's anchor, Anthropic's team), under the Claude app's own running main process (its pinned bundle identifier and team) for this instance, directly or through its pinned `disclaimer` helper (another Anthropic-signed program isn't a hop); pid versions equal before and after each check, start times and parents unchanged after them; an archived conversation isn't attested; the files are read through the no-link walk, owner and mode checked on the open file. What remains is rewriting a live session's entry or record to name another conversation of the same App (as §4: a same-user process can already read a launched session's key) | `this_processs_own_chain_…` (a forged entry for the test's runner refused), `a_tool_whose_running_code_isnt_genuine_is_refused`, `the_running_code_is_checked_against_the_requirement`, `an_archived_session_isnt_attested` |
| **S** a fake host: a program at `~/x/Claude.app/Contents/MacOS/Claude`, even signed with the app's identifier (M1 round 2, probe a) | the host is known by its running code's signature only (identifier **and** Anthropic's team under Apple's anchor), never a path or a folder name | `a_program_named_like_the_host_isnt_the_host`, `the_host_is_known_by_its_running_code_never_its_name` |
| **S/T** a genuine program renamed over the path of a running ad-hoc one (M1 round 2, probe b) | the running code is checked (`SecCodeCopyGuestWithAttributes` by pid, `SecCodeCheckValidity` against the pinned requirement), never the file at `proc_pidpath`; verdicts kept per (pid, pid version) | `a_genuine_program_renamed_over_a_running_one_changes_nothing` |
| **S** a host started with its profile folder given twice or in another form, so AI Juggler and the app read different instances (the third review's Low 1) | only `--user-data-dir=<folder>`, once, before `--`, is read; anything else is refused; and the verified main process must hold that instance's own lock (`F_GETLK` on the adapter's lock file, no lock taken) | `the_profile_folder_is_read_only_in_its_one_strict_form`, `the_profile_must_be_held_by_the_app_that_runs_the_session`, `a_write_locks_holder_is_named_and_asking_locks_nothing`; live, read-only: `the_running_claude_apps_profiles_pass_the_instance_checks` |
| **S** a process that exits or execs while it's checked | its pid version read before and after each check; start time and parent read again after all of them | `a_process_that_changed_while_it_was_checked_is_refused` |
| **E** the genuine Claude app made to run someone else's code (Node mode, `NODE_OPTIONS`, `--inspect`) | rests on its Electron fuses being off and the hardened runtime (4b); the fuse wire is checked on every Mac with the app | `the_installed_claude_apps_fuses_stay_off` |
| **T** the connector's files changed to point elsewhere | Add records each file's sha256; status says "changed by someone"; Remove deletes only what still matches | connector tests |
| **I** secrets | the registry's `*.key` files are never opened; no value of the session's environment is read by the app (D-37); the connector holds no secret | `the_key_files_are_never_read` |
| **D** call storms | the same limits: 30 calls in 10 s per conversation; before any attestation, 30 keyless lines in 10 s per process (a hook's report counts as its session's process above it, a new hook process each time) and 240 for every keyless line together | `a_looping_keyless_caller_is_limited_before_it_is_attested`, `a_hostile_nc_loop_is_capped_and_keyed_calls_still_answer` |
| **D** a flood of new processes starves members (the review's global-cap finding) | a process attested in the last 60 s (or held in an attested wait) isn't counted against the common cap, and its lines are charged to its session's one allowance (its attested tool's pid and version: the fifth finding), not one per process; a full table of attested processes drops its oldest, never all; past the cap the answer is `Busy`, its own reason, and the caller's own allowance isn't spent; the connector's waker tries a `Busy` again after its 5 s least pause, never backing off. **Accepted:** while a same-user flood runs, a member not attested in the last 60 s (a session just started, a connector restarted, one idle longer) is told `Busy` until the flood ends: a denial of service by a process of the same user, inside the OS-user boundary (§ threat model), never a way in | `members_still_wake_promptly_during_and_right_after_a_flood`, `a_flood_fills_the_common_cap_but_never_a_members_lines_or_a_newcomers_allowance`, `busy_is_tried_again_after_the_least_pause_and_other_refusals_back_off`, `an_attested_sessions_processes_share_one_allowance`, `a_full_table_of_attested_processes_drops_the_oldest_only` |
| **E** elevation through the attested path | the same `allowed()`, levels and talk rules; a desktop member can do nothing a keyed member can't | `attested_calls_go_through_the_same_decision` |

### Waking (D-119)

An idle desktop Code-tab member is woken only when the pool's waking
rule allows it (the pool at Full, or a rule the user made), with the same
gate and wake budget as a terminal member. The connector, a child of that
session, posts one facts-only line ("AI Juggler: 1 new pool message from
Planner; read it with read_inbox") to the session's own inbox socket
(`CLAUDE_CODE_MESSAGING_SOCKET`, with the documented auth line on macOS).
The app never opens that socket and never reads the token: the bridge
does, from its own environment, on the app's say-so. Claude Code treats
it as a peer message (it can't approve or change anything) and applies
its own rate limit on top of ours.

- **How the connector hears of a wake.** Its MCP server holds one wait
  open (`wire::Wait`, keyless, at most 110 s, in a slot of its own: 32 at
  most, never one of the calls' 16). The app attests it, then looks every
  5 s for an unread agent's message in an open thread of a pool neither
  paused nor stopped that `allowed(sender, pool, deliver_to_session, hop)`
  lets it deliver (the pool at Full, or a rule; the hop limit), at most one
  wake a minute per session, and answers with the facts-only note
  (`messaging::unread_note`). A person's message waits for the turn,
  unless they Deliver it (their compose with Deliver, or Deliver on the
  waiting copy): their click is the permission, and it wakes the member
  (the A9 lab walk, 2026-11-15). What counts as new is what came since the
  session's Claude Code started, not since the first answered wait, and the
  connector asks again every 5 s while it isn't attested yet in its first
  two minutes (the start race), backing off to at most a minute after that.
- **Which variables the server has (checked statically, 2026-11-15; lab
  A9 proves it live).** The bundled Claude Code 2.1.325 sets
  `process.env.CLAUDE_CODE_MESSAGING_SOCKET` when it binds the inbox, so
  children it starts after that inherit it; it keeps the token in a
  separate child environment (`….set("CLAUDE_CODE_MESSAGING_TOKEN", …)`),
  documented for hooks and Bash. So the server may have the socket and
  not the token. The connector posts with the auth line when it has the
  token and without it otherwise (the docs: on macOS an own child is
  verified while the posting process runs, which a long-lived MCP server
  is). With no socket in its environment it wakes nothing and the member
  reads on its next turn (its hooks' note). No token is ever passed
  through a file or to AI Juggler: the hook route the research named
  would need that, and isn't built.

## Unverified until the spike (AJ-E1701, the maintainer's)

- That a server from `--mcp-config` loads in Claude Code without an
  approval prompt, and that `--allowedTools` pre-allows its tools.
- That Codex's `-c mcp_servers.*` with `env_vars` passes the key, and that
  `tools.<tool>.approval_mode="approve"` pre-allows each tool.
- Which MCP revision each tool sends (both lines are answered).

## Alternatives considered

- **An MCP SDK** (`rmcp`): more code than the bridge uses, a new
  dependency tree to audit; rejected for now.
- **An HTTP server in the app** (MCP's streamable HTTP): a listener any
  local process can reach, `Origin` checks, a bearer token; C7 prefers
  stdio and a Unix socket.
- **The key in argv or in the `--mcp-config` file**: visible to every
  process (`ps`) or on disk; rejected.
- **Reading the key back from the session's environment**: D-37 forbids
  reading another process's environment.
