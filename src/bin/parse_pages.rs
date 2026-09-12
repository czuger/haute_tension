//! The offline import pipeline, run by hand when the source pages change.

use std::path::PathBuf;

use clap::Parser;
use haute_tension::pipeline::parse_pages::{parse_book, print_report, write_book};

/// Parse every book found in the input directory.
#[derive(Parser, Debug)]
#[command(about = "Parse local gamebook HTML pages into one JSON file per book.")]
struct Arguments {
    /// Directory whose sub-directories are books.
    #[arg(short = 'i', long = "input-dir", default_value = "work/raw_data")]
    input_dir: PathBuf,

    /// Directory receiving one JSON file per book.
    #[arg(short = 'o', long = "output-dir", default_value = "work/parsed_data")]
    output_dir: PathBuf,

    /// Append each link's target number after its text, as `<text> <number>`.
    #[arg(short = 'n', long = "inline-numbers")]
    inline_numbers: bool,

    /// Log per-file details.
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = Arguments::parse();
    let input_dir = arguments.input_dir.canonicalize()?;
    let base_dir = input_dir.parent().unwrap_or(&input_dir).to_path_buf();

    let mut book_dirs: Vec<PathBuf> = std::fs::read_dir(&input_dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    book_dirs.sort();

    for book_dir in book_dirs {
        let (pages, report) = parse_book(&book_dir, &base_dir, arguments.inline_numbers);
        let name = book_dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        write_book(&pages, &arguments.output_dir.join(format!("{name}.json")))?;
        print_report(&name, &report);
        if arguments.verbose {
            for (file_path, reason) in &report.failures {
                eprintln!("DEBUG skipped {file_path}: {reason}");
            }
        }
    }
    Ok(())
}
