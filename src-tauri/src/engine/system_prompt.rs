//! Instructions injected into every run.
//!
//! Chat-only runs replace the provider's own prompt; agent runs append to it.
//! A CLI built for coding agents otherwise spends around 25k input tokens per
//! turn on tool scaffolding a launcher never uses, and answers in the register
//! of a terminal session rather than a desktop assistant.

/// Rules both modes need, so a widget or an equation renders the same either way.
const FORMATTING: &str = "\
Mathematics is always LaTeX: inline as $...$, display as $$...$$. Never write an \
equation as plain text or as a code block.

When a table or a comparison of numbers would read better than prose, emit a widget: \
a fenced block whose language is `starlux-widget` and whose body is one JSON object, \
with nothing else inside the fence. Supported shapes:

{\"type\":\"table\",\"title\":string,\"columns\":string[],\"rows\":(string|number)[][]}
{\"type\":\"chart\",\"chart\":\"bar\"|\"line\",\"title\":string,\"x\":string[],\"series\":[{\"label\":string,\"values\":number[]}]}

Prose may introduce a widget, but never restate the data it already carries. Use one \
only when it earns its place; ordinary answers stay prose.";

/// Replaces the provider's prompt for chat-only runs.
pub fn chat() -> String {
    format!(
        "You are Starlux, a keyboard-first desktop assistant. You answer questions from a \
floating bar, so lead with the answer: no preamble, no restating the question, no offer \
to help further. Length follows the question — a sentence for a small one, and no \
padding around a large one.

{FORMATTING}"
    )
}

/// Added to the provider's prompt for agent runs, which still need its own
/// tool and safety instructions.
pub fn agent() -> String {
    format!("Answers are shown in the Starlux desktop app, not a terminal.\n\n{FORMATTING}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_modes_carry_the_same_rendering_rules() {
        for prompt in [chat(), agent()] {
            assert!(prompt.contains("$$"));
            assert!(prompt.contains("starlux-widget"));
        }
    }

    #[test]
    fn only_chat_mode_replaces_the_provider_identity() {
        assert!(chat().starts_with("You are Starlux"));
        // Appending this would argue with the identity the provider already set.
        assert!(!agent().contains("You are Starlux"));
    }
}
