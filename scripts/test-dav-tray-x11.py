#!/usr/bin/env python3
"""Send GNOME-style tray events only to the selected TwoDrive GTK process.
Requires an already running client and X11 tools. Does not inject global input.
"""
import argparse
import ctypes as c,subprocess,time,re,json
x=c.CDLL('libX11.so.6');x.XOpenDisplay.restype=c.c_void_p;d=x.XOpenDisplay(None)
class Button(c.Structure):
 _fields_=[('type',c.c_int),('serial',c.c_ulong),('send_event',c.c_int),('display',c.c_void_p),('window',c.c_ulong),('root',c.c_ulong),('subwindow',c.c_ulong),('time',c.c_ulong),('x',c.c_int),('y',c.c_int),('x_root',c.c_int),('y_root',c.c_int),('state',c.c_uint),('button',c.c_uint),('same_screen',c.c_int)]
class Crossing(c.Structure):
 _fields_=Button._fields_[:12]+[('mode',c.c_int),('detail',c.c_int),('same_screen',c.c_int),('focus',c.c_int),('state',c.c_uint)]
class Event(c.Union):
 _fields_=[('button',Button),('crossing',Crossing),('pad',c.c_long*24)]
x.XSendEvent.argtypes=[c.c_void_p,c.c_ulong,c.c_int,c.c_long,c.POINTER(Event)];x.XFlush.argtypes=[c.c_void_p]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--pid',type=int,required=True)
args=parser.parse_args()
tree=subprocess.check_output(['xwininfo','-root','-tree'],text=True)
windows={}
for line in tree.splitlines():
 for role,marker in [('tray','("twodrive-development" "twodrive-development")'),('window','("app.twodrive.twodrive_dav" "App.twodrive.twodrive_dav")')]:
  if marker not in line or 'TwoDrive Development' not in line:continue
  ident=line.strip().split()[0]
  prop=subprocess.check_output(['xprop','-id',ident,'_NET_WM_PID'],text=True)
  if re.search(r'= '+str(args.pid)+r'\b',prop):windows[role]=ident
assert set(windows)=={'tray','window'}, 'selected process does not own the expected GTK tray/window'
window=windows['window'];tray=int(windows['tray'],16)
info=subprocess.check_output(['xwininfo','-id',windows['tray']],text=True)
tray_x=int(re.search(r'Absolute upper-left X:\s+(-?\d+)',info).group(1))
tray_y=int(re.search(r'Absolute upper-left Y:\s+(-?\d+)',info).group(1))
x.XDefaultRootWindow.argtypes=[c.c_void_p];x.XDefaultRootWindow.restype=c.c_ulong
root_window=x.XDefaultRootWindow(d)
assert d, 'X11 display is unavailable'
x.XQueryPointer.argtypes=[c.c_void_p,c.c_ulong,c.POINTER(c.c_ulong),c.POINTER(c.c_ulong),c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(c.c_uint)]
workarea_text=subprocess.check_output(['xprop','-root','_NET_WORKAREA'],text=True)
workarea=[int(v) for v in workarea_text.split('=',1)[1].strip().split(',')[:4]]

def pointer():
 root=c.c_ulong();child=c.c_ulong();rx=c.c_int();ry=c.c_int();wx=c.c_int();wy=c.c_int();mask=c.c_uint()
 assert x.XQueryPointer(d,root_window,c.byref(root),c.byref(child),c.byref(rx),c.byref(ry),c.byref(wx),c.byref(wy),c.byref(mask)), 'pointer is on another X screen'
 return [rx.value,ry.value]
def expected_geometry(point,geometry):
 scale=int(geometry['Width'])//390
 assert scale>=1 and int(geometry['Width'])==390*scale, 'fixture requires a workarea wider than the panel'
 ax,ay,aw,ah=[v//scale for v in workarea]
 px,py=[int(v/scale+.5) for v in point]
 left=max(ax+8,min(px+20-390,ax+aw-390-8))
 preferred=py-570-20 if py-570-20>=ay+8 else py+20
 top=max(ay+8,min(preferred,ay+ah-570-8))
 return [left*scale,top*scale,390*scale,570*scale]

def state():
 data=subprocess.check_output(['xwininfo','-id',window],text=True)
 return {k:re.search(k+r':\s+(.+)',data).group(1) for k in ['Absolute upper-left X','Absolute upper-left Y','Width','Height','Map State']}
def click():
 stamp=int(time.monotonic()*1000)&0xffffffff
 e=Event();e.crossing=Crossing(7,0,1,d,tray,root_window,0,stamp,16,16,16,16,0,3,1,0,0)
 x.XSendEvent(d,tray,0,0,c.byref(e));x.XFlush(d)
 for typ,mask in [(4,4),(5,8)]:
  e=Event();e.button=Button(typ,0,1,d,tray,root_window,0,stamp,16,16,16,16,0,1,1)
  x.XSendEvent(d,tray,0,0,c.byref(e));x.XFlush(d)
results=[]
anchor_point=None
initial=state()['Map State']
for wanted in (['IsUnMapped','IsViewable'] if initial=='IsViewable' else ['IsViewable','IsUnMapped'])*3:
 time.sleep(.35);point=pointer();t=time.monotonic();click()
 while True:
  s=state()
  if s['Map State']==wanted:break
  assert time.monotonic()-t<3,(wanted,s)
  time.sleep(.01)
 latency=round((time.monotonic()-t)*1000)
 time.sleep(.4)
 settled=state()
 assert settled['Map State']==wanted, 'visibility changed after settling'
 if wanted=='IsViewable':
  assert all(abs(a-b)<=4 for a,b in zip(point,pointer())), 'pointer moved during this read-only fixture; rerun when stationary'
  if anchor_point is None:anchor_point=point
  assert all(abs(a-b)<=4 for a,b in zip(anchor_point,point)), 'pointer moved between activations; rerun when stationary'
  actual=[int(settled[k]) for k in ['Absolute upper-left X','Absolute upper-left Y','Width','Height']]
  expected=expected_geometry(anchor_point,settled)
  assert actual==expected, ('anchor mismatch: GNOME icon-local coordinates must not position the panel',actual,expected)
 results.append(dict(expected=wanted,ms=latency,geometry=settled,pointer=point))
positions=[r['geometry'] for r in results if r['expected']=='IsViewable']
assert positions[0]==positions[1]==positions[2], 'window manager moved the panel between activations'
print(json.dumps({'single_click_roundtrips':results,'settling_ms':400,'stable':True,'pointer_anchor_verified':True,'host_event_root':'icon-local center'},indent=2))
