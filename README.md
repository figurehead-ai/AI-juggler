# AI Juggler

**See which AI coding sessions are open on your Mac and where, get to any of them in one step, and know how much room each account has left.**

Free and open source (MIT), local-first, for macOS. AI Juggler works with the
coding tools you already use. It never proxies their traffic, never signs in
for you, and never stores your credentials.

[![CI](https://github.com/figurehead-ai/AI-juggler/actions/workflows/ci.yml/badge.svg)](https://github.com/figurehead-ai/AI-juggler/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/figurehead-ai/AI-juggler?include_prereleases&label=release)](https://github.com/figurehead-ai/AI-juggler/releases)
[![macOS 14+](https://img.shields.io/badge/macOS-14%2B-informational)](#download)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

[Download](#download) · [Quick start](#quick-start) · [User guide](./docs/guide/) · [Privacy](#what-ai-juggler-reads-writes-and-sends) · [Discussions](https://github.com/figurehead-ai/AI-juggler/discussions)

## Why

If you pay for more than one AI coding subscription, say two Claude accounts
and Codex, you end up juggling by hand: several Claude app profiles, config
folders, shell aliases, terminal tabs, and guessing which window holds which
session and which account still has room. AI Juggler puts all of it in one
window:

- **What's open, where, and one step to it.** The list shows the sessions
  open right now in the Claude app, your terminals, the Codex app and
  Cursor, grouped by the app and profile each runs in, with what needs you
  on top. **Go to it** brings the right app and profile forward. Ended and
  inactive sessions are one click away, never in the way.
- **Profiles and accounts kept apart.** A profile is the setup an app runs
  on (a Claude Code folder, a Claude app profile, a Codex folder). An
  account is the subscription it's signed in to. Several profiles on one
  account are normal, and they share its limits.
- **Usage and limits you can trust.** A number is shown only when it was
  measured. Anything else says "Not measured yet", never a made-up 100%.
- **Your Mac at a glance.** How much memory and CPU your AI tools use, which
  window is heavy, and leftover processes you can clean up.
- **Profiles managed for you.** Use each setup where it already is, or as a
  separate copy, and start a session on the profile you choose. A new
  profile can start with your usual MCP servers and permissions.
- **Carry a conversation to another account or tool** when one runs out of
  room, and switch back later.
- **Find anything you said.** History searches every ended session by what
  it was about, and by the words said in it.
- **Your rules and MCP servers, the same everywhere.** See where
  `CLAUDE.md` and `AGENTS.md` differ, and bring them in line when you choose.

## Download

| App | Channel | Get it |
| --- | --- | --- |
| **AI Juggler** | Stable or Beta (you choose in Settings) | [Latest release](https://github.com/figurehead-ai/AI-juggler/releases) — `AI-Juggler-<version>-universal.dmg` |
| **AI Juggler Nightly** | Nightly (a separate app with its own data) | [Releases](https://github.com/figurehead-ai/AI-juggler/releases) — `AI-Juggler-Nightly-<version>-universal.dmg` |

**Requirements:** macOS 14 Sonoma or later, on Apple silicon or Intel (one
universal app). At least one of [Claude Code](https://www.npmjs.com/package/@anthropic-ai/claude-code)
or [Codex](https://github.com/openai/codex) installed and signed in.

Every build is signed and notarized by Apple. The first time you open it,
macOS asks whether you're sure; choose **Open**.

AI Juggler checks for updates at launch and offers them; it never installs
one without you. Settings › General › About & updates shows your version,
lets you switch between Stable and Beta, and can go back to an earlier
version.

## Quick start

1. Open AI Juggler and choose **Find my accounts**. It looks for your Claude
   Code and Codex setups (for example `~/.claude` and `~/.codex`). Each one
   is a profile.
2. For each setup, choose **Use it where it is** (nothing is copied) or
   **Make a separate copy** (AI Juggler keeps its own copy, and your original
   is never touched).
3. If a tool needs you to sign in, it opens that tool's own sign-in.
   AI Juggler never sees your password.
4. The list on the left shows what's open now, grouped by place: the app
   and profile each session runs in, such as "Claude app · Work". Press
   Enter on a session (**Go to it**) to bring its app forward. Ended and
   inactive sessions sit behind the "Also:" line under the list.
5. Start a new session with **New session**, and pick the profile it runs
   on.
6. When an account runs low, open the session and choose **Switch
   account…** to carry the conversation to another account or tool.

The [user guide](./docs/guide/) covers each step in more detail, starting
with [What's open, and getting to it](./docs/guide/whats-open.md).

## What works today

| | Status |
| --- | --- |
| What's open now in the Claude app, terminals, the Codex app and Cursor, grouped by app and profile, with Go to it | Beta: new in 0.1.x |
| Accounts as subscriptions, with the profiles signed in to each | Beta: new in 0.1.x |
| Every session and project, including ones started outside AI Juggler | Works |
| History of finished sessions, searchable by what was said in them (for sessions started outside AI Juggler, turn it on in Settings › Privacy) | Works |
| How many subagents a session is running, and what each was asked to do | Works |
| The terminal: Find, clickable links and paths, text size, Maximize, its own window | Works |
| Starting a session from a saved context pack, in either tool | Works |
| Usage and costs per session, account and project, from the tools' own records | Works |
| Codex limits, read from what Codex records | Works |
| About how many of your usual sessions are left in each window, and a burn-down chart | Works |
| Monthly caps for an account or a group of projects: a warning at 80 % and 100 %, never a stop | Works |
| Claude limits | Works for sessions started in AI Juggler; for other sessions, turn it on per profile (see [Privacy](#what-ai-juggler-reads-writes-and-sends)) |
| This Mac: memory and CPU of your AI tools, including the Claude app | Works |
| Cleaning up leftover processes (automatic cleanup is off unless you turn it on) | Works |
| Rename, remove and choose a default profile | Works |
| Coming from Claude Squad or Vibe Kanban: their worktrees stay with your projects, and the config folders their profiles use are offered as profiles | Works |
| A new profile starting with your usual MCP servers, permissions and status line | Works |
| Rules files and MCP servers compared across your tools, brought in line when you apply | Works |
| Restarting crashed sessions, keeping the Mac awake while they work, reopening after a crash | Works (each one is a setting) |
| Opening AI Juggler at login | Beta: new in 0.1, not yet checked on an installed build |
| macOS notifications when a session needs you (each kind off until you choose) | Beta: new in 0.1, not yet checked on an installed build |
| Backups, and going back to an earlier version | Works |
| Switching a conversation to another account or tool | Beta: checked live between Claude Code accounts, between Codex accounts, and between Claude Code and Codex, both ways; every pair still says "Not proven yet" |
| A switch prepared while an account runs low, so it's ready when the limit comes | Beta: new in 0.1 |
| Crash reports, scrubbed on your Mac (off unless you choose) | Kept on your Mac; this version can't send them anywhere yet |
| Fleet: your other Macs in one window: their sessions, starting a session on one, usage by machine | Beta: checked with two copies on one Mac, not yet across two Macs ([guide](./docs/guide/fleet.md)) |
| OpenCode, Kimi Code and Grok Build | Beta: built from their source and docs, not yet checked live |
| More tools (ZCode, Kiro and others) | Planned |
| Windows and Linux | Planned: Windows' first release comes before 1.0; Linux's timing is open |

## Supported tools

| Tool | Support |
| --- | --- |
| **Claude Code** (the `claude` command) | Sessions, usage, limits, starting sessions, switching (beta) |
| **Codex** (the `codex` command) | Sessions, usage, limits, starting sessions, switching (beta) |
| **OpenCode** (the `opencode` command, 1.x and 2.x) | Beta, not yet checked live: sessions, usage (tokens, and its own cost estimate), starting sessions, rules and MCP servers |
| **Grok Build** (the `grok` command) | Beta, not yet checked live: sessions, starting sessions (your Claude and Cursor setups stay out of it), rules and MCP servers; its usage isn't read yet |
| **Kimi Code** (the `kimi` command, 2.x) | Beta, not yet checked live: sessions, starting sessions (never by itself on a Kimi plan, which is for interactive use only), rules, skills and MCP servers; its usage isn't read yet |
| **Claude app** (desktop) | Its open sessions are shown under each of its profiles, with Go to it; This Mac measures its memory and CPU |
| **Codex app** (desktop) | Its open threads are shown, with Go to it |
| **Through another service**: OpenCode Go and Kimi, on Claude Code and Codex | Beta, not yet checked live: an account whose key you paste once (kept in the macOS Keychain), with a Claude Code or Codex profile pointed at the address the service publishes. Its usage is an estimate (OpenCode Go) or unknown (Kimi), and switching to it is never a proven one-click switch |

AI Juggler finds each tool through your shell's `PATH`, the way your terminal
does, even when you open the app from Finder.

## What AI Juggler reads, writes and sends

**Reads, on your Mac only:**

- Your tool setups: Claude Code's `~/.claude` (and other Claude folders you
  use) and Codex's `~/.codex`, to list conversations and count usage from
  their transcripts; OpenCode's `~/.local/share/opencode` database, read
  without changing it, and its settings in `~/.config/opencode`; Kimi
  Code's `~/.kimi-code` session list and settings; Grok Build's
  `~/.grok` sessions and settings.
- Account names and emails from those setups, so you can tell accounts
  apart and see which profiles share one.
- Which sessions each app has open, from the apps' own session files, and
  which app each running process belongs to (from its parent processes).
  Never a process's environment, key files or token caches.
- The last few messages of a session, from its transcript on this Mac, when
  you open its page (**Last messages**, on by default; turn it off in
  Settings › Desktop apps). They're kept nowhere.
- The list of running processes, to show live sessions and This Mac.
- If you use Claude Squad or Vibe Kanban: their own lists, read without
  changing them, for which repository each of their worktrees belongs to and
  which config folders their profiles use. Nothing else they keep.

It checks only whether a sign-in exists. It never reads the passwords,
tokens or API keys your tools keep, and never opens the tools' Keychain
items. The one key it holds is one you give it: for an account added
through another service, kept in AI Juggler's own Keychain item and handed
only to that account's sessions when they start.

**Writes:**

- Its own data folder, `~/Library/Application Support/AI Juggler`
  (`AI Juggler Nightly` for Nightly). Copies of setups, backups and settings
  live there, and the search index of sessions started outside AI Juggler
  if you turn that on (it holds the words to search, not the conversations,
  and turning it off deletes it).
- In your own setups and projects, only what you choose, each shown before
  it's made:
  - **Measuring Claude limits for sessions started elsewhere** (off by
    default, per profile). It shows the exact one-line change to that
    profile's `settings.json` before making it, keeps your own status line
    working, and turning it off restores the file exactly.
  - **Switching a conversation** into a profile adds the conversation's
    files to that profile, beside its own and never over them. The MCP
    servers, skills and the like you tick on the switch screen are set up
    there too.
  - **Copying settings** into a profile: the MCP servers, permissions and
    status line you tick. Never a sign-in, history or caches.
  - **Bringing a rules file in line** writes that one file in your
    project, and keeps the old one for Undo.
  - **Copying an MCP server** to another profile.

  A secret is never copied: a server that needs one reads it from a named
  variable instead, or isn't copied at all. Every set-up, settings copy and
  rules change is listed in Settings › Switching › Context packs & secrets,
  with an Undo that takes out only what was added, and only while it's
  unchanged. A switched conversation's files stay in the other profile,
  where you can switch back from.

**Sends:**

- An update check to GitHub at launch.
- Once a day, a question to npm's public registry for the newest version of
  each tool you have an account on (turn it off on any tool's page). It
  never updates a tool.
- Nothing else unless you turn it on: usage statistics (off by default, see
  [docs/usage-stats.md](./docs/usage-stats.md)), Fleet (your own Macs talk
  directly to each other on your network, with no relay), and the optional
  Figurehead account.
- Crash reports stay on your Mac. Nothing is ever sent automatically. With
  crash reports on, it also looks through macOS's own crash reports for AI
  Juggler's, and keeps only what failed and where.

AI Juggler makes no requests to Anthropic, OpenAI or any other AI provider,
and sets no proxy for your tools. It points a tool at another address only
for an account you add through another service, and only at the address
that service publishes.

[What AI Juggler touches](./docs/guide/trust.md) has the full list, and
names the tests that check each line. The same list is in the app, under
More › **Privacy**, and Settings › Privacy links to it.

## Permissions

All optional. AI Juggler works without any of them.

| Permission | What it's for | Without it |
| --- | --- | --- |
| Accessibility | Bringing a session's exact window to the front | The right app and profile come forward, but not the exact window |
| Automation | Selecting a session's terminal tab | The terminal comes forward, but not the tab |
| Screen Recording | A live preview of a session's window, on this Mac only (offered only where a tool can show one) | No live preview |
| Notifications | Telling you when a session needs you while you're in another app | You see it when you come back to AI Juggler |
| Local Network | Fleet, finding your other Macs | Fleet stays off |

Turn on Accessibility and Automation in Settings › Desktop apps › **Going
to a session**. Going to a session never asks for a permission by itself.

## Staying within your tools' terms

- Only accounts you own. AI Juggler never shares or pools accounts.
- No automatic rotation to get around limits. Switching is something you
  choose, one conversation at a time.
- The real, unmodified tools, run the way you'd run them yourself.

The [proposal](./docs/PROPOSAL.md) explains the reasoning.

## FAQ

**Do I need an API key?** No. AI Juggler uses the tools you've already signed
in to.

**What's the difference between a profile and an account?** A profile is the
setup an app runs on: a Claude Code folder, a Claude app profile, or a Codex
folder. An account is the subscription a profile is signed in to. Sessions
run in a profile; limits and money belong to the account. The same account
signed in to two profiles is normal, and both share its limits. See
[Accounts and profiles](./docs/guide/accounts.md).

**Why isn't a session I closed in the list?** The list shows what's open now.
Ended, inactive and background sessions are behind the "Also:" line under
it, or **View** (⇧V) › **Show**, and every ended session is in History.
Nothing is removed.

**Go to it brought the app forward, but not the session's window.** Bringing
the exact window forward needs the Accessibility permission, and the right
terminal tab needs Automation. Turn them on in Settings › Desktop apps. For
the Claude app, AI Juggler brings the right profile forward and tells you to
pick the session in its sidebar.

**Is there a shortcut to open AI Juggler from any app?** You can choose one
in Settings › Desktop apps › **Open AI Juggler from anywhere**. There's none
until you pick one. ⌥Space isn't accepted, because ChatGPT uses it.

**Why does a limit say "Not measured yet"?** The tool hasn't reported that
limit to AI Juggler yet. AI Juggler shows a percentage only when one was
measured, never a guess.

**Does it change my `~/.claude` or `~/.codex`?** Only when you choose one of
the changes [listed above](#what-ai-juggler-reads-writes-and-sends). Each
shows you what it will change first, and the set-ups and copies can be
undone.

**Can I run Stable and Nightly side by side?** Yes. AI Juggler Nightly is a
separate app with its own data.

**Is switching lossless?** No, and AI Juggler never says it is. Between two
accounts of the same tool, the conversation's files move and it resumes.
Between Claude Code and Codex, the other tool is told the story so far. The
switch screen shows exactly what carries over before you confirm, and you
can switch back. See [Continue on another account](./docs/guide/switching.md).

**I'm coming from Claude Squad, Vibe Kanban, Crystal or cc-switch.** Most of
it comes across by itself, since AI Juggler reads the tools' own files.
[Coming from another tool](./docs/guide/coming-from-other-tools.md) says what
does and what to do about the rest.

**Can it stop me spending too much?** It warns, and never stops anything. Set
a monthly cap for an account or a group of projects (Usage › **Set a monthly
cap**), and Needs you tells you at 80 % and 100 %. See
[Monthly caps](./docs/guide/usage-and-limits.md#monthly-caps).

**Windows or Linux?** Not yet. Windows comes first: 1.0 waits for its first
release. Linux follows; its timing is open. The core is written to make
both ports possible.

## Troubleshooting

- **A tool isn't found when AI Juggler is opened from Finder.** AI Juggler
  reads your login shell's `PATH`. Check that `claude` or `codex` runs in a
  new terminal window, then reopen AI Juggler.
- **"Another AI Juggler is open."** AI Juggler waits a few seconds for an
  earlier copy to finish quitting. If it says this, choose **Try again**, or
  quit the other copy.
- **An update failed, or you want the previous version.** Settings › General ›
  About & updates › **Go back to an earlier version**.
- **Something else?** Ask in [Discussions](https://github.com/figurehead-ai/AI-juggler/discussions),
  or report a bug from Settings › Advanced › Diagnostics › **Report a
  problem**. **Export diagnostics…** there shows you, file by file, what it
  would save (scrubbed of names, paths and keys) before you attach it.

## Uninstall

1. In AI Juggler, open Settings › Advanced › **Before you uninstall** and
   choose **Prepare to uninstall**. It puts back the setting AI Juggler may
   have added to a Claude profile (measuring limits for sessions started
   elsewhere), and lists what's left for you to remove. Anything else you
   set up through AI Juggler stays unless you undo it; the
   [uninstall guide](./docs/guide/uninstall.md) has the steps.
2. Quit AI Juggler and move it to the Trash.
3. Delete `~/Library/Application Support/AI Juggler` (and
   `AI Juggler Nightly`) to remove its data, copies and backups.

Your own `~/.claude` and `~/.codex` folders are otherwise left as they are.

## Roadmap

- **0.4:** trust and the finished basics: a security pass, a real
  command palette, cost per finished task and wasted spend, notification
  and shortcut settings that fit you, and every core feature checked on
  real tools. Also accounts and apps each with their own page, every page
  leading to everything related, pools redesigned around how people run
  several agents, and a simpler New session, all walked through as real
  use cases before release.
- **0.5:** more tools (ZCode, Kiro, Hermes, Muse, Cursor, Antigravity),
  pools across your Macs, and handing work to a colleague.
- **0.6:** the first Stable release.
- **0.7 and 0.8:** AI Juggler for Windows, then Linux.
- **1.0:** after the Windows release has succeeded.

The full plan, with what each version holds and why, is in
[docs/plan/ROADMAP.md](./docs/plan/ROADMAP.md).

Ideas and votes are welcome in
[Discussions](https://github.com/figurehead-ai/AI-juggler/discussions).

## Build from source

You need Rust (the version is pinned by `rust-toolchain.toml`), Node 22 or
later with pnpm 10, [`just`](https://github.com/casey/just), the Tauri CLI
(`cargo install tauri-cli --version "^2" --locked`) and the Xcode Command Line
Tools.

```bash
git clone https://github.com/figurehead-ai/AI-juggler.git
cd AI-juggler
pnpm --dir ui install
just dev        # runs "AI Juggler Dev", with its own data, and reloads the UI as you edit
just verify     # every check CI runs
```

[CONTRIBUTING.md](./CONTRIBUTING.md) is the developer guide. The
[technical specification](./docs/SPEC.md) and the
[architecture decisions](./docs/adr/) explain how it's built.

## Community

- [Contributing](./CONTRIBUTING.md)
- [Code of Conduct](./CODE_OF_CONDUCT.md)
- [Security policy](./SECURITY.md): report vulnerabilities privately
- [Support](./SUPPORT.md)

## License and trademarks

AI Juggler is made by [Figurehead](https://figurehead.ai), which builds AI
tools for developers.

The code is [MIT](./LICENSE) © Faruk Brbovic and AI Juggler contributors.

The names **AI Juggler** and **Figurehead**, the logos and the app icon are
trademarks of Vuxtra LLC and aren't covered by the MIT License. You can use
the code for anything and refer to the project by name; a modified build you
distribute needs its own name and icon. See [TRADEMARKS.md](./TRADEMARKS.md).
