#!/usr/bin/env python3
"""Verify the packed 2-bit glyph arrays and the set used by the Arabic shaper."""
from pathlib import Path
import re
import sys

root = Path(sys.argv[1]).resolve()
src = root / "Watch-Replies-1.2.6/watch-replies/src"
fonts = (src / "fonts.rs").read_text(encoding="utf-8")
shape = (src / "arabic.rs").read_text(encoding="utf-8")
font_impl = (src / "font.rs").read_text(encoding="utf-8")
forms = {int(x,16) for x in re.findall(r'\\u\{([0-9A-Fa-f]+)\}', shape) if 0xFE70 <= int(x,16) <= 0xFEFF}
keep = forms | {0x0640,0x060C,0x061B,0x061F} | set(range(0x0660,0x066A)) | set(range(0x06F0,0x06FA))
if "i / 4" not in font_impl or "6 - 2 * (i % 4)" not in font_impl:
    raise SystemExit("font.rs does not contain the 2-bit/four-pixels-per-byte decoder")
blocks = list(re.finditer(r'pub static (\w+): Font = Font \{(.*?)\n\};', fonts, re.S))
if len(blocks) != 2:
    raise SystemExit(f"Expected T and B font tables, found {len(blocks)}")

def arr(block,key):
    m=re.search(rf'    {key}: &\[(.*?)\n    \],',block,re.S)
    if not m: raise ValueError(f"missing {key}")
    return [int(x) for x in re.findall(r'\b\d+\b',m.group(1))]

def glyphs(block,key):
    m=re.search(rf'    {key}: &\[(.*?)\n    \],',block,re.S)
    if not m: raise ValueError(f"missing {key}")
    result=[]
    for line in m.group(1).splitlines():
        if 'Glyph {' not in line: continue
        cm=re.search(r"c: '(.*?)', adv:",line)
        vals={k:int(v) for k,v in re.findall(r'(adv|x|y|w|h|off): (-?\d+)',line)}
        raw=cm.group(1)
        cp=int(raw[3:-1],16) if raw.startswith('\\u{') else ord(raw.replace("\\'", "'").replace('\\\\','\\'))
        result.append((cp,vals))
    return result

size=0
for m in blocks:
    name,block=m.group(1),m.group(2)
    for key in ('bits','arabic_bits'):
        data=arr(block,key); size+=len(data)
        gkey='glyphs' if key=='bits' else 'arabic_glyphs'
        for cp,g in glyphs(block,gkey):
            need=(g['w']*g['h']+3)//4
            if g['off']+need>len(data):
                raise SystemExit(f"{name}:{gkey} U+{cp:04X} exceeds packed bitmap data")
    ag=glyphs(block,'arabic_glyphs')
    cps=[cp for cp,_ in ag]
    if cps != sorted(cps) or len(cps)!=len(set(cps)):
        raise SystemExit(f"{name}: Arabic glyphs are not strictly sorted/unique")
    missing=keep-set(cps)
    if missing:
        raise SystemExit(f"{name}: missing Arabic glyphs: {', '.join(f'U+{x:04X}' for x in sorted(missing))}")
    print(f"{name}: {len(glyphs(block,'glyphs'))} Latin + {len(ag)} Arabic glyphs")
print(f"Packed bitmap total: {size:,} bytes")
