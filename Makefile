# Haute Tension — the suite, the development server, and the deployment.
#
# The database is SQLite, so there is nothing to bring up: the suite runs every
# test against an in-memory database of its own, and the application opens a
# file under DATABASE_DIR (see .env.example). `make test` is how the repository
# is checked before pushing.
#
# Everything about the deployment lives in deploy/ — see deploy/config.sh for the
# server settings, all overridable from the environment:
#
#   make deploy SSH_HOST=ced@other-box

# Arguments passed to cargo test: `make test ARGS="history"`.
ARGS ?=

.PHONY: help test lint coverage serve import setup deploy nginx checks status logs versions rollback

SSH_HOST ?= ced@nuc150
REMOTE_DIR ?= /home/ced/rust

help:  ## Show this help
	@grep -E '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) | awk -F':.*?## ' '{printf "  %-10s %s\n", $$1, $$2}'
	@echo ""
	@echo "  ARGS passes arguments to cargo test:  make test ARGS='history'"
	@echo "  MIN is the line coverage to reach:    make coverage MIN=90"

# --- Developing -------------------------------------------------------------------------------

test:  ## Run the whole suite
	cargo test $(ARGS)

lint:  ## cargo clippy over the library, the binaries and the tests
	cargo clippy --all-targets

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

coverage:  ## The whole suite, with an HTML report in target/llvm-cov/html/
	cargo llvm-cov --no-report
	cargo llvm-cov report --html
	@echo "HTML report: target/llvm-cov/html/index.html"
	cargo llvm-cov report --show-missing-lines --fail-under-lines $(MIN)

# Reads .env: APP_ENV picks the file under DATABASE_DIR, so a dev run never opens the prod one.
# The book itself is read off disk, so this serves pages with or without a database — only the
# reading history, the play-throughs and the flagged pages need one.
serve:  ## Run the development server on port 5001
	cargo run --bin serve

# Nothing the application calls: it reads the HTML under work/raw_data and writes one JSON file
# per book into work/parsed_data. Run by hand when the source pages change.
import:  ## Run the offline import pipeline over work/raw_data
	cargo run --bin parse_pages

# --- Deploying --------------------------------------------------------------------------------

setup:  ## One-time server preparation (repository, toolchain, .env, nginx)
	@./deploy/setup_server.sh

deploy:  ## Ship the minimum, build it on the server, restart it, check it
	@./deploy/deploy.sh

nginx:  ## Re-install the nginx snippet and reload the proxy (no deploy)
	@./deploy/nginx.sh

rollback:  ## Rebuild and restart the version before this one
	@./deploy/rollback.sh

checks:  ## Everything that has to be true for the site to be up, one line each
	@./deploy/remote_command.sh checks

status:  ## What is running, which version, and whether it answers
	@./deploy/remote_command.sh status

versions:  ## The versions deployed, newest first
	@./deploy/remote_command.sh versions

logs:  ## Follow the application's log on the server (Ctrl-C to stop)
	@ssh -t $(SSH_HOST) "tail -f -n 100 $(REMOTE_DIR)/logs/general.log"
