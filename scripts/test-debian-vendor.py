#!/usr/bin/env python3
"""Verify Cargo's full vendored file set survives Debian clean/source handling."""
import argparse
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('source', type=Path)
args = parser.parse_args()
checked = 0
for manifest in sorted((args.source / 'vendor').glob('*/.cargo-checksum.json')):
    for relative, digest in json.loads(manifest.read_text())['files'].items():
        path = manifest.parent / relative
        assert path.is_file(), f'missing vendored input: {path}'
        with path.open('rb') as stream:
            assert hashlib.file_digest(stream, 'sha256').hexdigest() == digest, f'changed vendored input: {path}'
        checked += 1
assert checked, 'missing vendor component'
print(f'PASS: {checked} vendored files retain their Cargo checksums')
