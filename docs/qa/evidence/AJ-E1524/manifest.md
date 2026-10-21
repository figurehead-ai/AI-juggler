# Evidence manifest — AJ-E1524

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1524 (a welcome Home for new
installs; the maintainer's request, 2026-10-21). Status:
**DONE-EVIDENCED**. 2026-10-21, reviewer claude-opus.

## Before and after

Before: Home with no accounts was one line and a button
([1280 dark](before-no-accounts-1280x800-dark.png),
[1280 light](before-no-accounts-1280x800-light.png),
[390 dark](before-no-accounts-390x844-dark.png),
[390 light](before-no-accounts-390x844-light.png)).

After: the welcome
([1280 dark](../_gallery/67-home/no-accounts-1280x800-dark.png),
[1280 light](../_gallery/67-home/no-accounts-1280x800-light.png),
[390 dark](../_gallery/67-home/no-accounts-390x844-dark.png),
[390 light](../_gallery/67-home/no-accounts-390x844-light.png),
[1440](../_gallery/67-home/welcome-1440x900-dark.png),
[1024](../_gallery/67-home/welcome-1024x680-light.png),
[compact rail](../_gallery/67-home/welcome-compact-1280x800-dark.png),
[How it works open](../_gallery/67-home/welcome-how-1280x800-dark.png)).

## What exists now

- **`Welcome`** (`ui/src/components/Welcome.tsx`) replaces Home's
  no-accounts block, and so also shows after first run's Not now:
  - the three-balls mark as a calm hero in a tile with the accent glow
    (it rises in once, and doesn't move at all under reduced motion);
  - the headline "All your AI coding accounts, in one calm place.";
  - a lead naming first run's tools from the catalog;
  - a found line from the install probe ("We found Claude Code on this
    Mac.", or "No AI coding tools found yet." with an Install link per
    tool);
  - **Find my accounts** (first run) and **Add a fresh account** (Add
    account on its fresh step, `FirstRun startAt="fresh"`);
  - four benefit cards in Home's card style, and the trust line;
  - "How it works" behind a disclosure, with three numbered steps.
  No new screens and no new palette. There is one keyframe
  (`welcome-rise`) and one icon (`IconShield`), both in the existing
  files.
- **`WelcomePointer`**: one line in Usage's empty state, and in Fleet's
  while no account is in, back to Home.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 where (no accounts; after Not now; Usage and Fleet point back) | vitest `welcome`: zero accounts shows it, one account hides it; the pointer opens Home; `home.test` (Find my accounts opens first run) | PASS |
| Req.2 content (headline, scan line, benefits, trust) | vitest: the found line for installed, partly installed and nothing installed (with install links); four benefits; the trust line | PASS |
| Req.3 actions (Find my accounts, Add a fresh account, How it works) | vitest: each action calls its own flow; How it works is a closed `<details>` with three steps; walked: Add a fresh account opens on "Name the account", and Find my accounts opens first run | PASS |
| Req.4 design (both themes, both densities, 390–1440 px, tokens only) | gallery `67-home` no-accounts at 1280 and 390, welcome at 1440 and 1024, compact rail, How it works open, both themes; regression `welcome-*` (8 cells) | PASS |
| Req.5 accessibility | vitest: one h1, the mark `aria-hidden`, reading order h1 → found line → actions; home-gallery axe 0 critical/serious on every welcome capture; the a11y gate | PASS |

## Walked by hand

The vite dev server on an empty fixture, with no accounts, as a brand-new
user, in dark and light at 1280, 1024 and 390. On first look the
headline says what it's for, the found line says what's here, and two
clear buttons lead in. Tab reaches Find my accounts, Add a fresh account
and How it works, in that order. Add a fresh account opened Add account
on "Name the account"; Find my accounts opened first run. ⌘3 opened
Usage, whose "New here?" line led back to the welcome.

Judged honestly over two rounds. The first hero's mark was too small in
its tile, and a background glow read as a grey smudge in light mode. The
mark is now larger, the glow is gone, and the tile's own accent shadow
carries the warmth. A reduced-motion race that blanked the tile in a
capture is fixed: no animation runs under reduced motion.

## Gates

GATES_PLACEHOLDER

## Not covered here

- The real Mac app's first launch: agent sessions have no Screen
  Recording grant, so it was walked in the browser on the mock bridge.
- The rail's own empty line still says "Start one with New session"
  while no account exists. That's a separate wording question for the
  rail.
