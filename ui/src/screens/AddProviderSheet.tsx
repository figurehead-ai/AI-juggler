import { ToolMark } from "../components/ToolMark";
import { useEffect, useRef, useState } from "react";
import { Button, Card, Chip } from "../components/primitives";
import { IconJuggler } from "../components/icons";
import { funnelStep } from "../lib/funnel";
import { providersDiscover, type AccountInfo, type ProviderInstallData } from "../lib/bridge";
import { FOUND_UNCHECKED, providerDisplayName, toolFound, versionUnchecked } from "../lib/providers";
import { servicesList, type ServiceInfo } from "../lib/ipc/services";
import { lazyNamed } from "../lib/lazy";
import { LetterMark } from "../components/ToolMark";

const ServiceAccountSheet = lazyNamed(() => import("./accounts/ServiceAccountSheet"), "ServiceAccountSheet");

/**
 * AJ-E306 flow (c), folded into Add account (D-25 stage 4, AJ-E1023):
 * when more than one tool is installed, Add account first asks which
 * tool the account is for. One card per tool with its install state,
 * verified-range badge and kind — desktop cards state their
 * single-instance limit BEFORE selection (Req.3); not-installed cards
 * offer [Install →] and are never selectable (Req.1). Choosing a card
 * hands the tool to Add account (find → bring in or a fresh account →
 * sign in), which keeps flow (c) within 4 clicks.
 *
 * AJ-E1601: More › Through another service — a subscription that answers
 * for Claude Code or Codex at its own published address (OpenCode Go,
 * Kimi), added with its key.
 */
export function AddProviderSheet({
  onClose,
  onChoose,
  onServiceAdded,
}: {
  onClose: () => void;
  onChoose: (provider: string) => void;
  /** AJ-E1601: an account through another service was added (its
   * profiles). Without it, More isn't offered. */
  onServiceAdded?: (profiles: AccountInfo[]) => void;
}) {
  const [installs, setInstalls] = useState<ProviderInstallData[] | null>(null);
  const [services, setServices] = useState<ServiceInfo[]>([]);
  const [serviceFor, setServiceFor] = useState<string | null>(null);
  const frameRef = useRef<HTMLDivElement | null>(null);
  // The parent passes a new onClose on every render (App re-renders on
  // its clock and on every session event): keep the latest in a ref so
  // the effects below run once, and focus never jumps back to the frame.
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;
  // A modal dialog (WCAG 2.4.3): focus moves in once, Tab stays inside,
  // Escape closes it, and focus goes back to what opened it.
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    frameRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onCloseRef.current();
        return;
      }
      if (event.key !== "Tab" || !frameRef.current) return;
      const focusables = Array.from(
        frameRef.current.querySelectorAll<HTMLElement>("button:not(:disabled), a[href], [tabindex]:not([tabindex='-1'])"),
      );
      if (focusables.length === 0) return;
      const first = focusables[0]!;
      const last = focusables[focusables.length - 1]!;
      const inside = frameRef.current.contains(document.activeElement);
      if (event.shiftKey && (document.activeElement === first || !inside || document.activeElement === frameRef.current)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && (document.activeElement === last || !inside)) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      if (opener && opener.isConnected) opener.focus();
    };
  }, []);

  useEffect(() => {
    funnelStep("c-add-provider", "open");
    void providersDiscover().then(setInstalls);
    void servicesList().then(setServices, () => setServices([]));
  }, []);

  // Found counts, even when its version couldn't be read (0.0.11).
  const selectable = (row: ProviderInstallData): boolean => toolFound(row);

  const sorted = [...(installs ?? [])].sort(
    (a, b) => Number(selectable(b)) - Number(selectable(a)),
  );

  // AJ-E1601: a service picked under More opens its own sheet; closing it
  // comes back here.
  if (serviceFor && onServiceAdded) {
    return <ServiceAccountSheet serviceId={serviceFor} onClose={() => setServiceFor(null)} onAdded={onServiceAdded} />;
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 outline-none backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-label="Add an account: which tool?"
      ref={frameRef}
      tabIndex={-1}
    >
      <Card className="flex max-h-[86vh] w-full max-w-xl flex-col overflow-hidden p-0">
        <div className="flex items-center gap-3 border-b border-hairline/60 px-6 py-4">
          <span className="text-accent">
            <IconJuggler width={16} height={16} />
          </span>
          <h3 className="text-sm font-semibold">Add an account</h3>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5" data-testid="provider-cards">
          <p className="text-xs leading-relaxed text-muted">
            Which tool is it for? Installed tools are ready; others link to their install page.
          </p>
          {installs === null && <p className="mt-4 text-xs text-muted">Checking your machine…</p>}
          <ul className="mt-3 flex flex-col gap-2">
            {sorted.map((row) => {
              const ready = selectable(row);
              const desktop = row.kind === "desktop";
              return (
                <li
                  key={row.provider}
                  className="flex items-center gap-3 rounded-xl border border-hairline px-4 py-3"
                  data-testid="provider-card"
                  data-status={row.status}
                  data-kind={row.kind ?? "terminal"}
                >
                  <div className="flex min-w-0 flex-1 flex-col gap-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <ToolMark tool={row.provider} size={16} className="text-muted" />
                      <span className="text-xs font-medium text-ink">{row.display_name}</span>
                      {/* 0.0.11 walk, Q4: plain words here; the version and
                          what it was tested with are in the tool's own view. */}
                      {desktop && <Chip tone="accent">Desktop</Chip>}
                      <span className="text-[10px] text-muted" data-testid="provider-state">
                        {(row.status === "installed" || row.status === "unverified") && "Installed"}
                        {row.status === "not_installed" && "Not installed"}
                        {versionUnchecked(row) && FOUND_UNCHECKED}
                        {row.status === "probe_failed" && !row.binary && "Couldn't check"}
                      </span>
                    </div>
                    {desktop && row.single_instance_note && (
                      <span className="text-[10px] leading-relaxed text-muted">
                        {row.single_instance_note} — one session at a time with this tool.
                      </span>
                    )}
                  </div>
                  {ready ? (
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={() => {
                        funnelStep("c-add-provider", "choose");
                        onChoose(row.provider);
                      }}
                      data-testid="provider-choose"
                    >
                      {row.display_name} →
                    </Button>
                  ) : (
                    row.status === "not_installed" &&
                    row.install_url && (
                      <a
                        href={row.install_url}
                        target="_blank"
                        rel="noreferrer"
                        className="shrink-0 text-[11px] text-accent underline-offset-2 hover:underline"
                      >
                        Install {row.display_name} →
                      </a>
                    )
                  )}
                </li>
              );
            })}
          </ul>
          {onServiceAdded && services.length > 0 && (
            <details className="mt-5" data-testid="add-more">
              <summary className="cursor-pointer text-xs text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
                More
              </summary>
              <div className="mt-3 flex flex-col gap-2" data-testid="add-through-service">
                <p className="text-[11px] uppercase tracking-wider text-muted">Through another service</p>
                <p className="text-xs leading-relaxed text-muted">
                  A subscription that answers for your tools at the address it publishes. You paste its key once.
                </p>
                <ul className="flex flex-col gap-2">
                  {services.map((row) => (
                    <li key={row.service.id} className="flex items-center gap-3 rounded-xl border border-hairline px-4 py-3" data-testid="service-card" data-service={row.service.id}>
                      <LetterMark name={row.service.name} size={16} className="text-muted" />
                      <div className="flex min-w-0 flex-1 flex-col">
                        <span className="text-xs font-medium text-ink">{row.service.name}</span>
                        <span className="text-[10px] text-muted">{row.tools.map((tool) => providerDisplayName(tool.tool)).join(", ")}</span>
                      </div>
                      <Button size="sm" variant="secondary" onClick={() => setServiceFor(row.service.id)} data-testid="service-choose">
                        {row.service.name} →
                      </Button>
                    </li>
                  ))}
                </ul>
              </div>
            </details>
          )}
        </div>

        <div className="flex items-center justify-end border-t border-hairline/60 px-6 py-4">
          <Button size="sm" variant="secondary" onClick={onClose}>
            Close
          </Button>
        </div>
      </Card>

    </div>
  );
}
