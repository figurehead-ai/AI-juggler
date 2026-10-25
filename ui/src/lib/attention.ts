/**
 * The words for an attention rule (D-25: never the rule's code), wherever
 * a row or a header says why a session needs you or ended as it did.
 */
export function attentionWords(rule: string | null | undefined): string {
  switch (rule) {
    case "waiting_prompt":
    case "waiting_user":
      return "waiting for your reply";
    // AJ-E2306/E2303: the tool itself said it asks (its hooks or protocol).
    case "tool_asks":
      return "asks you";
    case "stuck_no_output":
    case "stuck":
      return "no output for a while";
    case "quota_exit":
    case "quota_blocked":
    case "quota_wall":
      return "hit its usage limit";
    case "failed":
      return "stopped with an error";
    // AJ-E1509: why a crashed session wasn't started again, and a session
    // that ended with AI Juggler.
    case "restart_gave_up":
      return "crashed 3 times in 10 minutes, so it wasn't restarted";
    case "app_quit_unexpectedly":
      return "ended when AI Juggler quit unexpectedly";
    default:
      return "needs attention";
  }
}
