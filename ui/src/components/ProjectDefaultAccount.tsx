/**
 * AJ-E1501 — a project's own account for new sessions (the project page).
 */
import { useEffect, useState } from "react";
import { cliAppName } from "../lib/app-name";
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
  if (accounts.length === 0) return null;
  const here = accounts.filter((account) => usedHere?.includes(account.id)).map((account) => account.id);
  const fallback = accounts.find((account) => account.is_default);
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
            { value: "", pinned: true, label: fallback ? `Your default (${cliAppName(fallback.provider, fallback.label)})` : "The app used last" },
            ...accounts.map((account) => ({ value: account.id, label: cliAppName(account.provider, account.label) })),
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
