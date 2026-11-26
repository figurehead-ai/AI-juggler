/**
 * Settings › General: the theme (D-16, kept in sync with ⌘⇧L), the
 * rail's density and Open now's order (D-47). Moved out of Settings.tsx so it stays within its size.
 * 0.3: each choice is a row of equal cards on one grid, its key said
 * under its name (no caption floating under the cards).
 */
import { useKeys } from "../lib/use-keys";
import { useEffect, useId, useState, type JSX, type ReactNode } from "react";
import { Card } from "./primitives";
import { DensityChoice } from "./DensityChoice";
import { RailOrderChoice } from "./RailOrderChoice";
import { ChoiceOption, type ControlIcon } from "./controls";
import { IconThemeDark, IconThemeLight, IconThemeSystem } from "./icons-settings";
import { THEME_EVENT, setTheme, storedTheme, type ThemeChoice } from "../theme";
import { CardTitle } from "./CardTitle";

/** One choice of Settings › General: its name, a line under it, then
 * its options as equal cards. */
export function ChoiceGroup({ legend, hint, testId, labelledBy, children, end }: { legend: string; hint?: string; testId?: string; labelledBy?: string; children: ReactNode; end?: ReactNode }): JSX.Element {
  const id = `${useId().replace(/:/g, "")}-legend`;
  return (
    <div role="group" aria-labelledby={id} className="@container flex flex-col gap-2" data-testid={testId}>
      <div className="flex items-end justify-between gap-3">
        <div className="flex min-w-0 flex-col">
          <span id={id} className="text-meta font-medium text-ink">
            {legend}
          </span>
          {hint && <span className="text-label text-muted">{hint}</span>}
        </div>
        {end && <div className="shrink-0 whitespace-nowrap">{end}</div>}
      </div>
      <div role="radiogroup" aria-labelledby={labelledBy ?? id} className="grid grid-cols-1 gap-2 @xl:grid-cols-3">
        {children}
      </div>
    </div>
  );
}

export function AppearanceCard(): JSX.Element {
  const themeKeys = useKeys("theme");
  const [choice, setChoice] = useState<ThemeChoice>(storedTheme());
  useEffect(() => {
    const sync = () => setChoice(storedTheme());
    window.addEventListener(THEME_EVENT, sync);
    return () => window.removeEventListener(THEME_EVENT, sync);
  }, []);
  const options: Array<{ id: ThemeChoice; label: string; icon: ControlIcon }> = [
    { id: "system", label: "System", icon: IconThemeSystem },
    { id: "light", label: "Light", icon: IconThemeLight },
    { id: "dark", label: "Dark", icon: IconThemeDark },
  ];
  return (
    <Card className="flex flex-col gap-5 p-5" data-testid="settings-appearance">
      <CardTitle id="appearance-title">Appearance</CardTitle>
      <ChoiceGroup legend="Theme" hint={`${themeKeys.words} switches between them from anywhere.`} labelledBy="appearance-title">
        {options.map((option) => (
          <ChoiceOption
            key={option.id}
            type="radio"
            look="card"
            name="appearance"
            icon={option.icon}
            label={option.label}
            checked={choice === option.id}
            onChange={() => {
              setTheme(option.id);
              setChoice(option.id);
            }}
            data-testid={`appearance-${option.id}`}
          />
        ))}
      </ChoiceGroup>
      <DensityChoice />
      <RailOrderChoice />
    </Card>
  );
}
