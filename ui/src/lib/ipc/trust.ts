/**
 * AJ-E1518: the Privacy page's links. The page names what to open (a
 * tool's terms, a cited test's file, the private security report) and the
 * app picks the address (`trust_open`), so nothing the page holds is ever
 * opened as a URL. The preview has no browser to hand off to: it records
 * the ask in `__AIJ_OPENED__`, which the tests read.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";

export type TrustLink = { kind: "terms"; id: string } | { kind: "source"; id: string } | { kind: "report" } | { kind: "guide"; id: string } | { kind: "account-settings"; id: string };

/** Opens `link` in the user's browser. */
export async function trustOpen(link: TrustLink): Promise<void> {
  const id = "id" in link ? link.id : "";
  if (!hasLiveBackend()) {
    const w = window as unknown as { __AIJ_OPENED__?: Array<{ kind: string; id: string }> };
    (w.__AIJ_OPENED__ ??= []).push({ kind: link.kind, id });
    return;
  }
  await invoke<void>("trust_open", { kind: link.kind, id });
}
