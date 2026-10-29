/**
 * AJ-E1601 — the services the preview and the tests offer under Add
 * account › More › Through another service, matching what
 * `services_list` serves from crates/juggler-adapters/src/services.json.
 * Preview data only: the app reads the backend. Tests can swap it through
 * `window.__AIJ_SERVICES__`.
 */
import type { ServiceInfo } from "../types/generated/ServiceInfo";

export const PREVIEW_SERVICES: ServiceInfo[] = [
  {
    service: {
      id: "opencode-go",
      name: "OpenCode Go",
      short_name: "Go",
      usage: "estimate",
      usage_words:
        "Its limits are dollars per model: every 5 hours allows a fifth of the month's amount, and every week half. OpenCode doesn't tell other tools what's left, so AI Juggler can only estimate it from what its sessions used.",
      console_url: "https://opencode.ai/go",
      key_name: "OpenCode Go API key",
    },
    vendor: "Anomaly",
    plans: ["Go", "Go Plus"],
    tools: [
      { tool: "claude-code", models: ["minimax-m3", "claude-haiku-5-5"], guide: "https://opencode.ai/v2/docs/console/harnesses/claude-code/" },
      { tool: "codex", models: [], guide: "https://opencode.ai/v2/docs/console/harnesses/codex/" },
    ],
  },
  {
    service: {
      id: "kimi",
      name: "Kimi",
      short_name: "Kimi",
      usage: "unknown",
      usage_words:
        "Kimi counts a 5-hour window and a monthly total, shared with its own apps. It doesn't tell other tools what's left, so the limits are unknown here; Kimi's own pages show them.",
      console_url: "https://www.kimi.com/code/docs/en/kimi-code/membership.html",
      key_name: "Kimi Code API key",
    },
    vendor: "Moonshot AI",
    plans: ["Plus", "Pro", "Max", "Ultra"],
    tools: [
      { tool: "claude-code", models: ["k3-256k", "k3[1m]"], guide: "https://www.kimi.com/code/docs/en/third-party-tools/claude-code.html" },
      { tool: "codex", models: ["k3-256k", "k3"], guide: "https://www.kimi.com/code/docs/en/third-party-tools/codex.html" },
    ],
  },
];
