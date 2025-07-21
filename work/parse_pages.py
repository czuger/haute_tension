"""Parse local gamebook HTML pages into one JSON file per book.

Every top-level sub-directory of the input directory is one book. Each HTML
file beneath it is one numbered page of a French choose-your-own-adventure
book scraped from an over-blog site. Nothing is fetched: page numbers and
branch targets are derived from the local HTML only.
"""

from __future__ import annotations

import argparse
import json
import logging
import re
import unicodedata
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional
from urllib.parse import unquote, urlparse

from bs4 import BeautifulSoup, NavigableString, Tag

LOGGER = logging.getLogger("parse_pages")
WORK_DIR = Path(__file__).resolve().parent
PAGE_HOST_MARKER = "lesitedontvousetesleheros"
SEGMENT_TAGS = ("p", "li", "pre")
LINE_BREAK_MARKER = "\x1f"
LINK_MARKER = "\x00"
NUMERIC_HREF = re.compile(r"(\d+)(-\d+)?(?:\.html)?")
PAGE_VARIANT_ID = re.compile(r"(\d+)('*)")
MIN_SLUG_KEY_LENGTH = 20


@dataclass
class PageLink:
    """One outgoing link found in a page, before and after resolution."""

    path: str
    base: Optional[str] = None
    slug: Optional[str] = None
    resolved: Optional[str] = None


@dataclass
class ParsedPage:
    """One page extracted from a single HTML file."""

    page_id: str
    text: list[str]
    links: list[PageLink]
    file_path: str
    url_path: Optional[str]

    @property
    def numbers(self) -> list[str]:
        """Resolved branch targets, in document order."""
        return [link.resolved for link in self.links if link.resolved]


@dataclass
class BookReport:
    """Summary of one book's parsing run."""

    files_scanned: int = 0
    pages_parsed: int = 0
    failures: list[tuple[str, str]] = field(default_factory=list)
    pages_without_numbers: list[str] = field(default_factory=list)
    non_numeric_page_ids: list[str] = field(default_factory=list)
    unresolved_slug_links: list[tuple[str, str]] = field(default_factory=list)
    dangling_targets: dict[str, list[str]] = field(default_factory=dict)
    url_remaps: list[tuple[str, str, str]] = field(default_factory=list)
    variant_remaps: list[tuple[str, str, str]] = field(default_factory=list)
    unreachable_variants: list[str] = field(default_factory=list)


def normalize_text(text: str) -> str:
    """Collapse whitespace and drop soft hyphens from an extracted string."""
    cleaned = text.replace("\xa0", " ").replace("\xad", "")
    return re.sub(r"\s+", " ", cleaned).strip()


def slug_key(text: str) -> str:
    """Reduce text to lowercase ASCII alphanumerics for slug matching."""
    replaced = text.lower().replace("œ", "oe").replace("æ", "ae")
    decomposed = unicodedata.normalize("NFKD", replaced)
    return "".join(c for c in decomposed if c.isascii() and c.isalnum())


def url_path(url: str) -> str:
    """Return the decoded path of a page URL, without trailing slash."""
    return unquote(urlparse(url).path).rstrip("/")


def parse_href(href: str) -> Optional[PageLink]:
    """Interpret a link target as a page reference.

    Args:
        href: Raw `href` attribute value.

    Returns:
        A link carrying the URL path plus either the page number found in it
        (the over-blog duplicate suffix is dropped) or a text slug, or None for
        links that do not point at the gamebook site.
    """
    if PAGE_HOST_MARKER not in urlparse(href).netloc:
        return None
    path = url_path(href)
    last_segment = path.rsplit("/", 1)[-1]
    if not last_segment:
        return None
    match = NUMERIC_HREF.fullmatch(last_segment)
    if match:
        return PageLink(path, base=match.group(1))
    slug = last_segment[:-5] if last_segment.endswith(".html") else last_segment
    return PageLink(path, slug=slug)


def extract_url_path(soup: BeautifulSoup) -> Optional[str]:
    """Read the page's own URL path from its `og:url` meta tag, if present."""
    meta = soup.find("meta", property="og:url")
    content = meta.get("content") if isinstance(meta, Tag) else None
    return url_path(content) if isinstance(content, str) and content else None


def extract_page_id(soup: BeautifulSoup) -> str:
    """Read the page identifier from the `h2` heading, falling back to the title.

    Raises:
        ValueError: If neither heading nor title provides an identifier.
    """
    heading = soup.find("h2")
    page_id = normalize_text(heading.get_text()) if heading else ""
    if not page_id and soup.title and soup.title.string:
        page_id = normalize_text(soup.title.string.split(" - ")[0])
    if not page_id:
        raise ValueError("no page heading or title found")
    return page_id


def collect_links(text_div: Tag, links: list[PageLink], mark: bool) -> None:
    """Register page links of a text block, optionally marking them in place.

    Args:
        text_div: The `div.ob-text` block being processed.
        links: Accumulator receiving the links in document order.
        mark: When true, append a placeholder after each link's text so the
            resolved target number can be inlined once resolution is done.
    """
    for anchor in text_div.find_all("a", href=True):
        link = parse_href(anchor["href"])
        if link is None:
            continue
        if mark:
            anchor.append(NavigableString(f"{LINK_MARKER}{len(links)}{LINK_MARKER}"))
        links.append(link)


def block_segments(block: Tag) -> list[str]:
    """Split one block element into normalized text segments on `<br>` breaks."""
    for line_break in block.find_all("br"):
        line_break.replace_with(NavigableString(LINE_BREAK_MARKER))
    pieces = block.get_text().split(LINE_BREAK_MARKER)
    return [segment for segment in map(normalize_text, pieces) if segment]


def extract_segments(text_div: Tag) -> list[str]:
    """Return the narrative segments of a text block, one per paragraph."""
    segments: list[str] = []
    for block in text_div.find_all(SEGMENT_TAGS):
        if block.find(SEGMENT_TAGS):
            continue
        segments.extend(block_segments(block))
    if not segments:
        segments.extend(block_segments(text_div))
    return segments


def parse_file(path: Path, file_path: str, mark_links: bool) -> ParsedPage:
    """Parse one HTML page file.

    Args:
        path: Location of the HTML file.
        file_path: Relative path recorded in the output.
        mark_links: Whether to leave inline placeholders for link targets.

    Raises:
        ValueError: If the page lacks an identifier or a text block.
    """
    soup = BeautifulSoup(path.read_text(encoding="utf-8"), "html.parser")
    page_id = extract_page_id(soup)
    text_divs = soup.find_all("div", class_="ob-text")
    if not text_divs:
        raise ValueError("no div.ob-text block found")
    links: list[PageLink] = []
    segments: list[str] = []
    for text_div in text_divs:
        collect_links(text_div, links, mark_links)
        segments.extend(extract_segments(text_div))
    return ParsedPage(page_id, segments, links, file_path, extract_url_path(soup))


def resolve_by_url(pages: dict[str, ParsedPage], report: BookReport) -> None:
    """Resolve links whose path equals a page's own URL path, else use the number.

    The site occasionally renumbered a page after publishing it, so the number
    in a URL is not always the number of the page it leads to; an exact URL
    match takes precedence and such cases are reported.
    """
    by_path = {page.url_path: page_id for page_id, page in pages.items() if page.url_path}
    for page in pages.values():
        for link in page.links:
            link.resolved = by_path.get(link.path, link.base)
            if link.base and link.resolved != link.base:
                report.url_remaps.append((page.page_id, link.base, link.resolved))


def resolve_slug_links(pages: dict[str, ParsedPage], report: BookReport) -> None:
    """Resolve remaining text-slug links by matching them against page openings."""
    index = [(page_id, slug_key(" ".join(page.text))) for page_id, page in pages.items()]
    for page in pages.values():
        for link in page.links:
            if link.slug is None or link.resolved is not None:
                continue
            key = slug_key(link.slug.replace("-", " "))
            candidates = [page_id for page_id, page_key in index if page_key.startswith(key)]
            if len(key) >= MIN_SLUG_KEY_LENGTH and len(candidates) == 1:
                link.resolved = candidates[0]
                LOGGER.debug("slug link resolved: %s -> %s", link.slug[:40], candidates[0])
            else:
                report.unresolved_slug_links.append((page.page_id, link.slug))
                LOGGER.warning(
                    "unresolved slug link on page %s (%d candidates): %s",
                    page.page_id, len(candidates), link.slug[:60],
                )


def variant_groups(pages: dict[str, ParsedPage]) -> dict[str, list[str]]:
    """Group page ids sharing a number, such as `32` and `32'`, by that number."""
    groups: dict[str, list[str]] = {}
    for page_id in pages:
        match = PAGE_VARIANT_ID.fullmatch(page_id)
        if match:
            groups.setdefault(match.group(1), []).append(page_id)
    return {base: ids for base, ids in groups.items() if len(ids) > 1}


def remap_variant_pages(pages: dict[str, ParsedPage], report: BookReport) -> None:
    """Send links to the right variant of a number published several times.

    Each variant (`32`, `32'`, ...) is a detour that ends with a single "return"
    link to the page that led to it, so that page's link to the shared number
    is pointed at this variant even when the site's own link disagrees.
    """
    for base, variant_ids in variant_groups(pages).items():
        for page_id, origin in unambiguous_origins(pages, variant_ids).items():
            for link in origin.links:
                if link.base == base and link.resolved != page_id:
                    report.variant_remaps.append((origin.page_id, str(link.resolved), page_id))
                    link.resolved = page_id
        reachable = {link.resolved for page in pages.values() for link in page.links}
        report.unreachable_variants.extend(v for v in variant_ids if v not in reachable)


def unambiguous_origins(
    pages: dict[str, ParsedPage], variant_ids: list[str]
) -> dict[str, ParsedPage]:
    """Map each variant to the page it returns to, when that page is claimed once.

    Args:
        pages: All pages of the book.
        variant_ids: Page ids sharing one number.

    Returns:
        Variants having a single return target that no sibling variant shares,
        with the return page itself as value.
    """
    origins: dict[str, str] = {}
    for page_id in variant_ids:
        targets = {link.resolved for link in pages[page_id].links if link.resolved}
        if len(targets) == 1 and next(iter(targets)) in pages:
            origins[page_id] = next(iter(targets))
    claims = list(origins.values())
    for origin in sorted({o for o in claims if claims.count(o) > 1}, key=sort_key):
        variants = ", ".join(v for v, o in origins.items() if o == origin)
        LOGGER.warning("variants %s all return to page %s; keeping site links", variants, origin)
    return {
        page_id: pages[origin] for page_id, origin in origins.items() if claims.count(origin) == 1
    }


def inline_numbers(pages: dict[str, ParsedPage]) -> None:
    """Replace link placeholders with the resolved target numbers."""
    pattern = re.compile(f"{LINK_MARKER}(\\d+){LINK_MARKER}")
    for page in pages.values():
        def substitute(match: re.Match[str]) -> str:
            resolved = page.links[int(match.group(1))].resolved
            return f" {resolved}" if resolved else ""
        page.text = [normalize_text(pattern.sub(substitute, segment)) for segment in page.text]


def finalize_report(pages: dict[str, ParsedPage], report: BookReport) -> None:
    """Fill the parts of the report that need the whole book."""
    report.pages_parsed = len(pages)
    for page_id, page in pages.items():
        if not page.numbers:
            report.pages_without_numbers.append(page_id)
        if not page_id.isdigit():
            report.non_numeric_page_ids.append(page_id)
        missing = sorted({n for n in page.numbers if n not in pages}, key=sort_key)
        if missing:
            report.dangling_targets[page_id] = missing


def sort_key(page_id: str) -> tuple[int, int, str]:
    """Order numeric pages first, primed variants after their base number."""
    match = re.fullmatch(r"(\d+)('*)", page_id)
    if match:
        return (0, int(match.group(1)), match.group(2))
    return (1, 0, page_id)


def parse_book(
    book_dir: Path, base_dir: Path, mark_links: bool
) -> tuple[dict[str, ParsedPage], BookReport]:
    """Parse every HTML file beneath one book directory.

    Args:
        book_dir: Top-level directory holding the book's pages.
        base_dir: Directory against which recorded file paths are relative.
        mark_links: Whether resolved target numbers should be inlined in text.

    Returns:
        The pages keyed by page identifier, and the run report.
    """
    report = BookReport()
    pages: dict[str, ParsedPage] = {}
    for path in sorted(book_dir.rglob("*.html")):
        report.files_scanned += 1
        file_path = path.relative_to(base_dir).as_posix()
        try:
            page = parse_file(path, file_path, mark_links)
        except Exception as error:  # noqa: BLE001 - one bad file must not stop the run
            report.failures.append((file_path, f"{type(error).__name__}: {error}"))
            LOGGER.warning("skipping %s: %s", file_path, error)
            continue
        if page.page_id in pages:
            report.failures.append((file_path, f"duplicate page id {page.page_id}"))
            LOGGER.warning("skipping %s: duplicate page id %s", file_path, page.page_id)
            continue
        pages[page.page_id] = page
    resolve_by_url(pages, report)
    resolve_slug_links(pages, report)
    remap_variant_pages(pages, report)
    if mark_links:
        inline_numbers(pages)
    finalize_report(pages, report)
    return pages, report


def write_book(pages: dict[str, ParsedPage], output_path: Path) -> None:
    """Write the book's pages as a JSON object keyed by page identifier."""
    payload = {
        page_id: {"text": page.text, "numbers": page.numbers, "file_path": page.file_path}
        for page_id, page in sorted(pages.items(), key=lambda item: sort_key(item[0]))
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, ensure_ascii=False, indent=2)
        handle.write("\n")


def print_report(book_name: str, report: BookReport) -> None:
    """Print the human-readable summary of one book."""
    print(f"== {book_name}")
    print(f"   files scanned: {report.files_scanned}, pages parsed: {report.pages_parsed}, "
          f"failed: {len(report.failures)}")
    for file_path, reason in report.failures:
        print(f"   FAILED {file_path}: {reason}")
    if report.non_numeric_page_ids:
        print(f"   non-numeric page ids: {', '.join(report.non_numeric_page_ids)}")
    print(f"   pages without numbers ({len(report.pages_without_numbers)}): "
          f"{', '.join(sorted(report.pages_without_numbers, key=sort_key))}")
    for origin, base, target in report.url_remaps:
        print(f"   url remap: page {origin} link numbered {base} leads to page {target}")
    for origin, before, target in report.variant_remaps:
        print(f"   variant remap: page {origin} link {before} -> {target}")
    for page_id in report.unreachable_variants:
        print(f"   WARNING variant page {page_id} is not reachable from any link")
    for page_id, slug in report.unresolved_slug_links:
        print(f"   WARNING unresolved slug link on page {page_id}: {slug[:70]}")
    if report.dangling_targets:
        targets = sorted({t for ts in report.dangling_targets.values() for t in ts}, key=sort_key)
        print(f"   targets without a page in this book ({len(targets)}): {', '.join(targets)}")


def build_parser() -> argparse.ArgumentParser:
    """Define the command-line interface."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("-i", "--input-dir", type=Path, default=WORK_DIR / "raw_data",
                        help="directory whose sub-directories are books (default: work/raw_data)")
    parser.add_argument("-o", "--output-dir", type=Path, default=WORK_DIR / "parsed_data",
                        help="directory receiving one JSON file per book (default: work/parsed_data)")
    parser.add_argument("-n", "--inline-numbers", action="store_true",
                        help="append each link's target number after its text, as '<text> <number>'")
    parser.add_argument("-v", "--verbose", action="store_true", help="log per-file details")
    return parser


def main() -> None:
    """Parse every book found in the input directory."""
    args = build_parser().parse_args()
    logging.basicConfig(level=logging.DEBUG if args.verbose else logging.WARNING,
                        format="%(levelname)s %(message)s")
    input_dir: Path = args.input_dir.resolve()
    book_dirs = sorted(path for path in input_dir.iterdir() if path.is_dir())
    for book_dir in book_dirs:
        pages, report = parse_book(book_dir, input_dir.parent, args.inline_numbers)
        write_book(pages, args.output_dir / f"{book_dir.name}.json")
        print_report(book_dir.name, report)


if __name__ == "__main__":
    main()
