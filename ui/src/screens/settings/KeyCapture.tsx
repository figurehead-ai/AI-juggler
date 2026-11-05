/**
 * AJ-E2905: the one key capture Settings uses — Desktop apps' go-to key
 * and every shortcut in Keyboard. It takes focus, hands the next key
 * press up (a lone modifier is the caller's to ignore), Esc cancels, and
 * leaving it cancels. `GoToKey` is the go-to key's whole control, drawn
 * in both panes, so its refusals (`hotkeyRefusal`) are the same in each.
 */
import { useEffect, useState, type JSX, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import { Button } from "../../components/primitives";
import { appSettingGet, errorText, goToHotkeySet } from "../../lib/bridge";
import { hotkeyRefusal, hotkeyWords, toAccelerator } from "../../lib/hotkey";
import { GO_TO_HOTKEY_EVENT } from "../../lib/outside";

export function KeyCapture({ onKeys, onCancel, label, testId, compact = false }: { onKeys: (event: KeyboardEvent) => void; onCancel: () => void; label?: string; testId?: string; compact?: boolean }): JSX.Element {
  return (
    <button
      type="button"
      autoFocus
      className="whitespace-nowrap rounded-lg border border-accent bg-accent-soft px-2.5 py-1 text-xs text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      onKeyDown={(event: ReactKeyboardEvent<HTMLButtonElement>) => {
        event.preventDefault();
        event.stopPropagation();
        if (event.key === "Escape") onCancel();
        else onKeys(event.nativeEvent);
      }}
      onBlur={onCancel}
      data-testid={testId}
    >
      {/* Compact where a row says "Esc cancels" itself (Settings › Keyboard). */}
      {compact ? "Press keys…" : <>Press the keys… <span className="text-muted">(Esc cancels)</span></>}
      {label && <span className="sr-only"> — {label}</span>}
    </button>
  );
}

/** The go-to key: what it is (or None), Choose…/Change…, Turn off, and
 * the capture; `prefix` names its test ids ("desktop-hotkey"). */
export function useGoToKey(): { binding: string | null; capturing: boolean; error: string | null; start: () => void; cancel: () => void; save: (next: string) => void; onKeys: (event: KeyboardEvent) => void } {
  const [binding, setBinding] = useState<string | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void appSettingGet("go_to_hotkey").then((stored) => setBinding(stored ?? ""), () => setBinding(""));
  }, []);
  const save = (next: string) => {
    setError(null);
    void goToHotkeySet(next).then(
      () => {
        setBinding(next);
        setCapturing(false);
        window.dispatchEvent(new CustomEvent(GO_TO_HOTKEY_EVENT, { detail: next }));
      },
      (cause) => setError(errorText(cause)),
    );
  };
  const onKeys = (event: KeyboardEvent) => {
    const combo = toAccelerator(event);
    if (combo === null) return;
    const refusal = hotkeyRefusal(combo);
    if (refusal) setError(refusal);
    else save(combo);
  };
  return { binding, capturing, error, start: () => setCapturing(true), cancel: () => setCapturing(false), save, onKeys };
}

export function GoToKeyControl({ hotkey, prefix }: { hotkey: ReturnType<typeof useGoToKey>; prefix: string }): JSX.Element {
  const { binding, capturing } = hotkey;
  if (capturing) return <KeyCapture onKeys={hotkey.onKeys} onCancel={hotkey.cancel} testId={`${prefix}-capture`} />;
  return (
    <>
      {binding ? (
        <kbd className="rounded-md border border-hairline bg-surface-2 px-2 py-0.5 font-mono text-xs text-ink" data-testid={`${prefix}-key`}>
          {hotkeyWords(binding)}
        </kbd>
      ) : (
        <span className="text-xs text-muted">None</span>
      )}
      <Button size="sm" variant="secondary" onClick={hotkey.start} data-testid={`${prefix}-choose`}>
        {binding ? "Change…" : "Choose…"}
      </Button>
      {binding && (
        <Button size="sm" variant="ghost" onClick={() => hotkey.save("")} data-testid={`${prefix}-clear`}>
          Turn off
        </Button>
      )}
    </>
  );
}

export function GoToKeyError({ hotkey, prefix }: { hotkey: ReturnType<typeof useGoToKey>; prefix: string }): ReactNode {
  return hotkey.error ? (
    <span className="text-xs text-danger" role="alert" data-testid={`${prefix}-error`}>
      {hotkey.error}
    </span>
  ) : null;
}
