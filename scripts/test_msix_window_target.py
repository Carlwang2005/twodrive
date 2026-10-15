"""A helper window on another thread must never receive the GUI quit message."""
import ctypes as c
from ctypes import wintypes as w
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import threading
import time
import unittest


def fixture(directory):
    u=c.WinDLL('user32',use_last_error=True)
    k=c.WinDLL('kernel32',use_last_error=True)
    callback=c.WINFUNCTYPE(c.c_ssize_t,w.HWND,w.UINT,w.WPARAM,w.LPARAM)
    u.DefWindowProcW.argtypes=[w.HWND,w.UINT,w.WPARAM,w.LPARAM]
    u.DefWindowProcW.restype=c.c_ssize_t
    proc=callback(lambda *args:u.DefWindowProcW(*args))
    class WindowClass(c.Structure):
        _fields_=[('style',w.UINT),('proc',callback),('class_extra',c.c_int),('window_extra',c.c_int),('instance',w.HINSTANCE),('icon',w.HICON),('cursor',w.HANDLE),('background',w.HBRUSH),('menu',w.LPCWSTR),('name',w.LPCWSTR)]
    k.GetModuleHandleW.argtypes=[w.LPCWSTR];k.GetModuleHandleW.restype=w.HMODULE
    u.RegisterClassW.argtypes=[c.POINTER(WindowClass)];u.RegisterClassW.restype=w.ATOM
    u.CreateWindowExW.argtypes=[w.DWORD,w.LPCWSTR,w.LPCWSTR,w.DWORD,c.c_int,c.c_int,c.c_int,c.c_int,w.HWND,w.HMENU,w.HINSTANCE,w.LPVOID];u.CreateWindowExW.restype=w.HWND
    u.SetWindowPos.argtypes=[w.HWND,w.HWND,c.c_int,c.c_int,c.c_int,c.c_int,w.UINT]
    u.GetWindowThreadProcessId.argtypes=[w.HWND,c.POINTER(w.DWORD)]
    u.GetMessageW.argtypes=[c.POINTER(w.MSG),w.HWND,w.UINT,w.UINT]
    instance=k.GetModuleHandleW(None)
    cls=WindowClass();cls.proc=proc;cls.instance=instance;cls.name='FLUTTER_RUNNER_WIN32_WINDOW'
    assert u.RegisterClassW(c.byref(cls))
    runner=u.CreateWindowExW(0,cls.name,'isolated runner',0,0,0,1,1,None,None,instance,None)
    assert runner
    ready=threading.Event()
    def decoy():
        window=u.CreateWindowExW(0,'STATIC','isolated COM-like helper',0,0,0,1,1,None,None,instance,None)
        assert window
        assert u.SetWindowPos(window,w.HWND(-1),0,0,0,0,0x1|0x2|0x10)
        ready.set()
        msg=w.MSG()
        while u.GetMessageW(c.byref(msg),None,0,0)>0: pass
    threading.Thread(target=decoy,daemon=True).start()
    assert ready.wait(5)
    windows=[]
    enum=c.WINFUNCTYPE(w.BOOL,w.HWND,w.LPARAM)
    def collect(window,_):
        pid=w.DWORD();u.GetWindowThreadProcessId(window,c.byref(pid))
        if pid.value==os.getpid(): windows.append(window)
        return True
    u.EnumWindows.argtypes=[enum,w.LPARAM]
    assert u.EnumWindows(enum(collect),0)
    assert windows and windows[0]!=runner,'fixture requires the helper to precede the runner'
    (directory/'ready.json').write_text(json.dumps({'pid':os.getpid(),'helper_precedes_runner':True}),encoding='utf-8')
    msg=w.MSG()
    while u.GetMessageW(c.byref(msg),None,0,0)>0: pass
    (directory/'runner-stopped').write_text('runner received WM_QUIT',encoding='utf-8')


@unittest.skipUnless(os.name=='nt','native Win32 fixture')
class WindowTargetTests(unittest.TestCase):
    def test_quit_targets_runner_after_another_thread_helper(self):
        source=Path(os.environ.get('TWODRIVE_TEST_MSIX_HELPER_SOURCE',Path(__file__).with_name('test-msix-runtime.ps1'))).read_text(encoding='utf-8-sig')
        code=re.search(r"Add-Type -TypeDefinition @'\n(.*?)\n'@",source.replace('\r\n','\n'),re.S).group(1)
        with tempfile.TemporaryDirectory(prefix='td-window-target-') as name:
            directory=Path(name)
            log=(directory/'fixture.log').open('w',encoding='utf-8')
            child=subprocess.Popen([sys.executable,__file__,'--fixture',name],stdout=log,stderr=log)
            try:
                deadline=time.monotonic()+8
                while not (directory/'ready.json').exists():
                    self.assertIsNone(child.poll(), 'window fixture exited')
                    self.assertLess(time.monotonic(),deadline)
                    time.sleep(.02)
                helper=directory/'close.ps1'
                helper.write_text("$ErrorActionPreference='Stop'\nAdd-Type -TypeDefinition @'\n"+code+"\n'@\n[TwoDriveMsixSmoke.Native]::Close("+str(child.pid)+")\n",encoding='utf-8')
                result=subprocess.run(['powershell.exe','-NoProfile','-NonInteractive','-File',str(helper)],capture_output=True,text=True,timeout=15)
                self.assertEqual(result.returncode,0,result.stdout+result.stderr)
                deadline=time.monotonic()+3
                while not (directory/'runner-stopped').exists() and time.monotonic()<deadline:time.sleep(.02)
                self.assertTrue((directory/'runner-stopped').exists(),'quit reached another thread instead of the runner: '+result.stdout)
                child.wait(timeout=5)
            finally:
                if child.poll() is None:child.kill();child.wait(timeout=5)
                log.close()


if __name__=='__main__':
    if len(sys.argv)>1 and sys.argv[1]=='--fixture':fixture(Path(sys.argv[2]))
    else:unittest.main()
