# ADR-0027: The test bridge writes only on a throwaway lab

**Status:** Accepted
**Date:** 2026-11-05
**Deciders:** the coordinating session (an engineering ruling on the test method, 2026-11-05)

## Context

0.4's validation (AJ-E3302–E3306, the pools of E30) needs walks that change things: make a pool, start a session, switch it, stop it. The walks are done by an agent, the maintainer's standing rule.

- The agent can't drive AI Juggler's native window: it has no Accessibility grant, and won't be given one for the real Mac.
- The test bridge (docs/qa/DEV-BRIDGE.md) shows the real UI in a browser, but it is read-only by construction. Every write gets 403.
- Each walk already runs on a lab: a throwaway HOME under `/tmp/aij-lab…`, seeded from fixtures, torn down after (`scripts/qa/lab_scenario.sh`).

## Decision

The bridge has a lab-only write mode. Each condition below is part of the decision; dropping one voids it.

1. **Built only with its own feature.** `lab-bridge` sits on top of `dev-bridge`, so it compiles into debug builds only. `scripts/dev_bridge_check.sh` proves the shipped feature sets lack it: no default feature turns it on, no release workflow names it, and a release build with it fails to compile.
2. **It starts only on a throwaway lab, and refuses loudly otherwise.** With `AIJ_DEV_BRIDGE=1 AIJ_LAB_BRIDGE=1`, the app checks `$HOME` first (`dev_bridge::lab::checked_home`):
   - The path must be absolute, and spelled exactly as it resolves (only macOS's `/tmp` → `/private/tmp` allowed), so a symlink anywhere in it is refused.
   - Resolved, it must be a folder directly under `/tmp` named `aij-lab…`, this user's and closed to everyone else (0700; `lab_scenario.sh` makes it so).
   - It must not overlap the home the user record names. That keeps it away from the real AI Juggler data too.
   - The app's data folder must be inside it.
   - No tool's folder override may be set in the process (each adapter's `isolation_env_keys`, such as `CLAUDE_CONFIG_DIR` or `CODEX_HOME`). A shell inside a tool's session carries one pointing at a real folder; the lab script starts everything through `scripts/qa/lab_env.sh`, which drops them.

   On any failure the whole bridge doesn't start, with the reason on stderr. In a build without the feature, asking for it also refuses.
3. **Each write is the window's own command.** The lab's route table (`LAB_ROUTES`) names each command and the function it calls, with typed arguments and `deny_unknown_fields`. It's the same function the window invokes, with no logic of its own. A test holds each route to a command classed `Write` in the registry. The allowlist is per command, never a switch for all writes. Everything else still gets 403. The token, loopback and Origin checks are unchanged.
4. **Real accounts stay out.** A lab's Accounts and Apps come only from its seeded fixtures, or from the sign-ins the maintainer agreed to, under the live-test rules (docs/qa/runtime-lab.md). Before each write, a guard (`dev_bridge::lab::guard`) resolves every folder the write would touch, and refuses with 403 when one is outside the lab:
   - the session's folder;
   - its launch settings' folder and their Account's folder (and where that Account came from);
   - a launch's working folder. A launch with none starts in the lab's folder, never `$TMPDIR`.

   At start, the same rule covers every Account the lab's store holds: one outside the lab, or on a tool's default folder, refuses the mode. The app's own background work, such as the limits probe, runs every Account it holds, not only the ones a write names. Writes that only touch the store but name a session (moving one between pools, setting its role) are held to the lab's sessions too.

   An Account on a tool's own default folder (`vendor_default_dir`) is refused too: with its folder override unset, Claude Code signs in through the login Keychain, which a lab HOME doesn't move. A switch to a desktop app is refused, since desktop apps run outside the lab.
5. **Reviewed before any walk uses it.** The threat model is updated (DEV-BRIDGE.md, SECURITY-GUIDELINES.md), and the diff gets a security review. Tests cover: refusal on the real home, on a symlinked `/tmp` folder, on an overlap with the real home, on a data folder outside the lab, on another owner, and without the feature. Any security finding blocks the mode.

7. **One real sign-in, named when the lab starts (amendment, 2026-11-05).** For the live steps with a real tool, the coordinating session agreed to "use the real Codex sign-in where it is, in the lab, under the same rules". `AIJ_LAB_ADOPT=<tool>` lets the lab use that one tool's own default folder in the real home, where it is:
   - only a tool whose own default folder holds its sign-in (the adapter's `default_folder_holds_sign_in`: Codex's `auth.json`; not Claude Code, which signs in through the login Keychain), which can use a folder in place (ADR-0018: checked by its sign-in's shape, never written by AI Juggler);
   - only that folder: the adapter's default folder, moved from the lab's HOME to the home the user record names, resolved, inside the real home and not the home itself, a folder of this user's (`dev_bridge::lab::adopted_folder`);
   - only for an Account of that tool used in place on exactly that folder, and only to launch, stop, resume and message its sessions. A switch from or to it is refused, since a switch writes the carried conversation into the target's folder. Every other Account, folder and write keeps conditions 2–4;
   - the lab indexes only that folder's conversations whose folder is in the lab (`lab_scope::hides`): the rest of the real history never reaches the lab's lists, screens or captures. A pool-wide message is delivered member by member, each Deliver held to the lab's sessions.

   The start line names the tool and the folder. Anything that doesn't hold refuses the whole mode, as condition 2 does. The tool itself writes its own session files into its folder, as it does outside the lab. The live-test rules say how: one or two tools at a time, small tasks, Codex's own trust screen never answered (AI Juggler passes trust in memory), and the folder's paths, sizes and times (never contents) recorded before and after.

   **7b. Live Claude Code through the Lab app (amendment, 2026-11-07, the maintainer's approval).** Claude Code signs in through the login Keychain, which a `/tmp` lab can't reach, so condition 7 can't adopt it. For the walks that need a live Claude session (P1, P2 and P7.3), the maintainer approved, in their own words: "agents may drive the Lab Claude Code app live from AI Juggler Dev in ~/lab/ with harmless commands". This is not the bridge's write mode, which still refuses the real home (condition 2 is unchanged). It is this, and only this:
   - **AI Juggler Dev on the real HOME**, its own Dev data folder (never the installed app's), driven in its native window by its process id only (`osascript`, `screencapture -l`), never by name: Dev and the installed AI Juggler share the process name `juggler-app`. Any macOS permission prompt (Accessibility, Automation) is clicked by the maintainer, never the agent.
   - **Only the "Lab" Claude Code App acts**: its folder and every working folder under `~/lab/`. Sessions are launched, messaged, stopped, switched and pooled only on it; no other App, profile or Account is started, stopped, switched or messaged, and the maintainer's other Claude profiles and the installed AI Juggler are never touched.
   - **Harmless commands only**: `ls`, `echo`, a write to `lab.txt` inside `~/lab/`. No sign-in, no Allow on a sign-in or Keychain dialog, no read of a credential file, the Keychain or a process environment.
   - **Captures**: the real HOME's other Apps and names can show on screen, so every capture is opened by eye and redacted with the fail-closed tool and its sidecar before it's committed; unredacted ones never leave the scratchpad.
   - **After**: the lab sessions and project are removed, the Dev app quit, and nothing left running (checked).

6. **A lab sees only its own processes.** With the mode on, the process table the app discovers apps and sessions from keeps a process only if its program or an argument names a path in the lab, or if this app started it (`lab_scope`). This Mac's own apps, and their profile names, never show on a lab's screens or in its walk captures. A process's environment is never read for this.

## Alternatives considered

- **The maintainer walks by hand.** That's eleven scenarios, and the standing rule is that the agent walks them.
- **Grant the agent Accessibility on this Mac.** That would give control of every app on the real setup, which is far broader than a lab.
- **A blanket "allow writes" switch on the bridge.** One flag would open every write, including imports, sign-ins and settings. It's refused in favour of a per-command list.

## Consequences

- Walks of every write path run in a browser against the real backend, with captures and timings.
- The bridge now has a write path, in Dev builds only. Its exposure is the lab's own throwaway data, to whoever holds the per-run token on this Mac.
- On a lab, session launches start the tools, unlike the read-only bridge, which starts none. A launch uses the lab's own config folders only, never a tool's default folder, so it signs in with the lab's own sign-in, or not at all.
- The first review of this mode (2026-11-05) found that an inherited folder override, the default folder's Keychain sign-in, an unchecked working folder and a desktop switch could reach real data. The env check, the per-write guard and the 0700 / exact-spelling checks are its fixes.
- Adding a command to the list is a reviewed change to `dev_bridge/lab.rs`. The walks added opening a thread (it marks it read), the person's own message to a pool or a member (`messages_send`, held to the lab's sessions when it names one), and the pool page's "Let agents … work together?" answer (`agent_bridge_answer`, a setting in the lab's store). On 2026-11-07, with the maintainer's approval, E23's runtime lab added a turn sent to a driven session, an ask answered and the hand-over (`runtime_send`, `runtime_answer`, `runtime_handover`, each held to a lab session), the folder-trust choice (`runtime_trust_choose`, a lab folder and lab launch settings) and the Beta switch (`runtime_beta`, the lab's own setting).
- Condition 7 puts one real sign-in in a lab, by name, for the live steps. Its exposure is that tool's own folder as the tool itself uses it; AI Juggler reads it as it does outside the lab, lists only the lab's own conversations from it, and writes nothing there. Usage and limits read from the folder (numbers, no names) do show on the lab's screens.
- Its review (2026-11-05) found a same-tool switch would write into the real folder, the real history would be indexed into the lab, any in-place tool was accepted, and a pool-wide delivery wasn't held to the lab. The switch refusal, the index scope (decided before the read model's first pass, with anything indexed earlier read again), admitting only a tool whose own folder holds its sign-in (Codex today), and the per-member Deliver are its fixes. Left, low: a lab tool's own pid file could name a process outside the lab (`procs::some` isn't scoped); it can't reach the adopted folder.
