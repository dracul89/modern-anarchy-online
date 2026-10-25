"""Reads Setupf/gfxtweak.bin as records [id, kind, n, n payload words] and prints what it finds (research tool)."""
import struct, collections, sys
path = sys.argv[1] if len(sys.argv) > 1 else 'C:/Users/danny/Workspace/rea_ao/Anarchy Online/Setupf/gfxtweak.bin'
d = open(path, 'rb').read()
w = struct.unpack('<%di' % (len(d) // 4), d[:len(d) // 4 * 4])
count = w[0]
i = 1
records = []
while i + 3 <= len(w):
    rid, kind, n = w[i], w[i + 1], w[i + 2]
    if n < 0 or i + 3 + n > len(w):
        print('stop at', i, 'record', rid, kind, n)
        break
    records.append((rid, kind, w[i + 3:i + 3 + n]))
    i += 3 + n
print('records', len(records), 'of', count, 'consumed', i, 'of', len(w))
kinds = collections.Counter(k for _, k, _ in records)
print(sorted(kinds.items()))

by_id = {}
for r in records:
    by_id.setdefault(r[0], []).append(r)
def fl(v):
    f = struct.unpack('<f', struct.pack('<i', v))[0]
    return f
def show(rid):
    for rid_, kind, p in by_id.get(rid, []):
        cells = []
        for v in p:
            if v in by_id and abs(v) > 100: cells.append(f'->{v}')
            elif abs(v) > 1 << 20 and 1e-6 < abs(fl(v)) < 1e6: cells.append(f'{fl(v):.3g}')
            else: cells.append(str(v))
        print(f'{rid_} kind {kind} n {len(p)}:', ' '.join(cells))
if len(sys.argv) > 2:
    for a in sys.argv[2:]:
        show(int(a))
