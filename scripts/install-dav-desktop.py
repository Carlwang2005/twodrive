#!/usr/bin/env python3
"""Install the Linux DAV app for this user, without enabling autostart."""
import argparse
import os
from pathlib import Path
import shutil
import tempfile


def quote_exec(value):
    if any(c in value for c in '\n\r%'):
        raise ValueError('unsupported desktop launcher path')
    return '"' + value.replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$') + '"'


def install(bundle, engine_dir, data, bin_dir):
    app = data / 'twodrive-dav'
    app.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix='install-', dir=app))
    previous = app / 'previous'
    runtime = app / 'runtime'
    try:
        shutil.copytree(bundle, stage, dirs_exist_ok=True)
        for name in ['twodrive-dev', 'twodrive-dav-engine']:
            shutil.copy2(engine_dir / name, stage / name)
            (stage / name).chmod(0o755)
        if not (stage / 'twodrive_dav').is_file():
            raise ValueError('Flutter Linux bundle is missing')
        if previous.exists():
            shutil.rmtree(previous)
        if runtime.exists():
            runtime.rename(previous)
        stage.rename(runtime)
    finally:
        if stage.exists():
            shutil.rmtree(stage)
    bin_dir.mkdir(parents=True, exist_ok=True)
    launcher = bin_dir / 'twodrive-dav'
    # exec argv is constructed in Python, so install paths need no shell expansion.
    launcher.write_text('#!/usr/bin/env python3\nimport os, sys\n'
        + 'if os.environ.get("DISPLAY"):\n    os.environ.setdefault("GDK_BACKEND", "x11")\n'
        + 'program = ' + repr(str(runtime / 'twodrive_dav')) + '\n'
        + 'os.execv(program, [program, "--state", ' + repr(str(app / 'state'))
        + ', "--launcher", ' + repr(str(launcher)) + '] + sys.argv[1:])\n')
    launcher.chmod(0o755)
    icons = data / 'icons/hicolor/scalable/apps'
    icons.mkdir(parents=True, exist_ok=True)
    shutil.copy2(runtime / 'data/flutter_assets/assets/cloud-connected.svg', icons / 'twodrive-dav.svg')
    applications = data / 'applications'
    applications.mkdir(parents=True, exist_ok=True)
    (applications / 'app.twodrive.twodrive_dav.desktop').write_text(
        '[Desktop Entry]\nType=Application\nName=TwoDrive Development\nComment=WebDAV files and TwoDrive paired shares\n'
        + 'Exec=' + quote_exec(str(launcher)) + '\nIcon=twodrive-dav\nTerminal=false\n'
        + 'Categories=Network;FileTransfer;\nStartupWMClass=app.twodrive.twodrive_dav\n')
    # Deliberately do not create or modify XDG_CONFIG_HOME/autostart.
    if previous.exists():
        shutil.rmtree(previous)
    print('Installed: ' + str(launcher))
    print('Autostart: unchanged (a new installation defaults to off)')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--engine-dir', type=Path, required=True)
    parser.add_argument('--data-dir', type=Path, default=Path(os.environ.get('XDG_DATA_HOME', Path.home() / '.local/share')))
    parser.add_argument('--bin-dir', type=Path, default=Path.home() / '.local/bin')
    args = parser.parse_args()
    install(args.bundle.resolve(), args.engine_dir.resolve(), args.data_dir.resolve(), args.bin_dir.resolve())
