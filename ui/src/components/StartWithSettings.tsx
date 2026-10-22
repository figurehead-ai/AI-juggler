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

/** What to copy, once the account is made; `null` for nothing. */
export type SettingsChoice = { source: string; picked: string[] } | null;

const KINDS: Record<string, string> = {
  "mcp-server": "MCP servers",
  permissions: "permissions",
  "status-line": "status line",
  plugin: "plugins",
};

/** What comes with it, in a few words ("MCP servers, permissions and status line"). */
function contents(check: SetupCheckInfo, picked: ReadonlySet<string>): string {
  const kinds = [...new Set(check.offers.filter((offer) => picked.has(offer.id)).map((offer) => KINDS[offer.kind] ?? offer.kind))];
  if (kinds.length === 0) return "nothing ticked";
  if (kinds.length === 1) return kinds[0]!;
  return `${kinds.slice(0, -1).join(", ")} and ${kinds[kinds.length - 1]}`;
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
  const [source, setSource] = useState<string | null>(sources[0]?.id ?? null);
  const [check, setCheck] = useState<SetupCheckInfo | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const listId = useId();

  useEffect(() => {
    if (!source) return;
    let alive = true;
    setCheck(null);
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
    onChange(on && source && picked.size > 0 ? { source, picked: [...picked] } : null);
  }, [on, source, picked, onChange]);

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
          <input type="checkbox" checked={on} onChange={(event) => setOn(event.target.checked)} />
          <span>Start with the same settings as</span>
        </label>
        {sources.length > 1 ? (
          <select
            value={source}
            onChange={(event) => setSource(event.target.value)}
            aria-label="Account to copy the settings from"
            className="rounded-lg border border-hairline bg-surface px-2 py-1 text-sm text-ink"
          >
            {sources.map((account) => (
              <option key={account.id} value={account.id}>
                {account.label}
              </option>
            ))}
          </select>
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
            Its {contents(check, picked)} {open ? "▾" : "▸"}
          </button>
          {open && (
            <div id={listId}>
              <SwitchSetupList check={check} picked={picked} onToggle={toggle} bare />
            </div>
          )}
          <p className="text-[11px] text-faint">Never its sign-in, history or caches. Each can be undone in Settings.</p>
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
