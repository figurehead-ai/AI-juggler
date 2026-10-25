/**
 * The app's parts loaded on first use (lib/lazy): each is fetched in idle
 * time after the first paint, so opening one doesn't wait. What shows at
 * start, or mounts at start to listen for its moment, stays in App's own
 * imports.
 */
import { lazyNamed } from "./lib/lazy";
import { prefetchMainScreens } from "./lib/prefetch-data";

// Their first data too, in the same idle queue (after the first paint).
prefetchMainScreens();

export const ConversationPage = lazyNamed(() => import("./components/ConversationPage"), "ConversationPage");
export const PortWarningSheet = lazyNamed(() => import("./components/PortMap"), "PortWarningSheet");
export const CleanupCentre = lazyNamed(() => import("./screens/CleanupCentre"), "CleanupCentre");
export const ThisMacScreen = lazyNamed(() => import("./screens/ThisMacScreen"), "ThisMacScreen");
export const PrivacyScreen = lazyNamed(() => import("./screens/PrivacyScreen"), "PrivacyScreen");
export const ShortcutSheet = lazyNamed(() => import("./components/ShortcutSheet"), "ShortcutSheet");
export const History = lazyNamed(() => import("./screens/History"), "History");
export const AddProviderSheet = lazyNamed(() => import("./screens/AddProviderSheet"), "AddProviderSheet");
export const FirstRun = lazyNamed(() => import("./screens/FirstRun"), "FirstRun");
export const UsageScreen = lazyNamed(() => import("./screens/Usage"), "UsageScreen");
export const LimitDialog = lazyNamed(() => import("./components/LimitDialog"), "LimitDialog");
export const NewSessionModal = lazyNamed(() => import("./screens/NewSession"), "NewSessionModal");
export const NewProjectSheet = lazyNamed(() => import("./screens/NewProjectSheet"), "NewProjectSheet");
export const FleetScreen = lazyNamed(() => import("./screens/FleetScreen"), "FleetScreen");
export const SettingsScreen = lazyNamed(() => import("./screens/Settings"), "SettingsScreen");
export const SessionPanel = lazyNamed(() => import("./screens/WorkPanels"), "SessionPanel", true);
export const SwitchFlow = lazyNamed(() => import("./components/SwitchFlow"), "SwitchFlow");
export const SwitchBackFlow = lazyNamed(() => import("./components/SwitchBackFlow"), "SwitchBackFlow");
