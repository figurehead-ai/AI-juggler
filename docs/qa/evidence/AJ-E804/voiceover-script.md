# AJ-E804 Req.3 — VoiceOver pass (per release, maintainer)

A manual pass over the rail, the inspector and one wizard with VoiceOver
on macOS, run against the release candidate with a throwaway data folder.
Record the findings in the table at the end and keep this file with the
release's evidence. Automated checks (axe, the keyboard traversals) don't
replace this pass: they can't tell whether announcements make sense when
heard.

## Setup

1. Start the release candidate with a fresh data folder, so no real
   Accounts are touched:
   `HOME=$(mktemp -d) open -n "AI Juggler.app"`. Add one fake Provider
   in onboarding, or use the fixture build's mocked data.
2. Turn VoiceOver on (⌘F5). Set the rotor to Headings and Landmarks.
   Use default verbosity.
3. Keep the mouse away. Every step below uses VoiceOver keys (VO = ⌃⌥)
   and Tab.

## Script

### A. Rail

| # | Do | Expect to hear |
|---|---|---|
| A1 | Tab to the rail's tab list; VO-→ across it | "Projects, tab, 1 of 3", "Instances, N need you, tab, selected, 2 of 3", "Sessions, tab" — never a bare count like "!20" |
| A2 | Open Instances; move through three rows | Each row reads its Account, Project/branch, status **in words** (e.g. "Waiting for you"), and the attention reason when one is shown |
| A3 | Type into the search field ("Search instances by …") | The field's label is read; the filtered result count, or "no matches", is announced politely, without moving focus |
| A4 | Change a filter (Provider) | The new value is read; focus stays on the control |

### B. Inspector

| # | Do | Expect to hear |
|---|---|---|
| B1 | Select an Instance; VO-→ to the inspector | The Instance's title and status, then the tab list "Overview, Usage, History, Quality / Performance, Processes, Actions" |
| B2 | Walk the tabs with ←/→ | Each tab is announced as selected and its panel is reachable with VO-→ |
| B3 | On Usage, reach a quota bar | The value and its confidence ("Measured", "Computed" or "Estimated") are read as text, not only as a bar |
| B4 | On Processes, reach the tree | Row name, CPU and RAM are read as table cells with their column headers |
| B5 | Trigger a status change (stop the Instance) | The change is announced by a live region; focus doesn't move |

### C. One wizard (switch to another Account)

| # | Do | Expect to hear |
|---|---|---|
| C1 | From Actions, start "Continue on another account" | Dialog role and title; focus lands inside the dialog |
| C2 | Continue through the steps | "Step 2 of 3" and "Step 3 of 3" are announced politely |
| C3 | Press Esc | The dialog closes and focus returns to the button that opened it |

## Findings

| Release | Date | Step | Heard | Expected | Verdict | Task |
|---|---|---|---|---|---|---|
| | | | | | | |
