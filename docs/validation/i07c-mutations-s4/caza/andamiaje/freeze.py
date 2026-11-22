import hashlib
import importlib.util
import json
from pathlib import Path
import random

ROOT = Path(__file__).resolve().parent
TREE = Path('C:/nexo-work/val/hunt-s4/tree')
CORPUS_DIR = Path('C:/nexo-work/val/hunt-s4/corpus')
SEED = 64193

def load_sources(name):
    path = ROOT / name
    spec = importlib.util.spec_from_file_location('family_module', path)
    # The source constructors precede the scripts' CLI entry points.
    data = path.read_text(encoding='utf-8')
    ns = {'__file__': str(path)}
    exec(compile(data[:data.index('p=argparse.ArgumentParser()')], str(path), 'exec'), ns)
    return ns['sources']()

def directed():
    out = []
    lefts = ['true', 'p', 'TryAdd(p,1)', 'AssertEq(p,1)', '(true+1)', 'r[true]']
    rights = ['r', 'r[0]', '[r]', 'Len(r)', '(if b then r else [r])', 'Some(r)']
    ops = ['+', '-', '*', 'and', 'or']
    consumers = [
        'let c=z;', 'let c=[z];', 'let c=Len(z);', 'let c=z[0];',
        'let c=TryAdd(z,1);', 'AssertEq(z,1);',
        'if z {let c=r;}', 'match z {Some(x)=>{let c=x;},_=>{}}',
    ]
    tails = ['r=[1];', 'r=1;', 'r=[r];', 'r[0]=1;']
    for left in lefts:
        for right in rights:
            for op in ops:
                for i, consume in enumerate(consumers):
                    decl = 'var' if i % 2 else 'let'
                    expr = f'({left} {op} {right})'
                    out.append(f'fn Main(){{var b=true;var p=1;var r=[];{decl} z={expr};{consume}{tails[i%4]}}}')
    # Explicit join, assignment, array, closure, match and returning-branch shapes.
    wrappers = [
        'if b then {} else r', 'if b then {{return;}} else {}',
        '[{}]', '({})[0]', 'Some({})', 'Len({})',
        'TryAdd({},1)', 'AssertEq({},1)',
        'match Some(r) {{Some(x)=>{{let y={};}},_=>{{}}}}',
        'let f=|| {{ {} }};',
    ]
    for j in range(1600):
        left = lefts[j % len(lefts)]
        right = rights[(j // len(lefts)) % len(rights)]
        op = ops[(j // 11) % len(ops)]
        wrapper = wrappers[(j // 13) % len(wrappers)]
        expr = f'({left} {op} {right})'
        stmt = wrapper.format(expr)
        if not stmt.endswith(';') and not stmt.endswith('}'):
            stmt = 'let z=' + stmt + ';'
        elif not stmt.startswith(('match', 'let f')):
            stmt = 'let z=' + stmt + ';'
        out.append(f'fn Main(){{var b=true;var p=1;var r=[];let marker{j}=1;{stmt}r=[[1]];r=1;}}')
    out = list(dict.fromkeys(out))
    random.Random(SEED).shuffle(out)
    assert len(out) >= 3000
    return out

def main():
    assert not (ROOT / 'corpus-freeze.json').exists(), 'already frozen'
    previous = json.loads(Path('D:/nexo-work/mut-s3/hunt-corpus.json').read_text(encoding='utf-8'))
    groups = [
        ('directed', directed()),
        ('s4_family', load_sources('family_fuzz.py')),
        ('s4_index', load_sources('family_index.py')),
        ('s4_recheck', load_sources('family_fuzz_recheck.py')),
        ('previous_corpus', previous),
    ]
    rd = json.loads((TREE / 'tests/fixtures/i07c_rd.json').read_text(encoding='utf-8'))
    groups.append(('rd_152', [r['source'] for r in rd['rows']]))
    for label, glob in [('corpus_files', 'corpus/**/*.n'), ('fixture_files', 'tests/fixtures/**/*.n')]:
        groups.append((label, [p.read_text(encoding='utf-8', errors='replace') for p in sorted(TREE.glob(glob))]))
    entries = []
    seen = set()
    meta = []
    CORPUS_DIR.mkdir(parents=True, exist_ok=True)
    for name, srcs in groups:
        raw = json.dumps(srcs, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
        nnew = 0
        for src in srcs:
            if src in seen:
                continue
            seen.add(src)
            entries.append({'source': src, 'family': name})
            nnew += 1
        meta.append({'family': name, 'count': len(srcs), 'new_unique': nnew, 'sha256': hashlib.sha256(raw).hexdigest()})
    raw = json.dumps(entries, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
    (ROOT / 'corpus-frozen.json').write_bytes(raw)
    for i, item in enumerate(entries):
        (CORPUS_DIR / f'p{i:05d}.n').write_text(item['source'], encoding='utf-8', newline='')
    freeze = {'seed': SEED, 'entries': len(entries), 'sha256': hashlib.sha256(raw).hexdigest(),
              'groups': meta, 'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'corpus_directory': str(CORPUS_DIR)}
    (ROOT / 'corpus-freeze.json').write_text(json.dumps(freeze, ensure_ascii=False, indent=2), encoding='utf-8')
    print(json.dumps(freeze, ensure_ascii=False))

if __name__ == '__main__':
    main()
