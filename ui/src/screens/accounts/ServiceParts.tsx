/**
 * AJ-E1601 (D-34) — an account through another service on the Accounts
 * page: its usage, said as honestly as the service allows (an estimate
 * from what its sessions used, or unknown), and a new key for it, pasted
 * once into a masked field and kept in the macOS Keychain.
 */
import { useState, type JSX } from "react";
import { Button, Chip } from "../../components/primitives";
import { Field, Input, Modal } from "../../components/form";
import { errorText, type QuotaDisplayData } from "../../lib/bridge";
import { tokenWords } from "../../lib/quotaWindow";
import { serviceAccountSetKey, type AccountServiceInfo } from "../../lib/ipc/services";
import type { SubscriptionInfo } from "../../lib/subscriptions";

/** "Estimate" or "Unknown": how honest the account's usage can be. */
export function UsageChip({ usage }: { usage: string }): JSX.Element {
  return <Chip tone={usage === "estimate" ? "accent" : "neutral"}>{usage === "estimate" ? "Estimate" : "Unknown"}</Chip>;
}

/** What its profiles' sessions used in the last 5 hours and the last week
 * (what AI Juggler saw: the estimate's input, never the service's own
 * reading). */
function usedHere(sub: SubscriptionInfo, displays: QuotaDisplayData[]): { five: number; week: number } {
  const mine = displays.filter((display) => sub.profiles.some((profile) => profile.id === display.account));
  const sum = (pick: (display: QuotaDisplayData) => number | undefined) => mine.reduce((total, display) => total + (pick(display) ?? 0), 0);
  // In and out, as every screen counts tokens (the 0.2 walk, F15).
  const io = (window: QuotaDisplayData["five_hour"]) => window?.burned_io_tokens ?? window?.burned_tokens;
  return { five: sum((display) => io(display.five_hour)), week: sum((display) => io(display.weekly)) };
}

/** The account card's usage: the chip, the service's rule in a line, what
 * its sessions used here (an estimate only), and where the service shows
 * its own figures. */
export function ServiceUsage({ sub, service, displays }: { sub: SubscriptionInfo; service: AccountServiceInfo; displays: QuotaDisplayData[] }): JSX.Element {
  const used = usedHere(sub, displays);
  return (
    <div className="flex flex-col gap-1.5 text-label text-muted" data-testid="account-service-usage" data-usage={service.usage}>
      <span className="flex items-start gap-2">
        <UsageChip usage={service.usage} />
        <span>{service.usage_words}</span>
      </span>
      {service.usage === "estimate" && (used.five > 0 || used.week > 0) && (
        <span data-testid="account-service-used">
          Its sessions used about {tokenWords(used.five)} tokens in and out in the last 5 hours and {tokenWords(used.week)} this week.
        </span>
      )}
      <a href={service.console_url} target="_blank" rel="noreferrer" className="w-fit text-accent-hi underline-offset-2 hover:underline">
        See it on {service.name}'s site →
      </a>
    </div>
  );
}

/** A new key for the account `profileId` is on: every profile on it
 * starts with it from then on. */
export function ReplaceKeyModal({ profileId, service, onClose, onDone }: { profileId: string; service: AccountServiceInfo; onClose: () => void; onDone: () => void }): JSX.Element {
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await serviceAccountSetKey(profileId, key);
      setKey("");
      onDone();
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };
  return (
    <Modal title={`New key · ${service.name}`} subtitle="Every profile on this account starts with it from now on." onClose={onClose} testId="service-key-modal">
      <div className="flex flex-col gap-4">
        <Field label={service.key_name} hint="Paste it once. It's kept in the macOS Keychain and never shown again.">
          <Input type="password" value={key} onChange={(event) => setKey(event.target.value)} autoComplete="off" spellCheck={false} placeholder="Paste it here" data-autofocus data-testid="service-new-key" />
        </Field>
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" disabled={busy || key.trim().length === 0} onClick={() => void save()} data-testid="service-save-key">
            {busy ? "Saving…" : "Save key"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
