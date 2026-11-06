/**
 * AJ-E2905: the one key capture Settings uses — Desktop apps' go-to key
 * and every shortcut in Keyboard. It takes focus, hands the next key
 * press up (a lone modifier is the caller's to ignore), Esc cancels, and
 * leaving it cancels. `GoToKey` is the go-to key's whole control, drawn
 * in both panes, so its refusals (`hotkeyRefusal`) are the same in each.
 * In Keyboard (design review 57, 73) the keycap itself is the button that
 * records new keys, "None" sits where the keys go, and the actions column
 * keeps only Choose… or Turn off (`GoToKeyCap`, `GoToKeyActions`).
 */
import { useEffect, useState, type JSX, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import { Button } from "../../components/primitives";
import { Keycap } from "../../components/ShortcutSheet";
import { appSettingGet, goToHotkeySet } from "../../lib/ipc/settings";
import { errorText } from "../../lib/bridge";
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

/** A row's keys as the button that records new ones: click, then press
 * them (design review 57: no "Change…" on every row). */
export function KeycapButton({ onClick, label, testId, children }: { onClick: () => void; label: string; testId?: string; children: ReactNode }): JSX.Element {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      title="Click, then press the new keys"
      className="flex shrink-0 items-center gap-1.5 rounded-md [&_kbd]:transition-colors hover:[&_kbd]:border-accent/60 hover:[&_kbd]:bg-accent-soft"
      data-testid={testId}
    >
      {children}
    </button>
  );
}

/** Settings › Keyboard's key column for the go-to key: the capture, the
 * keycap that records a new key, or "None". */
export function GoToKeyCap({ hotkey, prefix, label }: { hotkey: ReturnType<typeof useGoToKey>; prefix: string; label: string }): JSX.Element {
  const { binding, capturing } = hotkey;
  if (capturing) return <KeyCapture compact onKeys={hotkey.onKeys} onCancel={hotkey.cancel} label={`new keys for ${label}`} testId={`${prefix}-capture`} />;
  if (!binding) {
    return (
      <span className="text-label text-muted" data-testid={`${prefix}-none`}>
        None
      </span>
    );
  }
  return (
    <KeycapButton onClick={hotkey.start} label={`Change the keys for ${label} (${hotkeyWords(binding)})`} testId={`${prefix}-change`}>
      <span data-testid={`${prefix}-key`} className="contents">
        <Keycap>{hotkeyWords(binding)}</Keycap>
      </span>
    </KeycapButton>
  );
}

/** …and its actions column: Choose… while there's none, else Turn off. */
export function GoToKeyActions({ hotkey, prefix }: { hotkey: ReturnType<typeof useGoToKey>; prefix: string }): JSX.Element | null {
  if (hotkey.capturing) return null;
  return hotkey.binding ? (
    <Button size="sm" variant="ghost" onClick={() => hotkey.save("")} data-testid={`${prefix}-clear`}>
      Turn off
    </Button>
  ) : (
    <Button size="sm" variant="secondary" onClick={hotkey.start} data-testid={`${prefix}-choose`}>
      Choose…
    </Button>
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
