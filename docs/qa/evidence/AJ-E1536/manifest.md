# Evidence manifest — AJ-E1536

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1536, History you can scan (P1),
with QA's P20 (History covers every account's sessions). Status:
**DONE-EVIDENCED** (IN-REVIEW until the release PR). Full-text search
over the conversations started outside AI Juggler is AJ-E1538 (opt-in,
TODO), as the coordinator decided (below).

Gallery: `docs/qa/evidence/_gallery/56-history/` (search, archived,
replay, delete) and `_gallery/77-menus/history-menu-*`, both themes.

## What exists now

1. **Each row says what it was about**, with its marks, and opens its
   page.
   - **The title:** the user's name for it, else its conversation's
     title (the tool's summary, else the first thing asked), else its
     branch or folder.
     - AI Juggler's sessions now carry their conversation's title. The
       backend joins it from the conversation index (`InstanceInfo.title`,
       `features::conversations::titled`), so the rail, the pages and
       History read the same (`sessionTitle`).
     - Sessions started outside AI Juggler were already titled this way.
   - **The marks:** its tool's mark, its project's mark and name (the
     shared `components/ProjectMark.tsx`, also the page header's), and
     its account's disc and name.
   - **Opening:** a row opens its session's page, or the outside
     conversation's page.
2. **Actions are in the row's ⋯ menu** (also right-click and Shift+F10,
   AJ-E1537):
   - For AI Juggler's: Open, Replay, Continue on another account,
     Archive or Unarchive, then Delete… last, in its own group.
   - For outside ones: Open, Continue in AI Juggler, Copy resume
     command, Show its project (read-only, D-08).
   - The row's inline Replay, Archive and Delete buttons are gone.
3. **P20: History covers every account's sessions.**
   - It lists AI Juggler's ended sessions and every ended conversation
     started outside it (AJ-E1529's index).
   - Search matches the title, project, account, tool (id or name),
     model, branch and dates for both. Transcript text, with the
     matching words in context, covers AI Juggler's sessions, as before.
   - Archiving stays AI Juggler's: an outside conversation is only ever
     under Ended.
4. **One list.** History is the session list the project and account
   pages use (`pages/SessionsList.tsx`).
   - Every row on those pages gets the same ⋯ menu for what it is.
   - A list across projects shows each row's project.
   - The time column no longer wraps ("10 min ago").

## Decided (P20, full text everywhere): later, opt-in

The coordinator chose option (c) on 2026-10-22:
- **Now:** sessions started outside AI Juggler are found by their title,
  project, account, tool, branch and date.
- **Later, opt-in (AJ-E1538, TODO):** full-text search of their words,
  with Settings showing the index size and a switch. Its trade-offs
  (disk, a copy of all conversation text, privacy) are written up there.
- **History says what it searches**, so a missing text match isn't
  taken to mean there is none:
  - its description says words are searched in AI Juggler's sessions
    only;
  - "Nothing matches" adds that words said in sessions started outside
    aren't searched.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| 1 title, marks, opens its page | vitest `history` ("lists AI Juggler's ended sessions and the ones started outside it, each by what it was about, with its marks"; "a row opens its page") | PASS |
| 2 Delete in the ⋯ menu, last | vitest `history` ("Delete is only in its ⋯ menu, last"; Unarchive through the menu); `menus-gallery` history entry (Open first, Delete… last); `capture` 56-history and a11y (Replay and Delete through the menu) | PASS |
| P20 every account's ended sessions | vitest `history` (an outside conversation listed and found by its title); the preview walk | PASS |
| titles from the conversation index | `features::conversations::titled` on `instances_list`, the session events and the Fleet snapshot | PASS |

## Walked in the preview: the three questions

1. **Does it solve the real problem?** History reads as a list of
   sessions you can scan: what each was about, where, on which account,
   and when. It includes the ones you ran outside AI Juggler, which it
   didn't show before.
2. **Is it clear without explanation?** A row opens what it names.
   Actions are where every other row keeps them, and Delete never sits
   next to a click.
3. **Is it consistent with its siblings?** It is the same row as a
   project's and an account's session lists, with the same marks and
   the same menu.
