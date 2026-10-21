/**
 * AJ-E1537 — the sessions feature's calls (ENGINEERING-GUIDE U3).
 */
import { invoke } from "@tauri-apps/api/core";
import { hasLiveBackend } from "../bridge";

/** The copyable line that resumes a session in a terminal: built by its
 * tool's adapter, on its account, from its folder. */
export async function sessionsResumeCommand(sessionId: string): Promise<string> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_RESUME_COMMAND__?: string }).__AIJ_RESUME_COMMAND__;
    return seeded ?? `cd ~/work && tool --resume ${sessionId}`;
  }
  return invoke<string>("sessions_resume_command", { sessionId });
}
