#!/usr/bin/env python3
"""Two pictures of the window, Qt's on the left: side-by-side.py qt.png gpui.png out.png [x,y,w,h]

The crop is in the window's own units (1440x860); the pictures are taken at 2x.
"""
import sys
from PIL import Image

qt, gpui, out = (Image.open(p).convert("RGB") for p in sys.argv[1:3]), None, sys.argv[3]
qt, gpui = list(qt)
if len(sys.argv) > 4:
    x, y, w, h = (int(v) * 2 for v in sys.argv[4].split(","))
    qt, gpui = (p.crop((x, y, x + w, y + h)) for p in (qt, gpui))
both = Image.new("RGB", (qt.width + gpui.width + 8, max(qt.height, gpui.height)), (255, 0, 255))
both.paste(qt, (0, 0))
both.paste(gpui, (qt.width + 8, 0))
both.save(out)
