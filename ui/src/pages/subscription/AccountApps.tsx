/**
 * D-62 (AJ-E3205) — an Account's Apps: each App signed in to it, with
 * its icon, what's open in it and when it was last used, and a door to
 * its page. Quiet ones (no session yet) last, on a tinted surface.
 */
import type { JSX } from "react";
import { AppIcon } from "../../components/AppIcon";
import { AppKindBadge } from "../../components/AppKindBadge";
import { ProfileBadge } from "../../components/ProfileBadge";
import { openApp } from "../../lib/apps";
import type { AccountApp } from "../../lib/account-items";
import { agoWords } from "../../lib/time-words";

/** "Instant and paged": an App row's shape while the Apps are read. */
export function AppRowPending(): JSX.Element {
  return (
    <div className="flex items-center gap-3 px-2 py-1.5" aria-busy="true" aria-label="Reading its apps" data-testid="account-apps-pending">
      <span className="size-5 shrink-0 animate-pulse rounded-md bg-skeleton" />
      <span className="h-3 w-40 animate-pulse rounded bg-skeleton" />
    </div>
  );
}

/** `apps`: `null` while they're read (a placeholder row). */
export function AccountAppRows({ apps, now, compact = false, empty = "No app uses this account yet." }: { apps: AccountApp[] | null; now: number; compact?: boolean; empty?: string }): JSX.Element {
  if (apps === null) return <AppRowPending />;
  if (apps.length === 0) return <p className="text-meta text-ink-2">{empty}</p>;
  const busy = apps.filter((one) => one.sessions > 0);
  const quiet = apps.filter((one) => one.sessions === 0);
  return (
    <div className="flex flex-col gap-2" data-testid="account-apps">
      {busy.length > 0 && <AppList apps={busy} now={now} compact={compact} />}
      {quiet.length > 0 && (
        <div className="flex flex-col gap-1 rounded-xl bg-surface-2/60 px-1 py-1" data-testid="account-apps-quiet">
          {!compact && <p className="px-2 pt-1 text-label text-muted">No sessions yet</p>}
          <AppList apps={quiet} now={now} compact={compact} />
        </div>
      )}
    </div>
  );
}

function AppList({ apps, now, compact }: { apps: AccountApp[]; now: number; compact: boolean }): JSX.Element {
  return (
    <ul className="flex flex-col">
      {apps.map(({ app, sessions, open, lastMs }) => (
        <li key={app.id}>
          <button
            type="button"
            onClick={() => openApp(app.id)}
            className="flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
            data-testid="account-app"
            data-app-id={app.id}
          >
            <span className="flex shrink-0 items-center gap-1">
              <AppIcon appId={app.appId} appPid={app.place?.app_pid ?? null} name={app.app} size={compact ? 20 : 24} />
              {/* D-100: Desktop or CLI, beside its icon. */}
              <AppKindBadge kind={app.kind} />
            </span>
            <span className="flex min-w-0 flex-1 items-center gap-1.5">
              <span className="truncate text-body font-medium text-ink">{app.app}</span>
              {app.profile && (
                <span className="flex min-w-0 items-center gap-1 text-body text-ink-2">
                  <ProfileBadge app={app.slot} name={app.profile} />
                  <span className="truncate">{app.named ? app.name : app.profile}</span>
                </span>
              )}
            </span>
            <span className="shrink-0 text-meta text-ink-2">
              {sessions === 0 ? (lastMs > 0 ? `seen ${agoWords(lastMs, now)}` : "not used yet") : open > 0 ? `${open} open · ${sessions} in all` : `${sessions === 1 ? "1 session" : `${sessions} sessions`} · ${agoWords(lastMs, now)}`}
            </span>
            <span className="shrink-0 text-label text-muted" aria-hidden="true">
              ›
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
