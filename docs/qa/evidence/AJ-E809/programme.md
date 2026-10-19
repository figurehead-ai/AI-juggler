# AJ-E809 — beta dogfood programme

The programme that decides beta exit from behaviour, not downloads. It
runs four weeks with real developers on their own machines and
accounts. The maintainer runs it; agents only tally the run sheets.
Each run is one row in [`runs.csv`](runs.csv) (format in
[`run-sheet.md`](run-sheet.md)). `scripts/dogfood_report.py` turns the
rows into [`report.md`](report.md), which the beta gate packet reads
(criteria B2 and B3).

## Cohort (Req.1)

At least five active users every week for four weeks, covering the three
personas from `docs/review/deep/07-personas-and-journeys.md`:

| Persona | Who | Recruit from |
|---|---|---|
| `maya` | Solo developer paying for two or more Claude/Codex subscriptions | Indie and freelance communities |
| `devon` | Power user running ten or more parallel sessions | Former Vibe Kanban / Crystal / Conductor users |
| `priya` | Consultant keeping client accounts apart | Agencies, contractors |

Personas are composites. They earn or lose their claims here, and the
persona files change only from recorded runs. A participant is a
pseudonymous ID (`P01`, `P02`, …). The key between IDs and people
stays with the maintainer, outside the repository.

## Consent and privacy (Req.5, AJ-E801 checklist)

Before the first session, each participant agrees in writing to:

- [ ] Recorded fields only: the run-sheet columns, nothing else.
- [ ] No credentials, tokens, API keys, account emails or transcripts,
      ever. Never paste from a terminal into a note.
- [ ] Friction notes are their words, with project names, paths,
      company and client names replaced by `[project]`, `[path]`,
      `[client]`.
- [ ] Screen captures only at the switch and cleanup moments, only with
      consent for that session, 1280x800 where possible. Account and
      project names are blurred before anything is stored. Files are
      named by task and date, never by person.
- [ ] Their data comes out on request: rows and captures under their ID
      are deleted.

The maintainer checks each new row against the list above before
committing it. `just evidence` scans the store for credential patterns.

## The scripted tasks (Req.2)

Each task is run on the participant's real work, unaided first. Help
given counts the run as aided. Time runs from the trigger to the
success condition.

| Task | Trigger | Success | Also record |
|---|---|---|---|
| `quota-wall-switch` | An Instance hits its usage limit (or is quota-blocked) | The conversation continues on another Account through *Continue on another account*, and the target answers a question that needs pre-switch context | Minutes; what the participant read before confirming |
| `switch-back` | The original Account's limit resets after a `quota-wall-switch` | Back on the original Account (A→B→A), with the work done on B arriving as the "while you were away" update | **continuity**: does the participant judge the thread intact? (`yes`/`no`) |
| `zombie-cleanup` | A process (dev server, watcher) outlives its Instance | Found in Cleanup and stopped; its port is free afterwards | **false_kill** = `yes` if anything stopped that the participant didn't mean to stop. Any false kill blocks beta exit |
| `cross-provider-switch` | The participant wants to move a conversation from Claude Code to Codex (or back) | The target runs with what carried. Before confirming, the participant saw the pair's label ("Beta · Not yet validated") and the drop list | Did they read the drop list? Their words about it |
| `fleet-join` | A second Mac is available | Paired through the pairing wizard; the other Mac's Instances show on this one | Minutes; any security prompt that confused them |

## Thresholds (Req.3)

- Unaided completion ≥ 80 %.
- A→B→A continuity judged successful ≥ 80 %.
- Zero false cleanup kills.
- Median switch completion < 2 min.

`report.md` shows each one as MET or NOT MET.

## Findings (Req.4)

Every friction note becomes a task within 48 hours, linked in the row's
`filed_task` column, with the persona and evidence in the task. The
report lists unfiled notes and stays NOT MET while any remain.
Proposals marked `[inferred]` (pass2/07 W5, W6, W11) aren't built until
runs here show the demand.

## Weekly cadence (≤ 45 min per participant, ≤ 2 h analysis)

1. Monday: send each participant the week's tasks. Not every task fits
   every week; quota walls happen when they happen.
2. During the week: participants run tasks as their work produces the
   triggers, and the maintainer fills a row per run.
3. Friday: `python3 scripts/dogfood_report.py`, then commit `runs.csv`
   and `report.md` with the week in the message. File the friction
   tasks, and post the week's numbers where the cohort can see them.
4. After week 4: rebuild the beta gate packet (`just gate-packet`). B2
   and B3 read this report.

The live runs listed in [`docs/qa/dogfood-backlog.md`](../../dogfood-backlog.md)
ride the same sessions, and their evidence goes to each task's own
folder.
