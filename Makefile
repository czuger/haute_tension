# Haute Tension — running the test suite.
#
# `make test` brings up a test MongoDB in a container, waits for it to answer, then runs pytest
# pointing it at it. That is the way to check the repository before pushing: the suite also runs
# without any of this (`python -m pytest` falls back to an in-memory mongomock server), but
# mongomock is a reimplementation, and a driver behaviour it does not share is a bug only a real
# server shows.
#
# The container is separate from the application's: it listens on another port (27019) and works
# in its own database, `haute_tension_test`, which the suite empties before every test. Both
# separations matter, and the second is the one that saves you: the port keeps the two servers
# apart, but nothing stops a MONGO_URI from pointing the application at this very container, and
# only the distinct database name then keeps `make test` from dropping a reading history. It stays
# up between two `make test`, which makes series fast; `make mongo-stop` removes it.
#
# Port 27019 is deliberate: 27017 is the default a local application container takes, and 27018 is
# already the neighbouring tenebrae repository's test server. Override it if it clashes anyway:
#
#     make test PORT=27023

CONTAINER ?= haute-tension-mongo-test
IMAGE     ?= mongo:7
PORT      ?= 27019
URI       := mongodb://localhost:$(PORT)

# Arguments passed to pytest: `make test ARGS="-k history -v"`.
ARGS ?=

.PHONY: help test test-fast coverage mongo mongo-stop serve

help:
	@echo "make test       — brings up MongoDB and runs the whole suite against it"
	@echo "make test-fast  — the same suite on the in-memory server, no container"
	@echo "make coverage   — the whole suite, with an HTML report in htmlcov/"
	@echo "make mongo      — brings up the test MongoDB and waits for it"
	@echo "make mongo-stop — removes the container"
	@echo "make serve      — runs the development server on port 5001"
	@echo ""
	@echo "ARGS passes arguments to pytest:  make test ARGS='-k history -v'"

test: mongo
	MONGO_URI_TEST=$(URI) python3 -m pytest $(ARGS)

# The pass one wants while writing a test: no container, no wait, the whole suite in well under a
# second. What it cannot catch is anything mongomock does differently from a real driver.
test-fast:
	python3 -m pytest $(ARGS)

# Two reports at once, because they are not read for the same thing: the terminal one names the
# lines nobody reached, and the HTML one — htmlcov/index.html, not versioned — colours them in the
# source, which is what one wants when the missing lines are a branch rather than a block.
coverage: mongo
	MONGO_URI_TEST=$(URI) python3 -m pytest --cov-report=html $(ARGS)
	@echo "HTML report: htmlcov/index.html"

# Brings the container up if it is not already there, then waits for the database to really answer:
# a container that is "Up" is not yet a server accepting connections, and pytest would then fail on
# a connection refused rather than on anything it was meant to check.
mongo:
	@if ! command -v docker >/dev/null 2>&1; then \
		echo "Docker is missing: 'make test' needs the MongoDB it brings up."; \
		echo "Run 'make test-fast' to use the in-memory server instead."; \
		exit 1; \
	fi
	@if [ -z "$$(docker ps -q -f name=^/$(CONTAINER)$$)" ]; then \
		if [ -n "$$(docker ps -aq -f name=^/$(CONTAINER)$$)" ]; then \
			echo "Restarting container $(CONTAINER)..."; \
			docker start $(CONTAINER) >/dev/null; \
		else \
			echo "Creating container $(CONTAINER) on port $(PORT)..."; \
			docker run -d --name $(CONTAINER) -p $(PORT):27017 $(IMAGE) >/dev/null; \
		fi; \
	fi
	@printf "Waiting for MongoDB on port $(PORT)"
	@for i in $$(seq 1 60); do \
		if docker exec $(CONTAINER) mongosh --quiet --eval "db.runCommand({ping:1})" \
			>/dev/null 2>&1; then \
			echo " — ready."; exit 0; \
		fi; \
		printf "."; sleep 1; \
	done; \
	echo " — no answer after 60 s."; exit 1

mongo-stop:
	@docker rm -f $(CONTAINER) >/dev/null 2>&1 && echo "Container $(CONTAINER) removed." \
		|| echo "No container $(CONTAINER)."

# --- Running the application ------------------------------------------------------------------
#
# Reads .env, not the test container above: the application's database is its own, and `make test`
# must never be able to write to it. The book itself is read off disk, so this serves pages with
# or without a reachable server — only the reading history needs one.

serve:
	cd haute_tension && PYTHONPATH=.. python3 app.py
