#!/bin/sh
# Run only in a disposable root-owned container/VM. Never on a user's laptop.
set -eu
if [ "${TWODRIVE_DISPOSABLE_PACKAGE_TEST:-}" != 1 ]; then
    echo 'Requires TWODRIVE_DISPOSABLE_PACKAGE_TEST=1 in a disposable system' >&2
    exit 1
fi
old=$(realpath "$1")
new=$(realpath "$2")
export DEBIAN_FRONTEND=noninteractive
export HOME=/home/twodrive-package-test
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export TWODRIVE_MOUNT_DIR="$HOME/TwoDrive/OneDrive"
export TWODRIVE_BACKEND=mock
mkdir -p "$HOME"
apt-get install -y "$old"
twodrive init-mock
# Synthetic account/cache/recovery sentinels, never real credentials.
printf 'synthetic-private-sentinel\n' > "$XDG_CONFIG_HOME/twodrive/tokens.json"
mkdir -p "$XDG_DATA_HOME/twodrive/cache"
printf 'unuploaded-content-sentinel\n' > "$XDG_DATA_HOME/twodrive/cache/lifecycle-sentinel"
printf 'pending-session-sentinel\n' > "$XDG_DATA_HOME/twodrive/upload-sessions.json"
find "$HOME" -type f -exec sha256sum '{}' + | sort > /tmp/twodrive-state-before.sha256
apt-get install -y "$new"
sha256sum -c /tmp/twodrive-state-before.sha256
AUTOPKGTEST_TMP=$(mktemp -d) sh /source/debian/tests/smoke
# Installing a package must not opt every desktop user into a cloud mount.
test ! -e /etc/systemd/user/default.target.wants/twodrive-daemon.service
# Check the ordinary remove/reinstall/purge lifecycle and user state preservation.
apt-get remove -y twodrive
sha256sum -c /tmp/twodrive-state-before.sha256
apt-get install -y "$new"
apt-get purge -y twodrive
sha256sum -c /tmp/twodrive-state-before.sha256
printf 'PASS: old-to-new upgrade, installed mock smoke, remove/reinstall/purge preserve user state\n'
