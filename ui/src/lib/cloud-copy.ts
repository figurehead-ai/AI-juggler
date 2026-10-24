/**
 * Settings › Account & Cloud: what each cloud feature does and sends, and
 * which start on (moved out of Settings.tsx so it stays within its size).
 */

/** What each cloud feature does and sends (plain words, D-18 §4.5). */
export const FEATURE_COPY: Record<string, { title: string; does: string; sends: string; note?: string; locked?: string }> = {
  cloudBackup: {
    title: "Cloud backup",
    does: "Back up AI Juggler's settings, Launch settings and history, and restore them on another Mac.",
    sends: "An encrypted backup file. Figurehead can't read it.",
  },
  fleetDiscovery: {
    title: "Find my Macs",
    does: "Lists your other linked Macs in Fleet so they can pair without a code — you still confirm each one.",
    sends: "This Mac's name, platform, app version and public key.",
    note: "Arrives in an upcoming beta build — your choice is kept until then.",
  },
  usageSync: {
    title: "Sync usage stats to my dashboard",
    does: "Shows your weekly usage statistics on figurehead.ai.",
    sends: "The weekly usage-statistics report, linked to your account.",
    locked: "Turn on in Settings › Privacy, while sharing usage stats",
  },
  alerts: {
    title: "Alerts when you're away",
    does: "Sends AI Juggler's alerts to you when you're away from this Mac.",
    sends: "The alert's text and time.",
  },
  configSync: {
    title: "Sync config across Macs",
    does: "Keeps your rules and settings the same on your Macs — you review each change.",
    sends: "Encrypted config files. Figurehead can't read them.",
  },
  transcriptVault: {
    title: "Transcript vault",
    does: "Keeps an encrypted archive of your transcripts you can search on any of your Macs.",
    sends: "Encrypted transcripts. Figurehead can't read them.",
  },
};

/** The consent screen's defaults (D-18 §4.3): the services you connect
 * for are on; usage sync is opt-in; everything else starts off. */
export const CONSENT_DEFAULT_ON = new Set(["cloudBackup", "fleetDiscovery"]);
