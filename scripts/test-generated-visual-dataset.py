#!/usr/bin/env python3
"""Independently decode ordinary exported fixtures; never read a live sync root."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import struct
import zlib

parser=argparse.ArgumentParser()
parser.add_argument('directory',type=Path)
args=parser.parse_args()
root=args.directory.absolute()
if os.name=='nt':
    from ctypes import wintypes
    cf=ctypes.WinDLL('cldapi')
    cf.CfGetSyncRootInfoByPath.argtypes=[wintypes.LPCWSTR,ctypes.c_int,ctypes.c_void_p,wintypes.DWORD,ctypes.c_void_p]
    cf.CfGetSyncRootInfoByPath.restype=ctypes.c_int32
    basic=ctypes.c_int64()
    code=cf.CfGetSyncRootInfoByPath(str(root),0,ctypes.byref(basic),ctypes.sizeof(basic),None)
    filesystem=ctypes.create_unicode_buffer(64)
    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    kernel.GetVolumeInformationW.argtypes=[wintypes.LPCWSTR,wintypes.LPWSTR,wintypes.DWORD,ctypes.c_void_p,ctypes.c_void_p,ctypes.c_void_p,wintypes.LPWSTR,wintypes.DWORD]
    volume_ok=kernel.GetVolumeInformationW(root.anchor,None,0,None,None,None,filesystem,len(filesystem))
    print(json.dumps({'preflight_api':'CfGetSyncRootInfoByPath/BASIC','hresult':f'0x{code&0xffffffff:08x}','filesystem':filesystem.value if volume_ok else 'unknown','root_exists':root.is_dir(),'root_reparse':bool(root.lstat().st_file_attributes&0x400)}),flush=True)
    assert code==ctypes.c_int32(0x80070186).value, f'ordinary-data validation refuses managed/unknown root: HRESULT=0x{code&0xffffffff:08x}'

manifest=json.loads(root.with_suffix('.dataset.json').read_text(encoding='utf-8'))
assert manifest['dataset']=='visual_v1' and manifest['registered'] is False
files=manifest['files'];folders=manifest['directories']
assert len(files)==3 and len(folders)==1
expected=set()
for entry in folders+files:
    relative=PurePosixPath(entry['relative_path'])
    assert not relative.is_absolute() and len(relative.parts)<=2 and all(p not in ('..','.') for p in relative.parts)
    path=root.joinpath(*relative.parts)
    assert not path.is_symlink() and not getattr(path.lstat(),'st_file_attributes',0)&0x400
    expected.add(path)
actual=set(root.iterdir())
actual.update((root/folders[0]['relative_path']).iterdir())
assert actual==expected, 'unexpected or missing ordinary fixture entry'
rows=[]
for entry in files:
    data=(root/entry['relative_path']).read_bytes()
    assert len(data)==entry['size']==65536
    rows.append({'id':entry['id'],'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
    if entry['relative_path'].endswith('.png'):
        assert data[:8]==b'\x89PNG\r\n\x1a\n'
        offset=8;chunks=[];payload=b''
        while offset<len(data):
            length=struct.unpack('>I',data[offset:offset+4])[0]
            kind=data[offset+4:offset+8];body=data[offset+8:offset+8+length]
            assert len(body)==length
            crc=struct.unpack('>I',data[offset+8+length:offset+12+length])[0]
            assert zlib.crc32(kind+body)&0xffffffff==crc
            chunks.append(kind.decode('ascii'))
            if kind==b'IHDR':assert struct.unpack('>IIBBBBB',body)==(64,64,8,2,0,0,0)
            if kind==b'IDAT':payload+=body
            offset+=12+length
            if kind==b'IEND':break
        assert offset==len(data) and chunks==['IHDR','IDAT','tEXt','IEND']
        pixels=zlib.decompress(payload)
        expected_pixels=bytes(value for y in range(64) for value in ([0]+[v for x in range(64) for v in (x*4,y*4,(x^y)*4)]))
        assert pixels==expected_pixels
        rows[-1].update(format='PNG',width=64,height=64,crc_and_zlib_and_pixels='passed')
assert sum(r.get('format')=='PNG' for r in rows)==1
print(json.dumps({'registered':False,'files':rows,'directory_count':1,'total_file_bytes':196608},ensure_ascii=True))
