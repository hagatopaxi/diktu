//! Typing text at the cursor as X keysyms.

use std::collections::HashSet;

use unicode_normalization::UnicodeNormalization;

/// The keysym of `c` as Unicode: Latin-1 code points map to themselves, the rest of Unicode
/// to `0x0100_0000 + code point`. Layouts name most other characters by their legacy keysym
/// (`Cyrillic_a`, `EuroSign`), which `Keymap::keysym` returns instead.
pub fn unicode_keysym(c: char) -> u32 {
    if (c as u32) < 0x100 {
        c as u32
    } else {
        0x0100_0000 + c as u32
    }
}

/// The keyboard layouts of the session. GNOME types a keysym only if a key of the active layout
/// produces it, without modifier, with Shift or with AltGr, and silently drops any other.
pub struct Keymap {
    /// The keysyms each layout produces; empty when unknown, which types every character as is.
    pub layouts: Vec<HashSet<u32>>,
    /// Index of the active layout in `layouts`.
    pub active: usize,
    /// The keysym naming a character.
    pub keysym: fn(char) -> u32,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            layouts: Vec::new(),
            active: 0,
            keysym: unicode_keysym,
        }
    }
}

/// The dead key adding a combining mark to the next letter.
fn dead_key(mark: char) -> Option<u32> {
    Some(match mark {
        '\u{300}' => 0xfe50, // dead_grave
        '\u{301}' => 0xfe51, // dead_acute
        '\u{302}' => 0xfe52, // dead_circumflex
        '\u{303}' => 0xfe53, // dead_tilde
        '\u{304}' => 0xfe54, // dead_macron
        '\u{306}' => 0xfe55, // dead_breve
        '\u{307}' => 0xfe56, // dead_abovedot
        '\u{308}' => 0xfe57, // dead_diaeresis
        '\u{30a}' => 0xfe58, // dead_abovering
        '\u{30b}' => 0xfe59, // dead_doubleacute
        '\u{30c}' => 0xfe5a, // dead_caron
        '\u{327}' => 0xfe5b, // dead_cedilla
        '\u{328}' => 0xfe5c, // dead_ogonek
        '\u{323}' => 0xfe60, // dead_belowdot
        '\u{309}' => 0xfe61, // dead_hook
        '\u{31b}' => 0xfe62, // dead_horn
        _ => return None,
    })
}

/// The keysyms typing `c` with `keymap`. Among the character itself, its dead-key sequence
/// (dead_circumflex, e for ê) and its unaccented spelling (C for Ç, oe for œ), the winner is the
/// first that the active layout types, exact spellings before the unaccented one, then that the
/// most layouts type, since the active layout may be out of date. Nothing is typed when no
/// layout can.
pub fn keysyms(c: char, keymap: &Keymap) -> Vec<u32> {
    let direct = match c {
        '\n' => 0xff0d, // Return
        '\t' => 0xff09, // Tab
        // The model outputs typographic apostrophes, which are on no common layout.
        '’' | '‘' => 0x27,
        c if c.is_control() => return Vec::new(),
        c => (keymap.keysym)(c),
    };
    if keymap.layouts.is_empty() {
        return vec![direct];
    }
    let mut candidates = vec![(vec![direct], false)];
    let nfd: Vec<char> = c.nfd().collect();
    if let [base, marks @ ..] = nfd.as_slice()
        && let Some(mut sequence) = marks
            .iter()
            .map(|&m| dead_key(m))
            .collect::<Option<Vec<_>>>()
        && !marks.is_empty()
    {
        sequence.push((keymap.keysym)(*base));
        candidates.push((sequence, false));
    }
    let plain = match c {
        'œ' => "oe",
        'Œ' => "OE",
        'æ' => "ae",
        'Æ' => "AE",
        'ß' => "ss",
        'ẞ' => "SS",
        _ => "",
    };
    let plain: Vec<char> = if plain.is_empty() {
        nfd.iter()
            .copied()
            .filter(|m| dead_key(*m).is_none())
            .collect()
    } else {
        plain.chars().collect()
    };
    if plain != [c] {
        candidates.push((plain.into_iter().map(keymap.keysym).collect(), true));
    }
    let types =
        |layout: &HashSet<u32>, sequence: &[u32]| sequence.iter().all(|k| layout.contains(k));
    candidates
        .into_iter()
        .map(|(sequence, lossy)| {
            let active = keymap
                .layouts
                .get(keymap.active)
                .is_some_and(|l| types(l, &sequence));
            let count = keymap
                .layouts
                .iter()
                .filter(|l| types(l, &sequence))
                .count();
            ((active, !lossy, count), sequence)
        })
        .filter(|((_, _, count), _)| *count > 0)
        .rev()
        .max_by_key(|(rank, _)| *rank)
        .map(|(_, sequence)| sequence)
        .unwrap_or_default()
}

/// Where dictated text goes.
pub trait TextSink {
    /// Types `text` at the cursor of the focused application.
    fn type_text(&mut self, text: &str) -> Result<(), String>;
}

/// Records keysyms instead of typing them.
#[derive(Default)]
pub struct MemorySink {
    pub keymap: Keymap,
    pub keysyms: Vec<u32>,
}

impl TextSink for MemorySink {
    fn type_text(&mut self, text: &str) -> Result<(), String> {
        let keymap = &self.keymap;
        self.keysyms
            .extend(text.chars().flat_map(|c| keysyms(c, keymap)));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::Emitter;

    /// Keysyms of xkb's basic layouts, at the levels GNOME types (none, Shift, AltGr).
    fn layout(name: &str) -> HashSet<u32> {
        let ascii = (0x20..0x7f).collect::<Vec<u32>>();
        let extra: &[u32] = match name {
            // French AZERTY: dead_circumflex and dead_diaeresis, but no ê, no Ç, no œ.
            "fr" => &[
                0xe9, 0xe8, 0xe0, 0xe7, 0xf9, 0xb2, 0xb0, 0xa3, 0xa7, 0xb5, 0x20ac, 0xfe52, 0xfe57,
            ],
            // French (alt.) on AltGr: ê, œ, Ç, but no dead_diaeresis.
            "fr-oss" => &[
                0xe9, 0xe8, 0xe0, 0xe7, 0xf9, 0xea, 0x13bd, 0xc7, 0x20ac, 0xfe52,
            ],
            "de" => &[
                0xe4, 0xf6, 0xfc, 0xc4, 0xd6, 0xdc, 0xdf, 0x20ac, 0xfe50, 0xfe51, 0xfe52,
            ],
            "es" => &[
                0xf1, 0xd1, 0xe7, 0xc7, 0xa1, 0xbf, 0x20ac, 0xfe50, 0xfe51, 0xfe52, 0xfe57,
            ],
            // Russian ЙЦУКЕН, legacy Cyrillic keysyms: а = 0x6c1, ё = 0x6a3, й = 0x6ca.
            "ru" => &[0x6c1, 0x6a3, 0x6ca, 0x6b3],
            _ => &[],
        };
        ascii.into_iter().chain(extra.iter().copied()).collect()
    }

    fn keymap(layouts: &[&str], active: usize) -> Keymap {
        Keymap {
            layouts: layouts.iter().map(|l| layout(l)).collect(),
            active,
            // The legacy keysyms GDK returns for the characters these tests use.
            keysym: |c| match c {
                '€' => 0x20ac,
                'œ' => 0x13bd,
                'а' => 0x6c1,
                'ё' => 0x6a3,
                'й' => 0x6ca,
                c => unicode_keysym(c),
            },
        }
    }

    #[test]
    fn without_a_keymap_characters_are_typed_as_is() {
        let none = Keymap::default();
        let cases = [
            ('a', 0x61),
            (' ', 0x20),
            ('é', 0xe9),
            ('É', 0xc9),
            ('’', 0x27),
            ('€', 0x0100_20ac),
            ('😀', 0x0101_f600),
            ('\n', 0xff0d),
            ('\t', 0xff09),
        ];
        for (c, k) in cases {
            assert_eq!(keysyms(c, &none), [k], "{c:?}");
        }
        assert!(keysyms('\u{7}', &none).is_empty());
        assert!(keysyms('\u{85}', &none).is_empty());
    }

    #[test]
    fn french_layout() {
        let fr = keymap(&["fr"], 0);
        assert_eq!(keysyms('é', &fr), [0xe9]);
        assert_eq!(keysyms('ç', &fr), [0xe7]);
        assert_eq!(keysyms('ê', &fr), [0xfe52, 0x65]);
        assert_eq!(keysyms('Û', &fr), [0xfe52, 0x55]);
        assert_eq!(keysyms('ï', &fr), [0xfe57, 0x69]);
        assert_eq!(keysyms('€', &fr), [0x20ac]);
        // No key and no dead key: the letter without its accent rather than nothing.
        assert_eq!(keysyms('É', &fr), [0x45]);
        assert_eq!(keysyms('Ç', &fr), [0x43]);
        assert_eq!(keysyms('œ', &fr), [0x6f, 0x65]);
        assert!(keysyms('а', &fr).is_empty());
    }

    #[test]
    fn other_layouts() {
        let de = keymap(&["de"], 0);
        assert_eq!(keysyms('ä', &de), [0xe4]);
        assert_eq!(keysyms('ß', &de), [0xdf]);
        assert_eq!(keysyms('é', &de), [0xfe51, 0x65]);
        let es = keymap(&["es"], 0);
        assert_eq!(keysyms('á', &es), [0xfe51, 0x61]);
        assert_eq!(keysyms('ñ', &es), [0xf1]);
        assert_eq!(keysyms('ü', &es), [0xfe57, 0x75]);
        assert_eq!(keysyms('¿', &es), [0xbf]);
        let ru = keymap(&["ru"], 0);
        assert_eq!(keysyms('а', &ru), [0x6c1]);
        assert_eq!(keysyms('ё', &ru), [0x6a3]);
        assert_eq!(keysyms('й', &ru), [0x6ca]);
    }

    #[test]
    fn several_layouts() {
        // ê is a key of French (alt.) only: the dead key types it in both layouts.
        let fr = keymap(&["fr", "fr-oss"], 1);
        assert_eq!(keysyms('ê', &fr), [0xfe52, 0x65]);
        // The active layout comes first: œ is a key of French (alt.).
        assert_eq!(keysyms('œ', &fr), [0x13bd]);
        assert_eq!(keysyms('œ', &keymap(&["fr", "fr-oss"], 0)), [0x6f, 0x65]);
        // Cyrillic, on the Russian layout only, is typed even if French looks active.
        assert_eq!(keysyms('а', &keymap(&["fr", "ru"], 0)), [0x6c1]);
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
            .map(|&k| char::from_u32(if k >= 0x0100_0000 { k - 0x0100_0000 } else { k }).unwrap())
            .collect();
        assert_eq!(typed, "Ça coûte 5 € 😀 ");
        assert!(!sink.keysyms.contains(&0xff08), "no BackSpace");
    }
}
