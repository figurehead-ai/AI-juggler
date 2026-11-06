# Evidence manifest — AJ-E3201

Task: `docs/tasks/E32-account-and-app.md` AJ-E3201, one word per thing:
Account, App, Tool, Project, Session, on screen, in the spec and in the
guide. Built 2026-11-02. Checked on 2026-11-06 (release/0.4.1). Status:
**DONE, 1 gap open** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `docs/SPEC.md` §3 carries the D-62 amendment (Account, App, Tool, Project, Session; launch settings stay) and §9.1 its "Amended by D-62 (words)" block. D-35 and D-36 in `docs/review/decision-log.md` carry "Superseded in part by D-62" and "Amended by D-62". The guide (`docs/guide/*.md`) is read by `scripts/terminology_words.py`, which finds no old word in it (run 2026-11-06). |
| R2 | PASS | `scripts/terminology_words.py` reads every string literal and JSX text in shipped `ui/src` code; on 2026-11-06 it found no old word on screen (its two hits are code, see the lint row). Labels in use: `ui/src/components/place-board.test.tsx` "reads Needs you, then Open now by place…" (each App's Open says "Claude Code · Work Max", "Claude app · Personal"); `ui/src/screens/accounts/accounts-grouped.test.tsx` (an Account card lists its Apps as "Claude Code · Work"). |
| R3 | PASS | `place-board.test.tsx` "reads Needs you, then Open now by place…": AI Juggler's session and the ones in Terminal on one folder are one App, "Claude Code · Work Max", not grouped by the terminal; `ui/src/components/outside-conversations.test.tsx` shows the terminal as a detail of the row ("open in Claude · Work and Terminal"). |
| R4 | GAP | The lint flags the old uses: `python3 scripts/terminology_words.py --selftest` prints OK (each rule's bad and good cases, the guide and the UI), and `scripts/terminology_lint.sh` runs it. Missing: the lint is red on HEAD (exit 1, three hits). `docs/SPEC.md:136` "A profile under the system's temporary folder" (D-84's amendment, flagged by the profile rule), and `ui/src/components/RightInspector.tsx:315` and `:345`, code (`props.data.instance`, `provider:`) read as words. The SPEC line needs rewording or an exception; the two code lines need the tokenizer fixed or an exception, with a self-test case. |

Not yet done: scenarios A1–A8 ("read every label") are E33's validation.
