/**
 * 0.3 (the redesign: every door obvious) — a session page's doors in its
 * header's facts line ("Claude Code · Work Max · started 8h ago · Pool";
 * the 0.3 re-render moved them there from the chips, which they pushed
 * into ellipses at 1280 px): its pool, opening the pool's page (a pool of
 * yours, or its project's own pool once another session shares it; a
 * session alone there has nothing to see), and its tool, which opens
 * the tool's page (AJ-E3104; its Settings one click on from there),
 * moved here from the window's top bar, which lost its title.
 */
import type { JSX } from "react";
import { useActionHost } from "../../components/ItemMenu";
import { poolById, usePoolPlace, usePools } from "../../lib/pools";
import { useTools } from "../../lib/tools";
import { ToolMark } from "../../components/ToolMark";
import { IconAccount } from "../../components/usage/controls";
import { openAccount } from "../../lib/accounts-page";
import { accountName, useSubscriptions } from "../../lib/subscriptions";
import { accountForApp } from "../app/AppTerms";
import { IconComputer, IconPool } from "../../components/icons-actions";
import { THIS_MACHINE } from "../../lib/machine-words";
import { openApp, useApps } from "../../lib/apps";
import type { AccountInfo } from "../../types/generated/AccountInfo";

/** A door on the header's facts line: its icon and its words, in the
 * line's own size, ink on hover (the facts are muted). */
const DOOR =
  "inline-flex min-w-0 max-w-full items-center gap-1 whitespace-nowrap rounded text-meta text-ink/85 outline-none decoration-hairline underline-offset-2 transition-colors hover:text-ink hover:underline focus-visible:ring-2 focus-visible:ring-accent/60";

/** The facts line's separator. */
export const Dot = (): JSX.Element => (
  <span aria-hidden="true" className="text-muted">
    ·
  </span>
);

export function SessionPoolChip({ sessionId }: { sessionId: string }): JSX.Element | null {
  const host = useActionHost();
  const pools = usePools();
  const place = usePoolPlace(sessionId);
  const pool = place ? poolById(pools, place.pool_id) : undefined;
  if (!pool || !host.openPool || (pool.kind === "default" && pool.members < 2)) return null;
  return (
    <>
      <Dot />
      <button type="button" onClick={() => host.openPool?.(pool.id)} title={`Open the pool ${pool.name}`} className={DOOR} data-testid="session-pool-chip">
        <IconPool width={13} height={13} aria-hidden="true" className="shrink-0 text-muted" />
        {/* Its project's own pool is just "Pool". */}
        <span className="truncate">{pool.kind === "default" ? "Pool" : `Pool: ${pool.name}`}</span>
      </button>
    </>
  );
}

/** Its tool, a door to the tool's page; left out where `said` (the
 * header's App chip, "Claude Code · Work") names the tool already (the
 * 0.4 review, 46: "Claude Code" three times). */
export function SessionToolChip({ tool, said = null }: { tool: string; said?: string | null }): JSX.Element | null {
  const host = useActionHost();
  const spec = useTools().find((entry) => entry.id === tool);
  if (!spec || !host.openTool) return null;
  if (said !== null && said.includes(spec.display_name)) return null;
  return (
    // Its mark and its name: the chips name the app it runs in, never the
    // tool, and a bare mark read as an unlabelled icon (the 0.3
    // re-render's review).
    <>
      <button
        type="button"
        onClick={() => host.openTool?.(spec)}
        aria-label={`About ${spec.display_name}`}
        title={`Open ${spec.display_name}'s page`}
        className={DOOR}
        data-testid="context-tool"
      >
        <ToolMark tool={tool} size={13} />
        <span className="truncate">{spec.display_name}</span>
      </button>
      <Dot />
    </>
  );
}

/** AJ-E3104: an App's Account, one click to its page (an Account is a
 * relation to one: a door, not a tab), found as the App's Plan card finds
 * it. Neutral, as Accounts are beside Apps (SPEC §9.3). */
export function AccountDoor({ folderId, accountId, lead }: { folderId: string | null; accountId: string | null; lead?: JSX.Element }): JSX.Element | null {
  const account = accountForApp(useSubscriptions() ?? [], folderId, accountId);
  if (!account?.account_id) return null;
  const id = account.account_id;
  const name = accountName(account);
  return (
    <>
      {lead}
      <button type="button" onClick={() => openAccount(id)} title={`Open ${name}'s page`} className={DOOR} data-testid="door-account">
        <IconAccount width={13} height={13} aria-hidden="true" className="shrink-0 text-muted" />
        <span className="truncate">{name}</span>
      </button>
    </>
  );
}

/** AJ-E3104 R1: a project's default App, the one its new sessions start
 * in, one click to its page (set in the project's body). The CLI App of
 * the folder it names; nothing while that folder isn't one of yours. */
export function DefaultAppDoor({ folderId, folders, lead }: { folderId: string; folders: readonly AccountInfo[]; lead?: JSX.Element }): JSX.Element | null {
  const app = useApps(folders)?.find((one) => one.kind === "cli" && one.folder?.id === folderId);
  if (!app) return null;
  return (
    <>
      {lead}
      <button type="button" onClick={() => openApp(app.id)} title={`New sessions here start in ${app.name}. Open its page`} className={DOOR} data-testid="door-default-app">
        <span className="text-muted">starts in</span>
        <span className="truncate">{app.name}</span>
      </button>
    </>
  );
}

/** AJ-E3104 R1: a project worked on more than one Mac (paired in Fleet),
 * one click to Fleet; on this Mac alone, nothing (every page here is). */
export function MacsDoor({ count, lead }: { count: number; lead?: JSX.Element }): JSX.Element | null {
  if (count < 2) return null;
  return (
    <>
      {lead}
      <button type="button" onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "fleet" } }))} title="Open Fleet, your Macs" className={DOOR} data-testid="door-macs">
        <IconComputer width={13} height={13} aria-hidden="true" className="shrink-0 text-muted" />
        <span className="truncate">on {count} Macs</span>
      </button>
    </>
  );
}

/** AJ-E3204 R1: the Mac it runs on, a door to This Mac (its load, its
 * processes). An App is always this Mac's; a paired Mac's sessions have
 * their own page under Fleet. AJ-E3104 R1: a session here has it too (one
 * on another Mac has that Mac's chip instead). */
export function MacDoor({ lead }: { lead?: JSX.Element }): JSX.Element {
  return (
    <>
      {lead}
      <button type="button" onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "thismac" } }))} title={`Open ${THIS_MACHINE}`} className={DOOR} data-testid="door-mac">
        <IconComputer width={13} height={13} aria-hidden="true" className="shrink-0 text-muted" />
        <span className="truncate">{THIS_MACHINE}</span>
      </button>
    </>
  );
}

/** AJ-E3204 R1: "room shared with n Apps" beside the Account door, the
 * other Apps named on hover. */
export function SharedRoom({ names }: { names: readonly string[] }): JSX.Element | null {
  if (names.length === 0) return null;
  return (
    <span className="whitespace-nowrap text-meta text-muted" title={`Its account's room is shared with ${names.join(", ")}`} data-testid="app-header-shared">
      room shared with {names.length} {names.length === 1 ? "App" : "Apps"}
    </span>
  );
}
