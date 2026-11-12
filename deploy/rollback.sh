#!/usr/bin/env bash
# Back to the version before this one: make rollback
#
# The server's repository is the history of what was deployed, so this is one
# commit back in it, rebuilt and restarted. The version rolled away from is still
# in the history — `make deploy` puts the current sources back.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

echo "==> Rolling $SSH_HOST:$REMOTE_DIR back one version"
remote rollback
echo "==> Health"
remote health
echo
echo "Rolled back: $PUBLIC_URL"
