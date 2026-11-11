//! 0.4.1: a background thread on the lowest priority the system gives
//! one, so catch-up work (the subagent count's first reads after a start)
//! never competes with the window. The safe API of `thread-priority`.

/// Puts the calling thread on the lowest priority. False where the system
/// refused (the work then runs as before, only not lowered).
#[must_use]
pub fn lowest_priority() -> bool {
    thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Min).is_ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_new_thread_can_be_lowered() {
        let lowered = std::thread::spawn(super::lowest_priority)
            .join()
            .unwrap_or(false);
        assert!(lowered);
    }
}
