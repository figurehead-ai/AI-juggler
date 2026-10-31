<!-- captures: d4d3fcdd0214 -->
# 82-project-sync: comparing and syncing a project's rules files and MCP servers

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

header-line shows the project path with an amber "1 rules file differs" link, and menu shows the project menu with "Compare rules and servers…" among icon rows and Forget… last in red. sheet-diff is a clear sheet: radio cards pick the master (CLAUDE.md, tinted violet), the diff shows line numbers with a red removed line, and "Apply to AGENTS.md" is the solid violet primary. In light the added lines are a grey-green barely different from the unchanged ones, while in dark they are a clear green. sheet-done confirms each change with its own small Undo button and points to Settings › Switching for later. The status text reads "1 rules file differs", which is ungrammatical.

## Weak

- sheet-diff-light: added lines in the diff are a faint grey-green, close to the unchanged lines.
- header-line, sheet-diff (both themes): "1 rules file differs" should be singular.
