<!-- captures: 68ad4b0377a0 -->
# 79-pages: account, project, session and settings pages at 1024 and 1280

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

account, session-usage and session-terminal separate their cards from the canvas in both themes; the Limits card, the four usage tiles and the "Its account's window" card read as distinct areas, and the session's chip row (Working · 1 min, claude) sits clearly under the title. At 1024x680 the account, session and tool pages fold the rail to marks and keep the main content full width; the bottom cards are cut by the window with a "More below" cue, which is acceptable. outside-session puts Go to it as the solid violet primary, but in the 1280 rail the selected row's hover buttons still overlap its title and "Idle". tool shows Settings › Accounts (Accounts and profiles, Tools on this computer with each tool's account count and version, Renewal dates); its renewal date fields are unstyled native "mm/dd/yyyy" inputs that start wherever their label ends, so they don't line up. project-overview-1024x680 and project-sessions-1024x680 are now re-rendered on the 0.3.2 shell: they keep the full Project rail at 1024, and the header stays one row by shortening the clean-up pill to a broom and "7" and dropping the This Mac line; the page has its breadcrumb, "1 asks you" and "1 working" chips and New session here, Overview shows four raised tiles (Open sessions 4, Tasks 0, Tokens this week 36M with its sparkline, Accounts used 7) over the Activity chart, and Sessions is a raised list with Working, Asks you and Idle in their colours. In dark the lighter accent shows on the logo, New session, the selected tab and the sparkline. In the second pass the rail's payments-api row keeps its time whole ("payments-… · 1 min ago" where it read "payments-api · 1 min a…") in account, outside-session, session-usage, tool and session-terminal, and after a later fix the gap after the "…" is gone and short rows keep their time whole ("blog · 1 min ago"); and session-terminal's black now fills the terminal card to its edge (the lighter dark frame and the white light frame are gone), so "claude · session" sits inside a black margin instead of flush at the edge.

## 0.4, the account page

account (1024, 1280, both themes) is now reached as 0.4 does: from the Account's name on its card on Accounts (a folder's name opens its App). It is the Account page: "c…@example.com (1)" (the replica's Accounts have no plan, so they are named by masked email, told apart by number), "1 app · 4 open", the tabs Overview, Apps 1, Sessions 31, Projects 14, Usage, Settings, its Limits, Its apps (Claude Code · claude with its App's dot), and Open now and Last used in the one row, each naming its App with its dot.

## Weak

- outside-session-1280x800 (both themes): the selected rail row's hover buttons overlap its title and "Idle" state.
- tool-1024x680, tool-1280x800: renewal dates are native date inputs, unlike the app's styled controls, and they don't align in a column.
- project-overview-1024x680, project-overview-1280x800: the "Accounts used" tile cuts its list to "claude, claude-client-acme, claude-client-globex, cl…".
- project-overview-1024x680, project-sessions-1024x680: rail rows cut their meta at "Idle · outside AI Juggler ·…".

## 0.4, AJ-E3103: the one related list and the one row

Recaptured: project-sessions and project-overview, at 1024 and 1280, in both themes. The other captures were left as they were (their changes since the last shoot aren't this task's).

**project-sessions.** The Sessions tab is the one related list. Its tab reads "Sessions 27", the same as its heading "27 sessions · 4 open". Before, the tab said 3 in the preview and counted only AI Juggler's open sessions. A search field and three chips (State, App, Account) sit above the list, since the list has more than eight rows. The rows come in sections: Needs you, Working, Open, Last 7 days. Each row is the one session row:
- the app's icon with the state's dot;
- the title;
- "● Claude Code · claude · 1 min ago" on line 2: the App's colour dot and name, then when. The project is left out on its own page;
- the state word at the line's end.

Before, the state led the second line, and the App sat in a column of its own on wide windows.

**project-overview.** The tab count now reads 27 to match the list. The Open sessions card lists its rows in the order Needs you, Working, Open, with section headings when more than one is present.
