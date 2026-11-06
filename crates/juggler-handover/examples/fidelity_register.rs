//! AJ-E808: prints the fidelity register as Markdown
//! (`just fidelity-register` writes it to the evidence store).
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example's output is what it prints"
)]

fn main() {
    print!(
        "{}",
        juggler_handover::register::render_markdown(juggler_handover::register::embedded())
    );
}
