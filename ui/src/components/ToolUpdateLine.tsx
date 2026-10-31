/**
 * AJ-E1611 — a calm line on a tool's row when a newer version is out,
 * whether AI Juggler was tested with it, and the command to update it
 * yourself (AI Juggler never updates a tool). Never a popup. 0.3.2: the
 * tool page folded into Settings › Accounts (0.1.5) and these went with
 * it — each tool's row says when a newer version is out, and the section
 * asks once whether to look (one setting for every tool).
 */
import { useEffect, useState } from "react";
import { toolUpdateCheckSet, toolUpdateStatus, type ToolUpdateInfo } from "../lib/ipc/toolUpdates";
import { Checkbox } from "./controls";

/** Each tool's update status, read once (a tool without a feed is absent). */
export function useToolUpdates(tools: string[]): [Record<string, ToolUpdateInfo>, (on: boolean) => void] {
  const [rows, setRows] = useState<Record<string, ToolUpdateInfo>>({});
  const key = tools.join(",");
  useEffect(() => {
    let alive = true;
    void Promise.all(key.split(",").filter(Boolean).map((tool) => toolUpdateStatus(tool).catch(() => null))).then((found) => {
      if (!alive) return;
      setRows(Object.fromEntries(found.filter((row): row is ToolUpdateInfo => row !== null && row.has_feed).map((row) => [row.tool, row])));
    });
    return () => {
      alive = false;
    };
  }, [key]);
  const setChecks = (on: boolean) => {
    void toolUpdateCheckSet(on).then(
      () => setRows((prior) => Object.fromEntries(Object.entries(prior).map(([tool, row]) => [tool, { ...row, checks_on: on }]))),
      () => undefined,
    );
  };
  return [rows, setChecks];
}

/** "OpenCode 1.19.4 is out…", or nothing while the installed one is the latest. */
export function ToolNewerLine({ info, name }: { info: ToolUpdateInfo | undefined; name: string }) {
  if (!info?.newer || !info.latest) return null;
  return (
    <p className="text-label text-ink-2" data-testid="provider-update-available">
      {name} {info.latest} is out{info.verified ? "" : "; not tested with AI Juggler yet"}.
      {info.update_command && (
        <>
          {" "}To update, run <code className="font-mono text-ink">{info.update_command}</code> yourself.
        </>
      )}
    </p>
  );
}

/** The one setting: look for new versions once a day (shown while any tool has a feed). */
export function ToolUpdateChecks({ rows, onChange }: { rows: Record<string, ToolUpdateInfo>; onChange: (on: boolean) => void }) {
  const any = Object.values(rows)[0];
  if (!any) return null;
  return (
    <label className="flex items-center gap-2 text-label text-muted" data-testid="provider-update">
      <Checkbox checked={any.checks_on} onChange={(event) => onChange(event.target.checked)} data-testid="provider-update-check" />
      Look for new versions of your tools once a day
    </label>
  );
}

/** One tool's line and the setting together (a tool's own overview). */
export function ToolUpdateLine({ tool, name }: { tool: string; name: string }) {
  const [rows, setChecks] = useToolUpdates([tool]);
  if (!rows[tool]) return null;
  return (
    <div className="flex flex-col gap-1">
      <ToolNewerLine info={rows[tool]} name={name} />
      <ToolUpdateChecks rows={rows} onChange={setChecks} />
    </div>
  );
}
