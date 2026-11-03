/**
 * What every usage figure counts as "tokens" (0.3.6, the usage audit):
 * new tokens — input, cache writes, output and reasoning, everything a
 * call sent or made for the first time. Cache reads, the same context
 * read again, are shown apart. The backend's `usage_history::fold::io`
 * is the same rule.
 *
 * Why not input and output: Claude Code sends nearly all new input as
 * cache writes (its plain input averages 2 tokens a call) and Codex's
 * reasoning is output, so "in and out" made Claude look five times
 * smaller than its use.
 */

/** The token classes a figure is made from. */
export interface TokenClasses {
  input: number;
  output: number;
  cache_read: number;
  cache_write: number;
  reasoning?: number;
}

/** New tokens: input, cache writes, output and reasoning. */
export function newTokens(tokens: TokenClasses): number {
  return tokens.input + tokens.cache_write + tokens.output + (tokens.reasoning ?? 0);
}

/** The definition, in words, on hover where the figure leads. */
export const NEW_TOKENS_WORDS = "Input, output, cache writes and reasoning; what was re-read from the cache is counted apart";

/** The definition, short enough for one line under the figure: input and
 * output go without saying. */
export const NEW_TOKENS_SHORT = "With cache writes, reasoning";
