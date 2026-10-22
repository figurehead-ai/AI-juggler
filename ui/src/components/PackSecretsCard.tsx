import { primaryToolId } from "../lib/tools";
import { useCallback, useEffect, useState, type JSX } from "react";
import { Card, Chip } from "./primitives";
import { SwitchSetUpsCard } from "./SwitchSetUpsCard";
import { PACK_POLICY_OPTIONS, packPolicyRestatement, type PackPolicyWire } from "./PackPolicyChooser";
import { providerDisplayName } from "../lib/providers";
import {
  errorText,
  redactionPolicyGet,
  redactionPolicySetGlobal,
  redactionPolicySetOverride,
  type RedactionInfoData,
} from "../lib/bridge";

/**
 * AJ-E309 Req.2 / AJ-E208 (D-14): Accounts → Pack secrets. The GLOBAL
 * choice (the same words as the first-run chooser) and one override
 * row per Provider in use ("Use global" clears it). Every write is
 * the user's click; an unset global says so plainly — the first
 * switch asks, nothing is assumed.
 */
export function PackSecretsCard({ providers }: { providers: string[] }): JSX.Element {
  const [info, setInfo] = useState<Record<string, RedactionInfoData>>({});
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const providerKey = providers.join("|");

  const reload = useCallback(async () => {
    const primary = primaryToolId();
    const targets = providers.length > 0 ? providers : primary ? [primary] : [];
    const rows = await Promise.all(
      targets.map(async (provider) => [provider, await redactionPolicyGet(provider)] as const),
    );
    setInfo(Object.fromEntries(rows));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [providerKey]);

  useEffect(() => {
    void reload().catch((cause) => setError(errorText(cause)));
  }, [reload]);

  const first = Object.values(info)[0];
  const global = (first?.global ?? null) as PackPolicyWire | null;
  const storedOptions = PACK_POLICY_OPTIONS.filter((option) => option.value !== "decide_later");

  const run = async (action: () => Promise<void>, done: string) => {
    setError(null);
    setStatus(null);
    try {
      await action();
      await reload();
      setStatus(done);
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  return (
    <>
      <Card className="flex flex-col gap-3 p-5" data-testid="pack-secrets">
        <div>
          <h3 className="text-sm font-semibold">Pack secrets</h3>
          <p className="mt-0.5 text-xs leading-relaxed text-muted">
            Context packs carry your work to another account when you switch. Choose how keys and
            tokens found in them are handled — remembered for every tool unless one has its own.
          </p>
        </div>
        <fieldset className="flex flex-col gap-1.5" data-testid="pack-secrets-global">
          <legend className="mb-1 text-[11px] font-medium uppercase tracking-wider text-muted">
            Your choice (all tools)
          </legend>
          {global === null && (
            <p className="text-[11px] text-muted" data-testid="pack-secrets-unset">
              Not chosen yet — your first switch asks before any pack is built.
            </p>
          )}
          {storedOptions.map((option) => (
            <label
              key={option.value}
              className={
                "flex items-start gap-2.5 rounded-lg border px-3 py-2 text-xs transition-colors " +
                (global === option.value ? "border-accent/40 bg-accent-soft" : "border-hairline hover:border-accent/30")
              }
            >
              <input
                type="radio"
                name="pack-secrets-global"
                value={option.value}
                checked={global === option.value}
                onChange={() =>
                  void run(
                    () => redactionPolicySetGlobal(option.value),
                    packPolicyRestatement(option.value as PackPolicyWire, "global", ""),
                  )
                }
                className="mt-0.5 size-3.5 shrink-0 accent-[var(--color-accent)]"
              />
              <span className="flex flex-col gap-0.5">
                <span className="font-medium text-ink">{option.title}</span>
                <span className="text-[11px] leading-relaxed text-muted">{option.consequence}</span>
              </span>
            </label>
          ))}
        </fieldset>
        {providers.length > 0 && (
          <div className="flex flex-col gap-1.5" data-testid="pack-secrets-overrides">
            <p className="text-[11px] font-medium uppercase tracking-wider text-muted">
              Per-tool choice
            </p>
            {providers.map((provider) => {
              const row = info[provider];
              const name = providerDisplayName(provider);
              return (
                <label key={provider} className="flex flex-wrap items-center gap-2 text-xs" data-testid="provider-redaction">
                  <span className="min-w-28 font-medium">{name}</span>
                  <select
                    value={row?.override ?? ""}
                    onChange={(event) =>
                      void run(
                        () => redactionPolicySetOverride(provider, event.target.value || null),
                        event.target.value ? `${name} now overrides the global choice.` : `${name} follows your global choice.`,
                      )
                    }
                    aria-label={`Pack secrets for ${name}`}
                    className="rounded-lg border border-hairline bg-surface-2 px-2 py-1 text-xs text-ink"
                  >
                    <option value="">Use my choice{global ? "" : " (not chosen yet)"}</option>
                    {storedOptions.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.title}
                      </option>
                    ))}
                  </select>
                  {row?.override && <Chip tone="accent">Override</Chip>}
                </label>
              );
            })}
          </div>
        )}
        {status && (
          <p className="text-[11px] text-muted" role="status">
            {status}
          </p>
        )}
        {error && (
          <p className="text-[11px] text-danger" role="alert">
            {error}
          </p>
        )}
      </Card>
      <SwitchSetUpsCard />
    </>
  );
}
