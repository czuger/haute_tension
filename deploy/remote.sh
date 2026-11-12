#!/usr/bin/env bash
# The server side of every deploy step. Never run by hand: deploy/config.sh pipes
# it in over ssh with the settings in front of it, so there is no copy of it on the
# server to go stale.
#
#   remote <command> [arguments]
set -euo pipefail

: "${APP_NAME:?}" "${REMOTE_DIR:?}" "${RUN_USER:?}" "${SOCKET_DIR:?}" "${SOCKET_PATH:?}"
: "${NGINX_SOCKET_DIR:?}" "${NGINX_SOCKET_PATH:?}" "${NGINX_CONTAINER:?}"
: "${NGINX_COMPOSE_DIR:?}" "${NGINX_CONF_DIR:?}" "${NGINX_CONF_NAME:?}"
: "${SUPERVISOR_CONF_DIR:?}" "${SUPERVISOR_CONF_NAME:?}" "${TMPFILES_CONF:?}"
: "${URL_PREFIX:?}" "${PUBLIC_URL:?}"

CARGO="$HOME/.cargo/bin/cargo"

# --- One-time preparation ---------------------------------------------------------------------

# The repository the deploy pushes into, and the two directories the application
# writes. `updateInstead` is what makes a push check the new version out: the
# working tree is what gets built, so it has to move with the branch.
init_repository() {
  mkdir -p "$REMOTE_DIR" "$REMOTE_DIR/data" "$REMOTE_DIR/logs"
  if [ ! -d "$REMOTE_DIR/.git" ]; then
    git init -q -b main "$REMOTE_DIR"
    echo "    created $REMOTE_DIR (git, branch main)"
  fi
  git -C "$REMOTE_DIR" config receive.denyCurrentBranch updateInstead
  git -C "$REMOTE_DIR" config gc.auto 0
}

# The directory the socket goes in. /run is tmpfs and root's: tmpfs is why no
# socket can outlive a boot, and root's is why the directory is declared to
# systemd-tmpfiles rather than made by hand — a `mkdir` here would be gone at the
# next boot, and the server would come up with nowhere to listen.
#
# `--create` applies the file now, so the directory is there for this deploy too.
init_runtime_dir() {
  printf 'd %s 0755 %s %s -\n' "$SOCKET_DIR" "$RUN_USER" "$RUN_USER" \
    | sudo tee "$TMPFILES_CONF" >/dev/null
  sudo systemd-tmpfiles --create "$TMPFILES_CONF"
  echo "    ok      $TMPFILES_CONF: $(cat "$TMPFILES_CONF")"
  stat -c '    ok      %n  mode %a  owner %U (made again at every boot)' "$SOCKET_DIR"
}

# The toolchain. Not installed here for its own sake: the binary resolves .env,
# books/, data/ and logs/ from the directory it was *built* in, so it must be built
# in $REMOTE_DIR, which means the toolchain lives on the server.
init_toolchain() {
  if [ -x "$CARGO" ]; then
    echo "    ok      $("$CARGO" --version)"
    return
  fi
  echo "    installing rustup (minimal profile), this takes a few minutes"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path
  echo "    ok      $("$CARGO" --version)"
}

# The settings a run reads. Written once — the session key in it is what keeps
# every reader's hero across a restart, so it is never generated twice.
#
# Two of the keys are not the server's to choose: URL_PREFIX and LISTEN_SOCKET have
# to match what the nginx snippet was rendered from, and deploy/config.sh is where
# both come from. So on an existing file those two are reconciled and everything
# else — the key, the environment, the log settings — is left exactly as it is.
init_env() {
  local file="$REMOTE_DIR/.env"
  if [ -f "$file" ]; then
    echo "    ok      $file (kept; the session key is never rewritten)"
    reconcile_setting "$file" URL_PREFIX "$URL_PREFIX"
    reconcile_setting "$file" LISTEN_SOCKET "$SOCKET_PATH"
    return
  fi
  umask 077
  cat > "$file" <<ENV
# Written by the first \`make setup\`, on the server. The session key below is this
# server's own and no deploy ever rewrites it; URL_PREFIX and LISTEN_SOCKET are
# kept in step with deploy/config.sh by every \`make setup\`.
APP_ENV=prod
DATABASE_DIR=data

# Signs the session cookie. Changing it starts every reader on a new hero.
SESSION_SECRET_KEY=$(openssl rand -hex 32)

# nginx strips this on the way in; the application puts it back on every link.
URL_PREFIX=$URL_PREFIX

# A socket, because this is the deployed run: no port is published, and nginx
# reaches the application through this file. A dev run leaves it empty and listens
# on 127.0.0.1:5001 instead.
LISTEN_SOCKET=$SOCKET_PATH

# DEBUG, and on: a trace one must first go and turn on is a trace one does not
# have on the day it is needed. The log rotates at 512 KB and keeps five archives.
LOG_LEVEL=DEBUG
LOG_VALUE_LIMIT=2000
ENV
  echo "    created $file (session key generated)"
}

# One setting of .env brought in line with what this deploy was rendered from.
# Anything else in the file, and the order of it, is left alone.
reconcile_setting() {
  local file="$1" key="$2" wanted="$3" current
  current="$(sed -n "s/^$key=//p" "$file" | head -1)"
  if [ "$current" = "$wanted" ]; then
    echo "    ok      $key=$wanted"
    return
  fi
  if grep -q "^$key=" "$file"; then
    sed -i "s|^$key=.*|$key=$wanted|" "$file"
    echo "    changed $key: '$current' -> '$wanted'"
  else
    printf '\n%s=%s\n' "$key" "$wanted" >> "$file"
    echo "    added   $key=$wanted"
  fi
}

# --- Building and running ---------------------------------------------------------------------

build() {
  cd "$REMOTE_DIR"
  # --locked: the lock file came with the payload, and a deploy is not the place to
  # resolve a dependency differently from the version that was tested.
  # --bin serve: the import pipeline's binary is never run here.
  "$CARGO" build --release --locked --bin serve
  ls -l "$REMOTE_DIR/target/release/serve" | sed 's/^/    /'
}

# supervisorctl talks over a root-owned socket, so every call sudoes. `reread` +
# `update` pick up a changed program file; `restart` is what puts the new binary in
# place of the running one.
supervise() {
  sudo supervisorctl reread >/dev/null
  sudo supervisorctl update "$APP_NAME" >/dev/null
  sudo supervisorctl restart "$APP_NAME" | sed 's/^/    /'
}

# --- Checks -----------------------------------------------------------------------------------

# What the socket answers, which is what nginx will get.
over_the_socket() {
  local path="${1:-/}"
  curl -sS --max-time 10 --unix-socket "$SOCKET_PATH" \
    -o /dev/null -w '%{http_code}' "http://localhost$path" 2>/dev/null || echo "000"
}

# Whether the container nginx runs in can see the socket at all. Without the mount
# every request under the prefix is a 502, and nothing else says why.
socket_is_mounted() {
  docker exec "$NGINX_CONTAINER" test -S "$NGINX_SOCKET_PATH" 2>/dev/null
}

mount_instructions() {
  cat <<HOWTO
    The socket directory is not visible inside $NGINX_CONTAINER.

    Add it to $NGINX_COMPOSE_DIR/docker-compose.yml, under the nginx service's
    volumes — the same path on both sides, so there is nothing to keep in step:

      - $SOCKET_DIR:$NGINX_SOCKET_DIR

    then recreate the container (a few seconds, every site behind it):

      cd $NGINX_COMPOSE_DIR && docker compose up -d nginx
HOWTO
}

case "${1:-checks}" in

  init)
    echo "==> Repository and directories"
    init_repository
    echo "==> The runtime directory the socket goes in"
    init_runtime_dir
    echo "==> Toolchain"
    init_toolchain
    echo "==> Settings"
    init_env
    ;;

  build)
    echo "==> Building $APP_NAME in $REMOTE_DIR"
    build
    ;;

  release)
    echo "==> Building $APP_NAME in $REMOTE_DIR"
    build
    echo "==> Restarting under supervisord"
    supervise
    ;;

  restart)
    supervise
    ;;

  # Both halves of the nginx side: the file has to parse before nginx is asked to
  # take it, or a bad snippet takes down every other site with it.
  nginx_reload)
    echo "==> Validating the nginx configuration"
    docker exec "$NGINX_CONTAINER" nginx -t 2>&1 | sed 's/^/    /'
    docker exec "$NGINX_CONTAINER" nginx -s reload
    echo "    reloaded $NGINX_CONTAINER"
    if ! socket_is_mounted; then
      echo "==> One thing is missing"
      mount_instructions
    fi
    ;;

  nginx_mount)
    if socket_is_mounted; then
      echo "    ok      $NGINX_SOCKET_PATH is visible inside $NGINX_CONTAINER"
    else
      mount_instructions
      exit 1
    fi
    ;;

  # The health check a deploy waits on: the socket answers, and the pages that
  # need the database answer too.
  health)
    for _ in $(seq 1 20); do
      [ "$(over_the_socket /)" = "200" ] && break
      sleep 0.5
    done
    landing="$(over_the_socket /)"
    page="$(over_the_socket /book/1)"
    echo "    landing page over the socket : $landing"
    echo "    a story page (needs the database) : $page"
    [ "$landing" = "200" ] && [ "$page" = "200" ]
    ;;

  # Everything that has to be true for the site to be up, each printed whether it
  # holds or not, so one run says what is wrong rather than only that something is.
  checks)
    trouble=0
    echo "==> supervisord"
    if sudo supervisorctl status "$APP_NAME" | sed 's/^/    /'; then :; else trouble=1; fi

    echo "==> The socket"
    if [ -S "$SOCKET_PATH" ]; then
      stat -c '    ok      %n  mode %a  owner %U' "$SOCKET_PATH"
    else
      echo "    MISSING $SOCKET_PATH"
      trouble=1
    fi
    # /run is tmpfs: without this file the directory is gone at the next boot, and
    # the server comes up with nowhere to listen. Nothing else would say so until
    # then, which is why it is checked here and not only written by setup.
    if [ -f "$TMPFILES_CONF" ]; then
      echo "    ok      $TMPFILES_CONF remakes $SOCKET_DIR at every boot"
    else
      echo "    MISSING $TMPFILES_CONF — $SOCKET_DIR would not survive a reboot"
      trouble=1
    fi
    if socket_is_mounted; then
      echo "    ok      visible inside $NGINX_CONTAINER at the same path"
    else
      echo "    MISSING not visible inside $NGINX_CONTAINER — every request is a 502"
      mount_instructions
      trouble=1
    fi

    echo "==> The application, over the socket"
    landing="$(over_the_socket /)"
    page="$(over_the_socket /book/1)"
    echo "    /       $landing"
    echo "    /book/1 $page   (needs the database)"
    { [ "$landing" = "200" ] && [ "$page" = "200" ]; } || trouble=1

    echo "==> nginx"
    if docker exec "$NGINX_CONTAINER" nginx -t >/dev/null 2>&1; then
      echo "    ok      the configuration parses"
    else
      echo "    BROKEN  the configuration does not parse"
      trouble=1
    fi
    through_nginx="$(curl -sSk --max-time 10 -o /dev/null -w '%{http_code}' "$PUBLIC_URL" || echo 000)"
    echo "    $PUBLIC_URL -> $through_nginx"
    [ "$through_nginx" = "200" ] || trouble=1

    echo "==> The database"
    file="$REMOTE_DIR/data/haute_tension_prod.sqlite3"
    if [ -f "$file" ]; then
      echo "    ok      $file (schema $(sqlite3 "$file" 'PRAGMA user_version;'), $(du -h "$file" | cut -f1))"
    else
      echo "    absent  $file — made on the first request that needs it"
    fi

    echo "==> The settings the two sides have to agree on"
    in_env="$(grep -E '^URL_PREFIX=' "$REMOTE_DIR/.env" | cut -d= -f2-)"
    if [ "${in_env%/}" = "${URL_PREFIX%/}" ]; then
      echo "    ok      URL_PREFIX is $URL_PREFIX in .env and in the nginx snippet"
    else
      echo "    WRONG   URL_PREFIX is '$in_env' in .env, '$URL_PREFIX' in the nginx snippet"
      trouble=1
    fi
    in_env="$(grep -E '^LISTEN_SOCKET=' "$REMOTE_DIR/.env" | cut -d= -f2-)"
    if [ "$in_env" = "$SOCKET_PATH" ]; then
      echo "    ok      LISTEN_SOCKET is the socket nginx connects to"
    else
      echo "    WRONG   LISTEN_SOCKET is '$in_env', nginx connects to '$SOCKET_PATH'"
      trouble=1
    fi

    echo "==> The log"
    if [ -f "$REMOTE_DIR/logs/general.log" ]; then
      tail -3 "$REMOTE_DIR/logs/general.log" | sed 's/^/    /'
    else
      echo "    absent  $REMOTE_DIR/logs/general.log"
    fi

    [ "$trouble" = "0" ] || { echo; echo "Something above is not right."; exit 1; }
    echo
    echo "All checks pass."
    ;;

  status)
    sudo supervisorctl status "$APP_NAME" | sed 's/^/    /' || true
    echo "    version : $(cat "$REMOTE_DIR/RELEASE" 2>/dev/null || echo 'none deployed')"
    echo "    commit  : $(git -C "$REMOTE_DIR" log -1 --format='%h %s' 2>/dev/null || echo 'no history')"
    echo "    socket  : $(over_the_socket /) over $SOCKET_PATH"
    echo "    public  : $(curl -sSk --max-time 10 -o /dev/null -w '%{http_code}' "$PUBLIC_URL" || echo 000) at $PUBLIC_URL"
    ;;

  # The deployed versions, newest first: the repository's own history.
  versions)
    git -C "$REMOTE_DIR" log -10 --format='%h  %ad  %s' --date=format:'%Y-%m-%d %H:%M' 2>/dev/null \
      || echo "    no history yet"
    ;;

  # One version back in the same repository, rebuilt and restarted. The push that
  # brought the bad version is still in the history, so this is reversible too.
  rollback)
    cd "$REMOTE_DIR"
    git log -1 --format='    leaving %h %s'
    git reset --hard -q HEAD~1
    git log -1 --format='    now on  %h %s'
    build
    supervise
    ;;

  *)
    echo "unknown command: ${1:-}" >&2
    exit 64
    ;;
esac
