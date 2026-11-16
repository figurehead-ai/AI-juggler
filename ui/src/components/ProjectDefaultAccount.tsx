/**
 * AJ-E1501 — a project's own account for new sessions (the project page).
 */
import { useEffect, useState } from "react";
import { cliAppName } from "../lib/app-name";
import { useCliApps } from "../lib/apps";
import { Picker } from "./controls";
import { errorText } from "../lib/bridge";
import { projectSetDefaultAccount } from "../lib/ipc/accounts";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { useRelatedIds } from "../lib/relation-counts";

/** AJ-E1501 Req.4: which account this project's new sessions start
 * with — its own pick, else the global default or the most recently
 * used one (New session preselects it). */
export function ProjectDefaultAccount({
  project,
  accounts,
  onChanged,
}: {
  project: ProjectInfo;
  accounts: AccountInfo[];
  onChanged: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [value, setValue] = useState(project.default_account_id ?? "");
  useEffect(() => setValue(project.default_account_id ?? ""), [project.default_account_id]);
  // AJ-E3105: the Apps this project's sessions ran on first.
  const usedHere = useRelatedIds("project", project.id, "folder");
  // The CLI Apps, by their names (the 0.4.6 RC walks A8 and K4: a Claude
  // app profile's folder isn't one, and read "Claude Code CLI · Work Max").
  const apps = useCliApps(accounts);
  if (accounts.length === 0) return null;
  const offered = apps.flatMap((app) => (app.folder ? [{ id: app.folder.id, name: app.name }] : []));
  const here = offered.filter((app) => usedHere?.includes(app.id)).map((app) => app.id);
  const fallback = accounts.find((account) => account.is_default);
  const fallbackName = fallback ? (offered.find((app) => app.id === fallback.id)?.name ?? cliAppName(fallback.provider, fallback.label)) : null;
  return (
    <div className="flex flex-col gap-1 pb-2">
      <div className="flex flex-wrap items-center gap-2 text-meta text-muted" data-testid="project-default-account">
        New sessions here start with
        <Picker
          size="sm"
          ariaLabel="New sessions here start with"
          className="max-w-full"
          value={value}
          onChange={async (next) => {
            setValue(next);
            setError(null);
            try {
              await projectSetDefaultAccount(project.id, next === "" ? null : next);
              onChanged();
            } catch (cause) {
              setError(errorText(cause));
              setValue(project.default_account_id ?? "");
            }
          }}
          groups={here.length > 0 ? [{ label: "Used in this project", values: here }] : []}
          options={[
            { value: "", pinned: true, label: fallbackName ? `Your default (${fallbackName})` : "The app used last" },
            ...offered.map((app) => ({ value: app.id, label: app.name })),
          ]}
        />
      </div>
      {error && (
        <p className="text-label text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
