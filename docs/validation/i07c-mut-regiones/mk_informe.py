"""Regenerates the counters and the per-mutant tables of INFORME.md from results-*.jsonl and hunt-*.json."""
import collections, json, re, subprocess
txt = open('INFORME.md', encoding='utf-8').read()
cut = txt.index('## Tablas por mutante'); head = txt[:cut]
def cnt(c): return collections.Counter(json.loads(l)['verdict'] for l in open(f'results-{c}.jsonl', encoding='utf-8') if l.strip())
def inc(c):
    try: return len({json.loads(l)['id'] for l in open(f'results-{c}-inconclusos.jsonl', encoding='utf-8') if l.strip()})
    except FileNotFoundError: return 0
a, b = cnt('s6cor'), cnt('s5b1'); na, nb = sum(a.values()), sum(b.values())
res = f"""| Corte | Poblacion | Ejecutados | Muertos | No compilan | Supervivientes | Inconclusos en resultados | Inconclusos a repetir (en `-inconclusos`) |
|---|---|---|---|---|---|---|---|
| S6-COR | 400 (de 1.313) | {na} | {a['MUERTO']} | {a['NO COMPILA']} | {a['SUPERVIVIENTE']} | {a['INCONCLUSO']} | {inc('s6cor')} |
| S5b.1 | 400 (de 1.048) | {nb} | {b['MUERTO']} | {b['NO COMPILA']} | {b['SUPERVIVIENTE']} | {b['INCONCLUSO']} | {inc('s5b1')} |
"""
i0 = head.index('| Corte | Poblacion'); i1 = head.index('**Cobertura incompleta')
head = head[:i0] + res + '\n' + head[i1:]
head = re.sub(r'se ejecutaron \d+ \(\d+ de S6-COR y \d+ de S5b\.1\)', f'se ejecutaron {na+nb} ({na} de S6-COR y {nb} de S5b.1)', head)
head = re.sub(r'Quedan \*\*\d+ pendientes', f'Quedan **{800-na-nb} pendientes', head)
t6 = subprocess.run(['python', 'gen_tables.py', 's6cor'], capture_output=True, text=True, encoding='utf-8').stdout
t5 = subprocess.run(['python', 'gen_tables.py', 's5b1'], capture_output=True, text=True, encoding='utf-8').stdout
open('INFORME.md', 'w', encoding='utf-8', newline='\n').write(head + '## Tablas por mutante\n\n### S6-COR (`1010938..2bc5c8d`; `parse.rs`, `effects.rs`, `names.rs`, `types.rs`)\n\n' + t6 + '\n\n### S5b.1 (`4acee20..1010938`; `types.rs`)\n\n' + t5 + '\n')
print(na, nb, na + nb)
for c in ('s6cor', 's5b1'):
    h = set(json.load(open(f'hunt-{c}.json', encoding='utf-8')))
    print(c, 'sin caza:', [json.loads(l)['id'] for l in open(f'results-{c}.jsonl', encoding='utf-8') if l.strip() and json.loads(l)['verdict'] == 'SUPERVIVIENTE' and json.loads(l)['id'] not in h])
