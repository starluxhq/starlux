//! Instructions injected into every run.
//!
//! Chat-only runs replace the provider's own prompt; agent runs append to it.
//! A CLI built for coding agents otherwise spends around 25k input tokens per
//! turn on tool scaffolding a launcher never uses, and answers in the register
//! of a terminal session rather than a desktop assistant.

use super::Past;

/// How much of a carried conversation is sent, in bytes.
///
/// Bounded by argv rather than by the model: gemini takes its prompt as an
/// argument, and Linux refuses a single argument over 128 KiB outright. That
/// limit has nothing to do with what a model could have read, which is why this
/// is not a context calculation and does not pretend to be one.
const CARRIED_BYTES: usize = 60_000;

/// The thread so far, for a provider that has not answered in it — what
/// changing provider mid-conversation leaves behind.
///
/// Sent as ordinary text ahead of the question, because that is the one thing
/// all three CLIs can take. The newest turns are kept and the oldest dropped,
/// and a thread that did not fit says so rather than reading as the whole of
/// what was said.
pub fn carried(history: &[Past]) -> Option<String> {
    if history.is_empty() {
        return None;
    }

    let mut kept: Vec<String> = Vec::new();
    let mut spent = 0;
    for turn in history.iter().rev() {
        let text = turn.text.trim();
        if text.is_empty() {
            continue;
        }
        let speaker = if turn.role == "assistant" {
            "Assistant"
        } else {
            "User"
        };
        let rendered = format!("{speaker}: {text}");
        if spent + rendered.len() > CARRIED_BYTES {
            break;
        }
        spent += rendered.len();
        kept.push(rendered);
    }

    if kept.is_empty() {
        return None;
    }
    if kept.len() < history.len() {
        kept.push("[earlier messages are not included]".to_owned());
    }
    kept.reverse();

    Some(format!(
        "The conversation so far, answered by a different assistant:

{}

That is the conversation you are joining. Do not reply to it. Answer only the \
message that follows.",
        kept.join("\n\n")
    ))
}

/// Rules both modes need, so a widget or an equation renders the same either way.
const FORMATTING: &str = "\
Mathematics is always LaTeX: inline as $...$, display as $$...$$. Never write an \
equation as plain text or as a code block.

When a table or a comparison of numbers would read better than prose, emit a widget: \
a fenced block whose language is `starlux-widget` and whose body is one JSON object, \
with nothing else inside the fence. Supported shapes:

{\"type\":\"table\",\"title\":string,\"columns\":string[],\"rows\":(string|number)[][]}
{\"type\":\"chart\",\"chart\":\"bar\"|\"line\",\"title\":string,\"x\":string[],\"series\":[{\"label\":string,\"values\":number[]}]}

When answering needs values only the user has, ask for them with a form rather than \
a paragraph of questions:

{\"type\":\"form\",\"title\":string,\"submit\":string,\"fields\":[{\"name\":string,\"label\":string,\"kind\":\"text\"|\"number\"|\"checkbox\"|\"select\",\"options\":string[],\"value\":string|number|boolean}]}

`options` belongs to `select` alone, `value` is an optional starting value, and every \
`name` is used once. `submit` is the question the filled form asks, with `{name}` where \
each value goes — write it as the user would have typed it. Submitting it sends that \
sentence as their next message, so ask only for what you cannot work out yourself, and \
answer directly when nothing is missing.

Prose may introduce a widget, but never restate the data it already carries. Use one \
only when it earns its place; ordinary answers stay prose.

Diagrams — flowcharts, sequences, state machines, timelines — go in a `mermaid` fence.

When the answer is something to try rather than something to read, such as a simulation, \
a visualisation or a small tool, emit a fenced block whose language is `starlux-artifact` \
and whose body is one complete HTML document with all CSS and JavaScript inline. Name it \
on the fence: ```starlux-artifact title=\"Orbit simulation\". It runs sandboxed with no \
network access at all, so nothing may be loaded from a CDN and no fonts, styles or scripts \
may be fetched. Everything it needs is in the document.";

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

/// Names a conversation from the question that opened it. It runs alongside
/// that question, so the answer does not exist yet to be shown.
///
/// Deliberately without `FORMATTING`: the rules for widgets, artifacts and
/// LaTeX have nothing to say about four words in a sidebar, and paying for
/// them here would undo the point of running a second, cheap model at all.
pub fn title() -> String {
    "You name conversations. You are given the first message a user sent, and you reply \
with a title for the conversation it opens.

Six words at most. Name the subject, not the act of asking: `Spectral classes`, never \
`User asks about spectral classes`. Write it in the language the user wrote in. No \
quotation marks, no trailing punctuation, no preamble.

Answer the message with a title for it. Never answer the message itself."
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn past(role: &str, text: &str) -> Past {
        Past {
            role: role.into(),
            text: text.into(),
        }
    }

    #[test]
    fn a_thread_with_nothing_in_it_carries_nothing() {
        assert_eq!(carried(&[]), None);
        assert_eq!(carried(&[past("user", "   ")]), None);
    }

    #[test]
    fn both_sides_of_the_thread_are_named_and_the_question_is_not_reopened() {
        let text = carried(&[
            past("user", "what is a pulsar?"),
            past("assistant", "a spinning neutron star"),
        ])
        .unwrap();
        assert!(text.contains("User: what is a pulsar?"));
        assert!(text.contains("Assistant: a spinning neutron star"));
        assert!(text.contains("Do not reply to it"));
    }

    /// The oldest go first, and the gap is declared: a thread that silently
    /// lost its beginning reads as the whole of what was said.
    #[test]
    fn a_thread_too_long_for_argv_keeps_the_newest_and_says_so() {
        let bulk = "x".repeat(CARRIED_BYTES / 2);
        let text = carried(&[
            past("user", &bulk),
            past("assistant", &bulk),
            past("user", "the newest thing said"),
        ])
        .unwrap();
        assert!(text.contains("the newest thing said"));
        assert!(text.contains("earlier messages are not included"));
        assert!(text.len() < CARRIED_BYTES * 2);
    }

    #[test]
    fn both_modes_carry_the_same_rendering_rules() {
        for prompt in [chat(), agent()] {
            assert!(prompt.contains("$$"));
            assert!(prompt.contains("starlux-widget"));
            assert!(prompt.contains("\"form\""));
            assert!(prompt.contains("starlux-artifact"));
            assert!(prompt.contains("mermaid"));
        }
    }

    // A title is four words in a sidebar. Sending the widget, artifact and
    // LaTeX rules with it would cost more than the answer it is naming.
    #[test]
    fn naming_a_conversation_carries_none_of_the_rendering_rules() {
        let title = title();
        for rule in ["$$", "starlux-widget", "starlux-artifact", "mermaid"] {
            assert!(!title.contains(rule), "title prompt still carries `{rule}`");
        }
    }

    #[test]
    fn only_chat_mode_replaces_the_provider_identity() {
        assert!(chat().starts_with("You are Starlux"));
        // Appending this would argue with the identity the provider already set.
        assert!(!agent().contains("You are Starlux"));
    }
}
