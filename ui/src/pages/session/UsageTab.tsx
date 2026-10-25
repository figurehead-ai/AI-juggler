/**
 * AJ-E2108 — a session page's Usage tab: its tokens by kind (input,
 * output, cache read and written) as one bar and a table, the model(s) it
 * ran on, and its account's window beside them. Every part says when it
 * isn't measured, laid out like the rest. One small component so a richer
 * one (charts over time) can take its place.
 */
import { useEffect, useState, type JSX } from "react";
import { Card } from "../../components/primitives";
import type { AccountInfo, QuotaDisplayData } from "../../lib/bridge";
import { IconUsage } from "../../components/icons";
import { conversationsLastMessages } from "../../lib/ipc/conversations";
import { CardTitle } from "./CardTitle";
import { Runway } from "./Runway";

export type TokenKinds = { input: number; output: number; cache_read: number; cache_write: number };

const KINDS: { key: keyof TokenKinds; label: string; swatch: string }[] = [
  { key: "input", label: "Input", swatch: "bg-accent" },
  { key: "output", label: "Output", swatch: "bg-live" },
  { key: "cache_read", label: "Read from cache", swatch: "bg-ink/35" },
  { key: "cache_write", label: "Written to cache", swatch: "bg-warning" },
];

/** A count in a few characters: "52k", "1.4M", "940". */
export function shortCount(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 100_000) return `${Math.round(n / 1000)}k`;
  if (n >= 1_000) return `${(n / 1000).toFixed(1)}k`;
  return `${n}`;
}

/** The models an assistant turn named in the end of its transcript, read
 * once while the tab is shown: `null` until read (or not readable). */
export function useTranscriptModels(profileId: string, conversationId: string | null, visible: boolean, known: string | null): { models: string[] | null; why: string | null } {
  const [read, setRead] = useState<{ key: string; models: string[] | null; why: string | null } | null>(null);
  const key = `${profileId}\u0000${conversationId ?? ""}`;
  useEffect(() => {
    if (known || !visible || conversationId === null || read?.key === key) return;
    let alive = true;
    conversationsLastMessages(profileId, conversationId, { max: 60 }).then(
      (info) => {
        if (!alive) return;
        if (info.state !== "shown") {
          setRead({ key, models: null, why: info.state === "off" ? "Last messages are off, so its transcript isn't read" : "Its transcript can't be read here" });
          return;
        }
        const models = [...new Set(info.turns.map((turn) => turn.model).filter((model): model is string => !!model))];
        setRead({ key, models, why: null });
      },
      () => alive && setRead({ key, models: null, why: "Its transcript couldn't be read just now" }),
    );
    return () => {
      alive = false;
    };
  }, [known, visible, key, profileId, conversationId, read?.key]);
  if (known) return { models: [known], why: null };
  return read?.key === key ? { models: read.models, why: read.why } : { models: null, why: null };
}

export function SessionUsageTab({
  tokens,
  models,
  modelsWhy,
  display,
  accountTitle,
  profile = null,
}: {
  tokens: TokenKinds | null;
  /** The models it ran on; `null` while unknown. */
  models: string[] | null;
  /** Why the models aren't known, when they aren't. */
  modelsWhy?: string | null;
  display: QuotaDisplayData | undefined;
  accountTitle: string | null;
  /** Its profile (the runway's "Measure its limits"). */
  profile?: AccountInfo | null;
}): JSX.Element {
  const total = tokens ? tokens.input + tokens.output + tokens.cache_read + tokens.cache_write : 0;
  const side = (
    <>
      <Card className="flex flex-col gap-3 p-5" data-testid="session-usage-models">
        <CardTitle>{models && models.length > 1 ? "Models" : "Model"}</CardTitle>
        {models && models.length > 0 ? (
          <ul className="flex flex-wrap gap-2">
            {models.map((model) => (
              <li key={model} className="rounded-full border border-hairline bg-surface-2 px-2.5 py-0.5 font-mono text-meta text-ink">
                {model}
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-meta leading-[1.5] text-muted">{modelsWhy ?? (models ? "Not recorded in the end of its transcript." : "Reading its transcript…")}</p>
        )}
      </Card>
      <Runway display={display} title={accountTitle} heading="Its account's window" testId="session-usage-runway" profile={profile}>
        <p className="border-t border-hairline pt-3 text-label leading-[1.5] text-muted">This session's share isn't measured: the window counts every session on the account.</p>
      </Runway>
    </>
  );
  // Nothing counted yet: one line and what would count it (the review of
  // 0.1.5: "Not measured" filled a whole card), then the model and the
  // account's window side by side.
  if (!tokens || total === 0) {
    return (
      <div className="@container w-full" data-testid="session-usage">
        <div className="mx-auto flex w-full max-w-[1440px] flex-col gap-4">
          <Card className="flex items-center gap-4 px-5 py-4" data-testid="session-usage-tokens">
            <span aria-hidden="true" className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-hairline bg-surface-2 text-muted">
              <IconUsage width={18} height={18} />
            </span>
            <div className="flex min-w-0 flex-col gap-0.5" data-testid="session-usage-none">
              <p className="text-body font-semibold text-ink">No token counts for it yet</p>
              <p className="text-meta leading-[1.5] text-muted">They show here once its app writes them on this Mac: input, output and what came from the cache.</p>
            </div>
          </Card>
          <div className="grid grid-cols-1 items-start gap-4 @4xl:grid-cols-2">{side}</div>
        </div>
      </div>
    );
  }
  return (
    <div className="@container w-full" data-testid="session-usage">
      <div className="mx-auto grid w-full max-w-[1440px] grid-cols-1 items-start gap-4 @4xl:grid-cols-[minmax(0,1fr)_340px]">
        <Card className="flex flex-col gap-5 p-5" data-testid="session-usage-tokens">
          <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
            <CardTitle>Tokens</CardTitle>
            <span className="text-meta text-muted">each message counted once</span>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-stat font-semibold leading-none tabular-nums text-ink" data-testid="session-usage-total" title={total.toLocaleString()}>
              {shortCount(total)}
            </span>
            <span className="text-body text-muted">tokens in all</span>
          </div>
          <div className="flex h-3 w-full overflow-hidden rounded-full bg-surface-3" role="img" aria-label={KINDS.map((kind) => `${kind.label} ${Math.round((tokens[kind.key] / total) * 100)}%`).join(", ")}>
            {KINDS.map((kind) =>
              tokens[kind.key] > 0 ? <span key={kind.key} className={"h-full " + kind.swatch} style={{ width: `${(tokens[kind.key] / total) * 100}%` }} /> : null,
            )}
          </div>
          <table className="w-full text-body">
            <caption className="sr-only">Tokens by kind</caption>
            <thead>
              <tr className="text-left text-meta text-muted">
                <th scope="col" className="pb-2 font-medium">
                  Kind
                </th>
                <th scope="col" className="pb-2 text-right font-medium">
                  Tokens
                </th>
                <th scope="col" className="w-20 pb-2 text-right font-medium">
                  Share
                </th>
              </tr>
            </thead>
            <tbody>
              {KINDS.map((kind) => (
                <tr key={kind.key} className="border-t border-hairline" data-testid="session-usage-kind">
                  <th scope="row" className="py-2 text-left font-normal text-ink">
                    <span className="inline-flex items-center gap-2.5">
                      <span aria-hidden="true" className={"size-2.5 rounded-sm " + kind.swatch} />
                      {kind.label}
                    </span>
                  </th>
                  <td className="py-2 text-right tabular-nums text-ink">{tokens[kind.key].toLocaleString()}</td>
                  <td className="py-2 text-right tabular-nums text-muted">{Math.round((tokens[kind.key] / total) * 100)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
        <div className="flex flex-col gap-4">{side}</div>
      </div>
    </div>
  );
}
