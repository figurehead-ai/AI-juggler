import { useEffect, useId, useState } from "react";
import { Button } from "./primitives";
import { errorText, launchRecipeEnvGet, launchRecipeEnvSet } from "../lib/bridge";

/**
 * AJ-E309 Req.5: the environment-override editor for one Launch
 * settings — extra variables injected when an Instance starts. The
 * backend is the authority (it refuses isolation keys and
 * credential-shaped names with a coded error); the same rules run
 * here first so the user sees the reason inline, before Save.
 */

const CREDENTIAL_SEGMENTS = new Set([
  "KEY",
  "APIKEY",
  "TOKEN",
  "SECRET",
  "PASSWORD",
  "PASSWD",
  "CREDENTIAL",
  "CREDENTIALS",
  "AUTH",
  "OAUTH",
  "BEARER",
  "COOKIE",
]);

/** The inline reason a row would be refused, or null when it's fine
 * (mirrors `validate_env_overlay` in the app crate). */
export function envRowProblem(key: string, value: string, protectedKeys: string[]): string | null {
  if (key.length === 0) return value.length > 0 ? "Name the variable" : null;
  if (!/^[A-Z_][A-Z0-9_]{0,63}$/.test(key)) return "Use capitals, digits and _ (e.g. MY_FLAG)";
  if (protectedKeys.includes(key)) return "Managed by AI Juggler — it keeps this account isolated";
  if (key.split("_").some((segment) => CREDENTIAL_SEGMENTS.has(segment))) {
    return "Looks like a credential — sign in with the vendor's own flow instead";
  }
  if (value.length > 1024 || /[\0\n\r]/.test(value)) return "One line, under 1 KB";
  return null;
}

type Row = { key: string; value: string };

export function EnvOverlayEditor({
  recipeId,
  protectedKeys,
  onSaved,
}: {
  recipeId: string;
  protectedKeys: string[];
  onSaved?: (env: Record<string, string>) => void;
}): JSX.Element {
  const base = useId();
  const [rows, setRows] = useState<Row[] | null>(null);
  const [saved, setSaved] = useState<Row[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setRows(null);
    setError(null);
    setNotice(null);
    void launchRecipeEnvGet(recipeId)
      .then((env) => {
        if (!alive) return;
        const loaded = Object.entries(env).map(([key, value]) => ({ key, value }));
        setRows(loaded);
        setSaved(loaded);
      })
      .catch((cause) => {
        if (alive) {
          setRows([]);
          setError(errorText(cause));
        }
      });
    return () => {
      alive = false;
    };
  }, [recipeId]);

  if (rows === null) {
    return <p className="text-[11px] text-muted">Loading environment overrides…</p>;
  }

  const problems = rows.map((row) => envRowProblem(row.key, row.value, protectedKeys));
  const filled = rows.filter((row) => row.key.length > 0);
  const duplicate = filled.find(
    (row, index) => filled.findIndex((other) => other.key === row.key) !== index,
  );
  const dirty = JSON.stringify(filled) !== JSON.stringify(saved);
  const blocked = problems.some((problem) => problem !== null) || duplicate !== undefined;

  const save = async () => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const env = Object.fromEntries(filled.map((row) => [row.key, row.value]));
      const stored = await launchRecipeEnvSet(recipeId, env);
      const next = Object.entries(stored).map(([key, value]) => ({ key, value }));
      setRows(next);
      setSaved(next);
      setNotice("Saved — applies the next time this account starts.");
      onSaved?.(stored);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-2" data-testid="env-overlay-editor">
      {rows.length === 0 ? (
        <p className="text-[11px] text-muted">No environment overrides — the defaults apply.</p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {rows.map((row, index) => {
            const problem = problems[index];
            const problemId = `${base}-problem-${index}`;
            return (
              <li key={index} className="flex flex-col gap-0.5" data-testid="env-row">
                <div className="flex flex-wrap items-center gap-1.5">
                  <input
                    value={row.key}
                    aria-label={`Variable name ${index + 1}`}
                    aria-invalid={problem !== null}
                    aria-describedby={problem !== null ? problemId : undefined}
                    placeholder="NAME"
                    spellCheck={false}
                    onChange={(event) =>
                      setRows((current) =>
                        (current ?? []).map((entry, at) =>
                          at === index ? { ...entry, key: event.target.value.toUpperCase() } : entry,
                        ),
                      )
                    }
                    className="w-[clamp(10rem,42%,20rem)] min-w-0 rounded-lg border border-hairline bg-canvas px-2 py-1 font-mono text-[11px] text-ink outline-none focus:border-accent/50"
                    data-testid="env-key"
                  />
                  <span className="text-muted" aria-hidden="true">
                    =
                  </span>
                  <input
                    value={row.value}
                    aria-label={`Value for ${row.key || `variable ${index + 1}`}`}
                    placeholder="value"
                    spellCheck={false}
                    onChange={(event) =>
                      setRows((current) =>
                        (current ?? []).map((entry, at) =>
                          at === index ? { ...entry, value: event.target.value } : entry,
                        ),
                      )
                    }
                    className="min-w-0 flex-1 rounded-lg border border-hairline bg-canvas px-2 py-1 font-mono text-[11px] text-ink outline-none focus:border-accent/50"
                    data-testid="env-value"
                  />
                  <button
                    type="button"
                    aria-label={`Remove ${row.key || `variable ${index + 1}`}`}
                    onClick={() => setRows((current) => (current ?? []).filter((_, at) => at !== index))}
                    className="rounded-md px-1.5 py-0.5 text-[11px] text-muted hover:bg-surface hover:text-ink"
                  >
                    ✕
                  </button>
                </div>
                {problem !== null && (
                  <span id={problemId} className="text-[11px] text-danger" data-testid="env-problem">
                    {problem}
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {duplicate !== undefined && (
        <p className="text-[11px] text-danger">{duplicate.key} is listed twice.</p>
      )}
      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          variant="secondary"
          onClick={() => setRows((current) => [...(current ?? []), { key: "", value: "" }])}
          data-testid="env-add"
        >
          ＋ Add variable
        </Button>
        <Button
          size="sm"
          variant="primary"
          disabled={!dirty || blocked || busy}
          onClick={() => void save()}
          data-testid="env-save"
        >
          {busy ? "Saving…" : "Save overrides"}
        </Button>
        {notice && (
          <span className="text-[11px] text-muted" role="status">
            {notice}
          </span>
        )}
      </div>
      {error && (
        <p className="text-[11px] text-danger" role="alert" data-testid="env-error">
          {error}
        </p>
      )}
    </div>
  );
}
