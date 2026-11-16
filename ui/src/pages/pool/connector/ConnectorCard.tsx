/**
 * AJ-E3013 (D-116, D-30's amendment): the consent card for AI Juggler's
 * connector in one desktop App, one component wherever an App without it
 * joins a pool (New pool's Who's in, the pool's inspector) or the switch
 * finds it missing.
 *
 * - None yet: "Add AI Juggler's connector to <App> so its sessions can
 *   talk in this pool", its consequence, an example and the guide's
 *   section; Add first shows the preview (each file and its exact text),
 *   then "Add to <App>" writes it. After: "A running session picks it up
 *   at its next start" and the App's own sentence. No restart is offered:
 *   nothing says one is safe.
 * - There: one line, and Remove, which first shows exactly what it
 *   deletes and names what someone changed (it stays).
 * - Changed, an older edition's, or a preview that refuses: said, with
 *   what fixes it; can't be added here (the Codex app): why, and that no
 *   guide can do it either.
 */
import { useRef, useState, type JSX, type ReactNode } from "react";

import { Button } from "../../../components/primitives";
import { errorText } from "../../../lib/bridge";
import { CONNECTOR, CONNECTOR_GUIDE, connectorChanged, previewSentence, shortPath, statusLine, useConnectorStatus } from "../../../lib/connector";
import { Disclosure } from "../../../components/controls";
import { connectorAdd, connectorPreview, connectorRemove, connectorRemoveAnyway, connectorStatus, type ConnectorPreviewInfo, type ConnectorRemovalInfo, type ConnectorStatusInfo } from "../../../lib/ipc/connector";
import { trustOpen } from "../../../lib/ipc/trust";
import { useAgentBridge, type AgentBridgeInfo } from "../../../lib/ipc/agent-bridge";
import { useReveal } from "../../../lib/reveal";
import { ConnectorFiles, RemoveFiles } from "./ConnectorFiles";
import { ConnectorGuide } from "./ConnectorGuide";

type Phase =
  | { kind: "idle" }
  | { kind: "preview"; preview: ConnectorPreviewInfo }
  | { kind: "guide"; preview: ConnectorPreviewInfo | null }
  | { kind: "added"; pickedUp: string | null }
  | { kind: "remove"; files: string[]; changed: string[] }
  | { kind: "removed"; removal: ConnectorRemovalInfo };

/** What every part of the card shares. */
interface Parts {
  app: string;
  appId: string;
  testId: string;
  poolId?: string;
  busy: boolean;
  heading: 3 | 4;
  state: string;
  /** The agent program's state (why the connector can't go in, in words). */
  bridge: AgentBridgeInfo | null;
  /** The App as the preview's sentence says it ("your Claude app (Work Max)"). */
  who: { name: string; kind?: string; profile?: string | null };
  where: string;
  failed: ReactNode;
  cancel: JSX.Element;
}

/** The card: its title (the `heading` level), what's in it, a failure. */
function Card({ parts, title, children }: { parts: Parts; title: string; children: ReactNode }): JSX.Element {
  const Title = parts.heading === 4 ? "h4" : "h3";
  return (
    <section aria-labelledby={`${parts.testId}-title`} className="flex flex-col gap-2 rounded-xl bg-surface-2 px-4 py-3.5 shadow-card" data-testid={parts.testId} data-state={parts.state}>
      <Title id={`${parts.testId}-title`} className="text-body font-semibold text-ink">
        {title}
      </Title>
      {children}
      {parts.failed}
    </section>
  );
}

export function ConnectorCard({
  appId,
  app,
  where = "this pool",
  poolId,
  heading = 3,
  testId = "connector",
  kind,
  profile,
}: {
  /** The App's kind and profile, for the preview's sentence ("Claude
   * app", "Work Max"). */
  kind?: string;
  profile?: string | null;
  appId: string;
  /** The App as named ("Claude · Work"). */
  app: string;
  /** What its sessions talk in ("this pool", "pools"). */
  where?: string;
  /** In a pool: the guide also counts a session's first call. */
  poolId?: string;
  heading?: 3 | 4;
  testId?: string;
}): JSX.Element | null {
  const status = useConnectorStatus(appId);
  const bridge = useAgentBridge();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const act = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const actions = {
    review: () => act(async () => setPhase({ kind: "preview", preview: await connectorPreview(appId) })),
    guide: () => act(async () => setPhase({ kind: "guide", preview: await connectorPreview(appId) })),
    add: () =>
      act(async () => {
        const next = await connectorAdd(appId);
        connectorChanged(appId);
        setPhase({ kind: "added", pickedUp: next.picked_up });
      }),
    askRemove: () =>
      act(async () => {
        const preview = await connectorPreview(appId);
        setPhase({ kind: "remove", files: preview.files.map((file) => file.path), changed: preview.status.changed });
      }),
    // All or nothing (93b495f28): refused for a file changed since Add,
    // the card then offers Remove anyway, naming it.
    remove: () =>
      act(async () => {
        try {
          const removal = await connectorRemove(appId);
          connectorChanged(appId);
          setPhase({ kind: "removed", removal });
        } catch (cause) {
          const now = await connectorStatus(appId);
          if (now.changed.length === 0) throw cause;
          setPhase({ kind: "remove", files: phase.kind === "remove" ? phase.files : now.changed, changed: now.changed });
        }
      }),
    removeAnyway: () =>
      act(async () => {
        const removal = await connectorRemoveAnyway(appId);
        connectorChanged(appId);
        setPhase({ kind: "removed", removal });
      }),
  };
  if (status === undefined) {
    return (
      <p className="text-meta text-muted" data-testid={`${testId}-loading`}>
        Checking {app} for AI Juggler's connector…
      </p>
    );
  }
  const failed = error && (
    <p className="text-meta text-danger" role="alert">
      {error}
    </p>
  );
  const parts: Parts = {
    app,
    appId,
    testId,
    poolId,
    busy,
    heading,
    state: status.state,
    bridge,
    who: { name: app, kind, profile },
    where,
    failed,
    cancel: (
      <Button size="sm" variant="ghost" disabled={busy} onClick={() => setPhase({ kind: "idle" })} data-autofocus data-testid={`${testId}-cancel`}>
        {CONNECTOR.cancel}
      </Button>
    ),
  };
  if (phase.kind !== "idle") {
    return (
      <Revealed key={phase.kind === "remove" && phase.changed.length > 0 ? "remove-changed" : phase.kind} on={!busy}>
        <PhaseView phase={phase} status={status} parts={parts} onAdd={() => void actions.add()} onRemove={() => void actions.remove()} onRemoveAnyway={() => void actions.removeAnyway()} />
      </Revealed>
    );
  }
  if (status.state === "current") {
    return (
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-meta text-ink-2" data-testid={`${testId}-there`}>
        <span>{statusLine(status, bridge)}</span>
        <Button size="sm" variant="ghost" disabled={busy} onClick={() => void actions.askRemove()} data-testid={`${testId}-remove`}>
          {CONNECTOR.remove}
        </Button>
        {failed}
      </div>
    );
  }
  return <Resting status={status} parts={parts} where={where} onReview={() => void actions.review()} onGuide={() => void actions.guide()} onRemove={() => void actions.askRemove()} />;
}

/** The danger button's tint on this card's raised surface: at /15 its red
 * stands 4.48:1 there in dark (axe, Remove anyway); at /10, 4.87:1. */
const DANGER_ON_CARD = "bg-danger/10! hover:bg-danger/20!";

/** A step that opens in the panel: in view in its list, Cancel focused
 * (the aij-test proof: a confirmation clipped at the panel's foot), once
 * it can take focus (Cancel is disabled while a call runs). */
function Revealed({ children, on }: { children: ReactNode; on: boolean }): JSX.Element {
  const ref = useRef<HTMLDivElement | null>(null);
  useReveal(ref, on);
  return <div ref={ref}>{children}</div>;
}

/** The guide's section, opened in the browser. */
function Docs({ testId }: { testId: string }): JSX.Element {
  return (
    <button type="button" onClick={() => void trustOpen({ kind: "guide", id: CONNECTOR_GUIDE })} className="self-start rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid={`${testId}-docs`}>
      {CONNECTOR.docs} ↗
    </button>
  );
}

/** Nothing under way: the ask (none yet, an older edition's), what was
 * changed, or why it can't go here. */
function Resting({ status, parts, where, onReview, onGuide, onRemove }: { status: ConnectorStatusInfo; parts: Parts; where: string; onReview: () => void; onGuide: () => void; onRemove: () => void }): JSX.Element {
  const { app, testId, busy } = parts;
  if (status.state === "unavailable") {
    return (
      <Card parts={parts} title={`${app} can't take AI Juggler's connector yet`}>
        <p className="text-meta text-ink" data-testid={`${testId}-why-not`}>
          {statusLine(status, parts.bridge)}
        </p>
        <p className="text-meta text-ink-2" data-testid={`${testId}-guide-none`}>
          There's no guide for it either, so its sessions stay Watched in pools: shown and counted, without the pool's tools.
        </p>
        <Docs testId={testId} />
      </Card>
    );
  }
  if (status.state === "changed") {
    return (
      <Card parts={parts} title={`AI Juggler's connector in ${app} was changed`}>
        <p className="text-meta text-ink" data-testid={`${testId}-changed`}>
          {statusLine(status, parts.bridge)}
        </p>
        <span className="flex flex-wrap gap-2">
          <Button size="sm" variant="secondary" disabled={busy} onClick={onRemove} data-testid={`${testId}-remove`}>
            {CONNECTOR.remove}
          </Button>
        </span>
      </Card>
    );
  }
  const outdated = status.state === "outdated";
  return (
    <Card parts={parts} title={CONNECTOR.title(app, where)}>
      {/* Where "connector" is first named, what it is (Casey's PC1 walk). */}
      <p className="text-meta text-ink" data-testid={`${testId}-meaning`}>
        {CONNECTOR.meaning}
      </p>
      {outdated && (
        <p className="text-meta text-ink" data-testid={`${testId}-outdated`}>
          {CONNECTOR.outdated}
        </p>
      )}
      <p className="text-meta text-ink-2">{CONNECTOR.consequence}</p>
      <p className="text-meta text-ink-2">{CONNECTOR.example}</p>
      <Docs testId={testId} />
      <span className="mt-1 flex flex-wrap gap-2">
        <Button size="sm" variant="primary" disabled={busy} onClick={onReview} data-testid={`${testId}-add`}>
          {outdated ? CONNECTOR.update : CONNECTOR.add}
        </Button>
        <Button size="sm" variant="ghost" disabled={busy} onClick={onGuide} data-testid={`${testId}-guide-open`}>
          {CONNECTOR.showGuide}
        </Button>
      </span>
    </Card>
  );
}

/** A step under way: the preview, the guide, what Remove deletes, and
 * what Add or Remove did. */
function PhaseView({ phase, status, parts, onAdd, onRemove, onRemoveAnyway }: { phase: Exclude<Phase, { kind: "idle" }>; status: ConnectorStatusInfo; parts: Parts; onAdd: () => void; onRemove: () => void; onRemoveAnyway: () => void }): JSX.Element {
  const { app, appId, testId, poolId, busy, cancel } = parts;
  switch (phase.kind) {
    case "preview":
      if (phase.preview.refused) {
        return (
          <Card parts={parts} title={`AI Juggler won't add its connector to ${app} as it is`}>
            <p className="text-meta text-ink" data-testid={`${testId}-refused`}>
              {phase.preview.refused}.
            </p>
            <ConnectorFiles files={phase.preview.files} testId={testId} />
            <ConnectorGuide appId={appId} app={app} guide={phase.preview.guide} poolId={poolId} why="You can add it yourself instead:" testId={testId} />
            <span className="flex gap-2">{cancel}</span>
          </Card>
        );
      }
      return (
        <Card parts={parts} title={`What AI Juggler writes into ${app}`}>
          {/* Casey's PC1 walk: one plain sentence first, the paths under a fold. */}
          <p className="text-meta text-ink" data-testid={`${testId}-preview-sentence`}>
            {previewSentence(phase.preview.files, parts.who, parts.where)}
          </p>
          <p className="text-meta text-ink-2">Nothing is written until you add it.</p>
          <Disclosure className="text-meta" summary={CONNECTOR.showFiles} testId={`${testId}-files-fold`}>
            <div className="mt-1.5">
              <ConnectorFiles files={phase.preview.files} testId={testId} />
            </div>
          </Disclosure>
          <span className="mt-1 flex flex-wrap gap-2">
            <Button size="sm" variant="primary" disabled={busy} onClick={onAdd} data-testid={`${testId}-confirm`}>
              {CONNECTOR.confirm(app)}
            </Button>
            {cancel}
          </span>
        </Card>
      );
    case "guide":
      return (
        <Card parts={parts} title={`Add AI Juggler's connector to ${app} by hand`}>
          <ConnectorGuide appId={appId} app={app} guide={phase.preview?.guide ?? null} poolId={poolId} why={status.state === "unavailable" ? statusLine(status, parts.bridge) : null} testId={testId} />
          <span className="flex gap-2">{cancel}</span>
        </Card>
      );
    case "added":
      return (
        <Card parts={parts} title={`AI Juggler's connector is in ${app}`}>
          <output className="block text-meta font-medium text-ink" data-testid={`${testId}-added`}>
            {CONNECTOR.added}
          </output>
          {phase.pickedUp && <p className="text-meta text-ink-2">{phase.pickedUp}</p>}
        </Card>
      );
    case "remove":
      return (
        <Card parts={parts} title={`What Remove takes out of ${app}`}>
          <RemoveFiles files={phase.files} changed={phase.changed} testId={testId} />
          {phase.changed.length > 0 && (
            <p className="text-meta text-ink" data-testid={`${testId}-remove-changed`}>
              {changedWords(phase.changed)} changed since AI Juggler added it, so Remove takes nothing out. Remove anyway takes the connector out whole, deleting {phase.changed.length === 1 ? "that file" : "those files"} too.
            </p>
          )}
          <span className="mt-1 flex flex-wrap gap-2">
            {phase.changed.length > 0 ? (
              <Button size="sm" variant="danger" className={DANGER_ON_CARD} disabled={busy} onClick={onRemoveAnyway} data-testid={`${testId}-remove-anyway`}>
                Remove anyway (also deletes {changedWords(phase.changed)})
              </Button>
            ) : (
              <Button size="sm" variant="danger" className={DANGER_ON_CARD} disabled={busy} onClick={onRemove} data-testid={`${testId}-confirm-remove`}>
                {CONNECTOR.confirmRemove(app)}
              </Button>
            )}
            {cancel}
          </span>
        </Card>
      );
    default:
      return <Removed removal={phase.removal} parts={parts} />;
  }
}

/** Changed files by name ("SKILL.md", "plugin.json and SKILL.md"). */
function changedWords(paths: readonly string[]): string {
  const names = paths.map((path) => path.split("/").pop() ?? path);
  return names.length <= 1 ? (names[0] ?? "a file") : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** What Remove did: all of it out, or what it left, named. */
function Removed({ removal, parts }: { removal: ConnectorRemovalInfo; parts: Parts }): JSX.Element {
  const { app, testId } = parts;
  const left = removal.kept.length;
  return (
    <Card parts={parts} title={left > 0 ? `AI Juggler took out what it could from ${app}` : `AI Juggler's connector is out of ${app}`}>
      <output className="block text-meta text-ink" data-testid={`${testId}-removed`}>
        {left > 0 ? `It left ${left === 1 ? "one file that changed since it wrote it" : `${left} files that changed since it wrote them`}: ${removal.kept.map(shortPath).join(", ")}.` : "A running session keeps it until it next starts."}
      </output>
    </Card>
  );
}
