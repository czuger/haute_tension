# Haute Tension — running the test suite and the development server.
#
# The database is SQLite, so there is nothing to bring up: the suite runs every
# test against an in-memory database of its own, and the application opens a
# file under DATABASE_DIR (see .env.example). `make test` is how the repository
# is checked before pushing.

# Arguments passed to pytest: `make test ARGS="-k history -v"`.
ARGS ?=

.PHONY: help test coverage rust-coverage serve

help:
	@echo "make test          — runs the whole suite"
	@echo "make coverage      — the whole suite, with an HTML report in htmlcov/"
	@echo "make rust-coverage — the Rust suite, with an HTML report in rust/target/llvm-cov/html/"
	@echo "make serve         — runs the development server on port 5001"
	@echo ""
	@echo "ARGS passes arguments to pytest:  make test ARGS='-k history -v'"
	@echo "MIN is the line coverage the Rust suite must reach:  make rust-coverage MIN=90"

test:
	python3 -m pytest $(ARGS)

# Two reports at once, because they are not read for the same thing: the terminal one names the
# lines nobody reached, and the HTML one — htmlcov/index.html, not versioned — colours them in the
# source, which is what one wants when the missing lines are a branch rather than a block.
coverage:
	python3 -m pytest --cov-report=html $(ARGS)
	@echo "HTML report: htmlcov/index.html"

# --- Coverage for the Rust port ---------------------------------------------------------------
#
# The counterpart of `make coverage` for the suite in rust/. It needs cargo-llvm-cov and the
# toolchain's LLVM binaries, installed once:
#
#     brew install cargo-llvm-cov            # or: cargo install cargo-llvm-cov --locked
#     rustup component add llvm-tools-preview
#
# The line coverage the Rust suite must reach, the counterpart of --cov-fail-under in
# pyproject.toml: `make rust-coverage MIN=90`. The gate is on lines because cargo-llvm-cov measures
# branches on a nightly toolchain only; the terminal table prints regions, the finer count, beside
# them. The tests are run once and reported twice, for the same reason as above.
MIN ?= 95

rust-coverage:
	cd rust && cargo llvm-cov --no-report
	cd rust && cargo llvm-cov report --html
	@echo "HTML report: rust/target/llvm-cov/html/index.html"
	cd rust && cargo llvm-cov report --show-missing-lines --fail-under-lines $(MIN)

# --- Running the application ------------------------------------------------------------------
#
# Reads .env: APP_ENV picks the file under DATABASE_DIR, so a dev run never opens the prod one.
# The book itself is read off disk, so this serves pages with or without a database — only the
# reading history, the play-throughs and the flagged pages need one.

serve:
	cd haute_tension && PYTHONPATH=.. python3 app.py
