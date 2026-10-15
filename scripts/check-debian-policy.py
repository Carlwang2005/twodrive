#!/usr/bin/env python3
"""Enforce the shared release packaging entry point on main and dev."""
from pathlib import Path
import re

root = Path(__file__).resolve().parent.parent
script = (root / 'scripts/build-deb.sh').read_text()
assert 'dpkg-buildpackage' in script, 'release packaging must invoke dpkg-buildpackage'
assert not re.search(r'dpkg-deb\s+(?:--build|-b)\b', script), 'manual release packaging is prohibited'
assert not (root / 'packaging/deb/control').exists(), 'duplicate package control metadata'
assert 'dh $@' in (root / 'debian/rules').read_text(), 'debhelper must drive package assembly'
assert (root / 'debian/source/format').read_text().strip() == '3.0 (quilt)'
for name in ('AGENTS.md', 'CONTRIBUTING.md'):
    assert 'doc/debian-packaging.md' in (root / name).read_text(), f'missing policy link in {name}'
print('PASS: standard Debian release workflow and shared policy')
