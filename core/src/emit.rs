//! Turns growing recognizer hypotheses into text that can be typed without BackSpace.

/// Emits only the stable part of each hypothesis: whole words, up to the last space.
#[derive(Default)]
pub struct Emitter {
    typed: String,
    /// Capitalise the segment and end it with a period.
    pub postprocess: bool,
}

impl Emitter {
    pub fn new(postprocess: bool) -> Self {
        Self {
            typed: String::new(),
            postprocess,
        }
    }

    /// Text to type for a partial hypothesis, if a new word became stable.
    pub fn partial(&mut self, hypothesis: &str) -> Option<String> {
        let text = self.shape(hypothesis.trim_start());
        let stable = &text[..text.rfind(' ').map_or(0, |i| i + 1)];
        // The model may revise text already typed: then wait for the end of the segment.
        if stable.len() > self.typed.len() && stable.starts_with(&self.typed) {
            let delta = stable[self.typed.len()..].to_owned();
            self.typed = stable.to_owned();
            Some(delta)
        } else {
            None
        }
    }

    /// Text to type once the segment is final (the rest, then a space); resets the emitter.
    pub fn finish(&mut self, hypothesis: &str) -> Option<String> {
        let mut text = self.shape(hypothesis.trim());
        if self.postprocess && text.ends_with(char::is_alphanumeric) {
            text.push('.');
        }
        let typed = std::mem::take(&mut self.typed);
        let rest = match text.strip_prefix(&typed) {
            Some(rest) => rest.to_owned(),
            // Diverged from what was typed: skip as many words as were already typed.
            None => text
                .split_inclusive(' ')
                .skip(typed.split_whitespace().count())
                .collect(),
        };
        (!rest.is_empty()).then(|| rest + " ")
    }

    fn shape(&self, text: &str) -> String {
        let mut chars = text.chars();
        match chars.next() {
            Some(first) if self.postprocess => first.to_uppercase().chain(chars).collect(),
            _ => text.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(emitter: &mut Emitter, partials: &[&str], last: &str) -> String {
        let mut out: String = partials.iter().filter_map(|p| emitter.partial(p)).collect();
        out += &emitter.finish(last).unwrap_or_default();
        out
    }

    #[test]
    fn types_whole_words_only_then_the_rest() {
        let mut e = Emitter::new(false);
        assert_eq!(e.partial("bon"), None);
        assert_eq!(e.partial("bonjour"), None);
        assert_eq!(e.partial("bonjour à"), Some("bonjour ".into()));
        assert_eq!(e.partial("bonjour à tou"), Some("à ".into()));
        assert_eq!(e.partial("bonjour à tous"), None);
        assert_eq!(e.finish("bonjour à tous"), Some("tous ".into()));
    }

    #[test]
    fn postprocess_capitalises_and_ends_with_period() {
        let mut e = Emitter::new(true);
        let out = run(
            &mut e,
            &["ce", "ce dernier", "ce dernier évolue"],
            "ce dernier évolue",
        );
        assert_eq!(out, "Ce dernier évolue. ");
        // Existing punctuation is kept, and each segment starts afresh.
        assert_eq!(run(&mut e, &["ça va"], "ça va ?"), "Ça va ? ");
        assert_eq!(run(&mut Emitter::new(false), &[], "ça va"), "ça va ");
    }

    #[test]
    fn revised_words_are_never_retyped() {
        let mut e = Emitter::new(false);
        assert_eq!(e.partial("le chat est"), Some("le chat ".into()));
        // The model rewrites a word already typed: nothing more until the end.
        assert_eq!(e.partial("le chats est là"), None);
        assert_eq!(e.finish("le chats est là"), Some("est là ".into()));
    }

    #[test]
    fn empty_segment_types_nothing() {
        let mut e = Emitter::new(true);
        assert_eq!(e.partial(""), None);
        assert_eq!(e.finish("  "), None);
    }
}
