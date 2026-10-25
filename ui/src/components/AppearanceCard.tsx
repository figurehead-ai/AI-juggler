/**
 * Settings › General: the theme (D-16, kept in sync with ⌘⇧L) and the
 * rail's density. Moved out of Settings.tsx so it stays within its size.
 */
import { useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { DensityChoice } from "./DensityChoice";
import { THEME_EVENT, setTheme, storedTheme, type ThemeChoice } from "../theme";
import { CardTitle } from "./CardTitle";

export function AppearanceCard(): JSX.Element {
  const [choice, setChoice] = useState<ThemeChoice>(storedTheme());
  useEffect(() => {
    const sync = () => setChoice(storedTheme());
    window.addEventListener(THEME_EVENT, sync);
    return () => window.removeEventListener(THEME_EVENT, sync);
  }, []);
  const options: Array<{ id: ThemeChoice; label: string }> = [
    { id: "system", label: "System" },
    { id: "light", label: "Light" },
    { id: "dark", label: "Dark" },
  ];
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-appearance">
      <CardTitle id="appearance-title">
        Appearance
      </CardTitle>
      <div role="radiogroup" aria-labelledby="appearance-title" className="flex flex-wrap gap-2">
        {options.map((option) => (
          <label
            key={option.id}
            className={
              "flex items-center gap-2 rounded-lg border px-3 py-1.5 text-sm " +
              (choice === option.id ? "border-accent/60 bg-accent-soft" : "border-hairline")
            }
          >
            <input
              type="radio"
              name="appearance"
              checked={choice === option.id}
              onChange={() => {
                setTheme(option.id);
                setChoice(option.id);
              }}
              data-testid={`appearance-${option.id}`}
            />
            {option.label}
          </label>
        ))}
      </div>
      <p className="text-[11px] text-muted">⌘⇧L switches between them from anywhere.</p>
      <DensityChoice />
    </Card>
  );
}
