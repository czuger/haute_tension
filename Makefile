# Haute Tension — running the test suite and the development server.
#
# The database is SQLite, so there is nothing to bring up: the suite runs every
# test against an in-memory database of its own, and the application opens a
# file under DATABASE_DIR (see .env.example). `make test` is how the repository
# is checked before pushing.

# Arguments passed to cargo test: `make test ARGS="history -- --nocapture"`.
ARGS ?=

.PHONY: help test coverage lint serve import

help:
	@echo "make test     — runs the whole suite"
	@echo "make coverage — the whole suite, with an HTML report in target/llvm-cov/html/"
	@echo "make lint     — cargo clippy over the library, the binaries and the tests"
	@echo "make serve    — runs the development server on port 5001"
	@echo "make import   — runs the offline import pipeline over work/raw_data"
	@echo ""
	@echo "ARGS passes arguments to cargo test:  make test ARGS='history'"
	@echo "MIN is the line coverage the suite must reach:  make coverage MIN=90"

test:
	cargo test $(ARGS)

lint:
	cargo clippy --all-targets

# --- Coverage ---------------------------------------------------------------------------------
#
# Needs cargo-llvm-cov and the toolchain's LLVM binaries, installed once:
#
#     brew install cargo-llvm-cov            # or: cargo install cargo-llvm-cov --locked
#     rustup component add llvm-tools-preview
#
# The gate is on lines because cargo-llvm-cov measures branches on a nightly toolchain only; the
# terminal table prints regions, the finer count, beside them. The tests are run once and reported
# twice, because the two reports are not read for the same thing: the terminal one names the lines
# nobody reached, and the HTML one — target/llvm-cov/html/index.html, not versioned — colours them
# in the source, which is what one wants when the missing lines are a branch rather than a block.
MIN ?= 95

coverage:
	cargo llvm-cov --no-report
	cargo llvm-cov report --html
	@echo "HTML report: target/llvm-cov/html/index.html"
	cargo llvm-cov report --show-missing-lines --fail-under-lines $(MIN)

# --- Running the application ------------------------------------------------------------------
#
# Reads .env: APP_ENV picks the file under DATABASE_DIR, so a dev run never opens the prod one.
# The book itself is read off disk, so this serves pages with or without a database — only the
# reading history, the play-throughs and the flagged pages need one.

serve:
	cargo run --bin serve

# --- The offline import pipeline --------------------------------------------------------------
#
# Nothing the application calls: it reads the HTML under work/raw_data and writes one JSON file
# per book into work/parsed_data. Run by hand when the source pages change.

import:
	cargo run --bin parse_pages
