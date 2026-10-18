# ADR-0006: Smart onboarding — environment scan and config import

- Status: Accepted
- Date: 2026-10-12
- Deciders: Vuxtra engineering
- Relates to: ADR-0003 (automation policy), SPEC §12 (onboarding workflow)

## Context

M1-3's launcher assumed the user arrives knowing what a config directory is
and manually labels accounts in a form. User feedback at the M1-3 design
checkpoint set the actual bar: **assume the user knows nothing** — not how
login works, not where Claude keeps its state, not what a profile is. Their
goal is a working parallel setup in minutes, fully guided.

Meanwhile the machine usually already knows the answers: an existing
`~/.claude` (often logged in), `$CLAUDE_CONFIG_DIR` overrides left by other
manager tools, manually- or AI-created `.claude-*` directories the user
forgot about. Asking the user to re-enter what the filesystem can tell us
is exactly the "bunch of stuff they don't understand" the feedback rejects.

The login aid bar was also raised: open the vendor's web sign-in page, open
the vendor's CLI for them — the maximum help that stays inside the vendors'
terms (ADR-0003: we never automate the credential flow itself).

## Decision

1. **Environment scan (metadata-only).** Every adapter exposes
   `scan_environment()` returning existing installs: the vendor default
   location, env-var overrides, and home-naming-pattern matches (manual /
   other-tool / AI-created setups). Probing is limited to file presence,
   sizes, and timestamps plus the existing namespaced-keychain existence
   check — never file contents (ADR-0003 applies to scanning too).

2. **Import = byte copy, source untouched.** `import_account(source, dest)`
   copies a discovered config into a fresh managed dir (0700, marker file).
   Rebuildable noise (`shell-snapshots/`, `statsig/`, `telemetry/`,
   `cache/`, `.DS_Store`) is skipped and reported. A 2 GiB ceiling fails
   fast, before any writes. The source directory is never written to,
   marked, or moved — verified by tree-snapshot tests.

3. **Keychain honesty.** Claude Code on macOS keeps logins in the Keychain,
   namespaced per config-dir path. A copy therefore cannot carry them, and
   cloning keychain entries would mean reading credential contents — a line
   we do not cross. Imports report the plain-language truth ("sign in once
   inside the imported account; it takes a few seconds") and the wizard
   walks the user through it in the embedded terminal (M1-4).

4. **Login aids, capped at "we open things".** `login_url()` per adapter;
   the app opens the vendor's own sign-in page in the system browser
   (tauri-plugin-opener). The guided terminal (already shipped as a
   copyable command) becomes an embedded terminal in M1-4. Nothing in
   between: no form-filling, no token handling, no automation of the
   vendor's flow.

5. **Wizard, not a form.** The onboarding UI (Phase C) is a step-flow:
   detect (automatic scan) → choose (import what was found / add fresh) →
   name & plan in human terms → login guided (web page / embedded terminal /
   copyable command) → summary (what was created, where it lives, that the
   original setup is untouched).

## Consequences

- Users with any prior Claude setup onboard in seconds via import; the
  scan surfaces forgotten setups (including other tools' and AI-assisted
  ones) instead of silently double-managing them.
- Managed dirs are refused by the importer (they're already first-class
  accounts), keeping a single source of truth per setup.
- The scan walks only the home directory's top level plus bounded depth-2
  sizing — no unbounded disk crawling, no scanning outside the home.
- Adapters gain three trait methods (`scan_environment`,
  `import_account`, `login_url`), all default-implemented, so future
  adapters (Codex, ZCode) adopt onboarding incrementally.
- macOS imports will usually need one re-login (keychain path-locking).
  We accept this friction as the cost of never touching secrets; the
  wizard makes it a 30-second guided step.
