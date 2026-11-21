# Evidence manifest — AJ-E1905

Task: `docs/tasks/E19-agent-tasks.md` AJ-E1905, a Claude Code plugin (P2).
Status: **IN-PROGRESS**: built, tested and packaged for release; published with the first 0.5 nightly (D-73, which settled D-43 point 6: "Publish it with 0.5"). 2026-11-13, claude-opus.

Design: `plugins/claude-code/` (a marketplace listing one plugin, `ai-juggler`: its manifest, one MCP server run from the plugin's own folder by `bin/juggler-mcp`, and the `pool-tasks` skill), `crates/juggler-agentbridge/src/plugin.rs` (`juggler-agent --mcp --plugin`). Test output: [plugin-tests.txt](docs/qa/evidence/AJ-E1905/plugin-tests.txt).

## What exists now

1. **The bundle**: the repository's marketplace `.claude-plugin/marketplace.json` (at the root since 2026-11-13, so `/plugin marketplace add figurehead-ai/AI-juggler` finds it; `claude plugin marketplace add` takes no sub-path) and `plugins/claude-code/ai-juggler/` with `.claude-plugin/plugin.json` (version 0.2.0, the app's), `.mcp.json` (`${CLAUDE_PLUGIN_ROOT}/bin/juggler-mcp`), and `skills/pool-tasks/SKILL.md` (when to list, claim, update, give back; what only the user does).
2. **The server**: `bin/juggler-mcp` finds AI Juggler's own `juggler-agent` (the launch's `JUGGLER_AGENT`, else the installed editions' bundles) and runs it as `--mcp --plugin`. In a session AI Juggler started (its key in the environment) it lists no tools, since the launch already added them; anywhere else every call says the session wasn't started by AI Juggler with its agent tools, or that AI Juggler isn't running. With no AI Juggler on the Mac it says so and exits. It never reads, writes or carries a key.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| R1 a plugin bundling the bridge's MCP config and a task skill | `plugin.rs::the_plugins_manifests_are_what_claude_code_reads_and_hold_no_secret` (the root marketplace lists `./plugins/claude-code/ai-juggler`; the plugin's version is the app's); `claude plugin validate` on the plugin and on the marketplace, each with a throwaway config folder: both pass (2026-11-13) | PASS |
| R1 packaged with the app's releases | `scripts/plugin_package.py` writes `AI-Juggler-Claude-Code-plugin-X.Y.Z.zip` (the plugin's folder only, its `plugin.json` at the release's version, entries sorted, fixed times, the launcher 0755; refuses a file holding a key, the socket, an address or a user's folder), run by the nightly's release job before `SHA256SUMS.txt` and the provenance, which names `*.zip`. `--selftest` (in `just release-selftest`): the contents, the version, same bytes on a second run, bad versions refused, a plugin holding an address refused with no zip left. `release_version.py stamp` writes the same version into the checkout's `plugin.json`; `release_ships.py` counts `plugins/**` and the packager as shipped. Run locally as CI runs it, 0.5.0: 5 files, sha256 `b97dc2ae…7bc7f`; unpacked, `claude plugin validate` passes; `claude --help`: `--plugin-dir` loads "a directory or .zip" | PASS |
| R1 published with the app's releases (on a real release) | The first nightly built from main after 0.5 merges: its release lists the zip and `SHA256SUMS.txt` names it | PENDING (first 0.5 nightly) |
| R2 works only while AI Juggler is running, and says so otherwise | `plugin.rs::installed_in_a_fixture_home_it_runs_the_bridge_and_says_why_not` (the real `juggler-agent`: the tools and why not outside AI Juggler's launches; nothing listed twice inside them; no AI Juggler: says so); `plugin::tests` (2) | PASS |
| Goal: "even outside AI Juggler's launches" | Ruled by D-73: the plugin works only while AI Juggler runs, in sessions it started, and says so elsewhere (ADR-0020 gives a session its identity only with a launch; D-30) | PASS (as ruled, D-73) |
| Live: the plugin in a real Claude Code session outside AI Juggler | 2026-10-26, `claude -p --plugin-dir plugins/claude-code/ai-juggler` (session-only, nothing installed) on the lab profile, the server pointed at this branch's `juggler-agent` ([AJ-E1902/lab-2026-10-26.md](docs/qa/evidence/AJ-E1902/lab-2026-10-26.md)): connected; the tools listed and the skill loaded | PASS |
| Gate: a plugin install test in a fixture home | `installed_in_a_fixture_home_…` (copied as Claude Code's plugin cache keeps it, run as its `.mcp.json` says) | PASS |
| Docs | `docs/guide/pools.md` § The Claude Code plugin (optional; what it does and doesn't; marketplace, a release's tag, the zip, removing it); `plugins/claude-code/ai-juggler/README.md`; `docs/RELEASING.md` (the asset) | PASS |

Open design note (2026-11-13): the lab below saw that outside AI Juggler's launches the model gets the tools, each answering why it can't. D-73 ruled publishing it as built; listing no tools there (with the reason in the server's instructions) is the alternative if the tools' failures prove noisy in use.

The labs' recommendation for D-43 point 6 (in that file): don't publish. AI Juggler's own Claude launches load a skill-only copy with `--plugin-dir`, which installs nothing. If a published plugin is still wanted, its server should list no tools when there is no key.

Last updated: 2026-11-13.
