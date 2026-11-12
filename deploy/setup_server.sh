#!/usr/bin/env bash
# ONE-TIME server preparation. Run from your laptop: make setup
#
# Creates the repository the deploy pushes into, installs the toolchain it is built
# with, writes the settings file if there is none, and puts the nginx snippet in
# place. It never touches $REMOTE_DIR/.env if it already exists — that file holds
# the session key, and rewriting it would start every reader on a new hero.
#
# Safe to run again: every step says what it found rather than redoing it.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

echo "==> Target: $SSH_HOST:$REMOTE_DIR"
ssh "$SSH_HOST" 'true' || { echo "error: cannot reach $SSH_HOST over ssh" >&2; exit 1; }

# 1. supervisord has to be there, and its include directory has to be one this user
#    can write: the program file is shipped, not the daemon's own configuration.
echo "==> Checking supervisord"
if ! ssh "$SSH_HOST" 'command -v supervisorctl >/dev/null'; then
  echo "error: supervisord is not installed on $SSH_HOST" >&2
  exit 1
fi
if ! ssh "$SSH_HOST" 'sudo -n true 2>/dev/null'; then
  echo "error: $RUN_USER needs passwordless sudo for supervisorctl on $SSH_HOST" >&2
  exit 1
fi
ssh "$SSH_HOST" "grep -q '$SUPERVISOR_CONF_DIR' /etc/supervisor/supervisord.conf" \
  || { echo "error: /etc/supervisor/supervisord.conf does not include $SUPERVISOR_CONF_DIR" >&2; exit 1; }
echo "    ok      $(ssh "$SSH_HOST" 'supervisord --version') including $SUPERVISOR_CONF_DIR"

# 2. The repository, the two directories the application writes, the socket
#    directory, the toolchain and the settings.
remote init

# 3. The nginx snippet, validated on the server before nginx is asked to take it.
echo "==> Installing the nginx snippet ($NGINX_CONF_NAME -> $NGINX_CONF_DIR)"
install_nginx_conf

cat <<EOF

Ready on $SSH_HOST:$REMOTE_DIR
  .env      the settings, session key included — server-owned, no deploy touches it
  data/     the SQLite file, made on the first request that needs it
  logs/     general.log, and what supervisord catches on the way up
  target/   the build, kept between deploys so a deploy is not a first build

The site will answer at $PUBLIC_URL, over $SOCKET_PATH.

Next:  make deploy
EOF
