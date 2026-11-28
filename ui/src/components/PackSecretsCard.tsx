import { primaryToolId } from "../lib/tools";
import { useCallback, useEffect, useState, type JSX } from "react";
import { Card, Chip } from "./primitives";
import { SwitchSetUpsCard } from "./SwitchSetUpsCard";
import { PACK_POLICY_ICONS, PACK_POLICY_OPTIONS, packPolicyRestatement, type PackPolicyWire } from "./PackPolicyChooser";
import { providerDisplayName } from "../lib/providers";
import { errorText } from "../lib/bridge";
import { redactionPolicyGet, redactionPolicySetGlobal, redactionPolicySetOverride, type RedactionInfoData } from "../lib/ipc/redaction";
import { CardTitle } from "./CardTitle";
import { ChoiceOption, PickRow, Picker } from "./controls";
import { ToolMark } from "./ToolMark";

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
          <CardTitle>Keys and passwords</CardTitle>
          <p className="mt-0.5 text-meta text-ink-2">
            When you switch to another tool, or save a handover, the other tool reads the
            conversation. Choose what happens to keys and passwords in it — remembered for every
            tool unless one has its own. A switch between apps of the same tool copies the
            conversation as it is, on this computer.
          </p>
        </div>
        <fieldset className="flex flex-col gap-1.5" data-testid="pack-secrets-global">
          <legend className="mb-1 text-section font-semibold text-ink">
            Your choice (all tools)
          </legend>
          {global === null && (
            <p className="text-label text-muted" data-testid="pack-secrets-unset">
              Not chosen yet — your first switch to another tool asks.
            </p>
          )}
          {/* AJ-E1558 (D-126): each choice a card with its icon chip, the
              chosen one a fill, the recommended one its accent tag. */}
          {storedOptions.map((option) => (
            <ChoiceOption
              key={option.value}
              type="radio"
              look="card"
              name="pack-secrets-global"
              value={option.value}
              icon={PACK_POLICY_ICONS[option.value]}
              label={
                option.value === "strip" ? (
                  <span className="flex flex-wrap items-center gap-2">
                    {option.title.replace(" (recommended)", "")}
                    <span className="rounded-full bg-accent-soft px-2 py-px text-label font-medium text-accent-hi">Recommended</span>
                  </span>
                ) : (
                  option.title
                )
              }
              hint={option.consequence}
              checked={global === option.value}
              onChange={() =>
                void run(
                  () => redactionPolicySetGlobal(option.value),
                  packPolicyRestatement(option.value as PackPolicyWire, "global", ""),
                )
              }
            />
          ))}
        </fieldset>
        {providers.length > 0 && (
          <div className="flex flex-col gap-1.5" data-testid="pack-secrets-overrides">
            <p className="text-section font-semibold text-ink">
              Per-tool choice
            </p>
            {providers.map((provider) => {
              const row = info[provider];
              const name = providerDisplayName(provider);
              return (
                <PickRow key={provider} icon={<ToolMark tool={provider} size={20} />} on={Boolean(row?.override)} testId="provider-redaction">
                  <span className="min-w-28 font-medium text-ink">{name}</span>
                  <Picker choice
                    size="sm"
                    value={row?.override ?? ""}
                    onChange={(value) =>
                      void run(
                        () => redactionPolicySetOverride(provider, value || null),
                        value ? `${name} now overrides the global choice.` : `${name} follows your global choice.`,
                      )
                    }
                    ariaLabel={`Keys and passwords for ${name}`}
                    options={[
                      { value: "", label: `Use my choice${global ? "" : " (not chosen yet)"}` },
                      ...storedOptions.map((option) => ({ value: option.value, label: option.title, icon: PACK_POLICY_ICONS[option.value] })),
                    ]}
                  />
                  {row?.override && <Chip tone="accent">Override</Chip>}
                </PickRow>
              );
            })}
          </div>
        )}
        {status && (
          <p className="text-label text-muted" role="status">
            {status}
          </p>
        )}
        {error && (
          <p className="text-label text-danger" role="alert">
            {error}
          </p>
        )}
      </Card>
      <SwitchSetUpsCard />
    </>
  );
}
