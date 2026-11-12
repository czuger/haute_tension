#!/usr/bin/env bash
# Shared settings for the deploy scripts, sourced by all of them (never run alone).
#
# Every value can be overridden from the environment without editing this file:
#   SSH_HOST=ced@other-box make deploy
#
# This file is versioned on purpose: it says *where* the application goes, which
# is not a secret. The secrets live only in $REMOTE_DIR/.env on the server, which
# no deploy step reads or writes.

# --- Where it goes ----------------------------------------------------------------------------
#
# REMOTE_DIR is a git repository of its own: the deploy pushes the minimum into it
# and the server builds and runs from that working tree. It is not this repository
# — no history, no tests, no work/ — and the build has to happen there, because the
# binary resolves .env, books/, data/ and logs/ from the directory it was built in.
SSH_HOST="${SSH_HOST:-ced@nuc150}"
REMOTE_DIR="${REMOTE_DIR:-/home/ced/rust}"

# The user the server runs as: the owner of the repository, the database and the
# socket. Taken from SSH_HOST, which is who pushes.
RUN_USER="${RUN_USER:-${SSH_HOST%%@*}}"

# --- What runs it -----------------------------------------------------------------------------
#
# supervisord keeps the process up and restarts it when it dies. The system-wide
# daemon includes SUPERVISOR_CONF_DIR, so the program file goes there and needs no
# root to write — only `supervisorctl` does, which is why every call to it sudoes.
APP_NAME="${APP_NAME:-haute_tension}"
SUPERVISOR_CONF_DIR="${SUPERVISOR_CONF_DIR:-/home/ced/supervisor/conf.d}"
SUPERVISOR_CONF_NAME="${SUPERVISOR_CONF_NAME:-$APP_NAME.conf}"

# --- How nginx reaches it ---------------------------------------------------------------------
#
# Over a UNIX socket, not a port: nothing of the application is reachable on the
# network, and the file's own permissions are what says who may connect.
#
# This is the prod half of the arrangement, and the only half that has a socket: a
# dev run listens on 127.0.0.1:5001 and is reached in a browser directly. What
# picks one or the other is LISTEN_SOCKET in the run's own .env — set here, empty
# in .env.example.
#
# The socket lives under /run, where runtime files belong: tmpfs, so it cannot
# outlive a boot and there is never a socket from last week to take over. /run is
# root's, so the directory is declared to systemd-tmpfiles (TMPFILES_CONF), which
# makes it again — owned by RUN_USER — at every boot. `/var/run` is a compatibility
# symlink to `/run`: the same directory, spelled the way the FHS spells it.
#
# nginx runs in a container, so the directory is bind-mounted into it at the very
# same path: one spelling for both sides, and nothing that can disagree.
SOCKET_DIR="${SOCKET_DIR:-/run/$APP_NAME}"
SOCKET_NAME="${SOCKET_NAME:-serve.sock}"
NGINX_SOCKET_DIR="${NGINX_SOCKET_DIR:-$SOCKET_DIR}"
TMPFILES_CONF="${TMPFILES_CONF:-/etc/tmpfiles.d/$APP_NAME.conf}"

# The dockerized nginx: its container, the compose project it belongs to, and the
# directory whose *.conf the site's server{} includes.
NGINX_CONTAINER="${NGINX_CONTAINER:-nginx-proxy}"
NGINX_COMPOSE_DIR="${NGINX_COMPOSE_DIR:-/home/ced/services/nginx_proxy}"
NGINX_CONF_DIR="${NGINX_CONF_DIR:-/home/ced/services/nginx_proxy/sites/apps}"
NGINX_CONF_NAME="${NGINX_CONF_NAME:-$APP_NAME.conf}"

# The sub-path the site answers under, and the host it answers on. The server's
# .env must carry the same URL_PREFIX: nginx strips it on the way in, and the
# application puts it back on every link and redirect going out. This file is the
# source of both — `make setup` writes this value and the socket into the server's
# .env, leaving everything else in it alone, and `make checks` compares them.
URL_PREFIX="${URL_PREFIX:-/haute-tension}"
PUBLIC_HOST="${PUBLIC_HOST:-apps.ieroe.com}"

# --- Derived; no need to touch it -------------------------------------------------------------
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOCKET_PATH="$SOCKET_DIR/$SOCKET_NAME"
NGINX_SOCKET_PATH="$NGINX_SOCKET_DIR/$SOCKET_NAME"
PUBLIC_URL="https://$PUBLIC_HOST${URL_PREFIX%/}/"
# The local clone the payload is built in, and pushed from. Git-ignored, and
# rebuilt by cloning the server's repository when it is missing.
CLONE="${CLONE:-$REPO_ROOT/dist/deploy}"

# THE MINIMUM. Everything the server needs to build and run, and nothing else: no
# tests (they would pull in the dev dependencies), no work/ (three thousand
# scraped pages), no documentation, no history.
PAYLOAD=(Cargo.toml Cargo.lock askama.toml src templates books migrations)

# Run one command of deploy/remote.sh on the server.
#
# The script is piped over stdin rather than left on the server, so there is no
# copy of it there to go stale, and the settings above are passed in front of it
# so the server never keeps its own.
remote() {
  ssh "$SSH_HOST" \
    "APP_NAME='$APP_NAME' REMOTE_DIR='$REMOTE_DIR' RUN_USER='$RUN_USER' \
     SOCKET_DIR='$SOCKET_DIR' SOCKET_PATH='$SOCKET_PATH' \
     NGINX_SOCKET_DIR='$NGINX_SOCKET_DIR' NGINX_SOCKET_PATH='$NGINX_SOCKET_PATH' \
     NGINX_CONTAINER='$NGINX_CONTAINER' NGINX_COMPOSE_DIR='$NGINX_COMPOSE_DIR' \
     NGINX_CONF_DIR='$NGINX_CONF_DIR' NGINX_CONF_NAME='$NGINX_CONF_NAME' \
     SUPERVISOR_CONF_DIR='$SUPERVISOR_CONF_DIR' SUPERVISOR_CONF_NAME='$SUPERVISOR_CONF_NAME' \
     TMPFILES_CONF='$TMPFILES_CONF' \
     URL_PREFIX='$URL_PREFIX' PUBLIC_URL='$PUBLIC_URL' \
     bash -s -- $*" < "$REPO_ROOT/deploy/remote.sh"
}

# Render a template's __PLACEHOLDERS__ from the settings above.
render() {
  sed -e "s|__APP_NAME__|$APP_NAME|g" \
      -e "s|__REMOTE_DIR__|$REMOTE_DIR|g" \
      -e "s|__RUN_USER__|$RUN_USER|g" \
      -e "s|__PREFIX__|${URL_PREFIX%/}|g" \
      -e "s|__SOCKET__|$NGINX_SOCKET_PATH|g" \
      "$1"
}

# Ship the nginx snippet, have the server validate it, and only then reload nginx.
# Used by `make setup` and `make nginx`; no deploy calls it — routing does not
# change from one version to the next.
install_nginx_conf() {
  local rendered="$REPO_ROOT/dist/$NGINX_CONF_NAME"
  mkdir -p "$REPO_ROOT/dist"
  render "$REPO_ROOT/deploy/nginx/$NGINX_CONF_NAME.template" > "$rendered"
  ssh "$SSH_HOST" "mkdir -p '$NGINX_CONF_DIR'"
  scp -q "$rendered" "$SSH_HOST:$NGINX_CONF_DIR/$NGINX_CONF_NAME"
  rm -f "$rendered"
  remote nginx_reload
}

# Ship the supervisord program file. Done on every deploy, so a changed path or
# setting cannot drift from what is actually running.
install_supervisor_conf() {
  local rendered="$REPO_ROOT/dist/$SUPERVISOR_CONF_NAME"
  mkdir -p "$REPO_ROOT/dist"
  render "$REPO_ROOT/deploy/supervisor/$APP_NAME.conf.template" > "$rendered"
  ssh "$SSH_HOST" "mkdir -p '$SUPERVISOR_CONF_DIR'"
  scp -q "$rendered" "$SSH_HOST:$SUPERVISOR_CONF_DIR/$SUPERVISOR_CONF_NAME"
  rm -f "$rendered"
}
