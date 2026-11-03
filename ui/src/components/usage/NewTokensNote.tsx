/**
 * The line under a "New tokens" figure (0.3.7, the usage audit): what the
 * figure counts, then the tokens re-read from the cache, counted apart.
 * Two short lines, so the tile keeps its siblings' height where it's
 * narrow (the account page).
 */
import type { JSX } from "react";
import { NEW_TOKENS_SHORT, NEW_TOKENS_WORDS } from "../../lib/usage/tokens";
import { short } from "../charts/scale";

export function NewTokensNote({ cacheRead }: { cacheRead: number }): JSX.Element {
  return (
    <span className="block" title={NEW_TOKENS_WORDS}>
      <span className="block">{NEW_TOKENS_SHORT}</span>
      <span className="block">{short(cacheRead, "tokens")} more re-read from cache</span>
    </span>
  );
}
