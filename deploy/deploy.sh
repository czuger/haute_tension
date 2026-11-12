#!/usr/bin/env bash
# Build, ship, restart and check: make deploy
#
# What is shipped is THE MINIMUM — deploy/config.sh's PAYLOAD, which is what the
# server needs to build and run and nothing else. It goes as a commit in the
# server's own repository, so the versions on the server are its history and a
# rollback is one commit back.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

release="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
git -C "$REPO_ROOT" diff --quiet || release="$release+dirty"
stamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

echo "==> Deploying $release to $SSH_HOST:$REMOTE_DIR"

# 1. The local clone the payload is built in. Cloned from the server when it is
#    missing, so a wiped dist/ does not lose the history the rollback reads.
if [ ! -d "$CLONE/.git" ]; then
  echo "==> Cloning the server's repository into ${CLONE#"$REPO_ROOT"/}"
  mkdir -p "$(dirname "$CLONE")"
  git clone -q "$SSH_HOST:$REMOTE_DIR" "$CLONE" 2>/dev/null || {
    git init -q "$CLONE"
    git -C "$CLONE" remote add origin "$SSH_HOST:$REMOTE_DIR"
  }
fi
git -C "$CLONE" fetch -q origin || true
if git -C "$CLONE" rev-parse -q --verify refs/remotes/origin/main >/dev/null; then
  git -C "$CLONE" checkout -q -B main origin/main
else
  git -C "$CLONE" symbolic-ref HEAD refs/heads/main
fi

# 2. The payload, replaced whole: what is gone here has to go there too, and
#    copying over the top would leave a deleted file behind for ever.
echo "==> Building the payload: ${PAYLOAD[*]}"
for path in "${PAYLOAD[@]}" .gitignore RELEASE; do
  rm -rf "$CLONE/$path"
done
for path in "${PAYLOAD[@]}"; do
  cp -R "$REPO_ROOT/$path" "$CLONE/$path"
done
cp "$REPO_ROOT/deploy/payload.gitignore" "$CLONE/.gitignore"
printf '%s %s\n' "$release" "$stamp" > "$CLONE/RELEASE"
du -sh "$CLONE" | sed 's/^/    /'

# 3. One commit per deploy, even when nothing changed: the history is then the
#    record of what was deployed and when, not only of what was edited.
git -C "$CLONE" add -A
git -C "$CLONE" commit -q --allow-empty -m "Deploy $release at $stamp"
echo "==> Pushing"
git -C "$CLONE" push -q origin main
git -C "$CLONE" log -1 --format='    %h %s' | sed 's/^/  /'

# 4. The server builds the version it just checked out, and supervisord is given
#    the program file again in case a path in it changed.
install_supervisor_conf
remote release

# 5. It is not deployed until it answers.
echo "==> Health"
remote health || {
  echo "error: the new version does not answer — the log says why:" >&2
  ssh "$SSH_HOST" "tail -20 '$REMOTE_DIR/logs/supervisor.log' '$REMOTE_DIR/logs/general.log'" >&2
  exit 1
}
echo
echo "$release is up: $PUBLIC_URL"
