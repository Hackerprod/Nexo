import json,glob,re,hashlib,collections,os
D='D:/nexo-work/gate-d/docs/validation/i07c-mutations-ccf'
OUT='D:/nexo-work/evidence'; os.makedirs(OUT,exist_ok=True)
pop=json.load(open(D+'/population.json',encoding='utf-8'))['mutations']
order=[m['id'] for m in pop]; P={m['id']:m for m in pop}
rows={}
for f in sorted(glob.glob(D+'/results-0?-of-06.jsonl')):
    for l in open(f,encoding='utf-8'):
        if l.strip(): r=json.loads(l); rows[r['id']]=r
assert set(rows)==set(order),'faltan filas'
# resultados unidos en orden de poblacion
with open(OUT+'/results.jsonl','w',encoding='utf-8',newline='\n') as f:
    for i in order: f.write(json.dumps(rows[i],ensure_ascii=False)+'\n')
# tablas de los lotes
LOTES=['I07c-C-mutaciones-lote%d.md'%n for n in range(1,7)]
lote={}
for n,fn in enumerate(LOTES,1):
    p='C:/projects/Nexo-i07c/docs/correcciones/'+fn
    for l in open(p,encoding='utf-8'):
        m=re.match(r'\|\s*(M\d{5})\s*\|(.*)',l)
        if m:
            cells=[c.strip() for c in m.group(2).split('|')]
            lote[m.group(1)]=(n,' | '.join(cells[:-1]) if len(cells)>1 else cells[0])
# no terminacion y verificaciones
nt_pairs=collections.defaultdict(list)
for fn in ('verify-hang.jsonl','verify-lib.jsonl'):
    for l in open('D:/nexo-work/mut-s3/'+fn,encoding='utf-8'):
        r=json.loads(l)
        if r.get('tipo')=='mutante' and r['resultado'].startswith('NO_TERMINA'): nt_pairs[r['id']].append((r.get('perfil','debug'),r.get('filtro','lib completa'),r['resultado']))
        if r.get('tipo')=='mutante' and r['resultado']=='FALLA': nt_pairs[r['id']].append(('FALLA',r.get('filtro'),'detectado por fallo de test'))
nx=[json.loads(l) for l in open('D:/nexo-work/mut-s3/verify-nextest.jsonl',encoding='utf-8')]
for r in nx:
    if r['timeouts']: nt_pairs[r['id']].append(('debug','nextest suite completa','TIMEOUT por test 600 s: '+r['timeouts'][0].split(') ')[-1]))
hunt={}
for f in sorted(glob.glob('D:/nexo-work/mut-s3/hunt-?.jsonl')):
    for l in open(f,encoding='utf-8'):
        r=json.loads(l)
        if 'id' in r: hunt[r['id']]=r
def prof_classes(r): return set(r['result'].values())
cls={}
for i in order:
    r=rows[i]; v=prof_classes(r); L=lote.get(i)
    if i in nt_pairs and any(x[2].startswith(('NO_TERMINA','TIMEOUT')) for x in nt_pairs[i]) and not (L and L[0]>=1 and 'matado' in L[1]):
        cls[i]={'clase':'detectada_por_no_terminacion','evidencia':nt_pairs[i]}
    elif i in nt_pairs and any(x[0]=='FALLA' for x in nt_pairs[i]):
        cls[i]={'clase':'detectada','evidencia':'fallo de test aislado (verify-hang)'}
    elif v<= {'detectada','no_compila','equivalente_test'} and 'superviviente' not in v and 'inconclusa' not in v:
        cls[i]={'clase':'detectada' if 'detectada' in v else ('no_compila' if 'no_compila' in v else 'equivalente_test'),'evidencia':'campana'}
    elif L:
        n,txt=L; t=txt.lower()
        h=hunt.get(i)
        if re.match(r'\W*matado por',t): c='detectada_por_regresion_nueva'
        elif re.match(r'\W*equivalente',t) or re.match(r'\W*(invariante|equivalencia)',t): c='equivalente_con_argumento'
        elif re.match(r'\W*sin programa',t):
            c='sin_programa_tras_caza' if h and h['resultado']=='SIN_DIFERENCIA' else 'sin_programa'
        else: c='revisar'
        cls[i]={'clase':c,'lote':n,'detalle':txt[:300]}
    else:
        cls[i]={'clase':'sin_clasificar','campana':r['result']}
# los 10 del lote 6 sobreescriben
for i,(n,txt) in lote.items():
    if n==6 and i in cls: cls[i]={'clase':'detectada_por_regresion_nueva','lote':6,'detalle':txt[:300]}
cnt=collections.Counter(c['clase'] for c in cls.values())
print(dict(cnt),'total',sum(cnt.values()))
json.dump(cls,open(OUT+'/clasificacion.json','w',encoding='utf-8'),ensure_ascii=False,indent=1)
rev=[i for i,c in cls.items() if c['clase'] in ('revisar','sin_clasificar','sin_programa')]
print('a revisar:',rev[:30],len(rev))
