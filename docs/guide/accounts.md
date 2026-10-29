# Accounts and profiles

Two words, kept apart:

- A **profile** is the setup an app runs on: a Claude Code folder (such as
  `~/.claude`, or one named by `CLAUDE_CONFIG_DIR`), a Claude app profile
  (its data folder), or a Codex folder (`CODEX_HOME`). Sessions run in a
  profile.
- An **account** is the subscription a profile is signed in to. Limits and
  money belong to the account.

One account can have several profiles. Signing in to the same account in
two profiles is normal, and AI Juggler doesn't warn about it. Those
profiles share the account's limits, so moving work between them gives you
no new room.

The Accounts page shows each account once, with its limits, and its
profiles under it: each with its badge, name, the apps that use it, its
folder and how many of its sessions are open. Accounts are worked out from
who each profile is signed in to, so a personal plan and a team seat on the
same email are two accounts.

## Add a profile

Add a profile from the Accounts page, then pick the tool.

- **A setup already on this Mac** (one AI Juggler found, or a folder you
  point it to): use it where it is, or make a separate copy, just as on
  [your first run](first-run.md).
- **A fresh profile:** give it a name that says what it's for (you can
  rename it later), then choose **Create and sign in**. The tool's own
  sign-in opens in a terminal. AI Juggler never sees your password. The
  account it belongs to is whichever one you sign in to.

### Start with your usual settings

A fresh profile starts empty. When you already have a profile of the same
tool, **Start with the same settings as …** is ticked and names it. Open it
to see what comes with it, and pick another profile to copy from if you
like:

- **MCP servers**, copied as they are. One that needs a secret or a
  sign-in starts unticked, and says what you need to do.
- **Permissions:** for Claude Code, the commands and tools the profile may
  use without asking; for Codex, its approval policy and sandbox mode.
- **The status line**, if you set one up yourself.
- **Plugins**, listed with the command that installs them, since a plugin
  comes from its marketplace. Run that command in the new profile.

Each item that can be copied starts ticked. Permissions that turn the
tool's checks off (such as Claude Code's `bypassPermissions`, or Codex's
`danger-full-access`) are offered, but never ticked for you.

The profile's sign-in, history and caches are never copied. A setting with
a secret written in it, such as a token in a permission rule, stays behind,
and the line says how many did. Settings only copy between profiles of the
same tool, because each tool's permissions work in their own way.

Anything that couldn't be copied is shown before you sign in.

### Later, or for a copy

Every profile's menu has **Copy settings from…**. It lists what this
profile lacks from another profile of the same tool, each with a box.
Choose **Copy 2 settings** (or however many are ticked). Nothing the
profile already has is changed.

Every copied setting can be undone in Settings › Switching › Handovers and keys. An undo takes out only what was copied, and only while it's
unchanged.

## Through another service

Some subscriptions answer for Claude Code or Codex at an address of their
own: **OpenCode Go** and **Kimi** publish one for each tool. Add one as an
account from the Accounts page: **Add account**, then **More** ›
**Through another service**, and pick the service.

1. Pick its plan, and paste its key once. The key goes straight into AI
   Juggler's own item in the macOS Keychain. It's never shown again, and
   never written to a file, a log, the search index or a backup.
2. Tick the tools to use it with. Each gets a profile of its own, in a
   folder AI Juggler makes, pointed at the address the service publishes,
   with the model you choose. The name starts as the service's short name
   (**Go**, **Kimi**), so a session in a terminal reads "Terminal · Go".

Only that account's sessions get the key, when they start, in the setting
their tool reads (for example `ANTHROPIC_AUTH_TOKEN` for Claude Code on
OpenCode Go). Your own `~/.claude` and `~/.codex` are never changed, and no
other address is ever set.

The account's card says how much can be known about its limits:

- **OpenCode Go: estimate.** Its limits are dollars per model: every 5
  hours allows a fifth of the month's amount, and every week half.
  OpenCode doesn't tell other tools what's left, so the card shows what
  this account's sessions used here, and links to OpenCode's own page.
- **Kimi: unknown.** Kimi doesn't tell other tools what's left. Its own
  pages show it. Kimi's membership is for interactive use only, so AI
  Juggler never starts one of its sessions again by itself.

**Replace key…** on the card takes a new key. If a Mac doesn't have the
key (after restoring a backup on another Mac, say), its profiles say so,
and **Add its key…** puts it back. Removing the account's last profile
removes its key from the Keychain.

Switching a conversation to or from a profile through another service is
never a proven switch: the model behind it changes, and whether the
service takes a conversation another provider started hasn't been tried.
It always asks before it moves anything.

## Your default profile

New sessions start on the default profile, unless a project has its own
(set it from the project's menu). To choose the default, open a profile's
menu and choose **Make the default**. With no default, new sessions start
on the profile you used last in that folder, else the one whose account has
the most room. A default (or the one used last) that's out of room gives way
to the one with the most room, and New session says so.

## Launch settings

Launch settings sit under a profile: a name (for example "Deep work"), a
model, and environment overrides. Choose **Edit launch settings** in the
profile's menu. A session started on that profile uses them.

Under the launch settings' options, **If a session crashes** can follow
Settings, always restart it, or leave it ended. See
[Keep work going](keep-work-going.md).

## Renewal dates

In Settings › Accounts › **Renewal dates**, enter when each subscription
renews if you'd like a reminder, and how many days before. AI Juggler
never reads your billing and never guesses a date. The pane has a link to
the Accounts page.

## Rename or remove a profile

- **Rename…** changes only the name AI Juggler shows.
- **Remove…** takes the profile out of AI Juggler. Its past sessions stay
  in History, marked as removed.
  - A setup used where it is stays on your Mac exactly as it was.
  - For a separate copy, you choose: **Keep the folder** in AI Juggler's
    data, or **Move it to the Trash**, where you can still get it back.

An account has nothing to remove: it leaves the page when none of your
profiles is signed in to it.
