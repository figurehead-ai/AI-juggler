/** What the detail panel shows: the item selected in the rail, a page, or
 * nothing (Home). */
export type Selection =
  | { kind: "none" }
  | { kind: "project"; id: string }
  | { kind: "session"; id: string }
  | { kind: "login"; id: string }
  | { kind: "provider"; id: string }
  | { kind: "conversation"; accountId: string; id: string; projectId: string | null } // AJ-E1529
  | { kind: "pool"; id: string } // AJ-E1703: a pool's page
  | { kind: "app"; id: string } | { kind: "subscription"; id: string } // D-62: an App's page (AJ-E3204), an Account's (AJ-E3205)
  // AJ-E705: a paired machine (instanceId null) or an Instance on it.
  | { kind: "remote"; machineId: string; instanceId: string | null };
