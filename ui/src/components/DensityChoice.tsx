/**
 * D-25 — Settings › Appearance: the session list's density.
 */
import { useEffect, useState, type JSX } from "react";
import { DENSITY_EVENT, readDensity, storeDensity, type Density } from "../lib/density";

/** D-25: the session list's density, moved here from the header. */
export function DensityChoice(): JSX.Element {
  const [density, setDensityState] = useState<Density>(readDensity());
  useEffect(() => {
    const sync = () => setDensityState(readDensity());
    window.addEventListener(DENSITY_EVENT, sync);
    return () => window.removeEventListener(DENSITY_EVENT, sync);
  }, []);
  const options: Array<{ id: Density; label: string; what: string }> = [
    { id: "comfortable", label: "Comfortable", what: "Two lines per session" },
    { id: "compact", label: "Compact", what: "One line per session" },
    { id: "minimal", label: "Minimal", what: "Icons only" },
  ];
  return (
    <fieldset className="mt-2 flex flex-col gap-2" data-testid="settings-density">
      <legend className="mb-1 text-xs font-medium text-ink">Session list</legend>
      <div className="flex flex-wrap gap-2">
        {options.map((option) => (
          <label
            key={option.id}
            className={
              "flex items-center gap-2 rounded-lg border px-3 py-1.5 text-sm " +
              (density === option.id ? "border-accent/60 bg-accent-soft" : "border-hairline")
            }
          >
            <input
              type="radio"
              name="density"
              checked={density === option.id}
              onChange={() => {
                storeDensity(option.id);
                setDensityState(option.id);
                window.dispatchEvent(new Event(DENSITY_EVENT));
              }}
            />
            <span className="flex flex-col">
              <span>{option.label}</span>
              <span className="text-[11px] text-muted">{option.what}</span>
            </span>
          </label>
        ))}
      </div>
      <p className="text-[11px] text-muted">Ctrl+D cycles through them from anywhere.</p>
    </fieldset>
  );
}
