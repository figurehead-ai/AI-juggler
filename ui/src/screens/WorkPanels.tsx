import { useState } from "react";
import { Button, Card, Chip, type ChipTone } from "../components/primitives";
import { TerminalView } from "../components/TerminalView";
import { IconProjects, IconTerminal } from "../components/icons";
import {
  launchSession,
  launchRecipesList,
  type AccountInfo,
  type LaunchRecipeInfo,
  type ProjectInfo,
  type InstanceInfo, errorText,} from "../lib/bridge";

const isLive = (status: string) => status === "running" || status === "starting";

function statusTone(status: string): { tone: ChipTone; label: string } {
  switch (status) {
    case "running":
      return { tone: "live", label: "running" };
    case "starting":
      return { tone: "accent", label: "starting" };
    case "finished":
      return { tone: "success", label: "finished" };
    case "stopped":
      return { tone: "neutral", label: "stopped" };
    case "failed":
      return { tone: "danger", label: "failed" };
    default:
      return { tone: "neutral", label: status };
  }
}

function since(ms: number): string {
  const secs = Math.max(1, Math.floor((Date.now() - ms) / 1000));
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  return `${Math.floor(secs / 3600)}h`;
}

/**
 * The login-terminal panel (ADR-0007): a vendor sign-in flowing in an
 * embedded terminal. Instance/project detail moved to RightInspector
 * (AJ-E406) — extracted, not duplicated.
 */
export function SessionPanel({
  session,
  project,
  loginTerminal,
  profiles,
  accounts,
  onOpenProject,
  onStopped,
  onHandover,
}: {
  session: InstanceInfo | null;
  project: ProjectInfo | null;
  loginTerminal?: { id: string; title: string; ended: boolean } | null;
  profiles: LaunchRecipeInfo[];
  accounts: AccountInfo[];
  onOpenProject: (id: string) => void;
  onStopped: () => void;
  onHandover: (sessionId: string, targetProfileId: string, targetLabel: string) => void;
}) {
  const isLoginTerminal = Boolean(loginTerminal);
  const title = loginTerminal?.title ?? session?.account_label ?? "Session";
  const status = session?.status;
  const view = status ? statusTone(status) : null;
  const terminalId = loginTerminal?.id ?? session?.id;
  const live = status ? isLive(status) : Boolean(loginTerminal && !loginTerminal.ended);

  return (
    <div className="flex h-full min-h-0 flex-col gap-4">
      <header className="flex flex-wrap items-center gap-3">
        <span
          className={
            "size-2 rounded-full " +
            (live ? "bg-live animate-breathe" : status === "failed" ? "bg-danger" : "bg-faint")
          }
        />
        <h2 className="text-lg font-semibold tracking-tight">{title}</h2>
        {view && <Chip tone={view.tone}>{view.label}</Chip>}
        {!isLoginTerminal && session && (
          <Chip tone={session.kind === "desktop" ? "accent" : "neutral"}>{session.kind}</Chip>
        )}
        {project && !isLoginTerminal && (
          <button
            type="button"
            onClick={() => onOpenProject(project.id)}
            className="flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] text-muted hover:bg-surface hover:text-ink"
          >
            <IconProjects width={12} height={12} />
            {project.name}
          </button>
        )}
        <div className="ml-auto flex items-center gap-2">
          {session && !isLive(session.status) && !isLoginTerminal && (
            <span className="text-[11px] text-faint">
              ran {since(session.started_at_ms)} · scrollback replayed
            </span>
          )}
          {live && terminalId && (
            <Button
              size="sm"
              variant="secondary"
              onClick={async () => {
                const { terminalKill } = await import("../lib/bridge");
                await terminalKill(terminalId);
                onStopped();
              }}
            >
              Stop
            </Button>
          )}
        </div>
      </header>

      {/* The terminal IS the panel for terminal-kind occurrences */}
      <Card className="min-h-0 flex-1 overflow-hidden bg-[#05070b] p-0">
        {terminalId ? (
          <TerminalView id={terminalId} title={title} />
        ) : (
          <div className="flex h-full items-center justify-center text-xs text-faint">
            Desktop harnesses open their own window — focus controls arrive
            with the desktop adapter.
          </div>
        )}
      </Card>

      {isLoginTerminal && !live && (
        <Card className="flex items-center gap-3 p-3">
          <span className="dot-live size-1.5 rounded-full bg-live" />
          <p className="text-[11px] leading-relaxed text-muted">
            Complete Anthropic's sign-in above, then check the account again
            from <span className="text-ink">Accounts</span> — it flips to
            ready.
          </p>
        </Card>
      )}

      {!isLoginTerminal && session && (
        <div className="flex flex-col gap-2">
          <p className="text-[11px] uppercase tracking-wider text-faint">
            Continue on another account
          </p>
          <div className="flex flex-wrap gap-1.5">
            {profiles
              .filter((recipe) => recipe.id !== session.launch_recipe_id)
              .slice(0, 6)
              .map((recipe) => {
                const ready =
                  accounts.find((a) => a.id === recipe.account_id)?.login_state ===
                  "logged_in";
                return (
                  <button
                    key={recipe.id}
                    type="button"
                    disabled={!ready}
                    onClick={() => onHandover(session.id, recipe.id, recipe.account_label)}
                    title={
                      ready
                        ? "Moves this session's context to this account and resumes it"
                        : "Needs sign-in first (Ready home)"
                    }
                    className={
                      "rounded-lg border px-2.5 py-1.5 text-[11px] transition-colors " +
                      (ready
                        ? "border-accent/30 bg-accent-soft text-accent-hi hover:border-accent/50"
                        : "border-hairline/50 text-faint")
                    }
                  >
                    {recipe.account_label}
                    {!ready && " · sign in first"}
                  </button>
                );
              })}
            {profiles.filter((p) => p.id !== session.launch_recipe_id).length === 0 && (
              <p className="text-[11px] text-faint">
                Add another account to juggle between them.
              </p>
            )}
          </div>
          <div className="flex flex-wrap items-center gap-x-5 gap-y-1 text-[11px] text-faint">
            <span>provider: <span className="text-muted">{session.provider}</span></span>
            <span>age: <span className="text-muted">{since(session.started_at_ms)}</span></span>
            <span className="ml-auto">context moves with the session — transcript copied, then resumed</span>
          </div>
        </div>
      )}
    </div>
  );
}

/** New-session launcher: profile + directory → live terminal. */
export function NewSessionModal({
  onClose,
  onLaunched,
}: {
  onClose: () => void;
  onLaunched: (sessionId: string, title: string) => void;
}) {
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[] | null>(null);
  const [directory, setDirectory] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (profiles === null) {
    void launchRecipesList().then(setProfiles);
  }

  const launch = async (recipe: LaunchRecipeInfo) => {
    setBusy(true);
    setError(null);
    try {
      const result = await launchSession(recipe.id, directory.trim() || null);
      if (!result) {
        setError("Preview mode — no backend connected.");
        setBusy(false);
        return;
      }
      onLaunched(result.session_id, `${recipe.account_label} · ${recipe.name}`);
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-6 backdrop-blur-sm"
      onKeyDown={(event) => event.key === "Escape" && onClose()}
    >
      <Card className="w-full max-w-md p-5">
        <h3 className="text-sm font-semibold">New session</h3>
        <p className="mt-0.5 text-xs text-muted">
          Pick a profile — its account, isolation, and defaults apply. Point
          it at a directory to make (or reuse) a project.
        </p>

        <label className="mt-4 block text-[11px] uppercase tracking-wider text-faint">
          Directory (optional)
          <input
            value={directory}
            onChange={(event) => setDirectory(event.target.value)}
            placeholder="~/Dev/my-project"
            className="mt-1 w-full rounded-lg border border-hairline bg-canvas px-3 py-2 font-mono text-xs text-ink outline-none placeholder:text-faint focus:border-accent/50"
          />
        </label>

        <div className="mt-4 flex max-h-64 flex-col gap-1.5 overflow-y-auto">
          {(profiles ?? []).map((recipe) => (
            <button
              key={recipe.id}
              type="button"
              disabled={busy}
              onClick={() => void launch(recipe)}
              className="flex items-center gap-2 rounded-lg border border-hairline px-3 py-2.5 text-left text-xs transition-colors hover:border-accent/40 hover:bg-surface disabled:opacity-50"
            >
              <IconTerminal width={13} height={13} />
              <span className="font-medium">{recipe.account_label}</span>
              <span className="text-faint">·</span>
              <span className="text-muted">{recipe.name}</span>
              {recipe.model && (
                <span className="ml-auto font-mono text-[10px] text-faint">{recipe.model}</span>
              )}
            </button>
          ))}
          {profiles !== null && profiles.length === 0 && (
            <p className="py-4 text-center text-xs text-muted">
              No profiles yet — add an account first (Accounts, top right).
            </p>
          )}
        </div>

        {error && <p className="mt-3 text-xs text-danger">{error}</p>}
        <div className="mt-4 flex justify-end">
          <Button size="sm" variant="secondary" onClick={onClose}>
            Close
          </Button>
        </div>
      </Card>
    </div>
  );
}
