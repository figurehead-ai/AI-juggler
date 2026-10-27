# Evidence manifest — AJ-E1539

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1539 (a live switching lab). Status: **DONE-EVIDENCED**. 2026-10-22, reviewer claude-opus.

## What it is

`crates/juggler-app/tests/switch_lab_live.rs`, run with `just switch-lab` (ignored and a no-op unless every `AIJ_LAB_*` variable is set, so it never runs in CI). It drives AI Juggler's own switch code over a throwaway data folder and a new throwaway project, with the real Claude Code and Codex on accounts whose owner agreed. The maintainer allowed their own accounts on 2026-10-22, in a lab project only.

The journey is the maintainer's:

1. A small coding task on Claude account A: `calc.py` with `add`, its test, and a three-item task list. The first turn is told a codename, `BLUE-HERON`.
2. Switch A → B (same tool, a transplant). A probe on B asks for the codename and the open tasks. B then adds `sub` and decides that the multiply function will be called `times`, without writing it.
3. Switch B → Codex (cross-tool, a replay). A probe asks for the codename, the name chosen on B, and the open tasks. Codex then writes `times`, and says a release name, `Copper Kettle`, only in its reply and never in a file.
4. Switch back to A, where the work started (past B). A probe asks for the codename, the release name and what happened meanwhile. Only the conversation can know the release name.

After every step the project's tests must pass. A step it can't verify fails: nothing passes by default.

With `AIJ_LAB_PROBE=<name>`, the lab also checks the switch's set-up step (AJ-E1542). Account A alone has a probe MCP server and a probe skill of that name. The server is a local standard-library script whose tool answers `KESTREL`, and only with its environment carried. The skill's word is `PEREGRINE`. Before each switch the lab sets up only those two on the target. It then asks the target for both words in a fresh session: the switched conversation already holds them, so asking there would prove nothing (lab 6 did, and is not counted). On the way back, A must not be offered what it already has. At the end, every set-up is undone.

## Runs

Labs 1–8 on 2026-10-22, labs 9–11 on 2026-10-23; all on Claude Code 2.1.302 and codex-cli 0.163.0, accounts A and B (two Claude subscriptions) and the Mac's Codex home.

| Lab | Steps | Result | What it found or proved |
|---|---|---|---|
| 1–4 | 1–4 | FAIL, then fixed | A switch of a session started outside AI Juggler couldn't find its file (now found by conversation id). Switching back reached only the last hop, its update was empty, and it promised a pack that never came (now the whole chain, with every hop's turns). Codex's older rollouts are compressed (`.jsonl.zst`) and were skipped (now read). |
| 5 | 1–4 | PASS | Every probe answered from the conversation. On A: "The codename is BLUE-HERON and the release name is "Copper Kettle"; meanwhile `sub` and `times` (mul) were added with tests…" |
| 6 | 1–5 with the probe | Not counted | Every step passed, but Codex's set-up check was asked inside the switched conversation, and Codex answered from it with its tool call blocked. Fixed in the lab. Also found: Codex's config backup outlived the undo (fixed: the last undo removes it). |
| 7 | 1–5 with the probe | 7 of 8 PASS | B, from a fresh session: "KESTREL PEREGRINE". Codex's check didn't run, because a lab flag clashed (`--approve-for-me` refuses `-s`; fixed). After the undo, no backup was left in the Codex folder. |
| 8 | 1–5 with the probe | 7 of 8 PASS, then fixed | Codex, from a fresh session: "KESTREL PEREGRINE". So the server, translated into Codex's `config.toml`, and the skill, in the shared `~/.agents/skills`, both work there. The switch back to A failed. The probe's own Codex session had begun in the same folder after the replay, so finding the replay's conversation by time alone was ambiguous. Switching back then left the Codex hop out without a word (A: "I can't see any release name being picked"). Fixed: the replay's conversation is now found by its opening message, and an account whose conversation can't be found is named in the update (`codex_switch` tests `switching_back_finds_the_replays_conversation_beside_another_one` and `switching_back_says_when_it_cant_tell_which_conversation`). |
| 9 | 1–4, 6, 7 (2026-10-23) | Every check PASS; two set-up rows recorded by mistake | The handover file (AJ-E1541): Codex started from the pointer prompt and read `switches/<edge>/HANDOVER.md`. The project now has a `CLAUDE.md` and an `AGENTS.md`, each with a rule only it states. B: "`CLAUDE.md` says every Python function's docstring must start with `LAB:`". The first turn called a subagent: its folder travelled to B. Codex: "Codename: BLUE-HERON; multiply function: `times`; open task: mul; every Python function's docstring must start with `LAB:`" (Claude's rule, through the handover). Step 6, a forward Codex → Claude hop (a task begun on Codex): B: "The second codename is OSPREY, mod is the only open plan item, and AGENTS.md says every unittest test method name must end with `_lab`". Step 7, in a git worktree: B continued in the worktree (its `neg` landed there, not in the root), then switching straight back gave A "The next helper was decided to be named `flip`". The two set-up rows (2b, 3b) were recorded without the probe; fixed: they're recorded only with it. |
| 10 | 1–4, 6, 6b, 7 (2026-10-23) | PASS | Lab 9's journey again, all PASS, plus 6b: B said a release name only in its reply, and switching back to the Codex task gave "Silver Finch". |
| 11 | 1–4, 6, 6b, 7, 8 (2026-10-23) | PASS | Lab 10's journey again, all PASS, plus step 8, Codex → Codex: a task begun in the Mac's Codex home switched to a second Codex home the maintainer signed in for the lab (`AIJ_LAB_CODEX_B`; its own sign-in, no `auth.json` copied, read beyond the product's field-name check, or moved; the snapshot shows neither home's `auth.json` changed). The rollout was installed there and resumed by id: "The third codename is PLOVER; power is the only open plan item". |

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 an env-gated lab over a throwaway data folder and project | The test refuses a non-empty data folder or an existing project folder, and refuses to run with `CLAUDE_CONFIG_DIR` or `CODEX_HOME` set. `just switch-lab` runs it. | PASS |
| Req.2 the journey, with probes only the conversation can answer | Labs 5, 7 and 8, steps 1–4; labs 9 and 10, steps 1–7 (above). The release names are never written to a file; each tool's rule is in its own rules file only. | PASS |
| Req.3 versions, commands, probes and answers recorded; register cells | Each report records the tool versions, the switch's launch command line, every probe and its answer. Eighteen cells are PASS (`run: live`, Claude Code 2.1.302, codex-cli 0.163.0). Claude→Claude: transcript-resume and todos (labs 5, 7–10), mcp-references (B in labs 7 and 8), rules-carry, subagents, cwd-worktree-edges and switch-back (labs 9 and 10). Claude→Codex: transcript-resume and todos (labs 5, 7–10), mcp-references (lab 8), switch-back (labs 5, 7, 9, 10), rules-carry (labs 9 and 10). Codex→Claude: transcript-resume, todos and rules-carry (labs 9–11), switch-back (labs 10 and 11). Codex→Codex: transcript-resume and todos (lab 11). Lab 14 (Claude Code 2.1.310, codex-cli 0.163.0) re-decided both cross-tool pairs: Claude→Codex every area (six PASS; subagents and file-history FAIL with drop notes), Codex→Claude six PASS (subagents and file-history stay UNVALIDATED). They're in `crates/juggler-handover/data/fidelity-register.json`, rendered in `docs/qa/evidence/AJ-E808/register.md`. A tool upgrade resets them by itself. | PASS |
| Req.4 before and after snapshots; only allowed differences | Paths, sizes and modification times of A's, B's and Codex's folders, never contents. Every difference in labs 5–10 is the lab's own session files (`projects/`, `file-history/`, Codex `sessions/`), the approved set-up and its undo, or the tools' own housekeeping from being run at all (remote settings, policy limits, logs, caches, Codex's databases and bundled skills). After each probe run, A's and B's MCP servers and skills, Codex's `config.toml` and the shared `~/.agents/skills` were compared with copies taken before. They matched, after removing one line Codex writes itself: it trusts the lab folder. | PASS |
| Req.5 no sign-in, no credentials, nothing outside the lab folder | The lab only runs `claude -p` and `codex exec` in the lab project. The interactive session each switch starts is stopped as soon as its command line has been read. The snapshot never reads contents. | PASS |

The reports themselves hold real paths and stay out of the repository.

## Labs 12–14 (0.1.7, 2026-10-26): a source another app runs, and the old path again

The maintainer's requirement for 0.1.7: prove the switch delivers what §7 promises (context, chat, history, rules, tasks, the screen's list, lineage), not only that it switched. New in 0.1.7: a switch whose source is a session another app runs (its key `conversation:<account>:<id>`, AJ-E2108), and a changed resolver for a session AI Juggler runs.

**Harness.** `switch_lab_outside_live` in the same test file, run by `just switch-lab-outside <root> <claude profile> <codex home> <report>` (`AIJ_LAB_SUITE=outside`; the same refusals: an empty data folder, a new root, no `CLAUDE_CONFIG_DIR` or `CODEX_HOME`). One Claude profile only (bookauth-luka, in place) and the Mac's Codex home (in place), `DISABLE_AUTOUPDATER=1`, at most two real tool processes at once (a turn, or the interactive start a switch launches, which is stopped as soon as its command line and whether it asks to trust the folder are read). Every first turn runs in a process the lab owns, outside AI Juggler (`claude -p --session-id`, `codex exec`); AI Juggler finds it in its conversations index and switches it by its key. The redaction policy is set to Strip before any switch, and every first turn plants a fake key, `sk-lab-NOT-A-REAL-KEY-…`. With `AIJ_LAB_PROBE` the source side gets a probe MCP server (its tool answers `KESTREL`, only with its environment) and skill (`PEREGRINE`) through the tool's own `mcp add`; the switch's set-up step must offer and set up only those two on the target, a fresh target session must answer both words, the way back must not offer them, and every set-up is undone. A control turn on the target tool alone, with no handover, must not know the rule the other tool's rules file states.

Claude Code 2.1.310, codex-cli 0.163.0.

| Lab | Result | What it found or proved |
|---|---|---|
| 12 | J1 PASS; J2, J3 not reached (harness) | J1 passed every check. The harness read the conversations index once; the app's background pass, which the lab doesn't run, is what reads a conversation begun later (fixed in the lab: it refreshes the index). Found in J1's facts, then fixed with regression tests (`tests/switch_from_outside.rs`, failing before): History › Replay named the source by the account's id ("Arrived from claude-code-888…"); back from Codex the one delta turn said "0 shared before the switch, 14 new here on Lab Luka" for an original that hadn't moved (a replay shares no message ids, so the whole original counted as new); the switch graph showed the outside original as "no transcript" with Resume off while switching back resumed it; the screen listed "Doesn't carry: skills & commands" while its own set-up step carried the probe skill to Codex and Codex used it. |
| 13 | 13 of 15 steps PASS | J1, J2.1–2.2 and J3 passed. J2.3 delivered (Codex answered "Codename: TERN; release: Quiet Harbor; open plan item: mul") but the lab counted Codex's user turns by an event 0.163 no longer writes (fixed in the lab: one user `message` item per turn). Found and fixed with tests: an exported pack said "dirty — diff snapshotted" over an empty `diffs/` (`pack_v1.rs`, `worktree.rs`); every Codex step read "used exec" in a handover and a delta, since 0.163 runs each step as one code-mode script (`tool_summary.rs`). |
| 14 | PASS, every step | The run below. |

### Lab 14, step by step

J1: a Claude conversation started outside AI Juggler → Codex → back to Claude (project folder).

| Step | Probe or check | Answer, excerpt | Result |
|---|---|---|---|
| J1.1 outside Claude task | Codename MAGPIE, `calc.py` `add` + test, a task list add/sub/mul (add done), a subagent counts README.md's lines | "README.md has 3 lines… Task list: add is marked done; sub and mul are still pending." Listed as Lab Luka's, with no AI Juggler session; a subagent folder exists. | PASS |
| control | Codex alone: "what do this project's rules say about docstrings?" | "NONE" | PASS |
| J1.2 switch Claude→Codex | "codename, open tasks, the rules on docstrings, the dummy key exactly?" | "MAGPIE; open tasks: sub, mul; every Python function's docstring starts with `LAB:`; dummy key: REDACTED." Codex then wrote `sub` with a `LAB:` docstring and `test_sub_lab` (both rules obeyed; the docstring one only in `CLAUDE.md`), tests pass, and said the release name "Amber Lantern" only in its reply (in no file). Fresh Codex session: "KESTREL PEREGRINE". | PASS |
| J1.3 switch back to Claude | the delta, then "codename, release name picked while away, open tasks?" | "Codename MAGPIE; the release name picked while I was away is "Amber Lantern"; only mul is still open." The delta opens "While you were away on Lab Codex — 5 new turns (28 shared before the switch)" and names what Codex ran and edited. | PASS |

J2: a Codex conversation started outside (`codex exec`, in a git worktree) → Claude → back to Codex.

| Step | Probe or check | Answer, excerpt | Result |
|---|---|---|---|
| J2.1 outside Codex task | Codename TERN, `add` + test, a plan add/sub/mul (add done) | "Remembered: TERN… `test_add_lab`… [x] add [ ] sub [ ] mul". Listed as Lab Codex's, no AI Juggler session. | PASS |
| control | Claude alone: "what do this project's rules say about test method names?" | "NONE. The project's rules don't say anything about test method names." | PASS |
| J2.2 switch Codex→Claude | "codename, open plan items, the rules on test method names, the dummy key exactly?" | "Codename TERN; sub and mul are still open (add is done); AGENTS.md says every unittest test method name must end with `_lab`; the dummy key is REDACTED". Claude then added `test_sub_lab` (the `AGENTS.md` rule) in the worktree, not the root, tests pass, release name "Quiet Harbor" only in its reply. Fresh Claude session: "KESTREL PEREGRINE". | PASS |
| J2.3 switch back to Codex | the delta, then "codename, release name, open plan items?" | "Codename: TERN; release: Quiet Harbor; open plan item: mul." Delta: "10 new turns (3 shared before the switch)". | PASS |

J3, the old path: the Claude conversation (in a git worktree) picked up with Continue in AI Juggler, so the source is AI Juggler's own session → Codex → back.

| Step | Probe or check | Answer, excerpt | Result |
|---|---|---|---|
| J3.1 task, continued in AI Juggler | Codename WREN, `neg` + test, tasks neg/absval | The conversation lists as AI Juggler's session; the continue launched `claude --resume <id>`. | PASS |
| J3.2 switch Claude→Codex | "codename, open tasks, the dummy key exactly?" | "Codename: WREN; open task: absval; dummy key: REDACTED." The session's missing transcript link was found again by conversation id; it rests as handed over. Codex added `absval` in the worktree only; the milestone "Velvet Comet" only in its reply. | PASS |
| J3.3 switch back to Claude | the delta, then "codename, next milestone?" | "The worktree's codename is WREN, and the next milestone is called "Velvet Comet"." | PASS |

### Each area, and what the user sees

| Check | Evidence (lab 14) | Result |
|---|---|---|
| transcript-resume | Every codename and every decision said only in a reply (Amber Lantern, Quiet Harbor, Velvet Comet) was answered on the other side; none is in any file. | PASS, both directions |
| rules-carry | Each rule obeyed on the tool that doesn't read its file (`sub`'s `LAB:` docstring on Codex, `test_sub_lab` on Claude), and each control turn answered NONE. | PASS, both directions |
| todos | Open items answered after every switch (Claude's task list via its task tools and the handover's task list; Codex's plan as conversation text, since `codex exec` 0.163 offers no plan tool). | PASS |
| mcp-references, set-up | Offered exactly the probe's server (`copy`) and skill (`translate` into `~/.agents/skills`; `copy` into Claude's `skills/`), both set up, a fresh target session used both, the way back didn't offer them, every set-up undone. | PASS, both directions |
| cwd-worktree-edges | Each new session's folder is the source's (J1 the folder, J2 and J3 a git worktree); work landed in the worktree, not the root; each switch back resumed there. | PASS |
| switch-back | The original's own id resumed (`claude --resume <id>`, `codex resume <id>`) on its own account; exactly one delta turn in the original afterwards; its earlier bytes untouched (appended to, never copied over); one file for it; the graph's back edge is a native resume to the original's epoch. | PASS |
| subagents | Claude → Codex: the source used a subagent; the screen lists "Doesn't carry: subagent state". | FAIL with drop note (by design) |
| file-history | Claude → Codex: the source has Claude's file checkpoints; they stay; the screen says "Tool state and subagents don't carry". | FAIL with drop note (by design) |
| The source only read | The outside original's file hashed before and after the switch, the undo and the probes: byte-identical. | PASS |
| The screen's list | Carried: chat transcript, rules & guidelines, uncommitted git diff, MCP config, skills & commands, model settings; rebuilt: todo list; doesn't carry: subagent state. Everything listed as carried carried; the one drop is listed. The door is the wizard ("This switch replays…", "This move hasn't been proven…"), the words "Full story, fresh start", and for an outside source the screen says it stays as it is. | PASS (after the lab 12 fix) |
| Secrets | Redaction line "Strip — replace keys with ••• (your global choice)"; one `sk-key` item, masked `l•••5f60`; the handover holds `sk-•••`; every target answered REDACTED; the edge's manifest records `redaction/policy: strip`; the exported pack's conversation, PACK.md and patch hold no key, its state records `sk-key: 1`. | PASS |
| Pack (ADR-0009) | Exported from J3's source epoch: PACK.md, conversation.md, state.json, manifest.json, and `diffs/uncommitted.patch` holding both changed files ("dirty — diff snapshotted", now true). | PASS (after the lab 13 fix) |
| Lineage | The edge: replay, confirmed, from the original's epoch (no session row, its account and native id). History › Replay: "Arrived from Lab Luka (Claude Code)", "Arrived from Lab Codex (Codex)", "Moved to Lab Codex (Codex)". The graph: Lab Luka on Claude Code, resumable, then Lab Codex on Codex, "Full story, fresh start". | PASS (after the lab 12 fixes) |
| The conversations list | The outside original stays the other app's (no AI Juggler session) after the switch; the new session is AI Juggler's; after switching back the original is AI Juggler's session (it now continues it). | PASS |
| Undo, outside source | Refused in words ("The original is a session another app runs, and it's still there, untouched…"); the original byte-identical. | PASS |

**Snapshots.** Paths, sizes and modification times of the Claude profile, the Codex home, `~/.agents` and `~/.claude.json`, never contents. Every difference is the lab's own sessions (`projects/`, `file-history/`, `tasks/`, Codex `sessions/`), the probe set-up and its undo, or the tools' own housekeeping (backups, caches, databases, Codex's bundled skills, the trust lines Codex writes for the lab folders). The probe's server and skill are gone from both sides; Claude Code keeps its own use count of the skill (`skillUsage`) in the profile's state. Another session was using the same Claude profile during the lab; the probe server was in its user settings for the minutes J1 and J2 ran. The lab folders were moved aside afterwards.

**Not run live here: Claude → Claude across two Claude accounts.** Only one lab Claude profile is allowed now, so that pair was not run on 2.1.310. Its cells are recorded against 2.1.302 and read UNVALIDATED on the installed 2.1.310 (the app compares versions). Codex → Codex wasn't re-run either.

## Not covered here

- **Codex → Codex, beyond the transcript and its plan.** Lab 11 decided transcript-resume and todos; rules, MCP, the folder and worktree edges and switch-back between two Codex homes stay UNVALIDATED (the second home is signed out and removed after the run).
- **File history, Claude → Claude.** In labs 1–11 A's headless first turn left no file history; on 2.1.310 it does (lab 14), but Claude → Claude wasn't run (one lab profile), so that cell stays UNVALIDATED; it travels whole in the adapter's test (`a_session_with_its_folder_and_file_history_travels_whole`).
- **Cross-tool subagents and file history, Claude → Codex** are decided by lab 14 as FAIL with their drop notes. **Codex → Claude** subagents and file history stay UNVALIDATED: the Codex source has neither to carry.
- **Codex's plan as a task list.** `codex exec` 0.163 offers no plan tool, so a Codex source's open items travel as conversation text; the handover's task-list section is proven for a Claude source only.
- **The session the switch starts.** The lab stops each interactive start once its command line is read, and asks the same tool and account headless (as labs 1–11 did). The Codex target's own conversation is therefore never linked to its session row in the lab (its graph node reads not resumable there).
- After labs 1–11 the project went to the Trash; labs 12–14's folders were moved aside.
