#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
for tool in dpkg-buildpackage dpkg-parsechangelog dh cargo python3; do
    command -v "$tool" >/dev/null || { echo "Missing build tool: $tool (see debian/README.source)" >&2; exit 1; }
done
# Prepare explicit, self-contained source inputs first. dpkg-buildpackage and
# debian/rules never download dependencies or use a developer's Cargo cache.
output="$project_root/dist/debian"
tree="$(python3 scripts/prepare-debian-source.py --output "$output")"
(
    cd "$tree"
    dpkg-buildpackage --build=full --no-sign "$@"
)
shopt -s nullglob
artifacts=("$tree"/../*.deb "$tree"/../*.ddeb "$tree"/../*.dsc "$tree"/../*.tar.xz "$tree"/../*.buildinfo "$tree"/../*.changes)
cp -- "${artifacts[@]}" "$output/"
python3 "$project_root/scripts/test-debian-package.py" "$output"/twodrive_*_"$(dpkg --print-architecture)".deb
printf 'Debian source, binary and upload manifests: %s\n' "$output"
