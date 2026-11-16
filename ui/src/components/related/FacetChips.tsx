/**
 * E31 (AJ-E3103) — a list's filter chips by what its rows belong to (App,
 * Account, project, Mac, tool): History's and every related list's. Each
 * offers the values `useFacets` read, with how many rows each shows with
 * the other chips' choices (the 0.4.6 RC walk K2, F-K2-5: App = Codex CLI
 * chosen, Tool still offered Claude Code), "No project" and "No account"
 * among them; a value that would show nothing isn't offered. A Mac only
 * when the rows ran on more than one. A chosen value the other choices
 * leave empty stays listed, at 0, so its chip still names it.
 */
import type { JSX } from "react";
import { FilterChip } from "./FilterChip";
import type { FacetsByKind } from "./useFacets";
import type { FacetKind, Filters } from "./order";

/** The chip's value for "No project" and "No account" (`""` to the backend). */
const NONE = "(none)";

export interface FacetChip {
  kind: FacetKind;
  label: string;
}

export function FacetChips({
  chips,
  facets,
  filters,
  nameOf,
  onChange,
  testIdOf,
}: {
  chips: readonly FacetChip[];
  /** `null` until read. */
  facets: FacetsByKind | null;
  filters: Filters;
  nameOf: (kind: FacetKind, id: string) => string;
  onChange: (kind: FacetKind, value: string | null) => void;
  testIdOf: (kind: FacetKind) => string;
}): JSX.Element {
  return (
    <>
      {chips.map(({ kind, label }) => {
        const values = facets?.[kind] ?? [];
        const value = filters[kind];
        if (kind === "machine" && values.length < 2 && value === null) return null;
        const listed = value === null || values.some((facet) => facet.id === value) ? values : [...values, { id: value, count: facets ? 0 : undefined }];
        return (
          <FilterChip
            key={kind}
            label={label}
            value={value === "" ? NONE : value}
            options={listed.map((facet) => ({ id: facet.id || NONE, label: nameOf(kind, facet.id), count: facet.count }))}
            onChange={(next) => onChange(kind, next === NONE ? "" : next)}
            testId={testIdOf(kind)}
          />
        );
      })}
    </>
  );
}
