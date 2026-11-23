import sys, os, json
here = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, here)
import eval3 as E
from concurrent.futures import ThreadPoolExecutor
exe = sys.argv[1]; P = sys.argv[2].split(',')
d = json.load(open(os.path.join(here, 'clasificacion-56.json'))); items = d['items']
core = lambda r: json.dumps([x[:4] for x in r])
MAIN = json.load(open(os.path.join(E.RES, 'main.json')))
def mn(it):
    base = core(MAIN['%s|%s' % (it[0], it[1])][it[2]])
    if core(E.run_bin(exe, it[3], it[1], '')) != base: return ['ALT2-base (todo apagado ya difiere de main)']
    cur = list(P)
    assert core(E.run_bin(exe, it[3], it[1], ','.join(cur))) != base
    for p in list(cur):
        t = [x for x in cur if x != p]
        if core(E.run_bin(exe, it[3], it[1], ','.join(t))) != base: cur = t
    return cur
with ThreadPoolExecutor(6) as ex: ms = list(ex.map(mn, items))
by = {}
for it, m in zip(items, ms): by.setdefault(','.join(m), []).append((it[0], it[2]))
for k, v in sorted(by.items(), key=lambda kv: -len(kv[1])): print(len(v), k, v[:5])
json.dump({k: v for k, v in by.items()}, open(os.path.join(here, 'clasificacion-56-minimos.json'), 'w'))
