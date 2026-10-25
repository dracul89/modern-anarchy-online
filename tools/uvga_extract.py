"""Unpacks a skin's Graphics.UVGA into PNG files using its Graphics.UVGI index (research tool).

UVGI is text: a title line, a count, then `NAME offset length` lines; each entry is a whole PNG at that offset of the UVGA.
Usage: uvga_extract.py <skin folder> <out folder>"""
import os, sys, struct

skin, out = sys.argv[1], sys.argv[2]
index = open(os.path.join(skin, 'Graphics.UVGI'), encoding='latin1').read().splitlines()
data = open(os.path.join(skin, 'Graphics.UVGA'), 'rb').read()
os.makedirs(out, exist_ok=True)
n = 0
for line in index[2:]:
    parts = line.split()
    if len(parts) != 3:
        continue
    name, off, length = parts[0], int(parts[1]), int(parts[2])
    png = data[off:off + length]
    if png[1:4] != b'PNG':
        print('not a PNG:', name, off)
        continue
    w, h = struct.unpack('>II', png[16:24])
    open(os.path.join(out, name + '.png'), 'wb').write(png)
    print(f'{name} {w}x{h}')
    n += 1
print(n, 'files')
