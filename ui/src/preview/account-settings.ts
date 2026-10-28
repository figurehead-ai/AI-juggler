/**
 * AJ-E1508: what the preview's accounts offer a new account (the preview
 * and the gallery; tests set `window.__AIJ_ACCOUNT_SETTINGS__` instead).
 * Plain data, so a Playwright spec can read it too.
 */
import type { SetupCheckInfo } from "../types/generated/SetupCheckInfo";
import { EXAMPLE_GITHUB_OFFER } from "./mcp-secret-offer";

/** By the account copied from. */
export const EXAMPLE_ACCOUNT_SETTINGS: Record<string, SetupCheckInfo> = {
  "claude-work": {
    from: "Work Max",
    to: "the new account",
    already_there: 0,
    offers: [
      { id: "o-docs", kind: "mcp-server", name: "docs", scope: "user", verdict: "copy", note: null, suggested: true, about: null, used: null, origin: "own", provided_by: null, get_it: null, can_copy: false },
      { ...EXAMPLE_GITHUB_OFFER, id: "o-github", used: null },
      { id: "o-perm", kind: "permissions", name: "permissions", scope: "user", verdict: "copy", note: null, suggested: true, about: "4 allowed, 1 denied", used: null, origin: "own", provided_by: null, get_it: null, can_copy: false },
      { id: "o-line", kind: "status-line", name: "status line", scope: "user", verdict: "copy", note: null, suggested: true, about: "runs ~/bin/statusline", used: null, origin: "own", provided_by: null, get_it: null, can_copy: false },
      {
        id: "o-tidy",
        kind: "plugin",
        name: "tidy@market",
        scope: "user",
        verdict: "needs-you",
        note: "install it in this account with `claude plugin install tidy@market`",
        suggested: false,
        about: null, used: null, origin: "own", provided_by: null, get_it: null, can_copy: false,
      },
    ],
  },
  "claude-personal": { from: "Personal", to: "the new account", already_there: 0, offers: [] },
};
