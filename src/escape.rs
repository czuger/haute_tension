//! HTML escaping as the pages carry it, and not as Askama's own escaper does.
//!
//! The two differ on two characters: an apostrophe is written `&#39;` here and
//! `&#x27;` there, a quote `&#34;` and `&quot;`. The book is French and full of
//! apostrophes — "La Forteresse d'Alamuth", "Le début de l'aventure" — so the
//! difference is on nearly every page, and the pages are what the tests read.
//! `askama.toml` names this escaper for every `.html` template.

use std::fmt::{self, Write};

/// The escaper every template is rendered through.
#[derive(Clone, Copy, Debug, Default)]
pub struct Html;

impl askama::filters::Escaper for Html {
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
