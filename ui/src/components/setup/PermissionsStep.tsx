/**
 * AJ-E1545 step 2, "Let AI Juggler work with your Mac": one card per
 * permission this setup needs (the OS-prompt registry's shape, AJ-E1547;
 * `lib/setup-permissions`). Each says what it does, shows a short clip of
 * the effect, what macOS will show and what to click, what happens if you
 * say no, and that it is optional and can be changed later in Settings.
 * Allow raises the system's own prompt; the card sees the grant live
 * (re-checked while shown and when the window comes back) and turns to
 * ✓ Allowed. Skip for now is on every card that isn't allowed.
 */
import { useState, type JSX } from "react";
import { Button } from "../primitives";
import { Clip } from "../Clip";
import { useAccessActions } from "../OsAccessRows";
import { IconCheck } from "../icons-controls";
import { IconWindow } from "../icons-places";
import { IconGoTo } from "../icons-actions";
import { IconShield } from "../icons";
import { accessCard } from "../../lib/osAccess";
import { permissionClip } from "../../lib/clips";
import { fromAccess, type PermissionEntry } from "../../lib/setup-permissions";
import type { OsAccessInfo } from "../../types/generated/OsAccessInfo";
import { SYSTEM_NAME } from "../../lib/machine-words";

// One meaning per icon (ICN-1): Go to it brings a window forward; a window is what the preview shows.
const ICONS: Record<string, typeof IconWindow> = { focus: IconGoTo, preview: IconWindow };

function Fact({ label, children }: { label: string; children: string }): JSX.Element {
  return (
    <span className="flex flex-col gap-0.5">
      <span className="text-label text-muted">{label}</span>
      <span className="text-meta text-ink">{children}</span>
    </span>
  );
}

function PermissionCard({
  entry,
  skipped,
  onSkip,
  act,
}: {
  entry: PermissionEntry;
  skipped: boolean;
  onSkip: () => void;
  act: ReturnType<typeof useAccessActions>["act"];
}): JSX.Element {
  const card = accessCard(entry.source);
  const allowed = card.state === "allowed";
  const Icon = ICONS[entry.id] ?? IconShield;
  const clip = permissionClip(entry.id);
  // PAT-USE (6): the system's dialog can open behind this window, and
  // Allow then seems to do nothing; after Allow, the card says so.
  const [asked, setAsked] = useState(false);
  return (
    <li className={"flex flex-col gap-4 rounded-[14px] bg-surface p-5 inset-ring-1 " + (allowed ? "inset-ring-success/40" : "inset-ring-hairline")} data-testid={`setup-permission-${entry.id}`} data-state={allowed ? "allowed" : skipped ? "skipped" : card.state}>
      <div className="flex flex-wrap items-start gap-3">
        <span aria-hidden="true" className={"flex size-9 shrink-0 items-center justify-center rounded-[10px] " + (allowed ? "bg-accent-soft text-accent-hi" : "bg-surface-3 text-ink-2")}>
          <Icon width={18} height={18} />
        </span>
        <span className="flex min-w-0 flex-1 basis-60 flex-col gap-0.5">
          <span className="text-section font-semibold text-ink">{entry.title}</span>
          <span className="text-body text-ink-2">{entry.why}</span>
        </span>
        {allowed ? (
          <output className="flex h-9 items-center gap-1.5 text-body font-medium text-success" data-testid={`setup-permission-${entry.id}-allowed`}>
            <IconCheck width={16} height={16} strokeWidth={2.4} aria-hidden="true" />
            Allowed
          </output>
        ) : (
          <span className="flex flex-wrap items-center gap-2">
            {!skipped && (
              <Button variant="ghost" onClick={onSkip} data-testid={`setup-permission-${entry.id}-skip`}>
                Skip for now <span className="sr-only">{entry.title}</span>
              </Button>
            )}
            {card.secondary && (
              <Button variant="secondary" onClick={() => act(entry.source, card.secondary!.action)}>
                {card.secondary.label} <span className="sr-only">{entry.title}</span>
              </Button>
            )}
            {card.primary && (
              <Button
                variant={card.primary.action === "settings" ? "secondary" : "accent"}
                onClick={() => {
                  if (card.primary!.action === "request") setAsked(true);
                  act(entry.source, card.primary!.action);
                }}
                data-testid={`setup-permission-${entry.id}-allow`}
              >
                {card.primary.label}
                {card.primary.action !== "reopen" && <span className="sr-only"> {entry.title}</span>}
              </Button>
            )}
          </span>
        )}
      </div>
      {card.note && <p className={card.noteLeads ? "text-body text-ink" : "text-meta text-muted"}>{card.note}</p>}
      {asked && !allowed && card.state !== "reopen" && (
        <output className="block text-body text-ink" data-testid={`setup-permission-${entry.id}-asked`}>
          {SYSTEM_NAME}'s dialog is open. Don't see it? It can open behind this window: press ⌘Tab to find it. This card turns to Allowed as soon as you say yes.
        </output>
      )}
      {!allowed && (
        <div className="grid gap-4 sm:grid-cols-[minmax(0,240px)_minmax(0,1fr)]">
          {clip && <Clip id={clip} label={entry.title} />}
          <div className="flex flex-col gap-3 rounded-xl bg-surface-2 p-4" data-testid={`setup-permission-${entry.id}-prompt`}>
            {entry.picture ? (
              <img src={entry.picture} alt={`What ${SYSTEM_NAME} shows: ${entry.system_says}`} className="w-full rounded-lg" />
            ) : (
              <Fact label={`${SYSTEM_NAME} will show`}>{entry.system_says}</Fact>
            )}
            <Fact label="Click">{entry.click}</Fact>
            <Fact label="If you say no">{entry.if_declined}</Fact>
          </div>
        </div>
      )}
      <p className="text-meta text-muted">{skipped && !allowed ? "Skipped for now. Allow it here or later in Settings." : `${SYSTEM_NAME} will ask you · optional · change it later in Settings`}</p>
    </li>
  );
}

export function PermissionsStep({ sources, setSources }: { sources: OsAccessInfo[]; setSources: (next: OsAccessInfo[]) => void }): JSX.Element {
  const [skipped, setSkipped] = useState<ReadonlySet<string>>(new Set());
  const { error, reopenBlock, setReopenBlock, act } = useAccessActions(sources, setSources);
  const entries = sources.map(fromAccess).filter((entry) => entry.needed);
  return (
    <div className="flex flex-col gap-4" data-testid="setup-permissions">
      <p className="text-body text-ink-2">
        Each one is optional: AI Juggler works without them. Allow opens the system's own prompt, and this page sees your answer as soon as you give it.
      </p>
      <ul className="flex flex-col gap-4">
        {entries.map((entry) => (
          <PermissionCard key={entry.id} entry={entry} skipped={skipped.has(entry.id)} onSkip={() => setSkipped((now) => new Set([...now, entry.id]))} act={act} />
        ))}
      </ul>
      {reopenBlock && (
        <div className="flex flex-wrap items-center gap-3 text-body" role="alert" data-testid="setup-reopen-block">
          <span className="text-ink">{reopenBlock.reason}</span>
          {reopenBlock.can_reopen_anyway && (
            <Button size="sm" variant="secondary" onClick={() => entries[0] && act(entries[0].source, "reopen", true)}>
              Reopen anyway
            </Button>
          )}
          <Button size="sm" variant="ghost" onClick={() => setReopenBlock(null)}>
            Not now
          </Button>
        </div>
      )}
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

