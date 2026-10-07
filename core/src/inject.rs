//! Typing text at the cursor as X keysyms.

/// The keysym typing `c`: Latin-1 code points map to themselves, the rest of Unicode
/// to `0x0100_0000 + code point`. Control characters other than newline and tab are dropped.
pub fn keysym(c: char) -> Option<u32> {
    match c {
        '\n' => Some(0xff0d), // Return
        '\t' => Some(0xff09), // Tab
        // Typographic apostrophes, which the model outputs, are on no common layout and GNOME
        // drops keysyms missing from the active keymap: type the ASCII apostrophe.
        '’' | '‘' => Some(0x27),
        c if c.is_control() => None,
        c if (c as u32) < 0x100 => Some(c as u32),
        c => Some(0x0100_0000 + c as u32),
    }
}

/// The keysyms typing `c`. Circumflex and diaeresis letters are missing from the AZERTY
/// keymap, so GNOME would drop them: they are typed as dead key + base letter, like by hand.
pub fn keysyms(c: char) -> Vec<u32> {
    const CIRCUMFLEX: u32 = 0xfe52; // dead_circumflex
    const DIAERESIS: u32 = 0xfe57; // dead_diaeresis
    let (dead, base) = match c {
        'â' => (CIRCUMFLEX, 'a'),
        'ê' => (CIRCUMFLEX, 'e'),
        'î' => (CIRCUMFLEX, 'i'),
        'ô' => (CIRCUMFLEX, 'o'),
        'û' => (CIRCUMFLEX, 'u'),
        'Â' => (CIRCUMFLEX, 'A'),
        'Ê' => (CIRCUMFLEX, 'E'),
        'Î' => (CIRCUMFLEX, 'I'),
        'Ô' => (CIRCUMFLEX, 'O'),
        'Û' => (CIRCUMFLEX, 'U'),
        'ä' => (DIAERESIS, 'a'),
        'ë' => (DIAERESIS, 'e'),
        'ï' => (DIAERESIS, 'i'),
        'ö' => (DIAERESIS, 'o'),
        'ü' => (DIAERESIS, 'u'),
        'ÿ' => (DIAERESIS, 'y'),
        'Ä' => (DIAERESIS, 'A'),
        'Ë' => (DIAERESIS, 'E'),
        'Ï' => (DIAERESIS, 'I'),
        'Ö' => (DIAERESIS, 'O'),
        'Ü' => (DIAERESIS, 'U'),
        _ => return keysym(c).into_iter().collect(),
    };
    vec![dead, base as u32]
}

/// Where dictated text goes.
pub trait TextSink {
    /// Types `text` at the cursor of the focused application.
    fn type_text(&mut self, text: &str) -> Result<(), String>;
}

/// Records keysyms instead of typing them.
#[derive(Default)]
pub struct MemorySink {
    pub keysyms: Vec<u32>,
}

impl TextSink for MemorySink {
    fn type_text(&mut self, text: &str) -> Result<(), String> {
        self.keysyms.extend(text.chars().flat_map(keysyms));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::Emitter;

    #[test]
    fn maps_characters_to_keysyms() {
        let cases = [
            ('a', 0x61),
            (' ', 0x20),
            ('é', 0xe9),
            ('ç', 0xe7),
            ('É', 0xc9),
            ('’', 0x27),
            ('€', 0x0100_20ac),
            ('😀', 0x0101_f600),
            ('\n', 0xff0d),
            ('\t', 0xff09),
        ];
        for (c, k) in cases {
            assert_eq!(keysym(c), Some(k), "{c:?}");
        }
        assert_eq!(keysym('\u{7}'), None);
        assert_eq!(keysym('\u{85}'), None);
        assert_eq!(keysyms('û'), [0xfe52, 0x75]);
        assert_eq!(keysyms('Ë'), [0xfe57, 0x45]);
        assert_eq!(keysyms('é'), [0xe9]);
        assert!(keysyms('\u{7}').is_empty());
    }

    #[test]
    fn dictation_reaches_the_sink_without_backspace() {
        let mut emitter = Emitter::new(true);
        let mut sink = MemorySink::default();
        let partials = ["ça", "ça coûte", "ça coûte 5 €", "ça coûte 5 € 😀"];
        for p in partials {
            if let Some(t) = emitter.partial(p) {
                sink.type_text(&t).unwrap();
            }
        }
        sink.type_text(&emitter.finish("ça coûte 5 € 😀").unwrap())
            .unwrap();
        let typed: String = sink
            .keysyms
            .iter()
            .map(|&k| match k {
                0xfe52 => '^',
                k => char::from_u32(if k >= 0x0100_0000 { k - 0x0100_0000 } else { k }).unwrap(),
            })
            .collect();
        assert_eq!(typed, "Ça co^ute 5 € 😀 ");
        assert!(!sink.keysyms.contains(&0xff08), "no BackSpace");
    }
}
