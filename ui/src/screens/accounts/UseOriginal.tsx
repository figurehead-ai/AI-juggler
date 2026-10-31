/**
 * ADR-0018 — where a profile's folder is, in words, and switching a copy
 * to the folder it came from (moved here from Profiles.tsx, AJ-E2109).
 */
import { useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Modal } from "../../components/form";
import { sameAsWords } from "../../lib/providers";
import { accountUseOriginal, errorText, type AccountInfo } from "../../lib/bridge";
import { shortPath } from "../AccountManage";

/** Where a profile lives, in a few words — "uses ~/x where it is" or "a
 * copy of ~/x"; nothing for one made fresh here. */
export function placementWords(account: AccountInfo): string | null {
  if (!account.import_source_path) return null;
  const where = shortPath(account.import_source_path);
  return account.in_place ? `uses ${where} where it is` : `a copy of ${where}`;
}

/** Confirm switching a copy to its original folder — what changes, and
 * that nothing is deleted. */
export function UseOriginalModal({
  account,
  source,
  onClose,
  onDone,
}: {
  account: AccountInfo;
  source: string;
  onClose: () => void;
  onDone: () => Promise<void>;
}): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const where = shortPath(source);
  return (
    <Modal title="Use the original folder?" onClose={onClose}>
      <div className="flex flex-col gap-4 text-sm" data-testid="use-original">
        <p className="truncate font-mono text-xs text-ink" title={source}>
          {where}
        </p>
        <p className="leading-relaxed text-ink-2">
          {account.label} will use your original folder: the same sign-in, settings and history {sameAsWords([account.provider])}.
          AI Juggler's copy stays where it is (nothing is deleted); sessions you started in the copy stay in the copy, and
          won't resume from the original.
        </p>
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button onClick={onClose}>Not now</Button>
          <Button
            variant="primary"
            disabled={busy}
            data-testid="use-original-confirm"
            onClick={async () => {
              setBusy(true);
              setError(null);
              try {
                await accountUseOriginal(account.id);
                await onDone();
              } catch (cause) {
                setError(errorText(cause));
                setBusy(false);
              }
            }}
          >
            {busy ? "Switching…" : "Use the original"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
