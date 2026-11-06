/**
 * The validation matrix (dogfood cells).
 */
import { invoke, liveBackend as hasBackend } from "./transport";

/** AJ-E210: one recorded live dogfood cell as the Beta table reads it. */
interface DogfoodCellSummaryData {
  at: string;
  project_shape: string;
  verdict: "pass" | "fail" | string;
  source_sha256: string;
  target_sha256: string;
  note: string;
}

/** AJ-E210: one ordered-pair validation matrix row (Settings →
 * Beta). The SAME matrix the switch badges derive from — one
 * source, no drift (D-04). */
export interface ValidationCellData {
  from: string;
  to: string;
  /** The tools on each side, and (AJ-E1601) whether a side runs its tool
   * through another service. */
  from_tool?: string;
  to_tool?: string;
  from_service?: boolean;
  to_service?: boolean;
  mechanism: string;
  researched_ceiling: string;
  advertised: string;
  badge: string;
  validated: boolean;
  mechanism_green: boolean;
  ci_green: boolean;
  dogfood?: DogfoodCellSummaryData | null;
  status: string;
  notes: string[];
}

/** The no-backend matrix default (mirrors the researched table:
 * one pair with real-tool tests, every pair honestly in beta). */
const DEFAULT_VALIDATION_MATRIX: ValidationCellData[] = [
  ["claude-code", "claude-code", "transplant", "F3", true],
  ["codex", "codex", "transplant", "F3", false],
  ["zcode", "zcode", "transplant", "F1", false],
  ["claude-code", "codex", "replay", "F1", false],
  ["codex", "claude-code", "replay", "F1", false],
  ["claude-code", "zcode", "replay", "F0", false],
  ["zcode", "claude-code", "replay", "F1", false],
  ["codex", "zcode", "replay", "F0", false],
  ["zcode", "codex", "replay", "F1", false],
  ["claude-code", "gemini-cli", "pack-only", "F0", false],
  ["claude-code", "opencode", "pack-only", "F0", false],
].map(([from, to, mechanism, ceiling, ciGreen]) => ({
  from: from as string,
  to: to as string,
  mechanism: mechanism as string,
  researched_ceiling: ceiling as string,
  advertised: ceiling as string,
  badge: `${ceiling === "F3" ? "Same conversation, another app" : ceiling === "F1" ? "Full story, fresh start" : "Handover notes"} · Beta`,
  validated: false,
  mechanism_green: true,
  ci_green: ciGreen as boolean,
  dogfood: null,
  status: ciGreen
    ? "Automatic tests pass with the real tools; not tried by hand yet"
    : "Automatic tests with the real tools aren't in place; not tried by hand yet",
  notes: [],
}));

/** AJ-E210: the validation matrix (read-only disclosure — the UI
 * shows the same table the badges derive from; cells are recorded
 * by the maintainer, never from this surface). */
export async function validationMatrix(): Promise<ValidationCellData[]> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_VALIDATION_MATRIX__?: ValidationCellData[] })
        .__AIJ_VALIDATION_MATRIX__ ?? DEFAULT_VALIDATION_MATRIX
    );
  }
  return invoke<ValidationCellData[]>("validation_matrix");
}
