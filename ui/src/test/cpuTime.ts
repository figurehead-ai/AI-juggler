// This process's CPU time (user + system) around synchronous work, for
// timing budgets in tests: other programs' turns on a busy machine don't
// count against it, as wall time does. Wall time is returned beside it.
// Tests run under Node; the app's types don't include Node's, so the one
// call is typed here.

interface CpuUsage {
  user: number;
  system: number;
}

const node = (globalThis as unknown as { process: { cpuUsage(previous?: CpuUsage): CpuUsage } }).process;

/** Runs `work` and returns its result with the CPU and wall time it took, in ms. */
export function cpuTimed<T>(work: () => T): { result: T; cpuMs: number; wallMs: number } {
  const wall = performance.now();
  const start = node.cpuUsage();
  const result = work();
  const used = node.cpuUsage(start);
  return { result, cpuMs: (used.user + used.system) / 1000, wallMs: performance.now() - wall };
}
