# Each outdoor playfield's place on the Atlas of Rubi-Ka's detail picture, measured against Saavick's map (whose
# coordinates are known): patches of Saavick's big level inside the playfield's ground are matched in the Atlas'
# picture, a line is put through the offsets found (east and north apart), and the playfield's coordinates for the
# Atlas follow from it. The Atlas' detail level is given Saavick's rectangle moved BASE[0] pixels left (8 0 3568
# 4582); the 12 pixels it lies lower are put in the coordinates. This made assets/planetmaps/AoRK.xml (see "The Atlas
# of Rubi-Ka" in docs/CLIENT_UI.md). Needs numpy and Pillow.
#
# usage: python planetmap_fit.py <saavick level picture> <atlas picture> <saavick coords.xml> <sizes.txt> <out coords.xml>
#   <saavick level picture>: Saavick's third level whole (the `planetmaps` example's --image, level 2)
#   <atlas picture>: the Atlas' detail picture (AoRK_map3) as a PNG
#   <sizes.txt>: the `pfsizes` example's lines for the playfields of the coordinates file
import re
import sys

import numpy as np
from PIL import Image

Image.MAX_IMAGE_PIXELS = None
ORIGIN = (31022.0, 24880.0)
EXTENT = (18751.0, 24115.0)
RECT = (220, 0, 3780, 4582)
K = (RECT[2] - RECT[0]) / EXTENT[0]
KY = (RECT[3] - RECT[1]) / EXTENT[1]
BASE = (212, -12)
SEARCH = 14

s = np.asarray(Image.open(sys.argv[1]).convert('L'), np.float32)
a = np.asarray(Image.open(sys.argv[2]).convert('L'), np.float32)
sizes = {int(l.split()[0]): (float(l.split()[1]), float(l.split()[2])) for l in open(sys.argv[4]) if len(l.split()) == 3 and l.split()[1] != 'none'}


def places(path):
    out = []
    for tag in open(path, encoding='latin-1').read().split('<Playfield')[1:]:
        g = lambda n: re.search(n + r'="([^"]*)"', tag).group(1)
        out.append((int(g('id')), g('name'), float(g('x')), float(g('xscale')), float(g('z')), float(g('zscale'))))
    return out


def pixel(p, x, z):
    return RECT[0] + (p[2] + x - ORIGIN[0]) * K * p[3], RECT[3] - (p[4] + z - ORIGIN[1]) * KY * p[5]


def ncc(t, w):
    p = t.shape[0]
    t = t - t.mean()
    tn = np.sqrt((t * t).sum())
    if tn < 1e-3:
        return None
    size = [w.shape[0], w.shape[1]]
    corr = np.fft.irfft2(np.fft.rfft2(w, size) * np.fft.rfft2(t[::-1, ::-1], size), size)[p - 1:, p - 1:]
    c1 = np.cumsum(np.cumsum(np.pad(w, ((1, 0), (1, 0))), 0), 1)
    c2 = np.cumsum(np.cumsum(np.pad(w * w, ((1, 0), (1, 0))), 0), 1)
    s1 = c1[p:, p:] - c1[:-p, p:] - c1[p:, :-p] + c1[:-p, :-p]
    s2 = c2[p:, p:] - c2[:-p, p:] - c2[p:, :-p] + c2[:-p, :-p]
    var = np.maximum(s2 - s1 * s1 / (p * p), 1e-3)
    score = corr[:var.shape[0], :var.shape[1]] / (tn * np.sqrt(var))
    at = np.unravel_index(np.argmax(score), score.shape)
    return int(at[0]), int(at[1]), float(score[at])


def line(pts, baseline):
    # pts: (metres, offset, weight): the line most of them lie on (within a pixel or so), as
    # (offset at zero, slope, points on it, worst of them); candidates are the flat lines and those through two points
    pts = list(pts)
    if len(pts) < 3:
        return None
    m = np.array([p[0] for p in pts], float)
    o = np.array([p[1] for p in pts], float)
    w = np.array([p[2] for p in pts], float)
    cands = [(float(v), 0.0) for v in set(o.tolist())]
    if len(pts) >= 6 and m.max() - m.min() >= baseline:
        for i in range(len(pts)):
            for j in range(i + 1, len(pts)):
                if abs(m[i] - m[j]) >= 700:
                    slope = (o[i] - o[j]) / (m[i] - m[j])
                    if abs(slope) <= 0.006:
                        cands.append((float(o[i] - slope * m[i]), float(slope)))
    best = None
    for at0, slope in cands:
        on = np.abs(o - (at0 + slope * m)) <= 1.2
        score = float(w[on].sum()) - (0.5 if slope else 0.0)
        if best is None or score > best[0]:
            best = (score, on)
    on = best[1]
    if on.sum() >= 6 and m[on].max() - m[on].min() >= baseline:
        A = np.vstack([np.ones(on.sum()), m[on]]).T * w[on][:, None]
        at0, slope = np.linalg.lstsq(A, o[on] * w[on], rcond=None)[0]
    else:
        at0, slope = float(np.average(o[on], weights=w[on])), 0.0
    return float(at0), float(slope), int(on.sum()), float(np.abs(o[on] - (at0 + slope * m[on])).max())


out = ['<?xml version="1.0" ?><root>']
for p in places(sys.argv[3]):
    if not (0.8 <= p[3] <= 1.01) or p[0] not in sizes:
        continue
    pid, name, X, xs, Z, zs = p
    w, h = sizes[pid]
    side = 64 if min(w, h) <= 1100 else 128 if min(w, h) >= 3000 else 96
    x0, y1 = pixel(p, w * 0.06, h * 0.06)
    x1, y0 = pixel(p, w * 0.94, h * 0.94)
    xpts, ypts = [], []
    y = int(y0)
    while y + side <= y1:
        x = int(x0)
        while x + side <= x1:
            t = s[y:y + side, x:x + side] if y >= 0 and x >= 0 and y + side <= s.shape[0] and x + side <= s.shape[1] else None
            bx, by = x - BASE[0] - SEARCH, y - BASE[1] - SEARCH
            if t is not None and t.std() >= 8 and bx >= 0 and by >= 0 and bx + side + 2 * SEARCH <= a.shape[1] and by + side + 2 * SEARCH <= a.shape[0]:
                got = ncc(t, a[by:by + side + 2 * SEARCH, bx:bx + side + 2 * SEARCH])
                if got and got[2] >= 0.55:
                    dy, dx, score = got
                    # (the patch's middle, in metres from the playfield's corner)
                    mx = (x + side / 2 - RECT[0]) / (K * xs) - (X - ORIGIN[0])
                    mz = (RECT[3] - (y + side / 2)) / (KY * zs) - (Z - ORIGIN[1])
                    xpts.append((mx, x - (bx + dx), score))
                    ypts.append((mz, y - (by + dy), score))
            x += side // 2
        y += side // 2
    fx, fy = line(xpts, 1500), line(ypts, 1500)
    if not fx or not fy or fx[2] < 3 or fy[2] < 3 or pid in (6550, 6551):
        # (a town drawn another way on the two maps: it stays where Saavick's coordinates have it)
        print('%5d %-28s no match (%d patches): as Saavick has it' % (pid, name, len(xpts)))
        fx = fy = None
    if not fx:
        fx, fy = (BASE[0], 0.0, 0, 0.0), (BASE[1], 0.0, 0, 0.0)
    xs2 = xs - fx[1] / K
    zs2 = zs + fy[1] / KY
    X2 = ORIGIN[0] + (BASE[0] - fx[0] + (X - ORIGIN[0]) * K * xs) / (K * xs2)
    # (the Atlas' rectangle starts at the picture's top as Saavick's does: the 12 pixels lower are in the coordinates)
    Z2 = ORIGIN[1] + ((Z - ORIGIN[1]) * KY * zs + fy[0]) / (KY * zs2)
    print('%5d %-28s %4d patches  x: %7.2f %+.5f (%d, worst %.1f)  z: %7.2f %+.5f (%d, worst %.1f)  ->  %.1f %.4f %.1f %.4f' % (pid, name, len(xpts), fx[0], fx[1], fx[2], fx[3], fy[0], fy[1], fy[2], fy[3], X2, xs2, Z2, zs2))
    out.append('\t<Playfield id="%d" x="%.1f" xscale="%.4f" z="%.1f" zscale="%.4f"/>' % (pid, X2, xs2, Z2, zs2))
out.append('</root>')
open(sys.argv[5], 'w', encoding='utf-8', newline='\n').write('\n'.join(out) + '\n')
