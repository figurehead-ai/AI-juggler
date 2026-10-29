/**
 * AJ-E1611 — a calm line on a tool's page when a newer version is out,
 * whether AI Juggler was tested with it, and the command to update it
 * yourself (AI Juggler never updates a tool). Never a popup.
 */
import { useEffect, useState } from "react";
import { toolUpdateCheckSet, toolUpdateStatus, type ToolUpdateInfo } from "../lib/ipc/toolUpdates";
import { Checkbox } from "./controls";

export function ToolUpdateLine({ tool, name }: { tool: string; name: string }) {
  const [info, setInfo] = useState<ToolUpdateInfo | null>(null);
  useEffect(() => {
    let alive = true;
    void toolUpdateStatus(tool).then(
      (row) => alive && setInfo(row),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [tool]);
  if (!info || !info.has_feed) return null;
  const toggle = (on: boolean) => {
    void toolUpdateCheckSet(on).then(
      () => setInfo({ ...info, checks_on: on }),
      () => undefined,
    );
  };
  return (
    <div className="flex flex-col gap-1 text-label" data-testid="provider-update">
      {info.newer && info.latest && (
        <p className="text-ink" data-testid="provider-update-available">
          {name} {info.latest} is out{info.verified ? "" : "; not tested with AI Juggler yet"}.
          {info.update_command && (
            <>
              {" "}To update, run <code className="font-mono">{info.update_command}</code> yourself.
            </>
          )}
        </p>
      )}
      <label className="flex items-center gap-2 text-muted">
        <Checkbox
          checked={info.checks_on}
          onChange={(event) => toggle(event.target.checked)}
          data-testid="provider-update-check"
        />
        Look for new versions of your tools once a day
      </label>
    </div>
  );
}
