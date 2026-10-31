<!-- captures: 236a2d6097fc -->
# 79-pages: account, project, session and settings pages at 1024 and 1280

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

account, session-usage and session-terminal separate their cards from the canvas in both themes; the Limits card, the four usage tiles and the "Its account's window" card read as distinct areas, and the session's chip row (Working · 1 min, claude) sits clearly under the title. At 1024x680 the account, session and tool pages fold the rail to marks and keep the main content full width; the bottom cards are cut by the window with a "More below" cue, which is acceptable. outside-session puts Go to it as the solid violet primary, but in the 1280 rail the selected row's hover buttons still overlap its title and "Idle". tool shows Settings › Accounts (Accounts and profiles, Tools on this computer with each tool's account count and version, Renewal dates); its renewal date fields are unstyled native "mm/dd/yyyy" inputs that start wherever their label ends, so they don't line up. project-overview-1024x680 and project-sessions-1024x680 are now re-rendered on the 0.3.2 shell: they keep the full Project rail at 1024, and the header stays one row by shortening the clean-up pill to a broom and "7" and dropping the This Mac line; the page has its breadcrumb, "1 asks you" and "1 working" chips and New session here, Overview shows four raised tiles (Open sessions 4, Tasks 0, Tokens this week 36M with its sparkline, Accounts used 7) over the Activity chart, and Sessions is a raised list with Working, Asks you and Idle in their colours. In dark the lighter accent shows on the logo, New session, the selected tab and the sparkline. In the second pass the rail's payments-api row keeps its time whole ("payments-… · 1 min ago" where it read "payments-api · 1 min a…") in account, outside-session, session-usage, tool and session-terminal, though a wide gap is left between the "…" and the "·"; and session-terminal's black now fills the terminal card to its edge (the lighter dark frame and the white light frame are gone), so "claude · session" sits inside a black margin instead of flush at the edge.

## Weak

- outside-session-1280x800 (both themes): the selected rail row's hover buttons overlap its title and "Idle" state.
- tool-1024x680, tool-1280x800: renewal dates are native date inputs, unlike the app's styled controls, and they don't align in a column.
- project-overview-1024x680, project-overview-1280x800: the "Accounts used" tile cuts its list to "claude, claude-client-acme, claude-client-globex, cl…".
- account-1280x800, outside-session-1280x800, session-usage-1280x800, tool-1280x800, session-terminal-1280x800 (both themes): the rail's payments-api row reads "payments-…  · 1 min ago" with a visibly wider gap after the "…" than the other rows' " · ".
- project-overview-1024x680, project-sessions-1024x680: rail rows cut their meta at "Idle · outside AI Juggler ·…".
