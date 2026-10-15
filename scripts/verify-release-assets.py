#!/usr/bin/env python3
"""Verify the exact release payload; publish only after every check succeeds."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from urllib.parse import quote


def verify(release, tag, target, files):
    expected = {Path(path).name: Path(path) for path in files}
    if not expected or len(expected) != len(files):
        raise ValueError('expected payload must contain unique filenames')
    if len(target) != 40 or any(c not in '0123456789abcdef' for c in target):
        raise ValueError('expected source must be a full commit SHA')
    if (release['tagName'] != tag or release['targetCommitish'] != target
            or release['resolvedTagCommit'] != target):
        raise ValueError('release tag/source target mismatch')
    assets = release['assets']
    if len(assets) != len(expected) or {a['name'] for a in assets} != set(expected):
        raise ValueError('release asset set is incomplete or unexpected')
    for asset in assets:
        path = expected[asset['name']]
        checksum = hashlib.sha256()
        with path.open('rb') as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b''):
                checksum.update(block)
        digest = 'sha256:' + checksum.hexdigest()
        if (asset['state'] != 'uploaded' or asset['size'] <= 0
                or asset['size'] != path.stat().st_size or asset.get('digest') != digest):
            raise ValueError('release asset state/size/digest mismatch: ' + asset['name'])
    return sorted(expected)


def check_and_publish(release, tag, target, files, channel=None, run=subprocess.run):
    names = verify(release, tag, target, files)
    if channel is not None:
        if not release['isDraft']:
            raise ValueError('publication requires a draft release')
        if release['isPrerelease'] != (channel == 'preview'):
            raise ValueError('release channel mismatch')
        command = ['gh', 'release', 'edit', tag, '--draft=false']
        command += ['--latest'] if channel == 'stable' else ['--prerelease', '--latest=false']
        run(command, check=True)
    return names


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag', required=True)
    parser.add_argument('--target', required=True, help='Verified full source commit SHA')
    parser.add_argument('--publish', choices=['stable', 'preview'], help='Otherwise verify only')
    parser.add_argument('files', nargs='+', type=Path)
    args = parser.parse_args()
    release = json.loads(subprocess.check_output([
        'gh', 'release', 'view', args.tag, '--json',
        'tagName,targetCommitish,isDraft,isPrerelease,assets',
    ], text=True))
    # Require an existing, immutable source tag; never create a tag implicitly
    # while publishing. Annotated tags must resolve to the same source commit.
    endpoint = 'repos/{owner}/{repo}/git/ref/tags/' + quote(args.tag, safe='')
    obj = json.loads(subprocess.check_output(['gh', 'api', endpoint], text=True))['object']
    for _ in range(8):
        if obj['type'] == 'commit':
            break
        if obj['type'] != 'tag':
            raise ValueError('release tag does not resolve to a commit')
        obj = json.loads(subprocess.check_output([
            'gh', 'api', 'repos/{owner}/{repo}/git/tags/' + obj['sha'],
        ], text=True))['object']
    else:
        raise ValueError('release tag nesting limit exceeded')
    release['resolvedTagCommit'] = obj['sha']
    names = check_and_publish(release, args.tag, args.target, args.files, args.publish)
    print(json.dumps({'tag': args.tag, 'verified_assets': names,
                      'publication_requested': args.publish is not None}))


if __name__ == '__main__':
    main()
