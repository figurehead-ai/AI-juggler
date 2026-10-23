# Rules and MCP servers

Each tool reads its own rules file in a project: Claude Code reads
`CLAUDE.md`, and Codex reads `AGENTS.md`. When you use both, the two drift
apart. AI Juggler shows you where they differ and keeps them the same, but
only when you apply a change.

## See how a project stands

A project's page says **Rules in sync**, or how many rules files differ (for
example **1 rules file differs**). Choose that line to open **Rules and MCP
servers**. Every project's menu has it too: **Compare rules and servers…**.

A file counts as missing only for a tool you use in that project. A
project you only open with Claude Code doesn't need an `AGENTS.md`.

## The rules files

One file is **the master copy**. The others are compared with it. By
default, the master is the newest file with rules of its own. To pick
another, choose the round button beside that file.

Each other file says how it stands:

- **Same as AGENTS.md:** the same text, apart from line endings and blank
  lines at the end.
- **Reads AGENTS.md:** it holds only a line that makes its tool read the
  master, such as `@AGENTS.md` in a `CLAUDE.md`. It always stays in sync.
- **A link to AGENTS.md:** a symbolic link, left as it is.
- **Differs from AGENTS.md:** choose **Show changes** to see the lines
  added and removed.
- **Not there:** the file doesn't exist yet.

### Bring a file in line

1. Open the file that differs. Its changes are shown first.
2. Choose how to apply it. For a tool that can read another file, you get
   two ways:
   - **Copy the rules:** the file gets the master's text.
   - **Read AGENTS.md instead:** the file becomes the one line that makes
     its tool read the master, so you only ever edit one file.
3. Choose **Apply to CLAUDE.md** (or whichever file).

An apply writes only that one rules file. It refuses a link, and a file
that changed since you looked at it. It keeps the file as it was, so you
can undo it: right away in the sheet, or later in Settings › Context packs
& secrets, where it's listed under the project's name. A file the apply
created is removed by its undo.

## MCP servers

Below the rules, the sheet compares the MCP servers of your accounts. Each
server an account lacks is offered once, as a line such as "github for
Work · from Personal", with a **Copy** button.

- Between accounts of the same tool, the server is copied as it is.
- Between Claude Code and Codex, it's written in the other tool's form.
  Some can't go across, such as a server whose kind Codex doesn't support,
  and the line says why.
- A secret never moves. A server with a secret in its environment or
  headers is set up to read it from a named variable, and the line names
  what stays behind. A server with a secret in its address or arguments
  can't be copied.
- Each copy has its own **Undo**, in the sheet and later in Settings ›
  Context packs & secrets.

When nothing is missing, the sheet says **Every account has the same
servers.**

## What it never does

- It never writes anything just because you opened the sheet.
- It never changes a rules file outside the project, or any file you
  didn't apply.
- It never starts a tool or a server to find out what it has. It reads the
  config files.
