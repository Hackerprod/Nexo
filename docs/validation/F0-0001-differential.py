"""Independent validation only. Run after cargo build and cargo build --release.
No third-party packages, network, or implementation changes.
"""
from pathlib import Path
import random,subprocess,copy,json,tempfile
REPO=Path(__file__).resolve().parents[2]
TEMP=tempfile.TemporaryDirectory(prefix="nexo-differential-")
root=Path(TEMP.name);rng=random.Random(20261020)
results=[]
def run_case(name,src,expected_output,expected_code,expected_events):
 pth=root/'differential_case.nx';pth.write_text(src)
 for b in [str(REPO/'target/debug/nexo'),str(REPO/'target/release/nexo')]:
  p=subprocess.run([b,'run',str(pth)],capture_output=True,text=True,timeout=4)
  events=[s.removeprefix('mock-resource:') for s in p.stderr.splitlines() if s.startswith('mock-resource:')]
  errors=[s.split(':')[0] for s in p.stderr.splitlines() if not s.startswith('mock-resource:')]
  assert p.returncode==(1 if expected_code else 0) and p.stdout==expected_output and errors==([expected_code] if expected_code else []) and events==expected_events, (name,b,p.stdout,p.stderr,expected_output,expected_code,expected_events)
  results.append(name+' '+b)
for n in range(200):
 model={};src=['fn main(){','let guard=mock_open(42);']
 for i in range(5):
  model[i]=[[rng.randint(-20,20) for _ in range(3)] for _ in range(3)]
  src.append(f'var a{i}={model[i]};')
 for j in range(40):
  target,source=rng.randrange(5),rng.randrange(5);r,c=rng.randrange(3),rng.randrange(3);op=rng.randrange(4)
  if op==0:
   src.append(f'a{target}=a{source};');model[target]=copy.deepcopy(model[source])
  elif op==1:
   q=rng.randrange(3);src.append(f'a{target}[{r}]=a{source}[{q}];');model[target][r]=copy.deepcopy(model[source][q])
  elif op==2:
   v=rng.randint(-100,100);src.append(f'a{target}[{r}][{c}]={v};');model[target][r][c]=v
  else:
   src.append(f'{{var a{target}=a{source};a{target}[{r}][{c}]=999;}}')
 src.extend(f'print(a{i});' for i in range(5));src.append('}')
 run_case(f'copies_{n}',''.join(src),''.join(str(model[i])+'\n' for i in range(5)),None,['open:0:42','close:0:42'])
# Independently evaluate left-to-right checked arithmetic; include i64 endpoints and nested bounds errors.
lo,hi=-(1<<63),(1<<63)-1
def expr(depth):
 if depth==0 or rng.random()<.3:
  x=rng.choice([lo,hi,-1,0,1,rng.randint(-10**18,10**18)])
  return str(x),x,None
 if rng.random()<.1:return '[1][2]',None,'R0102'
 a,av,ae=expr(depth-1);b,bv,be=expr(depth-1)
 if ae:return f'({a}+{b})',None,ae
 if be:return f'({a}+{b})',None,be
 x=av+bv
 return f'({a}+{b})',x if lo<=x<=hi else None,None if lo<=x<=hi else 'R0101'
for n in range(200):
 s,val,err=expr(6)
 run_case(f'arithmetic_{n}',f'fn main(){{let guard=mock_open(42);print({s});}}','' if err else str(val)+'\n',err,['open:0:42','close:0:42'])
(root/'differential-results.json').write_text(json.dumps(dict(seed=20261020,programs=400,runs=len(results),passed=len(results)),indent=2))
print('PASS independently modeled programs 400, debug/release runs',len(results))
# Exact resource lifecycle order, not substring matches.
run_case('ownership_order','fn main(){let a=mock_open(1);let b=mock_open(2);{let a=move a;let c=mock_open(3);inspect(a);drop(c);}let d=move b;inspect(d);}','',None,['open:0:1','open:1:2','open:2:3','inspect:0:1','close:2:3','close:0:1','inspect:1:2','close:1:2'])
run_case('return_skips_acquisition','fn main(){let a=mock_open(1);{let b=move a;let c=mock_open(2);return;}let unreachable=mock_open(3);}','',None,['open:0:1','open:1:2','close:1:2','close:0:1'])
run_case('budget_cleanup','fn main(){let a=mock_open(1);{let b=mock_open(2);let a0=[1];'+''.join(f'let a{i}=[a{i-1},a{i-1}];' for i in range(1,25))+'}}','','R0001',['open:0:1','open:1:2','close:1:2','close:0:1'])
print('PASS 3 additional exact resource/event/budget cases, debug/release')
