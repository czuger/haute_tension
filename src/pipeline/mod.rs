//! The offline import pipeline: local gamebook HTML turned into one JSON file
//! per book.
//!
//! Unconnected to the rest at runtime. The application reads
//! `books/<series>/<book>/pages.json`; this is what produced it, and it is run by
//! hand when the source pages change.

pub mod parse_pages;
