<!-- captures: ee80cf077485 -->
# 07-inspector-content: a session page's Overview and Usage tabs, its header menu, and an untitled session that has ended

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

instance-usage reads well as a grid of raised cards (Model, Its account's window, then Tokens / min, CPU and memory, Errors, Context, Session cost, How long its limit lasts) with small grey source lines under each value. Its copy still contradicts itself: the top card says "No token counts for it yet" while Tokens / min shows "406 a minute" and Context "6,000 tokens"; Errors reads "1 errors / 12 turns"; and Session cost is priced "for Opus" while the Model card says sonnet-5. session-header-actions opens the header's more menu with icons, disabled items explained underneath ("It has no conversation to carry over yet") and a red "Stop this session" last; the menu is now wide enough that "Continue on another account…" shows whole beside its ⌘⇧H shortcut. instance-overview-populated and session-overview have clear raised cards and a black terminal; in session-overview the rail is scrolled so its search field and tabs are out of view and the sacrificial header is a cut sliver at the top. The lighter dark accent shows on the logo mark, the selected tab, the terminal cursor and the rail's New session button; the top bar stays one row, and the rail's error rows read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- instance-usage: "No token counts for it yet" beside token figures; "1 errors"; cost "for Opus" on a sonnet-5 session.
- session-overview: the rail is scrolled; the sacrificial project header is cut to a sliver at its top.
