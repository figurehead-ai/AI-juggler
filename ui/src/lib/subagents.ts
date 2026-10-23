/** AJ-E1520: `1 subagent running`, `3 subagents running` (I-08). */
export function subagentsRunning(count: number): string {
  return `${count} ${count === 1 ? "subagent" : "subagents"} running`;
}
