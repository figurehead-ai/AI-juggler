/**
 * AJ-E1508: Add account's "Start with the same settings as …": one line,
 * ticked, that names the account to copy from; it opens to the list of
 * what comes with it (MCP servers, permissions, the status line, plugins),
 * each ticked when it can be set up. Credentials, history and caches are
 * never among them. Nothing is copied until the account is made.
 */
import { useEffect, useId, useLayoutEffect, useState, type JSX } from "react";
import { errorText, type AccountInfo } from "../lib/bridge";
import { accountSettingsCheck } from "../lib/ipc/accountSettings";
import type { SetupCheckInfo } from "../lib/ipc/switchSetup";
import { SwitchSetupList } from "./SwitchSetupList";
import { Checkbox, Picker } from "./controls";

/** What to copy, once the account is made; `null` for nothing. While
 * what the account offers is still being read, `pending`: the step waits
 * (review of AJ-E1508, M3: a quick Create copied nothing). */
export type SettingsChoice = { source: string; picked: string[]; pending: boolean } | null;

const KINDS: Record<string, string> = {
  "mcp-server": "MCP servers",
  permissions: "permissions",
  "status-line": "status line",
  plugin: "plugins",
};

/** What comes with it, in a few words ("MCP servers, permissions and status line"). */
function contents(check: SetupCheckInfo, picked: ReadonlySet<string>): string {
  const kinds = [...new Set(check.offers.filter((offer) => picked.has(offer.id)).map((offer) => KINDS[offer.kind] ?? offer.kind))];
  if (kinds.length === 0) return "Nothing ticked";
  if (kinds.length === 1) return `Its ${kinds[0]!}`;
  return `Its ${kinds.slice(0, -1).join(", ")} and ${kinds[kinds.length - 1]}`;
}

export function StartWithSettings({
  sources,
  onChange,
}: {
  /** The accounts of the same tool to copy from. */
  sources: AccountInfo[];
  onChange: (choice: SettingsChoice) => void;
}): JSX.Element | null {
  const [on, setOn] = useState(true);
  // The account picked, else the first: the list may arrive after this
  // first draws (review of AJ-E1508, M4).
  const [chosen, setChosen] = useState<string | null>(null);
  const source = chosen ?? sources[0]?.id ?? null;
  const [check, setCheck] = useState<SetupCheckInfo | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const listId = useId();

  useEffect(() => {
    if (!source) return;
    let alive = true;
    // Another account's ticks never carry over to this one's.
    setCheck(null);
    setPicked(new Set());
    setError(null);
    accountSettingsCheck(null, source).then(
      (next) => {
        if (!alive) return;
        setCheck(next);
        setPicked(new Set(next.offers.filter((offer) => offer.suggested).map((offer) => offer.id)));
      },
      (cause) => alive && setError(errorText(cause)),
    );
    return () => {
      alive = false;
    };
  }, [source]);

  // Before anything can react to the list just shown (a quick Create
  // must copy what's ticked), the step knows the choice.
  useLayoutEffect(() => {
    if (!on || !source) onChange(null);
    else if (!check && !error) onChange({ source, picked: [], pending: true });
    else onChange(picked.size > 0 ? { source, picked: [...picked], pending: false } : null);
  }, [on, source, check, error, picked, onChange]);

  if (sources.length === 0 || !source) return null;
  // One account to copy from, with nothing to copy: no line at all.
  if (sources.length === 1 && check?.offers.length === 0) return null;
  const name = sources.find((account) => account.id === source)?.label ?? "";
  const toggle = (id: string) =>
    setPicked((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  return (
    <div className="flex max-w-xl flex-col gap-2 text-sm" data-testid="start-with-settings">
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-2">
          <Checkbox
            checked={on}
            onChange={(event) => setOn(event.target.checked)}
            aria-label={`Start with the same settings as ${name}`}
          />
          <span>Start with the same settings as</span>
        </label>
        {sources.length > 1 ? (
          <Picker
            size="sm"
            value={source}
            onChange={setChosen}
            ariaLabel="Account to copy the settings from"
            options={sources.map((account) => ({ value: account.id, label: account.label }))}
          />
        ) : (
          <span className="font-medium text-ink">{name}</span>
        )}
      </div>
      {on && check && check.offers.length === 0 && (
        <p className="text-xs text-muted">{name} has no settings to copy: no MCP servers, permissions or status line.</p>
      )}
      {on && check && check.offers.length > 0 && (
        <div className="flex flex-col gap-2 pl-6">
          <button
            type="button"
            aria-expanded={open}
            aria-controls={listId}
            onClick={() => setOpen(!open)}
            className="self-start text-xs text-muted hover:text-ink"
          >
            {contents(check, picked)} <span aria-hidden="true">{open ? "▾" : "▸"}</span>
          </button>
          {open && (
            <div id={listId}>
              <SwitchSetupList check={check} picked={picked} onToggle={toggle} bare />
            </div>
          )}
          <p className="text-label text-muted">Never its sign-in, history or caches. Each can be undone in Settings.</p>
        </div>
      )}
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
