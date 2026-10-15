#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
for tool in dpkg-buildpackage dpkg-parsechangelog dh cargo; do
    command -v "$tool" >/dev/null || { echo "Missing build tool: $tool (see debian/README.source)" >&2; exit 1; }
done
# Provision dependencies explicitly before this step. Package builds never fetch.
dpkg-buildpackage --build=binary --no-sign "$@"
version="$(dpkg-parsechangelog -S Version)"
mkdir -p dist/debian
shopt -s nullglob
outputs=(../twodrive*"_${version}_"*.deb ../twodrive*"_${version}_"*.ddeb ../twodrive_"${version}_"*.buildinfo ../twodrive_"${version}_"*.changes)
if ((${#outputs[@]} == 0)); then echo 'No Debian build outputs found' >&2; exit 1; fi
cp -- "${outputs[@]}" dist/debian/
printf 'Debian build outputs: %s/dist/debian\n' "$project_root"
