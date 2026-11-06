# Not captured (explicit omissions)

| Screen | State | Reason |
|---|---|---|
| 05-session-panel | loading (skeleton) | NOT CAPTURED: pending AJ-E405+ (state emission not yet implemented) |
| 01-rail | error (banner + retained data) | NOT CAPTURED: pending AJ-E405+ (error states not yet implemented) |
| 05-session-panel | error (banner + retained data) | NOT CAPTURED: pending AJ-E405+ (error states not yet implemented) |
| 97-pool-canvas | a member's ⋯ menu on the canvas (right-click, ⋯ or Shift+F10) | NOT CAPTURED (AJ-E2814): the shared row menu, whose look 77-menus captures; its lines are the inspector's actions. To add to pool-canvas-gallery.spec.ts. |
| 97-pool-canvas | the header's Agents picker and the inspector's Agents and "Pause and ask after" pickers, open | NOT CAPTURED (AJ-E2814): the shared Picker; its open state is captured on History's filter (97-controls picker-open-history) and Tasks' Holder (97-nav tasks-holder-menu). |
| 97-nav | Tasks' Project and Pool pickers, open | NOT CAPTURED (AJ-E2814): the same Picker as Holder, captured open (tasks-holder-menu). |
| 88-subagents | the Subagents tab's Type and Sort pickers, open | NOT CAPTURED (AJ-E2814): the same Picker (captured open in 97-controls and 97-nav); tab-most-tokens shows Sort's choice applied. |
| 99-related-list | "↑ 3 new" (rows that arrived while the list was read) and "Show in order" | NOT CAPTURED (AJ-E3103): it shows only when the backend says sessions changed while a list is open (`onSessionsUpdated`); the design preview has no backend. The hold is unit-tested (`components/related/related-order.test.ts`, `use-held.test.tsx`); its pill is the accent chip the list draws above its rows. |
| 99-related-list | the State, App and Account filter menus, open | NOT CAPTURED (AJ-E3103): the same FilterChip and Picker as the Project filter, captured open (filter-menu); the State menu's choice is captured applied (filtered). |
