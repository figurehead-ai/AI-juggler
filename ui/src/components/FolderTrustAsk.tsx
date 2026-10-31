/**
 * Lab 2 (D-40) — a tool's folder-trust question, asked in AI Juggler, once
 * per tool and folder, in the tool's own words ("Trust this folder?",
 * "Trust and continue", "Open restricted" for Codex). The answer is kept in
 * AI Juggler and passed to each launch in memory: the tool's own config is
 * never written, which its own trust screen would do.
 *
 * `confirm(profile, folder)` resolves `true` to go on (answered now, before,
 * in the tool itself, or a tool that asks nothing) and `false` on Cancel.
 */
import { useRef, useState, type JSX } from "react";
import { Modal } from "./form";
import { Button } from "./primitives";
import { errorText } from "../lib/bridge";
import { runtimeTrust, runtimeTrustChoose, type FolderTrustInfo } from "../lib/ipc/runtime";

export function useFolderTrust(): { confirm: (profileId: string, cwd: string | null) => Promise<boolean>; element: JSX.Element | null } {
  const [asking, setAsking] = useState<{ info: FolderTrustInfo; profileId: string; cwd: string | null } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const settle = useRef<((go: boolean) => void) | null>(null);
  const confirm = async (profileId: string, cwd: string | null): Promise<boolean> => {
    const info = await runtimeTrust(profileId, cwd).catch(() => null);
    if (!info || info.choice !== null) return true;
    setError(null);
    setAsking({ info, profileId, cwd });
    return new Promise<boolean>((resolve) => {
      settle.current = resolve;
    });
  };
  const done = (go: boolean) => {
    settle.current?.(go);
    settle.current = null;
    setAsking(null);
  };
  const answer = (trusted: boolean) => {
    if (!asking) return;
    void runtimeTrustChoose(asking.profileId, asking.cwd, trusted).then(
      () => done(true),
      (cause) => setError(errorText(cause)),
    );
  };
  const element = asking ? (
    <Modal title={asking.info.question} subtitle={asking.info.folder} onClose={() => done(false)} testId="folder-trust">
      <div className="flex flex-col gap-4 text-xs">
        <p className="leading-relaxed text-ink-2">{asking.info.note}</p>
        {error && <p className="text-warning">{error}</p>}
        <div className="flex flex-wrap justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={() => done(false)}>
            Cancel
          </Button>
          <Button size="sm" variant="secondary" onClick={() => answer(false)} data-testid="folder-trust-restrict">
            {asking.info.restrict}
          </Button>
          <Button size="sm" variant="primary" onClick={() => answer(true)} data-testid="folder-trust-trust">
            {asking.info.trust}
          </Button>
        </div>
      </div>
    </Modal>
  ) : null;
  return { confirm, element };
}
