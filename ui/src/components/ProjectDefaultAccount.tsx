/**
 * AJ-E1501 — a project's own account for new sessions (the project page).
 */
import { useEffect, useState } from "react";
import { Select } from "./form";
import { errorText, projectSetDefaultAccount, type AccountInfo, type ProjectInfo } from "../lib/bridge";

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
  if (accounts.length === 0) return null;
  const fallback = accounts.find((account) => account.is_default);
  return (
    <div className="flex flex-col gap-1 pb-2">
      <label className="flex flex-wrap items-center gap-2 text-meta text-muted" data-testid="project-default-account">
        New sessions here start with
        <Select
          aria-label="New sessions here start with"
          className="max-w-full"
          value={value}
          onChange={async (event) => {
            const next = event.target.value;
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
        >
          <option value="">{fallback ? `Your default (${fallback.label})` : "The account used last"}</option>
          {accounts.map((account) => (
            <option key={account.id} value={account.id}>
              {account.label}
            </option>
          ))}
        </Select>
      </label>
      {error && (
        <p className="text-label text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
