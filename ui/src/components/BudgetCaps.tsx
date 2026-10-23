/**
 * AJ-E1514 — monthly caps on Usage: each cap's month so far against its
 * amount, and the form to set one. A cap only warns (in Needs you, at
 * 80 % and 100 %); nothing is ever stopped. The amounts are the cost
 * ledger's estimates at list prices, as the table below them.
 */
import { useEffect, useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { Field, Input, Modal, Select } from "./form";
import { errorText, type AccountInfo, type ProjectInfo } from "../lib/bridge";
import { budgetRemove, budgetSave, budgetsList, dollars, percentOf, type BudgetInfo } from "../lib/ipc/budgets";

/** What a cap counts, in words: an account's name, or its projects'. */
export function scopeWords(cap: BudgetInfo, accounts: AccountInfo[], projects: ProjectInfo[]): string {
  if (cap.account_id !== null) {
    return accounts.find((account) => account.id === cap.account_id)?.label ?? "An account that's been removed";
  }
  const names = cap.project_ids.map((id) => projects.find((project) => project.id === id)?.name).filter(Boolean);
  return names.length === 0 ? "Projects that have been removed" : names.join(", ");
}

/** "≈$243 of $300 this month" (or "Not priced yet" when nothing could be). */
export function spentWords(cap: BudgetInfo): string {
  if (cap.spent_usd === null) return `Not priced yet · cap ${dollars(cap.monthly_usd)} a month`;
  return `≈${dollars(cap.spent_usd)} of ${dollars(cap.monthly_usd)} this month`;
}

const TONE: Record<string, string> = { under: "bg-accent", near: "bg-warning", over: "bg-danger" };

export function BudgetCaps({ accounts, projects }: { accounts: AccountInfo[]; projects: ProjectInfo[] }): JSX.Element {
  const [caps, setCaps] = useState<BudgetInfo[]>([]);
  const [editing, setEditing] = useState<BudgetInfo | "new" | null>(null);
  const load = () => void budgetsList().then(setCaps, () => null);
  useEffect(() => {
    load();
    const timer = window.setInterval(load, 60_000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <section aria-labelledby="usage-caps" className="flex flex-col gap-2.5" data-testid="usage-caps">
      <div className="flex items-center justify-between gap-3">
        <h3 id="usage-caps" className="text-[11px] font-semibold uppercase tracking-wider text-muted">
          Monthly caps
        </h3>
        <Button size="sm" variant="ghost" onClick={() => setEditing("new")} data-testid="cap-add">
          Set a monthly cap
        </Button>
      </div>
      {caps.length === 0 ? (
        <p className="text-xs text-muted">
          A cap tells you in Needs you when an account or a group of projects reaches 80 % and 100 % of an amount a month.
          Nothing is ever stopped.
        </p>
      ) : (
        <ul className="grid gap-3 md:grid-cols-2">
          {caps.map((cap) => (
            <li key={cap.id}>
              <Card className="flex h-full flex-col gap-2 p-4" data-testid="cap-row">
                <span className="flex items-baseline justify-between gap-3">
                  <span className="truncate text-sm font-medium">{cap.name}</span>
                  <span className="shrink-0 text-xs tabular-nums text-muted">{percentOf(cap)}%</span>
                </span>
                <span className="truncate text-xs text-muted">{scopeWords(cap, accounts, projects)}</span>
                <span className="relative h-1.5 overflow-hidden rounded-full bg-surface-3" aria-hidden="true">
                  <span
                    className={`absolute inset-y-0 left-0 rounded-full ${TONE[cap.level] ?? "bg-accent"}`}
                    style={{ width: `${Math.min(100, Math.max(2, percentOf(cap)))}%` }}
                  />
                </span>
                <span className="text-xs tabular-nums text-ink" data-testid="cap-spent">
                  {spentWords(cap)}
                </span>
                {cap.unpriced && <span className="text-[11px] text-muted">Some usage has no price yet, so it isn't counted.</span>}
                <span className="mt-auto flex justify-end gap-1">
                  <Button size="sm" variant="ghost" onClick={() => setEditing(cap)}>
                    Edit <span className="sr-only">— {cap.name}</span>
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => void budgetRemove(cap.id).then(load, () => null)}>
                    Remove <span className="sr-only">— {cap.name}</span>
                  </Button>
                </span>
              </Card>
            </li>
          ))}
        </ul>
      )}
      {caps.length > 0 && (
        <p className="text-[11px] text-faint">
          Estimated at list prices from what this Mac measured, as in the table below. On a subscription that's what your use
          would cost by the token, not your bill.
        </p>
      )}
      {editing !== null && (
        <BudgetSheet
          cap={editing === "new" ? null : editing}
          accounts={accounts}
          projects={projects}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            load();
          }}
        />
      )}
    </section>
  );
}

/** The form: a name, an amount a month, and what it counts. */
export function BudgetSheet({
  cap,
  accounts,
  projects,
  onClose,
  onSaved,
}: {
  cap: BudgetInfo | null;
  accounts: AccountInfo[];
  projects: ProjectInfo[];
  onClose: () => void;
  onSaved: () => void;
}): JSX.Element {
  const [name, setName] = useState(cap?.name ?? "");
  const [amount, setAmount] = useState(cap ? String(cap.monthly_usd) : "");
  const [kind, setKind] = useState<"account" | "projects">(cap && cap.account_id === null ? "projects" : "account");
  const [account, setAccount] = useState(cap?.account_id ?? accounts[0]?.id ?? "");
  const [picked, setPicked] = useState<string[]>(cap?.project_ids ?? []);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <Modal title={cap ? `Edit ${cap.name}` : "Set a monthly cap"} subtitle="You'll hear at 80 % and 100 %. Nothing is stopped." onClose={onClose}>
      <form
        className="flex flex-col gap-4"
        data-testid="cap-sheet"
        onSubmit={async (event) => {
          event.preventDefault();
          setBusy(true);
          setError(null);
          try {
            await budgetSave({
              id: cap?.id ?? null,
              name,
              monthly_usd: Number(amount.replace(/[$,\s]/g, "")),
              account_id: kind === "account" ? account : null,
              project_ids: kind === "projects" ? picked : [],
            });
            onSaved();
          } catch (cause) {
            setError(errorText(cause));
            setBusy(false);
          }
        }}
      >
        <Field label="Name" hint="For example the client's name.">
          <Input autoFocus value={name} maxLength={40} onChange={(event) => setName(event.target.value)} data-testid="cap-name" />
        </Field>
        <Field label="Amount a month (USD)">
          <Input inputMode="decimal" value={amount} placeholder="300" onChange={(event) => setAmount(event.target.value)} data-testid="cap-amount" />
        </Field>
        <fieldset className="flex flex-col gap-2">
          <legend className="mb-1.5 text-[11px] uppercase tracking-[0.14em] text-faint">Counts</legend>
          <label className="flex items-center gap-2 text-sm">
            <input type="radio" name="cap-kind" checked={kind === "account"} onChange={() => setKind("account")} className="accent-[var(--color-accent)]" />
            One account
          </label>
          {kind === "account" && (
            <Select aria-label="Account" value={account} onChange={(event) => setAccount(event.target.value)} data-testid="cap-account">
              {accounts.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.label}
                </option>
              ))}
            </Select>
          )}
          <label className="flex items-center gap-2 text-sm">
            <input type="radio" name="cap-kind" checked={kind === "projects"} onChange={() => setKind("projects")} className="accent-[var(--color-accent)]" />
            A group of projects, on any account
          </label>
          {kind === "projects" && (
            <ul className="flex max-h-48 flex-col gap-1 overflow-y-auto rounded-xl border border-hairline p-2" data-testid="cap-projects">
              {projects.map((project) => (
                <li key={project.id}>
                  <label className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={picked.includes(project.id)}
                      onChange={(event) =>
                        setPicked((now) => (event.target.checked ? [...now, project.id] : now.filter((id) => id !== project.id)))
                      }
                      className="accent-[var(--color-accent)]"
                    />
                    {project.name}
                  </label>
                </li>
              ))}
            </ul>
          )}
        </fieldset>
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" type="submit" disabled={busy} data-testid="cap-save">
            {busy ? "Saving…" : "Save"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
