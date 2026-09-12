//! The offline import pipeline, read with `scraper`.
//!
//! The pipeline is run by hand and the application never calls it, so these read
//! pages off disk the way `parse_book` does: the document is walked once and the
//! link markers are written into the text as it is collected, the tree itself
//! left alone.

use haute_tension::pipeline::parse_pages::{
    normalize_text, parse_book, parse_file, slug_key, sort_key, url_path, PageError,
};
use std::path::Path;

/// One HTML page on disk, as the site wrote it.
fn page(body: &str) -> String {
    format!(
        "<!doctype html><html><head><meta property=\"og:url\" \
         content=\"https://lesitedontvousetesleheros.example/22.html\">\
         <title>22 - Le livre</title></head><body>{body}</body></html>"
    )
}

/// Write one page into a book directory of the test's own.
fn a_book(pages: &[(&str, String)]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory is always made");
    let book = directory.path().join("livre");
    std::fs::create_dir_all(&book).expect("the book directory is made");
    for (name, source) in pages {
        std::fs::write(book.join(name), source).expect("the page is written");
    }
    directory
}

/// Collapsing whitespace and dropping soft hyphens.
mod normalising_text {
    use super::*;

    #[test]
    fn runs_of_whitespace_become_one_space() {
        assert_eq!(normalize_text("un   deux\n\ttrois"), "un deux trois");
    }

    #[test]
    fn a_non_breaking_space_is_an_ordinary_one() {
        assert_eq!(normalize_text("un\u{a0}deux"), "un deux");
    }

    #[test]
    fn a_soft_hyphen_is_dropped() {
        assert_eq!(normalize_text("for\u{ad}teresse"), "forteresse");
    }

    #[test]
    fn the_edges_are_trimmed() {
        assert_eq!(normalize_text("  un  "), "un");
    }
}

/// Reducing text to what a slug can be matched on.
mod slug_keys {
    use super::*;

    #[test]
    fn accents_and_case_are_dropped() {
        assert_eq!(slug_key("Le Début"), "ledebut");
    }

    #[test]
    fn ligatures_are_spelled_out() {
        assert_eq!(slug_key("cœur"), "coeur");
        assert_eq!(slug_key("nævus"), "naevus");
    }

    #[test]
    fn punctuation_goes_altogether() {
        assert_eq!(slug_key("l'épée, brisée !"), "lepeebrisee");
    }
}

/// The decoded path of a page URL.
mod page_url_paths {
    use super::*;

    #[test]
    fn the_trailing_slash_is_dropped() {
        assert_eq!(
            url_path("https://lesitedontvousetesleheros.example/22/"),
            "/22"
        );
    }

    #[test]
    fn an_escaped_path_is_read_back() {
        assert_eq!(
            url_path("https://lesitedontvousetesleheros.example/le-d%C3%A9but.html"),
            "/le-début.html"
        );
    }
}

/// Ordering numeric pages first, primed variants after their base number.
mod ordering_pages {
    use super::*;

    #[test]
    fn numbers_come_before_anything_else() {
        assert!(sort_key("32") < sort_key("annexe"));
    }

    #[test]
    fn a_variant_follows_its_base_number() {
        assert!(sort_key("32") < sort_key("32'"));
        assert!(sort_key("32'") < sort_key("33"));
    }

    #[test]
    fn numbers_are_ordered_as_numbers() {
        assert!(sort_key("9") < sort_key("10"));
    }
}

/// Reading one page off disk.
mod parsing_one_file {
    use super::*;

    #[test]
    fn the_heading_names_the_page() {
        let book = a_book(&[("22.html", page("<h2>22</h2><div class=\"ob-text\"><p>Un.</p></div>"))]);

        let parsed = parse_file(
            &book.path().join("livre").join("22.html"),
            "livre/22.html",
            false,
        )
        .expect("the page reads");

        assert_eq!(parsed.page_id, "22");
        assert_eq!(parsed.text, ["Un."]);
    }

    #[test]
    fn the_title_names_a_page_with_no_heading() {
        let book = a_book(&[("22.html", page("<div class=\"ob-text\"><p>Un.</p></div>"))]);

        let parsed = parse_file(
            &book.path().join("livre").join("22.html"),
            "livre/22.html",
            false,
        )
        .expect("the page reads");

        assert_eq!(parsed.page_id, "22");
    }

    #[test]
    fn a_page_without_a_text_block_is_refused() {
        let book = a_book(&[("22.html", page("<h2>22</h2><p>Un.</p>"))]);

        let trouble = parse_file(
            &book.path().join("livre").join("22.html"),
            "livre/22.html",
            false,
        )
        .expect_err("the page has no text block");

        assert!(matches!(trouble, PageError::NoText));
    }

    #[test]
    fn a_line_break_splits_one_block_into_two_segments() {
        let book = a_book(&[(
            "22.html",
            page("<h2>22</h2><div class=\"ob-text\"><p>Un.<br>Deux.</p></div>"),
        )]);

        let parsed = parse_file(
            &book.path().join("livre").join("22.html"),
            "livre/22.html",
            false,
        )
        .expect("the page reads");

        assert_eq!(parsed.text, ["Un.", "Deux."]);
    }

    #[test]
    fn a_block_wrapping_another_is_not_a_segment_of_its_own() {
        let book = a_book(&[(
            "22.html",
            page("<h2>22</h2><div class=\"ob-text\"><li><p>Un.</p></li></div>"),
        )]);

        let parsed = parse_file(
            &book.path().join("livre").join("22.html"),
            "livre/22.html",
            false,
        )
        .expect("the page reads");

        assert_eq!(parsed.text, ["Un."]);
    }

    #[test]
    fn a_link_to_the_site_is_a_branch() {
        let book = a_book(&[(
            "22.html",
            page(
                "<h2>22</h2><div class=\"ob-text\"><p>Allez au \
                 <a href=\"https://lesitedontvousetesleheros.example/23.html\">23</a>.</p></div>",
            ),
        )]);

        let parsed = parse_file(
            &book.path().join("livre").join("22.html"),
            "livre/22.html",
            false,
        )
        .expect("the page reads");

        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].base.as_deref(), Some("23"));
    }

    #[test]
    fn a_link_off_the_site_is_not() {
        let book = a_book(&[(
            "22.html",
            page(
                "<h2>22</h2><div class=\"ob-text\"><p>Voir \
                 <a href=\"https://elsewhere.example/23.html\">là</a>.</p></div>",
            ),
        )]);

        let parsed = parse_file(
            &book.path().join("livre").join("22.html"),
            "livre/22.html",
            false,
        )
        .expect("the page reads");

        assert!(parsed.links.is_empty());
    }
}

/// Reading a whole book, and resolving what the links point at.
mod parsing_a_book {
    use super::*;

    /// One page whose own URL says which number it really is.
    fn linked_book() -> tempfile::TempDir {
        a_book(&[
            (
                "22.html",
                page(
                    "<h2>22</h2><div class=\"ob-text\"><p>Allez vers \
                     <a href=\"https://lesitedontvousetesleheros.example/23.html\">la forteresse</a>.</p></div>",
                ),
            ),
            (
                "23.html",
                "<!doctype html><html><head><meta property=\"og:url\" \
                 content=\"https://lesitedontvousetesleheros.example/23.html\">\
                 <title>23 - Le livre</title></head><body>\
                 <h2>23</h2><div class=\"ob-text\"><p>La fin.</p></div></body></html>"
                    .to_string(),
            ),
        ])
    }

    #[test]
    fn every_page_is_read() {
        let book = linked_book();

        let (pages, report) = parse_book(
            &book.path().join("livre"),
            book.path(),
            false,
        );

        assert_eq!(report.files_scanned, 2);
        assert_eq!(report.pages_parsed, 2);
        assert!(report.failures.is_empty());
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn a_link_resolves_to_the_page_its_url_names() {
        let book = linked_book();

        let (pages, _) = parse_book(&book.path().join("livre"), book.path(), false);

        assert_eq!(pages["22"].numbers(), ["23"]);
    }

    #[test]
    fn the_resolved_number_can_be_written_into_the_text() {
        let book = linked_book();

        let (pages, _) = parse_book(&book.path().join("livre"), book.path(), true);

        assert_eq!(pages["22"].text, ["Allez vers la forteresse 23."]);
    }

    #[test]
    fn a_page_nothing_points_at_is_reported() {
        let book = linked_book();

        let (_, report) = parse_book(&book.path().join("livre"), book.path(), false);

        assert_eq!(report.pages_without_numbers, ["23"]);
    }

    #[test]
    fn the_file_path_is_recorded_relative_to_the_base() {
        let book = linked_book();

        let (pages, _) = parse_book(&book.path().join("livre"), book.path(), false);

        assert_eq!(pages["22"].file_path, "livre/22.html");
        assert!(Path::new(&pages["22"].file_path).is_relative());
    }
}
