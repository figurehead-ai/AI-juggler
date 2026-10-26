/**
 * What a process probably is, in the words people use: the backend's
 * role guess ("dev server", "MCP server") said as a guess, never as a
 * fact — "probably a dev server", "probably an MCP server". Null when
 * nothing is known.
 */
export function probablyWords(role: string): string | null {
  if (role.trim() === "") return null;
  return `probably ${/^(MCP|AI|[aeiou])/i.test(role) ? "an" : "a"} ${role}`;
}

/** Interpreters whose OWN name is never the user's name for the
 * process (`node …/vite dev` is "vite", not "node"). */
const INTERPRETER_NAMES = new Set([
  "node",
  "npm",
  "npx",
  "pnpm",
  "yarn",
  "bun",
  "bunx",
  "deno",
  "python",
  "python3",
  "ruby",
  "sh",
  "bash",
  "zsh",
  "fish",
  "dotnet",
  "java",
]);

/** A runner's own verbs (`npm run dev` is "dev"). */
const RUNNER_VERBS = new Set(["run", "exec", "dlx"]);

/** The short process name for the consequence-named button (Req.1's
 * `Terminate vite (PID 8123) and free :5173`). */
export function killDisplayName(cmdline: string, exePath: string): string {
  const basename = (token: string) =>
    token.replaceAll("\\", "/").split("/").pop() ?? "";
  for (const token of cmdline.split(/\s+/)) {
    const base = basename(token);
    // Flags (`-m`, `--port`) and runner verbs (`npm run dev`) are never
    // the name.
    if (
      base !== "" &&
      !base.startsWith("-") &&
      !INTERPRETER_NAMES.has(base.toLowerCase()) &&
      !RUNNER_VERBS.has(base)
    ) {
      return base;
    }
  }
  return basename(cmdline.split(/\s+/)[0] ?? "") || basename(exePath) || "process";
}
