# Switch set-up: items an account brings, and guiding the user to them

Research date: 2026-10-27. Every source was accessed on 2026-10-27.
Labels follow `docs/research/EVIDENCE-STANDARD.md`: **VP** (the vendor's
own docs or binary say it; "VP (binary)" for strings read from the
installed CLI, which are the vendor's code but undocumented and may change),
**VS**, **NE** (not found by this search), **I** (our inference), **U**.

**Why:** the maintainer switched a Claude Code session between two account
profiles. The confirm step listed five skills the target lacked
(`built-in-browser`, `chrome-browser`, `computer-use`, `deep-research`,
`setup-writing-style`), each "Can't be set up: it comes with the claude.ai
account". The maintainer: "can we setup those things for them??? and if we
can't we need to give them instructions to do it … guide the user, not just
dump info." This page prepares step 3, Fix 1 (set up what we may, guide
the rest) and Fix 2 (say what this conversation actually used).

**What was run, and what wasn't:**
- Read-only, metadata only, in the maintainer's Claude profiles (the lab
  profile `bookauth-luka`; for the comparison, names and `source` values only
  from three other profiles of the same organization and one personal Max
  profile): folder listings,
  `skills/synced/<set>/manifest.json` (name, source, dates), and each
  `SKILL.md`'s frontmatter keys. Skill bodies were not read or copied.
- Read: the Anthropic license file that ships beside the proprietary skills
  (it is a license, not skill content).
- Strings read from the installed CLI binary
  (`~/.local/share/claude/versions/2.1.310`), and `~/.codex` folder names,
  `config.toml` `[plugins]` table headers and system-skill frontmatter.
- Nothing signed in, sent a prompt, wrote to a profile or opened a
  credential, `.claude.json`, `auth.json`, Keychain item or process
  environment. Ids are written as placeholders below.

---

## Answers in short

1. **claude.ai skills reach Claude Code by a one-way sync** into
   `<config dir>/skills/synced/<org>_<account>/<skill>/` at session start,
   then about every 10 minutes in use (40 idle). Turned-off skills move to
   `skills/.trash/`. (VP)
2. **The origin is in the files.** Each synced set has a `manifest.json`
   whose rows carry `source`: `anthropic` (always synced: pdf, docx, xlsx,
   pptx), `anthropic-example` (Anthropic-provided, follows the user's
   toggles), or `custom` / anything else (the user's or the organization's).
   Proprietary ones also carry `license: Proprietary…` frontmatter and an
   Anthropic `LICENSE.txt`. (VP (binary) + local observation)
3. **All five skills are Anthropic-provided** (`source: anthropic-example`).
   Four of them were once synced to the four profiles of one organization
   (the lab profile among them) and were moved to `.trash` on 2026-10-08
   within 20 minutes of each other; the fifth never reached them. Only the
   personal Max profile has all five.
4. **Copying an Anthropic-provided skill's files to another account is not
   acceptable**: the license beside them forbids "Reproduce or copy these
   materials, except for temporary copies…". AI Juggler guides the user to
   turn it on for the target account. The user's own skills may be copied
   with their consent.
5. **"Used in this conversation" is readable** from Claude Code and Codex
   transcripts (Skill tool calls, `<command-name>` tags, `mcp__…` tool
   names, Task/Agent and `spawn_agent`). No parser extracts it today.

---

## 1. claude.ai skills that sync into Claude Code

### How they arrive

| Claim | Source | Label |
|---|---|---|
| "Claude Code downloads your account's skills into `~/.claude/skills/synced/`" when a session starts; under `CLAUDE_CONFIG_DIR` every `~/.claude` path lives in that directory | https://code.claude.com/docs/en/skills.md (Skills synced from claude.ai); https://code.claude.com/docs/en/claude-directory.md | VP |
| Sync runs in the background at session start ("never delays startup"), then about every 10 minutes while in use and every 40 minutes idle; added, edited or turned-off skills apply without a restart | skills.md; CHANGELOG 2.1.293 | VP |
| "Some of Anthropic's skills, such as `pdf` and `xlsx`, always sync"; the rest follow what is on in the account's skills settings | skills.md | VP |
| One way: "Claude Code downloads synced skills and never uploads them" | skills.md | VP |
| Needs a claude.ai sign-in from `/login` (not an API key, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN` or `apiKeyHelper`); not on Bedrock, `--bare` or `--safe-mode`; `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` blocks it; v2.1.273+ (CHANGELOG says 2.1.275 added it) | skills.md; https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md | VP |
| Off switch `syncClaudeAiSkills: false`; an organization policy key `allow_account_skills_sync`; a server-side flag decides whether the feature is on ("setting true does not turn it on early") | skills.md; settings description in the binary | VP / VP (binary) |
| Turned-off or vanished skills move to `skills/.trash/` and are deleted after `cleanupPeriodDays` | settings description in the binary | VP (binary) |
| Synced skills run as `/<name>` or `/anthropic-skills:<name>`; `/skills` lists them under "claude.ai sync" | skills.md; binary | VP |
| The list comes from `GET /api/oauth/organizations/:orgUUID/skills/list-skills` (per entrypoint) | binary | VP (binary) |
| Plugins turned on in claude.ai (Customize > Plugins) sync as `<name>@synced` into `plugins/synced/`, once per start; changes need a restart or `/reload-plugins`; "If your organization turns off Skills on claude.ai, plugins stop syncing too" | https://code.claude.com/docs/en/plugins/loading.md#synced-plugins; https://claude.com/docs/plugins/overview | VP |
| Stale page: the platform Agent Skills overview still says custom skills "do not sync across surfaces". The newer code.claude.com and claude.com pages contradict it; don't quote it | https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview | VP (stale) |

### When a profile syncs (local observation, lab profiles)

- The synced folder is `skills/synced/<uuid>_<uuid>/`, with an empty
  `.bucket-<same>` marker beside it. The binary parses the name as
  `{org, account}` (`"<org>_<account>"`, or `<org>_unbound`). The four
  profiles of one organization share the first UUID and differ in the
  second; the personal profile has its own. (VP (binary) + local)
- `.last-complete-round` holds a round id and the entrypoints that completed
  it (`sdk-cli`, `cli`, `claude-desktop`): the CLI and the desktop app's
  Code tab both sync into the same folder. (local; meaning I)
- `manifest.json` `lastUpdated` moves with each round (lab profile:
  2026-10-25; personal profile: 2026-10-27). So a profile syncs only while one of its
  sessions runs. **A target account that hasn't run a session since a skill
  was turned on doesn't have it yet**: the guidance must end with "then
  start (or restart) a session on that account". (I, from VP timing)

### Telling Anthropic-provided from the user's own

| Signal | What it says | Label |
|---|---|---|
| `manifest.json` row `source` | `anthropic` = Anthropic "public" skill, always synced; `anthropic-example` = Anthropic-provided, provisioned for the account, follows the user's toggles; missing → written as `custom`; `session-refs` for skills a cloud session referenced | VP (binary: `te={public:"anthropic",examples:"anthropic-example"}`, `source:e.source??"custom"`) + local |
| Server row `creator_type` / `is_shared` / `partition_by` | The binary maps them to `anthropic`, `shared`, `org`, `own`, `other`. **These are not written to `manifest.json`**, so locally only `source` tells; an organization's or shared skill shows as `custom` (or its server `source`) | VP (binary) |
| `skillId` equals the folder name | true for every Anthropic row seen; the binary requires `id == name` before treating a row as Anthropic's | VP (binary) + local |
| Frontmatter `license: Proprietary. LICENSE.txt has complete terms` and a `LICENSE.txt` starting "© 2025 Anthropic, PBC. All rights reserved." | Anthropic proprietary skill (docx, pdf, pptx, xlsx, built-in-browser, chrome-browser, computer-use) | local |
| Frontmatter `compatibility:` naming Cowork / Claude in Chrome / Computer use | the skill needs that product feature | local |
| The help centre's own grouping | claude.ai lists "Created by you", "From your organization", "Shared with you", "From Anthropic & Partners" | https://claude.com/docs/skills/overview (VP) |
| Name reserved | skill names may not contain "anthropic" or "claude"; the `anthropic-skills:` namespace is reserved | platform overview; skills.md (VP) |

**Rule for the adapter:** read `manifest.json` per synced set. `source`
starting `anthropic` → **provided** (by Anthropic). `custom` → **own or
organization's, unknown which**: say "from your claude.ai account" and
offer both the upload steps and, with consent, a copy (it is the user's own
content when they made it; see §5). A synced folder with no manifest row
→ **unknown**.

### The five skills

Evidence: `manifest.json` rows and frontmatter, read 2026-10-27.

| Skill | Where it is | `source` | License | `compatibility` (frontmatter) | The feature it rides on | Provided? |
|---|---|---|---|---|---|---|
| built-in-browser | personal profile synced; the organization's four profiles `.trash` (2026-10-08) | anthropic-example | Proprietary (Anthropic LICENSE.txt) | Cowork in the desktop app; Cowork Remote | Cowork's built-in browser | **Provided** |
| chrome-browser | same | anthropic-example | Proprietary | Claude Code, Cowork, when the Claude in Chrome extension is connected | Claude in Chrome | **Provided** |
| computer-use | same | anthropic-example | Proprietary | Cowork … with Computer use turned on (macOS or Windows) | Computer use | **Provided** |
| deep-research | same | anthropic-example | none in frontmatter, no LICENSE.txt | — | Research (Claude Code also bundles a `/deep-research` workflow) | **Provided** |
| setup-writing-style | personal profile synced only | anthropic-example | none, no LICENSE.txt | — | Styles (I, weakest match) | **Provided** |

None of the five is named in Anthropic's public docs, help centre or the
`anthropics/skills` repo (NE: help centre articles 12512180, 12512176,
13119606, 16607400, 12012173, 14128542, 11088861; claude.com/docs skills
and plugins overviews; code.claude.com skills, commands, plugins pages;
github.com/anthropics/skills; web search for each name). Their origin rests
on the manifest rows, which is first-hand.

**Why the organization's accounts lost four of them (I):** all four of its
profiles moved the same four to `.trash` between 12:22 and 12:41 local on
2026-10-08, and `computer-use` was updated server-side the same day. The
binary's trash reasons are `sync_off` (setting or the organization's
policy), `account_change` and `skill_gone` ("gone from this account's
list"). The pattern points at an organization-level or Anthropic-side
change, not at a per-account toggle. Help-centre plan rules fit: computer
use is "in beta for Pro and Max plans" with no Team access, and the built-in
browser is on for Team "unless an owner has turned it off". So the guide
must say when the target's plan or organization decides, not only "turn it
on".

### Exact steps to turn a skill on for an account

All from the help centre and claude.com docs, accessed 2026-10-27.

**Anthropic skill, any plan with skills** (Free, Pro, Max, Team, Enterprise
per https://support.claude.com/en/articles/12512180-use-skills-in-claude;
claude.com/docs lists Pro, Max, Team, Enterprise):
1. Sign in to claude.ai as the target account.
2. Settings > Capabilities (https://claude.ai/settings/capabilities): turn
   on **Code execution and file creation** (newer layout: **Create and edit
   files**). Skills need it.
3. Customize > Skills (https://claude.ai/customize/skills): find the skill
   under **From Anthropic & Partners** (or the Discover tab) and flip its
   switch (**Turn on**). The help centre links both URLs (VP, fetched
   2026-10-27).
4. Start, or restart, a Claude Code session on that account. The skill
   arrives at the start, or within about 10 minutes in a running session.

**Team and Enterprise** (https://support.claude.com/en/articles/13119606-managing-skills-as-an-admin):
an Owner turns on **Cloud code execution and file creation** and
**Skills** under Organization settings > Plugins & skills > Policy
(https://claude.ai/admin-settings/skills; the claude.com page says
Capabilities). Team has it on by default.
Organization-provisioned skills are on for everyone; members can turn one
off. Anthropic skills stay available when creating your own is off.

**Feature-gated skills:**

| Skill | Also needs | Plans | Source |
|---|---|---|---|
| chrome-browser | the Claude in Chrome extension installed and connected | Pro, Max, Team, Enterprise ("available on paid plans"); on by default for Team, off for Enterprise | https://support.claude.com/en/articles/12012173-get-started-with-claude-in-chrome |
| built-in-browser | Cowork in the Claude desktop app; Settings > Cowork > Preferred browser | Pro, Max; Team and Enterprise "unless an owner has turned it off" | https://support.claude.com/en/articles/16607400-use-the-built-in-browser-in-claude-cowork |
| computer-use | desktop app Settings > Capabilities > **Enable computer use** (or Settings > System); in the CLI it is a built-in MCP server enabled in `/mcp`, macOS only | "in beta for Pro and Max plans"; not Team or Enterprise | https://support.claude.com/en/articles/14128542-let-claude-use-your-computer-in-cowork; https://code.claude.com/docs/en/computer-use.md |
| deep-research | web search on | paid plans (Pro, Max, Team, Enterprise) | https://support.claude.com/en/articles/11088861-use-research-on-claude |
| setup-writing-style | — | — | NE |

**A skill the user made** (Customize > Skills > **+** > **Create skill** >
**Upload a skill**, a ZIP of the folder; frontmatter limited to `name`,
`description`, `license`, `compatibility`, `metadata`, `allowed-tools`, or
the upload fails): https://support.claude.com/en/articles/12512180-use-skills-in-claude;
skills.md. (VP)

**A synced plugin** (`<name>@synced`): Customize > Plugins > Discover >
Add, on the target account; then restart or `/reload-plugins`
(https://claude.com/docs/plugins/overview). (VP)

---

## 2. Local evidence (lab profiles, metadata only)

`bookauth-luka` (the lab profile), 2026-10-27:

```
skills/
  synced/
    .bucket-<org>_<account>            (empty marker)
    <org>_<account>/
      manifest.json  .last-complete-round
      docs/ docx/ google-workspace/ import-memory/ morning/
      pdf/ pptx/ skill-creator/ xlsx/       (each with SKILL.md)
  .trash/
    <ms>-<pid>-<rand>/computer-use/      SKILL.md LICENSE.txt
    <ms>-<pid>-<rand>/chrome-browser/    SKILL.md LICENSE.txt
    <ms>-<pid>-<rand>/deep-research/     SKILL.md references/
    <ms>-<pid>-<rand>/built-in-browser/  SKILL.md LICENSE.txt
plugins/
  synced/<org>_<account>/.marketplaces.json  .last-complete-round
  known_marketplaces.json  marketplaces/claude-plugins-official/
```

`manifest.json` shape: `{ lastUpdated: <ms>, skills: [{ skillId, name,
description, source, updatedAt }] }`. Rows in the lab profile:
`pdf, docx, pptx, xlsx` → `anthropic`; `docs, morning, import-memory,
google-workspace, skill-creator` → `anthropic-example`. `skillId == name`
for every row.

Frontmatter keys seen: `name`, `description`, and on some `license`
(`Proprietary. LICENSE.txt has complete terms`) and `compatibility`. No
`author` or `source` key in any frontmatter: the origin is only in the
manifest. `skill-creator` ships an Apache-2.0 `LICENSE.txt`.

`plugins/synced/<set>/.marketplaces.json`: `{ etag, rows: [{ name:
"anthropic-plugin-directory", display_name: "Anthropic Directory", scope:
"default", source: { source: "claudeai" }, id: "marketplace_…" }],
parserVersion }`.

Other profiles (names and sources only): the organization's three other
profiles match it (same four in `.trash`, dated 2026-10-08 12:22–12:41).
The personal Max profile (another organization) has all five of the
maintainer's list as `anthropic-example`, plus the nine above.

---

## 3. "Was it used in this conversation?" per tool

| Use | Claude Code JSONL (CLI and the desktop Code tab) | Codex rollout JSONL |
|---|---|---|
| Skill, model-invoked | assistant `tool_use` with `"name":"Skill"`, `input: {skill, args?}`; then an `attachment` line `type:"invoked_skills"`, `skills:[{name, path, content}]` (VP (binary): the Skill tool's schema; Claude Code's own `invoked_skills` reader) | no "skill used" record: an explicit `$name` pick injects a user message wrapped in `<skill>` (`<name>`, `<path>`); otherwise only a shell read of its `SKILL.md`. The `<skills_instructions>` catalog lists all skills and is not use (§4) |
| Skill or command, user-typed | user line `"content":"<command-name>/name</command-name>…"` (Claude Code's own scanner: `<command-name>\/([\w:-]+)<\/command-name>`); synced skills may appear as `anthropic-skills:<name>`, plugin ones as `<plugin>:<name>` | `/prompts:<name>` expands into the user message; no separate record (I) |
| MCP tool | `tool_use` named `mcp__<server>__<tool>`; a plugin's server is `mcp__plugin_<plugin>_<server>__<tool>` (VP (binary)) | `function_call` named `mcp__<server>__<tool>`; legacy mode also writes `event_msg` `mcp_tool_call_end` with `invocation {server, tool}` (§4) |
| Subagent | `tool_use` `"name":"Task"` or `"Agent"`, `input.subagent_type` | `function_call` `spawn_agent` (`namespace: multi_agent`), `arguments.agent_type` |

Shapes from the repo's fixtures (`crates/juggler-adapters/tests/fixtures/`):

```jsonc
// claude-code/session-subagents.jsonl:2 (shortened)
{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_explore",
  "name":"Task","input":{"description":"Map the parsers","prompt":"…","subagent_type":"Explore"}}]}}
// codex/rollout-subagents.jsonl:3
{"type":"response_item","payload":{"type":"function_call","name":"spawn_agent",
  "namespace":"multi_agent","arguments":"{\"message\":\"…\",\"agent_type\":\"explorer\"}","call_id":"call_a"}}
```

**No fixture holds a Skill call, an `mcp__` call or a `<command-name>`
line.** Unit tests have only inline strings
(`last_turns.rs:566`, `claude_conversations.rs:234`: `<command-name>/clear`;
`tool_summary.rs:326`: `mcp__github__create_issue`). New fixtures are
listed in the design below.

**Claude desktop app, Chat and Cowork:** conversations are not stored
locally in a readable form (manifest `claude-code/desktop` `chat-read`:
unavailable). The Code tab writes ordinary Claude Code JSONL, so the Claude
parser covers it.

**OpenCode, Kimi, Grok:** our adapters read no tool calls today
(`opencode_store.rs` never reads the `part` table; `kimi_sessions/wire.rs`
reads turn, interaction and usage records only; `grok_sessions.rs` reads
`summary.json`). See §4 for their formats; until a reader exists, `used`
is unknown for them, never "no".

### What each existing parser would need

| File | Today | Needs |
|---|---|---|
| `claude_transcript.rs` `tool_uses` (250), `extract_message` (112) | every `tool_use` becomes a summary string via `tool_summary::summarize` (`other => "used {name}"`) | a pass that returns the raw names: `Skill` → `input.skill`; `mcp__a__b` → server `a` (plugin form → plugin + server); `Task`/`Agent` → `subagent_type`; user text `<command-name>/x</command-name>` → command or skill `x` (strip `anthropic-skills:` / `<plugin>:`); `attachment.invoked_skills[].name` |
| `codex_transcript.rs` `RolloutParser::value` (125), `call_of` (263) | `function_call`/`custom_tool_call` → summary | the same names from `function_call` (MCP form per §4), `spawn_agent` → `agent_type`, and skill reads per §4 |
| `tool_summary.rs` | display only | unchanged; the new pass lives beside it so the activity view keeps its words |
| `last_turns.rs` (271), `claude_conversations.rs` (35) | drop `<`-tagged user lines | unchanged (they're display) |
| `codex_appserver/codec.rs` (186) | `mcpToolCall` → `"{server}.{tool}"` | the live path for Codex sessions AI Juggler runs; feed the same `used` set |
| `opencode_store.rs`, `kimi_sessions/wire.rs`, `grok_sessions.rs` | no tool calls | per §4; ship as unknown first |

---

## 4. Codex and the other tools

Docs moved: developers.openai.com/codex/* now redirects to
learn.chatgpt.com/docs/*. OpenAI's help-centre articles returned 403 to a
fetch, so the claims that rest on them are **VS** (search snippets only).

### Codex: account-provided versus local

| Item | Where | Origin | What AI Juggler can do | Source | Label |
|---|---|---|---|---|---|
| User skills | `~/.agents/skills/<n>/SKILL.md` (shared by every Codex home on the computer); `$CODEX_HOME/skills/<n>` (deprecated, still read); repo `.agents/skills` | own | copy (already built) | https://learn.chatgpt.com/docs/build-skills; `codex-rs/ext/skills/src/host_roots.rs` | VP |
| System skills | `$CODEX_HOME/skills/.system/` with `.codex-system-skills.marker` (here: imagegen, openai-docs, review-agent, skill-creator, skill-installer) | provided (OpenAI, shipped with Codex, Apache-2.0) | never offered: every Codex install has them | docs "Bundled with Codex by OpenAI"; `codex-rs/skills/src/lib.rs`; local listing | VP |
| Admin skills | `/etc/codex/skills` | organization | guide ("your administrator installs these") | build-skills | VP |
| Cloud/workspace skills | refreshed "at turn start and after auth changes", read through model tools in namespace `skills` | account | turn on in ChatGPT (Enterprise/Edu: off by default, admins enable by role) | `codex-rs/ext/skills/src/cloud_skill.rs`; help.openai.com/en/articles/20001066 | VP (source) / VS |
| Plugins | `[plugins."name@marketplace"] enabled`; cache `~/.codex/plugins/cache/<marketplace>/<plugin>/` | by marketplace (below) | install a local-marketplace plugin with consent; guide for the rest | https://learn.chatgpt.com/docs/plugins; config reference | VP |
| — `@openai-bundled` | ships with the ChatGPT desktop app (browser, chrome, computer-use, visualize, …) | provided | never offered; "comes with the Codex app" | local `config.toml` headers and cache | local |
| — `@openai-primary-runtime` | a local runtime folder (`~/.cache/codex-runtimes/codex-primary-runtime/…`: documents, pdf, spreadsheets, presentations) | provided | never offered | local | local |
| — `openai-curated-remote` cache / `@openai-curated` | the plugin directory "shared by ChatGPT and Codex" (gmail, github, google-drive, canva, `app-…` ids) | provided or account (connectors) | guide: `/plugins` in Codex, or the ChatGPT plugin directory; connecting the app is separate and "stays connected in ChatGPT" | plugins page; local cache names | VP / local |
| — `created-by-me-remote` cache | plugins the user made in their ChatGPT workspace | account (own) | guide; copying is pointless (the target's sync owns it) | local cache name | local; meaning I |
| Apps / connectors | tools of the `codex_apps` MCP server; `[apps.<id>] enabled`; listed only with ChatGPT sign-in, cached per account and workspace | account | guide: turn on in ChatGPT, or ask the workspace admin ("Disabled by admin") | config reference; `core/src/connectors.rs`; help-centre snippets | VP / VS |
| MCP servers | `[mcp_servers.<name>]` in `config.toml` (also project `.codex/config.toml` when trusted) | own | set up (already built); `auth = "chatgpt"` only for "trusted first-party ChatGPT servers" | https://learn.chatgpt.com/docs/extend/mcp | VP |
| Custom prompts | `~/.codex/prompts/*.md`, `/prompts:<name>`; deprecated, not removed | own | translate (already built) | learn.chatgpt.com/docs/custom-prompts | VP |
| Subagents | `~/.codex/agents/*.toml` | own | copy/translate (already built) | learn.chatgpt.com/docs/agent-configuration/subagents | VP |

**Plan and admin gating** (VS unless marked): Business: apps on by
default. Enterprise/Edu: new apps and plugins off by default, role-based;
"Use plugins" on, "Upload plugins" and "Create plugins with MCPs" off by
default (VP, the MCP page). Local use needs "Allow members to use Codex
locally" (VP, roles page). An API-key sign-in has "No cloud-based
features", and some curated plugins need ChatGPT sign-in (VP, pricing and
plugins pages). No per-plan matrix for skills, plugins or apps was found
(NE).

### Codex rollouts: what "used" looks like

From `codex-rs/rollout/src/policy.rs` (fetched 2026-10-27) and the
installed CLI 0.163.0's strings:

- **MCP:** a `response_item` `function_call` whose `name` is
  `mcp__<server>__<tool>` (128-byte cap, sha1 suffix on collision;
  `codex-mcp/src/tools.rs`); and, in legacy history mode, an `event_msg`
  `mcp_tool_call_end` with `invocation {server, tool, arguments}` plus
  optional `connector_id`, `app_name`, `plugin_id`. App calls have server
  `codex_apps`. `mcp_tool_call_begin` is never written. (VP (source),
  `McpToolCallEndEvent`/`invocation` in the binary)
- **Skill:** no "skill used" record. An explicit `$name` pick injects a
  user-role message wrapped in `<skill>` with `<name>` and `<path>`
  (`ext/skills/src/fragments.rs`; `<skill>` in the binary). The catalog is
  a developer message in `<skills_instructions>`, which lists every skill,
  so it does **not** mean use. Implicit use is seen only as a shell read of
  a `SKILL.md` path, or a `function_call` in namespace `skills` named
  `read`. The `codex.skill_invocation` event exists only in OpenTelemetry,
  not in the rollout. (VP (source and binary))
- **Custom prompt:** expanded into the user message; no record (NE).
- **Subagent:** `function_call` `spawn_agent` (namespace `multi_agent` /
  `multi_agent_v1`; v2 adds `send_message`, `followup_task`,
  `list_agents`), `arguments.agent_type`; each child has its own rollout
  with `source.subagent.thread_spawn` in its `session_meta`. Collab begin
  and end events are never written. (VP (source); repo fixture
  `rollout-subagents.jsonl`)
- **Plugin:** a mention `plugin://<name>@<marketplace>` in user input;
  `TurnContext.disabled_plugin_ids`. (VP (source))

So for Codex, `used` is exact for MCP servers, apps and subagents, and a
best effort for skills (explicit picks and `SKILL.md` reads).

### OpenCode, Kimi Code, Grok Build

| Tool | Skills | MCP tool names | Skill use recorded as | Subagent | Account-provided items |
|---|---|---|---|---|---|
| OpenCode (repo now `anomalyco/opencode`) | `.opencode/skills`, `~/.config/opencode/skills`, and also `.claude/skills`, `~/.claude/skills`, `.agents/skills`, `~/.agents/skills` (https://opencode.ai/docs/skills) | `<server>_<tool>` (single underscore: ambiguous to split; I) | a `part` row `type:"tool"`, `tool:"skill"`, `state.input.name` (`packages/schema/src/v1/session.ts`) | `SubtaskPart {agent}`; the `task` tool; child sessions carry `parentID` | none documented: all local (VP / NE) |
| Kimi Code CLI (Node, `~/.kimi-code`) | `~/.kimi-code/skills`, `~/.agents/skills`, project `.kimi-code/skills`, `.agents/skills` (kimi.com/code/docs) | `mcp__<server>__<tool>` | a `Skill` tool call (`skill`, `args`); `/skill:<name>` | `Agent` tool (`subagent_type`); `agents/<id>/wire.jsonl` | official and curated plugin catalogs, installed per user; no account sync documented |
| Grok Build | `./.grok/skills`, `~/.grok/skills`, plugins' `skills/`, `~/.agents/skills`; reads Claude Code's skills and MCP "with zero configuration" (docs.x.ai/build) | `<server>__<tool>` | `tool_call` updates in `updates.jsonl` with skill tools prefixed `skill:` (third-party write-up, VS) | `.grok/agents`; sub-sessions have a parent in `summary.json` | none documented (NE) |

Two consequences for the design:
- **`~/.agents/skills` is shared** by Codex, Kimi Code, OpenCode and Grok.
  A skill copied there for one appears in all of them; the offer's note
  should say so (Codex's already does).
- **OpenCode and Grok read Claude Code's own skills folders.** A Claude
  skill may already reach them without a copy; their inventories should
  count those as present (follow-up; today they list no skills).

`grok inspect --json` lists the rules, skills, plugins, hooks and MCP
servers Grok finds for a folder (VP, docs.x.ai): the sanctioned reader for
Grok's inventory, before parsing files.

**Terms (OpenAI):** the Terms of Use bar users from "Modify, copy, lease,
sell or distribute any of our Services" (mirror of the 2024-12-11 text;
openai.com returned 403, so VS). The Codex repo, including its system
skills, is Apache-2.0; curated skills carry their own `LICENSE.txt`. The
same rule applies as for Anthropic: provided items are never copied, own
ones are, with consent.

---

## 5. Terms: copy or guide?

**The maintainer's rule** (FINDINGS.md): AI Juggler uses vendors' SDKs and
APIs "to enhance their existing tools, we never replace their tools".

**The license that ships beside Anthropic's proprietary skills** (local
`LICENSE.txt`, identical bytes for built-in-browser, chrome-browser,
computer-use, docx, pdf, pptx, xlsx; same text at
https://raw.githubusercontent.com/anthropics/skills/main/skills/docx/LICENSE.txt):
use "is governed by your agreement with Anthropic"; users may not
"Reproduce or copy these materials, except for temporary copies created
automatically during authorized use of the Services", nor "Distribute,
sublicense, or transfer these materials to any third party". (VP)

**Consumer Terms** (https://www.anthropic.com/legal/consumer-terms): "You
may not share your Account login information" and "You also may not make
your Account available to anyone else." **Commercial Terms** D.4 (Team,
Enterprise): customers may not "reverse engineer or duplicate the
Services". **Claude Code legal page**
(https://code.claude.com/docs/en/legal-and-compliance.md): developers "may
not collect, store, or intermediate Claude.ai credentials or session
tokens". (VP)

**Reading (I, not legal advice):**
- **Anthropic-provided skills: guide, never copy.** A copy of a synced
  skill from account A's folder into B's `skills/` is a reproduction
  outside "temporary copies created automatically during authorized use",
  and it would hand B something B's plan or organization may not grant
  (computer use isn't offered on Team at all). It would also replace
  Anthropic's own sync (which updates and withdraws the skill) with our
  copy: replacing, not enhancing. `deep-research` and `setup-writing-style`
  carry no license file, but they're Anthropic's materials under the same
  agreement, and the same reasoning about the sync applies. Treat all
  `source: anthropic*` rows alike.
- **The user's own skills: copy with consent.** A skill the user wrote is
  their input ("Inputs", Consumer Terms §4). Copying it into B's local
  `skills/` is fine once they tick it; uploading it on claude.ai for B is
  the alternative that also reaches B's Chat and Cowork.
- **Organization or shared skills (`custom` that the user didn't make):**
  we can't tell them apart locally (§1). Offer the copy only with a note
  that it is for the user's own skills, and lead with the claude.ai steps.
- **Apache-2.0 Anthropic skills** (`skill-creator`): the license allows a
  copy, but it is still Anthropic's synced skill; guiding keeps it updated.
  Same treatment as other provided skills.

---

## Design proposal (step 3, Fix 1 and Fix 2)

### Item classes

| Class (`origin`) | Examples | What AI Juggler does | Verdict on the wire |
|---|---|---|---|
| **own** | `skills/<name>/` files; a Codex `~/.agents/skills` skill; a command, subagent, local MCP server | set it up (copy or translate), as today, with consent | `copy`, `translate`, `needs-you` |
| **account** (own content synced by the account, origin not provable) | claude.ai `source: custom`; Codex `created-by-me-remote` plugins | lead with the account steps (upload or turn on for the target); offer a copy as the second choice, labelled "if you made it" | `turn-on` with `can_copy: true` |
| **provided** | Anthropic `anthropic` / `anthropic-example` skills; OpenAI `openai-bundled`, `openai-primary-runtime`, `openai-curated-remote` plugins; `.system` skills | steps to turn it on for the target account, a button to open the page, "then start a session on <account>" | `turn-on` |
| **plan-gated** (provided, and the feature depends on the plan, an admin or another product) | computer-use, built-in-browser, chrome-browser, deep-research | the same steps, plus a plain line: "Needs a plan that includes computer use (Pro or Max)", "Your organization's owner may have to allow it", "Needs the Claude in Chrome extension" | `turn-on` with `needs[]` |
| **unknown** | a synced folder with no manifest row; a tool whose files say nothing | "From the account; we can't tell where it came from" and the generic account steps | `turn-on` (generic) |

Cross-tool (Claude → Codex and so on): a provided item can never move. The
target adapter may name its own counterpart instead (Codex has a Computer
Use and a Chrome plugin of its own). That needs a shared capability tag
and is a follow-up, not part of this step.

### Shared types (`crates/juggler-adapter/src/inventory.rs`)

```rust
/// Where an item came from, as the source tool's files say.
pub enum Origin {
    /// The user's own files on this computer.
    Own,
    /// Synced from the tool's account, origin not provable locally
    /// (claude.ai `custom`): the user's, their organization's or shared.
    Account,
    /// Provided by the vendor with the account or the app.
    Provided { by: String },          // "Anthropic", "OpenAI"
    Unknown,
}

/// How the user gets an item the target can't be given by copying.
pub struct GetIt {
    /// Short imperative steps, in order, in the app's words.
    pub steps: Vec<String>,
    /// The page that does it (https only; opened by the shell seam).
    pub open_url: Option<String>,
    /// What has to be true first, each in a sentence.
    pub needs: Vec<Need>,
    /// What happens after ("starts working in the next session on <account>").
    pub then: Option<String>,
}

pub enum Need {
    Plan(String),         // "a Pro or Max plan"
    Admin(String),        // "your organization's owner to allow Skills"
    Feature(String),      // "the Claude in Chrome extension, connected"
}

// Item gains:
pub origin: Origin,

// Detail::FromTheAccount gains what the reader knows:
FromTheAccount { source: Option<String>, files: Option<PathBuf>, requires: Vec<String> }
//   source  : the manifest's `source` value, kept as text
//   files   : the synced folder (only for origin Account: an own copy needs it)
//   requires: frontmatter `compatibility`, for the plan-gated wording

// SetUp gains one verdict; Impossible stays for real impossibilities:
TurnOn { get_it: GetIt, can_copy: bool },
```

`HarnessAdapter` (`crates/juggler-adapter/src/adapter.rs`) gains, each
with a default that keeps every other adapter compiling and honest:

```rust
/// What `transcript` used: skills, commands, MCP servers, subagents,
/// plugins, by kind and name. `None` when this tool's transcripts don't say.
fn used_in(&self, transcript: &Path) -> Option<Vec<(ItemKind, String)>> { None }
```

The verdict text and steps stay in each adapter's `can_set_up`: the
target adapter knows its own account pages (Claude's `can_set_up` gives
claude.ai steps when `item.tool` is its own; Codex's says "Codex can't have
a claude.ai skill" and, later, its counterpart). `switch_setup` keeps
comparing by `item.tool == target.harness`, with no tool literal.

### Wire fields (`OfferInfo`, ts-rs generated)

```ts
used: boolean | null;          // null: this tool's transcripts don't say
origin: "own" | "account" | "provided" | "unknown";
provided_by: string | null;    // "Anthropic"
get_it: { steps: string[]; open_url: string | null; needs: string[]; then: string | null } | null;
can_copy: boolean;             // turn-on items the user may still copy (own content)
```

`verdict` gains `"turn-on"`. `suggested` becomes: copy/translate **and**
(`used == true` or `used == null`). Sort: used first, then can-set-up, then
turn-on, then the rest.

### UI (progressive disclosure)

- The list shows **what this conversation used** first. Items not used sit
  under one collapsed row: "5 more the other account has, not used here".
  The maintainer's five would land there (none was used in that session),
  so the dialog stops dumping them.
- A `turn-on` row reads, e.g., "computer-use — turn it on for Personal"
  with **Show me how** (expands the numbered steps and the needs) and
  **Open claude.ai** (`open_url`, through the existing open-URL seam).
  Needs render as one quiet line ("Needs Pro or Max").
- After the switch, the set-ups card keeps a "To turn on" reminder until
  the next inventory read finds the item on the target (it syncs on the
  target's next session).
- Reuse `SwitchSetupList` rows, marks and the detail level; no new
  pattern (ui-consistency). Plain words lint applies to every step string.

### Per-adapter "how to get it" text

| Adapter | Origin rule | `get_it` |
|---|---|---|
| Claude Code (`claude_inventory.rs`) | synced row `source` starts `anthropic` → Provided{Anthropic}; `custom` or other → Account; no row → Unknown; `skills/<name>` → Own; `@synced` plugin → Account (claude.ai plugin directory) | Skills: "Sign in to claude.ai as <account>" · "In Settings > Capabilities, turn on Create and edit files" · "In Customize > Skills, find <name> under From Anthropic & Partners and turn it on" · then "Start a session on <account>; it arrives within about 10 minutes". `open_url: https://claude.ai/customize/skills`. Needs from a small table keyed by name and `compatibility` (computer use: Pro or Max; built-in browser: Cowork in the desktop app, owner may turn it off on Team/Enterprise; Chrome: the extension, paid plan; Research: paid plan). Team/Enterprise: "Your organization's owner may need to allow Skills". Plugins: "Customize > Plugins > Discover > Add", then `/reload-plugins`. Account skills: "Upload it: Customize > Skills > + > Create skill > Upload a skill (a ZIP of the folder)", `can_copy: true` |
| Codex (`codex_inventory.rs`) | `.system` skills → Provided{OpenAI} (shipped with Codex; never offered); `[plugins."x@openai-bundled"]`, `@openai-primary-runtime` → Provided{OpenAI} ("comes with the Codex app", no steps); `@openai-curated` / `openai-curated-remote` → Provided{OpenAI}; `created-by-me-remote` → Account; `~/.agents/skills`, `skills/<n>` → Own | Curated plugin: "In Codex, type /plugins, find <name> and install it" · "If it connects an app, sign in to it when Codex asks"; Enterprise/Edu need "Your workspace admin may have to allow it". App/connector: "Turn it on in ChatGPT's apps settings for <account>". Then "Start a new Codex session on <account>". No `open_url` until a stable ChatGPT settings URL is verified (NE) |
| OpenCode, Kimi, Grok | everything they list is local → Own | none needed today; `used_in` → None until their readers exist |

Table entries that rest on a help-centre page carry its URL in a code
comment and in the manifest, and are re-checked with the manifest's
30-day cycle.

### Tests and fixtures

- **Fixtures** (synthetic, no personal text):
  - `claude-code/session-used.jsonl`: a `Skill` call (`{"skill":"pdf"}`),
    an `invoked_skills` attachment, a user `<command-name>/anthropic-skills:docs</command-name>`,
    a `<command-name>/review</command-name>` command, an
    `mcp__github__create_issue` call, an `mcp__plugin_notes_store__save`
    call, a `Task` with `subagent_type`.
  - `codex/rollout-used.jsonl`: an MCP call and a skill read in the shapes
    §4 records, plus `spawn_agent`.
  - A synced-skills tree under a temp dir: `skills/synced/<org>_<acct>/`
    with `manifest.json` rows for `anthropic`, `anthropic-example`,
    `custom`, and a folder with no row; a `.trash/` entry that must be
    ignored; a proprietary `compatibility:` frontmatter.
  - A Codex home with `.system/`, `[plugins."x@openai-bundled"]` and a
    `created-by-me-remote` cache folder.
- **Adapter tests:** origin per row; `used_in` returns exactly the
  expected (kind, name) set; `.trash` never read; skill bodies never read
  (only frontmatter keys); a malformed manifest gives Unknown, not a panic.
- **App tests (`switch_setup`):** `turn-on` offers carry `get_it`; used
  first; unused collapsed; `suggested` ignores unused items; a
  provided item is never applied even if its id is sent (refused, like
  `impossible` today); no tool literal (lint).
- **UI tests:** `switch-setup.test.tsx` replaces the "Can't be set up: it
  comes with the claude.ai account" expectation with the steps; the
  gallery shot for the dialog; a11y gate on the expanded steps.
- **Live (lab, maintainer-approved):** switch the lab profile to the
  personal profile and back in AI Juggler Dev; confirm the five read as provided, the steps open the
  right page, and nothing is written to either profile.

### Files to touch

| File | Change |
|---|---|
| `crates/juggler-adapter/src/inventory.rs` | `Origin`, `GetIt`, `Need`; `Item.origin`; `Detail::FromTheAccount` fields; `SetUp::TurnOn` |
| `crates/juggler-adapter/src/adapter.rs` | `used_in` with a `None` default |
| `crates/juggler-adapters/src/claude_inventory.rs` | read `manifest.json` per synced set (keys `skillId`, `name`, `source` only); frontmatter `compatibility`/`license`; origin; `can_set_up` → `TurnOn` with claude.ai steps |
| `crates/juggler-adapters/src/claude_used.rs` (new) | Claude `used_in` over JSONL (beside `claude_transcript.rs`) |
| `crates/juggler-adapters/src/codex_inventory.rs` | `.system`, marketplace suffix and remote-cache origins; `TurnOn` for OpenAI-provided plugins |
| `crates/juggler-adapters/src/codex_used.rs` (new) | Codex `used_in` over rollouts |
| `crates/juggler-adapters/src/{claude_code,codex}.rs` | wire `used_in` |
| `crates/juggler-adapters/src/{opencode,kimi,grok}_inventory.rs` | `origin: Own` for what they list; their `FromTheAccount` arm keeps `Impossible` ("it comes with the other tool's own account"), since only the item's own tool can be told to turn it on |
| `crates/juggler-app/src/features/switch_setup/mod.rs` | `OfferInfo` fields; call the source adapter's `used_in` on the session's transcript (`TranscriptRef`); sort and suggest by use; refuse to apply `turn-on` |
| `ui/src/types/generated/OfferInfo.ts` | regenerated |
| `ui/src/lib/ipc/switchSetup.ts` | `EXAMPLE_SETUP` with a `turn-on` row and `used` |
| `ui/src/components/SwitchSetupList.tsx` | collapsed "not used here" group; Show me how; Open claude.ai |
| `ui/src/components/SwitchSetUpsCard.tsx` | "To turn on" reminders |
| tests above; `docs/research/capabilities/manifest.json` | re-check dates |

Architecture checks: steps and URLs live only in adapters; `switch_setup`
compares tool ids from the registry, never literals; paths come from
`state_dir` and each adapter's `user_home()` seam; the URL opens through
the app's existing external-link seam, so nothing here is macOS-specific.
