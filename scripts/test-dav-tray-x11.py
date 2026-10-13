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

def state():
 data=subprocess.check_output(['xwininfo','-id',window],text=True)
 return {k:re.search(k+r':\s+(.+)',data).group(1) for k in ['Absolute upper-left X','Absolute upper-left Y','Width','Height','Map State']}
def click():
 stamp=int(time.monotonic()*1000)&0xffffffff
 e=Event();e.crossing=Crossing(7,0,1,d,tray,root_window,0,stamp,16,16,tray_x+16,tray_y+16,0,3,1,0,0)
 x.XSendEvent(d,tray,0,0,c.byref(e));x.XFlush(d)
 for typ,mask in [(4,4),(5,8)]:
  e=Event();e.button=Button(typ,0,1,d,tray,root_window,0,stamp,16,16,tray_x+16,tray_y+16,0,1,1)
  x.XSendEvent(d,tray,0,0,c.byref(e));x.XFlush(d)
results=[]
initial=state()['Map State']
for wanted in (['IsUnMapped','IsViewable'] if initial=='IsViewable' else ['IsViewable','IsUnMapped'])*3:
 time.sleep(.35);t=time.monotonic();click()
 while True:
  s=state()
  if s['Map State']==wanted:break
  assert time.monotonic()-t<3,(wanted,s)
  time.sleep(.01)
 latency=round((time.monotonic()-t)*1000)
 time.sleep(.4)
 settled=state()
 assert settled['Map State']==wanted, 'visibility changed after settling'
 results.append(dict(expected=wanted,ms=latency,geometry=settled))
positions=[r['geometry'] for r in results if r['expected']=='IsViewable']
assert positions[0]==positions[1]==positions[2], 'window manager moved the panel between activations'
print(json.dumps({'single_click_roundtrips':results,'settling_ms':400,'stable':True},indent=2))
