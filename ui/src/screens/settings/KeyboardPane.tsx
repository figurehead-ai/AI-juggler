/**
 * AJ-E2905: Settings › Keyboard — every shortcut by area, each with its
 * keys. A rebindable one has Change… (the next keys pressed, Esc cancels,
 * refused with the reason when another shortcut, the system or typing has
 * them) and, once changed, Reset; a fixed one says why. The go-to key from
 * any app is the same control as in Desktop apps. A bar at the top counts
 * the changes and has Reset all, with Undo (at the top, not after some 40
 * rows, so it's found). A change applies at once: the ? sheet, ⌘K, the
 * tooltips and the keys themselves read it.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Keycap } from "../../components/ShortcutSheet";
import { showToast } from "../../components/Toast";
import { keysOf, SHORTCUT_AREAS, SHORTCUTS, type Shortcut, type ShortcutId } from "../../lib/shortcuts";
import { bindingOf, DEFAULT_BINDINGS, FIXED_WHY, isChanged, onBindingsChanged, refusal, resetAll, resetBinding, restoreBindings, setBinding } from "../../lib/shortcut-bindings";
import { GoToKeyControl, GoToKeyError, KeyCapture, useGoToKey } from "./KeyCapture";
import { Pane, PaneSection } from "./parts";
import { keyboardSectionId } from "./settings-index";

const labelOf = (id: ShortcutId) => SHORTCUTS.find((row) => row.id === id)?.label ?? id;

/** Re-renders when a binding changes. */
function useBindingsVersion(): number {
  const [version, setVersion] = useState(0);
  useEffect(() => onBindingsChanged(() => setVersion((n) => n + 1)), []);
  return version;
}

function Row({ id, label, keys, actions, note }: { id: string; label: string; keys: JSX.Element; actions: JSX.Element | null; note?: JSX.Element | null }): JSX.Element {
  return (
    <li className="flex flex-col gap-1 px-4 py-2.5" data-testid="keyboard-row" data-shortcut={id}>
      <div className="flex items-center gap-3">
        <span className="min-w-0 flex-1 text-body text-ink">{label}</span>
        {keys}
        {/* A fixed width, so every row's keys line up. */}
        <span className="flex w-40 shrink-0 items-center justify-end gap-1">{actions}</span>
      </div>
      {note}
    </li>
  );
}

/** The go-to key: the same control as Settings › Desktop apps. */
function GoToRow(): JSX.Element {
  const hotkey = useGoToKey();
  return (
    <Row
      id="go-to"
      label="Open AI Juggler from anywhere"
      keys={<span />}
      actions={<GoToKeyControl hotkey={hotkey} prefix="keyboard-go-to" />}
      note={
        <>
          <span className="text-label text-muted">Works from any app. ⌥Space isn't offered, because ChatGPT uses it.</span>
          <GoToKeyError hotkey={hotkey} prefix="keyboard-go-to" />
        </>
      }
    />
  );
}

function ShortcutRow({ entry }: { entry: Shortcut }): JSX.Element {
  const id = entry.id as ShortcutId;
  const [capturing, setCapturing] = useState(false);
  const [why, setWhy] = useState<string | null>(null);
  const bindable = DEFAULT_BINDINGS[id] !== undefined;
  const changed = isChanged(id);
  const onKeys = (event: KeyboardEvent) => {
    const binding = bindingOf(event);
    if (!binding) return;
    const refused = refusal(id, binding, labelOf, entry.area);
    if (refused) {
      setWhy(refused);
      return;
    }
    setCapturing(false);
    setWhy(null);
    void setBinding(id, binding);
  };
  const keys = (
    <span className="flex shrink-0 items-center gap-1.5">
      {changed && <span className="text-label text-muted">changed</span>}
      {keysOf(id).map((keys) => (
        <Keycap key={keys}>{keys}</Keycap>
      ))}
    </span>
  );
  const actions = !bindable ? null : capturing ? (
    <KeyCapture compact onKeys={onKeys} onCancel={() => setCapturing(false)} label={`new keys for ${entry.label}`} testId={`keyboard-capture-${id}`} />
  ) : (
    <>
      <Button
        size="sm"
        variant="secondary"
        onClick={() => {
          setCapturing(true);
          setWhy(null);
        }}
        aria-label={`Change the keys for ${entry.label}`}
        data-testid={`keyboard-change-${id}`}
      >
        Change…
      </Button>
      {changed && (
        <Button size="sm" variant="ghost" onClick={() => void resetBinding(id)} aria-label={`Reset the keys for ${entry.label}`} data-testid={`keyboard-reset-${id}`}>
          Reset
        </Button>
      )}
    </>
  );
  const note =
    capturing && why === null ? (
      <span className="text-label text-muted">Press the new keys, or Esc to cancel.</span>
    ) : why !== null ? (
      <span className="text-label text-warning" role="alert" data-testid="keyboard-refused">
        {why}
      </span>
    ) : !bindable && FIXED_WHY[id] ? (
      <span className="text-label text-muted">{FIXED_WHY[id]}</span>
    ) : null;
  return <Row id={id} label={entry.label} keys={keys} actions={actions} note={note} />;
}

export function KeyboardPane({ anchor }: { anchor?: string }): JSX.Element {
  useBindingsVersion();
  const changedCount = (Object.keys(DEFAULT_BINDINGS) as ShortcutId[]).filter(isChanged).length;
  const resetEverything = async () => {
    const before = await resetAll();
    showToast({ id: "keyboard-reset-all", message: "Every key is back as it was.", undo: { onUndo: () => restoreBindings(before) } });
  };
  return (
    <Pane id="keyboard" title="Keyboard" anchor={anchor}>
      {changedCount > 0 && (
        <div className="-mt-3 flex items-center justify-between gap-3 rounded-lg bg-surface-2 px-3 py-2" data-testid="keyboard-changed">
          <span className="text-meta text-ink-2">{changedCount === 1 ? "You changed 1 shortcut." : `You changed ${changedCount} shortcuts.`}</span>
          <Button size="sm" variant="secondary" onClick={() => void resetEverything()} data-testid="keyboard-reset-all">
            Reset all
          </Button>
        </div>
      )}
      {SHORTCUT_AREAS.map((area, index) => (
        <PaneSection
          key={area}
          id={keyboardSectionId(area, index)}
          title={area}
          footer={index === 0 ? "In a text field or the terminal, only the ⌘ shortcuts work. Your changes are kept on this computer." : undefined}
        >
          {(SHORTCUTS as readonly Shortcut[])
            .filter((entry) => entry.area === area)
            .map((entry) => (entry.id === "go-to" ? <GoToRow key={entry.id} /> : <ShortcutRow key={entry.id} entry={entry} />))}
        </PaneSection>
      ))}
    </Pane>
  );
}
