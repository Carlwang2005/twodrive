#!/usr/bin/env python3
"""Exercise the native Windows tray callback in an isolated desktop process.
Sends Shell notification messages, not hardware input. Does not use real profiles.
"""
import ctypes as c
from ctypes import wintypes as w
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

assert os.name == 'nt'
u = c.WinDLL('user32', use_last_error=True)
u.FindWindowW.argtypes = [w.LPCWSTR, w.LPCWSTR]
u.FindWindowW.restype = w.HWND
u.IsWindowVisible.argtypes = [w.HWND]
u.PostMessageW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM]
u.GetWindowRect.argtypes = [w.HWND, c.POINTER(w.RECT)]
u.GetWindowThreadProcessId.argtypes = [w.HWND, c.POINTER(w.DWORD)]

with tempfile.TemporaryDirectory(prefix='td-ui-') as directory:
    root = Path(directory)
    exe = Path(sys.argv[1]).resolve()
    with (root/'stderr.log').open('w', encoding='utf-8') as log:
        proc = subprocess.Popen([str(exe), '--background', '--state', str(root/'state')],
            cwd=exe.parent, stdout=log, stderr=log)
        try:
            deadline = time.monotonic()+30
            while True:
                hwnd = u.FindWindowW(None, 'TwoDrive Development')
                owner = w.DWORD()
                if hwnd:
                    u.GetWindowThreadProcessId(hwnd, c.byref(owner))
                if owner.value == proc.pid and (root/'state/run.lock').exists(): break
                assert proc.poll() is None, 'desktop process exited during startup'
                assert time.monotonic() < deadline, 'desktop did not initialize'
                time.sleep(.1)
            time.sleep(1)
            assert not u.IsWindowVisible(hwnd), 'background startup showed a window'
            positions=[]
            for visible in [True, False]*3:
                time.sleep(.3)
                started=time.monotonic()
                assert u.PostMessageW(hwnd, 0x8000+41, 0, 0x400), 'tray notification delivery failed'
                while bool(u.IsWindowVisible(hwnd)) != visible:
                    assert time.monotonic()-started<3, 'tray did not toggle'
                    time.sleep(.01)
                if visible:
                    rect=w.RECT();u.GetWindowRect(hwnd,c.byref(rect))
                    positions.append([rect.left,rect.top,rect.right-rect.left,rect.bottom-rect.top])
            assert positions[0]==positions[1]==positions[2], 'tray panel drifted'
            print(json.dumps({'native_tray_toggle':True,'background_hidden':True,'stable_position':True,
                'boundary':'Shell notification message simulation; no hardware input or cloud profile'}))
        finally:
            # This verifies window interaction. Graceful menu quit is a separate manual check.
            proc.terminate()
            proc.wait(timeout=10)
    # The child sees IPC EOF and releases its private state/job; never stop another app.
    deadline=time.monotonic()+10
    while True:
        try:
            import shutil
            shutil.rmtree(root/'state')
            break
        except FileNotFoundError:
            break
        except PermissionError:
            assert time.monotonic()<deadline, 'desktop engine retained private state after GUI exit'
            time.sleep(.1)
