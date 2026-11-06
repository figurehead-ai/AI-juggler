/**
 * AJ-E2108 — a session page's "Where it runs": the terminal or editor a
 * CLI one runs in, its window, account, model and working folder, only
 * the ones it knows; then whatever the page adds (a session AI Juggler
 * runs: its project and its details). Its App (D-62) is the header's
 * App chip, a door to the App's page: said once (the 0.4 review, 46).
 */
import { openAccount } from "../../lib/accounts-page";
import { modelName } from "../../components/charts/colors";
import type { JSX, ReactNode } from "react";
import { Card } from "../../components/primitives";
import { shortPath } from "../../screens/AccountManage";
import { CardTitle } from "./CardTitle";
import { sessionHost, type SessionModel } from "./model";
import { useLastPlace } from "../../lib/places";

export function WhereItRuns({
  model,
  accountWords,
  outside,
  project,
  accountId = null,
  children,
}: {
  model: SessionModel;
  accountWords: string | null;
  /** D-62 (A3): its Account, a link to its page. */
  accountId?: string | null;
  /** Started outside AI Juggler: said once, quietly, at the end. */
  outside: boolean;
  /** Its project, a link to its page. */
  project?: { name: string; onOpen: () => void } | null;
  children?: ReactNode;
}): JSX.Element {
  const place = model.places[0];
  const host = sessionHost(model);
  const desktop = host === null && ((place !== undefined && place.app !== "") || model.desktop === true);
  // 0.3.3: an ended session of another app names where it was last open
  // (its header's Open in starts that app).
  const last = useLastPlace(model.tool, model.conversationId ?? "", outside && !model.live && model.conversationId !== null, model.startedIn ? { account: model.profileId, app: model.startedIn } : null);
  // Where its window is: a CLI App's terminal or editor; not open now, the
  // last one known. A desktop App's window is the App itself.
  const runsIn = host ?? (desktop ? null : model.live ? "Not known" : last ? `Not open now · last in ${last.app}` : "Not open now");
  // D-62 (AJ-E3206, A3): its Account opens its page.
  const doors: Record<string, (() => void) | undefined> = {
    Account: accountId ? () => openAccount(accountId) : undefined,
  };
  const rows: [string, string, boolean?][] = [
    ...(runsIn ? ([["Runs in", runsIn]] as [string, string][]) : []),
    ...(place?.instance ? ([["Window", place.label]] as [string, string][]) : []),
    ...(accountWords ? ([["Account", accountWords]] as [string, string][]) : []),
    // The model as people say it ("Sonnet"), never its id.
    ...(model.model ? ([["Model", modelName(model.model)]] as [string, string][]) : []),
    ...(model.cwd ? ([["Folder", shortPath(model.cwd), true]] as [string, string, boolean][]) : []),
  ];
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="where-it-runs">
      <CardTitle>Where it runs</CardTitle>
      <dl className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-4 gap-y-2 text-meta">
        {rows.map(([label, value, mono]) => (
          <div key={label} className="contents">
            <dt className="text-muted">{label}</dt>
            <dd className={"min-w-0 truncate text-ink " + (mono ? "font-mono text-meta" : "")} title={value}>
              {doors[label] ? (
                <button type="button" onClick={doors[label]} className="max-w-full truncate rounded text-left text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid={`where-${label.toLowerCase()}`}>
                  {value}
                </button>
              ) : (
                value
              )}
            </dd>
          </div>
        ))}
        {project && (
          <div className="contents">
            <dt className="text-muted">Project</dt>
            <dd className="min-w-0">
              <button
                type="button"
                onClick={project.onOpen}
                className="max-w-full truncate rounded text-left text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60"
                data-testid="where-project"
              >
                {project.name}
              </button>
            </dd>
          </div>
        )}
      </dl>
      {model.places.length > 1 && (
        <p className="text-meta text-ink-2" data-testid="where-also">
          Also open in {model.places.slice(1).map((p) => p.label || p.app).join(", ")}.
        </p>
      )}
      {children}
      {outside && (
        <p className="border-t border-hairline pt-3 text-label leading-[1.5] text-muted" data-testid="where-outside">
          Started <span data-testid="outside-chip">outside AI Juggler</span>: shown here to read and follow. AI Juggler never stops or changes it.
        </p>
      )}
    </Card>
  );
}
