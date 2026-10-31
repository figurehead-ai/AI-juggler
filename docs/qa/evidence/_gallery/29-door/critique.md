<!-- captures: e631c979154b -->
# 29-door: the one-click door into a switch: the switch dialog from one click, the wizard's Target and What's carried steps (3- and 5-step), and the Undo toast after switching

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The wizard steps are tidy and consistent: step chips with the current one tinted, a large title, What goes along as neutral chips and Stays behind in an amber box, and the dark dialog sits well above the dimmed page while the light one is a crisp white panel. The footer primaries ("Continue", "Switch now") are now solid violet in every wizard and oneclick-popover capture, so the main action clearly leads Back and Cancel; in dark the current step chip, the radio and the "What does this mean?" links carry the lighter violet. At 800x600 the page behind now keeps its top bar on one row: undo-toast shows the cleanup pill as a broom with "7" and drops the This Mac line, where the first pass wrapped the bar to two rows. wizard-target still stacks the same warning (the "Beta · Not proven yet" chip plus two amber boxes, "This move hasn't been proven on a real setup yet" and "Doesn't carry: Subagents' work"), which dilutes the one that matters, and in oneclick-popover "Before you switch" and "When to stop the original" are two headings stacked with nothing between. In undo-toast the user lands on a page titled just "Session" with "Unavailable · Nothing to show here yet", and at 800x600 the toast covers the footer line ("process monit…"). The 1280 rail now shows "Stopped with an error" whole. wizard-3step-carried and wizard-5step-carried differ only in the step chips.

## Weak

- undo-toast-1280x800-*.png, undo-toast-800x600-*.png: after the switch the page shows a generic "Session" title and "Unavailable"; the toast is the only sign of where the work went.
- undo-toast-800x600-dark.png, undo-toast-800x600-light.png: the toast covers the footer text.
- wizard-target-*.png: three stacked warnings on step 1.
- oneclick-popover-*.png: "Before you switch" and "When to stop the original" are two headings in a row with no content between them.
