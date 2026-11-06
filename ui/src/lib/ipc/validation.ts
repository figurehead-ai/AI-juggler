/**
 * The validation matrix (dogfood cells).
 */
import { invoke, liveBackend as hasBackend } from "./transport";
import { previewHooks } from "../preview-hooks";

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

/** AJ-E210: the validation matrix (read-only disclosure — the UI
 * shows the same table the badges derive from; cells are recorded
 * by the maintainer, never from this surface). */
export async function validationMatrix(): Promise<ValidationCellData[]> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_VALIDATION_MATRIX__?: ValidationCellData[] })
        .__AIJ_VALIDATION_MATRIX__ ?? previewHooks().switching.DEFAULT_VALIDATION_MATRIX
    );
  }
  return invoke<ValidationCellData[]>("validation_matrix");
}
