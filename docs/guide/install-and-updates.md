# Install and updates

## Install

1. Download `AI-Juggler-<version>-universal.dmg` from the
   [latest release](https://github.com/figurehead-ai/AI-juggler/releases).
2. Open it and drag **AI Juggler** to Applications.
3. Open AI Juggler. macOS asks whether you're sure you want to open an app
   downloaded from the internet; choose **Open**. Every build is signed and
   notarized by Apple, so you shouldn't see any other warning.

You need macOS 14 Sonoma or later, on Apple silicon or Intel, and Claude Code
or Codex installed.

## The apps and channels

| App | Updates from | Data folder |
| --- | --- | --- |
| **AI Juggler** | Stable or Beta, your choice | `~/Library/Application Support/AI Juggler` |
| **AI Juggler Nightly** | Nightly, every day with new changes | `~/Library/Application Support/AI Juggler Nightly` |

- **Stable** gets a version after it has been on Beta for at least two weeks
  without a blocking problem.
- **Beta** gets versions that have been used for real and passed their
  checks.
- **Nightly** is a separate app, so you can try what's coming next to your
  everyday AI Juggler. The two never share data or settings.

To change between Stable and Beta: Settings › About & updates. A steadier
channel never moves you back to an older version; you get the next one when
it reaches that channel.

## Updates

AI Juggler checks for an update when it starts and offers it. Nothing is
installed until you choose to. Before installing, it saves a backup of its
own data in `backups/` inside its data folder, and every update is checked
against AI Juggler's signature before it's installed.

## Going back to an earlier version

Settings › About & updates › **Go back to an earlier version**.

- If the earlier version can open your data as it is now, install its `.dmg`
  from [Releases](https://github.com/figurehead-ai/AI-juggler/releases)
  over the newer one.
- If a version changed AI Juggler's data (its release notes say so), use
  **Go back to an earlier version** first. It says what you'll lose, puts
  the copy saved before the change back in place, keeps the current data
  beside it, and quits. Then install the earlier version.

Your Claude Code and Codex folders aren't affected either way.
