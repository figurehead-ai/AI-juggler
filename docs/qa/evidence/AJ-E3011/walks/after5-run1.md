# AJ-E3011 done-gate walk, Sam through the diagram path, on the bundled build (2026-11-16)

- **Build:** release/0.4.6 at e687398d5. A clean snapshot was served with `scripts/qa/walk_bundle.sh` (a bundled build, no dev-server compiles); the bridge is from the same snapshot.
- **Lab:** P10, seed state.
- **Tester:** a fresh Sonnet tester with the same persona, pointed at the project; browser tools only. Firewall checked: 69 browser calls.

## Result

**SD**. Ease 4/7, value 4/7, solved partly, feeling +1 ("hopeful, cluttered, uncertain"). The bundled build removed the delay complaints. What is left is layout, on the diagram path.

| # | Step | Worked | Value | Clarity | Feeling | Note |
|---|---|---|---|---|---|---|
| 1 | Projects › web › its Pools tab | 4 | 3 | 3 | 3 | "web's own pool: every session in web, as one team" bridged "team" and "pool" |
| 2 | Open the pool | 4 | 3 | **2** | **2** | a big consent card, a checklist, the diagram and a side panel, all at once |
| 3 | Add Claude, then Codex | 5 | 4 | 4 | 4 | the page jumped after each Add, and one click seemed lost |
| 4 | Who leads › Change › Claude's next session | 4 | 4 | 3 | 3 | the radio list was off-screen in the cramped side panel |
| 5 | Codex card › Its role › Reviewer | 5 | 4 | 3 | 4 | the card was found by chance; the menu first opened off-screen |
| 6 | On their own: kept Work together | 3 | 3 | 3 | 3 | "wake each other" unclear |
| 7 | The question card › Yes | 4 | 3 | 3 | 3 | "costs nothing extra: turns" unclear |

## Opus reading

Across five rounds, the New pool flow (a focused page, one step at a time) has scored at or above the diagram path. The diagram path asks a newcomer to set a pool up in a side panel beside a diagram they can't read yet. The remaining "no" answers ("Everyone in web", Member, Observer, "turns") come from what that crowded page shows at once.

## Round 5 (design call: one flow for setting up)

1. **A pool with no members opens in the guided setup**, the same component as New pool's steps, prefilled with the pool's projects (and its name for a project's own pool). "Show the diagram" is a quiet link. The diagram opens once the setup is made, or for a pool that already has members. A partly set-up pool keeps the diagram's checklist, but each item opens that same step full width, not in the side panel.
2. **No layout shift:** adding a member never moves what's under the pointer (space reserved, or the change animated in place); a test measures layout shift on Add.
3. **A project's own pool doesn't draw an "Everyone in web" card.** Its scope is said in the header ("Every session in web is part of this team"), and the join role is set in the setup's Everyone else step ("New sessions in web join as: Member").
4. **The question card's cost line** reads "It costs nothing extra. Your assistants use their own turns only when they act."
5. The side panel fits a 1280×800 window without scrolling for its first section, and menus open inside the window (the menus audit already guards this).
