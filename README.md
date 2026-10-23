# AI Juggler

**See every Claude Code and Codex session and account on your Mac, with honest usage and limits.**

Free and open source (MIT), local-first, for macOS. AI Juggler works with the
coding tools you already use. It never proxies their traffic, never signs in
for you, and never stores your credentials.

[![CI](https://github.com/figurehead-ai/AI-juggler/actions/workflows/ci.yml/badge.svg)](https://github.com/figurehead-ai/AI-juggler/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/figurehead-ai/AI-juggler?include_prereleases&label=release)](https://github.com/figurehead-ai/AI-juggler/releases)
[![macOS 14+](https://img.shields.io/badge/macOS-14%2B-informational)](#download)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

[Download](#download) · [Quick start](#quick-start) · [User guide](./docs/guide/) · [Privacy](#what-ai-juggler-reads-writes-and-sends) · [Discussions](https://github.com/figurehead-ai/AI-juggler/discussions)

<!-- Screenshots (Home, Accounts, a session, This Mac; light and dark) are captured from the test setup for the 0.1 release. -->

## Why

If you pay for more than one AI coding subscription, say two Claude accounts
and Codex, you end up juggling by hand: config folders, shell aliases,
terminal tabs, and guessing which account still has room. AI Juggler puts all
of it in one window:

- **Every session in one list**, grouped by project, including the ones you
  started in a terminal or in the Claude app.
- **Usage and limits you can trust.** A number is shown only when it was
  measured. Anything else says "Not measured yet", never a made-up 100%.
- **Your Mac at a glance.** How much memory and CPU your AI tools use, which
  window is heavy, and leftover processes you can clean up.
- **Accounts managed for you.** Use each setup where it already is, or as a
  separate copy, and start a session on the account you choose. A new
  account can start with your usual MCP servers and permissions.
- **Carry a conversation to another account or tool** when one runs out of
  room, and switch back later.
- **Your rules and MCP servers, the same everywhere.** See where
  `CLAUDE.md` and `AGENTS.md` differ, and bring them in line in one click.

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
one without you. Settings › About & updates shows your version, lets you
switch between Stable and Beta, and can go back to an earlier version.

## Quick start

1. Open AI Juggler and choose **Find my accounts**. It looks for your Claude
   Code and Codex setups (for example `~/.claude` and `~/.codex`).
2. For each setup, choose **Use it where it is** (nothing is copied) or
   **Make a separate copy** (AI Juggler keeps its own copy, and your original
   is never touched).
3. If a tool needs you to sign in, it opens that tool's own sign-in.
   AI Juggler never sees your password.
4. Your sessions show up in the list on the left, under their projects,
   wherever you started them. Start a new one with **New session**.

The [user guide](./docs/guide/) covers each step in more detail.

## What works today

| | Status |
| --- | --- |
| Every session and project, including ones started outside AI Juggler | Works |
| History of finished sessions, searchable | Works |
| Usage and costs per session, account and project, from the tools' own records | Works |
| Codex limits, read from what Codex records | Works |
| Claude limits | Works for sessions started in AI Juggler; for other sessions, turn it on per account (see [Privacy](#what-ai-juggler-reads-writes-and-sends)) |
| This Mac: memory and CPU of your AI tools, including the Claude app | Works |
| Cleaning up leftover processes (automatic cleanup is off unless you turn it on) | Works |
| Rename, remove and choose a default account | Works |
| A new account starting with your usual MCP servers, permissions and status line | Works |
| Rules files and MCP servers compared across your tools, brought in line when you apply | Works |
| Restarting crashed sessions, keeping the Mac awake while they work, reopening after a crash | Works (each one is a setting) |
| Opening AI Juggler at login | Beta: new in 0.1 |
| Backups, and going back to an earlier version | Works |
| Switching a conversation to another account or tool | Beta: Claude Code → Claude Code → Codex → back has been checked live; every pair still says "Not proven yet" |
| Fleet: your other Macs in the same window | Beta: not yet verified across two Macs |
| More tools (OpenCode, ZCode, Kimi and others) | Planned |
| Windows and Linux | Not before 1.0 |

## Supported tools

| Tool | Support |
| --- | --- |
| **Claude Code** (the `claude` command) | Sessions, usage, limits, starting sessions, switching (beta) |
| **Codex** (the `codex` command) | Sessions, usage, limits, starting sessions, switching (beta) |
| **Claude app** (desktop) | Its Claude Code sessions are listed; This Mac measures its memory and CPU |

AI Juggler finds each tool through your shell's `PATH`, the way your terminal
does, even when you open the app from Finder.

## What AI Juggler reads, writes and sends

**Reads, on your Mac only:**

- Your tool setups: Claude Code's `~/.claude` (and other Claude folders you
  use) and Codex's `~/.codex`, to list conversations and count usage from
  their transcripts.
- Account names and emails from those setups, so you can tell accounts
  apart.
- The list of running processes, to show live sessions and This Mac.

It checks only whether a sign-in exists. It never reads your passwords,
tokens or API keys, and never opens the tools' Keychain items.

**Writes:**

- Its own data folder, `~/Library/Application Support/AI Juggler`
  (`AI Juggler Nightly` for Nightly). Copies of setups, backups and settings
  live there.
- In your own setups and projects, only what you choose, each shown before
  it's made, and each with an Undo:
  - **Measuring Claude limits for sessions started elsewhere** (off by
    default, per account). It shows the exact one-line change to that
    account's `settings.json` before making it, keeps your own status line
    working, and turning it off restores the file exactly.
  - **Switching a conversation** into an account adds the conversation's
    files to that account, beside its own and never over them. The MCP
    servers, skills and the like you tick on the switch screen are set up
    there too.
  - **Copying settings** into an account: the MCP servers, permissions and
    status line you tick. Never a sign-in, history or caches.
  - **Bringing a rules file in line** writes that one file in your
    project, and keeps the old one for Undo.
  - **Copying an MCP server** to another account.

  A secret is never copied: a server that needs one reads it from a named
  variable instead. Every set-up is listed in Settings › Context packs &
  secrets, with its Undo, which takes out only what was added and only
  while it's unchanged.

**Sends:**

- An update check to GitHub at launch.
- Nothing else unless you turn it on: usage statistics (off by default, see
  [docs/usage-stats.md](./docs/usage-stats.md)), Fleet (your own Macs talk
  directly to each other on your network, with no relay), and the optional
  Figurehead account.
- Crash reports stay on your Mac. Nothing is ever sent automatically.

AI Juggler makes no requests to Anthropic, OpenAI or any other AI provider.

## Permissions

All optional. AI Juggler works without any of them.

| Permission | What it's for | Without it |
| --- | --- | --- |
| Accessibility and Automation | Jumping to a session's own window | The app opens, but not the exact window |
| Local Network | Fleet, finding your other Macs | Fleet stays off |

## Staying within your tools' terms

- Only accounts you own. AI Juggler never shares or pools accounts.
- No automatic rotation to get around limits. Switching is something you
  choose, one conversation at a time.
- The real, unmodified tools, run the way you'd run them yourself.

The [proposal](./docs/PROPOSAL.md) explains the reasoning.

## FAQ

**Do I need an API key?** No. AI Juggler uses the tools you've already signed
in to.

**Why does a limit say "Not measured yet"?** The tool hasn't reported that
limit to AI Juggler yet. AI Juggler shows a percentage only when one was
measured, never a guess.

**Does it change my `~/.claude` or `~/.codex`?** Only when you choose one of
the changes [listed above](#what-ai-juggler-reads-writes-and-sends), and
each shows you what it will change first and can be undone.

**Can I run Stable and Nightly side by side?** Yes. AI Juggler Nightly is a
separate app with its own data.

**Is switching lossless?** No, and AI Juggler never says it is. Between two
accounts of the same tool, the whole conversation moves. Between Claude Code
and Codex, the other tool is told the story so far. The switch screen shows
exactly what carries over before you confirm, and you can switch back. See
[Continue on another account](./docs/guide/switching.md).

**Windows or Linux?** Not before 1.0. The core is written to make that port
possible.

## Troubleshooting

- **A tool isn't found when AI Juggler is opened from Finder.** AI Juggler
  reads your login shell's `PATH`. Check that `claude` or `codex` runs in a
  new terminal window, then reopen AI Juggler.
- **"Another AI Juggler is open."** AI Juggler waits a few seconds for an
  earlier copy to finish quitting. If it says this, choose **Try again**, or
  quit the other copy.
- **An update failed, or you want the previous version.** Settings › About &
  updates › **Go back to an earlier version**.
- **Something else?** Ask in [Discussions](https://github.com/figurehead-ai/AI-juggler/discussions)
  or [open an issue](https://github.com/figurehead-ai/AI-juggler/issues/new/choose).

## Uninstall

1. In AI Juggler, open Settings › Data › **Before you uninstall** and choose
   **Prepare to uninstall**. It puts back the setting AI Juggler may have
   added to a Claude account (measuring limits for sessions started
   elsewhere), and lists what's left for you to remove. Anything else you
   set up through AI Juggler stays unless you undo it; the
   [uninstall guide](./docs/guide/uninstall.md) has the steps.
2. Quit AI Juggler and move it to the Trash.
3. Delete `~/Library/Application Support/AI Juggler` (and
   `AI Juggler Nightly`) to remove its data, copies and backups.

Your own `~/.claude` and `~/.codex` folders are otherwise left as they are.

## Roadmap

- More tools: OpenCode, ZCode, Kimi and others.
- Notifications when a session needs you.
- Groups of accounts, and agents that can message each other.

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
