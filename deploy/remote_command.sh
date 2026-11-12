#!/usr/bin/env bash
# One command of deploy/remote.sh on the server: make checks / status / versions
#
# A thin front for the Makefile, so that the settings in config.sh are the only
# place the server is described.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

remote "${1:-checks}"
