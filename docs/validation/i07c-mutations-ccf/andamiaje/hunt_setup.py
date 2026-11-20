import json,os,subprocess,sys,shutil,time
from concurrent.futures import ThreadPoolExecutor
ROOT='C:/nexo-work/hunt'; os.makedirs(ROOT+'/corpus',exist_ok=True)
corpus=json.load(open('D:/nexo-work/mut-s3/hunt-corpus.json',encoding='utf-8'))
for i,s in enumerate(corpus): open(f'{ROOT}/corpus/p{i:05d}.n','w',encoding='utf-8',newline='').write(s)
print('programas escritos',len(corpus))
# arboles de trabajo: copia del arbol correcto sin target
for w in range(3):
    d=f'C:/nexo-work/val/hunt-w{w}'
    if not os.path.exists(d+'/src'):
        shutil.copytree('C:/nexo-work/val/i07c-verify',d,ignore=shutil.ignore_patterns('target','.git','.scratch'))
    print(d,os.path.exists(d+'/src/check/types.rs'))