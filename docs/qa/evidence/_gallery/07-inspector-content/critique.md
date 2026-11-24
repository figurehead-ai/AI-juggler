<!-- captures: a1a5a469eac3 -->
# 07-inspector-content: a session page's Overview and Usage tabs, its header menu, and an untitled session that has ended

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

instance-usage reads well as a grid of raised cards (Model, Its account's window, then Tokens / min, CPU and memory, Errors, Context, Session cost, How long its limit lasts) with small grey source lines under each value. Its copy still contradicts itself: the top card says "No token counts for it yet" while Tokens / min shows "406 a minute" and Context "6,000 tokens"; Errors reads "1 errors / 12 turns"; and Session cost is priced "for Opus" while the Model card says sonnet-5. session-header-actions opens the header's more menu with icons, disabled items explained underneath ("It has no conversation to carry over yet") and a red "Stop this session" last; the menu is now wide enough that "Continue on another account…" shows whole beside its ⌘⇧H shortcut. instance-overview-populated and session-overview have clear raised cards and a black terminal; after the second pass the terminal's black reaches its card's edge (the lighter frame around it is gone) and its text sits on a black margin instead of at the edge, as it does in session-header-actions too; in session-overview the rail is scrolled so its search field and tabs are out of view and the sacrificial header is a cut sliver at the top. The lighter dark accent shows on the logo mark, the selected tab, the terminal cursor and the rail's New session button; the top bar stays one row, and the rail's error rows read "Stopped with an error" with no cut age. All captures were re-rendered; the second pass changed only the terminal in instance-overview-populated, session-header-actions and session-overview.

## Weak

- instance-usage: "No token counts for it yet" beside token figures; "1 errors"; cost "for Opus" on a sonnet-5 session.
- session-overview: the rail is scrolled; the sacrificial project header is cut to a sliver at its top.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured with the family's four states; opened at full size: instance-usage (dark). Beside the session page of 05-session-panel and the Code-view row of §5.12. instance-usage is a calm grid of tone cards (Model, Its account's window, then How it's going's six cards with their source lines); session-header-actions' menu and instance-overview-populated / session-overview are unchanged but for the rail (F53's lens, F55's row ends).

- **Fixed** (51ea3254f): the Usage tab opened on "No token counts for it yet" above a card reading 406 tokens a minute and a context of 6,000 tokens. It now reads "No token totals from its app yet" and says How it's going, below, is read from the session itself; re-rendered (gate 09-quality, 26-quota) and re-captured.
- **Closed since 0.3.2:** "1 errors" reads "1 error / 12 turns"; the cost card that priced a sonnet session "for Opus" is gone.
- **Weak, kept:** session-overview still opens with the rail scrolled (a capture framing, the product unchanged). The red "Stopped with an error yesterday 6:26 PM" chip is red's soft step, one block in the header (COL-5 allows it).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | What the session used and how it's going, in one tab. |
| 2 | Prominence | 4 | The totals line, the model and its account's window, then the live cards. |
| 3 | Default | 4 | Chart by default with View as table; every 2 seconds. |
| 4 | Mode | 4 | Calm: tone cards, muted source lines. |
| 5 | Mock-up | 4 | SPEC §9's Usage tab; no mock of its own. |
| 6 | Areas | 4 | Three bands: totals, model and window, How it's going. |
| 7 | Colour | 4 | Red only for the stopped state; violet only on the Resume primary and the small bars. |
| 8 | Type | 4 | Labels muted, values ink, notes faint; one size per role. |
| 9 | Actions | 4 | Resume the one filled button, "Fresh conversation" / "Same conversation" under Relaunch and Resume. |
| 10 | Serious | 4 | Relaunch and Resume say what each keeps under them. |
| 11 | Doors | 4 | How it's measured › and the account's name open their pages. |
| 12 | Words | 4 | After the fix the totals line no longer contradicts the live figures. |
| 13 | States | 4 | "Collecting first samples…", "Not measured yet", "Not reported by Claude Code": each unknown said once. |
| 14 | Consistency | 4 | The same cards as 09-quality and 26-quota. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | No 0.5.1 task carries PAT-USE answers for this tab; re-read against the six questions: names match what they show, every unknown is said, nothing duplicated. Match. |

Lowest: 4 (every line 4 or 5).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Rail rows carry a model mark where the model changed (F77). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
