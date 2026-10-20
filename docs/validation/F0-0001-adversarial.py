"""Independent validation only. Run after cargo build and cargo build --release.
No third-party packages, network, or implementation changes.
"""
import subprocess,pathlib,time,random,json,tempfile
REPO=pathlib.Path(__file__).resolve().parents[2]
TEMP=tempfile.TemporaryDirectory(prefix="nexo-independent-")
root=pathlib.Path(TEMP.name)
CASES=[
('resource_multiple_moves','fn main(){let a=mock_open(1);let b=move a;{let c=move b;inspect(c);}}',0,['open:0:1','inspect:0:1','close:0:1']),
('resource_move_reorders_ownership','fn main(){let a=mock_open(1);let b=mock_open(2);let c=move a;}',0,['open:0:1','open:1:2','close:0:1','close:1:2']),
('shadow_move_then_reuse','fn main(){let a=mock_open(1);{let a=move a;}inspect(a);}',1,['E0202']),
('shadow_data_restores_outer','fn main(){var a=[[1],[2]];{var a=a[0];a[0]=9;}print(a);}',0,['[[1], [2]]']),
('move_wrap_parens','fn main(){let a=mock_open(1);let b=(((move a)));inspect(b);}',0,['open:0:1','inspect:0:1','close:0:1']),
('move_nesting','fn main(){let a=mock_open(1);let b=[move a];}',1,['E0203']),
('resource_var_rebind','fn main(){var a=mock_open(1);a=mock_open(2);}',1,['E0203']),
('move_data','fn main(){let a=[1];let b=move a;}',1,['E0102']),
('drop_data','fn main(){let a=1;drop(a);}',1,['E0102']),
('drop_twice','fn main(){let a=mock_open(1);drop(a);drop(a);}',1,['E0202']),
('double_move','fn main(){let a=mock_open(1);let b=move a;let c=move a;}',1,['E0202']),
('missing_branch_syntax','fn main(){if true{print(1);}}',1,['E0002']),
('effects_rejected','fn main() with no_alloc {}',1,['E0002']),
('bool_arithmetic','fn main(){print(true+1);}',1,['E0102']),
('bool_index','fn main(){print([1][true]);}',1,['E0102']),
('nested_length_independence','fn main(){var a=[[1],[2,3]];var b=a[1];b[0]=9;print(a);print(b);}',0,['[[1], [2, 3]]','[9, 3]']),
('self_assignment_copy','fn main(){var a=[[1],[2]];a[0]=a[1];a[1][0]=3;print(a);}',0,['[[2], [3]]']),
('bool_array_copy','fn main(){var a=[[true]];var b=a;b[0][0]=false;print(a);print(b);}',0,['[[true]]','[[false]]']),
('overflow_left_before_bounds','fn main(){print((9223372036854775807+1)+[1][2]);}',1,['R0101']),
('bounds_left_before_overflow','fn main(){print([1][2]+(9223372036854775807+1));}',1,['R0102']),
('array_literal_first_error','fn main(){print([9223372036854775807+1,[1][2]]);}',1,['R0101']),
('assert_error_cleanup','fn main(){let a=mock_open(1);{let b=mock_open(2);assert_eq([1],[2]);}}',1,['open:0:1','open:1:2','close:1:2','close:0:1','R0201']),
('nested_return_propagation','fn main(){let a=mock_open(1);{{return;}}print(1);}',0,['open:0:1','close:0:1']),
('failed_initializer_no_acquisition','fn main(){let a=mock_open([1][2]);}',1,['R0102']),
('negative_index_before_later_index','fn main(){var a=[[0]];a[-1][9223372036854775807+1]=0;}',1,['R0101']),
('negative_index_before_rhs','fn main(){var a=[0];a[-1]=9223372036854775807+1;}',1,['R0101']),
]
results=[]
for binary in [str(REPO/'target/debug/nexo'),str(REPO/'target/release/nexo')]:
 for name,src,exit,expected in CASES:
  pth=root/(name+'.nx');pth.write_text(src)
  p=subprocess.run([binary,'run',str(pth)],capture_output=True,text=True,timeout=5)
  out=p.stdout+p.stderr
  expected_events=[s for s in expected if s.startswith(('open:', 'inspect:', 'close:'))]
  expected_codes=[s for s in expected if len(s)==5 and s[0] in 'ER' and s[1:].isdigit()]
  expected_output=[s for s in expected if s not in expected_events and s not in expected_codes]
  actual_events=[s.removeprefix('mock-resource:') for s in p.stderr.splitlines() if s.startswith('mock-resource:')]
  actual_codes=[s.split(':',1)[0] for s in p.stderr.splitlines() if not s.startswith('mock-resource:')]
  ok=(p.returncode==exit and p.stdout==''.join(s+'\n' for s in expected_output)
      and actual_events==expected_events and actual_codes==expected_codes)
  result=dict(binary=binary,name=name,ok=ok,exit=p.returncode,output=out)
  results.append(result)
  if not ok:print('FAIL',result,flush=True)
print('Semantic cases:',sum(r['ok'] for r in results),'/',len(results))
(root/'semantic-results.json').write_text(json.dumps(results,indent=2))
# All truncation points and a reproducible mutation corpus, parser/checker robust handling.
sources=[s for _,s,_,_ in CASES]
rng=random.Random(19022026)
mutations=set()
for s in sources:
 for i in range(len(s)+1):mutations.add(s[:i])
for _ in range(1200):
 s=rng.choice(sources);i=rng.randrange(len(s)+1)
 mutations.add(s[:i]+rng.choice(['}','{',';',')','(','[',']','-','+','🙂','//','let','\x00'])+s[i:])
crashes=[];t=time.monotonic()
for n,s in enumerate(sorted(mutations)):
 pth=root/'mutation.nx';pth.write_text(s)
 p=subprocess.run([str(REPO/'target/debug/nexo'),'check',str(pth)],capture_output=True,text=True,timeout=3)
 if p.returncode not in (0,1):crashes.append((s,p.returncode,p.stderr))
print('Parser mutation inputs:',len(mutations),'crashes',len(crashes),'seconds',round(time.monotonic()-t,2),flush=True)
(root/'mutation-results.json').write_text(json.dumps(dict(seed=19022026,inputs=len(mutations),crashes=crashes),indent=2))

assert all(r["ok"] for r in results), "Semantic case failed"
assert not crashes, "Mutation caused abnormal process termination"

# Independent boundary repros, including the three review findings.
prefix=b'fn main(){}//'
expr='1'
for _ in range(110):
    expr='('+expr+'+'+'+'.join(['0']*128)+')'
def type_source(n):
    return ('fn main(){let a0=0;'+''.join(f'let a{i}=[a{i-1}];' for i in range(1,n+1))+'}').encode()
boundaries=[
    ('source_exact_limit',prefix+b' '*(1048576-len(prefix)),0,None),
    ('source_ascii_over',prefix+b' '*(1048577-len(prefix)),1,'E0003'),
    ('source_unicode_cut',prefix+b' '*(1048576-len(prefix))+'🙂'.encode(),1,'E0003'),
    ('source_invalid_utf8',b'fn main(){}//\xff',2,'E0004'),
    ('composed_depth',('fn main(){print('+expr+');}').encode(),1,'E0003'),
    ('inferred_depth_4000',type_source(4000),1,'E0003'),
    ('inferred_depth_128',type_source(128),0,None),
    ('inferred_depth_129',type_source(129),1,'E0003'),
]
for name,source,status,code in boundaries:
    path=root/(name+'.nx');path.write_bytes(source)
    for binary in [REPO/'target/debug/nexo', REPO/'target/release/nexo']:
        result=subprocess.run([str(binary),'check',str(path)],capture_output=True,text=True,timeout=5)
        assert result.returncode==status, (name,binary,result)
        assert result.stderr.startswith(code+':') if code else result.stderr=='', (name,binary,result)
print('PASS independent size, UTF-8, AST and type boundary checks:',len(boundaries)*2)
