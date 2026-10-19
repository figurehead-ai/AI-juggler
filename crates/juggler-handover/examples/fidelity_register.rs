//! AJ-E808: prints the fidelity register as Markdown
//! (`just fidelity-register` writes it to the evidence store).

fn main() {
    print!(
        "{}",
        juggler_handover::register::render_markdown(juggler_handover::register::embedded())
    );
}
