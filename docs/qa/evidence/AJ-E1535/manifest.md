# Evidence manifest — AJ-E1535

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1535, plain words and
consistency (a 0.1 P0), with QA batch 7's items for it. Status:
**DONE-EVIDENCED** (IN-REVIEW until the release PR).

## What exists now

1. **No developer words on screen.**
   - **A quota's tier** reads "measured", "worked out" or "estimate" next
     to its number. It used to read "MEAS", "COMP" or "EST", and a row's
     description said "computed".
   - **Token counts** say "tokens", not "tok".
   - **Switching says what carries.** Badges and the switch timeline use
     each fidelity's own user label ("Full story, fresh start", "Same
     conversation, new account", "Briefing pack", "Continues exactly").
     A return reads "Switched back · …". Gone: "F1 · Beta",
     "transplant F1 · 12:04", "switch-back ↺ · F1" and the "(T2
     transplant)" / "(T3 replay)" labels.
   - **One word for a switch's state: proven.**
     - A switch reads "· Proven" or "· Not proven yet". "Validated",
       "Unvalidated" and "F1 · Unvalidated" are gone.
     - The backend renders these badges once, for every surface
       (`PairCell::badge_for`, the cross-machine targets).
   - **Cross-machine blockers** say "moving the same conversation needs
     the same version" or "use “Full story, fresh start”" (no "T2" or
     "T3").
   - **Sessions found running after a restart** are "picked up after
     restart" (the chip, the session notice, Cleanup's filter, the port
     map). They were "re-adopted". The recovery summary reads "2 sessions
     picked up again, 1 helper left behind, 1 process could not be
     checked and was left alone." Its counts are now singular when they
     should be ("1 helpers" is gone).
   - **"root: … copy" and "imported sessions: 0"** went with AJ-E1533's
     project page (`ae3ef3df`). Nothing shows them.
   - **New session's footer** speaks only when a folder is picked
     ("Launching in ai-juggler"). "No folder picked — …" is gone.
   - **QA batch 7:**
     - The launch settings each account starts with (stored as "Main")
       read "Default" on every surface: Accounts, project actions, New
       session, the inspector, the switch rows and the sign-in mismatch
       dialog. One helper does it: `lib/account-words.ts`
       `launchSettingsName`. `launchTitle` gives a launch's first
       title.
     - Accounts › "Which switches between accounts are proven" is in
       plain words:
       - its columns are Switch, What carries, Automatic tests, Tried
         by hand, Status;
       - tools are named by their names;
       - it lists only switches between the tools AI Juggler has (it
         used to show six rows for Z Code, Gemini CLI and OpenCode,
         which no one can use);
       - the backend's status and notes are plain too ("Automatic tests
         pass with the real tools; not tried by hand yet").
2. **A finished session's row shows no account "% left".** One helper
   decides it (`indicator.tsx` `lowLeftWords`): the rail row, its
   screen-reader text and the session header. It speaks only for a
   running session whose account is low. QA's ", Done, Work Max, 0% left"
   now reads ", Done, Work Max".
3. **This Mac's "under 1 MB" for a Claude session.**
   - **What it measured:** a tree's resident memory (the session's
     process and everything under it, summed). A live process always
     has some.
   - **Where 0 came from:** when a session's process has exited but the
     session isn't marked ended yet, the collector reports zeros
     (`LiveProcessStats::dead`). "Under 1 MB" was those zeros, not a
     measurement.
   - **On this Mac, the real trees are large:**
     - the Claude app's own Code sessions (a 672 KB `disclaimer` helper
       launching a 126–208 MB `claude`) are counted inside the app's
       tree;
     - the replica's stand-in CLIs are 2.3–2.6 MB.
   - **The fix:** `LiveProcessStats::measured` treats zero as nothing
     measured. This Mac leaves such a tree out (and out of its totals),
     and the session's view shows its memory as unknown ("memory —").
   - The words "under 1 MB" stay for a real tree that small.
   - The gallery showed it too, from the test fixture: every running
     session was 123 KB. It is now 180 MB, as a Claude Code session
     measures, so the captures show a real size.
4. **New session opens without waiting.**
   - Its launch settings and its ranking were read by synchronous
     commands, which run on the app's main thread. When a tick held the
     storage lock, the whole window waited. `launch_recipes_list` and
     `ranking_rows` now run off it (`ipc::blocking`), and they leave the
     IPC audit's sync allowlist.
   - The sheet no longer reads every session again for names: it uses
     the sessions the app already has, named by `sessionTitle`.
   - Found on the walk and fixed: pressing Customise ▸ dropped keyboard
     focus to the page, so Esc and Tab no longer reached the sheet. Focus
     now moves to the twin toggle (`lib/use-twin-toggle.ts`).
   - Found on the walk and fixed: New session's account line read "Not
     measured yet" for Work Max while Home read "58% left, from its
     latest session". New session ignored the account's sessions, the
     fallback Home uses. `RunwayWords` now takes them, so both say the
     same (vitest `quotaWindow`: "New session says what Home says").
5. **The plain-words lint** (`scripts/plain_words_lint.py`, in `just
   terminology` and CI's terminology step).
   - **What it reads:** the text that can reach the screen. That is
     string literals and JSX text in shipped UI code, and string
     literals in the Rust crates whose strings the UI shows. It skips
     comments, tests, generated code, a one-word identifier (a wire
     value or a test id) and a template's `${…}`.
   - **What it bans:** tier tags, "tok", fidelity codes (F0–F4), switch
     tiers (T1–T4), the spec's failure codes (S-F1…), "transplant",
     "switch-back", "re-adopted", "dogfood", "CI row", "live cell",
     "beta debt", "tool pairs", "Unvalidated", "imported sessions" and
     "No folder picked".
   - **The allowlist** (`scripts/plain_words.allow`, `path term count`)
     only shrinks. Two areas are left:
     - the switch pipeline's error copy, which still opens with its spec
       code ("S-F4: staging failed mid-copy …") — a follow-up;
     - the maintainer's fidelity register document, which the app never
       shows.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| 1 no developer words | `plain_words_lint.py` (selftest + OK on the tree); vitest `beta-matrix` (no `F0–F4`, dogfood, CI row, ceiling, tool pairs, beta debt or live cell in the table; only registered tools), `switch-graph` (edge labels, "Not proven yet"), `SessionRow`/`density` ("tokens", "worked out"), `cleanup-centre`/`port-map`/`processes-tab`/`recovery-summary` ("picked up after restart"); Rust `switch_graph_targets` (edge labels in plain words, "Switched back · "), `validation_matrix`/`validation_gate`/`fleet_switch` (badges, status words), `recovery` (the summary) | PASS |
| 1 New session footer | `NewSession.tsx` (footer only with a folder); walked in the preview | PASS |
| 1 "Default" everywhere | `launchSettingsName` at every recipe name on screen | PASS |
| 2 finished rows show no "% left" | `lowLeftWords` (rail, its description, the session header); walked: the preview's finished `main · Done · Work Max` row | PASS |
| 3 "under 1 MB" | `machine_health` `a_tree_with_nothing_measured_is_not_on_this_mac`; the session read model uses `LiveProcessStats::measured`; the trees measured on this Mac (above) | PASS |
| consistency: New session's runway = Home's | vitest `quotaWindow` ("New session says what Home says for the same account") | PASS |
| 4 New session without delay | `launch_recipes_list` and `ranking_rows` async (`ipc_audit` allowlist shrank by 3); names from the app's sessions; focus kept on Customise (walked: Esc closes the sheet after Customise) | PASS |
| 5 the lint in CI | `justfile` `terminology`, `.github/workflows/ci.yml` terminology step; allowlist only shrinks (selftest) | PASS |

## Walked in the preview: the three questions

1. **Does it solve the real problem?** The words a user reads are the
   user's. A switch says what you get ("Full story, fresh start"), not
   our tier code. A finished session no longer claims an account is at
   "0% left". This Mac stops listing a session that isn't running as
   "under 1 MB".
2. **Is it clear without explanation?** The switches table explains
   itself: "Automatic tests: Pass with the real tools · Tried by hand:
   Not yet · Beta — not proven yet". A quota's tier is a word, not an
   abbreviation.
3. **Is it consistent with its siblings?** One name for the default
   launch settings, one word for a switch's state (proven), one helper
   for "% left", and one plain label per fidelity, rendered by the
   backend for every surface.

## Found on the way (follow-ups)

- The switch pipeline's error copy still opens with the spec's failure
  codes ("S-F1: …" to "S-F9: …"). It's allowlisted in the lint, so it
  can only shrink. Rewording it touches the switch taxonomy and about
  ten tests.
- `size_budget.py` and `tool_literal_lint.py` stop reading a Rust file
  at its first inline `#[cfg(test)] mod`. In `commands.rs` that is
  line 4321 of about 13,000, so most of the file is never counted. The
  plain-words lint skips each test module and reads on. Fixing the
  other two would show today's real sizes; that needs its own change.
- `instances_list` (the session list every screen polls) is still a
  synchronous command. It can't move off the main thread yet: QA's
  `qa_real_readonly` harness, copied verbatim from its branch, calls it
  directly.
