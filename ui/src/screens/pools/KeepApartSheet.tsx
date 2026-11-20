/**
 * R4 (main's ruling, from the persona walks: three testers expected one
 * confirmation and got New pool's nine steps, clarity 2): "Keep it
 * apart…" on a project opens this one sheet. It says what keeping the
 * project apart does and doesn't do, in plain words. As a serious choice
 * it gives the consequence, an example, "What does this mean?" and the
 * guide.
 *
 * - **Keep apart** makes the pool the wizard's own start would make on
 *   this project, unchanged (`startDraft`), with the wizard's own calls
 *   (`makePool`). A toast names it, with Open it.
 * - **Customise…** opens that start in the wizard.
 * - **Cancel** makes nothing.
 *
 * D-126: a making screen, so it's drawn (the project's sessions inside
 * one closed pool), with its three facts beside their icons, and one
 * filled primary.
 */
import { useMemo, useState, type JSX, type SVGProps } from "react";

import { Alert } from "../../components/Alert";
import { Modal } from "../../components/form";
import { IconCheck } from "../../components/icons-controls";
import { IconKey, IconPool } from "../../components/icons-actions";
import { IconMessage } from "../../components/icons-canvas";
import { Button } from "../../components/primitives";
import { Explain } from "../../components/switch/Explain";
import { showToast } from "../../components/Toast";
import { LogoTile } from "../../components/ToolLogo";
import { errorText } from "../../lib/bridge";
import { useAgentBridge } from "../../lib/ipc/agent-bridge";
import { KEEP_APART_GUIDE, KEEP_APART_START, keepApartWords, type ApartFact } from "../../lib/keep-apart";
import { makePool, startDraft } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { joinSentence } from "../../lib/pool-words";
import { refreshPools, usePools } from "../../lib/pools";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { ApartArt } from "./illustrations";
import { sayNotAdded } from "./NewPoolFrame";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

const FACT_ICONS: Record<ApartFact["id"], Icon> = { only: IconPool, closed: IconKey, team: IconMessage };

/** The project's sessions inside one closed pool, named over the drawing. */
function Drawing({ project }: { project: string }): JSX.Element {
  return (
    <div aria-hidden="true" className="flex justify-center bg-canvas-hi bg-[radial-gradient(var(--color-hairline)_1px,transparent_1.2px)] bg-[length:14px_14px] px-4 pb-3 pt-4">
      <div className="relative aspect-[420/110] w-full max-w-[400px]">
        <span className="absolute inset-0">
          <ApartArt />
        </span>
        <span className="absolute left-[9%] top-[12.7%] max-w-[46%] -translate-y-1/2 truncate rounded-md bg-surface px-1.5 text-label font-semibold text-accent-hi" data-testid="keep-apart-art-project">
          {project}
        </span>
        <span className="absolute left-[88%] top-[74%] -translate-x-1/2 whitespace-nowrap text-label text-muted">Other pools</span>
      </div>
    </div>
  );
}

export function KeepApartSheet({
  project,
  onClose,
  onCustomise,
  onOpenPool,
}: {
  project: ProjectInfo;
  onClose: () => void;
  /** The wizard, on the same start. */
  onCustomise: () => void;
  /** The pool made (the toast's Open it). */
  onOpenPool: (id: string) => void;
}): JSX.Element {
  const pools = usePools();
  const bridge = useAgentBridge();
  const taken = useMemo(() => (pools?.pools ?? []).map((pool) => pool.name), [pools]);
  // The wizard's own start on this project, named after it, nothing changed.
  const draft = useMemo(() => startDraft(templateById(KEEP_APART_START), project.id, taken, project.name), [project.id, project.name, taken]);
  const words = keepApartWords(project.name);
  const everyone = draft.members.find((member) => member.ref === "all");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const keep = async () => {
    setBusy(true);
    setError(null);
    try {
      const { id, notAdded } = await makePool(draft);
      await refreshPools();
      onClose();
      showToast({ id: "keep-apart", tone: "success", message: words.made, detail: words.madeDetail(draft.name), action: { label: words.open, run: () => onOpenPool(id) }, testId: "keep-apart" });
      // A chosen member is never dropped silently (the wizard's round 6).
      void sayNotAdded(notAdded, bridge);
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };
  return (
    <Modal title={words.title} onClose={onClose} broad testId="keep-apart">
      <div className="flex max-h-[calc(100dvh-9rem)] flex-col">
        <div className="-mx-1 flex min-h-0 flex-col gap-4 overflow-y-auto px-1 pb-1">
          <div className="flex items-start gap-3">
            <LogoTile icon={IconPool} size={40} on />
            <p className="text-body text-ink" data-testid="keep-apart-lead">
              {words.lead}
            </p>
          </div>
          <section aria-label="What keeping it apart does" className="shrink-0 overflow-hidden rounded-card bg-surface inset-ring-1 inset-ring-hairline/75">
            <Drawing project={project.name} />
            <ul className="grid grid-cols-1 sm:grid-cols-3">
              {words.facts.map((fact) => (
                <li key={fact.id} className="flex flex-col gap-1.5 px-4 py-3.5 sm:border-l sm:border-divider sm:first:border-l-0" data-testid="keep-apart-fact" data-fact={fact.id}>
                  <span className="flex items-center gap-2.5 text-body font-semibold text-ink">
                    <LogoTile icon={FACT_ICONS[fact.id]} size={24} />
                    {fact.title}
                  </span>
                  <span className="text-meta text-ink-2">{fact.line}</span>
                </li>
              ))}
            </ul>
            <p className="border-t border-divider px-4 py-3 text-meta text-muted" data-testid="keep-apart-example">
              {words.example}
            </p>
          </section>
          <Alert level="info" title={`${words.notTitle}.`} live={null} testId="keep-apart-not">
            {words.not}
          </Alert>
          <Explain tips={words.tips} guide={KEEP_APART_GUIDE} testId="keep-apart-explain" />
          {error && (
            <Alert level="broken" title={`${words.failed}.`} testId="keep-apart-error">
              {error}
            </Alert>
          )}
        </div>
        <p className="mt-4 text-meta text-muted" data-testid="keep-apart-makes">
          {words.makes(draft.name, joinSentence(project.name, everyone?.role ?? null, draft.level))}
        </p>
        <div className="mt-4 flex items-center gap-2 border-t border-divider pt-4">
          <Button variant="ghost" onClick={onClose} data-testid="keep-apart-cancel">
            Cancel
          </Button>
          <span className="flex-1" />
          <Button variant="secondary" onClick={onCustomise} disabled={busy} data-testid="keep-apart-customise">
            {words.customise}
          </Button>
          <Button variant="primary" onClick={() => void keep()} disabled={busy} data-autofocus data-testid="keep-apart-keep">
            <IconCheck aria-hidden="true" width={16} height={16} strokeWidth={2.4} className="mr-1.5" />
            {busy ? "Keeping it apart…" : words.keep}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
