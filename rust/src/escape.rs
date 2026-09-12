//! HTML escaping as Jinja does it, and not as Askama does.
//!
//! The two differ on two characters: Jinja writes `&#39;` for an apostrophe and
//! `&#34;` for a quote, Askama `&#x27;` and `&quot;`. The book is French and
//! full of apostrophes — "La Forteresse d'Alamuth", "Le début de l'aventure" —
//! so the difference is on nearly every page, and the pages are what the tests
//! read. `askama.toml` names this escaper for every `.html` template.

use std::fmt::{self, Write};

/// The escaper Jinja's `escape()` is.
#[derive(Clone, Copy, Debug, Default)]
pub struct Jinja;

impl askama::filters::Escaper for Jinja {
    fn write_escaped_str<W: Write>(&self, mut dest: W, string: &str) -> fmt::Result {
        let mut written = 0;
        for (index, character) in string.char_indices() {
            let replacement = match character {
                '&' => "&amp;",
                '<' => "&lt;",
                '>' => "&gt;",
                '"' => "&#34;",
                '\'' => "&#39;",
                _ => continue,
            };
            dest.write_str(&string[written..index])?;
            dest.write_str(replacement)?;
            written = index + character.len_utf8();
        }
        dest.write_str(&string[written..])
    }
}
