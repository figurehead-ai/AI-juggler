/**
 * AJ-E1532 — the accounts that worked in a project, for its row's stack
 * of avatars: from its sessions in AI Juggler, the conversations started
 * outside it (AJ-E1529) and the history imported before AI Juggler.
 * Accounts serving a live session there come first, then the most
 * recent.
 */
import type { InstanceInfo, ProjectInfo } from "./bridge";
import type { ConversationInfo } from "../types/generated/ConversationInfo";
import { knownAccounts } from "./palette";
import type { StackedAccount } from "../components/Avatar";
import { isActiveStatus } from "../components/indicator";

export function projectAccounts(
  project: ProjectInfo,
  sessions: InstanceInfo[],
  conversations: ConversationInfo[],
  accounts: { id: string; label: string }[] = knownAccounts(),
): StackedAccount[] {
  const seen = new Map<string, StackedAccount & { last: number }>();
  const add = (id: string | null, label: string, live: boolean, last: number) => {
    if (label === "" && id === null) return;
    const key = id ?? `label:${label}`;
    const known = seen.get(key);
    if (known) {
      known.live ||= live;
      known.last = Math.max(known.last, last);
    } else {
      seen.set(key, { id, label, live, last });
    }
  };
  for (const session of sessions) {
    if (session.project_id === project.id) add(session.account_id, session.account_label, isActiveStatus(session.status), session.last_activity_ms ?? session.started_at_ms);
  }
  for (const conversation of conversations) {
    if (conversation.project_id === project.id) add(conversation.account_id, conversation.account_label, conversation.live, conversation.last_activity_ms ?? 0);
  }
  const labels = new Set([...seen.values()].map((account) => account.label));
  for (const label of project.accounts_used) {
    if (labels.has(label)) continue;
    const account = accounts.find((row) => row.label === label);
    add(account?.id ?? null, label, false, -1);
  }
  return [...seen.values()]
    .sort((a, b) => Number(b.live) - Number(a.live) || b.last - a.last || a.label.localeCompare(b.label))
    .map(({ id, label, live }) => ({ id, label, live }));
}

/** The accounts in words, for the row's name ("on Work Max and Personal"). */
export function accountsWords(accounts: StackedAccount[]): string {
  if (accounts.length === 0) return "";
  const names = accounts.map((account) => (account.live ? `${account.label} (running)` : account.label));
  const list = names.length === 1 ? names[0]! : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
  return `on ${list}`;
}
