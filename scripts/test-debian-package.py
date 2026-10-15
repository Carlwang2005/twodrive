#!/usr/bin/env python3
"""Inspect the produced archive; fail for missing dependencies or payload."""
import argparse
from pathlib import Path
import subprocess
import tarfile
import io
import hashlib


def verify_manifests(directory):
    manifests = list(directory.glob('*.changes')) + list(directory.glob('*.dsc'))
    assert manifests, 'missing Debian upload manifests'
    for manifest in manifests:
        active = False
        checked = 0
        for line in manifest.read_text().splitlines():
            if line == 'Checksums-Sha256:':
                active = True
                continue
            if not line.startswith(' '):
                active = False
            if active:
                digest, size, name = line.split()
                assert Path(name).name == name, 'unexpected manifest path'
                path = directory / name
                assert path.is_file(), f'missing manifest member: {name}'
                assert path.stat().st_size == int(size), f'wrong size: {name}'
                with path.open('rb') as stream:
                    assert hashlib.file_digest(stream, 'sha256').hexdigest() == digest, f'wrong SHA-256: {name}'
                checked += 1
        assert checked, f'missing SHA-256 entries: {manifest.name}'
    print(f'Verified all members of {len(manifests)} Debian upload manifests')


def verify(package):
    fields = subprocess.check_output(['dpkg-deb', '-f', str(package)], text=True)
    for dependency in ['libc6', 'libsqlite3-0', 'python3', 'fuse3']:
        assert dependency in fields, f'missing generated/runtime dependency: {dependency}'
    assert 'noreply@localhost' not in fields
    archive = subprocess.check_output(['dpkg-deb', '--fsys-tarfile', str(package)])
    with tarfile.open(fileobj=io.BytesIO(archive)) as stream:
        members = stream.getmembers()
        assert all(m.uid == 0 and m.gid == 0 for m in members), 'non-root package ownership'
        names = {m.name.removeprefix('./') for m in members}
        for name in ['usr/bin/twodrive', 'usr/bin/twodrive-daemon',
                     'usr/share/doc/twodrive/copyright',
                     'usr/share/doc/twodrive/changelog.Debian.gz',
                     'usr/share/man/man1/twodrive.1.gz',
                     'usr/lib/systemd/user/twodrive-daemon.service']:
            assert name in names, f'missing payload: {name}'
    print(f'Verified Debian dependencies, ownership and payload: {package.name}')
    verify_manifests(package.parent)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    verify(parser.parse_args().package)
