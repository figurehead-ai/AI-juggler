import { useEffect, useState } from "react";
import { Button, Card, Chip } from "../components/primitives";
import { IconJuggler } from "../components/icons";
import { funnelStep } from "../lib/funnel";
import { OnboardingWizard } from "./OnboardingWizard";
import { providersDiscover, type ProviderInstallData } from "../lib/bridge";

/**
 * AJ-E306 flow (c): add a Provider in ≤ 4 clicks. One card per
 * adapter with its install state, verified-range badge and kind —
 * desktop cards state their single-instance limit BEFORE selection
 * (Req.3); not-installed cards offer [Install →] and are never
 * selectable (Req.1). Choosing a card runs the provider-scoped
 * wizard (LOCKED — flows (d)'s machinery: scan → proposal/identity →
 * queued sign-ins → Summary with its [Launch] and [Add another
 * account on this Provider] exits, Req.2).
 */
export function AddProviderSheet({
  onClose,
  onAccountCreated,
  openLoginTerminal,
}: {
  onClose: () => void;
  onAccountCreated: () => void;
  openLoginTerminal: (id: string, title: string) => void;
}) {
  const [installs, setInstalls] = useState<ProviderInstallData[] | null>(null);
  const [chosen, setChosen] = useState<ProviderInstallData | null>(null);

  useEffect(() => {
    funnelStep("c-add-provider", "open");
    void providersDiscover().then(setInstalls);
  }, []);

  const selectable = (row: ProviderInstallData): boolean =>
    row.status === "installed" || row.status === "unverified";

  const sorted = [...(installs ?? [])].sort(
    (a, b) => Number(selectable(b)) - Number(selectable(a)),
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <Card className="flex max-h-[86vh] w-full max-w-xl flex-col overflow-hidden p-0">
        <div className="flex items-center gap-3 border-b border-hairline/60 px-6 py-4">
          <span className="text-accent">
            <IconJuggler width={16} height={16} />
          </span>
          <h3 className="text-sm font-semibold">Add a Provider</h3>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5" data-testid="provider-cards">
          <p className="text-xs leading-relaxed text-muted">
            One card per tool AI Juggler knows. Installed Providers are ready to set up;
            others link to their install page.
          </p>
          {installs === null && <p className="mt-4 text-xs text-muted">Checking your machine…</p>}
          <ul className="mt-3 flex flex-col gap-2">
            {sorted.map((row) => {
              const ready = selectable(row);
              const desktop = row.kind === "desktop";
              const verified = (row.verified_against ?? []).length > 0;
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
                      <span className="text-xs font-medium text-ink">{row.display_name}</span>
                      {desktop && <Chip tone="accent">Desktop</Chip>}
                      {!desktop && <Chip tone="neutral">Terminal</Chip>}
                      <span className="text-[10px] text-muted">
                        {row.status === "installed" && `installed · ${row.version ?? ""}`}
                        {row.status === "unverified" &&
                          `found (${row.version ?? "unknown version"}) — not in the verified range`}
                        {row.status === "not_installed" && "not installed"}
                        {row.status === "probe_failed" && "check failed"}
                      </span>
                    </div>
                    <span className="text-[10px] text-muted" data-testid="verified-badge">
                      {verified
                        ? `Verified against ${row.verified_against!.join(", ")}`
                        : "Unverified — features may drift"}
                    </span>
                    {desktop && row.single_instance_note && (
                      <span className="text-[10px] leading-relaxed text-muted">
                        {row.single_instance_note} — one Instance at a time on this Provider.
                      </span>
                    )}
                  </div>
                  {ready ? (
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={() => {
                        funnelStep("c-add-provider", "choose");
                        setChosen(row);
                      }}
                      data-testid="provider-choose"
                    >
                      Set up {row.display_name} →
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
        </div>

        <div className="flex items-center justify-end border-t border-hairline/60 px-6 py-4">
          <Button size="sm" variant="secondary" onClick={onClose}>
            Close
          </Button>
        </div>
      </Card>

      {/* The chosen card runs the provider-scoped wizard INLINE —
          LOCKED (no multi-Provider gate), reusing the AJ-E305
          sign-ins queue and the Summary exits (Req.2). */}
      {chosen !== null && (
        <OnboardingWizard
          provider={chosen.provider}
          locked
          onClose={() => setChosen(null)}
          onAccountCreated={onAccountCreated}
          openLoginTerminal={openLoginTerminal}
        />
      )}
    </div>
  );
}
