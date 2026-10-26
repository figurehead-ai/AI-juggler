/**
 * AJ-E1601 (D-34) — Add account › More › Through another service. A
 * subscription that runs Claude Code or Codex for you (OpenCode Go, Kimi)
 * becomes an account with a profile per tool, each in its own folder AI
 * Juggler makes and points at the service's published address.
 *
 * The key is pasted once into a masked field and goes straight to the
 * macOS Keychain; it is never shown again, never kept here once added,
 * and only that account's sessions are started with it.
 */
import { useEffect, useMemo, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { UsageChip } from "./ServiceParts";
import { Field, Input, Modal, Select } from "../../components/form";
import { LetterMark, ToolMark } from "../../components/ToolMark";
import { accountsList, errorText, providersDiscover, type AccountInfo } from "../../lib/bridge";
import { providerDisplayName, toolFound } from "../../lib/providers";
import { toolSpec } from "../../lib/tools";
import { serviceAccountAdd, servicesList, type ServiceInfo } from "../../lib/ipc/services";
import { freeLabel } from "../FreshAccountStep";

/** A tool's row: on or off, its model and its name (`null`: the default). */
type Pick = { on: boolean; model: string | null; label: string | null };

export function ServiceAccountSheet({
  serviceId,
  onClose,
  onAdded,
}: {
  serviceId: string;
  onClose: () => void;
  /** The account was added: its profiles. */
  onAdded: (profiles: AccountInfo[]) => void;
}): JSX.Element {
  const [service, setService] = useState<ServiceInfo | null>(null);
  const [taken, setTaken] = useState<string[]>([]);
  const [found, setFound] = useState<Set<string> | null>(null);
  const [plan, setPlan] = useState<string>("");
  const [key, setKey] = useState("");
  const [picks, setPicks] = useState<Record<string, Pick>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    void servicesList().then((rows) => alive && setService(rows.find((row) => row.service.id === serviceId) ?? null));
    void accountsList().then((rows) => alive && setTaken(rows.map((row) => row.label)), () => undefined);
    void providersDiscover().then(
      (rows) => alive && setFound(new Set(rows.filter(toolFound).map((row) => row.provider))),
      () => alive && setFound(new Set()),
    );
    return () => {
      alive = false;
    };
  }, [serviceId]);

  // The first tool found on this Mac starts on; the rest wait for a click.
  useEffect(() => {
    if (!service || found === null) return;
    const first = service.tools.find((tool) => found.has(tool.tool)) ?? service.tools[0];
    setPicks(Object.fromEntries(service.tools.map((tool) => [tool.tool, { on: tool.tool === first?.tool, model: null, label: null }])));
    setPlan(service.plans[0] ?? "");
  }, [service, found]);

  // Default names: the service's short name ("Go"), then with the tool's.
  const names = useMemo(() => {
    const out: Record<string, string> = {};
    const used = [...taken];
    for (const tool of service?.tools ?? []) {
      const pick = picks[tool.tool];
      if (!pick?.on) continue;
      const short = service!.service.short_name;
      const base = Object.keys(out).length === 0 ? short : `${short} ${toolSpec(tool.tool).short_name}`;
      const name = pick.label ?? freeLabel(base, used);
      out[tool.tool] = name;
      used.push(name);
    }
    return out;
  }, [service, picks, taken]);

  if (!service) {
    return (
      <Modal title="Through another service" onClose={onClose} testId="service-account-sheet">
        <p className="text-xs text-muted" role="status">Loading…</p>
      </Modal>
    );
  }
  const chosen = service.tools.filter((tool) => picks[tool.tool]?.on);
  const modelOf = (tool: ServiceInfo["tools"][number]) => picks[tool.tool]?.model ?? tool.models[0] ?? "";
  const ready = key.trim().length > 0 && chosen.length > 0 && chosen.every((tool) => modelOf(tool).trim().length > 0 && names[tool.tool]?.trim());
  const set = (tool: string, change: Partial<Pick>) => setPicks((all) => ({ ...all, [tool]: { ...all[tool]!, ...change } }));

  const add = async () => {
    setBusy(true);
    setError(null);
    try {
      const profiles = await serviceAccountAdd({
        service: service.service.id,
        plan: plan || null,
        key,
        profiles: chosen.map((tool) => ({ tool: tool.tool, label: names[tool.tool]!.trim(), model: modelOf(tool).trim() })),
      });
      setKey("");
      onAdded(profiles);
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };

  return (
    <Modal
      title={`Add ${service.service.name}`}
      subtitle={`Through another service: ${service.service.name} answers for the tools you pick, at the address it publishes.`}
      onClose={onClose}
      wide
      testId="service-account-sheet"
    >
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-3">
          <span className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-accent/20 bg-accent-soft text-accent-hi">
            <LetterMark name={service.service.name} size={18} />
          </span>
          <div className="min-w-0 text-xs text-muted">
            <span className="block font-medium text-ink">{service.service.name}</span>
            {service.vendor}
          </div>
        </div>
        {service.plans.length > 0 && (
          <Field label="Plan">
            <Select value={plan} onChange={(event) => setPlan(event.target.value)} data-testid="service-plan">
              {service.plans.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </Select>
          </Field>
        )}
        <Field label={service.service.key_name} hint="Paste it once. It's kept in the macOS Keychain, handed only to this account's sessions when they start, and never shown again.">
          <Input
            type="password"
            value={key}
            onChange={(event) => setKey(event.target.value)}
            autoComplete="off"
            spellCheck={false}
            placeholder="Paste it here"
            data-autofocus
            data-testid="service-key"
          />
        </Field>
        <fieldset className="flex flex-col gap-2" data-testid="service-tools">
          <legend className="mb-1 text-label uppercase tracking-[0.14em] text-faint">Use it with</legend>
          {service.tools.map((tool) => {
            const pick = picks[tool.tool];
            const name = providerDisplayName(tool.tool);
            return (
              <div key={tool.tool} className="flex flex-col gap-2 rounded-xl border border-hairline px-4 py-3" data-testid="service-tool" data-tool={tool.tool}>
                <label className="flex items-center gap-2 text-xs font-medium text-ink">
                  <input type="checkbox" checked={pick?.on ?? false} onChange={(event) => set(tool.tool, { on: event.target.checked })} data-testid="service-tool-on" />
                  <ToolMark tool={tool.tool} size={14} className="text-muted" />
                  {name}
                  {found !== null && !found.has(tool.tool) && <span className="text-label font-normal text-muted">Not installed</span>}
                </label>
                {pick?.on && (
                  <div className="grid gap-3 sm:grid-cols-2">
                    <Field label="Model">
                      {tool.models.length > 0 ? (
                        <Select value={modelOf(tool)} onChange={(event) => set(tool.tool, { model: event.target.value })} data-testid="service-model">
                          {tool.models.map((model) => (
                            <option key={model} value={model}>
                              {model}
                            </option>
                          ))}
                        </Select>
                      ) : (
                        <Input value={modelOf(tool)} onChange={(event) => set(tool.tool, { model: event.target.value })} placeholder={`As ${service.service.name} names it`} spellCheck={false} data-testid="service-model" />
                      )}
                    </Field>
                    <Field label="Profile name">
                      <Input value={names[tool.tool] ?? ""} onChange={(event) => set(tool.tool, { label: event.target.value })} data-testid="service-profile-name" />
                    </Field>
                  </div>
                )}
              </div>
            );
          })}
        </fieldset>
        <div className="flex items-start gap-2 text-label text-muted" data-testid="service-usage">
          <UsageChip usage={service.service.usage} />
          <span>{service.service.usage_words}</span>
        </div>
        {error && (
          <p className="text-xs text-danger" role="alert" data-testid="service-error">
            {error}
          </p>
        )}
        <div className="flex items-center justify-end gap-2">
          <Button size="sm" variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" disabled={!ready || busy} onClick={() => void add()} data-testid="service-add">
            {busy ? "Adding…" : "Add account"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
