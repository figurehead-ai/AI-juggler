/**
 * D-25 — Settings › Appearance: the session list's density.
 * 0.3: equal cards on one grid, its key under its name.
 */
import { useEffect, useState, type JSX } from "react";
import { DENSITY_EVENT, readDensity, storeDensity, type Density } from "../lib/density";
import { ChoiceOption } from "./controls";
import { ChoiceGroup } from "./AppearanceCard";

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
    <ChoiceGroup legend="Session list" hint="Ctrl+D cycles through them from anywhere." testId="settings-density">
      {options.map((option) => (
        <ChoiceOption
          key={option.id}
          type="radio"
          look="card"
          name="density"
          label={option.label}
          hint={option.what}
          checked={density === option.id}
          onChange={() => {
            storeDensity(option.id);
            setDensityState(option.id);
            window.dispatchEvent(new Event(DENSITY_EVENT));
          }}
        />
      ))}
    </ChoiceGroup>
  );
}
