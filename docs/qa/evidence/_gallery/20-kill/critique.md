<!-- captures: f5a80a906a0f -->
# 20-kill: the "Stop vite?" sheet for a process AI Juggler started: choosing what to stop, the 5-second wait, the outcome, and when the PID now belongs to another program

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The verify-* sheets read top to bottom as they should: what was checked, a Details fold, three radio choices with a plain hint each ("The narrowest choice"), the process group in mono rows, an amber "Includes a terminal shell that may hold unsaved state", then Cancel and a red "Stop vite (PID 8123) and free :5173"; the page behind is blurred and dimmed. At 1024x700 the last paragraph ("First each process is asked to close… stopped by force.") is cut in half by the footer in verify-pid, verify-pgid and verify-tree, both themes; the previous render fitted it. The red button keeps the same "PID 8123" label when "Whole session tree" or "This process and its helpers" is chosen, so it does not say what will be stopped. grace-countdown-* end with an empty footer band under a divider, and the dialogs stay the same small size at 1440x900, where their 11–12 px text looks tiny; verification-failed-* say "Juggler will not touch it" beside "What AI Juggler recorded". windows-wording-* swap the last line for "On this system a stop is immediate", which fits. All captures were re-rendered.

## Weak

- verify-pid-1024x700-*, verify-pgid-1024x700-*, verify-tree-1024x700-*: the last paragraph is cut in half by the footer.
- verify-pgid-*, verify-tree-*: the stop button still names only "PID 8123" when a wider choice is selected.
- grace-countdown-*: an empty footer band under the divider.
- verification-failed-*: "Juggler" and "AI Juggler" in the same sheet.
