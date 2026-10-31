/**
 * AJ-E2108 — a session page's "Where it runs": its App (D-62: the app in
 * its profile, or the tool on its folder), the terminal or editor a CLI
 * one runs in, its window, account, model and working folder, only the
 * ones it knows; then whatever the page adds (a session AI Juggler runs:
 * its project and its details).
 */
import type { JSX, ReactNode } from "react";
import { Card } from "../../components/primitives";
import { shortPath } from "../../screens/AccountManage";
import { CardTitle } from "./CardTitle";
import { sessionAppName, sessionHost, type SessionModel } from "./model";
import { useLastPlace } from "../../lib/places";

export function WhereItRuns({
  model,
  accountWords,
  outside,
  project,
  children,
}: {
  model: SessionModel;
  accountWords: string | null;
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
  const last = useLastPlace(model.tool, model.conversationId ?? "", outside && !model.live && model.conversationId !== null);
  // Where its window is: a CLI App's terminal or editor; not open now, the
  // last one known. A desktop App's window is the App itself.
  const runsIn = host ?? (desktop ? null : model.live ? "Not known" : last ? `Not open now · last in ${last.app}` : "Not open now");
  const rows: [string, string, boolean?][] = [
    ["App", sessionAppName(model)],
    ...(runsIn ? ([["Runs in", runsIn]] as [string, string][]) : []),
    ...(place?.instance ? ([["Window", place.label]] as [string, string][]) : []),
    ...(accountWords ? ([["Account", accountWords]] as [string, string][]) : []),
    ...(model.model ? ([["Model", model.model, true]] as [string, string, boolean][]) : []),
    ...(model.cwd ? ([["Working folder", shortPath(model.cwd), true]] as [string, string, boolean][]) : []),
  ];
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="where-it-runs">
      <CardTitle>Where it runs</CardTitle>
      <dl className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-4 gap-y-2 text-meta">
        {rows.map(([label, value, mono]) => (
          <div key={label} className="contents">
            <dt className="text-muted">{label}</dt>
            <dd className={"min-w-0 truncate text-ink " + (mono ? "font-mono text-meta" : "")} title={value}>
              {value}
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
