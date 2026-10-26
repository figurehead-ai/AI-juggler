# Your first run

## Find your profiles

On first launch, choose **Find my accounts**. AI Juggler looks for your
Claude Code and Codex setups: `~/.claude`, `~/.codex`, other `~/.claude*` and
`~/.codex*` folders, the folders named by `CLAUDE_CONFIG_DIR` or `CODEX_HOME`
in your shell's startup files, and profiles made by other account managers.
It only reads; nothing is changed by looking.

Each setup it finds is a **profile**: the setup an app runs on. The
**account** is the subscription a profile is signed in to. Two profiles
signed in to the same account are normal, and share that account's limits.
[Accounts and profiles](accounts.md) says more.

## Use it where it is, or make a copy

For each setup it finds, you choose:

- **Use it where it is.** AI Juggler works with the folder you already
  have. Your existing sign-in, settings and history stay exactly where they
  are, and AI Juggler adds nothing to the folder unless you turn on an
  option that says so first.
- **Make a separate copy.** AI Juggler copies the setup into its own data
  folder and uses the copy. Your original is never changed. The copy doesn't
  include the sign-in, so you sign in once inside the copy.

You can change your mind later: add or remove profiles from the Accounts
page.

## Signing in

When a profile needs a sign-in, AI Juggler opens that tool's own sign-in,
in a terminal. You sign in exactly as you would by hand. AI Juggler never
sees your password or your tokens.

## What you'll see

- **The list on the left** shows what's open right now, grouped by place:
  the app and profile each session runs in, such as "Claude app · Work".
  What needs you is on top. Click a session, or press Enter, to open its
  page here. Its **↗** (or ⌘Enter) brings its app forward. See
  [What's open, and getting to it](whats-open.md).
- **Home** shows what needs you, the apps you have open, and how much room
  each account has left.
- **New session** starts the tool you choose, on the profile you choose, in
  the folder you choose.

## Optional permissions

AI Juggler works without any special permission. These are optional:

- **Accessibility**, to bring a session's exact window to the front.
- **Automation**, to select a session's terminal tab.
- **Local Network**, only if you pair your other Macs.

Turn on the first two in Settings › Desktop apps › **Going to a session**.
Going to a session never asks for a permission by itself. macOS asks for
Local Network when you first pair a Mac.
