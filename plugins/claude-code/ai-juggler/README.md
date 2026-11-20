# AI Juggler for Claude Code (optional)

AI Juggler's pool tools and a skill for working the pool's tasks, as a
Claude Code plugin. It is optional: sessions AI Juggler starts with its
agent tools already have them.

It works only while AI Juggler runs, in sessions AI Juggler started.
Anywhere else each tool answers why it can't, and with no AI Juggler on
the Mac the server says so and stops. It holds no key and reads none: a
session gets its key only from AI Juggler's launch.

## Install

From this repository's marketplace, in Claude Code:

```
/plugin marketplace add figurehead-ai/AI-juggler
/plugin install ai-juggler --marketplace figurehead-ai/AI-juggler
```

To stay on the version that matches your AI Juggler, add the marketplace
at its release tag instead, for example `figurehead-ai/AI-juggler#v0.5.0`.

Or, for one session, from the zip each release carries
(`AI-Juggler-Claude-Code-plugin-X.Y.Z.zip`):

```
claude --plugin-dir AI-Juggler-Claude-Code-plugin-X.Y.Z.zip
```

## What it has

- `.mcp.json`: one MCP server, `bin/juggler-mcp`, which runs the
  installed AI Juggler's own `juggler-agent`.
- `skills/pool-tasks`: how to claim, work and release a pool's tasks.

The guide: [Pools › The Claude Code plugin](../../../docs/guide/pools.md#the-claude-code-plugin).
