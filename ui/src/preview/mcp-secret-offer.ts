/**
 * AJ-E1543: an MCP server that needs a secret, as the Claude Code adapter
 * offers it (`claude_inventory::how_to_get`): it can be set up without the
 * value, and the steps give the account its own copy, with a placeholder,
 * never a value. Plain data for the preview, the tests and the gallery.
 */
import type { OfferInfo } from "../types/generated/OfferInfo";

/** The account's Claude Code folder, as the shell reads it. */
const FOLDER = "$HOME/.claude-personal";

export const EXAMPLE_GITHUB_OFFER: OfferInfo = {
  id: "mcp-github",
  kind: "mcp-server",
  name: "github",
  scope: "user",
  verdict: "needs-you",
  note: "it needs your GITHUB_TOKEN, and AI Juggler never copies a secret",
  suggested: false,
  about: null,
  steps: [
    "Open Terminal.",
    "Paste the command below. Put your own value in place of <your GITHUB_TOKEN> (keep the quotes around it), then press Return.",
    "It sets github up for this account, with the value in this account's own Claude Code settings. Its first line takes away a github set up without the value; if there isn't one, it says so, and that's fine.",
    "Come back here and choose Check again.",
  ],
  steps_command:
    `CLAUDE_CONFIG_DIR="${FOLDER}" claude mcp remove github --scope user\n` +
    `CLAUDE_CONFIG_DIR="${FOLDER}" claude mcp add-json --scope user github '{"args":["-y","@modelcontextprotocol/server-github"],"command":"npx","env":{"GITHUB_TOKEN":"<your GITHUB_TOKEN>"},"type":"stdio"}'`,
  steps_page: null,
  steps_page_label: null,
};
