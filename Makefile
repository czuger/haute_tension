# Haute Tension — running the test suite and the development server.
#
# The database is SQLite, so there is nothing to bring up: the suite runs every
# test against an in-memory database of its own, and the application opens a
# file under DATABASE_DIR (see .env.example). `make test` is how the repository
# is checked before pushing.

# Arguments passed to pytest: `make test ARGS="-k history -v"`.
ARGS ?=

.PHONY: help test coverage serve

help:
	@echo "make test       — runs the whole suite"
	@echo "make coverage   — the whole suite, with an HTML report in htmlcov/"
	@echo "make serve      — runs the development server on port 5001"
	@echo ""
	@echo "ARGS passes arguments to pytest:  make test ARGS='-k history -v'"

test:
	python3 -m pytest $(ARGS)

# Two reports at once, because they are not read for the same thing: the terminal one names the
# lines nobody reached, and the HTML one — htmlcov/index.html, not versioned — colours them in the
# source, which is what one wants when the missing lines are a branch rather than a block.
coverage:
	python3 -m pytest --cov-report=html $(ARGS)
	@echo "HTML report: htmlcov/index.html"

# --- Running the application ------------------------------------------------------------------
#
# Reads .env: APP_ENV picks the file under DATABASE_DIR, so a dev run never opens the prod one.
# The book itself is read off disk, so this serves pages with or without a database — only the
# reading history, the play-throughs and the flagged pages need one.

serve:
	cd haute_tension && PYTHONPATH=.. python3 app.py
