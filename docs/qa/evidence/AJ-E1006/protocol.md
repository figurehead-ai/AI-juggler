# AJ-E1006 falsification round — protocol

Two workflows are guesses from the research. Nobody has observed them
yet:

- **W5 Pair review.** One Account writes; a second Account (often a
  different Provider) reviews what it wrote.
- **W6 Model arbitrage.** A cheap model explores or drafts; a strong
  model finishes.

This round decides, from what participants actually do, whether either
becomes a build task. A clean "rejected" is a successful outcome.

## Participants

At least 5 dogfood users from the AJ-E809 cohort, each running two or
more Accounts. Record a persona for each (solo developer, consultant,
team lead, …) and nothing that identifies them. At most 2 h per person,
and at most 3 weeks for the whole round.

## Diary (two working weeks)

At the end of each day, the participant answers:

1. Did you ask one Account or model to check work another one did?
   What triggered it, and how did you move the work across (copy and
   paste, a pack, a switch, or something else)?
2. Did you start something on a cheaper model or Account and finish it
   on a stronger one? Why there, and why switch?
3. Which step was annoying enough that you would want AI Juggler to do
   it?

## Interview (30–45 min, after the diary)

- Walk through the most recent time each workflow happened (or why it
  never did).
- What did it cost them: time, tokens, mistakes?
- Ask "Would you use a built flow for this?" only after the
  walkthrough, and do not show a design.

## Coding and decision

- For each participant, code each workflow as *observed habit*,
  *occasional* or *absent*. Keep verbatim quotes.
- **Graduate** a workflow only when 3 or more independent participants
  show it as an observed habit and ask for it. A graduated workflow gets
  a new task ID that specifies pack-based handoff into a review
  Instance, with a diff-first Inspector view. Review is a switch or fork
  with lineage, never a silent copy (D-03/D-05).
- **Reject** otherwise. Close AJ-E1006 as REJECTED, link this packet,
  and keep the feature off the roadmap.
- **Undecided** keeps it out of beta scope.
- Either way the outcome becomes a new entry in
  `docs/review/decision-log.md`.

## Packet

Store it in this folder, one file per participant:
`participant-<n>.md`, holding the persona, the diary entries, interview
notes, codes and quotes. Then write `decision.md`.
