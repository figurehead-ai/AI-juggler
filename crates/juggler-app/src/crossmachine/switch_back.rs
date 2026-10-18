//! AJ-E706 Req.4 / AJ-E708 Req.4–5: switch-back across machines.

use std::sync::Arc;

use juggler_link::node::{HandoverAccepted, HandoverRejection, PeerAccess};
use juggler_link::proto;

use crate::state::AppState;

/// Node side: resume the original native session here with one delta
/// turn (D-05), refusing a diverged original unless the user chose.
pub(crate) fn switch_back_here(
    _state: &Arc<AppState>,
    _peer: &PeerAccess,
    _back: &proto::SwitchBack,
    _bundle: &proto::HandoverBundle,
) -> Result<HandoverAccepted, HandoverRejection> {
    Err(super::refuse(
        "unsupported",
        "Switch-back across machines isn't available yet.",
    ))
}
