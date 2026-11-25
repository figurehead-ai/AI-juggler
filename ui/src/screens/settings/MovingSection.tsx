/**
 * AJ-E2328: Settings › Desktop apps › Moving conversations. Each app
 * whose conversations move says its version and whether moves were
 * checked on it (req 9 c); a conversation an older move left split (its
 * worktree still belonging to the App it left) is a row with Repair…,
 * whose sheet names the Apps it quits and says the worktree's files are
 * never touched (req 7); a worktree another App's cleanup may remove is
 * an info line. Nothing found: only the version rows.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Modal } from "../../components/form";
import { Button, Chip } from "../../components/primitives";
import { AppIcon } from "../../components/AppIcon";
import { errorText } from "../../lib/bridge";
import { ownershipCheck, ownershipRepair, type OwnershipInfo } from "../../lib/ipc/ownership";
import { refreshKnownPlaces, useKnownPlaces } from "../../lib/places";
import type { OwnershipFindingInfo } from "../../types/generated/OwnershipFindingInfo";
import { PaneSection, RowNote, SettingRow } from "./parts";
import { IconDone } from "../../components/icons-menu";
import { IconGitBranch } from "../../components/icons";

/** "Claude app 5.11492.5 · moves checked". */
export function checkedWords(app: string, version: string | null, checked: boolean): string {
  return `${app}${version ? ` ${version}` : ""} · ${checked ? "moves checked" : "not yet checked"}`;
}

/** Names in words: "a", "a and b", "a, b and c". */
function andList(names: string[]): string {
  if (names.length < 2) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** What Repair quits and starts again, in words: the running Apps of
 * `names`; none: nothing. */
export function quitsWords(names: string[]): string {
  if (names.length === 0) return "Nothing: neither App is open now.";
  const one = names.length === 1;
  return `${andList(names)}: AI Juggler quits ${one ? "it" : "them"} first and starts ${one ? "it" : "them"} again after.`;
}

/** 0.5.1 F90: which of the repair's Apps run, read when the sheet opens
 * and kept current while it is open: the remembered places, the live
 * source the rail shows (an App started or quit is heard at once). Before,
 * the sheet said the check's snapshot from when Settings opened ("neither
 * App is open" while both ran). `null` until the first read since it
 * opened. */
function useRunningNow(finding: OwnershipFindingInfo): string[] | null {
  const known = useKnownPlaces();
  const [read, setRead] = useState(false);
  useEffect(() => {
    let live = true;
    void refreshKnownPlaces().finally(() => live && setRead(true));
    return () => {
      live = false;
    };
  }, []);
  if (!read || !known) return null;
  return finding.apps.filter((app) => known.some((place) => place.id === app.place_id && place.app_pid !== null)).map((app) => app.name);
}

function RepairSheet({ finding, busy, onCancel, onConfirm }: { finding: OwnershipFindingInfo; busy: boolean; onCancel: () => void; onConfirm: () => void }): JSX.Element {
  const running = useRunningNow(finding);
  const quits = running === null ? "Looking at which Apps are open…" : quitsWords(running);
  return (
    <Modal icon={IconGitBranch}
      title={`Repair “${finding.title}”?`}
      subtitle={`Its worktree ${finding.worktree} will belong to ${finding.holder}, which holds the conversation.`}
      onClose={onCancel}
      testId="ownership-repair-sheet"
      footer={
        <>
          <Button size="sm" variant="secondary" onClick={onCancel} data-autofocus>
            Cancel
          </Button>
          <Button size="sm" variant="primary" disabled={busy || running === null} onClick={onConfirm} data-testid="ownership-repair-confirm">
            {busy ? "Repairing…" : "Repair"}
          </Button>
        </>
      }
    >
      <dl className="flex flex-col gap-3 text-body" data-modal-body>
        <div className="flex flex-col gap-0.5">
          <dt className="text-meta font-semibold text-ink">Quits and starts again</dt>
          <dd className="text-ink-2" data-testid="ownership-repair-quits">
            {quits}
          </dd>
        </div>
        <div className="flex flex-col gap-0.5">
          <dt className="text-meta font-semibold text-ink">Never touched</dt>
          <dd className="text-ink-2" data-testid="ownership-repair-files">
            The worktree's files: AI Juggler changes only which App it belongs to, and can undo that.
          </dd>
        </div>
        {finding.notes.length > 0 && (
          <div className="flex flex-col gap-0.5">
            <dt className="text-meta font-semibold text-ink">Also</dt>
            {finding.notes.map((note) => (
              <dd key={note} className="text-ink-2">
                {note}.
              </dd>
            ))}
          </div>
        )}
      </dl>
    </Modal>
  );
}

export function MovingSection(): JSX.Element | null {
  const [info, setInfo] = useState<OwnershipInfo | null>(null);
  const [asking, setAsking] = useState<OwnershipFindingInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [said, setSaid] = useState<{ key: string; words: string; failed: boolean } | null>(null);
  const load = useCallback(() => {
    void ownershipCheck().then(setInfo, () => setInfo(null));
  }, []);
  useEffect(load, [load]);
  if (!info || (info.apps.length === 0 && info.findings.length === 0 && info.tidied.length === 0 && !info.unreadable)) return null;
  const repair = (finding: OwnershipFindingInfo) => {
    setBusy(true);
    ownershipRepair(finding.key).then(
      (done) => setSaid({ key: finding.key, words: [done.words, ...done.notes.map((n) => `${n}.`)].join(" "), failed: false }),
      (cause) => setSaid({ key: finding.key, words: errorText(cause), failed: true }),
    ).finally(() => {
      setBusy(false);
      setAsking(null);
      load();
    });
  };
  const done = said && !said.failed && !info.findings.some((f) => f.key === said.key) ? said : null;
  return (
    <PaneSection
      id="moving"
      title="Moving conversations"
      footer="A move takes a conversation to another App with its history and its worktree. AI Juggler moves only on app versions it has checked, and checks that each worktree belongs to the App that holds its conversation."
    >
      {info.apps.map((app) => (
        <SettingRow
          key={app.app_id}
          title={checkedWords(app.app, app.version, app.checked)}
          description={app.checked ? "Move and Repair are available." : "Move and Repair wait for an AI Juggler update that checks this version; Copy still works."}
          icon={<AppIcon appId={app.app_id} name={app.app} size={24} />}
          testId={`moving-app-${app.app_id}`}
          control={<Chip tone={app.checked ? "success" : "neutral"}>{app.checked ? "Checked" : "Not yet"}</Chip>}
        />
      ))}
      {info.unreadable && (
        <SettingRow icon={IconGitBranch} title="Worktrees couldn't be checked" testId="moving-unreadable">
          <RowNote tone="warning">{info.unreadable}</RowNote>
        </SettingRow>
      )}
      {info.findings.map((finding) => (
        <SettingRow icon={IconGitBranch}
          key={finding.key}
          title={finding.title}
          description={finding.words}
          testId="moving-finding"
          control={
            <Button size="sm" variant="secondary" onClick={() => (setSaid(null), setAsking(finding))} data-testid="moving-finding-repair">
              Repair…
            </Button>
          }
        >
          {finding.notes.map((note) => (
            <RowNote key={note}>{`${note}.`}</RowNote>
          ))}
          {said?.key === finding.key && <RowNote tone={said.failed ? "warning" : "info"}>{said.words}</RowNote>}
        </SettingRow>
      ))}
      {done && (
        <SettingRow icon={IconDone} title="Repaired" testId="moving-repaired">
          <RowNote>{done.words}</RowNote>
        </SettingRow>
      )}
      {info.tidied.map((one) => (
        <SettingRow icon={IconGitBranch} key={`${one.worktree}-${one.words}`} title={`Worktree ${one.worktree}`} testId="moving-tidied">
          <RowNote>{`${one.words}.`}</RowNote>
        </SettingRow>
      ))}
      {asking && <RepairSheet finding={asking} busy={busy} onCancel={() => setAsking(null)} onConfirm={() => repair(asking)} />}
    </PaneSection>
  );
}
