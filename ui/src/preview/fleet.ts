/** The design preview's Figurehead features and pairing peers (moved from lib/ipc, AJ-E2018). */
import type { CloudFeatureInfo } from "../types/generated/CloudFeatureInfo";

export const MOCK_FEATURES: CloudFeatureInfo[] = [
  { key: "cloudBackup", access: "included", tier: "free", on: false },
  { key: "fleetDiscovery", access: "included", tier: "free", on: false },
  { key: "usageSync", access: "included", tier: "free", on: false },
];

export const MOCK_JOINER = {
  label: "Laptop",
  fingerprint: "9B10 44CE 7A0F E2D3 51B8 06C9 3E7D A425",
};

export const MOCK_HOST = {
  label: "Mac mini",
  fingerprint: "C2E9 1F7A 0B36 8D54 E1A0 7C29 46BF 3D18",
};
