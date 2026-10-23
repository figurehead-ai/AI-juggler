# Evidence manifest — AJ-E1514

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1514, budget caps (P2).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

Gallery: `docs/qa/evidence/_gallery/89-budget-caps/`, in both themes, with axe showing 0 critical or serious issues:
- Usage's caps;
- the form, for a group of projects;
- Home's Needs you.

## What exists now

1. **A cap** is a name, a USD amount a month, and what it counts: one account, or a group of projects on any account (Req.1).
   - **Core:** `juggler-core/src/budget.rs`, pure. It holds `BudgetCap`, `BudgetScope`, the levels (`level`: under, near at 80 % or more, over at 100 % or more), the form's checks in words (`invalid`), and `spent`.
   - **App:** `features/budgets`. The caps are kept in app settings (`budget_caps`, as JSON), so there's no schema change.
2. **The spend is the cost ledger's own estimate.**
   - `spent` runs `cost_ledger::rollup` over the cap's events, so a cap and Usage's table always agree.
   - Usage that has no price is flagged (`unpriced`) and never counted as $0. "Not priced yet" is shown when nothing could be priced.
   - **Account caps** read that account's events since the month start.
   - **Project caps** map each event's session to its project through the conversations list, so worktrees fold into their repository as everywhere else. They count on every account.
3. **The month is the user's.** The screen sends where its local month starts, because the backend keeps no time zone. A start more than a month back, or ahead, is refused.
4. **Needs you warns at 80 % and 100 %** (Req.2), with Home's headline when a cap is what needs you.
   - **Got it** records the level seen this month (`budget_seen:<id>` = `<month>:<level>`). The item stays hidden until the cap reaches the next level or the month ends.
   - Changing a cap warns afresh.
   - The items' only actions are Got it and See usage. Each says "Nothing is stopped."
5. **Nothing is ever stopped** (the task's quality gate). The commands are one read (`budgets_list`) and three writes, each to the caps' own settings: `budgets_save`, `budgets_remove`, `budgets_seen`.
6. **UI:**
   - `components/BudgetCaps.tsx`: the Usage section (bar, "≈$243 of $300 this month", what it counts, Edit, Remove), the form, and the estimate's basis under the list.
   - `components/BudgetNeeds.tsx`: the Needs you items and the headline.
   - `lib/ipc/budgets.ts`: the calls, the local month start, and the preview seam `__AIJ_BUDGETS__`.
   - Home.tsx stays at its 776-line cap: two imports merged and the bridge types folded onto one line.
7. **The guide:** `docs/guide/usage-and-limits.md` › Monthly caps. It covers where to set one, the warnings, "Nothing is ever stopped", the month, and what the estimate means on a subscription.

The spec's note says Pools (E17) may replace "group of projects". A project cap's scope is a tagged enum, `{"kind":"projects"}`, so a pool can become a third kind.

## Tests

- **`juggler-core` `budget::tests`:**
  - The levels turn exactly at 80 % and 100 %, and NaN and a zero cap read as under.
  - Every refusal of the form, in words.
  - A scope reads back as it was saved.
  - `a_cap_only_warns`.
- **`crates/juggler-app/tests/budgets.rs`**, over real storage, two in-place accounts, transcripts and a priced lab model:
  - `an_accounts_cap_warns_at_80_and_100_percent_of_this_month`: last month and another account aren't counted.
  - `a_group_of_projects_counts_them_on_every_account`: 3 + 4 across two accounts, and the other project's 100 isn't counted.
  - `a_seen_warning_stays_quiet_until_the_next_level`: the next level warns again; a wrong month or level is refused; a changed cap warns afresh; Remove works.
  - `usage_with_no_price_is_flagged_never_counted_as_nothing`
  - `the_form_is_refused_in_words`
  - `a_cap_never_stops_anything`: the never-stop guardrail over the four files. *Planted:* a reference to `state.supervisor` in the feature fails it.
- **`ui/src/components/budget-caps.test.tsx`**, 10 tests:
  - the empty state's words;
  - a row's words, percent and what it counts;
  - the form for a project group (`$1,200` is read as 1200);
  - Remove;
  - "Not priced yet";
  - the local month start;
  - Needs you at near and over, not under, and not once seen;
  - the items' only two actions;
  - Got it;
  - See usage opens Usage;
  - the headline.
- **`ui/tests/visual/budget-caps.spec.ts`:** the gallery above.

## Walked by hand

In the preview, on 2026-10-23:
1. Usage › Set a monthly cap: typed "Client Acme" and "$1,500", chose a group of projects and ticked one, then Save.
2. The form closed and the cap showed as "≈$0.00 of $1,500 this month".
3. Home's Needs you showed the 81 % and over caps with Got it and See usage.

The captures were checked by eye. The first run's fixture used a project id the preview doesn't have, and the rows said "Projects that have been removed", which is the right words for that case. Fixed and recaptured.
