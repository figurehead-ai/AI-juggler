# Evidence manifest: AJ-E3002, the pools labs (talk per session, Claude's own channel, OpenCode and Grok tools per launch)

Task: `docs/tasks/E30-pools-redesign.md` AJ-E3002. The record is
`docs/qa/pools-lab.md`. Live rows were walked on 0.4.1 release candidates of
AI Juggler Dev under ADR-0027 condition 7b, and checked by Opus in the release
session (its walk results, 2026-11-09..10). The final candidate is rc31
(35c181e2d).

| Req | Result | Evidence | Commit | Date |
|---|---|---|---|---|
| R1 a held or refused message between two sessions AI Juggler started (Claude Code, Codex) is enforced by the bridge per receiving session and per pool | PASS | `pools-lab.md` §1: live P2 on Dev with the Lab App, 2026-11-07 (w1 → w2 held, Allow once delivered, Deny left w3's copy "Not sent"); Codex through app-server in P1 step 5. Re-walked live on rc27: P2 S1–S3 (Ask first held, Allow once delivered, Deny answered in words, the per-line override "Allowed here"); P4 S1, a pool that refuses: "This pool doesn't let you message that member."; P1 S1–S5 with the Codex reviewer | b69cac079, edf440758 | 2026-11-07, rc27 |
| R2 whether Claude Code's cross-session inbound setting, through the launch's `--settings` overlay, refuses or holds Claude's own peer messages for one session, and what the Claude desktop app does with a held one | PARTIAL | **Proven, live:** `refuse` per session. rc2 (2026-11-07): a team session's own SendMessage into the sealed client session was refused ("isn't accepting cross-session messages"), and the client ran nothing. rc27 P7 S3: "Cross-session message refused … not delivered". Built in 57f89e1bb and a3903836e; D-95 (471d8ab9b) makes the seal the only use. **Not proven by a lab:** `hold` was never set in any launch, and the Claude desktop app's handling of a held message was never observed. `pools-lab.md` §2 records both from Claude's documentation (the Code tab can't show the hold dialog, so a held message waits until `dialogExpiry`); D-95 rests on that reading | 57f89e1bb, a3903836e, 471d8ab9b | 2026-11-07, rc27 |
| R3 OpenCode (`OPENCODE_CONFIG_CONTENT`) and Grok Build load AI Juggler's agent tools per launch with no prompt; Kimi Code stays on its shell | PASS | Lab without sign-in: `pools-lab.md` §3 (OpenCode loads them from `OPENCODE_CONFIG_CONTENT`; Grok not from its `GROK_CONFIG` overlay but from `[mcp_servers]` in the `GROK_HOME` AI Juggler made; the real bridge: "✓ juggler connected", "14 tools discovered"). Built: 0c9a4711a, 25eb66563, 149d0e34f. Live on rc17 and rc20 with a signed-in test account of each (sheet `docs/qa/0.4-walks/E3002-R3.md`): both get the 14 agent tools with no prompt; step 8 delivered both ways after Allow once; step 12, tools off: Grok is denied the tools and shows no card. Live again on rc27 (P8 S1): the OpenCode and Grok scouts called `send_message` and `share_doc` with no prompt. Kimi Code: the lab's "Shell only" claim is checked against the adapter (`qa_scenario_seed.rs` `a_scenarios_shell_only_claim_is_the_adapters`) | 0c9a4711a, 25eb66563, 149d0e34f, 9874d9bd9 | 2026-11-09, rc17, rc20, rc27 |
| R4 each result recorded in `docs/qa/pools-lab.md` with its commands and output; a "no" changes the build tasks before they start | PARTIAL | **Recorded:** §1 and §2 with what was run and seen; §3's lab without sign-in with its commands and output; the "no" (Claude's channel can't hold per pair) is in the decision log as D-95, which amended AJ-E3005 R4 and the wizard's words. **Not recorded:** the live part of §3. `pools-lab.md` still reads "the live part is on rc17" and "Left for the live walk", and the rows of `docs/qa/0.4-walks/E3002-R3.md` are blank: the rc17/rc20 results exist only in the release session's walk results, without their commands and output | 55c80304d, 471d8ab9b | 2026-11-10 |

## Still to do for DONE

- R2: a lab that sets `crossSessionInbound: "hold"` on one launch and shows what Claude Code and the Claude desktop app's Code tab do with a held message; or the maintainer's decision that the documented behaviour stands in for it.
- R4: the rc17/rc20 live results written into `pools-lab.md` §3 and the E3002-R3 sheet, with the commands and what was seen.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] No captures here (the walk captures show real names and stay out of the repo)
- [x] No conversation content
