import { useEffect, useRef, useState } from "react";
import { Button } from "./primitives";
import { UsageStatsChooser } from "./UsageStatsChooser";
import { usageStatsInfo, usageStatsSet, type UsageStatsInfo, type UsageStatsMode } from "../lib/bridge";

/**
 * AJ-E1105: when what usage statistics may contain changes (a consent
 * version bump), AI Juggler asks again at launch, with the current
 * choice pre-selected. Until it's answered nothing is sent (the backend
 * holds every report); closing it keeps the choice and asks next time.
 */
export function UsageStatsReconsent(): JSX.Element | null {
  const [info, setInfo] = useState<UsageStatsInfo | null>(null);
  const [choice, setChoice] = useState<UsageStatsMode | null>(null);
  const [closed, setClosed] = useState(false);
  const panelRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    void usageStatsInfo()
      .then((next) => {
        setInfo(next);
        setChoice(next.chosen === "share" || next.chosen === "local" ? next.chosen : "off");
      })
      .catch(() => setInfo(null));
  }, []);
  const due =
    info !== null &&
    info.included &&
    info.decided_at_ms !== null &&
    info.consent_version < info.current_consent_version;
  const open = due && !closed;
  // Focus the current choice when the dialog opens; Escape asks next time.
  useEffect(() => {
    if (open) panelRef.current?.querySelector<HTMLInputElement>("input:checked")?.focus();
  }, [open]);
  if (!open) return null;
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="reconsent-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      data-testid="usage-stats-reconsent"
      onKeyDown={(event) => {
        if (event.key === "Escape") setClosed(true);
      }}
    >
      <div
        ref={panelRef}
        className="flex w-full max-w-md flex-col gap-3 rounded-xl border border-hairline bg-surface p-5 shadow-lg"
      >
        <h2 id="reconsent-title" className="text-sm font-semibold">
          Usage statistics changed
        </h2>
        <p className="text-[11px] leading-relaxed text-muted">
          What a weekly report can contain has changed. Nothing is sent until you choose again — your current choice is
          selected.
        </p>
        <UsageStatsChooser value={choice} onChange={setChoice} />
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="secondary" onClick={() => setClosed(true)} data-testid="reconsent-later">
            Ask me next time
          </Button>
          <Button
            size="sm"
            variant="primary"
            disabled={choice === null}
            onClick={() => {
              if (choice === null) return;
              void usageStatsSet(choice).then(() => setClosed(true));
            }}
            data-testid="reconsent-save"
          >
            Save
          </Button>
        </div>
      </div>
    </div>
  );
}
