import { useCallback, useEffect, useRef, useState, type JSX } from "react";
import { Button, Card } from "../components/primitives";
import {
  errorText,
  fleetDiscovered,
  pairingHostClose,
  pairingHostDecide,
  pairingHostOpen,
  pairingHostState,
  pairingJoinDecide,
  pairingJoinReset,
  pairingJoinStart,
  pairingJoinState,
  pairingReEnable,
  type DiscoveredMachineInfo,
  type PairPermission,
  type PairingStateInfo,
} from "../lib/bridge";
import { Radio } from "../components/controls";

/** Which side of pairing this Mac is on. */
export type PairingMode = "host" | "join";

/**
 * AJ-E704 pairing wizard — `Find → Verify → Permissions`, three steps
 * at most, Back always available (Nielsen #5, HIG progressive
 * disclosure). Trust comes only from the code exchange plus both users
 * comparing fingerprints; discovery never implies trust.
 *
 * - **Add a machine** (host): shows a single-use 6-digit code for two
 *   minutes, and a pairing link that also carries this Mac's
 *   fingerprint.
 * - **Join a machine**: pick a machine on this network (or type an
 *   address, for machines connected by Tailscale/WireGuard) and enter
 *   its code.
 *
 * Each failure has its own words and a next action.
 */
export function PairingWizard({
  mode,
  selfLabel,
  selfFingerprint,
  onClose,
  onPaired,
}: {
  mode: PairingMode;
  selfLabel: string;
  selfFingerprint: string;
  onClose: () => void;
  onPaired: () => void;
}): JSX.Element {
  const [state, setState] = useState<PairingStateInfo | null>(null);
  const [verified, setVerified] = useState(false);
  const [permission, setPermission] = useState<PairPermission>("view");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const dialogRef = useRef<HTMLDivElement | null>(null);

  const poll = useCallback(async () => {
    try {
      const next = mode === "host" ? await pairingHostState() : await pairingJoinState();
      setState(next);
    } catch (cause) {
      setError(errorText(cause));
    }
  }, [mode]);

  // Open the host window on mount; the joiner starts on Find.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const first = mode === "host" ? await pairingHostOpen() : await pairingJoinState();
        if (!cancelled) setState(first);
      } catch (cause) {
        if (!cancelled) setError(errorText(cause));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [mode]);

  useEffect(() => {
    // poll: the pairing exchange is live only while this wizard is open
    const timer = window.setInterval(() => void poll(), 500);
    return () => window.clearInterval(timer);
  }, [poll]);

  // A new confirm step always starts unverified with the safe default.
  const phase = state?.phase ?? "idle";
  useEffect(() => {
    if (phase !== "confirm") {
      setVerified(false);
      setPermission("view");
    }
  }, [phase]);

  const notified = useRef(false);
  useEffect(() => {
    if (phase === "paired" && !notified.current) {
      notified.current = true;
      onPaired();
    }
    if (phase !== "paired") notified.current = false;
  }, [phase, onPaired]);

  const close = useCallback(async () => {
    try {
      if (mode === "host") {
        if (phase === "confirm") await pairingHostDecide(false, "view");
        await pairingHostClose();
      } else {
        if (phase === "confirm") await pairingJoinDecide(false, "view");
        await pairingJoinReset();
      }
    } catch {
      // Closing never blocks on an error.
    }
    onClose();
  }, [mode, phase, onClose]);

  // Esc = Cancel; focus moves into the dialog on each step.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void close();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close]);

  const step: 1 | 2 | 3 | "done" | "failed" =
    phase === "paired"
      ? "done"
      : phase === "failed"
        ? "failed"
        : phase === "confirm"
          ? verified
            ? 3
            : 2
          : 1;

  useEffect(() => {
    const target = dialogRef.current?.querySelector<HTMLElement>("[data-autofocus]");
    target?.focus();
  }, [step]);

  const decide = async (commit: boolean) => {
    setBusy(true);
    try {
      const next =
        mode === "host"
          ? await pairingHostDecide(commit, permission)
          : await pairingJoinDecide(commit, permission);
      setState(next);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  const restart = async (reEnable = false) => {
    setError(null);
    setBusy(true);
    try {
      if (reEnable) await pairingReEnable();
      if (mode === "host") setState(await pairingHostOpen());
      else {
        await pairingJoinReset();
        setState(await pairingJoinState());
      }
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  const title = mode === "host" ? "Add a machine" : "Join a machine";

  return (
    <div ref={dialogRef} className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6">
      <Card
        className="flex max-h-full w-full max-w-lg flex-col gap-4 overflow-y-auto p-5"
        role="dialog"
        aria-modal="true"
        aria-labelledby="pairing-title"
        data-testid="pairing-wizard"
        data-mode={mode}
        data-step={String(step)}
      >
        <div className="flex flex-col gap-2">
          <h3 id="pairing-title" className="text-base font-semibold">
            {title}
          </h3>
          <StepIndicator step={step} />
        </div>

        {error && (
          <p className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-xs text-danger" role="alert">
            {error}
          </p>
        )}

        {step === 1 && mode === "host" && (
          <HostFind state={state} onCancel={() => void close()} />
        )}
        {step === 1 && mode === "join" && (
          <JoinFind
            state={state}
            busy={busy}
            onStart={async (target, code) => {
              setError(null);
              setBusy(true);
              try {
                setState(await pairingJoinStart(target, code));
              } catch (cause) {
                setError(errorText(cause));
              } finally {
                setBusy(false);
              }
            }}
            onCancel={() => void close()}
          />
        )}
        {step === 2 && state && (
          <Verify
            state={state}
            selfLabel={selfLabel}
            selfFingerprint={selfFingerprint}
            onBack={() => void decide(false)}
            onMatch={() => setVerified(true)}
          />
        )}
        {step === 3 && state && (
          <Permissions
            peerLabel={state.peer_label ?? "that machine"}
            permission={permission}
            busy={busy}
            onChange={setPermission}
            onBack={() => setVerified(false)}
            onPair={() => void decide(true)}
          />
        )}
        {step === "done" && state && (
          <div className="flex flex-col gap-3" data-testid="pairing-done" role="status">
            <p className="text-sm font-medium">✓ Paired with {state.peer_label}</p>
            <p className="text-xs leading-relaxed text-muted">
              It's listed under Paired machines and connects whenever Link is on on both
              machines. You can unpair it any time.
            </p>
            <Button variant="primary" className="self-end" onClick={() => void close()} data-autofocus>
              Done
            </Button>
          </div>
        )}
        {step === "failed" && state && (
          <Failure
            mode={mode}
            state={state}
            busy={busy}
            onRetry={(reEnable) => void restart(reEnable)}
            onClose={() => void close()}
          />
        )}
      </Card>
    </div>
  );
}

function StepIndicator({ step }: { step: 1 | 2 | 3 | "done" | "failed" }): JSX.Element {
  const names = ["Find", "Verify", "Permissions"];
  const current = typeof step === "number" ? step : step === "done" ? 4 : 0;
  return (
    <ol className="flex items-center gap-2 text-label" aria-label="Pairing steps">
      {names.map((name, index) => {
        const number = index + 1;
        const state = number < current ? "done" : number === current ? "current" : "todo";
        return (
          <li
            key={name}
            className={
              "flex items-center gap-1.5 " +
              (state === "current" ? "font-semibold text-ink" : "text-muted")
            }
            aria-current={state === "current" ? "step" : undefined}
          >
            <span
              aria-hidden="true"
              className={
                "flex h-4 w-4 items-center justify-center rounded-full border text-label " +
                (state === "current"
                  ? "border-accent bg-accent-soft text-accent-hi"
                  : state === "done"
                    ? "border-success/40 text-success"
                    : "border-hairline")
              }
            >
              {state === "done" ? "✓" : number}
            </span>
            {name}
            {index < names.length - 1 && <span aria-hidden="true" className="text-muted">→</span>}
          </li>
        );
      })}
    </ol>
  );
}

function useCountdown(until: number | null): string | null {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);
  if (until === null) return null;
  const left = Math.max(0, Math.round((until - now) / 1000));
  return `${Math.floor(left / 60)}:${String(left % 60).padStart(2, "0")}`;
}

function HostFind({
  state,
  onCancel,
}: {
  state: PairingStateInfo | null;
  onCancel: () => void;
}): JSX.Element {
  const left = useCountdown(state?.expires_at_ms ?? null);
  const [copied, setCopied] = useState(false);
  const code = state?.code ?? null;
  return (
    <div className="flex flex-col gap-3" data-testid="pairing-host-find">
      <p className="text-xs leading-relaxed text-muted">
        On the other machine, open Fleet, choose <span className="font-medium text-ink">Join a machine</span>,
        pick this Mac and enter this code:
      </p>
      {state?.reason === "mismatch" || state?.reason === "cancelled" ? (
        <p
          className="rounded-lg border border-warning/40 px-3 py-2 text-xs"
          role="status"
          data-testid="pairing-notice"
          data-reason={state.reason}
        >
          {state.reason === "mismatch"
            ? "⚠ That code didn't match — here's a new one."
            : "Pairing was cancelled on one of the machines. The code below still works."}
        </p>
      ) : null}
      <div className="flex flex-col items-center gap-1 rounded-lg border border-hairline bg-canvas py-4">
        <p
          className="font-mono text-3xl font-semibold tracking-[0.3em] text-ink"
          aria-label={code ? `Pairing code ${code.replace(/\s/g, "").split("").join(" ")}` : "Getting a code"}
          data-testid="pairing-code"
          aria-live="polite"
        >
          {code ?? "··· ···"}
        </p>
        {left !== null && (
          <p className="text-label text-muted" data-testid="pairing-countdown">
            Code works for {left}
          </p>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          variant="secondary"
          disabled={!state?.link}
          onClick={async () => {
            if (!state?.link) return;
            await navigator.clipboard?.writeText(state.link).catch(() => null);
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1500);
          }}
          data-testid="pairing-copy-link"
        >
          {copied ? "Copied" : "Copy pairing link"}
        </Button>
        <span className="flex-1 text-label leading-relaxed text-muted">
          The link also carries this Mac's fingerprint, so the other machine checks it for you.
        </span>
      </div>
      <p className="text-label text-muted" role="status">
        Waiting for the other machine…
      </p>
      <div className="flex justify-end">
        <Button variant="secondary" onClick={onCancel} data-autofocus>
          Cancel
        </Button>
      </div>
    </div>
  );
}

const PAIR_LINK = /^aijuggler-pair:v1:(\d{6}):[0-9A-Fa-f]{32}$/;

function JoinFind({
  state,
  busy,
  onStart,
  onCancel,
}: {
  state: PairingStateInfo | null;
  busy: boolean;
  onStart: (target: { instanceId: string } | { address: string }, code: string) => Promise<void>;
  onCancel: () => void;
}): JSX.Element {
  const [machines, setMachines] = useState<DiscoveredMachineInfo[] | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  const [byAddress, setByAddress] = useState(false);
  const [address, setAddress] = useState("");
  const [digits, setDigits] = useState<string[]>(["", "", "", "", "", ""]);
  const [link, setLink] = useState<string | null>(null);
  const boxes = useRef<Array<HTMLInputElement | null>>([]);

  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        const found = await fleetDiscovered();
        if (alive) setMachines(found);
      } catch {
        if (alive) setMachines([]);
      }
    };
    void load();
    const timer = window.setInterval(() => void load(), 2000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    if (machines && machines.length === 1 && chosen === null) setChosen(machines[0]!.instance_id);
  }, [machines, chosen]);

  const code = digits.join("");
  const complete = /^\d{6}$/.test(code);
  const target: { instanceId: string } | { address: string } | null = byAddress
    ? address.trim()
      ? { address: address.trim() }
      : null
    : chosen
      ? { instanceId: chosen }
      : null;
  const connecting = state?.phase === "connecting";

  const fill = (text: string) => {
    const trimmed = text.trim();
    const fromLink = PAIR_LINK.exec(trimmed);
    if (fromLink) {
      setLink(trimmed);
      setDigits(fromLink[1]!.split(""));
      return true;
    }
    const only = trimmed.replace(/\D/g, "");
    if (only.length === 6) {
      setLink(null);
      setDigits(only.split(""));
      boxes.current[5]?.focus();
      return true;
    }
    return false;
  };

  return (
    <form
      className="flex flex-col gap-4"
      data-testid="pairing-join-find"
      onSubmit={(event) => {
        event.preventDefault();
        if (target && complete && !busy && !connecting) void onStart(target, link ?? code);
      }}
    >
      <fieldset className="flex flex-col gap-2">
        <legend className="mb-1 text-xs font-medium">1. Choose the machine</legend>
        {!byAddress && machines === null && <p className="text-xs text-muted">Looking on this network…</p>}
        {!byAddress && machines !== null && machines.length === 0 && (
          <p className="rounded-lg border border-hairline px-3 py-2 text-xs leading-relaxed" data-testid="pairing-none-found">
            No machines found. Check both machines are on the same network with Link on.
          </p>
        )}
        {!byAddress && machines !== null && machines.length > 0 && (
          <div role="radiogroup" aria-label="Machines on this network" className="flex flex-col gap-1">
            {machines.map((machine, index) => (
              <label
                key={machine.instance_id}
                className={
                  "flex items-center gap-2 rounded-lg border px-3 py-2 text-sm " +
                  (chosen === machine.instance_id ? "border-accent/60 bg-accent-soft" : "border-hairline")
                }
                data-testid="pairing-machine"
              >
                <Radio
                  name="pairing-machine"
                  checked={chosen === machine.instance_id}
                  onChange={() => setChosen(machine.instance_id)}
                  data-autofocus={index === 0 ? true : undefined}
                />
                <span className="flex-1">{machine.label}</span>
                <span className="text-label text-muted">v{machine.version}</span>
              </label>
            ))}
          </div>
        )}
        {byAddress ? (
          <div className="flex flex-col gap-1">
            <label className="text-label font-medium" htmlFor="pairing-address">
              The other machine's address
            </label>
            <input
              id="pairing-address"
              value={address}
              onChange={(event) => setAddress(event.target.value)}
              placeholder="100.64.0.2"
              className="rounded-lg border border-hairline bg-canvas px-2.5 py-1.5 font-mono text-sm text-ink outline-none focus:border-accent/50"
              data-testid="pairing-address"
              data-autofocus
            />
            <p className="text-label leading-relaxed text-muted">
              Not on the same network? Connect your machines with Tailscale or WireGuard
              first, then enter the other machine's address. AI Juggler never relays
              through the internet.
            </p>
          </div>
        ) : null}
        <button
          type="button"
          className="self-start text-label text-accent-hi underline-offset-2 hover:underline"
          onClick={() => setByAddress((now) => !now)}
          data-testid="pairing-toggle-address"
        >
          {byAddress ? "Choose from this network instead" : "Pair by address instead (VPN)"}
        </button>
      </fieldset>

      <fieldset className="flex flex-col gap-2">
        <legend className="mb-1 text-xs font-medium">2. Enter the code it shows</legend>
        <div role="group" aria-label="Pairing code" className="flex items-center gap-1.5">
          {digits.map((digit, index) => (
            <input
              key={index}
              ref={(element) => {
                boxes.current[index] = element;
              }}
              value={digit}
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={1}
              aria-label={`Digit ${index + 1} of 6`}
              className={
                "h-10 w-9 rounded-lg border border-hairline bg-canvas text-center font-mono text-lg text-ink outline-none focus:border-accent/60 " +
                (index === 3 ? "ml-2" : "")
              }
              data-testid="pairing-digit"
              onFocus={(event) => event.target.select()}
              onPaste={(event) => {
                if (fill(event.clipboardData.getData("text"))) event.preventDefault();
              }}
              onChange={(event) => {
                const value = event.target.value;
                if (value.length > 1 && fill(value)) return;
                const next = value.replace(/\D/g, "").slice(-1);
                setLink(null);
                setDigits((now) => now.map((d, i) => (i === index ? next : d)));
                if (next && index < 5) boxes.current[index + 1]?.focus();
              }}
              onKeyDown={(event) => {
                if (event.key === "Backspace" && !digit && index > 0) {
                  boxes.current[index - 1]?.focus();
                }
                if (event.key === "ArrowLeft" && index > 0) boxes.current[index - 1]?.focus();
                if (event.key === "ArrowRight" && index < 5) boxes.current[index + 1]?.focus();
              }}
            />
          ))}
        </div>
        {link && (
          <p className="text-label text-muted" data-testid="pairing-link-pasted">
            Pairing link pasted — the other machine's fingerprint will be checked automatically.
          </p>
        )}
      </fieldset>

      {connecting && (
        <p className="text-xs text-muted" role="status">
          Connecting securely…
        </p>
      )}
      <div className="flex justify-end gap-2">
        <Button variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          type="submit"
          variant="primary"
          disabled={!target || !complete || busy || connecting}
          data-testid="pairing-connect"
        >
          Connect
        </Button>
      </div>
    </form>
  );
}

function Verify({
  state,
  selfLabel,
  selfFingerprint,
  onBack,
  onMatch,
}: {
  state: PairingStateInfo;
  selfLabel: string;
  selfFingerprint: string;
  onBack: () => void;
  onMatch: () => void;
}): JSX.Element {
  return (
    <div className="flex flex-col gap-3" data-testid="pairing-verify">
      <p className="text-xs leading-relaxed text-muted">
        The code matched. Now check you're pairing with the right machine: the
        fingerprint below must match the one Fleet shows on{" "}
        <span className="font-medium text-ink">{state.peer_label}</span> under This Mac.
      </p>
      <div className="flex flex-col gap-1 rounded-lg border border-hairline bg-canvas p-3">
        <p className="text-section font-semibold text-ink">The other machine</p>
        <p className="text-sm font-semibold" data-testid="pairing-peer-label">
          {state.peer_label}
        </p>
        <code
          className="font-mono text-sm tracking-wide text-ink"
          aria-label={`Its fingerprint ${state.peer_fingerprint ?? ""}`}
          data-testid="pairing-peer-fingerprint"
        >
          {state.peer_fingerprint}
        </code>
      </div>
      <p className="text-label leading-relaxed text-muted">
        It shows this Mac as <span className="font-medium text-ink">{selfLabel}</span> ·{" "}
        <span className="font-mono">{selfFingerprint}</span>
      </p>
      <div className="flex justify-end gap-2">
        <Button variant="secondary" onClick={onBack} data-testid="pairing-mismatch">
          They don't match — cancel
        </Button>
        <Button variant="primary" onClick={onMatch} data-autofocus data-testid="pairing-fingerprints-match">
          Fingerprints match
        </Button>
      </div>
    </div>
  );
}

function Permissions({
  peerLabel,
  permission,
  busy,
  onChange,
  onBack,
  onPair,
}: {
  peerLabel: string;
  permission: PairPermission;
  busy: boolean;
  onChange: (permission: PairPermission) => void;
  onBack: () => void;
  onPair: () => void;
}): JSX.Element {
  const options: Array<{ id: PairPermission; title: string; body: string }> = [
    {
      id: "view",
      title: "View only",
      body: `${peerLabel} can see this Mac's accounts, sessions and usage — it can't change anything.`,
    },
    {
      id: "control",
      title: "View + control",
      body: `${peerLabel} can also start and stop work here, using Launch settings that already exist on this Mac — it can never read credentials.`,
    },
  ];
  return (
    <div className="flex flex-col gap-3" data-testid="pairing-permissions">
      <p className="text-xs text-muted">What may {peerLabel} do on this Mac?</p>
      <div role="radiogroup" aria-label={`What ${peerLabel} may do on this Mac`} className="flex flex-col gap-2">
        {options.map((option) => (
          <label
            key={option.id}
            className={
              "flex items-start gap-2.5 rounded-lg border px-3 py-2.5 " +
              (permission === option.id ? "border-accent/60 bg-accent-soft" : "border-hairline")
            }
            data-testid={`pairing-permission-${option.id}`}
          >
            <Radio
              name="pairing-permission"
              className="mt-0.5"
              checked={permission === option.id}
              onChange={() => onChange(option.id)}
              data-autofocus={permission === option.id ? true : undefined}
            />
            <span className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{option.title}</span>
              <span className="text-label leading-relaxed text-muted">{option.body}</span>
            </span>
          </label>
        ))}
      </div>
      <p className="text-label text-muted">You can unpair it any time in Fleet.</p>
      <div className="flex justify-end gap-2">
        <Button variant="secondary" onClick={onBack}>
          Back
        </Button>
        <Button variant="primary" disabled={busy} onClick={onPair} data-testid="pairing-pair">
          Pair
        </Button>
      </div>
    </div>
  );
}

const FAILURES: Record<string, { title: string; body: string; action: string; reEnable?: boolean }> = {
  expired: {
    title: "The pairing window closed",
    body: "Codes work for 2 minutes. Start again when the other machine is ready.",
    action: "Start again",
  },
  mismatch: {
    title: "The code didn't match",
    body: "Nothing was paired. Check the 6 digits on the other machine — it now shows a new code.",
    action: "Try again",
  },
  cancelled: {
    title: "Pairing was cancelled",
    body: "One of the machines cancelled, so nothing was paired.",
    action: "Start over",
  },
  locked_out: {
    title: "Pairing is paused",
    body: "Too many wrong codes in a row. Wait a minute, then try again.",
    action: "Try again",
  },
  disabled: {
    title: "Pairing is turned off",
    body: "After 20 wrong codes in a day, pairing turns off to protect this Mac. Turn it back on only if you're the one pairing.",
    action: "Turn pairing back on",
    reEnable: true,
  },
  unreachable: {
    title: "Couldn't reach that machine",
    body: "Check both machines are on the same network — or connected by Tailscale or WireGuard — with Link on.",
    action: "Try again",
  },
  fingerprint: {
    title: "The fingerprint doesn't match the link",
    body: "Don't pair — something may be intercepting. Copy the pairing link again from the other machine.",
    action: "Start over",
  },
  not_open: {
    title: "Pairing isn't open on that machine",
    body: "Choose Add a machine on it first, then enter the code it shows.",
    action: "Try again",
  },
  busy: {
    title: "Another machine is pairing",
    body: "That machine is pairing with someone else right now. Wait a moment and try again.",
    action: "Try again",
  },
};

function Failure({
  mode,
  state,
  busy,
  onRetry,
  onClose,
}: {
  mode: PairingMode;
  state: PairingStateInfo;
  busy: boolean;
  onRetry: (reEnable: boolean) => void;
  onClose: () => void;
}): JSX.Element {
  const known = FAILURES[state.reason ?? ""];
  // The re-enable action only exists on the machine that turned off.
  const reEnable = Boolean(known?.reEnable) && mode === "host";
  return (
    <div className="flex flex-col gap-3" role="alert" data-testid="pairing-failure" data-reason={state.reason ?? "unknown"}>
      <p className="text-sm font-medium">{known?.title ?? "Pairing didn't finish"}</p>
      <p className="text-xs leading-relaxed text-muted">{known?.body ?? state.message ?? "Nothing was paired."}</p>
      <div className="flex justify-end gap-2">
        <Button variant="secondary" onClick={onClose}>
          Close
        </Button>
        {(known?.reEnable !== true || reEnable) && (
          <Button
            variant="primary"
            disabled={busy}
            onClick={() => onRetry(reEnable)}
            data-autofocus
            data-testid="pairing-retry"
          >
            {known?.action ?? "Try again"}
          </Button>
        )}
      </div>
    </div>
  );
}
