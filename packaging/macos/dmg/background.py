#!/usr/bin/env python3
"""Writes background.svg (the Lusk Texte DMG window background) with its text outlined.

Edit the wording or colours here, then run this script and generate.py:

    python3 packaging/macos/dmg/background.py && python3 packaging/macos/dmg/generate.py

Needs fontTools (pip install fonttools). Kerning is not applied (plain advances).
"""
import os
import re
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
HERE=os.path.dirname(os.path.abspath(__file__))
F=os.path.join(HERE,'..','..','..','assets','fonts')+os.sep
def text(font, s, x, y, size, fill, track=0.0):
    f=TTFont(F+font); gs=f.getGlyphSet(); cmap=f.getBestCmap(); upm=f['head'].unitsPerEm; hm=f['hmtx']
    sc=size/upm; out=[]; pen=SVGPathPen(gs)
    for ch in s:
        g=cmap.get(ord(ch))
        if g is None: continue
        tp=TransformPen(pen,(sc,0,0,-sc,x,y)); gs[g].draw(tp)
        x+=hm[g][0]*sc+track
    d=pen.getCommands()
    d=re.sub(r'-?\d+\.\d+',lambda m:f'{float(m.group()):.3f}'.rstrip('0').rstrip('.'),d)
    return f'<path fill="{fill}" d="{d}"/>'
INK='#0b0b0c'; TEAL='#13807c'
svg=f'''<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="660" height="400" viewBox="0 0 660 400">
<!-- Lusk Texte DMG background. Text is outlined (Inter, JetBrains Mono, OFL) so every machine
     draws the same pixels; regenerate with generate.py. -->
<rect width="660" height="400" fill="#efe9dc"/>
<rect width="232" height="400" fill="#ffffff"/>
<image href="../../../assets/app-icon/hicolor/256x256/apps/bzh.lusk.texte.png" x="16" y="100" width="200" height="200"/>
<line x1="232.5" y1="0" x2="232.5" y2="400" stroke="{INK}" stroke-width="1.2"/>
<line x1="236.5" y1="0" x2="236.5" y2="400" stroke="{INK}" stroke-width="0.5"/>
{text("Inter-SemiBold.ttf","Lusk Texte",267,72,30,INK,-0.4)}
<line x1="267" y1="88.5" x2="624" y2="88.5" stroke="{INK}" stroke-width="0.6"/>
<path d="M271.5,81.5 L276,88.5 L271.5,95.5 L267,88.5 Z" fill="{TEAL}"/>
{text("Inter-Regular.ttf","Ce que vous écrivez vous appartient.",267,113,10.5,'#4a4a4a')}
''' + ''.join(f'<circle cx="{410+7*i}" cy="210" r="1.3" fill="{INK}"/>' for i in range(10)) + f'''
<path d="M476,204 L485,210 L476,216 Z" fill="{INK}"/>
<line x1="490" y1="200" x2="490" y2="220" stroke="{TEAL}" stroke-width="2"/>
<line x1="267" y1="335.5" x2="624" y2="335.5" stroke="{INK}" stroke-width="0.6"/>
{text("Inter-Medium.ttf","GLISSEZ VERS APPLICATIONS POUR INSTALLER",267,358,10.5,'#55524c',1.6)}
</svg>
'''
open(os.path.join(HERE,'background.svg'),'w',encoding='utf-8').write(svg)
