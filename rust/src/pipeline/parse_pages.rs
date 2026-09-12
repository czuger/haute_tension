//! Parse local gamebook HTML pages into one JSON file per book.
//!
//! Every top-level sub-directory of the input directory is one book. Each HTML
//! file beneath it is one numbered page of a French choose-your-own-adventure
//! book scraped from an over-blog site. Nothing is fetched: page numbers and
//! branch targets are derived from the local HTML only.
//!
//! The one thing this does differently from the pipeline it replaces: the
//! document is read rather than rewritten. Marking a link and splitting a line
//! break used to be done by editing the parsed tree in place; here the tree is
//! walked once and the markers are written into the text as it is collected,
//! which comes to the same text and leaves the document alone.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ego_tree::iter::Edge;
use indexmap::IndexMap;
use regex::Regex;
use scraper::{ElementRef, Html, Node, Selector};
use unicode_normalization::UnicodeNormalization;

/// The host a link must be on to be a page of the book.
pub const PAGE_HOST_MARKER: &str = "lesitedontvousetesleheros";
/// The blocks a page's narrative is made of.
pub const SEGMENT_TAGS: [&str; 3] = ["p", "li", "pre"];
/// Where a `<br>` stood, so one block can be split into its lines.
pub const LINE_BREAK_MARKER: char = '\u{1f}';
/// Where a link stood, so its resolved number can be written in afterwards.
pub const LINK_MARKER: char = '\u{0}';
/// A slug must be at least this long before it is trusted to name one page.
pub const MIN_SLUG_KEY_LENGTH: usize = 20;

/// One outgoing link found in a page, before and after resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageLink {
    pub path: String,
    pub base: Option<String>,
    pub slug: Option<String>,
    pub resolved: Option<String>,
}

/// One page extracted from a single HTML file.
#[derive(Clone, Debug)]
pub struct ParsedPage {
    pub page_id: String,
    pub text: Vec<String>,
    pub links: Vec<PageLink>,
    pub file_path: String,
    pub url_path: Option<String>,
}

impl ParsedPage {
    /// Resolved branch targets, in document order.
    pub fn numbers(&self) -> Vec<String> {
        self.links
            .iter()
            .filter_map(|link| link.resolved.clone())
            .collect()
    }
}

/// Summary of one book's parsing run.
#[derive(Clone, Debug, Default)]
pub struct BookReport {
    pub files_scanned: usize,
    pub pages_parsed: usize,
    pub failures: Vec<(String, String)>,
    pub pages_without_numbers: Vec<String>,
    pub non_numeric_page_ids: Vec<String>,
    pub unresolved_slug_links: Vec<(String, String)>,
    pub dangling_targets: IndexMap<String, Vec<String>>,
    pub url_remaps: Vec<(String, String, String)>,
    pub variant_remaps: Vec<(String, String, String)>,
    pub unreachable_variants: Vec<String>,
}

/// Why one file could not be read as a page.
#[derive(Debug, thiserror::Error)]
pub enum PageError {
    #[error("ValueError: no page heading or title found")]
    NoHeading,
    #[error("ValueError: no div.ob-text block found")]
    NoText,
    #[error("OSError: {0}")]
    Unreadable(#[from] std::io::Error),
}

/// Collapse whitespace and drop soft hyphens from an extracted string.
pub fn normalize_text(text: &str) -> String {
    let cleaned = text.replace('\u{a0}', " ").replace('\u{ad}', "");
    cleaned.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Reduce text to lowercase ASCII alphanumerics for slug matching.
pub fn slug_key(text: &str) -> String {
    text.to_lowercase()
        .replace('œ', "oe")
        .replace('æ', "ae")
        .nfkd()
        .filter(|character| character.is_ascii() && character.is_alphanumeric())
        .collect()
}

/// Return the decoded path of a page URL, without trailing slash.
pub fn url_path(url: &str) -> String {
    let path = url::Url::parse(url).map_or_else(
        |_| url.split('?').next().unwrap_or(url).to_string(),
        |parsed| parsed.path().to_string(),
    );
    percent_encoding::percent_decode_str(&path)
        .decode_utf8_lossy()
        .trim_end_matches('/')
        .to_string()
}

/// Interpret a link target as a page reference.
///
/// # Returns
///
/// A link carrying the URL path plus either the page number found in it (the
/// over-blog duplicate suffix is dropped) or a text slug, or nothing for links
/// that do not point at the gamebook site.
pub fn parse_href(href: &str) -> Option<PageLink> {
    let parsed = url::Url::parse(href).ok()?;
    if !parsed.host_str()?.contains(PAGE_HOST_MARKER) {
        return None;
    }
    let path = url_path(href);
    let last_segment = path.rsplit('/').next().unwrap_or("");
    if last_segment.is_empty() {
        return None;
    }
    if let Some(number) = numeric_href(last_segment) {
        return Some(PageLink {
            path,
            base: Some(number),
            slug: None,
            resolved: None,
        });
    }
    let slug = last_segment
        .strip_suffix(".html")
        .unwrap_or(last_segment)
        .to_string();
    Some(PageLink {
        path,
        base: None,
        slug: Some(slug),
        resolved: None,
    })
}

/// The page number a numeric link carries, its duplicate suffix dropped.
fn numeric_href(segment: &str) -> Option<String> {
    static PATTERN: &str = r"^(\d+)(-\d+)?(?:\.html)?$";
    let matcher = Regex::new(PATTERN).expect("the pattern is written here");
    matcher
        .captures(segment)
        .map(|found| found[1].to_string())
}

/// Read the page's own URL path from its `og:url` meta tag, if present.
pub fn extract_url_path(document: &Html) -> Option<String> {
    let selector = Selector::parse(r#"meta[property="og:url"]"#).ok()?;
    let content = document
        .select(&selector)
        .next()?
        .value()
        .attr("content")
        .filter(|content| !content.is_empty())?;
    Some(url_path(content))
}

/// Read the page identifier from the `h2` heading, falling back to the title.
///
/// # Errors
///
/// If neither heading nor title provides an identifier.
pub fn extract_page_id(document: &Html) -> Result<String, PageError> {
    let heading = Selector::parse("h2").expect("the selector is written here");
    let mut page_id = document
        .select(&heading)
        .next()
        .map(|found| normalize_text(&found.text().collect::<String>()))
        .unwrap_or_default();
    if page_id.is_empty() {
        let title = Selector::parse("title").expect("the selector is written here");
        page_id = document
            .select(&title)
            .next()
            .map(|found| {
                let whole = found.text().collect::<String>();
                normalize_text(whole.split(" - ").next().unwrap_or(&whole))
            })
            .unwrap_or_default();
    }
    if page_id.is_empty() {
        return Err(PageError::NoHeading);
    }
    Ok(page_id)
}

/// Where the links of one text block stand, by the node they hang off.
type LinkPositions = HashMap<ego_tree::NodeId, usize>;

/// Register the page links of a text block, in document order.
///
/// # Returns
///
/// Where each link stands, so the marker can be written as the text is walked.
fn collect_links(text_div: ElementRef<'_>, links: &mut Vec<PageLink>) -> LinkPositions {
    let mut positions = LinkPositions::new();
    for edge in text_div.traverse() {
        let Edge::Open(node) = edge else { continue };
        let Some(element) = node.value().as_element() else {
            continue;
        };
        if element.name() != "a" {
            continue;
        }
        let Some(href) = element.attr("href") else {
            continue;
        };
        let Some(link) = parse_href(href) else {
            continue;
        };
        positions.insert(node.id(), links.len());
        links.push(link);
    }
    positions
}

/// The text of one node, with a marker where each link and line break stood.
fn text_with_markers(
    node: ego_tree::NodeRef<'_, Node>,
    positions: &LinkPositions,
    mark: bool,
) -> String {
    let mut written = String::new();
    for edge in node.traverse() {
        match edge {
            Edge::Open(node) => match node.value() {
                Node::Text(text) => written.push_str(text),
                Node::Element(element) if element.name() == "br" => {
                    written.push(LINE_BREAK_MARKER);
                }
                _ => {}
            },
            Edge::Close(node) => {
                if !mark {
                    continue;
                }
                if let Some(index) = positions.get(&node.id()) {
                    written.push(LINK_MARKER);
                    written.push_str(&index.to_string());
                    written.push(LINK_MARKER);
                }
            }
        }
    }
    written
}

/// Split one block element into normalized text segments on line breaks.
fn block_segments(written: &str) -> Vec<String> {
    written
        .split(LINE_BREAK_MARKER)
        .map(normalize_text)
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// Return the narrative segments of a text block, one per paragraph.
fn extract_segments(
    text_div: ElementRef<'_>,
    positions: &LinkPositions,
    mark: bool,
) -> Vec<String> {
    let mut segments = Vec::new();
    for edge in text_div.traverse() {
        let Edge::Open(node) = edge else { continue };
        let Some(block) = ElementRef::wrap(node) else {
            continue;
        };
        if block.id() == text_div.id() || !SEGMENT_TAGS.contains(&block.value().name()) {
            continue;
        }
        if holds_another_block(block) {
            continue;
        }
        segments.extend(block_segments(&text_with_markers(*block, positions, mark)));
    }
    if segments.is_empty() {
        segments.extend(block_segments(&text_with_markers(
            *text_div, positions, mark,
        )));
    }
    segments
}

/// Whether a block holds a block of its own, which is what makes it a wrapper.
fn holds_another_block(block: ElementRef<'_>) -> bool {
    block.traverse().any(|edge| match edge {
        Edge::Open(node) => ElementRef::wrap(node).is_some_and(|inner| {
            inner.id() != block.id() && SEGMENT_TAGS.contains(&inner.value().name())
        }),
        Edge::Close(_) => false,
    })
}

/// Parse one HTML page file.
///
/// # Arguments
///
/// * `path` - Location of the HTML file.
/// * `file_path` - Relative path recorded in the output.
/// * `mark_links` - Whether to leave inline placeholders for link targets.
///
/// # Errors
///
/// If the page lacks an identifier or a text block.
pub fn parse_file(path: &Path, file_path: &str, mark_links: bool) -> Result<ParsedPage, PageError> {
    let source = std::fs::read_to_string(path)?;
    let document = Html::parse_document(&source);
    let page_id = extract_page_id(&document)?;
    let text_block = Selector::parse("div.ob-text").expect("the selector is written here");
    let text_divs: Vec<ElementRef<'_>> = document.select(&text_block).collect();
    if text_divs.is_empty() {
        return Err(PageError::NoText);
    }
    let mut links: Vec<PageLink> = Vec::new();
    let mut segments: Vec<String> = Vec::new();
    for text_div in text_divs {
        let positions = collect_links(text_div, &mut links);
        segments.extend(extract_segments(text_div, &positions, mark_links));
    }
    Ok(ParsedPage {
        page_id,
        text: segments,
        links,
        file_path: file_path.to_string(),
        url_path: extract_url_path(&document),
    })
}

/// Every page of one book, in the order they were read.
pub type Pages = IndexMap<String, ParsedPage>;

/// Resolve links whose path equals a page's own URL path, else use the number.
///
/// The site occasionally renumbered a page after publishing it, so the number in
/// a URL is not always the number of the page it leads to; an exact URL match
/// takes precedence and such cases are reported.
pub fn resolve_by_url(pages: &mut Pages, report: &mut BookReport) {
    let by_path: HashMap<String, String> = pages
        .iter()
        .filter_map(|(page_id, page)| {
            page.url_path
                .clone()
                .map(|path| (path, page_id.clone()))
        })
        .collect();
    let page_ids: Vec<String> = pages.keys().cloned().collect();
    for page_id in page_ids {
        let own_id = pages[&page_id].page_id.clone();
        let links = &mut pages[&page_id].links;
        for link in links.iter_mut() {
            link.resolved = by_path.get(&link.path).cloned().or_else(|| link.base.clone());
            if let Some(base) = &link.base {
                if link.resolved.as_ref() != Some(base) {
                    report.url_remaps.push((
                        own_id.clone(),
                        base.clone(),
                        link.resolved.clone().unwrap_or_default(),
                    ));
                }
            }
        }
    }
}

/// Resolve remaining text-slug links by matching them against page openings.
pub fn resolve_slug_links(pages: &mut Pages, report: &mut BookReport) {
    let index: Vec<(String, String)> = pages
        .iter()
        .map(|(page_id, page)| (page_id.clone(), slug_key(&page.text.join(" "))))
        .collect();
    let page_ids: Vec<String> = pages.keys().cloned().collect();
    for page_id in page_ids {
        let own_id = pages[&page_id].page_id.clone();
        let links = &mut pages[&page_id].links;
        for link in links.iter_mut() {
            let Some(slug) = link.slug.clone() else {
                continue;
            };
            if link.resolved.is_some() {
                continue;
            }
            let key = slug_key(&slug.replace('-', " "));
            let candidates: Vec<&String> = index
                .iter()
                .filter(|(_, page_key)| page_key.starts_with(&key))
                .map(|(candidate, _)| candidate)
                .collect();
            if key.len() >= MIN_SLUG_KEY_LENGTH && candidates.len() == 1 {
                link.resolved = Some(candidates[0].clone());
            } else {
                report
                    .unresolved_slug_links
                    .push((own_id.clone(), slug.clone()));
            }
        }
    }
}

/// Group page ids sharing a number, such as `32` and `32'`, by that number.
pub fn variant_groups(pages: &Pages) -> IndexMap<String, Vec<String>> {
    let matcher = Regex::new(r"^(\d+)('*)$").expect("the pattern is written here");
    let mut groups: IndexMap<String, Vec<String>> = IndexMap::new();
    for page_id in pages.keys() {
        if let Some(found) = matcher.captures(page_id) {
            groups
                .entry(found[1].to_string())
                .or_default()
                .push(page_id.clone());
        }
    }
    groups.retain(|_, ids| ids.len() > 1);
    groups
}

/// Send links to the right variant of a number published several times.
///
/// Each variant (`32`, `32'`, ...) is a detour that ends with a single "return"
/// link to the page that led to it, so that page's link to the shared number is
/// pointed at this variant even when the site's own link disagrees.
pub fn remap_variant_pages(pages: &mut Pages, report: &mut BookReport) {
    for (base, variant_ids) in variant_groups(pages) {
        for (page_id, origin_id) in unambiguous_origins(pages, &variant_ids) {
            let own_id = pages[&origin_id].page_id.clone();
            let links = &mut pages[&origin_id].links;
            for link in links.iter_mut() {
                if link.base.as_ref() == Some(&base) && link.resolved.as_ref() != Some(&page_id) {
                    report.variant_remaps.push((
                        own_id.clone(),
                        link.resolved.clone().unwrap_or_else(|| "None".to_string()),
                        page_id.clone(),
                    ));
                    link.resolved = Some(page_id.clone());
                }
            }
        }
        let reachable: HashSet<String> = pages
            .values()
            .flat_map(|page| page.links.iter().filter_map(|link| link.resolved.clone()))
            .collect();
        report.unreachable_variants.extend(
            variant_ids
                .into_iter()
                .filter(|variant| !reachable.contains(variant)),
        );
    }
}

/// Map each variant to the page it returns to, when that page is claimed once.
///
/// # Returns
///
/// Variants having a single return target that no sibling variant shares, with
/// the return page's id as value.
pub fn unambiguous_origins(pages: &Pages, variant_ids: &[String]) -> IndexMap<String, String> {
    let mut origins: IndexMap<String, String> = IndexMap::new();
    for page_id in variant_ids {
        let targets: HashSet<String> = pages[page_id]
            .links
            .iter()
            .filter_map(|link| link.resolved.clone())
            .collect();
        if targets.len() == 1 {
            let only = targets.into_iter().next().expect("just counted one");
            if pages.contains_key(&only) {
                origins.insert(page_id.clone(), only);
            }
        }
    }
    let claims: Vec<String> = origins.values().cloned().collect();
    origins.retain(|_, origin| claims.iter().filter(|claim| *claim == origin).count() == 1);
    origins
}

/// Replace link placeholders with the resolved target numbers.
pub fn inline_numbers(pages: &mut Pages) {
    let pattern = Regex::new(&format!(
        "{}(\\d+){}",
        regex::escape(&LINK_MARKER.to_string()),
        regex::escape(&LINK_MARKER.to_string())
    ))
    .expect("the pattern is written here");
    for page in pages.values_mut() {
        let resolved: Vec<Option<String>> =
            page.links.iter().map(|link| link.resolved.clone()).collect();
        page.text = page
            .text
            .iter()
            .map(|segment| {
                let written = pattern.replace_all(segment, |found: &regex::Captures<'_>| {
                    let index: usize = found[1].parse().unwrap_or(usize::MAX);
                    match resolved.get(index).and_then(Clone::clone) {
                        Some(number) => format!(" {number}"),
                        None => String::new(),
                    }
                });
                normalize_text(&written)
            })
            .collect();
    }
}

/// Fill the parts of the report that need the whole book.
pub fn finalize_report(pages: &Pages, report: &mut BookReport) {
    report.pages_parsed = pages.len();
    for (page_id, page) in pages {
        if page.numbers().is_empty() {
            report.pages_without_numbers.push(page_id.clone());
        }
        if !page_id.chars().all(|character| character.is_ascii_digit()) || page_id.is_empty() {
            report.non_numeric_page_ids.push(page_id.clone());
        }
        let mut missing: Vec<String> = page
            .numbers()
            .into_iter()
            .filter(|number| !pages.contains_key(number))
            .collect();
        missing.sort_by_key(|number| sort_key(number));
        missing.dedup();
        if !missing.is_empty() {
            report.dangling_targets.insert(page_id.clone(), missing);
        }
    }
}

/// Order numeric pages first, primed variants after their base number.
pub fn sort_key(page_id: &str) -> (u8, u64, String) {
    let matcher = Regex::new(r"^(\d+)('*)$").expect("the pattern is written here");
    match matcher.captures(page_id) {
        Some(found) => (
            0,
            found[1].parse().unwrap_or(0),
            found[2].to_string(),
        ),
        None => (1, 0, page_id.to_string()),
    }
}

/// Parse every HTML file beneath one book directory.
///
/// # Arguments
///
/// * `book_dir` - Top-level directory holding the book's pages.
/// * `base_dir` - Directory against which recorded file paths are relative.
/// * `mark_links` - Whether resolved target numbers should be inlined in text.
///
/// # Returns
///
/// The pages keyed by page identifier, and the run report.
pub fn parse_book(book_dir: &Path, base_dir: &Path, mark_links: bool) -> (Pages, BookReport) {
    let mut report = BookReport::default();
    let mut pages: Pages = IndexMap::new();
    for path in html_files(book_dir) {
        report.files_scanned += 1;
        let file_path = path
            .strip_prefix(base_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        match parse_file(&path, &file_path, mark_links) {
            Err(trouble) => {
                report.failures.push((file_path, trouble.to_string()));
            }
            Ok(page) if pages.contains_key(&page.page_id) => {
                report
                    .failures
                    .push((file_path, format!("duplicate page id {}", page.page_id)));
            }
            Ok(page) => {
                pages.insert(page.page_id.clone(), page);
            }
        }
    }
    resolve_by_url(&mut pages, &mut report);
    resolve_slug_links(&mut pages, &mut report);
    remap_variant_pages(&mut pages, &mut report);
    if mark_links {
        inline_numbers(&mut pages);
    }
    finalize_report(&pages, &mut report);
    (pages, report)
}

/// Every HTML file beneath a directory, in the order a sorted walk gives them.
fn html_files(book_dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![book_dir.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|kind| kind == "html") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Write the book's pages as a JSON object keyed by page identifier.
///
/// # Errors
///
/// If the output directory cannot be made, or the file cannot be written.
pub fn write_book(pages: &Pages, output_path: &Path) -> std::io::Result<()> {
    let mut ordered: Vec<(&String, &ParsedPage)> = pages.iter().collect();
    ordered.sort_by_key(|(page_id, _)| sort_key(page_id));
    let mut payload = serde_json::Map::new();
    for (page_id, page) in ordered {
        payload.insert(
            page_id.clone(),
            serde_json::json!({
                "text": page.text,
                "numbers": page.numbers(),
                "file_path": page.file_path,
            }),
        );
    }
    if let Some(directory) = output_path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    let written = serde_json::to_string_pretty(&serde_json::Value::Object(payload))?;
    std::fs::write(output_path, format!("{written}\n"))
}

/// Print the human-readable summary of one book.
pub fn print_report(book_name: &str, report: &BookReport) {
    println!("== {book_name}");
    println!(
        "   files scanned: {}, pages parsed: {}, failed: {}",
        report.files_scanned,
        report.pages_parsed,
        report.failures.len()
    );
    for (file_path, reason) in &report.failures {
        println!("   FAILED {file_path}: {reason}");
    }
    if !report.non_numeric_page_ids.is_empty() {
        println!(
            "   non-numeric page ids: {}",
            report.non_numeric_page_ids.join(", ")
        );
    }
    let mut without_numbers = report.pages_without_numbers.clone();
    without_numbers.sort_by_key(|page_id| sort_key(page_id));
    println!(
        "   pages without numbers ({}): {}",
        without_numbers.len(),
        without_numbers.join(", ")
    );
    for (origin, base, target) in &report.url_remaps {
        println!("   url remap: page {origin} link numbered {base} leads to page {target}");
    }
    for (origin, before, target) in &report.variant_remaps {
        println!("   variant remap: page {origin} link {before} -> {target}");
    }
    for page_id in &report.unreachable_variants {
        println!("   WARNING variant page {page_id} is not reachable from any link");
    }
    for (page_id, slug) in &report.unresolved_slug_links {
        let head: String = slug.chars().take(70).collect();
        println!("   WARNING unresolved slug link on page {page_id}: {head}");
    }
    if !report.dangling_targets.is_empty() {
        let mut targets: Vec<String> = report
            .dangling_targets
            .values()
            .flatten()
            .cloned()
            .collect();
        targets.sort_by_key(|target| sort_key(target));
        targets.dedup();
        println!(
            "   targets without a page in this book ({}): {}",
            targets.len(),
            targets.join(", ")
        );
    }
}
