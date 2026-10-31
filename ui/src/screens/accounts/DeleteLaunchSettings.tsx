/**
 * D-60 (0.3.1): deleting launch settings asks first. One click used to
 * delete them and, with them, every session they made from History; now
 * the sessions stay, and launch settings with an open session are kept
 * (the backend refuses and names it, shown here in its own words).
 */
import { useState, type JSX } from "react";
import { Modal } from "../../components/form";
import { Button } from "../../components/primitives";
import { launchSettingsName } from "../../lib/account-words";
import { errorText, type LaunchRecipeInfo } from "../../lib/bridge";

export function DeleteLaunchSettingsModal({
  recipe,
  onClose,
  onDelete,
}: {
  recipe: LaunchRecipeInfo;
  onClose: () => void;
  onDelete: (id: string) => Promise<void>;
}): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const name = launchSettingsName(recipe.name);
  return (
    <Modal title={`Delete ${name}?`} onClose={onClose}>
      <div className="flex flex-col gap-4 text-sm" data-testid="delete-launch-settings">
        <p className="leading-relaxed text-ink-2">
          It's removed from this account's launch settings and won't start new sessions. Sessions already made with it stay in History.
        </p>
        {error && (
          <p className="text-xs text-danger" role="alert" data-testid="delete-launch-settings-error">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            {error ? "Close" : "Cancel"}
          </Button>
          {!error && (
            <Button
              size="sm"
              variant="danger"
              disabled={busy}
              data-testid="delete-launch-settings-confirm"
              onClick={async () => {
                setBusy(true);
                try {
                  await onDelete(recipe.id);
                  onClose();
                } catch (cause) {
                  setError(errorText(cause));
                  setBusy(false);
                }
              }}
            >
              {busy ? "Deleting…" : "Delete"}
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}
