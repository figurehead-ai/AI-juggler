# Evidence manifest — AJ-E1534

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1534, orientation (a 0.1 P0, from
the coordinator's review, and QA's P24). Status: **IN-REVIEW** until the
release PR.

Gallery: `docs/qa/evidence/_gallery/79-pages/` (with AJ-E1533): every
type's page with its mark, its rail row selected and the top bar's
title and highlight, at 1280 and 1024, both themes, on the replica.

## What exists now

1. **The rail selects the open item's row, whatever its type**: a
   session, a session started outside AI Juggler, a project's heading,
   and a project with no session yet.
   - Each gets a filled row, the accent bar and `aria-current`, the same
     on all four.
   - The row scrolls into view when the selection changes.
   - Project rows moved to `components/RailProjectRows.tsx`.
2. **The top bar comes from the page shown.**
   - Title: a session's page and an outside session's page are titled
     by their project. The outside one used to read "Home", and nothing
     falls back to "Home" any more.
   - Highlight: an account's page lights Accounts, its section. A tool's
     page lights More, where Tools is. Home lights only on Home.
   - In the code, History lit only History. The "History showed
     Accounts" report didn't reproduce.
3. **Tab memory is per type** (AJ-E1533, `aij.page.tab.<type>`). Each
   page type starts from its own memory, so a session's last tab is
   never carried to a project. Before, the bar showed the previous
   type's tab for one render.
4. **Each type has its own mark**:
   - a session: its terminal glyph with a dot in its state's colour;
   - a project: a rounded square in its colour;
   - an account: its round disc with initials (AJ-E1529's palette);
   - a tool: its own mark (AJ-E1526).
5. **P24: project buttons are named by the project's name.** Their
   folder is only a description, and home-relative (`~/…`, never the
   absolute path): in the rail, New session's picker and the project
   header (`lib/project-words.ts` `homeRelative`, one rule).

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| 1 the rail's selection for every row type | vitest `app-orientation` (a session row and a project heading carry `aria-current` once their page is open); `pages-gallery` (an outside session's row selected and scrolled into view) | PASS |
| 2 the top bar from the page shown | vitest `app-orientation` (Home lit only on Home; a session's page lights nothing and is titled by its project; an account's page lights Accounts) | PASS |
| 3 tab memory per type | vitest `inspector-shell` ("restores a type's last tab, never carrying it to another type") | PASS |
| 4 mark shapes | vitest `app-orientation` (`mark-session`, `mark-project`); `pages-gallery` | PASS |
| P24 named by name, path home-relative | `RailProjectRows.tsx`, `NewSession.tsx`, `ProjectPage.tsx`; axe in `pages-gallery` | PASS |

## Walked on the replica: the three questions

1. **Does it solve the real problem?** You always know where you are.
   - The open item is marked in the rail, even far down the list (it
     scrolls to it).
   - The page names its type and its parent, and the top bar agrees
     with both.
2. **Is it clear without explanation?** The selection uses the same
   accent bar on every row type, the mark tells a session from a project
   at a glance, and nothing says "Home" unless it is Home.
3. **Is it consistent with its siblings?** One selection style for every
   row, one way to light the top bar, and one rule for showing a folder.
   - The project's rounded square here is a first step: AJ-E1532 gives
     it a monogram and a palette colour of its own.
