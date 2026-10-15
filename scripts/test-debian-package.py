#!/usr/bin/env python3
"""Inspect the produced archive; fail for missing dependencies or payload."""
import argparse
from pathlib import Path
import subprocess
import tarfile
import io


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


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    verify(parser.parse_args().package)
