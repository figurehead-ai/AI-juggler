<!-- captures: 38ac749c25ec -->
# 29-door: the one-click door into a switch: the switch dialog from one click, the wizard's Target and What's carried steps (3- and 5-step), and the Undo toast after switching

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The wizard steps are tidy and consistent: step chips with the current one tinted, a large title, What goes along as neutral chips and Stays behind in an amber box, and the dark dialog sits well above the dimmed page while the light one is a crisp white panel. wizard-target stacks the same warning three times (the Beta · Not proven yet chip plus two amber boxes, "This move hasn't been proven on a real setup yet" and "Doesn't carry: Subagents' work"), which dilutes the one that matters. The footer primaries ("Continue", "Switch now") are still pale tinted buttons rather than the solid violet primary, so in oneclick-popover the main action is weak. In undo-toast the user lands on a page titled just "Session" with "Unavailable · Nothing to show here yet", and at 800x600 the toast covers the footer line ("process monit…"); the 1280 rail shows the known cut age on "Stopped with an error · 7…". wizard-3step-carried and wizard-5step-carried differ only in the step chips.

## Weak

- undo-toast-1280x800-*.png, undo-toast-800x600-*.png: after the switch the page shows a generic "Session" title and "Unavailable"; the toast is the only sign of where the work went.
- undo-toast-800x600-dark.png, undo-toast-800x600-light.png: the toast covers the footer text.
- wizard-target-*.png: three stacked warnings on step 1.
- oneclick-popover-*.png and all wizard captures: the primary is a pale tint, not the solid violet primary.
- undo-toast-1280x800-*.png: rail row "Stopped with an error · 7…" loses its age.
