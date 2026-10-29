/**
 * AJ-E1501 — managing an account from its card: rename it (and set its
 * plan), and remove it. Its own file, so Accounts stays a list (C.10).
 */
import { AccountSettingsSheet } from "../components/AccountSettingsSheet";
import { useState, type ReactNode } from "react";
import { Button } from "../components/primitives";
import { Field, Input, Modal } from "../components/form";
import { accountRemove, accountRename, accountSetDefault, errorText, type AccountInfo, type InstanceInfo } from "../lib/bridge";
import type { ActionHost } from "../lib/actions";
import { Radio } from "../components/controls";

/** A home-relative path, for display. */
export function shortPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~").replace(/^\/home\/[^/]+/, "~");
}

/** AJ-E1501: rename an account (and set its plan). A name another
 * account has is refused in words. */
export function RenameAccountModal({
  account,
  onClose,
  onDone,
}: {
  account: AccountInfo;
  onClose: () => void;
  /** `before`: the name when the sheet opened. */
  onDone: (renamed: AccountInfo, before: string) => Promise<void>;
}) {
  const [before] = useState(account.label);
  const [label, setLabel] = useState(account.label);
  const [plan, setPlan] = useState(account.plan ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <Modal title={`Rename ${account.label}`} onClose={onClose}>
      <form
        className="flex flex-col gap-4"
        data-testid="rename-account"
        onSubmit={async (event) => {
          event.preventDefault();
          setBusy(true);
          setError(null);
          try {
            await onDone(await accountRename(account.id, label, plan.trim() === "" ? null : plan), before);
          } catch (cause) {
            setError(errorText(cause));
            setBusy(false);
          }
        }}
      >
        <Field label="Name">
          <Input autoFocus value={label} onChange={(event) => setLabel(event.target.value)} data-testid="rename-account-label" maxLength={64} />
        </Field>
        <Field label="Plan (optional)" hint="Shown beside the name, like Max or Pro.">
          <Input value={plan} placeholder="Max" onChange={(event) => setPlan(event.target.value)} data-testid="rename-account-plan" />
        </Field>
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" type="submit" disabled={busy || label.trim() === ""} data-testid="rename-account-save">
            {busy ? "Saving…" : "Save"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

const OPEN = ["starting", "running", "waiting_user", "stuck", "quota_blocked"];

/** AJ-E1501: the remove sheet. An account used where it is: its folder
 * stays exactly as it is. A copy: keep its folder, or move it to the
 * Trash (never deleted). Open sessions keep it, named. */
export function RemoveAccountModal({
  account,
  sessions,
  onClose,
  onDone,
}: {
  account: AccountInfo;
  sessions: InstanceInfo[];
  onClose: () => void;
  onDone: (words: string) => Promise<void>;
}) {
  const [folder, setFolder] = useState<"keep" | "trash">("keep");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const open = sessions.filter((session) => OPEN.includes(session.status));
  const names = [...new Set(open.map((session) => session.git_branch ?? ((session.cwd ?? "").replace(/\/+$/, "").split("/").pop() || session.account_label)))];
  const where = account.import_source_path ? shortPath(account.import_source_path) : null;
  return (
    <Modal title={`Remove ${account.label}?`} onClose={onClose}>
      <div className="flex flex-col gap-4 text-sm" data-testid="remove-account" data-kind={account.in_place ? "in-place" : "copy"}>
        {open.length > 0 ? (
          <p className="leading-relaxed text-muted" data-testid="remove-account-open">
            {account.label} has {open.length === 1 ? "a session" : `${open.length} sessions`} open: <span className="text-ink">{names.join(", ")}</span>.
            Stop {open.length === 1 ? "it" : "them"} first, then remove the account.
          </p>
        ) : account.in_place ? (
          <>
            <p className="leading-relaxed text-muted">
              AI Juggler forgets this account. Your folder stays exactly as it is: nothing in it is moved or changed.
            </p>
            {where && (
              <p className="truncate font-mono text-xs text-ink" title={account.import_source_path ?? undefined}>
                {where}
              </p>
            )}
          </>
        ) : (
          <>
            <p className="leading-relaxed text-muted">
              AI Juggler forgets this account.{" "}
              {where ? (
                <>
                  It's a copy of <span className="font-mono text-xs text-ink">{where}</span>, which stays as it is. What should happen to the copy?
                </>
              ) : (
                "What should happen to its folder in AI Juggler's data?"
              )}
            </p>
            <div className="flex flex-col gap-1.5" role="radiogroup" aria-label="The account's folder">
              {(
                [
                  { id: "keep", label: "Keep the folder", what: "It stays in AI Juggler's data. Nothing is deleted." },
                  { id: "trash", label: "Move it to the Trash", what: "You can put it back from the Trash." },
                ] as const
              ).map((option) => (
                <label
                  key={option.id}
                  className={
                    "flex items-center gap-2 rounded-lg border px-3 py-1.5 text-sm " +
                    (folder === option.id ? "border-accent/60 bg-accent-soft" : "border-hairline")
                  }
                >
                  <Radio
                    name={`remove-folder-${account.id}`}
                    checked={folder === option.id}
                    onChange={() => setFolder(option.id)}
                    data-testid={`remove-folder-${option.id}`}
                  />
                  <span className="flex flex-col">
                    <span>{option.label}</span>
                    <span className="text-label text-muted">{option.what}</span>
                  </span>
                </label>
              ))}
            </div>
          </>
        )}
        {open.length === 0 && <p className="text-xs text-muted">Its past sessions stay in History, marked “removed account”.</p>}
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            {open.length > 0 ? "Close" : "Cancel"}
          </Button>
          {open.length === 0 && (
            <Button
              size="sm"
              variant="danger"
              disabled={busy}
              data-testid="remove-account-confirm"
              onClick={async () => {
                setBusy(true);
                setError(null);
                try {
                  const done = await accountRemove(account.id, folder);
                  await onDone(
                    done.folder === "trashed"
                      ? `Removed ${done.label}. Its folder is in the Trash.`
                      : done.folder === "kept"
                        ? `Removed ${done.label}. Its folder was kept.`
                        : `Removed ${done.label}. Your folder wasn't touched.`,
                  );
                } catch (cause) {
                  setError(errorText(cause));
                  setBusy(false);
                }
              }}
            >
              {busy ? "Removing…" : "Remove"}
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}

/** AJ-E1537: an account's rename, default and remove, for every account
 * menu (the Accounts cards, the account page). `changed` says what
 * happened (or null) and refreshes; `failed` shows a refusal. */
export function useAccountManage({
  sessions,
  changed,
  failed,
}: {
  sessions: InstanceInfo[];
  changed: (words: string | null) => Promise<void> | void;
  failed: (message: string) => void;
}): { host: ActionHost; element: ReactNode } {
  const [renameFor, setRenameFor] = useState<AccountInfo | null>(null);
  const [removeFor, setRemoveFor] = useState<AccountInfo | null>(null);
  const [copyFor, setCopyFor] = useState<AccountInfo | null>(null);
  const host: ActionHost = {
    renameAccount: setRenameFor,
    copyAccountSettings: setCopyFor,
    removeAccount: setRemoveFor,
    toggleDefaultAccount: async (account) => {
      const wasDefault = account.is_default === true;
      try {
        await accountSetDefault(wasDefault ? null : account.id);
        await changed(
          wasDefault
            ? `${account.label} is no longer the default. New sessions start with the account you used last.`
            : `New sessions start with ${account.label}, unless a project picks its own.`,
        );
      } catch (cause) {
        failed(errorText(cause));
      }
    },
  };
  const element = (
    <>
      {renameFor && (
        <RenameAccountModal
          account={renameFor}
          onClose={() => setRenameFor(null)}
          onDone={async (renamed, before) => {
            setRenameFor(null);
            await changed(renamed.label !== before ? `Renamed to ${renamed.label}.` : null);
          }}
        />
      )}
      {copyFor && (
        <AccountSettingsSheet
          account={copyFor}
          onClose={async (copied) => {
            setCopyFor(null);
            if (copied > 0) await changed(copied === 1 ? `Copied 1 setting into ${copyFor.label}.` : `Copied ${copied} settings into ${copyFor.label}.`);
          }}
        />
      )}
      {removeFor && (
        <RemoveAccountModal
          account={removeFor}
          sessions={sessions.filter((session) => session.account_id === removeFor.id)}
          onClose={() => setRemoveFor(null)}
          onDone={async (words) => {
            setRemoveFor(null);
            await changed(words);
          }}
        />
      )}
    </>
  );
  return { host, element };
}
