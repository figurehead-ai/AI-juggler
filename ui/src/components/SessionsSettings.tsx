/**
 * Settings › Sessions (AJ-E1505, AJ-E1509): when a quiet session is
 * marked Looks stuck, how its terminal reads, and what keeps work going —
 * restarting a crashed session, keeping the Mac awake while sessions
 * work, reopening AI Juggler after it quits unexpectedly, and opening it
 * at login. One card
 * owns the prefs, so no part of it saves over another's.
 */
import { useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { Disclosure, RowLead, type RowIcon } from "./controls";
import { IconJuggler, IconStateStuck } from "./icons";
import { IconAwake, IconBattery, IconPower, IconRestart, IconScrollback, IconTextSize } from "./icons-settings";
import { SettingSwitch as Switch } from "./SettingSwitch";
import { Status } from "./SettingsSections";
import { errorText } from "../lib/bridge";
import { appLoginItemGet, appLoginItemSet, type LoginItemInfo } from "../lib/ipc/app";
import {
  sessionPrefsGet,
  sessionPrefsSet,
  SESSION_PREFS_DEFAULT,
  SESSION_PREFS_RANGE,
  type SessionPrefsInfo,
} from "../lib/ipc/session-prefs";
import { CardTitle } from "./CardTitle";
import { RuntimeSettings } from "./RuntimeSettings";

type BooleanPref = "restart_on_crash" | "keep_awake_on_power" | "keep_awake_on_battery" | "reopen_after_crash";

/** A switch for one of Settings › Sessions' own prefs. */
function PrefSwitch({
  id,
  prefs,
  onSave,
  ...rest
}: {
  id: BooleanPref;
  icon: RowIcon;
  label: string;
  hint: string;
  prefs: SessionPrefsInfo;
  nested?: boolean;
  disabled?: boolean;
  onSave: (on: boolean) => void;
}): JSX.Element {
  return <Switch id={id} checked={prefs[id]} onChange={onSave} {...rest} />;
}

/** AJ-E1509 Req.4: open at login, through macOS's own login items (the
 * system keeps the choice; off until turned on). */
function LoginItemSwitch(): JSX.Element | null {
  const [state, setState] = useState<LoginItemInfo["state"] | null>(null);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    void appLoginItemGet().then(
      (info) => setState(info.state),
      () => setState("unavailable"),
    );
  }, []);
  if (state === null) return null;
  const hint =
    state === "unavailable"
      ? "Only the installed AI Juggler can open at login."
      : state === "needs_approval"
        ? "Waiting for your approval in System Settings › General › Login Items."
        : "When you log in. It's listed in System Settings › General › Login Items.";
  return (
    <Switch icon={IconPower}
      id="open_at_login"
      label="Open AI Juggler at login"
      hint={hint}
      checked={state === "on" || state === "needs_approval"}
      disabled={state === "unavailable"}
      note={note}
      onChange={(on) => {
        setNote(null);
        void appLoginItemSet(on).then(
          (info) => {
            setState(info.state);
            setNote(info.state === "on" ? "AI Juggler opens at login." : info.state === "needs_approval" ? "Approve it in System Settings › General › Login Items." : "AI Juggler doesn't open at login.");
          },
          (cause) => setNote(errorText(cause)),
        );
      }}
    />
  );
}

/** AJ-E1509: what keeps work going while you're away. */
function KeepGoing({ prefs, save }: { prefs: SessionPrefsInfo; save: (next: SessionPrefsInfo, done: string) => void }): JSX.Element {
  return (
    <fieldset className="flex flex-col gap-0.5 pt-2" data-testid="settings-keep-going">
      <legend className="sr-only">Keep work going</legend>
      <p className="section-label px-3 pb-1" aria-hidden="true">
        Keep work going
      </p>
      <PrefSwitch
        id="restart_on_crash"
        icon={IconRestart}
        label="Restart sessions that crash"
        hint="Started again on the same launch settings, its conversation resumed, at most 3 times in 10 minutes. Never after a usage limit, a stop you made or a failed sign-in. A launch setting can choose for itself."
        prefs={prefs}
        onSave={(on) => save({ ...prefs, restart_on_crash: on }, on ? "Sessions that crash are started again." : "Sessions that crash stay ended.")}
      />
      <PrefSwitch
        id="keep_awake_on_power"
        icon={IconAwake}
        label="Keep the Mac awake while sessions work"
        hint="Only while a session is working, not while it waits for you. The screen can still sleep, and closing the lid still sleeps the Mac."
        prefs={prefs}
        onSave={(on) => save({ ...prefs, keep_awake_on_power: on }, on ? "The Mac stays awake while sessions work, when plugged in." : "The Mac sleeps as usual while sessions work.")}
      />
      <PrefSwitch
        id="keep_awake_on_battery"
        icon={IconBattery}
        label="On battery too"
        hint="Off by default: an awake Mac drains its battery."
        prefs={prefs}
        nested
        disabled={!prefs.keep_awake_on_power}
        onSave={(on) => save({ ...prefs, keep_awake_on_battery: on }, on ? "The Mac stays awake while sessions work, on battery too." : "On battery, the Mac sleeps as usual.")}
      />
      <PrefSwitch
        id="reopen_after_crash"
        icon={IconJuggler}
        label="Reopen AI Juggler if it quits unexpectedly"
        hint="After a crash, never after you quit it. The sessions it was running show as they are, and come back where restarting is on."
        prefs={prefs}
        onSave={(on) => save({ ...prefs, reopen_after_crash: on }, on ? "AI Juggler opens again after a crash." : "After a crash, AI Juggler stays closed.")}
      />
      <LoginItemSwitch />
    </fieldset>
  );
}

/** One number in Settings › Sessions: saved on Enter or leaving the
 * field (never mid-typing), with its unit, one line of why, and Reset
 * once it differs from the default. */
function PrefNumber({
  id,
  icon,
  label,
  unit,
  hint,
  value,
  step = 1,
  range,
  fallback,
  onSave,
}: {
  id: keyof SessionPrefsInfo;
  icon: RowIcon;
  label: string;
  unit: string;
  hint: string;
  value: number;
  step?: number;
  range: readonly [number, number];
  fallback: number;
  onSave: (value: number) => void;
}): JSX.Element {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const commit = () => {
    const parsed = Number(draft);
    if (draft.trim() === "" || !Number.isFinite(parsed) || parsed === value) {
      setDraft(String(value));
      return;
    }
    onSave(parsed);
  };
  return (
    // AJ-E1558 (D-126): the kit's row: its chip, the field a fill (FRM-10).
    <div className="flex items-start gap-3 rounded-[10px] px-3 py-[9px]" data-testid={`pref-${id}`}>
      <RowLead icon={icon} on={value !== fallback} />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
      <div className="flex flex-wrap items-center gap-2 text-body">
        <label htmlFor={`pref-${id}-input`} className="font-medium text-ink">
          {label}
        </label>
        <input autoCorrect="off" autoCapitalize="off" spellCheck={false}
          id={`pref-${id}-input`}
          type="number"
          inputMode="decimal"
          min={range[0]}
          max={range[1]}
          step={step}
          value={draft}
          aria-describedby={`pref-${id}-hint`}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") commit();
          }}
          className="w-20 rounded-lg bg-control px-2 py-1 text-meta tabular-nums text-ink outline-none inset-ring-1 inset-ring-frame hover:bg-control-hi focus-visible:ring-2 focus-visible:ring-accent/60"
        />
        <span className="text-meta text-muted">{unit}</span>
        {value !== fallback && (
          <button
            type="button"
            onClick={() => onSave(fallback)}
            className="rounded-lg px-1.5 py-0.5 text-label text-muted hover:bg-surface-2 hover:text-ink"
            data-testid={`pref-${id}-reset`}
          >
            Reset to {fallback}
          </button>
        )}
      </div>
      <span id={`pref-${id}-hint`} className="text-meta text-muted">
        {hint}
      </span>
      </div>
    </div>
  );
}

/** AJ-E1505 (SPEC §5.1): when a quiet session counts as stuck, and the
 * terminal's text size and scrollback. "Stuck after" applies to running
 * sessions at once; the terminal ones to open terminals at once.
 * AJ-E1509: and what keeps work going. */
export function SessionsSettingsCard(): JSX.Element {
  const [prefs, setPrefs] = useState<SessionPrefsInfo | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void sessionPrefsGet().then(setPrefs, (cause) => setError(errorText(cause)));
  }, []);
  const save = (next: SessionPrefsInfo, done: string) => {
    setError(null);
    void sessionPrefsSet(next).then(
      (saved) => {
        setPrefs(saved);
        setStatus(done);
      },
      (cause) => setError(errorText(cause)),
    );
  };
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-sessions">
      <div>
        <CardTitle>Sessions</CardTitle>
        <p className="mt-0.5 text-meta text-ink-2">
          When a quiet session is marked Looks stuck, how its terminal reads, and what keeps work going.
        </p>
      </div>
      {prefs && (
        <>
          <PrefNumber
            id="stuck_after_minutes"
            icon={IconStateStuck}
            label="Stuck after"
            unit="minutes"
            hint="No output for this long, with the CPU near idle, marks a session Looks stuck. Raise it if your builds run quietly."
            value={prefs.stuck_after_minutes}
            range={SESSION_PREFS_RANGE.stuck_after_minutes}
            fallback={SESSION_PREFS_DEFAULT.stuck_after_minutes}
            onSave={(minutes) =>
              save({ ...prefs, stuck_after_minutes: minutes }, `Sessions are marked Looks stuck after ${minutes} minutes without output.`)
            }
          />
          <Disclosure className="px-3 text-meta text-muted" tone="heading" testId="settings-terminal" summary="Terminal">
            <div className="mt-2 flex flex-col gap-0.5">
              <PrefNumber
                id="terminal_font_size"
                icon={IconTextSize}
                label="Font size"
                unit="points"
                hint="The text size in every session's terminal."
                value={prefs.terminal_font_size}
                step={0.5}
                range={SESSION_PREFS_RANGE.terminal_font_size}
                fallback={SESSION_PREFS_DEFAULT.terminal_font_size}
                onSave={(size) => save({ ...prefs, terminal_font_size: size }, `Terminal text is ${size} points.`)}
              />
              <PrefNumber
                id="terminal_scrollback_lines"
                icon={IconScrollback}
                label="Scrollback"
                unit="lines"
                hint="Lines each terminal keeps above the screen. More lines use more memory."
                value={prefs.terminal_scrollback_lines}
                step={500}
                range={SESSION_PREFS_RANGE.terminal_scrollback_lines}
                fallback={SESSION_PREFS_DEFAULT.terminal_scrollback_lines}
                onSave={(lines) =>
                  save({ ...prefs, terminal_scrollback_lines: lines }, `Terminals keep ${lines.toLocaleString("en-US")} lines.`)
                }
              />
            </div>
          </Disclosure>
          <KeepGoing prefs={prefs} save={save} />
          <RuntimeSettings />
        </>
      )}
      <Status status={status} error={error} />
    </Card>
  );
}
