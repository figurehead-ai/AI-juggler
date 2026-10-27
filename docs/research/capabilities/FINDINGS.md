# What the 2026-10-27 capabilities research asks of AI Juggler

The manifest (`manifest.json`, rendered in `README.md`) says what each tool
offers. This page says what to do about it, best first. Each item names
its evidence in the family's research notes beside this file. Terms come
first: where a vendor forbids automation on a plan, the tool is track-only
on that plan, whatever its protocol allows.

## How AI Juggler uses what a vendor offers

The maintainer, 2026-10-27: AI Juggler uses vendors' SDKs and APIs "to
enhance their existing tools, we never replace their tools … their
original tools are always running". The user's own, unmodified app or CLI
keeps running with the user's own sign-in; AI Juggler observes,
coordinates and hands work between them, and never proxies, resells or
stands in for a vendor's client. Every terms reading below starts from
that, and still quotes the exact clause and says plainly where wording
restricts even this (those cases wait for the maintainer's ruling).

And a tool keeping its data in its own database is a reason to find the
sanctioned reader, not to stop: for conversations, subagents, usage and
state, look in this order — the vendor's documented API, protocol or SDK;
its CLI readers (export, `--json` listings); its documented files; then a
read-only database read. The manifest records which one each feature uses.
For example, OpenCode's conversations: its 2.x local HTTP API
(`/api/session/…`, durable event log) and `opencode export` /
`session list --format json` before its SQLite store.

## Bugs and corrections in what is built

1. **OpenCode 2.x runs one shared background service per user on port
   49374.** Each managed account starts its own, so a managed launch fails
   while the user's own OpenCode (or its Desktop) runs, or blocks it. Give
   each managed account its own `service.json` port and password, or launch
   with `--standalone`, and own the service's lifecycle. (`opencode.md`)
   **Fixed 2026-10-27:** managed 2.x launches pass `--standalone`, whose
   server ends with the TUI; live check on AJ-E1612.
2. **OpenCode 2.x keeps MCP servers under `mcp.servers`.** Our inventory
   reads only the 1.x `mcp.<name>` shape, so a 2.x config shows none.
   (`opencode.md`) **Fixed 2026-10-27:** both forms read; a set-up writes
   the file's form, else the installed version's.
3. **Grok Build takes a first prompt as an argument** (`grok "…"`); the
   adapter's "can't be started with a first message" is wrong for it, which
   blocks switching into Grok. Check `--resume <id> "…"` live.
   (`kimi-grok.md`) **Fixed 2026-10-27:** Primed and resume-with-message
   launches (`-- "<message>"`); the source confirms the resume case, the
   binary is still to be checked live.
4. **Kimi Code writes a versioned event log** (`agents/main/wire.jsonl`,
   protocol 1.5): working, needs-you and idle, and exact tokens, with no
   model. The adapter reads only the session index today. (`kimi-grok.md`)
   **Fixed 2026-10-27:** open sessions from its session locks, their state
   from the main log, exact tokens from every agent's log; another protocol
   version stays unknown.
5. **Codex: `approval_policy = "untrusted"` is unsupported** (config
   reference). AI Juggler's sessions set no policy; lab harnesses must not
   use it. (`codex.md`)
6. **The switch set-up says "Can't be set up" for skills the target
   account can simply turn on.** claude.ai's synced skills carry their
   origin in `skills/synced/<org>_<account>/manifest.json` (`source`:
   `anthropic`, `anthropic-example`, else `custom`); Codex's in `.system`
   and the plugin's marketplace. Provided items must never be copied (the
   Anthropic license forbids it) but can be guided: steps, the page to
   open, the plan or admin they need. And the list should lead with what
   the conversation used, readable from Claude and Codex transcripts.
   Design: `switch-setup-items.md`.

## Better ways to do what AI Juggler already does

1. **Claude Code reports its own state to the terminal (OSC 7501:
   working, blocked, done)** and `claude agents --json` is "the supported
   way to read session state from outside". Both are more exact than pid
   files and hooks alone. (`claude-code.md`)
2. **Codex history through the protocol, not the files**: a read-only
   app-server per profile (`thread/list`, `thread/read`,
   `thread/turns/list`, never resuming). Paged and modelled by the vendor;
   the rollout format "isn't a stable interface". (`codex.md`)
3. **Kiro, Cursor, Gemini, Hermes, Grok, Kimi and Muse all speak a
   documented protocol** (ACP, or their own: Muse's MSP, Hermes' gateway,
   ZCode's app-server). New adapters start there, not with screen text.

## New things AI Juggler could do

1. **Answer a terminal Claude session's permission prompt from Needs you**
   through Claude Code's channel permission relay on AI Juggler's own MCP
   server. A research preview: allowlisted plugins only, or a development
   flag that asks every launch. Lab only until Anthropic allowlists it.
   (`claude-code.md`)
2. **Reach the Claude desktop app's Code tab** through a mod in AI
   Juggler's plugin (state, both limit windows, pool messages between
   turns). Needs the user's consent to install; never approves anything by
   itself (D-28). (`claude-code.md`)
3. **Push a message into a running session without typing**: Claude Code's
   per-session inbox socket, Muse's session messaging, Hermes' gateway,
   OpenCode's `prompt` with `delivery: steer|queue`. MCP itself can't push
   into a session in any of the twelve tools.

## Terms that decide what is allowed

- **Claude:** the user's own unmodified `claude` with their subscription is
  allowed; the Agent SDK with claude.ai sign-in is not. Driving the
  desktop chat through Accessibility is out. (`claude-code.md`)
- **Codex:** app-server authentication is allowed for local and
  open-source apps, never for commercial or hosted services. (`codex.md`)
- **Antigravity (consumer plans):** wrapping is "not supported" and third-
  party access is a breach: track-only. Work plans fall under Google Cloud
  terms, where no wrapper clause was found; the Pre-GA terms are unread.
- **Cursor:** its acceptable-use policy bans "automated or non-human
  means", with no carve-out for its own ACP: needs the maintainer's
  ruling before an adapter (AJ-E1609).
- **Kiro:** "ACP compatible IDEs" are allowed; "third-party automation
  harnesses (such as OpenClaw)" are not. Spawning the native `kiro-cli acp`
  is the defensible path; ask AWS in writing first.
- **Kimi (membership plans):** "for interactive use only"; nothing
  unattended on those accounts.
- **Gemini CLI:** consumer Google sign-ins stopped on 2026-06-18; Code
  Assist, paid API keys and Vertex remain.
- **ZCode:** bans "automated means" and is non-commercial unless on a paid
  plan: needs a ruling before an adapter (AJ-E1605).
- **Hermes:** one Nous Portal account per person; and Hermes borrows and
  refreshes Claude Code and Codex sign-ins by default
  (`auth.adopt_external_logins`), which an adapter must switch off before
  first launch. (`hermes-muse.md`)
