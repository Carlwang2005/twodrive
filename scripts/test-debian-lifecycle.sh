#!/bin/sh
# Run only in a disposable root-owned container/VM. Never on a user's laptop.
set -eu
if [ "${TWODRIVE_DISPOSABLE_PACKAGE_TEST:-}" != 1 ]; then
    echo 'Requires TWODRIVE_DISPOSABLE_PACKAGE_TEST=1 in a disposable system' >&2
    exit 1
fi
# Minimal Ubuntu container images deliberately exclude manuals/docs during dpkg
# unpack. Package QA needs the complete installed payload, as on a normal desktop.
if [ -f /etc/dpkg/dpkg.cfg.d/excludes ]; then
    sed -i '\|^path-exclude=/usr/share/man/|d; \|^path-exclude=/usr/share/doc/|d' /etc/dpkg/dpkg.cfg.d/excludes
fi
old=$(realpath "$1")
new=$(realpath "$2")
export DEBIAN_FRONTEND=noninteractive
test_home=/home/twodrive-package-test
export XDG_CONFIG_HOME="$test_home/.config"
export XDG_DATA_HOME="$test_home/.local/share"
export XDG_CACHE_HOME="$test_home/.cache"
export TWODRIVE_MOUNT_DIR="$test_home/TwoDrive/OneDrive"
export TWODRIVE_BACKEND=mock
mkdir -p "$test_home"
apt-get install -y "$old"
twodrive init-mock
# Synthetic account/cache/recovery sentinels, never real credentials.
printf 'synthetic-private-sentinel\n' > "$XDG_CONFIG_HOME/twodrive/tokens.json"
mkdir -p "$XDG_DATA_HOME/twodrive/cache"
printf 'unuploaded-content-sentinel\n' > "$XDG_DATA_HOME/twodrive/cache/lifecycle-sentinel"
printf 'pending-session-sentinel\n' > "$XDG_DATA_HOME/twodrive/upload-sessions.json"
find "$test_home" -type f -exec sha256sum '{}' + | sort > /tmp/twodrive-state-before.sha256
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
