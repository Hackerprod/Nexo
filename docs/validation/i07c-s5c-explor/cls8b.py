import sys, os, json, itertools
here = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, here)
import eval3 as E
from concurrent.futures import ThreadPoolExecutor
exe = sys.argv[1]; P = sys.argv[2].split(',')
d = json.load(open(os.path.join(here, 'clasificacion-56.json'))); items = d['items']
main = json.load(open(os.path.join(E.RES, 'main.json')))
core = lambda r: json.dumps([x[:4] for x in r])
# main core por item: recomputar con exe sin flags NEXO_XV='' (todo apagado == main)
def run(fl): 
    with ThreadPoolExecutor(6) as ex: return list(ex.map(lambda it: E.run_bin(exe, it[3], it[1], fl), items))
base = run('')
sets = [(p,) for p in P] + list(itertools.combinations(P, 2))
res = {}
for s in sets:
    r = run(','.join(s)); res[s] = [core(r[j]) != core(base[j]) for j in range(len(items))]
# causales: la unitaria mas pequena que ya difiere de main
for j, it in enumerate(items[:56]):
    solo = [s[0] for s in sets if len(s) == 1 and res[s][j]]
    pair = [','.join(s) for s in sets if len(s) == 2 and res[s][j] and not any(x in solo for x in s)]
    print(it[0], it[2], 'solo:', solo, 'pares nuevos:', pair[:4])
