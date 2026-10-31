<!-- captures: 7530d4587a4a -->
# 07-inspector-content: a session page's Overview and Usage tabs, its header menu, and an untitled session that has ended

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

instance-usage reads well as a grid of raised cards (Model, Its account's window, then Tokens / min, CPU and memory, Errors, Context, Session cost, How long its limit lasts) with small grey source lines under each value. Its copy contradicts itself, though: the top card says "No token counts for it yet" while Tokens / min shows "406 a minute" and Context "6,000 tokens"; Errors reads "1 errors / 12 turns"; and Session cost is priced "for Opus" while the Model card says sonnet-5. session-header-actions opens the header's more menu with icons, disabled items explained underneath ("It has no conversation to carry over yet") and a red "Stop this session" last; its "Continue on another accou…" label is cut beside its shortcut. instance-overview-populated and session-overview have clear raised cards and a black terminal; in session-overview the rail is scrolled so its top row is cut under the search field. All captures were re-rendered.

## Weak

- instance-usage: "No token counts for it yet" beside token figures; "1 errors"; cost "for Opus" on a sonnet-5 session.
- session-header-actions: menu item "Continue on another accou…" is cut.
